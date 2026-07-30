#!/usr/bin/env bash
# rofi-wifi-menu.sh - Interactive Rofi Wi-Fi manager

set -euo pipefail

SCRIPT_PATH=$(realpath "$0")

if [[ -z "${ROFI_RETV:-}" ]]; then
    rofi -show wifi -modi "wifi:$SCRIPT_PATH" -theme /home/silas270/.config/rofi/wifi.rasi
    exit 0
fi

ICON_CONNECTED="󰄬"
ICON_LOCK="󰌾"
ICON_OPEN="󰖩"
ICON_RESCAN="󰑐 Rescan"
ICON_DISABLE_WIFI="󰖪 Disable Wi-Fi"
ICON_ENABLE_WIFI="󰖩 Enable Wi-Fi"

notify() {
    timeout 2 notify-send "$@" >/dev/null 2>&1 || echo "$*" >&2
}

wifi_enabled=$(nmcli radio wifi)

get_networks() {
    nmcli -t -f IN-USE,SSID,SIGNAL,SECURITY device wifi list --rescan no 2>/dev/null \
        | awk -F: '$2 != "" {print}' \
        | sort -t: -k3 -rn \
        | awk -F: '!seen[$2]++'
}

build_menu() {
    if [[ "$wifi_enabled" == "disabled" ]]; then
        printf "%s\x00info\x1f%s\n" "${ICON_ENABLE_WIFI}" "__enable_wifi__"
        return
    fi

    printf "%s\x00info\x1f%s\n" "${ICON_DISABLE_WIFI}" "__disable_wifi__"
    printf "%s\x00info\x1f%s\n" "${ICON_RESCAN}" "__rescan__"

    while IFS=: read -r inuse ssid signal security; do
        local icon="$ICON_OPEN"
        [[ -n "$security" ]] && icon="$ICON_LOCK"
        local mark=""
        [[ "$inuse" == "*" ]] && mark=" $ICON_CONNECTED"
        
        local display="${icon}  ${ssid} (${signal}%)${mark}"
        printf "%s\x00info\x1f%s\n" "$display" "$ssid"
    done < <(get_networks)
}

if [[ "$ROFI_RETV" -eq 0 ]]; then
    build_menu
    exit 0
fi

action="${ROFI_INFO:-}"

case "$action" in
    "__enable_wifi__")
        nmcli radio wifi on
        sleep 1.5
        wifi_enabled="enabled"
        build_menu
        ;;
    "__disable_wifi__")
        nmcli radio wifi off
        sleep 0.5
        wifi_enabled="disabled"
        build_menu
        ;;
    "__rescan__")
        nmcli device wifi rescan >/dev/null 2>&1 || true
        build_menu
        ;;
    *)
        if [[ -z "$action" ]]; then
            exit 0
        fi
        
        ssid="$action"
        
        active_ssid=$(nmcli -t -f ACTIVE,SSID dev wifi | awk -F: '$1=="yes"{print $2}')
        
        if [[ "$active_ssid" == "$ssid" ]]; then
            nmcli device disconnect wlan0 2>/dev/null \
                || nmcli device disconnect "$(nmcli -t -f DEVICE,TYPE dev | awk -F: '$2=="wifi"{print $1; exit}')" \
                && notify "Wi-Fi" "Disconnected from $ssid"
            exit 0
        fi
        
        saved_conn=$(nmcli -t -f NAME connection show | grep -xF "$ssid" || true)
        
        if [[ -n "$saved_conn" ]]; then
            nmcli connection up "$ssid" \
                && notify "Wi-Fi" "Connected to $ssid" \
                || notify "Wi-Fi" "Connection to $ssid failed"
            exit 0
        fi
        
        security=$(nmcli -t -f SSID,SECURITY device wifi list | awk -F: -v s="$ssid" '$1==s {print $2; exit}')
        
        if [[ -z "$security" || "$security" == "--" ]]; then
            nmcli device wifi connect "$ssid" \
                && notify "Wi-Fi" "Connected to $ssid" \
                || notify "Wi-Fi" "Connection to $ssid failed"
        else
            password=$(rofi -dmenu -password -p "[ password for $ssid ]" -theme /home/silas270/.config/rofi/wifi.rasi)
            [[ -z "$password" ]] && exit 0
            
            nmcli device wifi connect "$ssid" password "$password" \
                && notify "Wi-Fi" "Connected to $ssid" \
                || notify "Wi-Fi" "Connection to $ssid failed (invalid password?)"
        fi
        
        exit 0
        ;;
esac
