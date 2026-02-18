import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import Qt5Compat.GraphicalEffects
import QtQml.Models 2.15
import App 1.0

Item {
    ScrollView {
        anchors.fill: parent
        anchors.margins: 24
        clip: true
        visible: clipModel.count > 0

        GridView {
            id: gridView
            cellWidth: 320
            cellHeight: 240

            model: filteredModel

            DelegateModel {
                id: filteredModel
                model: clipModel
                filterOnGroup: "included"

                groups: [
                    DelegateModelGroup {
                        name: "included"
                        includeByDefault: true
                    }
                ]

                function applyFilter() {
                    for (var i = 0; i < items.count; i++) {
                        var item = items.get(i)
                        var name = item.model.name ?? ""
                        var match = root.searchText === "" || name.toLowerCase().indexOf(root.searchText.toLowerCase()) !== -1

                        if (match && !item.inIncluded) {
                            item.groups = ["items", "included"]
                        } else if (!match && item.inIncluded) {
                            item.groups = ["items"]
                        }
                    }
                }

                Component.onCompleted: applyFilter()
            }

            Connections {
                target: root
                function onSearchTextChanged() { filteredModel.applyFilter() }
            }

            delegate: Item {
                width: 320
                height: 240
                clip: true

                Rectangle {
                    id: card
                    anchors.fill: parent
                    anchors.margins: 12
                    radius: 12
                    color: bgSecondary
                    scale: mouseArea.containsMouse ? 1.015 : 1.0

                    layer.enabled: true
                    layer.smooth: true

                    Behavior on scale {
                        NumberAnimation { duration: 120; easing.type: Easing.OutQuad }
                    }

                    MouseArea {
                        id: mouseArea
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor

                        onClicked: {
                            root.currentVideoSource = model.path
                            root.currentVideoIndex = model.index
                            root.currentView = 1
                            root.videoSelected(model.dataFilePath)
                        }
                    }

                    ColumnLayout {
                        anchors.fill: parent
                        spacing: 0

                        Item {
                            Layout.fillWidth: true
                            Layout.preferredHeight: 150

                            OpacityMask {
                                id: mask
                                anchors.fill: parent

                                source: Image {
                                    anchors.fill: parent
                                    source: index >= 0 ? "image://thumbnails/" + index : ""
                                    fillMode: Image.PreserveAspectCrop
                                    asynchronous: true
                                    cache: true
                                }

                                maskSource: Rectangle {
                                    width: mask.width
                                    height: mask.height

                                    radius: 12
                                    color: "white"

                                    Rectangle {
                                        anchors.bottom: parent.bottom
                                        width: parent.width
                                        height: radius
                                        color: "white"
                                    }
                                }
                            }

                            // Gradient for readability
                            Rectangle {
                                anchors.left: parent.left
                                anchors.right: parent.right
                                anchors.bottom: parent.bottom
                                height: parent.height * 0.45
                                gradient: Gradient {
                                    GradientStop { position: 0.0; color: "#00000000" }
                                    GradientStop { position: 0.6; color: "#66000000" }
                                    GradientStop { position: 1.0; color: "#CC000000" }
                                }
                            }

                            // KDA badge
                            Rectangle {
                                id: kdaBadge
                                visible: model.kda !== undefined && model.kda !== null

                                anchors.left: parent.left
                                anchors.bottom: parent.bottom
                                anchors.margins: 8
                                height: 24
                                width: kdaLabel.implicitWidth + 16
                                radius: 4
                                color: "#000000"
                                opacity: 0.85

                                Label {
                                    id: kdaLabel
                                    anchors.centerIn: parent
                                    text: {
                                        var kda = model.kda
                                        if (!kda) return ""
                                        return kda.kills + "/" + kda.deaths + "/" + kda.assists
                                    }
                                    font.pixelSize: 12
                                    color: "#ffffff"
                                }
                            }

                            Rectangle {
                                visible: model.game_outcome !== undefined && model.game_outcome !== ""
                                anchors.left: kdaBadge.right
                                anchors.bottom: kdaBadge.bottom
                                anchors.leftMargin: 4
                                height: 24
                                width: badgeLabel.implicitWidth + 16
                                radius: 4
                                color: "#000000"
                                opacity: 0.85

                                Label {
                                    id: badgeLabel
                                    anchors.centerIn: parent
                                    text: model.game_outcome
                                    font.pixelSize: 12
                                    font.bold: true
                                    font.capitalization: Font.AllUppercase
                                    color: {
                                        switch (model.game_outcome) {
                                        case "Victory":
                                            return "#00FF00";
                                        case "Defeat":
                                            return "#FF0000";
                                        case "Draw":
                                            return "#AAAAAA";
                                        default:
                                            return "#FFFFFF";
                                        }
                                    }
                                }
                            }

                            // Duration badge
                            Rectangle {
                                anchors.right: parent.right
                                anchors.bottom: parent.bottom
                                anchors.margins: 8
                                height: 24
                                width: durationLabel.implicitWidth + 16
                                radius: 4
                                color: "#000000"
                                opacity: 0.85

                                Label {
                                    id: durationLabel
                                    anchors.centerIn: parent
                                    text: model.duration
                                    font.pixelSize: 12
                                    color: "#ffffff"
                                }
                            }

                            // ▶ Play overlay on hover
                            Rectangle {
                                anchors.centerIn: parent
                                width: 44
                                height: 44
                                radius: 22
                                color: "#80000000"
                                opacity: mouseArea.containsMouse ? 1 : 0

                                Behavior on opacity {
                                    NumberAnimation { duration: 120 }
                                }

                                Image {
                                    anchors.centerIn: parent
                                    source: "qrc:/resources/icons/media-playback-start-symbolic.svg"
                                    width: 18
                                    height: 18
                                    fillMode: Image.PreserveAspectFit
                                }
                            }
                        }

                        // 📝 Info section
                        ColumnLayout {
                            Layout.fillWidth: true
                            Layout.margins: 12
                            spacing: 6

                            Label {
                                text: model.name
                                font.pixelSize: 15
                                font.weight: Font.DemiBold
                                color: textPrimary
                                elide: Text.ElideRight
                                Layout.fillWidth: true
                            }

                            Label {
                                text: model.date
                                font.pixelSize: 12
                                color: textSecondary
                            }
                        }
                    }
                }
            }
        }
    }

    Item {
        anchors.centerIn: parent
        width: 300
        visible: gridView.count === 0

        ColumnLayout {
            anchors.centerIn: parent
            spacing: 10

            Text {
                text: "☹"
                font.pixelSize: 72
                font.family: "Segoe UI Emoji"
                color: textSecondary
                opacity: 0.5
                Layout.alignment: Qt.AlignHCenter
            }

            ColumnLayout {
                spacing: 8
                Layout.alignment: Qt.AlignHCenter

                Label {
                    text: "No captures matching this criteria"
                    font.pixelSize: 20
                    font.weight: Font.DemiBold
                    color: textPrimary
                    Layout.alignment: Qt.AlignHCenter
                }

                Label {
                    text: "Your captured videos will appear here"
                    font.pixelSize: 14
                    color: textSecondary
                    Layout.alignment: Qt.AlignHCenter
                }
            }
        }
    }
}
