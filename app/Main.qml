import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import QtMultimedia 6.5
import QtQuick.Shapes 1.15
import Qt5Compat.GraphicalEffects
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

    property string searchText: ""

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

    signal videoSelected(string dataFilePath)

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
                        flat: true

                        background: Rectangle {
                            color: {
                                if (currentView === 0) {
                                    return gridViewBtn.hovered ? hoverBg : Qt.darker(hoverBg, 1.1)
                                }
                                return gridViewBtn.hovered ? hoverBg : "transparent"
                            }
                            radius: 6
                        }

                        contentItem: Image {
                            anchors.centerIn: parent
                            width: 20
                            height: 20
                            sourceSize: Qt.size(width, height)

                            source: "qrc:/resources/icons/view-grid-symbolic.svg"
                            fillMode: Image.PreserveAspectFit
                            smooth: true

                            ColorOverlay {
                                anchors.fill: parent
                                source: parent
                                color: currentView === 0 ? "#ffffff" : textSecondary
                            }
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
                        flat: true

                        background: Rectangle {
                            color: {
                                if (currentView === 1) {
                                    return playerViewBtn.hovered ? hoverBg : Qt.darker(hoverBg, 1.1)
                                }
                                return playerViewBtn.hovered ? hoverBg : "transparent"
                            }
                            radius: 6
                        }

                        contentItem: Image {
                            anchors.centerIn: parent
                            width: 20
                            height: 20
                            sourceSize: Qt.size(width, height)

                            source: "qrc:/resources/icons/camera-video-symbolic.svg"
                            fillMode: Image.PreserveAspectFit
                            smooth: true

                            ColorOverlay {
                                anchors.fill: parent
                                source: parent
                                color: currentView === 1 ? "#ffffff" : textSecondary
                            }
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

                Item { Layout.fillWidth: true }

                // Search Bar
                Rectangle {
                    Layout.preferredWidth: 280
                    height: 32
                    radius: 8
                    color: darkMode ? "#3a3a3a" : "#ececec"
                    border.color: searchField.activeFocus ? accentBlue : "transparent"
                    border.width: 2

                    Behavior on border.color {
                        ColorAnimation { duration: 150 }
                    }

                    Image {
                        id: searchIcon
                        anchors.left: parent.left
                        anchors.leftMargin: 10
                        anchors.verticalCenter: parent.verticalCenter
                        width: 14
                        height: 14
                        source: "qrc:/resources/icons/system-search-symbolic.svg"
                        fillMode: Image.PreserveAspectFit
                        opacity: 0.45

                        ColorOverlay {
                            anchors.fill: parent
                            source: parent
                            color: currentView === 1 ? "#ffffff" : textSecondary
                        }
                    }

                    TextField {
                        id: searchField
                        anchors.left: searchIcon.right
                        anchors.leftMargin: 6
                        anchors.right: clearBtn.visible ? clearBtn.left : parent.right
                        anchors.rightMargin: 8
                        anchors.verticalCenter: parent.verticalCenter
                        height: parent.height

                        placeholderText: "Search captures…"
                        font.pixelSize: 13
                        color: textPrimary
                        verticalAlignment: TextInput.AlignVCenter

                        leftPadding: 0
                        rightPadding: 0
                        topPadding: 0
                        bottomPadding: 0

                        background: Item {}

                        onTextChanged: root.searchText = text
                    }

                    Item {
                        id: clearBtn
                        anchors.right: parent.right
                        anchors.rightMargin: 8
                        anchors.verticalCenter: parent.verticalCenter
                        width: 14
                        height: 14
                        visible: searchField.text.length > 0

                        Image {
                            anchors.centerIn: parent
                            width: 16
                            height: 16
                            source: "qrc:/resources/icons/window-close-symbolic.svg"
                            fillMode: Image.PreserveAspectFit
                            opacity: clearArea.containsMouse ? 1.0 : 0.45

                            Behavior on opacity {
                                NumberAnimation { duration: 120 }
                            }

                            ColorOverlay {
                                anchors.fill: parent
                                source: parent
                                color: currentView === 1 ? "#ffffff" : textSecondary
                            }
                        }

                        MouseArea {
                            id: clearArea
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                searchField.text = ""
                                root.searchText = ""
                            }
                        }
                    }
                }

                Item { Layout.fillWidth: true }

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
                function onVideoSelected(dataFilePath) {
                    playerPage.loadDataForVideo(dataFilePath)
                }
            }
        }
    }
}
