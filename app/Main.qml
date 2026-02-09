import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import QtMultimedia 6.5
import QtQuick.Shapes 1.15
import App 1.0

ApplicationWindow {
    id: root
    visible: true
    width: 1200
    height: 800
    title: "Kaptik"

    // Theme management
    property bool darkMode: Settings.value(Settings.Key_DarkMode)

    readonly property color bgPrimaryLight: Qt.color("#f6f5f4")
    readonly property color bgSecondaryLight: Qt.color("#ffffff")
    readonly property color bgTertiaryLight: Qt.color("#deddda")
    readonly property color textPrimaryLight: Qt.color("#2e3436")
    readonly property color textSecondaryLight: Qt.color("#5e5c64")
    readonly property color borderColorLight: Qt.color("#c0bfbc")
    readonly property color hoverBgLight: Qt.color("#e1e0de")

    readonly property color bgPrimaryDark: Qt.color("#242424")
    readonly property color bgSecondaryDark: Qt.color("#303030")
    readonly property color bgTertiaryDark: Qt.color("#3d3d3d")
    readonly property color textPrimaryDark: Qt.color("#ffffff")
    readonly property color textSecondaryDark: Qt.color("#deddda")
    readonly property color borderColorDark: Qt.color("#4d4d4d")
    readonly property color hoverBgDark: Qt.color("#3d3d3d")

    // Active colors based on theme
    readonly property color bgPrimary: darkMode ? bgPrimaryDark : bgPrimaryLight
    readonly property color bgSecondary: darkMode ? bgSecondaryDark : bgSecondaryLight
    readonly property color bgTertiary: darkMode ? bgTertiaryDark : bgTertiaryLight
    readonly property color accentBlue: Qt.color("#3584e4")
    readonly property color textPrimary: darkMode ? textPrimaryDark : textPrimaryLight
    readonly property color textSecondary: darkMode ? textSecondaryDark : textSecondaryLight
    readonly property color borderColor: darkMode ? borderColorDark : borderColorLight
    readonly property color hoverBg: darkMode ? hoverBgDark : hoverBgLight

    property int currentView: 0 // 0 = grid, 1 = player, 2 = settings
    property string currentVideoSource: ""
    property int currentVideoIndex: -1

    property bool videoSelectedNotPlaying: false
    property bool videoHasBeenPlayed: false

    onCurrentVideoSourceChanged: {
        if (currentVideoSource !== "") {
            videoSelectedNotPlaying = true
            videoHasBeenPlayed = false
        } else {
            videoSelectedNotPlaying = false
            videoHasBeenPlayed = false
        }
        playerPage.mediaPlayer.source = currentVideoSource
    }

    onCurrentViewChanged: {
        if (currentView === 1) {
            playerPage.videoPlayerArea.forceActiveFocus()
        }
    }

    signal videoSelected(string apmPath)

    color: bgPrimary

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // Header Bar (GNOME-style)
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 48
            color: bgSecondary

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 12
                anchors.rightMargin: 12
                spacing: 12

                // Navigation buttons
                Row {
                    spacing: 4

                    RoundButton {
                        id: gridViewBtn
                        width: 36
                        height: 36
                        text: "▦"
                        font.pixelSize: 16
                        flat: true

                        palette.buttonText: currentView === 0 ? accentBlue : textSecondary

                        background: Rectangle {
                            color: gridViewBtn.hovered ? hoverBg : (currentView === 0 ? hoverBg : "transparent")
                            radius: 6
                        }

                        onClicked: {
                            currentView = 0
                            playerPage.mediaPlayer.pause()
                        }
                    }

                    RoundButton {
                        id: playerViewBtn
                        width: 36
                        height: 36
                        text: "▶"
                        font.pixelSize: 14
                        flat: true

                        palette.buttonText: currentView === 1 ? accentBlue : textSecondary

                        background: Rectangle {
                            color: playerViewBtn.hovered ? hoverBg : (currentView === 1 ? hoverBg : "transparent")
                            radius: 6
                        }

                        onClicked: currentView = 1
                    }
                }

                Rectangle {
                    width: 1
                    height: 24
                    color: borderColor
                }

                Label {
                    text: currentView === 0 ? "All Captures" : (currentView === 1 ? "Player" : "Settings")
                    font.pixelSize: 16
                    font.weight: Font.DemiBold
                    color: textPrimary
                }

                Item {
                    Layout.fillWidth: true
                }

                // Theme Toggle Button
                BaseRoundButton {
                    iconSource: darkMode ? "qrc:/resources/icons/weather-clear-night-symbolic.svg" : "qrc:/resources/icons/weather-clear-symbolic.svg"

                    onClicked: {
                        root.darkMode = !root.darkMode
                        Settings.saveDarkMode(root.darkMode)
                    }
                }

                // Capture Button
                RoundButton {
                    id: captureBtn
                    text: recording ? "⏹ Stop" : "⏺ Capture"
                    width: 120
                    height: 36
                    flat: true

                    property bool recording: false

                    palette.buttonText: "#ffffff"
                    font.pixelSize: 14

                    background: Rectangle {
                        color: captureBtn.recording ? "#e01b24" : accentBlue
                        radius: 6
                        opacity: captureBtn.hovered ? 0.9 : 1.0
                    }

                    onClicked: {
                        recording = !recording
                    }
                }

                // Settings button
                BaseRoundButton {
                    iconSource: "qrc:/resources/icons/view-more-symbolic.svg"
                    onClicked: currentView = 2
                }
            }

            Rectangle {
                anchors.bottom: parent.bottom
                width: parent.width
                height: 1
                color: borderColor
            }
        }

        StackLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                currentIndex: currentView

                GridPage { id: gridPage }
                PlayerPage { id: playerPage }
                SettingsPage { id: settingsPage }

                Connections {
                    target: root
                    function onVideoSelected(apmPath) {
                        playerPage.loadApmDataForVideo(apmPath)
                    }
                }
            }
    }
}
