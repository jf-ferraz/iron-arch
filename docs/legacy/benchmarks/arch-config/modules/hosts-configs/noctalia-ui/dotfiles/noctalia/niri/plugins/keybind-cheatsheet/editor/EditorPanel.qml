import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell
import qs.Commons
import qs.Services.UI
import qs.Widgets
import "writers"

Item {
  id: root
  property var pluginApi: null

  // Reference to Main.qml via pluginApi
  property var mainComponent: pluginApi?.mainInstance || null

  // Settings
  property var cfg: pluginApi?.pluginSettings || ({})
  property var defaults: pluginApi?.manifest?.metadata?.defaultSettings || ({})

  // Editor data from Main.qml
  property var editorData: mainComponent?.editorData || ({ "categories": [], "deletedBinds": [], "hasUnsavedChanges": false })
  property string compositor: pluginApi?.pluginSettings?.detectedCompositor || ""

  // Panel geometry
  property real contentPreferredWidth: cfg.windowWidth ?? defaults.windowWidth ?? 1400
  property real contentPreferredHeight: calculateDynamicHeight()
  readonly property var geometryPlaceholder: panelContainer
  readonly property bool allowAttach: false
  readonly property bool panelAnchorHorizontalCenter: true
  readonly property bool panelAnchorVerticalCenter: true
  anchors.fill: parent

  property var panelOpenScreen: pluginApi?.panelOpenScreen
  property real maxScreenHeight: panelOpenScreen ? panelOpenScreen.height * 0.9 : 800

  function calculateDynamicHeight() {
    var baseHeight = 100; // Header + margins
    var categoriesHeight = 0;

    for (var i = 0; i < editorData.categories.length; i++) {
      var cat = editorData.categories[i];
      categoriesHeight += 50; // Category header
      categoriesHeight += cat.binds.length * 45; // Each bind row
      categoriesHeight += 20; // Spacing
    }

    return Math.max(400, Math.min(baseHeight + categoriesHeight, maxScreenHeight));
  }

  // Refresh when editor data changes
  Connections {
    target: mainComponent
    function onEditorDataChanged() {
      editorData = mainComponent.editorData;
    }
  }

  Rectangle {
    id: panelContainer
    anchors.fill: parent
    color: Color.mSurface
    radius: Style.radiusL
    clip: true

    // Header
    Rectangle {
      id: header
      anchors.top: parent.top
      anchors.left: parent.left
      anchors.right: parent.right
      height: 55
      color: Color.mSurfaceVariant
      radius: Style.radiusL

      RowLayout {
        anchors.fill: parent
        anchors.leftMargin: Style.marginM
        anchors.rightMargin: Style.marginM
        spacing: Style.marginS

        NIcon {
          icon: "edit"
          pointSize: Style.fontSizeM
          color: Color.mPrimary
        }

        NText {
          text: {
            var title = "Keybind Editor";
            if (root.compositor) {
              title += " (" + root.compositor.charAt(0).toUpperCase() + root.compositor.slice(1) + ")";
            }
            return title;
          }
          font.pointSize: Style.fontSizeM
          font.weight: Font.Bold
          color: Color.mPrimary
        }

        // Unsaved changes indicator
        NText {
          visible: editorData.hasUnsavedChanges
          text: "(unsaved changes)"
          font.pointSize: Style.fontSizeS
          color: Color.mError
        }

        Item { Layout.fillWidth: true }

        // Action buttons
        NButton {
          text: "Add Category"
          icon: "add"
          onClicked: addCategoryDialog.open()
        }

        NButton {
          text: "Add Keybind"
          icon: "add"
          onClicked: addKeybindDialog.open()
        }

        NButton {
          text: "Discard"
          icon: "close"
          visible: editorData.hasUnsavedChanges
          onClicked: {
            if (mainComponent) mainComponent.discardChanges();
          }
        }

        NButton {
          text: "Save"
          icon: "download"
          enabled: editorData.hasUnsavedChanges
          onClicked: saveChanges()
        }

        NIconButton {
          icon: "close"
          onClicked: {
            if (editorData.hasUnsavedChanges) {
              unsavedDialog.open();
            } else {
              pluginApi?.closePanel();
            }
          }
        }
      }
    }

    // Loading state
    NText {
      anchors.centerIn: parent
      text: "Loading editor data..."
      visible: editorData.categories.length === 0
      font.pointSize: Style.fontSizeL
      color: Color.mOnSurface
    }

    // Main content
    NScrollView {
      id: scrollView
      visible: editorData.categories.length > 0
      anchors.top: header.bottom
      anchors.bottom: parent.bottom
      anchors.left: parent.left
      anchors.right: parent.right
      anchors.margins: Style.marginM
      clip: true

      ColumnLayout {
        width: scrollView.availableWidth
        spacing: Style.marginM

        Repeater {
          model: editorData.categories

          CategorySection {
            Layout.fillWidth: true
            category: modelData
            compositor: root.compositor
            onEditKeybind: function(bindId) {
              editKeybindDialog.bindId = bindId;
              editKeybindDialog.open();
            }
            onDeleteKeybind: function(bindId) {
              if (mainComponent) mainComponent.deleteKeybind(bindId);
            }
            onAddKeybindToCategory: function(categoryId) {
              addKeybindDialog.selectedCategory = categoryId;
              addKeybindDialog.open();
            }
          }
        }
      }
    }
  }

  // ========== DIALOGS ==========

  // Unsaved changes confirmation
  Popup {
    id: unsavedDialog
    width: 350; height: 180
    modal: true; anchors.centerIn: parent
    background: Rectangle { color: Color.mSurface; radius: Style.radiusL; border.color: Color.mOutline }

    ColumnLayout {
      anchors.fill: parent; anchors.margins: Style.marginL; spacing: Style.marginM
      NText { text: "Unsaved Changes"; font.pointSize: Style.fontSizeL; font.weight: Font.Bold; color: Color.mPrimary }
      NText { text: "You have unsaved changes. Save before closing?"; wrapMode: Text.WordWrap; Layout.fillWidth: true }
      Item { Layout.fillHeight: true }
      RowLayout {
        Layout.fillWidth: true
        NButton { text: "Discard"; onClicked: { unsavedDialog.close(); pluginApi?.closePanel(); } }
        Item { Layout.fillWidth: true }
        NButton { text: "Cancel"; onClicked: unsavedDialog.close() }
        NButton { text: "Save"; onClicked: { saveChanges(); unsavedDialog.close(); pluginApi?.closePanel(); } }
      }
    }
  }

  // Add category dialog
  Popup {
    id: addCategoryDialog
    width: 350; height: 180
    modal: true; anchors.centerIn: parent
    background: Rectangle { color: Color.mSurface; radius: Style.radiusL; border.color: Color.mOutline }
    property string categoryName: ""

    ColumnLayout {
      anchors.fill: parent; anchors.margins: Style.marginL; spacing: Style.marginM
      NText { text: "Add Category"; font.pointSize: Style.fontSizeL; font.weight: Font.Bold; color: Color.mPrimary }
      NText { text: "Category name:" }
      NTextInput {
        id: categoryNameInput
        Layout.fillWidth: true
        placeholderText: "e.g., Custom Keybinds"
        onTextChanged: addCategoryDialog.categoryName = text
      }
      Item { Layout.fillHeight: true }
      RowLayout {
        Layout.fillWidth: true
        Item { Layout.fillWidth: true }
        NButton { text: "Cancel"; onClicked: addCategoryDialog.close() }
        NButton {
          text: "Add"
          enabled: addCategoryDialog.categoryName.trim() !== ""
          onClicked: {
            if (mainComponent) mainComponent.addCategory(addCategoryDialog.categoryName.trim());
            categoryNameInput.text = "";
            addCategoryDialog.close();
          }
        }
      }
    }
  }

  // Add keybind dialog
  AddKeybindDialog {
    id: addKeybindDialog
    categories: editorData.categories
    compositor: root.compositor
    onKeybindAdded: function(categoryId, bindData) {
      if (mainComponent) mainComponent.addKeybind(categoryId, bindData);
    }
  }

  // Edit keybind dialog
  AddKeybindDialog {
    id: editKeybindDialog
    property string bindId: ""
    categories: editorData.categories
    compositor: root.compositor
    editMode: true

    onKeybindAdded: function(categoryId, bindData) {
      if (mainComponent && bindId) {
        mainComponent.updateKeybind(bindId, bindData);
      }
    }
  }

  // Key capture dialog
  KeyCaptureDialog {
    id: keyCaptureDialog
  }

  // ========== FUNCTIONS ==========

  function saveChanges() {
    if (!mainComponent) return;

    // Determine which writer to use based on compositor
    if (compositor === "hyprland") {
      hyprlandWriter.saveAll(editorData);
    } else if (compositor === "niri") {
      niriWriter.saveAll(editorData);
    }
  }

  // Writers
  HyprlandWriter {
    id: hyprlandWriter
    pluginApi: root.pluginApi
    onSaveComplete: function(success, message) {
      if (success) {
        ToastService.showNotice("Keybinds saved successfully", "success");
        if (mainComponent) mainComponent.runParser(); // Refresh data
      } else {
        ToastService.showNotice("Failed to save: " + message, "error");
      }
    }
  }

  NiriWriter {
    id: niriWriter
    pluginApi: root.pluginApi
    onSaveComplete: function(success, message) {
      if (success) {
        ToastService.showNotice("Keybinds saved successfully", "success");
        if (mainComponent) mainComponent.runParser(); // Refresh data
      } else {
        ToastService.showNotice("Failed to save: " + message, "error");
      }
    }
  }
}
