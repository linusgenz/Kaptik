// SettingsSection.qml - Korrigierte Version
import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15

ColumnLayout {
    id: settingsSection
    property string sectionTitle: ""
    default property alias content: contentLayout.data

    spacing: 12

    Label {
        text: sectionTitle
        font.pixelSize: 18
        font.weight: Font.DemiBold
        color: textPrimary
        Layout.bottomMargin: 4
    }

    Rectangle {
        Layout.fillWidth: true
        Layout.preferredHeight: childrenRect.height + 24
        color: bgSecondary
        radius: 12
        border.width: 1
        border.color: borderColor

        ColumnLayout {
            id: contentLayout
            width: parent.width
            anchors.margins: 12
            anchors.top: parent.top
            anchors.left: parent.left
            anchors.right: parent.right
            spacing: 12
        }
    }
}
