// SettingsPage.qml
import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import QtQuick.Dialogs
import App 1.0

Item {
    id: settingsPage

    property string videoLibraryPath: Settings.loadVideoPath() || ""

    Rectangle {
        anchors.fill: parent
        color: bgPrimary

        Flickable {
            anchors.fill: parent
            contentHeight: contentColumn.height
            clip: true

            ColumnLayout {
                id: contentColumn
                width: parent.width
                spacing: 0

                // Header
                Item {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 80
                    Layout.topMargin: 24
                    Layout.bottomMargin: 12

                    Label {
                        anchors.centerIn: parent
                        text: "Settings"
                        font.pixelSize: 28
                        font.weight: Font.Bold
                        color: textPrimary
                    }
                }

                // Settings Container
                ColumnLayout {
                    Layout.fillWidth: true
                    Layout.leftMargin: 24
                    Layout.rightMargin: 24
                    Layout.alignment: Qt.AlignHCenter
                    Layout.maximumWidth: 800
                    spacing: 12

                    // Appearance Section
                    SettingsSection {
                        Layout.fillWidth: true
                        sectionTitle: "Appearance"

                        SettingsRow {
                            Layout.fillWidth: true
                            label: "Dark Mode"
                            description: "Use dark theme for the interface"

                            Switch {
                                id: themeSwitch
                                checked: root.darkMode

                                onToggled: {
                                    root.darkMode = checked
                                    Settings.saveDarkMode(checked)
                                }

                                indicator: Rectangle {
                                    implicitWidth: 52
                                    implicitHeight: 28
                                    x: themeSwitch.leftPadding
                                    y: parent.height / 2 - height / 2
                                    radius: height / 2
                                    color: themeSwitch.checked ? accentBlue : borderColor

                                    Behavior on color {
                                        ColorAnimation {
                                            duration: 150
                                        }
                                    }

                                    Rectangle {
                                        x: themeSwitch.checked ? parent.width - width - 3 : 3
                                        y: (parent.height - height) / 2
                                        width: 22
                                        height: 22
                                        radius: width / 2
                                        color: "#ffffff"

                                        Behavior on x {
                                            NumberAnimation {
                                                duration: 150
                                                easing.type: Easing.InOutQuad
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Storage Section
                    SettingsSection {
                        Layout.fillWidth: true
                        sectionTitle: "Storage"

                        SettingsRow {
                            Layout.fillWidth: true
                            label: "Video Folder"
                            description: "Choose where your captured videos are stored"

                            RowLayout {
                                spacing: 8

                                Rectangle {
                                    Layout.preferredWidth: 300
                                    Layout.preferredHeight: 36
                                    color: bgTertiary
                                    radius: 6
                                    border.width: 1
                                    border.color: borderColor

                                    Label {
                                        anchors.fill: parent
                                        anchors.leftMargin: 12
                                        anchors.rightMargin: 12
                                        text: settingsPage.videoLibraryPath || "No folder selected" // slice "file:///
                                        color: settingsPage.videoLibraryPath ? textPrimary : textSecondary
                                        verticalAlignment: Text.AlignVCenter
                                        elide: Text.ElideMiddle
                                    }
                                }

                                Button {
                                    text: "Browse..."
                                    flat: true

                                    palette.buttonText: textPrimary
                                    font.pixelSize: 14

                                    padding: 4

                                    background: Rectangle {
                                        color: parent.hovered ? hoverBg : bgSecondary
                                        radius: 6
                                        border.width: 1
                                        border.color: borderColor
                                    }

                                    onClicked: folderDialog.open()
                                }
                            }
                        }
                    }

                    // About Section
                    SettingsSection {
                        Layout.fillWidth: true
                        sectionTitle: "About"

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 8

                            Label {
                                text: "Kaptik"
                                font.pixelSize: 18
                                font.weight: Font.DemiBold
                                color: textPrimary
                            }

                            Label {
                                text: "Version 1.0.0"
                                font.pixelSize: 14
                                color: textSecondary
                            }

                            Label {
                                text: "© " + new Date().getFullYear() + " Kaptik. All rights reserved.\n" +
                                      "Developed by Linus Genz.\n" +
                                      "Unauthorized copying, distribution or modification is prohibited."
                                font.pixelSize: 12
                                color: textSecondary
                                wrapMode: Text.Wrap
                                Layout.topMargin: 4
                            }
                        }
                    }

                    Item {
                        Layout.fillHeight: true
                        Layout.minimumHeight: 24
                    }
                }
            }
        }
    }

    FolderDialog {
        id: folderDialog
        title: "Select Video Folder"

        currentFolder: settingsPage.videoLibraryPath
            ? "file:///" + settingsPage.videoLibraryPath.replace(/\\/g, "/")
            : ""

        onAccepted: {
            var url = selectedFolder.toString()

            // file:///C:/... → C:/...
            if (url.startsWith("file:///"))
                url = url.slice(8)
            else if (url.startsWith("file://"))
                url = url.slice(7)

            settingsPage.videoLibraryPath = url
            Settings.saveVideoPath(url)

            clipModel.loadFromPath(url)
        }
    }

}
