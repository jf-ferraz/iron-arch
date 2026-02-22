import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import qs.Commons
import qs.Widgets

Rectangle {
  id: root

  property var category: ({})
  property var allCategories: []  // All categories for move dropdown
  property string compositor: ""
  property bool collapsed: true

  signal collapseToggled(string categoryId, bool isCollapsed)
  signal editKeybind(string bindId)
  signal deleteKeybind(string bindId)
  signal moveKeybind(string bindId, string targetCategoryId)
  signal reorderKeybind(string bindId, int direction)  // direction: -1 = up, 1 = down
  signal addKeybindToCategory(string categoryId)
  signal renameCategory(string categoryId, string currentTitle)
  signal deleteCategoryRequested(string categoryId, string categoryTitle)

  color: Color.mSurfaceVariant
  Layout.fillWidth: true
  radius: Style.radiusM
  implicitHeight: contentColumn.implicitHeight + Style.marginS * 2

  ColumnLayout {
    id: contentColumn
    anchors.left: parent.left
    anchors.right: parent.right
    anchors.top: parent.top
    anchors.margins: Style.marginS
    spacing: Style.marginS

    // Category header
    RowLayout {
      Layout.fillWidth: true
      spacing: Style.marginS

      // Collapse button
      NIconButton {
        icon: root.collapsed ? "chevron-right" : "chevron-down"
        baseSize: Style.baseWidgetSize * 0.7
        onClicked: {
          root.collapsed = !root.collapsed;
          root.collapseToggled(category.id, root.collapsed);
        }
      }

      // Category title
      NText {
        text: category.title || "Untitled Category"
        font.pointSize: Style.fontSizeM
        font.weight: Font.Bold
        color: Color.mPrimary
        Layout.fillWidth: true
      }

      // Keybind count
      NText {
        text: "(" + (category.binds?.length || 0) + " keybinds)"
        font.pointSize: Style.fontSizeS
        color: Color.mOnSurfaceVariant
      }

      // Add to category button
      NIconButton {
        icon: "add"
        tooltipText: "Add keybind to this category"
        onClicked: root.addKeybindToCategory(category.id)
      }

      // Rename category button (only for Hyprland - Niri uses auto-detected categories)
      NIconButton {
        visible: root.compositor === "hyprland"
        icon: "edit"
        baseSize: Style.baseWidgetSize * 0.7
        tooltipText: "Rename category"
        onClicked: root.renameCategory(category.id, category.title)
      }

      // Delete category button
      NIconButton {
        icon: "trash"
        baseSize: Style.baseWidgetSize * 0.7
        tooltipText: "Delete category"
        onClicked: root.deleteCategoryRequested(category.id, category.title)
      }
    }

    // Keybinds list
    ColumnLayout {
      visible: !root.collapsed
      Layout.fillWidth: true
      spacing: 2

      Repeater {
        model: category.binds || []

        KeybindRow {
          Layout.fillWidth: true
          bind: Object.assign({}, modelData, { categoryId: root.category.id })
          categories: root.allCategories
          compositor: root.compositor
          isFirst: index === 0
          isLast: index === (category.binds?.length || 0) - 1
          onEditRequested: root.editKeybind(modelData.id)
          onDeleteRequested: root.deleteKeybind(modelData.id)
          onMoveRequested: function(targetCategoryId) {
            root.moveKeybind(modelData.id, targetCategoryId);
          }
          onMoveUpRequested: root.reorderKeybind(modelData.id, -1)
          onMoveDownRequested: root.reorderKeybind(modelData.id, 1)
        }
      }

      // Empty state
      NText {
        visible: !category.binds || category.binds.length === 0
        text: "No keybinds in this category"
        font.pointSize: Style.fontSizeS
        color: Color.mOnSurfaceVariant
        font.italic: true
        Layout.alignment: Qt.AlignHCenter
        Layout.topMargin: Style.marginS
        Layout.bottomMargin: Style.marginS
      }
    }
  }
}
