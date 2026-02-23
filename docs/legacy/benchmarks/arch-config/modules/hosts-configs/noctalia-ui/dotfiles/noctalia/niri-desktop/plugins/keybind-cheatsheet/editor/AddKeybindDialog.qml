import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell
import qs.Commons
import qs.Widgets

Popup {
  id: root

  property var categories: []
  property string compositor: ""
  property bool editMode: false
  property string selectedCategory: ""

  property string keys: ""
  property string rawKeys: ""
  property var modifiers: []
  property string key: ""
  property string action: ""
  property string description: ""

  signal keybindAdded(string categoryId, var bindData)

  // Get shell path with ~ instead of /home/user
  function getShellPath() {
    // shellDir is the directory containing shell.qml
    var path = Quickshell.shellDir ? Quickshell.shellDir.toString().replace("file://", "") : Quickshell.workingDirectory;
    var home = Quickshell.env("HOME");
    if (home && path.startsWith(home)) {
      return "~" + path.substring(home.length);
    }
    return path;
  }

  function getAllExistingBinds() {
    var binds = [];
    for (var i = 0; i < categories.length; i++) {
      var cat = categories[i];
      if (cat.binds) {
        for (var j = 0; j < cat.binds.length; j++) {
          binds.push(cat.binds[j]);
        }
      }
    }
    return binds;
  }

  width: 700
  height: root.compositor === "niri" ? 340 : 380
  modal: true
  closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
  anchors.centerIn: parent

  background: Rectangle {
    color: Color.mSurface
    radius: Style.radiusL
    border.color: Color.mOutline
  }

  onOpened: {
    if (!editMode) {
      keys = ""; rawKeys = ""; modifiers = []; key = ""; action = ""; description = "";
    }
    if (selectedCategory === "" && categories.length > 0) {
      selectedCategory = categories[0].id;
    }
  }

  ColumnLayout {
    anchors.fill: parent
    anchors.margins: Style.marginL
    spacing: Style.marginM

    NText {
      text: editMode ? "Edit Keybind" : "Add Keybind"
      font.pointSize: Style.fontSizeL
      font.weight: Font.Bold
      color: Color.mPrimary
    }

    NComboBox {
      id: categoryCombo
      visible: root.compositor === "hyprland"
      Layout.fillWidth: true
      label: "Category"
      minimumWidth: 580
      model: ListModel {
        id: categoryModel
      }
      currentKey: root.selectedCategory
      onSelected: key => root.selectedCategory = key

      function updateModel() {
        categoryModel.clear();
        for (var i = 0; i < root.categories.length; i++) {
          categoryModel.append({ name: root.categories[i].title, key: root.categories[i].id });
        }
      }

      Component.onCompleted: updateModel()
      Connections {
        target: root
        function onCategoriesChanged() { categoryCombo.updateModel() }
      }
    }

    RowLayout {
      Layout.fillWidth: true
      NText { text: "Keys:"; Layout.preferredWidth: 80 }
      Rectangle {
        Layout.fillWidth: true
        height: 36
        color: Color.mSurfaceVariant
        radius: Style.radiusS
        border.color: Color.mOutline

        Flow {
          anchors.centerIn: parent
          spacing: 4
          visible: root.keys !== ""
          Repeater {
            model: root.keys.split(" + ")
            Rectangle {
              width: kt.implicitWidth + 10; height: 22
              color: Color.mPrimary; radius: 4
              NText { id: kt; anchors.centerIn: parent; text: modelData; font.pointSize: 9; font.weight: Font.Bold; color: Color.mOnPrimary }
            }
          }
        }
        NText {
          anchors.centerIn: parent
          visible: root.keys === ""
          text: "Click to capture..."
          font.italic: true
          color: Color.mOnSurfaceVariant
        }
        MouseArea { anchors.fill: parent; onClicked: keyCapturePopup.open() }
      }
    }

    RowLayout {
      Layout.fillWidth: true
      NText { text: "Action:"; Layout.preferredWidth: 80 }
      NTextInput {
        Layout.fillWidth: true
        placeholderText: root.compositor === "niri" ? "e.g., spawn \"kitty\"" : "e.g., exec, kitty"
        text: root.action
        onTextChanged: root.action = text
      }
    }

    RowLayout {
      Layout.fillWidth: true
      NText { text: "Description:"; Layout.preferredWidth: 80 }
      NTextInput {
        Layout.fillWidth: true
        placeholderText: "What does this keybind do?"
        text: root.description
        onTextChanged: root.description = text
      }
    }

    // Quick templates section
    ColumnLayout {
      Layout.fillWidth: true
      Layout.fillHeight: true
      spacing: Style.marginS

      NText {
        text: "Quick Templates:"
        font.pointSize: Style.fontSizeS
        color: Color.mOnSurfaceVariant
      }

      Rectangle {
        Layout.fillWidth: true
        height: 32
        color: templateArea.containsMouse ? Qt.lighter(Color.mSurfaceVariant, 1.1) : Color.mSurfaceVariant
        radius: Style.radiusS
        border.color: templateArea.containsMouse ? Color.mPrimary : Color.mOutline

        RowLayout {
          anchors.fill: parent
          anchors.leftMargin: Style.marginS
          anchors.rightMargin: Style.marginS
          spacing: Style.marginS

          NText {
            text: "Noctalia IPC"
            font.pointSize: Style.fontSizeS
            font.weight: Font.Bold
            color: Color.mPrimary
          }

          NText {
            Layout.fillWidth: true
            text: root.compositor === "niri"
              ? "spawn \"qs\" \"-c\" \"" + root.getShellPath() + "\" \"ipc\" \"call\" ..."
              : "exec, qs -c " + root.getShellPath() + " ipc call ..."
            font.pointSize: Style.fontSizeXS
            color: Color.mOnSurfaceVariant
            elide: Text.ElideRight
          }

          NText {
            text: "Click to use"
            font.pointSize: Style.fontSizeXS
            font.italic: true
            color: Color.mPrimary
          }
        }

        MouseArea {
          id: templateArea
          anchors.fill: parent
          hoverEnabled: true
          cursorShape: Qt.PointingHandCursor
          onClicked: {
            var shellPath = root.getShellPath();
            if (root.compositor === "niri") {
              root.action = "spawn \"qs\" \"-c\" \"" + shellPath + "\" \"ipc\" \"call\" ";
            } else {
              root.action = "exec, qs -c " + shellPath + " ipc call ";
            }
          }
        }
      }

      Item { Layout.fillHeight: true }
    }

    RowLayout {
      Layout.fillWidth: true
      Item { Layout.fillWidth: true }
      NButton { text: "Cancel"; onClicked: root.close() }
      NButton {
        text: editMode ? "Save" : "Add"
        enabled: root.keys !== "" && root.description !== ""
        onClicked: {
          keybindAdded(selectedCategory, { keys: keys, rawKeys: rawKeys, modifiers: modifiers, key: key, action: action, description: description });
          root.close();
        }
      }
    }
  }

  KeyCaptureDialog {
    id: keyCapturePopup
    compositor: root.compositor
    existingBinds: root.getAllExistingBinds()
    onKeyCaptured: function(fk, rk, m, mk) { root.keys = fk; root.rawKeys = rk; root.modifiers = m; root.key = mk; }
  }
}
