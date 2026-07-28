#!/usr/bin/env bash
#
# rofi-wifi-menu.sh
# Zeigt verfuegbare WLANs in Rofi, erlaubt Verbinden/Trennen per Klick.
# Nutzen von Rofi Script-Modi für dynamisches Aktualisieren ohne Schließen.

set -euo pipefail

# Stelle sicher, dass wir den absoluten Pfad zu diesem Skript haben
SCRIPT_PATH=$(realpath "$0")

# Wenn wir NICHT von Rofi aufgerufen wurden, starte Rofi im Script-Modus
if [[ -z "${ROFI_RETV:-}" ]]; then
    rofi -show wifi -modi "wifi:$SCRIPT_PATH" -theme /home/silas270/.config/rofi/wifi.rasi
    exit 0
fi

# Icons (nerd font, passend zu deiner waybar config)
ICON_CONNECTED="󰄬"
ICON_LOCK="󰌾"
ICON_OPEN="󰖩"
ICON_RESCAN="󰑐 Rescan"
ICON_DISABLE_WIFI="󰖪 WLAN deaktivieren"
ICON_ENABLE_WIFI="󰖩 WLAN aktivieren"

# Wrapper um notify-send
notify() {
    timeout 2 notify-send "$@" >/dev/null 2>&1 || echo "$*" >&2
}

# Aktuellen Status holen
wifi_enabled=$(nmcli radio wifi)

get_networks() {
    # SSID, Signal, Security, In-Use auslesen
    # `--rescan no` verhindert explizit, dass nmcli bei der Abfrage einen langsamen Scan erzwingt!
    nmcli -t -f IN-USE,SSID,SIGNAL,SECURITY device wifi list --rescan no 2>/dev/null \
        | awk -F: '$2 != "" {print}' \
        | sort -t: -k3 -rn \
        | awk -F: '!seen[$2]++'  # doppelte SSIDs raus
}

build_menu() {
    if [[ "$wifi_enabled" == "disabled" ]]; then
        printf "%s\x00info\x1f%s\n" "${ICON_ENABLE_WIFI}" "__enable_wifi__"
        return
    fi

    # 1. Immer die ersten beiden Kontroll-Elemente
    printf "%s\x00info\x1f%s\n" "${ICON_DISABLE_WIFI}" "__disable_wifi__"
    printf "%s\x00info\x1f%s\n" "${ICON_RESCAN}" "__rescan__"

    # 2. Alle WLANs ausgeben
    while IFS=: read -r inuse ssid signal security; do
        local icon="$ICON_OPEN"
        [[ -n "$security" ]] && icon="$ICON_LOCK"
        local mark=""
        [[ "$inuse" == "*" ]] && mark=" $ICON_CONNECTED"
        
        local display="${icon}  ${ssid} (${signal}%)${mark}"
        printf "%s\x00info\x1f%s\n" "$display" "$ssid"
    done < <(get_networks)
}

# Falls Rofi initial aufruft (ROFI_RETV == 0)
if [[ "$ROFI_RETV" -eq 0 ]]; then
    build_menu
    exit 0
fi

# Wenn ein Element ausgewählt wurde (ROFI_RETV == 1)
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
        # Führe Rescan aus
        nmcli device wifi rescan >/dev/null 2>&1 || true
        build_menu
        ;;
    *)
        # Ein echtes Netzwerk wurde ausgewählt (SSID steht in $action)
        if [[ -z "$action" ]]; then
            exit 0
        fi
        
        ssid="$action"
        
        # Verbindung im Hintergrund ausführen, damit das Rofi-Fenster sofort schließt!
        (
            # Ist schon eine Verbindung fuer diese SSID bekannt?
            if nmcli -t -f NAME connection show | grep -Fxq "$ssid"; then
                nmcli connection up "$ssid" \
                    && notify "WiFi" "Verbunden mit $ssid" \
                    || notify "WiFi" "Verbindung zu $ssid fehlgeschlagen"
                exit 0
            fi
            
            # Neues Netzwerk: prüfen ob Sicherheit benötigt wird
            security=$(nmcli -t -f SSID,SECURITY device wifi list | awk -F: -v s="$ssid" '$1==s {print $2; exit}')
            
            if [[ -z "$security" || "$security" == "--" ]]; then
                # Offenes Netzwerk
                nmcli device wifi connect "$ssid" \
                    && notify "WiFi" "Verbunden mit $ssid" \
                    || notify "WiFi" "Verbindung zu $ssid fehlgeschlagen"
            else
                # Passwort abfragen
                password=$(rofi -dmenu -password -p "Passwort fuer $ssid" -theme /home/silas270/.config/rofi/wifi.rasi)
                [[ -z "$password" ]] && exit 0
                
                nmcli device wifi connect "$ssid" password "$password" \
                    && notify "WiFi" "Verbunden mit $ssid" \
                    || notify "WiFi" "Verbindung zu $ssid fehlgeschlagen (falsches Passwort?)"
            fi
        ) &
        
        # Leere Ausgabe signalisiert Rofi zu schließen
        exit 0
        ;;
esac
