#!/usr/bin/env bash
# rofi-wifi-menu.sh - Interactive Rofi network manager (wired + Wi-Fi)
#
# Architecture: a dmenu *loop*. The list is piped INTO rofi and the selection
# comes back as an index (-format i), so command output can never leak into the
# list and payloads never round-trip through rofi's text. Status is reported via
# -mesg, which requires the `message` widget present in connections-menu.rasi.

set -uo pipefail

THEME="$HOME/.config/rofi/wifi.rasi"

ICON_CONNECTED="󰄬"
ICON_LOCK="󰌾"
ICON_OPEN="󰖩"
ICON_ETH="󰈀"
ICON_ETH_OFF="󰈂"

# nmcli waits this long before giving up (default is 90s, which hangs the menu).
NM_WAIT=20

STATUS=""

die_msg() {
    rofi -e "$1" -theme "$THEME" >/dev/null 2>&1 || printf '%s\n' "$1" >&2
    exit 1
}

command -v nmcli >/dev/null 2>&1 || die_msg "nmcli not found - NetworkManager is not installed."

if ! nmcli -t -f RUNNING general >/dev/null 2>&1; then
    die_msg "NetworkManager is not running."
fi

wifi_device() {
    nmcli -t -f DEVICE,TYPE device 2>/dev/null | awk -F: '$2=="wifi"{print $1; exit}'
}

DEV="$(wifi_device)"

radio_on() {
    [[ "$(nmcli radio wifi 2>/dev/null)" == "enabled" ]]
}

# Resolve a saved profile by its 802-11-wireless.ssid, not by profile name:
# NetworkManager renames duplicates ("MyNet 1") and users rename profiles.
saved_profile_for() {
    local want="$1" name
    while IFS= read -r name; do
        [[ -z "$name" ]] && continue
        if [[ "$(nmcli -t --escape no -g 802-11-wireless.ssid connection show "$name" 2>/dev/null)" == "$want" ]]; then
            printf '%s' "$name"
            return 0
        fi
    done < <(nmcli -t --escape no -g NAME,TYPE connection show 2>/dev/null \
                | awk -F: '$2=="802-11-wireless"{print $1}')
    return 1
}

eth_devices() {
    nmcli -t --escape no -f DEVICE,TYPE device 2>/dev/null | awk -F: '$2=="ethernet"{print $1}'
}

# Numeric device state: 100 connected, 30 disconnected (cable in), 20
# unavailable (no carrier). Numeric because the STATE column is localised.
dev_state() {
    nmcli -t --escape no -f GENERAL.STATE device show "$1" 2>/dev/null \
        | sed -n 's/^GENERAL.STATE:\([0-9]*\).*/\1/p'
}

dev_conn() {
    nmcli -t --escape no -f GENERAL.CONNECTION device show "$1" 2>/dev/null \
        | sed -n 's/^GENERAL.CONNECTION://p'
}

# First saved ethernet profile bound to this device (or an unbound one).
eth_profile_for() {
    local dev="$1"
    nmcli -t --escape no -f NAME,TYPE,DEVICE connection show 2>/dev/null \
        | awk -F: -v d="$dev" '$2=="802-3-ethernet" && ($3==d || $3=="") {print $1; exit}'
}

signal_bars() {
    local s="${1:-0}"
    if   (( s >= 75 )); then printf '▂▄▆█'
    elif (( s >= 50 )); then printf '▂▄▆_'
    elif (( s >= 25 )); then printf '▂▄__'
    else                     printf '▂___'
    fi
}

# Populates the parallel arrays DISPLAY[] / SSIDS[] / SEC[], and ACTIVE_ROW.
DISPLAY=(); SSIDS=(); SEC=(); ACTIVE_ROW=""; CUR_SSID=""
scan_networks() {
    DISPLAY=(); SSIDS=(); SEC=(); ACTIVE_ROW=""; CUR_SSID=""

    local i=0 inuse ssid signal security
    # --escape no + tab delimiter: SSIDs legally contain ':'.
    while IFS=$'\t' read -r inuse ssid signal security; do
        [[ -z "$ssid" ]] && continue
        local icon="$ICON_OPEN"
        [[ -n "$security" && "$security" != "--" ]] && icon="$ICON_LOCK"
        local mark=""
        if [[ "$inuse" == "*" ]]; then
            mark="  $ICON_CONNECTED"
            ACTIVE_ROW="$i"
            CUR_SSID="$ssid"
        fi
        DISPLAY+=("$(printf '%s  %-24s %s %s%s' "$icon" "$ssid" "$(signal_bars "$signal")" "${security:---}" "$mark")")
        SSIDS+=("$ssid")
        SEC+=("$security")
        i=$((i+1))
    done < <(
        nmcli -t --escape no -f IN-USE,SSID,SIGNAL,SECURITY device wifi list --rescan no 2>/dev/null \
          | awk -F: -v OFS='\t' '
              { inuse=$1; ssid=$2; signal=$3; sec=$4;
                if (ssid == "") next;
                # keep the strongest BSS per SSID, but never drop the in-use one
                if (!(ssid in best) || signal+0 > bestsig[ssid] || inuse == "*") {
                  if (!(ssid in best)) order[++n] = ssid;
                  if (inuse == "*" || !(ssid in best) || signal+0 > bestsig[ssid]) {
                    best[ssid] = inuse OFS ssid OFS signal OFS sec;
                    if (inuse != "*") bestsig[ssid] = signal+0; else bestsig[ssid] = 999;
                  }
                }
              }
              END { for (j=1; j<=n; j++) print best[order[j]] }' \
          | sort -t$'\t' -k3 -rn
    )
}


# --- Unified row model -------------------------------------------------------
# ROWS_DISPLAY[i] is what rofi shows; ROWS_KIND[i]/ROWS_ARG[i] carry the action.
# Kinds: eth_up, eth_down, eth_none, wifi, enable_wifi, sep
ROWS_DISPLAY=(); ROWS_KIND=(); ROWS_ARG=(); ROWS_ACTIVE=""

add_row() {
    ROWS_DISPLAY+=("$1"); ROWS_KIND+=("$2"); ROWS_ARG+=("$3")
}

build_wired_rows() {
    local dev st conn prof
    for dev in $(eth_devices); do
        st="$(dev_state "$dev")"
        case "$st" in
            100)
                conn="$(dev_conn "$dev")"
                [[ -z "$conn" || "$conn" == "--" ]] && conn="$dev"
                ROWS_ACTIVE="${#ROWS_DISPLAY[@]}"
                add_row "$(printf '%s  %-24s %s' "$ICON_ETH" "$conn" "$ICON_CONNECTED")" eth_down "$dev"
                ;;
            30)
                prof="$(eth_profile_for "$dev")"
                add_row "$(printf '%s  %-24s %s' "$ICON_ETH" "${prof:-$dev}" "ready")" eth_up "$dev"
                ;;
            *)
                add_row "$(printf '%s  %-24s %s' "$ICON_ETH_OFF" "$dev" "no cable")" eth_none "$dev"
                ;;
        esac
    done
}

build_rows() {
    ROWS_DISPLAY=(); ROWS_KIND=(); ROWS_ARG=(); ROWS_ACTIVE=""
    build_wired_rows

    # No wireless hardware: the wired rows are the whole menu.
    [[ -z "$DEV" ]] && return

    if ! radio_on; then
        (( ${#ROWS_DISPLAY[@]} > 0 )) && add_row "$(printf '%.0s\u2500' {1..38})" sep ""
        add_row "$ICON_OPEN  Enable Wi-Fi" enable_wifi ""
        return
    fi

    scan_networks
    (( ${#ROWS_DISPLAY[@]} > 0 && ${#DISPLAY[@]} > 0 )) && add_row "$(printf '%.0s\u2500' {1..38})" sep ""

    local i
    for i in "${!DISPLAY[@]}"; do
        # Only mark Wi-Fi active when nothing wired already claimed the slot.
        if [[ "$i" == "$ACTIVE_ROW" && -z "$ROWS_ACTIVE" ]]; then
            ROWS_ACTIVE="${#ROWS_DISPLAY[@]}"
        fi
        add_row "${DISPLAY[$i]}" wifi "$i"
    done
}

# The connection actually carrying traffic, for the banner.
primary_label() {
    local dev st conn
    for dev in $(eth_devices); do
        st="$(dev_state "$dev")"
        if [[ "$st" == "100" ]]; then
            conn="$(dev_conn "$dev")"
            printf 'Wired: %s' "${conn:-$dev}"
            return
        fi
    done
    if [[ -n "$CUR_SSID" ]]; then
        printf 'Wi-Fi: %s' "$CUR_SSID"
        return
    fi
    printf 'Not connected'
}

rescan() {
    STATUS="Scanning…"
    local before after
    before="$(nmcli -t --escape no -g SSID device wifi list --rescan no 2>/dev/null | sort -u)"
    nmcli device wifi rescan >/dev/null 2>&1
    # rescan returns as soon as the request is queued; poll for the result.
    local t
    for t in 1 2 3 4 5 6 7 8; do
        sleep 0.5
        after="$(nmcli -t --escape no -g SSID device wifi list --rescan no 2>/dev/null | sort -u)"
        [[ "$after" != "$before" ]] && break
    done
    STATUS="Scan complete."
}

ask_password() {
    rofi -dmenu -password -no-fixed-num-lines -p "[ password ]" \
         -mesg "Password for <b>$(esc "$1")</b>" -theme "$THEME" </dev/null 2>/dev/null
}

esc() { printf '%s' "${1//&/&amp;}" | sed 's/</\&lt;/g; s/>/\&gt;/g'; }

connect_ssid() {
    local ssid="$1" security="$2" profile err rc

    if profile="$(saved_profile_for "$ssid")"; then
        STATUS="Connecting to $(esc "$ssid")…"
        err="$(nmcli -w "$NM_WAIT" connection up "$profile" 2>&1 >/dev/null)"; rc=$?
        if (( rc == 0 )); then
            STATUS="Connected to <b>$(esc "$ssid")</b>"
            return 0
        fi
        # A saved profile with a bad key would otherwise shadow every future
        # attempt, making the network permanently unconnectable from this menu.
        if [[ "$err" == *"Secrets were required"* || "$err" == *"no secrets"* || "$err" == *"802.1X supplicant"* ]]; then
            nmcli connection delete "$profile" >/dev/null 2>&1
            STATUS="Saved password for $(esc "$ssid") was rejected - forgotten. Select it again."
            return 1
        fi
        STATUS="<b>$(esc "$ssid")</b>: $(esc "${err:-connection failed}")"
        return 1
    fi

    if [[ -z "$security" || "$security" == "--" ]]; then
        STATUS="Connecting to $(esc "$ssid")…"
        err="$(nmcli -w "$NM_WAIT" device wifi connect "$ssid" ifname "$DEV" 2>&1 >/dev/null)"; rc=$?
    else
        local pw; pw="$(ask_password "$ssid")"
        [[ -z "$pw" ]] && { STATUS="Cancelled."; return 1; }
        err="$(nmcli -w "$NM_WAIT" device wifi connect "$ssid" password "$pw" ifname "$DEV" 2>&1 >/dev/null)"; rc=$?
    fi

    if (( rc == 0 )); then
        STATUS="Connected to <b>$(esc "$ssid")</b>"
        return 0
    fi

    # Failed keyed connect leaves a poisoned profile behind - remove it.
    if [[ -n "$security" && "$security" != "--" ]]; then
        if profile="$(saved_profile_for "$ssid")"; then
            nmcli connection delete "$profile" >/dev/null 2>&1
        fi
        STATUS="Failed: $(esc "${err:-wrong password?}")"
    else
        STATUS="Failed: $(esc "${err:-connection failed}")"
    fi
    return 1
}

while true; do
    build_rows

    if (( ${#ROWS_DISPLAY[@]} == 0 )); then
        STATUS="No network devices found."
        printf '(no network devices)\n' \
            | rofi -dmenu -format i -p "[ network ]" -mesg "$STATUS" -theme "$THEME" >/dev/null 2>&1
        exit 0
    fi

    if [[ -n "$STATUS" ]]; then
        mesg="$STATUS"
    else
        mesg="$(primary_label)"$'\n'"<small>alt+r rescan · alt+d disconnect · alt+x wifi off</small>"
    fi
    active_args=()
    [[ -n "$ROWS_ACTIVE" ]] && active_args=(-a "$ROWS_ACTIVE")

    choice="$(printf '%s\n' "${ROWS_DISPLAY[@]}" \
        | rofi -dmenu -format i -p "[ network ]" -mesg "$mesg" "${active_args[@]}" \
               -kb-custom-1 "alt+r" -kb-custom-2 "alt+d" -kb-custom-3 "alt+x" \
               -theme "$THEME" 2>/dev/null)"
    rc=$?

    STATUS=""
    case $rc in
        10)
            if radio_on; then rescan; else STATUS="Wi-Fi is off."; fi
            continue ;;
        11)
            # Disconnect whatever is currently primary.
            disconnected=0
            for dev in $(eth_devices); do
                if [[ "$(dev_state "$dev")" == "100" ]]; then
                    nmcli -w "$NM_WAIT" device disconnect "$dev" >/dev/null 2>&1 \
                        && STATUS="Disconnected $dev" || STATUS="Failed to disconnect $dev"
                    disconnected=1
                    break
                fi
            done
            if (( ! disconnected )); then
                if [[ -n "$CUR_SSID" ]]; then
                    nmcli -w "$NM_WAIT" device disconnect "$DEV" >/dev/null 2>&1 \
                        && STATUS="Disconnected from $(esc "$CUR_SSID")" \
                        || STATUS="Failed to disconnect."
                else
                    STATUS="Nothing to disconnect."
                fi
            fi
            continue ;;
        12)
            nmcli radio wifi off >/dev/null 2>&1
            STATUS="Wi-Fi disabled."
            continue ;;
        0) ;;
        *) exit 0 ;;
    esac

    idx="$choice"
    [[ "$idx" =~ ^[0-9]+$ ]] || exit 0
    (( idx < ${#ROWS_KIND[@]} )) || exit 0

    case "${ROWS_KIND[$idx]}" in
        sep)
            continue ;;
        eth_none)
            STATUS="${ROWS_ARG[$idx]}: no cable connected."
            continue ;;
        eth_up)
            dev="${ROWS_ARG[$idx]}"
            STATUS="Connecting $dev…"
            err="$(nmcli -w "$NM_WAIT" device connect "$dev" 2>&1 >/dev/null)"
            if [[ "$(dev_state "$dev")" == "100" ]]; then
                STATUS="Connected: <b>$(esc "$(dev_conn "$dev")")</b>"
            else
                STATUS="Failed: $(esc "${err:-could not bring up $dev}")"
            fi
            continue ;;
        eth_down)
            dev="${ROWS_ARG[$idx]}"
            nmcli -w "$NM_WAIT" device disconnect "$dev" >/dev/null 2>&1 \
                && STATUS="Disconnected $dev" || STATUS="Failed to disconnect $dev"
            continue ;;
        enable_wifi)
            nmcli radio wifi on >/dev/null 2>&1
            for _ in 1 2 3 4 5 6 7 8 9 10; do
                radio_on && break
                sleep 0.3
            done
            DEV="$(wifi_device)"
            STATUS="Wi-Fi enabled."
            continue ;;
        wifi)
            w="${ROWS_ARG[$idx]}"
            ssid="${SSIDS[$w]}"
            if [[ -n "$CUR_SSID" && "$ssid" == "$CUR_SSID" ]]; then
                nmcli -w "$NM_WAIT" device disconnect "$DEV" >/dev/null 2>&1 \
                    && STATUS="Disconnected from $(esc "$ssid")" \
                    || STATUS="Failed to disconnect."
            else
                connect_ssid "$ssid" "${SEC[$w]}"
            fi
            continue ;;
    esac
done
