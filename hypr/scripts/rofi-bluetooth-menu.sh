#!/usr/bin/env bash
#
# rofi-bluetooth-menu.sh
# Zeigt verfuegbare Bluetooth-Geräte in Rofi, erlaubt Verbinden/Trennen per Klick.
# Nutzen von Rofi Script-Modi für dynamisches Aktualisieren ohne Schließen.

set -euo pipefail

# Stelle sicher, dass wir den absoluten Pfad zu diesem Skript haben
SCRIPT_PATH=$(realpath "$0")

# Wenn wir NICHT von Rofi aufgerufen wurden, starte Rofi im Script-Modus
if [[ -z "${ROFI_RETV:-}" ]]; then
    rofi -show bluetooth -modi "bluetooth:$SCRIPT_PATH" -theme /home/silas270/.config/rofi/bluetooth.rasi
    exit 0
fi

# Icons (nerd font, passend zu deiner waybar config / latte theme)
ICON_CONNECTED="󰄬"
ICON_PAIRED="󰂱"
ICON_DISCOVERED="󰂯"
ICON_RESCAN="󰑐 Rescan"
ICON_DISABLE_BT="󰂲 BT deaktivieren"
ICON_ENABLE_BT="󰂯 BT aktivieren"

# Wrapper um notify-send
notify() {
    timeout 2 notify-send "$@" >/dev/null 2>&1 || echo "$*" >&2
}

is_powered() {
    if command -v bluetoothctl >/dev/null 2>&1; then
        bluetoothctl show 2>/dev/null | grep -q "Powered: yes"
    else
        rfkill list bluetooth 2>/dev/null | grep -q "Soft blocked: no"
    fi
}


get_devices() {
    # 1. Connected devices
    local connected_macs=""
    while read -r _ mac name; do
        if [[ -n "$mac" ]]; then
            echo "connected:$mac:$name"
            connected_macs="${connected_macs}${mac}\n"
        fi
    done < <(bluetoothctl devices Connected 2>/dev/null)

    # 2. Paired but not connected devices
    local paired_macs=""
    while read -r _ mac name; do
        if [[ -n "$mac" ]]; then
            if ! echo -e "$connected_macs" | grep -Fq "$mac"; then
                echo "paired:$mac:$name"
            fi
            paired_macs="${paired_macs}${mac}\n"
        fi
    done < <(bluetoothctl devices Paired 2>/dev/null)

    # 3. Discovered / cached devices
    while read -r _ mac name; do
        if [[ -n "$mac" ]]; then
            if ! echo -e "$connected_macs" | grep -Fq "$mac" && ! echo -e "$paired_macs" | grep -Fq "$mac"; then
                echo "discovered:$mac:$name"
            fi
        fi
    done < <(bluetoothctl devices 2>/dev/null)
}

build_menu() {
    if ! is_powered; then
        printf "%s\x00info\x1f%s\n" "${ICON_ENABLE_BT}" "__enable_bt__"
        return
    fi

    # 1. Kontroll-Elemente
    printf "%s\x00info\x1f%s\n" "${ICON_DISABLE_BT}" "__disable_bt__"
    printf "%s\x00info\x1f%s\n" "${ICON_RESCAN}" "__rescan__"

    # 2. Alle Geräte ausgeben
    while IFS=: read -r status mac name; do
        local icon="$ICON_DISCOVERED"
        local mark=""
        if [[ "$status" == "connected" ]]; then
            icon="$ICON_CONNECTED"
            mark="  [Verbunden]"
        elif [[ "$status" == "paired" ]]; then
            icon="$ICON_PAIRED"
        fi
        
        local display="${icon}  ${name} (${mac})${mark}"
        printf "%s\x00info\x1f%s\n" "$display" "$mac:$name:$status"
    done < <(get_devices)
}

# Falls Rofi initial aufruft (ROFI_RETV == 0)
if [[ "$ROFI_RETV" -eq 0 ]]; then
    build_menu
    exit 0
fi

# Wenn ein Element ausgewählt wurde (ROFI_RETV == 1)
action="${ROFI_INFO:-}"

case "$action" in
    "__enable_bt__")
        rfkill unblock bluetooth
        sleep 0.5
        bluetoothctl power on >/dev/null 2>&1 || true
        sleep 0.5
        build_menu
        ;;
    "__disable_bt__")
        bluetoothctl power off >/dev/null 2>&1 || true
        rfkill block bluetooth
        sleep 0.5
        build_menu
        ;;
    "__rescan__")
        # Starte Background Scan für 10 Sekunden
        timeout 10 bluetoothctl scan on >/dev/null 2>&1 &
        sleep 1.0
        build_menu
        ;;
    *)
        # Ein Gerät wurde ausgewählt
        if [[ -z "$action" ]]; then
            exit 0
        fi
        
        # Split info: mac:name:status
        IFS=: read -r mac name status <<< "$action"
        
        # Verbindung im Hintergrund ausführen, damit Rofi sofort schließt
        (
            if [[ "$status" == "connected" ]]; then
                notify "Bluetooth" "Trenne Verbindung mit $name..."
                bluetoothctl disconnect "$mac" \
                    && notify "Bluetooth" "Verbindung mit $name getrennt" \
                    || notify "Bluetooth" "Trennen von $name fehlgeschlagen"
            else
                notify "Bluetooth" "Verbinde mit $name..."
                bluetoothctl trust "$mac" >/dev/null 2>&1 || true
                bluetoothctl connect "$mac" \
                    && notify "Bluetooth" "Erfolgreich verbunden mit $name" \
                    || notify "Bluetooth" "Verbindung mit $name fehlgeschlagen"
            fi
        ) &
        
        # Leere Ausgabe schließt Rofi
        exit 0
        ;;
esac
