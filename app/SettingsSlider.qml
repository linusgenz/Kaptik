import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import App 1.0

Item {
    id: root
    height: 36
    width: 300

    property int settingsKey
    property int from: 0
    property int to: 100
    property string unit: "%"
    property int stepSize: 1
    property int currentValue: 0
    property bool liveEnabled: true

    enabled: liveEnabled
    opacity: enabled ? 1.0 : 0.4

    Component.onCompleted: {
        currentValue = Settings.value(settingsKey)
    }

    RowLayout {
        anchors.fill: parent
        spacing: 12

        Slider {
            id: slider
            Layout.fillWidth: true
            from: root.from
            to: root.to
            stepSize: root.stepSize
            value: root.currentValue

            onValueChanged: {
                root.currentValue = Math.round(value)
                Settings.setValue(root.settingsKey, root.currentValue)
            }

            background: Rectangle {
                x: slider.leftPadding
                y: slider.topPadding + slider.availableHeight / 2 - height / 2
                implicitWidth: 200
                implicitHeight: slider.hovered && enabled || enabled && slider.pressed ? 8 : 4
                width: slider.availableWidth
                height: implicitHeight
                radius: 2
                color: borderColor

                Rectangle {
                    anchors.left: parent.left
                    width: slider.visualPosition * parent.width
                    height: parent.height
                    radius: 2
                    color: accentBlue
                }

                Behavior on height {
                    NumberAnimation { duration: 150; easing.type: Easing.OutQuad }
                }
            }

            handle: Rectangle {
                x: slider.leftPadding + slider.visualPosition * (slider.availableWidth - width)
                y: slider.topPadding + slider.availableHeight / 2 - height / 2
                width: 16
                height: 16
                radius: 8
                color: accentBlue
            }
        }

        Label {
            text: Math.round(slider.value) + unit
            width: 44
            horizontalAlignment: Text.AlignRight
            color: textSecondary
            font.pixelSize: 13
        }
    }

    Connections {
        target: Settings
        function onSettingChanged(key, value) {
            if (key === root.settingsKey) {
                root.currentValue = value
            }
        }
    }
}
