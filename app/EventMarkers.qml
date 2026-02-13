import QtQuick 2.15
import QtQuick.Controls 2.15
import Qt5Compat.GraphicalEffects
import QtQuick.Layouts 1.15

Item {
    id: root

    property var eventData: []
    property real duration: 0
    property real currentPosition: 0

    signal seekToEvent(real timestamp)

    function getEventIcon(eventType) {
        switch(eventType) {
            case "Kill":
                return "qrc:/resources/icons/swords-symbolic.svg"
            case "Death":
                return "qrc:/resources/icons/skull-symbolic.svg"
            case "Assist":
                return "qrc:/resources/icons/handshake-symbolic.svg"
            default:
                return "qrc:/resources/icons/star-symbolic.svg"
        }
    }

    // Event markers
    Repeater {
        model: eventData

        delegate: Item {
            id: eventMarker

            property var event: modelData
            property real eventX: duration > 0 ? (event.timestamp / duration) * parent.width : 0

            x: eventX - 10
            y: 0
            width: 20
            height: parent.height

            visible: duration > 0 && event.timestamp >= 0 && event.timestamp <= duration

            // Vertical line on timeline (at slider position)
            Rectangle {
                anchors.horizontalCenter: parent.horizontalCenter
                anchors.verticalCenter: parent.verticalCenter
                width: 2
                height: 8
                color: textSecondary
                opacity: 0.6
            }

            // Event icon above timeline
            Item {
                id: eventIconContainer
                anchors.horizontalCenter: parent.horizontalCenter
                anchors.bottom: parent.verticalCenter
                anchors.bottomMargin: 16
                width: 20
                height: 20

                // Hover background
                Rectangle {
                    anchors.centerIn: parent
                    width: 28
                    height: 28
                    radius: 14
                    color: hoverBg
                    opacity: iconMouseArea.containsMouse ? 1 : 0

                    Behavior on opacity {
                        NumberAnimation { duration: 150; easing.type: Easing.OutQuad }
                    }
                }

                // Icon
                Image {
                    id: eventIcon
                    anchors.centerIn: parent
                    width: 24
                    height: 24
                    source: getEventIcon(event.event_type)
                    smooth: true
                    fillMode: Image.PreserveAspectFit

                    ColorOverlay {
                        anchors.fill: parent
                        source: parent
                        color: Qt.color("#ababab")
                    }
                }

                Behavior on scale {
                    NumberAnimation { duration: 150; easing.type: Easing.OutQuad }
                }
            }

            // Mouse area for icon click and hover
            MouseArea {
                id: iconMouseArea
                anchors.centerIn: eventIconContainer
                width: 28
                height: 28
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor

                onEntered: {
                    eventTooltip.visible = true
                    eventIconContainer.scale = 1.1
                }
                onExited: {
                    eventTooltip.visible = false
                    eventIconContainer.scale = 1.0
                }

                onClicked: {
                    var seekTime = Math.max(0, event.timestamp - 5000)
                    root.seekToEvent(seekTime)
                }
            }

            // Event tooltip
            Rectangle {
                id: eventTooltip
                visible: false
                x: {
                    var tooltipX = -width/2 + parent.width/2
                    return Math.min(Math.max(tooltipX, -eventMarker.x), root.width - eventMarker.x - width)
                }
                anchors.bottom: eventIconContainer.top
                anchors.bottomMargin: 12
                width: tooltipContent.width + 20
                height: tooltipContent.height + 16
                color: darkMode ? "#2a2a2a" : "#ffffff"
                radius: 8
                border.color: darkMode ? "#404040" : "#e0e0e0"
                border.width: 1

                layer.enabled: true
                layer.effect: DropShadow {
                    transparentBorder: true
                    horizontalOffset: 0
                    verticalOffset: 2
                    radius: 8
                    samples: 17
                    color: "#40000000"
                }

                RowLayout {
                    id: tooltipContent
                    anchors.centerIn: parent
                    spacing: 10

                    // Icon in tooltip
                    Image {
                        Layout.preferredWidth: 24
                        Layout.preferredHeight: 24
                        Layout.minimumWidth: 24
                        Layout.minimumHeight: 24
                        source: getEventIcon(event.event_type)
                        smooth: true
                        fillMode: Image.PreserveAspectFit

                        ColorOverlay {
                            anchors.fill: parent
                            source: parent
                            color: textSecondary
                        }
                    }

                    // Time
                    Label {
                        text: formatEventTime(event.timestamp)
                        font.pixelSize: 12
                        color: textPrimary
                    }

                    // Separator
                    Label {
                        text: "-"
                        font.pixelSize: 12
                        color: textSecondary
                    }

                    // Event name/type
                    Label {
                        text: event.name || event.event_type
                        font.pixelSize: 12
                        color: textPrimary
                    }
                }

                // Arrow pointing to icon
                Canvas {
                    anchors.top: parent.bottom
                    anchors.horizontalCenter: parent.horizontalCenter
                    width: 10
                    height: 6

                    onPaint: {
                        var ctx = getContext("2d")
                        ctx.clearRect(0, 0, width, height)
                        ctx.fillStyle = darkMode ? "#2a2a2a" : "#ffffff"
                        ctx.beginPath()
                        ctx.moveTo(0, 0)
                        ctx.lineTo(width, 0)
                        ctx.lineTo(width/2, height)
                        ctx.closePath()
                        ctx.fill()
                    }
                }
            }

            function formatEventTime(ms) {
                var totalSeconds = Math.floor(ms / 1000)
                var minutes = Math.floor(totalSeconds / 60)
                var seconds = totalSeconds % 60
                return minutes + ":" + (seconds < 10 ? "0" + seconds : seconds)
            }
        }
    }
}
