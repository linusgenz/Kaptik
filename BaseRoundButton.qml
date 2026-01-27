// BaseRoundButton.qml
import QtQuick 2.15
import QtQuick.Controls 2.15

RoundButton {
    id: baseButton
    property string iconSource
    property int iconWidth: 16
    property int iconHeight: 16
    property bool buttonEnabled: true

    enabled: buttonEnabled
    hoverEnabled: true
    flat: true

    width: 36
    height: 36

    background: Rectangle {
        anchors.fill: parent
        radius: width / 2
        implicitWidth: 36
        implicitHeight: 36
        color: hoverBg
        opacity: baseButton.hovered ? 1 : 0


        Behavior on opacity {
            NumberAnimation {
                duration: 100
                easing.type: Easing.InOutQuad
            }
        }
    }

    icon.source: iconSource
    icon.width: iconWidth
    icon.height: iconHeight
    icon.color: textPrimary

}
