// SettingsPage.qml
import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import QtQuick.Dialogs
import App 1.0

Item {
    id: settingsPage

    property string videoLibraryPath: Settings.value(Settings.Key_VideoPath) || ""

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
                                    Settings.setValue(Settings.Key_DarkMode, checked)
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
                                        text: settingsPage.videoLibraryPath || "No folder selected"
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

                                    MouseArea {
                                        anchors.fill: parent
                                        cursorShape: Qt.PointingHandCursor
                                        acceptedButtons: Qt.NoButton
                                    }

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

                    // Recording Section
                    SettingsSection {
                        Layout.fillWidth: true
                        sectionTitle: "Recording"

                        // Capture Resolution
                        SettingsRow {
                            Layout.fillWidth: true
                            label: "Capture Resolution"
                            description: "Higher resolutions produce larger file sizes"

                            SettingsComboBox {
                                settingsKey: Settings.Key_Resolution
                                model: [
                                    { text: "720p (Low size)", value: Settings.Resolution720p },
                                    { text: "1080p (Recommended)", value: Settings.Resolution1080p },
                                    { text: "1440p", value: Settings.Resolution1440p },
                                    { text: "4K (Large files)", value: Settings.Resolution4K },
                                    { text: "Source", value: Settings.ResolutionSource }
                                ]
                            }
                        }

                        // FPS Limit
                        SettingsRow {
                            Layout.fillWidth: true
                            label: "FPS Limit"
                            description: "Balance between performance and smoothness"

                            SettingsComboBox {
                                settingsKey: Settings.Key_FpsLimit
                                model: [
                                    { text: "30 FPS (Performance)", value: Settings.Fps30 },
                                    { text: "60 FPS (Recommended)", value: Settings.Fps60 },
                                    { text: "120 FPS (High refresh)", value: Settings.Fps120 }
                                ]
                            }
                        }
                    }

                    // Audio Section
                    SettingsSection {
                        Layout.fillWidth: true
                        sectionTitle: "Audio"

                        SettingsRow {
                            Layout.fillWidth: true
                            label: "Game Audio"
                            description: "Record in-game sounds and music"

                            Switch {
                                id: gameAudioSwitch
                                checked: Settings.value(Settings.Key_GameAudio)
                                onToggled: Settings.setValue(Settings.Key_GameAudio, checked)

                                indicator: Rectangle {
                                    implicitWidth: 52
                                    implicitHeight: 28
                                    x: gameAudioSwitch.leftPadding
                                    y: parent.height / 2 - height / 2
                                    radius: height / 2
                                    color: gameAudioSwitch.checked ? accentBlue : borderColor

                                    Behavior on color {
                                        ColorAnimation {
                                            duration: 150
                                        }
                                    }

                                    Rectangle {
                                        x: gameAudioSwitch.checked ? parent.width - width - 3 : 3
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

                        SettingsRow {
                            Layout.fillWidth: true
                            label: "Microphone"
                            description: "Record microphone audio"

                            Switch {
                                id: microphoneSwitch
                                checked: Settings.value(Settings.Key_Microphone)
                                onToggled: Settings.setValue(Settings.Key_Microphone, checked)

                                indicator: Rectangle {
                                    implicitWidth: 52
                                    implicitHeight: 28
                                    x: microphoneSwitch.leftPadding
                                    y: parent.height / 2 - height / 2
                                    radius: height / 2
                                    color: microphoneSwitch.checked ? accentBlue : borderColor

                                    Behavior on color {
                                        ColorAnimation {
                                            duration: 150
                                        }
                                    }

                                    Rectangle {
                                        x: microphoneSwitch.checked ? parent.width - width - 3 : 3
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

                        SettingsRow {
                            Layout.fillWidth: true
                            label: "System Sounds"
                            description: "Record notifications and alerts"

                            Switch {
                                id: systemSoundsSwitch
                                checked: Settings.value(Settings.Key_SystemSounds)
                                onToggled: Settings.setValue(Settings.Key_SystemSounds, checked)

                                indicator: Rectangle {
                                    implicitWidth: 52
                                    implicitHeight: 28
                                    x: systemSoundsSwitch.leftPadding
                                    y: parent.height / 2 - height / 2
                                    radius: height / 2
                                    color: systemSoundsSwitch.checked ? accentBlue : borderColor

                                    Behavior on color {
                                        ColorAnimation {
                                            duration: 150
                                        }
                                    }

                                    Rectangle {
                                        x: systemSoundsSwitch.checked ? parent.width - width - 3 : 3
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

                    SettingsSection {
                        Layout.fillWidth: true
                        sectionTitle: "HDR & Tonemapping"

                        SettingsRow {
                            Layout.fillWidth: true
                            label: "Tonemapping Algorithm"
                            description: "Controls how HDR is converted to SDR video"

                            SettingsComboBox {
                                settingsKey: Settings.Key_TonemapAlgorithm

                                model: [
                                    { text: "ACES Fitted (Best Quality)", value: Settings.AcesFitted },
                                    { text: "ACES Simple (Fast)", value: Settings.AcesSimple },
                                    { text: "Uncharted 2 (Filmic)", value: Settings.Uncharted2 },
                                    { text: "Reinhard (Balanced)", value: Settings.Reinhard },
                                    { text: "Hejl-Dawson (Fast Filmic)", value: Settings.HejlDawson }
                                ]
                            }
                        }

                        SettingsRow {
                            Layout.fillWidth: true
                            label: "HDR Brightness Mode"
                            description: "How the recorder determines HDR peak brightness"

                            SettingsComboBox {
                                settingsKey: Settings.Key_HdrNitsMode

                                model: [
                                    { text: "Automatic (Recommended)", value: Settings.HdrNitsAuto },
                                    { text: "Assume 1000 nits", value: Settings.HdrNits1000 },
                                    { text: "Assume 2000 nits", value: Settings.HdrNits2000 },
                                    { text: "Assume 4000 nits", value: Settings.HdrNits4000 },
                                    { text: "Assume 10000 nits (HDR10 Max)", value: Settings.HdrNits10000 }
                                ]
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
            Settings.setValue(Settings.Key_VideoPath, url)

            clipModel.loadFromPath(url)
        }
    }

}
