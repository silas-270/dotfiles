import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15

Rectangle {
    id: container
    width: 1920
    height: 1080
    color: "#1E130D"

    // Master Theme Color Tokens (Savanna Dusk)
    readonly property color colBgBase: "#54382B"
    readonly property color colBgInput: "#664839"
    readonly property color colFgPrimary: "#F2E3D5"
    readonly property color colFgMuted: "#C2AA95"
    readonly property color colAccent: "#D97736"
    readonly property color colBorder: "#7A523D"
    readonly property color colDanger: "#C0392B"
    readonly property color colDarkText: "#1E130D"

    readonly property string fontMono: "JetBrainsMono Nerd Font, JetBrains Mono, Monospace"
    readonly property int elementHeight: 42
    readonly property int gutterSpacing: 16
    readonly property int outerInset: 20   // top / bottom padding
    readonly property int hInset: 48       // left / right padding (wider)
    readonly property int btnFontSize: 10

    // 1. Wallpaper Background
    Image {
        id: background
        source: "wallpaper.jpg"
        anchors.fill: parent
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
    }

    // Flat subtle dimming layer
    Rectangle {
        anchors.fill: parent
        color: Qt.rgba(0.12, 0.07, 0.05, 0.35)
    }

    // 2. Warm Savanna Sunset Embers Canvas
    Canvas {
        id: emberCanvas
        anchors.fill: parent

        property var embers: []
        property int emberCount: 80

        Component.onCompleted: {
            var arr = []
            for (var i = 0; i < emberCount; i++) {
                arr.push({
                    x: Math.random() * width,
                    y: Math.random() * height,
                    r: 1.0 + Math.random() * 2.5,
                    speedY: 0.3 + Math.random() * 0.8,
                    drift: (Math.random() - 0.5) * 0.4,
                    alpha: 0.25 + Math.random() * 0.55,
                    phase: Math.random() * Math.PI * 2,
                    isAmber: Math.random() > 0.35
                })
            }
            embers = arr
        }

        Timer {
            id: emberTimer
            interval: 16 // ~60 fps
            running: true
            repeat: true
            onTriggered: {
                var arr = emberCanvas.embers
                var h = emberCanvas.height
                var w = emberCanvas.width
                var t = Date.now() / 1000

                for (var i = 0; i < arr.length; i++) {
                    var e = arr[i]
                    e.y -= e.speedY
                    e.x += e.drift + Math.sin(t * 0.7 + e.phase) * 0.35

                    if (e.y < -e.r) {
                        e.y = h + e.r
                        e.x = Math.random() * w
                    }
                    if (e.x > w + e.r) e.x = -e.r
                    if (e.x < -e.r) e.x = w + e.r
                }
                emberCanvas.embers = arr
                emberCanvas.requestPaint()
            }
        }

        onPaint: {
            var ctx = getContext("2d")
            ctx.clearRect(0, 0, width, height)

            var arr = embers
            for (var i = 0; i < arr.length; i++) {
                var e = arr[i]
                ctx.beginPath()
                ctx.arc(e.x, e.y, e.r, 0, Math.PI * 2, false)
                ctx.fillStyle = e.isAmber 
                    ? "rgba(217, 119, 54, " + e.alpha + ")" 
                    : "rgba(242, 227, 213, " + (e.alpha * 0.8) + ")"
                ctx.fill()
            }
        }
    }

    // 3. Flat Modular Alignment Column
    ColumnLayout {
        anchors.centerIn: parent
        spacing: 24

        // ── Panel 1: Clock panel ──
        Rectangle {
            id: clockPanel
            Layout.alignment: Qt.AlignHCenter
            implicitWidth: timeLabel.paintedWidth + hInset * 2
            implicitHeight: timeLabel.paintedHeight + outerInset * 2
            color: colBgBase
            radius: 0
            border.width: 0

            Text {
                id: timeLabel
                anchors.centerIn: parent
                text: Qt.formatDateTime(new Date(), "hh:mm")
                font.family: fontMono
                font.pointSize: 80
                font.bold: true
                color: colFgMuted

                Timer {
                    interval: 1000; running: true; repeat: true
                    onTriggered: timeLabel.text = Qt.formatDateTime(new Date(), "hh:mm")
                }
            }
        }

        // ── Panel 2: Password Input [ **** ] [→] ──
        Rectangle {
            Layout.alignment: Qt.AlignHCenter
            implicitWidth: clockPanel.implicitWidth
            implicitHeight: keyboard.capsLock ? (elementHeight + outerInset + 20) : (elementHeight + outerInset)
            color: colBgBase
            radius: 0
            border.width: 0

            ColumnLayout {
                anchors.fill: parent
                anchors.leftMargin: outerInset
                anchors.rightMargin: outerInset
                anchors.topMargin: outerInset / 2
                anchors.bottomMargin: outerInset / 2
                spacing: 6

                RowLayout {
                    id: inputRow
                    Layout.fillWidth: true
                    spacing: 10

                    // Left bracket
                    Text {
                        text: "["
                        font.family: fontMono
                        font.pointSize: 18
                        font.bold: true
                        color: colAccent
                        verticalAlignment: Text.AlignVCenter
                    }

                    // Naked TextField — fills remaining space between brackets
                    TextField {
                        id: passwordField
                        echoMode: TextInput.Password
                        passwordCharacter: "*"
                        focus: true
                        Layout.fillWidth: true
                        implicitHeight: elementHeight
                        placeholderText: userModel.lastUser ? userModel.lastUser : "user"
                        placeholderTextColor: Qt.rgba(0.76, 0.67, 0.58, 0.4)
                        color: colFgMuted
                        font.family: fontMono
                        font.pointSize: 18
                        font.kerning: false         // disable OpenType kerning pairs
                        font.preferShaping: false   // disable ligature/contextual substitution
                        leftPadding: 0
                        rightPadding: 0
                        topPadding: 0
                        bottomPadding: 0
                        background: null
                        verticalAlignment: TextInput.AlignVCenter

                        onAccepted: loginAction()
                    }

                    // Right bracket
                    Text {
                        text: "]"
                        font.family: fontMono
                        font.pointSize: 18
                        font.bold: true
                        color: colAccent
                        verticalAlignment: Text.AlignVCenter
                    }

                    // Submit arrow
                    Text {
                        id: loginBtn
                        text: "[→]"
                        font.family: fontMono
                        font.pointSize: 18
                        font.bold: true
                        color: loginHover.containsMouse ? colAccent : colFgMuted
                        verticalAlignment: Text.AlignVCenter

                        Behavior on color { ColorAnimation { duration: 80 } }

                        MouseArea {
                            id: loginHover
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: loginAction()
                        }
                    }
                }

                // Dynamic Caps Lock Warning
                Text {
                    visible: keyboard.capsLock
                    text: "[ CAPS LOCK ACTIVE ]"
                    font.family: fontMono
                    font.pointSize: 8
                    font.bold: true
                    color: colAccent
                    Layout.alignment: Qt.AlignHCenter
                }
            }
        }

        // ── Panel 3: Action Buttons ──
        RowLayout {
            Layout.alignment: Qt.AlignHCenter
            implicitWidth: clockPanel.implicitWidth
            spacing: gutterSpacing

            // Reboot Button
            Rectangle {
                id: rebootBtn
                Layout.fillWidth: true
                implicitHeight: 48
                radius: 0
                color: colBgBase
                border.width: 0

                activeFocusOnTab: true
                KeyNavigation.right: shutdownBtn
                KeyNavigation.up: passwordField
                Keys.onReturnPressed: sddm.reboot()
                Keys.onEnterPressed: sddm.reboot()

                Text {
                    anchors.centerIn: parent
                    text: "[ REBOOT ]"
                    font.family: fontMono
                    font.pointSize: btnFontSize
                    font.bold: true
                    color: (rebootHover.containsMouse || parent.activeFocus) ? colAccent : colFgMuted
                    Behavior on color { ColorAnimation { duration: 80 } }
                }

                MouseArea {
                    id: rebootHover
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: sddm.reboot()
                }
            }

            // Shutdown Button
            Rectangle {
                id: shutdownBtn
                Layout.fillWidth: true
                implicitHeight: 48
                radius: 0
                color: colBgBase
                border.width: 0

                activeFocusOnTab: true
                KeyNavigation.left: rebootBtn
                KeyNavigation.right: trollBtn
                KeyNavigation.up: passwordField
                Keys.onReturnPressed: sddm.powerOff()
                Keys.onEnterPressed: sddm.powerOff()

                Text {
                    anchors.centerIn: parent
                    text: "[ SHUTDOWN ]"
                    font.family: fontMono
                    font.pointSize: btnFontSize
                    font.bold: true
                    color: (shutdownHover.containsMouse || parent.activeFocus) ? colDanger : colFgMuted
                    Behavior on color { ColorAnimation { duration: 80 } }
                }

                MouseArea {
                    id: shutdownHover
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: sddm.powerOff()
                }
            }

            // Troll Button: [ rm -rf / ] (Purely cosmetic / harmless)
            Rectangle {
                id: trollBtn
                Layout.fillWidth: true
                implicitHeight: 48
                radius: 0
                color: colBgBase
                border.width: 0

                activeFocusOnTab: true
                KeyNavigation.left: shutdownBtn
                KeyNavigation.up: passwordField

                Text {
                    anchors.centerIn: parent
                    text: "[ rm -rf / ]"
                    font.family: fontMono
                    font.pointSize: btnFontSize
                    font.bold: true
                    color: (trollHover.containsMouse || parent.activeFocus) ? colDanger : colFgMuted
                    Behavior on color { ColorAnimation { duration: 80 } }
                }

                MouseArea {
                    id: trollHover
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        // Completely harmless easter egg: does nothing
                    }
                }
            }
        }


    }  // end ColumnLayout


    // Direct Login Execution Function
    function loginAction() {
        if (passwordField.text.length > 0) {
            var sessionIdx = 0
            if (typeof sessionModel !== "undefined" && sessionModel.lastIndex !== undefined) {
                sessionIdx = sessionModel.lastIndex
            }
            var user = (typeof userModel !== "undefined" && userModel.lastUser) ? userModel.lastUser : "silas"
            sddm.login(user, passwordField.text, sessionIdx)
        }
    }
}

