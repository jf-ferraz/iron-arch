import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import qs.Commons
import qs.Widgets
import qs.Services.UI
import "writers"

Item {
  id: root

  property var pluginApi: null
  property var mainComponent: null
  property string compositor: ""

  property var editorData: mainComponent?.editorData || ({ "categories": [], "deletedBinds": [], "hasUnsavedChanges": false })
  property int refreshKey: 0  // Forces UI refresh when incremented

  // Store collapsed state per category ID (persists across editorData changes)
  property var collapsedStates: ({})

  function isCollapsed(categoryId) {
    // Default to true (collapsed) if not set
    return collapsedStates[categoryId] !== false;
  }

  function setCollapsed(categoryId, collapsed) {
    var newStates = Object.assign({}, collapsedStates);
    newStates[categoryId] = collapsed;
    collapsedStates = newStates;
  }

  // Get flat list of all binds for Niri view (with position info for reordering)
  function getAllBinds() {
    var allBinds = [];
    var cats = editorData.categories || [];
    for (var i = 0; i < cats.length; i++) {
      var cat = cats[i];
      if (cat.binds) {
        var catLen = cat.binds.length;
        for (var j = 0; j < catLen; j++) {
          allBinds.push(Object.assign({}, cat.binds[j], {
            categoryId: cat.id,
            isFirstInCategory: j === 0,
            isLastInCategory: j === catLen - 1
          }));
        }
      }
    }
    return allBinds;
  }

  Connections {
    target: mainComponent
    function onEditorDataChanged() {
      editorData = mainComponent?.editorData || { "categories": [], "deletedBinds": [], "hasUnsavedChanges": false };
      refreshKey++;  // Force UI refresh
    }
  }

  ColumnLayout {
    anchors.fill: parent
    spacing: Style.marginS

    // Toolbar
    RowLayout {
      Layout.fillWidth: true
      spacing: Style.marginS

      NText {
        text: editorData.hasUnsavedChanges ? "Unsaved changes" : ""
        color: Color.mError
        font.pointSize: Style.fontSizeS
      }

      Item { Layout.fillWidth: true }

      NButton {
        text: "Add Category"
        icon: "add"
        visible: root.compositor === "hyprland"
        onClicked: addCategoryPopup.open()
      }

      NButton {
        text: "Add Keybind"
        icon: "add"
        visible: root.compositor === "niri"
        onClicked: { addPopup.selectedCategory = (editorData.categories && editorData.categories[0]) ? editorData.categories[0].id : ""; addPopup.open(); }
      }

      NButton {
        text: "Discard"
        visible: editorData.hasUnsavedChanges
        onClicked: { if (mainComponent) mainComponent.discardChanges(); }
      }

      NButton {
        text: "Save"
        icon: "download"
        enabled: editorData.hasUnsavedChanges
        onClicked: saveChanges()
      }
    }

    // Categories list (Hyprland)
    NScrollView {
      id: categoriesScrollView
      visible: root.compositor === "hyprland"
      Layout.fillWidth: true
      Layout.fillHeight: true
      clip: true

      ColumnLayout {
        width: categoriesScrollView.availableWidth
        spacing: Style.marginS

        Repeater {
          model: editorData.categories || []

          CategorySection {
            Layout.fillWidth: true
            category: modelData
            allCategories: editorData.categories || []
            compositor: root.compositor
            collapsed: root.isCollapsed(modelData.id)
            onCollapseToggled: function(catId, isCollapsed) { root.setCollapsed(catId, isCollapsed); }
            onEditKeybind: function(bindId) { editPopup.editBind(bindId); }
            onDeleteKeybind: function(bindId) { if (mainComponent) mainComponent.deleteKeybind(bindId); }
            onMoveKeybind: function(bindId, targetCategoryId) {
              if (mainComponent) mainComponent.moveKeybind(bindId, targetCategoryId);
            }
            onReorderKeybind: function(bindId, direction) {
              if (mainComponent) mainComponent.reorderKeybind(bindId, direction);
            }
            onAddKeybindToCategory: function(catId) { addPopup.selectedCategory = catId; addPopup.open(); }
            onRenameCategory: function(catId, currentTitle) {
              renameCategoryPopup.categoryId = catId;
              renameCategoryPopup.currentTitle = currentTitle;
              renameCategoryInput.text = currentTitle;
              renameCategoryPopup.open();
            }
            onDeleteCategoryRequested: function(catId, catTitle) {
              deleteCategoryPopup.categoryId = catId;
              deleteCategoryPopup.categoryTitle = catTitle;
              deleteCategoryPopup.open();
            }
          }
        }

        NText {
          visible: !editorData.categories || editorData.categories.length === 0
          text: "No categories found. Click 'Add Category' to create one."
          color: Color.mOnSurfaceVariant
          Layout.alignment: Qt.AlignHCenter
          Layout.topMargin: Style.marginL
        }
      }
    }

    // Flat binds list (Niri)
    NScrollView {
      id: niriBindsScrollView
      visible: root.compositor === "niri"
      Layout.fillWidth: true
      Layout.fillHeight: true
      clip: true

      ColumnLayout {
        width: niriBindsScrollView.availableWidth
        spacing: 2

        Repeater {
          model: root.getAllBinds()

          KeybindRow {
            Layout.fillWidth: true
            bind: modelData
            compositor: root.compositor
            categories: []  // No move between categories for Niri
            isFirst: modelData.isFirstInCategory || false
            isLast: modelData.isLastInCategory || false
            onEditRequested: editPopup.editBind(modelData.id)
            onDeleteRequested: { if (mainComponent) mainComponent.deleteKeybind(modelData.id); }
            onMoveUpRequested: { if (mainComponent) mainComponent.reorderKeybind(modelData.id, -1); }
            onMoveDownRequested: { if (mainComponent) mainComponent.reorderKeybind(modelData.id, 1); }
          }
        }

        NText {
          visible: root.getAllBinds().length === 0
          text: "No keybinds found. Click 'Add Keybind' to create one."
          color: Color.mOnSurfaceVariant
          Layout.alignment: Qt.AlignHCenter
          Layout.topMargin: Style.marginL
        }
      }
    }
  }

  // Add category popup
  Popup {
    id: addCategoryPopup
    width: 300; height: 150
    modal: true; anchors.centerIn: parent
    background: Rectangle { color: Color.mSurface; radius: Style.radiusL; border.color: Color.mOutline }

    ColumnLayout {
      anchors.fill: parent; anchors.margins: Style.marginM; spacing: Style.marginS
      NText { text: "Add Category"; font.weight: Font.Bold; color: Color.mPrimary }
      NTextInput { id: catNameInput; Layout.fillWidth: true; placeholderText: "Category name" }
      RowLayout {
        Layout.fillWidth: true
        Item { Layout.fillWidth: true }
        NButton { text: "Cancel"; onClicked: addCategoryPopup.close() }
        NButton {
          text: "Add"
          enabled: catNameInput.text.trim() !== ""
          onClicked: { if (mainComponent) mainComponent.addCategory(catNameInput.text.trim()); catNameInput.text = ""; addCategoryPopup.close(); }
        }
      }
    }
  }

  // Rename category popup
  Popup {
    id: renameCategoryPopup
    property string categoryId: ""
    property string currentTitle: ""
    width: 350; height: 160
    modal: true; anchors.centerIn: parent
    background: Rectangle { color: Color.mSurface; radius: Style.radiusL; border.color: Color.mOutline }

    ColumnLayout {
      anchors.fill: parent; anchors.margins: Style.marginM; spacing: Style.marginS
      NText { text: "Rename Category"; font.weight: Font.Bold; color: Color.mPrimary }
      NText { text: "Current: " + renameCategoryPopup.currentTitle; color: Color.mOnSurfaceVariant; font.pointSize: Style.fontSizeS }
      NTextInput { id: renameCategoryInput; Layout.fillWidth: true; placeholderText: "New category name" }
      RowLayout {
        Layout.fillWidth: true
        Item { Layout.fillWidth: true }
        NButton { text: "Cancel"; onClicked: renameCategoryPopup.close() }
        NButton {
          text: "Rename"
          enabled: renameCategoryInput.text.trim() !== "" && renameCategoryInput.text.trim() !== renameCategoryPopup.currentTitle
          onClicked: {
            console.log("[EditorContent] Rename clicked, mainComponent:", mainComponent);
            console.log("[EditorContent] categoryId:", renameCategoryPopup.categoryId, "newTitle:", renameCategoryInput.text.trim());
            if (mainComponent) {
              var result = mainComponent.renameCategory(renameCategoryPopup.categoryId, renameCategoryInput.text.trim());
              console.log("[EditorContent] renameCategory result:", result);
            } else {
              console.log("[EditorContent] ERROR: mainComponent is null!");
            }
            renameCategoryPopup.close();
          }
        }
      }
    }
  }

  // Delete category confirmation popup
  Popup {
    id: deleteCategoryPopup
    property string categoryId: ""
    property string categoryTitle: ""
    width: 400; height: 180
    modal: true; anchors.centerIn: parent
    background: Rectangle { color: Color.mSurface; radius: Style.radiusL; border.color: Color.mError }

    ColumnLayout {
      anchors.fill: parent; anchors.margins: Style.marginM; spacing: Style.marginS
      NText { text: "Delete Category"; font.weight: Font.Bold; color: Color.mError }
      NText {
        text: "Are you sure you want to delete \"" + deleteCategoryPopup.categoryTitle + "\"?\n\nAll keybinds in this category will be removed from config."
        wrapMode: Text.WordWrap
        Layout.fillWidth: true
        color: Color.mOnSurface
      }
      Item { Layout.fillHeight: true }
      RowLayout {
        Layout.fillWidth: true
        Item { Layout.fillWidth: true }
        NButton { text: "Cancel"; onClicked: deleteCategoryPopup.close() }
        NButton {
          text: "Delete"
          onClicked: {
            if (mainComponent) mainComponent.deleteCategory(deleteCategoryPopup.categoryId);
            deleteCategoryPopup.close();
          }
        }
      }
    }
  }

  // Add keybind popup
  AddKeybindDialog {
    id: addPopup
    categories: editorData.categories || []
    compositor: root.compositor
    onKeybindAdded: function(catId, data) { if (mainComponent) mainComponent.addKeybind(catId, data); }
  }

  // Edit keybind popup
  AddKeybindDialog {
    id: editPopup
    categories: editorData.categories || []
    compositor: root.compositor
    editMode: true

    property string currentBindId: ""

    function editBind(bindId) {
      currentBindId = bindId;
      // Find bind data and populate
      for (var i = 0; i < editorData.categories.length; i++) {
        var cat = editorData.categories[i];
        for (var j = 0; j < cat.binds.length; j++) {
          if (cat.binds[j].id === bindId) {
            var b = cat.binds[j];
            keys = b.keys || "";
            rawKeys = b.rawKeys || "";
            modifiers = b.modifiers || [];
            key = b.key || "";
            action = b.action || "";
            description = b.description || "";
            selectedCategory = cat.id;
            break;
          }
        }
      }
      open();
    }

    onKeybindAdded: function(catId, data) {
      if (mainComponent && currentBindId) {
        mainComponent.updateKeybind(currentBindId, data);
      }
    }
  }

  // Writers
  HyprlandWriter {
    id: hyprlandWriter
    pluginApi: root.pluginApi
    onSaveComplete: function(ok, msg) {
      if (ok) { ToastService.showNotice("Saved!", "success"); if (mainComponent) mainComponent.runParser(); }
      else { ToastService.showNotice("Error: " + msg, "error"); }
    }
  }

  NiriWriter {
    id: niriWriter
    pluginApi: root.pluginApi
    onSaveComplete: function(ok, msg) {
      if (ok) { ToastService.showNotice("Saved!", "success"); if (mainComponent) mainComponent.runParser(); }
      else { ToastService.showNotice("Error: " + msg, "error"); }
    }
  }

  function saveChanges() {
    console.log("[EditorContent] saveChanges called");
    console.log("[EditorContent] mainComponent: " + mainComponent);
    console.log("[EditorContent] compositor: " + compositor);
    console.log("[EditorContent] editorData.categories: " + (editorData.categories ? editorData.categories.length : "null"));
    console.log("[EditorContent] editorData.deletedBinds: " + (editorData.deletedBinds ? editorData.deletedBinds.length : "null"));
    console.log("[EditorContent] editorData.hasUnsavedChanges: " + editorData.hasUnsavedChanges);

    if (editorData.categories) {
      for (var i = 0; i < editorData.categories.length; i++) {
        var cat = editorData.categories[i];
        console.log("[EditorContent]   Category: " + cat.title + " (titleChanged: " + cat.titleChanged + ")");
        if (cat.binds) {
          for (var j = 0; j < cat.binds.length; j++) {
            var b = cat.binds[j];
            console.log("[EditorContent]     Bind: " + b.keys + " status=" + b.status + " sourceFile=" + (b.sourceFile ? "yes" : "no"));
          }
        }
      }
    }

    if (!mainComponent) return;
    if (compositor === "hyprland") hyprlandWriter.saveAll(editorData);
    else if (compositor === "niri") niriWriter.saveAll(editorData);
  }
}
