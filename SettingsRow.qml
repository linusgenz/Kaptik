// SettingsRow.qml - Korrigierte Version
import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15

Item {  // GEÄNDERT von RowLayout zu Item
    id: settingsRow
    property string label: ""
    property string description: ""
    default property alias control: controlArea.data

    Layout.fillWidth: true
    Layout.preferredHeight: rowLayout.height  // GEÄNDERT

    RowLayout {
        id: rowLayout
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        spacing: 16

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 4

            Label {
                text: label
                font.pixelSize: 15
                color: textPrimary
            }

            Label {
                id: descriptionLabel
                text: description
                font.pixelSize: 13
                color: textSecondary
                visible: description !== ""
                wrapMode: Text.WordWrap
                Layout.fillWidth: true
            }
        }

        Item {
            id: controlArea
            Layout.alignment: Qt.AlignVCenter | Qt.AlignRight
            implicitWidth: childrenRect.width  // HINZUGEFÜGT
            implicitHeight: childrenRect.height  // HINZUGEFÜGT
        }
    }
}
