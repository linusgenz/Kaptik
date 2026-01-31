import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import QtMultimedia 6.5
import App 1.0

RowLayout {
    spacing: 0

    property alias mediaPlayer: mediaPlayer
    property alias videoPlayerArea: videoPlayerArea

    // Sidebar
    Rectangle {
        Layout.preferredWidth: 280
        Layout.fillHeight: true
        color: bgSecondary

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 16
            spacing: 12

            Label {
                text: "Captures"
                font.pixelSize: 13
                font.weight: Font.DemiBold
                color: textSecondary
                Layout.bottomMargin: 4
            }

            ListView {
                id: captureList
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                spacing: 4

                model: clipModel

                delegate: ItemDelegate {
                    width: captureList.width

                    property bool isCurrent: index === root.currentVideoIndex

                    background: Rectangle {
                        color: isCurrent
                               ? Qt.rgba(accentBlue.r, accentBlue.g, accentBlue.b, darkMode ? 0.25 : 0.18)
                               : (parent.hovered ? hoverBg : "transparent")
                        radius: 6
                        //   Behavior on color { ColorAnimation { duration: 120 } }
                    }

                    MouseArea {
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor

                        onClicked: {
                            root.currentVideoSource = model.path
                            root.currentVideoIndex = index
                        }
                    }

                    contentItem: ColumnLayout {
                        spacing: 4

                        Label {
                            text: model.name
                            font.pixelSize: 14
                            color: textPrimary
                            elide: Text.ElideRight
                            Layout.fillWidth: true
                        }

                        RowLayout {
                            spacing: 8

                            Label {
                                text: model.duration
                                font.pixelSize: 12
                                color: textSecondary
                            }

                            Label {
                                text: "•"
                                font.pixelSize: 12
                                color: textSecondary
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

        Rectangle {
            anchors.right: parent.right
            width: 1
            height: parent.height
            color: borderColor
        }
    }

    // Video Player Area
    Rectangle {
        id: videoPlayerArea
        Layout.fillWidth: true
        Layout.fillHeight: true
        color: bgPrimary

        focus: true
        Keys.enabled: root.currentView === 1

        Keys.onPressed: (event) => {
                            if (root.currentVideoSource === "") return

                            switch (event.key) {
                                case Qt.Key_Space:
                                if (mediaPlayer.playbackState === MediaPlayer.PlayingState) {
                                    mediaPlayer.pause()
                                } else {
                                    mediaPlayer.play()
                                }

                                event.accepted = true
                                break
                                case Qt.Key_Left:
                                mediaPlayer.position = Math.max(0, mediaPlayer.position - 5000) // -5s
                                event.accepted = true
                                break
                                case Qt.Key_Right:
                                mediaPlayer.position = Math.min(mediaPlayer.duration, mediaPlayer.position + 5000) // +5s
                                event.accepted = true
                                break
                            }
                        }

        // MediaPlayer backend
        MediaPlayer {
            id: mediaPlayer
            videoOutput: videoOutput
            audioOutput: AudioOutput { id: audioOutput }

            source: root.currentVideoSource

            onPositionChanged: {
                if (!progressSlider.pressed) {
                    progressSlider.value = duration > 0 ? (position / duration * 100) : 0
                }
            }
        }

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 24
            spacing: 16

            Rectangle {
                Layout.fillWidth: true
                Layout.fillHeight: true
                color: darkMode ? "#1a1a1a" : "#2a2a2a"
                radius: 12
                clip: true

                MouseArea {
                    anchors.fill: parent
                    enabled: root.currentVideoSource !== ""
                    cursorShape: Qt.PointingHandCursor

                    onClicked: {
                        if (mediaPlayer.playbackState === MediaPlayer.PlayingState) {
                            mediaPlayer.pause()
                            pauseOverlay.showOnce()
                        } else {
                            mediaPlayer.play()
                        }
                    }
                }

                VideoOutput {
                    id: videoOutput
                    anchors.fill: parent
                    visible: mediaPlayer.mediaStatus !== MediaPlayer.NoMedia
                }

                Image {
                    anchors.fill: parent
                    source: root.currentVideoIndex >= 0
                            ? "image://thumbnails/" + root.currentVideoIndex
                            : ""
                    fillMode: Image.PreserveAspectFit
                    visible: mediaPlayer.playbackState === MediaPlayer.StoppedState
                             && mediaPlayer.mediaStatus === MediaPlayer.LoadedMedia
                    asynchronous: true
                }

                // Play Overlay
                Rectangle {
                    id: playOverlay
                    width: 96; height: 96
                    anchors.centerIn: parent
                    radius: width/2
                    color: "#66000000"
                    opacity: videoSelectedNotPlaying ? 1 : 0
                    scale: videoSelectedNotPlaying ? 1 : 0.5

                    Behavior on opacity {
                        NumberAnimation {
                            duration: videoHasBeenPlayed ? 300 : 0
                            easing.type: Easing.InOutQuad
                        }
                    }
                    Behavior on scale {
                        NumberAnimation {
                            duration: videoHasBeenPlayed ? 300 : 0
                            easing.type: Easing.OutBack
                        }
                    }

                    Image  {
                        anchors.centerIn: parent
                        width: 36
                        height: 36
                        sourceSize.width: 64
                        sourceSize.height: 64
                        source: "qrc:/resources/icons/media-playback-start-symbolic.svg"
                        fillMode: Image.PreserveAspectFit
                    }

                    function showTemporarily() {
                        playTimer.stop()
                        videoSelectedNotPlaying = true
                        playTimer.restart()
                    }

                    Timer {
                        id: playTimer
                        interval: 800
                        running: false
                        repeat: false
                        onTriggered: {
                            videoSelectedNotPlaying = false
                        }
                    }

                    Connections {
                        target: mediaPlayer

                        function onPlaybackStateChanged() {
                            if (mediaPlayer.playbackState === MediaPlayer.PlayingState) {
                                if (!videoHasBeenPlayed) {
                                    videoHasBeenPlayed = true
                                    videoSelectedNotPlaying = false
                                } else {
                                    playOverlay.showTemporarily()
                                }
                            }
                        }
                    }
                }

                // Pause Overlay
                Rectangle {
                    id: pauseOverlay
                    width: 96; height: 96
                    anchors.centerIn: parent
                    radius: width/2
                    color: "#66000000"
                    opacity: 0
                    scale: 0.5

                    Behavior on opacity {
                        NumberAnimation {
                            duration: 300
                            easing.type: Easing.InOutQuad
                        }
                    }
                    Behavior on scale {
                        NumberAnimation {
                            duration: 300
                            easing.type: Easing.OutBack
                        }
                    }

                    Image  {
                        anchors.centerIn: parent
                        width: 36
                        height: 36
                        sourceSize.width: 64
                        sourceSize.height: 64
                        source: "qrc:/resources/icons/media-playback-pause-symbolic.svg"
                        fillMode: Image.PreserveAspectFit
                        smooth: true
                        layer.enabled: true
                    }

                    function showOnce() {
                        opacity = 1
                        scale = 1
                        pauseTimer.restart()
                    }

                    Timer {
                        id: pauseTimer
                        interval: 800
                        running: false
                        repeat: false
                        onTriggered: {
                            pauseOverlay.opacity = 0
                            pauseOverlay.scale = 0.5
                        }
                    }
                }

                Label {
                    anchors.centerIn: parent
                    visible: root.currentVideoSource === ""
                    text: "Select a capture to view"
                    font.pixelSize: 16
                    color: "#808080"
                }
            }

            // Video controls
            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 72
                color: bgSecondary
                radius: 12

                RowLayout {
                    anchors.fill: parent
                    anchors.margins: 12
                    spacing: 12

                    BaseRoundButton {
                        width: 24
                        height: 24
                        iconWidth: 16
                        iconHeight: 16
                        iconSource: mediaPlayer.playbackState === MediaPlayer.PlayingState
                                    ? "qrc:/resources/icons/media-playback-pause-symbolic.svg"
                                    : "qrc:/resources/icons/media-playback-start-symbolic.svg"
                        buttonEnabled: root.currentVideoSource !== ""

                        onClicked: {
                            if (mediaPlayer.playbackState === MediaPlayer.PlayingState) {
                                mediaPlayer.pause()
                                pauseOverlay.showOnce()
                            } else {
                                mediaPlayer.play()
                            }
                        }
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        Layout.alignment: Qt.AlignVCenter
                        spacing: 12

                        Label {
                            id: currentTimeLabel
                            text: formatTime(progressSlider.seeking ? progressSlider.seekPosition : mediaPlayer.position)
                            font.pixelSize: 14
                            color: textSecondary

                            function formatTime(ms) {
                                if (!ms || ms < 0) return "0:00"
                                var totalSeconds = Math.floor(ms / 1000)
                                var minutes = Math.floor(totalSeconds / 60)
                                var seconds = totalSeconds % 60
                                return minutes + ":" + (seconds < 10 ? "0" + seconds : seconds)
                            }
                        }

                        Slider {
                            id: progressSlider
                            Layout.fillWidth: true
                            Layout.alignment: Qt.AlignVCenter
                            from: 0
                            to: 100
                            value: 0
                            enabled: root.currentVideoSource !== ""
                            hoverEnabled: true

                            property bool seeking: false
                            property bool wasPlaying: false
                            property real seekPosition: 0

                            Connections {
                                target: mediaPlayer
                                function onPositionChanged() {
                                    if (!progressSlider.seeking && mediaPlayer.duration > 0) {
                                        progressSlider.value = (mediaPlayer.position / mediaPlayer.duration) * 100
                                    }
                                }
                            }

                            onPressedChanged: {
                                if (pressed) {
                                    seeking = true
                                    wasPlaying = mediaPlayer.playbackState === MediaPlayer.PlayingState

                                    if (wasPlaying) {
                                        mediaPlayer.pause()
                                    }
                                } else {
                                    seeking = false

                                    if (wasPlaying) {
                                        mediaPlayer.play()
                                    }
                                }
                            }

                            onValueChanged: {
                                if (seeking && mediaPlayer.duration > 0) {
                                    seekPosition = (value / 100) * mediaPlayer.duration
                                }
                            }

                            onMoved: {
                                if (mediaPlayer.duration > 0) {
                                    // Update position in real-time while dragging
                                    mediaPlayer.position = (value / 100) * mediaPlayer.duration
                                }
                            }

                            background: Rectangle {
                                x: progressSlider.leftPadding
                                y: progressSlider.topPadding + progressSlider.availableHeight / 2 - height / 2
                                width: progressSlider.availableWidth
                                height: progressSlider.hovered || progressSlider.pressed ? 8 : 4
                                radius: height / 2
                                color: darkMode ? "#404040" : "#d5d3cf"

                                Behavior on height {
                                    NumberAnimation { duration: 150; easing.type: Easing.OutQuad }
                                }

                                Rectangle {
                                    width: progressSlider.visualPosition * parent.width
                                    height: parent.height
                                    color: accentBlue
                                    radius: height / 2
                                }
                            }

                            handle: Rectangle {
                                x: progressSlider.leftPadding + progressSlider.visualPosition * (progressSlider.availableWidth - width)
                                y: progressSlider.topPadding + progressSlider.availableHeight / 2 - height / 2
                                implicitWidth: progressSlider.pressed ? 20 : 16
                                implicitHeight: progressSlider.pressed ? 20 : 16
                                radius: width / 2
                                color: accentBlue
                                border.color: accentBlue
                                border.width: 2
                                visible: progressSlider.enabled
                                scale: progressSlider.pressed ? 1.1 : 1

                                Behavior on implicitWidth {
                                    NumberAnimation { duration: 150; easing.type: Easing.OutQuad }
                                }
                                Behavior on implicitHeight {
                                    NumberAnimation { duration: 150; easing.type: Easing.OutQuad }
                                }
                                Behavior on scale {
                                    NumberAnimation { duration: 100; easing.type: Easing.OutQuad }
                                }
                            }
                        }

                        Label {
                            id: totalTimeLabel
                            text: formatTime(mediaPlayer.duration)
                            font.pixelSize: 14
                            color: textSecondary

                            function formatTime(ms) {
                                if (!ms || ms < 0) return "0:00"
                                var totalSeconds = Math.floor(ms / 1000)
                                var minutes = Math.floor(totalSeconds / 60)
                                var seconds = totalSeconds % 60
                                return minutes + ":" + (seconds < 10 ? "0" + seconds : seconds)
                            }
                        }
                    }

                    BaseRoundButton {
                        iconSource: audioOutput.muted
                                    ? "qrc:/resources/icons/audio-volume-muted-symbolic.svg"
                                    : "qrc:/resources/icons/audio-volume-high-symbolic.svg"
                        buttonEnabled: root.currentVideoSource !== ""

                        onClicked: audioOutput.muted = !audioOutput.muted
                    }
                }
            }
        }
    }
}
