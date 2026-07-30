#!/usr/bin/env bash
# rofi-bluetooth-menu.sh - Interactive Rofi Bluetooth manager

set -euo pipefail

SCRIPT_PATH=$(realpath "$0")

if [[ -z "${ROFI_RETV:-}" ]]; then
    rofi -show bluetooth -modi "bluetooth:$SCRIPT_PATH" -theme ~/.config/rofi/bluetooth.rasi
    exit 0
fi

ICON_CONNECTED="󰄬"
ICON_PAIRED="󰂱"
ICON_DISCOVERED="󰂯"
ICON_RESCAN="󰑐 Rescan"
ICON_DISABLE_BT="󰂲 Disable Bluetooth"
ICON_ENABLE_BT="󰂯 Enable Bluetooth"

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
    local connected_macs=""
    while read -r _ mac name; do
        if [[ -n "$mac" ]]; then
            echo "connected:$mac:$name"
            connected_macs="${connected_macs}${mac}\n"
        fi
    done < <(bluetoothctl devices Connected 2>/dev/null)

    local paired_macs=""
    while read -r _ mac name; do
        if [[ -n "$mac" ]]; then
            if ! echo -e "$connected_macs" | grep -Fq "$mac"; then
                echo "paired:$mac:$name"
            fi
            paired_macs="${paired_macs}${mac}\n"
        fi
    done < <(bluetoothctl devices Paired 2>/dev/null)

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

    printf "%s\x00info\x1f%s\n" "${ICON_DISABLE_BT}" "__disable_bt__"
    printf "%s\x00info\x1f%s\n" "${ICON_RESCAN}" "__rescan__"

    while IFS=: read -r state mac name; do
        [[ -z "$mac" ]] && continue
        local icon="$ICON_DISCOVERED"
        local suffix=""
        
        if [[ "$state" == "connected" ]]; then
            icon="$ICON_CONNECTED"
            suffix=" (Connected)"
        elif [[ "$state" == "paired" ]]; then
            icon="$ICON_PAIRED"
            suffix=" (Paired)"
        fi
        
        local display="${icon}  ${name:-$mac}${suffix}"
        printf "%s\x00info\x1f%s\n" "$display" "${state}:${mac}:${name}"
    done < <(get_devices)
}

if [[ "$ROFI_RETV" -eq 0 ]]; then
    build_menu
    exit 0
fi

action="${ROFI_INFO:-}"

case "$action" in
    "__enable_bt__")
        bluetoothctl power on >/dev/null 2>&1 || rfkill unblock bluetooth 2>/dev/null || true
        sleep 1
        build_menu
        notify "Bluetooth" "Bluetooth Enabled"
        ;;
    "__disable_bt__")
        bluetoothctl power off >/dev/null 2>&1 || rfkill block bluetooth 2>/dev/null || true
        sleep 0.5
        build_menu
        notify "Bluetooth" "Bluetooth Disabled"
        ;;
    "__rescan__")
        (
            bluetoothctl scan on >/dev/null 2>&1 &
            scan_pid=$!
            sleep 5
            kill "$scan_pid" 2>/dev/null || true
            bluetoothctl scan off >/dev/null 2>&1 || true
        ) &
        sleep 0.5
        build_menu
        notify "Bluetooth" "Scanning for devices..."
        ;;
    *)
        if [[ -z "$action" ]]; then
            exit 0
        fi
        
        IFS=: read -r state mac name <<< "$action"
        
        (
            if [[ "$state" == "connected" ]]; then
                bluetoothctl disconnect "$mac" \
                    && notify "Bluetooth" "Disconnected from ${name:-$mac}" \
                    || notify "Bluetooth" "Failed to disconnect ${name:-$mac}"
            else
                notify "Bluetooth" "Connecting to ${name:-$mac}..."
                bluetoothctl pair "$mac" 2>/dev/null || true
                bluetoothctl trust "$mac" 2>/dev/null || true
                
                if bluetoothctl connect "$mac"; then
                    notify "Bluetooth" "Connected to ${name:-$mac}"
                else
                    notify "Bluetooth" "Connection to ${name:-$mac} failed"
                fi
            fi
        ) &
        
        exit 0
        ;;
esac
