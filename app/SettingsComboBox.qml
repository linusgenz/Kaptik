import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import App 1.0

ComboBox {
    id: control

    property string settingsKey: ""
    property int itemHeight: 38

    width: 220
    textRole: "text"
    hoverEnabled: true

    Component.onCompleted: {
        if (settingsKey !== "") {
            let current = Settings.value(settingsKey)
            for (let i = 0; i < model.length; ++i) {
                if (model[i].value === current) {
                    currentIndex = i
                    break
                }
            }
        }
    }

    onActivated: function(index) {
        if (settingsKey !== "") {
            Settings.setValue(settingsKey, model[index].value)
        }
    }

    palette.buttonText: textPrimary
    font.pixelSize: 14
    topPadding: 4
    bottomPadding: 4

    background: Rectangle {
        color: control.hovered ? hoverBg : bgTertiary
        radius: 6
        border.width: 1
        border.color: borderColor
    }

    MouseArea {
        anchors.fill: parent
        cursorShape: Qt.PointingHandCursor
        acceptedButtons: Qt.NoButton
    }

    contentItem: Text {
        leftPadding: 12
        rightPadding: control.indicator.width + 12
        text: control.displayText
        font: control.font
        color: textPrimary
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
    }

    delegate: ItemDelegate {
        id: delegateItem
        width: control.width
        height: control.itemHeight
        hoverEnabled: true
        highlighted: control.highlightedIndex === index || hovered

        contentItem: Text {
            text: modelData.text
            color: textPrimary
            font: control.font
            elide: Text.ElideRight
            verticalAlignment: Text.AlignVCenter
            leftPadding: 12
            z: 2
        }

        MouseArea {
            anchors.fill: parent
            cursorShape: Qt.PointingHandCursor
            acceptedButtons: Qt.NoButton
        }

        background: Rectangle {
            color: bgSecondary

            Rectangle {
                anchors.fill: parent
                anchors.margins: 2
                color: hoverBg
                radius: 4
                opacity: delegateItem.highlighted ? 1 : 0
                Behavior on opacity { NumberAnimation { duration: 100 } }
            }

            Rectangle {
                width: 3
                height: parent.height * 0.6
                anchors.left: parent.left
                anchors.leftMargin: 2
                anchors.verticalCenter: parent.verticalCenter
                color: accentBlue
                visible: delegateItem.highlighted
                radius: 2
            }
        }
    }

    popup: Popup {
        y: control.height + 5
        width: control.width
        implicitHeight: Math.min(5, control.count) * control.itemHeight + 8
        padding: 4

        contentItem: ListView {
            clip: true
            implicitHeight: contentHeight + 1
            model: control.popup.visible ? control.delegateModel : null
            currentIndex: control.highlightedIndex
            boundsBehavior: Flickable.StopAtBounds
            interactive: contentHeight > height
            spacing: 0
        }

        background: Rectangle {
            color: bgSecondary
            border.width: 1
            border.color: borderColor
            radius: 8
        }
    }
}
