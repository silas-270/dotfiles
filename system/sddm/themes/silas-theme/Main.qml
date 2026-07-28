import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Qt5Compat.GraphicalEffects

Rectangle {
    id: container
    width: 1920
    height: 1080
    color: "black"

    // 1. Hintergrundbild
    Image {
        id: background
        source: "wallpaper.png"
        anchors.fill: parent
        fillMode: Image.PreserveAspectCrop
    }

    // 2. Schnee-Animation
    Canvas {
        id: snowCanvas
        anchors.fill: parent

        property var flakes: []
        property int flakeCount: 120

        // Schneeflocken beim Start initialisieren
        Component.onCompleted: {
            var arr = []
            for (var i = 0; i < flakeCount; i++) {
                arr.push({
                    x:     Math.random() * width,
                    y:     Math.random() * height,
                    r:     1.5 + Math.random() * 3.5,
                    speed: 0.4 + Math.random() * 1.2,
                    drift: (Math.random() - 0.5) * 0.6,
                    alpha: 0.35 + Math.random() * 0.55,
                    phase: Math.random() * Math.PI * 2
                })
            }
            flakes = arr
        }

        Timer {
            id: snowTimer
            interval: 16   // ~60 fps
            running: true
            repeat: true
            onTriggered: {
                var arr = snowCanvas.flakes
                var h   = snowCanvas.height
                var w   = snowCanvas.width
                var t   = Date.now() / 1000

                for (var i = 0; i < arr.length; i++) {
                    var f = arr[i]
                    f.y += f.speed
                    f.x += f.drift + Math.sin(t * 0.8 + f.phase) * 0.3

                    // Zurück nach oben wenn unten raus
                    if (f.y > h + f.r) {
                        f.y = -f.r
                        f.x = Math.random() * w
                    }
                    // Seitenränder wrappen
                    if (f.x > w + f.r)  f.x = -f.r
                    if (f.x < -f.r)     f.x = w + f.r
                }
                snowCanvas.flakes = arr
                snowCanvas.requestPaint()
            }
        }

        onPaint: {
            var ctx = getContext("2d")
            ctx.clearRect(0, 0, width, height)

            var arr = flakes
            for (var i = 0; i < arr.length; i++) {
                var f = arr[i]
                ctx.beginPath()
                ctx.arc(f.x, f.y, f.r, 0, Math.PI * 2, false)
                ctx.fillStyle = "rgba(255, 255, 255, " + f.alpha + ")"
                ctx.fill()
            }
        }
    }

    // 3. Das unscharfe Rechteck (Glassmorphism Look)
    Rectangle {
        id: blurBox
        anchors.centerIn: parent
        width: 540
        height: 440
        color: "transparent"

        ShaderEffectSource {
            id: croppedBackground
            sourceItem: background
            sourceRect: Qt.rect(blurBox.x, blurBox.y, blurBox.width, blurBox.height)
            visible: false
        }

        FastBlur {
            id: blurredBg
            anchors.fill: parent
            source: croppedBackground
            radius: 48
        }

        Rectangle {
            anchors.fill: parent
            color: Qt.rgba(1, 1, 1, 0.10)
            radius: 20
        }

        Rectangle {
            anchors.fill: parent
            color: "transparent"
            radius: 20
            border.color: Qt.rgba(1, 1, 1, 0.28)
            border.width: 1
        }
    }

    // 4. Der Inhalt (Uhrzeit, Input, Bar)
    ColumnLayout {
        anchors.centerIn: blurBox

        // HIER FINE-TUNEN:
        // Schiebt den gesamten Inhalt leicht nach oben (negative Werte) oder unten (positive Werte),
        // um den optischen Leerraum der großen Uhrzeit-Schriftart auszugleichen.
        anchors.verticalCenterOffset: -4

        spacing: 20

        Text {
            id: timeLabel
            text: Qt.formatDateTime(new Date(), "hh:mm")
            font.pointSize: 64
            font.weight: Font.Bold
            color: "white"
            style: Text.Raised
            styleColor: Qt.rgba(0, 0, 0, 0.25)
            Layout.alignment: Qt.AlignHCenter

            Timer {
                interval: 1000; running: true; repeat: true
                onTriggered: timeLabel.text = Qt.formatDateTime(new Date(), "hh:mm")
            }
        }

        // Passwort Feld
        TextField {
            id: passwordField
            placeholderText: "Password"
            echoMode: TextInput.Password
            focus: true
            Layout.preferredWidth: 380
            Layout.alignment: Qt.AlignHCenter
            implicitHeight: 42
            color: "white"
            font.pointSize: 11

            // Explizites Padding für konsistente Darstellung
            leftPadding: 16
            rightPadding: 16

            // Navigation
            KeyNavigation.tab: sessionSelector
            KeyNavigation.down: sessionSelector

            onAccepted: sddm.login(userModel.lastUser, text, sessionSelector.currentIndex)

            background: Rectangle {
                radius: 10
                color: Qt.rgba(1, 1, 1, 0.10)
                border.color: passwordField.activeFocus ? Qt.rgba(1, 1, 1, 0.8) : Qt.rgba(1, 1, 1, 0.45)
                border.width: passwordField.activeFocus ? 2 : 1
                Behavior on border.color { ColorAnimation { duration: 150 } }
            }

            placeholderTextColor: Qt.rgba(1, 1, 1, 0.45)
        }

        RowLayout {
            Layout.preferredWidth: 380
            Layout.alignment: Qt.AlignHCenter
            spacing: 6
            Layout.preferredHeight: passwordField.implicitHeight

            // Session ComboBox
            ComboBox {
                id: sessionSelector
                model: sessionModel
                textRole: "name"
                currentIndex: sessionModel.lastIndex
                font.pointSize: 11
                Layout.fillWidth: true
                Layout.preferredHeight: passwordField.implicitHeight

                // Navigation
                activeFocusOnTab: true
                KeyNavigation.up: passwordField
                KeyNavigation.right: suspendBtn
                KeyNavigation.tab: suspendBtn

                contentItem: Text {
                    leftPadding: 10
                    text: sessionSelector.displayText
                    color: sessionSelector.activeFocus ? "white" : Qt.rgba(1, 1, 1, 0.50)
                    verticalAlignment: Text.AlignVCenter
                    font: sessionSelector.font
                }

                background: Rectangle {
                    radius: 8
                    color: Qt.rgba(1, 1, 1, 0.05)
                    border.color: sessionSelector.activeFocus ? Qt.rgba(1, 1, 1, 0.7) : Qt.rgba(1, 1, 1, 0.18)
                    border.width: sessionSelector.activeFocus ? 2 : 1
                    Behavior on border.color { ColorAnimation { duration: 150 } }
                }
            }

            // ── Power-Button: Suspend (Standby) ──
            Rectangle {
                id: suspendBtn
                width: passwordField.implicitHeight
                height: passwordField.implicitHeight
                radius: 8
                opacity: sddm.canSuspend ? 1.0 : 0.3

                // Navigation
                activeFocusOnTab: true
                focus: true
                KeyNavigation.left: sessionSelector
                KeyNavigation.right: shutdownBtn
                KeyNavigation.tab: shutdownBtn
                KeyNavigation.up: passwordField
                Keys.onReturnPressed: if(sddm.canSuspend) sddm.suspend()
                Keys.onEnterPressed: if(sddm.canSuspend) sddm.suspend()

                color: (sddm.canSuspend && (suspendHover.containsMouse || activeFocus))
                       ? Qt.rgba(1, 1, 1, 0.18)
                       : Qt.rgba(1, 1, 1, 0.06)
                border.color: activeFocus ? Qt.rgba(1, 1, 1, 0.7) : Qt.rgba(1, 1, 1, 0.18)
                border.width: activeFocus ? 2 : 1

                Behavior on color { ColorAnimation { duration: 120 } }
                Behavior on opacity { NumberAnimation { duration: 200 } }

                Image {
                    anchors.fill: parent
                    anchors.margins: 8
                    source: "standby.svg"
                    sourceSize: Qt.size(width, height)
                    opacity: parent.activeFocus ? 1.0 : 0.7
                    fillMode: Image.PreserveAspectFit
                }

                MouseArea {
                    id: suspendHover
                    anchors.fill: parent
                    hoverEnabled: sddm.canSuspend
                    enabled: sddm.canSuspend
                    cursorShape: enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
                    onClicked: sddm.suspend()
                }
            }

            // ── Power-Button: Herunterfahren ──
            Rectangle {
                id: shutdownBtn
                width: passwordField.implicitHeight
                height: passwordField.implicitHeight
                radius: 8

                // Navigation
                activeFocusOnTab: true
                KeyNavigation.left: suspendBtn
                KeyNavigation.right: rebootBtn
                KeyNavigation.tab: rebootBtn
                KeyNavigation.up: passwordField
                Keys.onReturnPressed: sddm.powerOff()
                Keys.onEnterPressed: sddm.powerOff()

                color: (shutdownHover.containsMouse || activeFocus)
                       ? Qt.rgba(1, 1, 1, 0.18)
                       : Qt.rgba(1, 1, 1, 0.06)
                border.color: activeFocus ? Qt.rgba(1, 1, 1, 0.7) : Qt.rgba(1, 1, 1, 0.18)
                border.width: activeFocus ? 2 : 1

                Behavior on color { ColorAnimation { duration: 120 } }

                Image {
                    anchors.fill: parent
                    anchors.margins: 8
                    source: "shutdown.svg"
                    sourceSize: Qt.size(width, height)
                    opacity: parent.activeFocus ? 1.0 : 0.7
                    fillMode: Image.PreserveAspectFit
                }

                MouseArea {
                    id: shutdownHover
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: sddm.powerOff()
                }
            }

            // ── Power-Button: Neu starten ──
            Rectangle {
                id: rebootBtn
                width: passwordField.implicitHeight
                height: passwordField.implicitHeight
                radius: 8

                // Navigation
                activeFocusOnTab: true
                KeyNavigation.left: shutdownBtn
                KeyNavigation.right: passwordField
                KeyNavigation.tab: passwordField
                KeyNavigation.up: passwordField
                Keys.onReturnPressed: sddm.reboot()
                Keys.onEnterPressed: sddm.reboot()

                color: (rebootHover.containsMouse || activeFocus)
                       ? Qt.rgba(1, 1, 1, 0.18)
                       : Qt.rgba(1, 1, 1, 0.06)
                border.color: activeFocus ? Qt.rgba(1, 1, 1, 0.7) : Qt.rgba(1, 1, 1, 0.18)
                border.width: activeFocus ? 2 : 1

                Behavior on color { ColorAnimation { duration: 120 } }

                Image {
                    anchors.fill: parent
                    anchors.margins: 10
                    source: "reboot.svg"
                    sourceSize: Qt.size(width, height)
                    opacity: parent.activeFocus ? 1.0 : 0.7
                    fillMode: Image.PreserveAspectFit
                }

                MouseArea {
                    id: rebootHover
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: sddm.reboot()
                }
            }
        }
    }
}
