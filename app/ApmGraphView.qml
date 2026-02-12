import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Shapes 1.15
import Qt5Compat.GraphicalEffects
import QtQuick.Layouts 1.15

Item {
    id: root

    property var apmData: []
    property var eventData: []
    property real duration: 0
    property real currentPosition: 0
    property color graphColor: accentBlue
    property color backgroundColor: "transparent"
    property color gridColor: darkMode ? "#2a2a2a" : "#e0e0e0"
    property bool showGrid: true
    property real maxApm: 300
    property real gameAverageApm: calculateAverageApm()

    property real currentApm: getCurrentApm()

    function calculateMaxApm() {
        if (apmData.length === 0) return 300
        var max = 0
        for (var i = 0; i < apmData.length; i++) {
            if (apmData[i].apm > max) {
                max = apmData[i].apm
            }
        }
        return Math.max(max * 1.1, 50)
    }

    function getCurrentApm() {
        if (apmData.length === 0 || duration <= 0) return 0

        // Find the closest data point or interpolate
        for (var i = 0; i < apmData.length - 1; i++) {
            if (currentPosition >= apmData[i].timestamp &&
                    currentPosition <= apmData[i + 1].timestamp) {

                // Linear interpolation
                var ratio = (currentPosition - apmData[i].timestamp) /
                        (apmData[i + 1].timestamp - apmData[i].timestamp)
                return apmData[i].apm + (apmData[i + 1].apm - apmData[i].apm) * ratio
            }
        }

        if (currentPosition < apmData[0].timestamp) {
            return apmData[0].apm
        }
        return apmData[apmData.length - 1].apm
    }

    function calculateAverageApm() {
        if (apmData.length === 0) return 0
        var sum = 0
        for (var i = 0; i < apmData.length; i++) {
            sum += apmData[i].apm
        }
        return sum / apmData.length
    }

    function getEventIcon(eventType) {
        switch(eventType) {
            case "Kill":
                return "qrc:/resources/icons/skull-symbolic.svg"
            case "Death":
                return "qrc:/resources/icons/cross-symbolic.svg"
            case "Assist":
                return "qrc:/resources/icons/assist-symbolic.svg"
            default:
                return "qrc:/resources/icons/star-symbolic.svg"
        }
    }

    function getEventColor(eventType) {
        switch(eventType) {
            case "Kill":
                return "#ef4444" // red
            case "Death":
                return "#8b5cf6" // purple
            case "Assist":
                return "#10b981" // green
            default:
                return accentBlue
        }
    }

    onApmDataChanged: {
        maxApm = calculateMaxApm()
        currentApm = getCurrentApm()
        canvas.requestPaint()
    }

    onCurrentPositionChanged: {
        currentApm = getCurrentApm()
        canvas.requestPaint()
    }

    Rectangle {
        anchors.fill: parent
        color: backgroundColor
        radius: 4
    }

    // APM Graph
    Canvas {
        id: canvas
        anchors.fill: parent

        onPaint: {
            if (apmData.length < 2 || duration <= 0) return

            var ctx = getContext("2d")
            ctx.clearRect(0, 0, width, height)

            function point(i) {
                return {
                    x: (apmData[i].timestamp / duration) * width,
                    y: height - ((apmData[i].apm / maxApm) * height)
                }
            }

            var smoothing = 0.25

            var gradient = ctx.createLinearGradient(0, 0, 0, height)
            gradient.addColorStop(0, Qt.rgba(graphColor.r, graphColor.g, graphColor.b, 0.3))
            gradient.addColorStop(1, Qt.rgba(graphColor.r, graphColor.g, graphColor.b, 0.05))

            ctx.fillStyle = gradient
            ctx.beginPath()
            ctx.moveTo(0, height)

            var p0 = point(0)
            ctx.lineTo(p0.x, p0.y)

            for (var i = 1; i < apmData.length; i++) {
                var p1 = point(i)
                var pPrev = point(i - 1)
                var pNext = i < apmData.length - 1 ? point(i + 1) : p1

                var cp1x = pPrev.x + (p1.x - p0.x) * smoothing
                var cp1y = pPrev.y + (p1.y - p0.y) * smoothing

                var cp2x = p1.x - (pNext.x - pPrev.x) * smoothing
                var cp2y = p1.y - (pNext.y - pPrev.y) * smoothing

                ctx.bezierCurveTo(cp1x, cp1y, cp2x, cp2y, p1.x, p1.y)
                p0 = p1
            }

            ctx.lineTo(width, height)
            ctx.lineTo(0, height)
            ctx.closePath()
            ctx.fill()

            ctx.strokeStyle = graphColor
            ctx.lineWidth = 2
            ctx.lineJoin = "round"
            ctx.lineCap = "round"

            ctx.beginPath()
            p0 = point(0)
            ctx.moveTo(p0.x, p0.y)

            for (i = 1; i < apmData.length; i++) {
                p1 = point(i)
                pPrev = point(i - 1)
                pNext = i < apmData.length - 1 ? point(i + 1) : p1

                cp1x = pPrev.x + (p1.x - p0.x) * smoothing
                cp1y = pPrev.y + (p1.y - p0.y) * smoothing

                cp2x = p1.x - (pNext.x - pPrev.x) * smoothing
                cp2y = p1.y - (pNext.y - pPrev.y) * smoothing

                ctx.bezierCurveTo(cp1x, cp1y, cp2x, cp2y, p1.x, p1.y)
                p0 = p1
            }

            ctx.stroke()
        }
    }

    // NEW: Event markers layer
    Repeater {
        model: eventData

        delegate: Item {
            id: eventMarker

            property var event: modelData
            property real eventX: duration > 0 ? (event.timestamp / duration) * parent.width : 0
            property color eventColor: getEventColor(event.event_type)

            x: eventX - 12
            y: 0
            width: 24
            height: parent.height

            visible: duration > 0 && event.timestamp >= 0 && event.timestamp <= duration

            // Vertical line
            Rectangle {
                anchors.horizontalCenter: parent.horizontalCenter
                y: 28
                width: 2
                height: parent.height - 28
                color: eventMarker.eventColor
                opacity: 0.4
            }

            // Event icon background
            Rectangle {
                id: eventIconBg
                anchors.horizontalCenter: parent.horizontalCenter
                y: 4
                width: 24
                height: 24
                radius: 12
                color: eventMarker.eventColor
                opacity: 0.9

                // Icon
                Image {
                    anchors.centerIn: parent
                    width: 14
                    height: 14
                    source: getEventIcon(event.event_type)
                    smooth: true
                    fillMode: Image.PreserveAspectFit

                    ColorOverlay {
                        anchors.fill: parent
                        source: parent
                        color: "white"
                    }
                }

                // Glow effect
                layer.enabled: true
                layer.effect: Glow {
                    radius: 8
                    samples: 17
                    color: eventMarker.eventColor
                    spread: 0.3
                }
            }

            // Hover area for tooltip
            MouseArea {
                anchors.fill: parent
                hoverEnabled: true

                onEntered: {
                    eventTooltip.visible = true
                    eventIconBg.scale = 1.15
                }
                onExited: {
                    eventTooltip.visible = false
                    eventIconBg.scale = 1.0
                }

                Behavior on scale {
                    NumberAnimation { duration: 150; easing.type: Easing.OutQuad }
                }
            }

            // Event tooltip
            Rectangle {
                id: eventTooltip
                visible: false
                x: {
                    var tooltipX = -width/2 + parent.width/2
                    // Keep within bounds
                    return Math.min(Math.max(tooltipX, -eventMarker.x), root.width - eventMarker.x - width)
                }
                y: -height - 8
                width: tooltipEventContent.width + 20
                height: tooltipEventContent.height + 16
                color: darkMode ? "#2a2a2a" : "#ffffff"
                radius: 8
                border.color: eventMarker.eventColor
                border.width: 2

                layer.enabled: true
                layer.effect: DropShadow {
                    transparentBorder: true
                    horizontalOffset: 0
                    verticalOffset: 2
                    radius: 8
                    samples: 17
                    color: "#40000000"
                }

                ColumnLayout {
                    id: tooltipEventContent
                    anchors.centerIn: parent
                    spacing: 6

                    Label {
                        text: event.event_type
                        font.pixelSize: 13
                        font.weight: Font.Bold
                        color: eventMarker.eventColor
                        Layout.alignment: Qt.AlignHCenter
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredHeight: 1
                        color: darkMode ? "#404040" : "#e0e0e0"
                        visible: event.name !== ""
                    }

                    Label {
                        text: event.name
                        font.pixelSize: 11
                        color: textPrimary
                        visible: event.name !== ""
                        Layout.alignment: Qt.AlignHCenter
                    }

                    Label {
                        text: formatEventTime(event.timestamp)
                        font.pixelSize: 10
                        color: textSecondary
                        Layout.alignment: Qt.AlignHCenter
                    }
                }

                // Arrow
                Canvas {
                    anchors.top: parent.bottom
                    anchors.horizontalCenter: parent.horizontalCenter
                    width: 10
                    height: 6

                    onPaint: {
                        var ctx = getContext("2d")
                        ctx.clearRect(0, 0, width, height)
                        ctx.fillStyle = eventMarker.eventColor
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

    // Hover tooltip (existing APM tooltip)
    MouseArea {
        id: hoverArea
        anchors.fill: parent
        hoverEnabled: true
        propagateComposedEvents: true

        property real hoveredApm: 0
        property real hoveredTime: 0

        onPositionChanged: {
            if (apmData.length === 0 || duration <= 0) return

            hoveredTime = (mouse.x / width) * duration

            // Find APM at hovered position
            for (var i = 0; i < apmData.length - 1; i++) {
                if (hoveredTime >= apmData[i].timestamp &&
                        hoveredTime <= apmData[i + 1].timestamp) {

                    var ratio = (hoveredTime - apmData[i].timestamp) /
                            (apmData[i + 1].timestamp - apmData[i].timestamp)
                    hoveredApm = apmData[i].apm + (apmData[i + 1].apm - apmData[i].apm) * ratio
                    break
                }
            }
        }

        // Vertikale Linie bei Hover
        Rectangle {
            visible: parent.containsMouse && apmData.length > 0
            x: parent.mouseX - width/2
            y: 0
            width: 2
            height: parent.height
            color: graphColor
            opacity: 0.6
        }

        // Punkt auf dem Graphen bei Hover
        Rectangle {
            visible: parent.containsMouse && apmData.length > 0
            x: parent.mouseX - width/2
            y: parent.height - ((hoverArea.hoveredApm / maxApm) * parent.height) - height/2
            width: 10
            height: 10
            radius: width / 2
            color: graphColor
            border.color: darkMode ? "#1a1a1a" : "#ffffff"
            border.width: 2
        }

        // Tooltip
        Rectangle {
            visible: parent.containsMouse && apmData.length > 0
            x: {
                var tooltipX = parent.mouseX - width/2
                return Math.min(Math.max(tooltipX, 0), parent.width - width)
            }
            y: -height - 12
            width: tooltipContent.width + 20
            height: tooltipContent.height + 16
            color: darkMode ? "#2a2a2a" : "#ffffff"
            radius: 8
            border.color: darkMode ? "#404040" : "#d5d3cf"
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

            ColumnLayout {
                id: tooltipContent
                anchors.centerIn: parent
                spacing: 6

                RowLayout {
                    spacing: 8
                    Layout.alignment: Qt.AlignLeft

                    Label {
                        text: "APM"
                        font.pixelSize: 11
                        color: textSecondary
                    }

                    Label {
                        text: Math.round(hoverArea.hoveredApm)
                        font.pixelSize: 14
                        font.weight: Font.Bold
                        color: graphColor
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 1
                    color: darkMode ? "#404040" : "#e0e0e0"
                }

                RowLayout {
                    spacing: 8
                    Layout.alignment: Qt.AlignLeft

                    Label {
                        text: "Average"
                        font.pixelSize: 11
                        color: textSecondary
                    }

                    Label {
                        text: Math.round(gameAverageApm)
                        font.pixelSize: 12
                        color: textPrimary
                    }
                }
            }

            // Arrow
            Canvas {
                anchors.top: parent.bottom
                anchors.horizontalCenter: parent.horizontalCenter
                width: 10
                height: 6

                onPaint: {
                    var ctx = getContext("2d")
                    ctx.clearRect(0, 0, width, height)
                    ctx.fillStyle = parent.color
                    ctx.beginPath()
                    ctx.moveTo(0, 0)
                    ctx.lineTo(width, 0)
                    ctx.lineTo(width/2, height)
                    ctx.closePath()
                    ctx.fill()
                }
            }
        }
    }
}
