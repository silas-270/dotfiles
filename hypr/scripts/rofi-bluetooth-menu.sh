#!/usr/bin/env bash
# rofi-bluetooth-menu.sh - Interactive Rofi Bluetooth manager
#
# Architecture: a dmenu *loop* (see rofi-wifi-menu.sh for the rationale).
#
# Two things make this actually work where the old version could not:
#   1. No bluetoothctl call is allowed to abort the flow. The old script ran
#      `pair` unguarded under `set -e`; for an already-paired device pair
#      returns org.bluez.Error.AlreadyExists, so `connect` was never reached.
#   2. Pairing runs under `bluetoothctl --agent`, which registers a pairing
#      agent for the lifetime of the command. Without an agent registered
#      anywhere on the system, pairing simply cannot complete.

set -uo pipefail

THEME="$HOME/.config/rofi/bluetooth.rasi"

ICON_CONNECTED="󰄬"
ICON_PAIRED="󰂱"
ICON_DISCOVERED="󰂯"

# "Just works" pairing - correct for headphones/earbuds, which have no keypad.
AGENT_CAP="NoInputNoOutput"
OP_TIMEOUT=20

STATUS=""

die_msg() {
    rofi -e "$1" -theme "$THEME" >/dev/null 2>&1 || printf '%s\n' "$1" >&2
    exit 1
}

esc() { printf '%s' "${1//&/&amp;}" | sed 's/</\&lt;/g; s/>/\&gt;/g'; }

command -v bluetoothctl >/dev/null 2>&1 || die_msg "bluetoothctl not found - install bluez-utils."

# Distinguish "no adapter" from "powered off": check command success, not just
# whether the binary exists.
have_adapter() {
    bluetoothctl show 2>/dev/null | grep -q '^Controller '
}

is_powered() {
    bluetoothctl show 2>/dev/null | grep -q "Powered: yes"
}

# Order matters: while the adapter is rfkill soft-blocked, `power on` fails with
# org.bluez.Error.Blocked. Unblock first, then power on, then wait for it.
power_on() {
    rfkill unblock bluetooth >/dev/null 2>&1
    local i
    for i in $(seq 1 15); do
        is_powered && return 0
        bluetoothctl power on >/dev/null 2>&1
        sleep 0.4
    done
    is_powered
}

power_off() {
    bluetoothctl power off >/dev/null 2>&1
    rfkill block bluetooth >/dev/null 2>&1
    return 0
}

# Detached discovery. `bluetoothctl scan on &` is a one-shot RPC whose scan dies
# with the client, and it holds the parent's stdout open; --timeout keeps the
# client alive for the duration instead.
start_scan() {
    setsid bluetoothctl --timeout 12 scan on >/dev/null 2>&1 </dev/null &
    disown 2>/dev/null || true
}

# bluetoothctl exits 0 even when an operation fails ("Failed to connect:
# org.bluez.Error.InProgress" still returns 0), so every result below is
# verified against the device's actual state rather than an exit code.
dev_flag() {
    bluetoothctl info "$1" 2>/dev/null | sed -n "s/^[[:space:]]*$2: //p" | head -1
}
is_connected() { [[ "$(dev_flag "$1" Connected)" == "yes" ]]; }
is_paired()    { [[ "$(dev_flag "$1" Paired)"    == "yes" ]]; }

# Strip ANSI colour codes and pull the last meaningful failure line.
last_error() {
    printf '%s' "$1" | sed 's/\x1b\[[0-9;]*m//g' \
        | grep -iE 'failed|error|not available|busy' | tail -1
}

battery_of() {
    bluetoothctl info "$1" 2>/dev/null \
        | sed -n 's/.*Battery Percentage:.*(\([0-9]\+\)).*/\1/p' | head -1
}

macs_from() {
    bluetoothctl devices ${1:+"$1"} 2>/dev/null | awk '$1=="Device"{print $2}'
}

DISPLAY=(); MACS=(); NAMES=(); STATES=(); ACTIVE_ROW=""
build_devices() {
    DISPLAY=(); MACS=(); NAMES=(); STATES=(); ACTIVE_ROW=""

    local connected paired mac name state icon suffix batt i=0
    connected=" $(macs_from Connected | tr '\n' ' ')"
    paired=" $(macs_from Paired | tr '\n' ' ')"

    while read -r _ mac name; do
        [[ -z "$mac" ]] && continue
        [[ -z "$name" ]] && name="$mac"

        if [[ "$connected" == *" $mac "* ]]; then
            state="connected"; icon="$ICON_CONNECTED"; suffix="  (connected)"
            ACTIVE_ROW="$i"
        elif [[ "$paired" == *" $mac "* ]]; then
            state="paired";    icon="$ICON_PAIRED";    suffix="  (paired)"
        else
            state="new";       icon="$ICON_DISCOVERED"; suffix=""
        fi

        if [[ "$state" == "connected" ]]; then
            batt="$(battery_of "$mac")"
            [[ -n "$batt" ]] && suffix="  (connected · ${batt}%)"
        fi

        DISPLAY+=("$(printf '%s  %s%s' "$icon" "$name" "$suffix")")
        MACS+=("$mac")
        NAMES+=("$name")
        STATES+=("$state")
        i=$((i+1))
    done < <(
        {
            bluetoothctl devices Connected 2>/dev/null
            bluetoothctl devices Paired 2>/dev/null
            bluetoothctl devices 2>/dev/null
        } | awk '$1=="Device" && !seen[$2]++'
    )
}

# The smart one-click flow: power -> pair (agent-backed, only if needed)
# -> trust -> connect, with each step guarded and its real error surfaced.
smart_connect() {
    local mac="$1" name="$2" out err

    if ! is_powered; then
        STATUS="Powering on…"
        power_on || { STATUS="Could not power on the adapter."; return 1; }
    fi

    if is_connected "$mac"; then
        STATUS="Already connected to <b>$(esc "$name")</b>"
        return 0
    fi

    # Pair only when actually needed; an already-paired device fails pair with
    # AlreadyExists, which must never abort the flow.
    if ! is_paired "$mac"; then
        STATUS="Pairing with $(esc "$name")…"
        out="$(bluetoothctl --agent "$AGENT_CAP" --timeout "$OP_TIMEOUT" pair "$mac" 2>&1)"
        if ! is_paired "$mac"; then
            err="$(last_error "$out")"
            STATUS="Pairing failed: $(esc "${err:-could not pair with $name}")"
            return 1
        fi
    fi

    bluetoothctl trust "$mac" >/dev/null 2>&1

    local attempt i
    for attempt in 1 2 3; do
        STATUS="Connecting to $(esc "$name")…"
        out="$(bluetoothctl --timeout "$OP_TIMEOUT" connect "$mac" 2>&1)"

        # Verify against real state - the exit code lies.
        for i in 1 2 3 4 5 6; do
            is_connected "$mac" && break
            sleep 0.5
        done

        if is_connected "$mac"; then
            local batt; batt="$(battery_of "$mac")"
            if [[ -n "$batt" ]]; then
                STATUS="Connected to <b>$(esc "$name")</b> · ${batt}%"
            else
                STATUS="Connected to <b>$(esc "$name")</b>"
            fi
            return 0
        fi

        # br-connection-busy means an attempt is still settling; back off longer.
        if [[ "$out" == *"busy"* || "$out" == *"InProgress"* ]]; then
            sleep 3
        else
            # Audio devices routinely refuse the first attempt after waking.
            sleep 1
        fi
    done

    err="$(last_error "$out")"
    STATUS="Could not connect to $(esc "$name"): $(esc "${err:-device not responding (is it on and in range?)}")"
    return 1
}

have_adapter || die_msg "No Bluetooth adapter found."

while true; do
    if ! is_powered; then
        choice="$(printf '󰂯  Enable Bluetooth\n' \
            | rofi -dmenu -format i -p "[ bluetooth ]" \
                   -mesg "${STATUS:-Bluetooth is <b>off</b>}" -theme "$THEME" 2>/dev/null)"
        [[ -z "$choice" ]] && exit 0
        STATUS=""
        if power_on; then
            start_scan
            STATUS="Bluetooth enabled."
        else
            STATUS="Could not power on the adapter."
        fi
        continue
    fi

    build_devices

    mesg="${STATUS:-<b>alt+r</b> scan   <b>alt+f</b> forget   <b>alt+x</b> turn Bluetooth off}"
    active_args=()
    [[ -n "$ACTIVE_ROW" ]] && active_args=(-a "$ACTIVE_ROW")

    if (( ${#DISPLAY[@]} == 0 )); then
        printf '(no devices - alt+r to scan)\n' \
            | rofi -dmenu -format i -p "[ bluetooth ]" -mesg "$mesg" \
                   -kb-custom-1 "alt+r" -kb-custom-2 "alt+f" -kb-custom-3 "alt+x" \
                   -theme "$THEME" >/dev/null 2>&1
        rc=$?
        case $rc in
            10) start_scan; STATUS="Scanning…"; sleep 3; continue ;;
            12) power_off; STATUS="Bluetooth disabled."; continue ;;
            *)  exit 0 ;;
        esac
    fi

    choice="$(printf '%s\n' "${DISPLAY[@]}" \
        | rofi -dmenu -format i -p "[ bluetooth ]" -mesg "$mesg" "${active_args[@]}" \
               -kb-custom-1 "alt+r" -kb-custom-2 "alt+f" -kb-custom-3 "alt+x" \
               -theme "$THEME" 2>/dev/null)"
    rc=$?

    idx="$choice"
    valid_idx=0
    [[ "$idx" =~ ^[0-9]+$ ]] && (( idx < ${#MACS[@]} )) && valid_idx=1

    case $rc in
        10) start_scan; STATUS="Scanning…"; sleep 3; continue ;;
        11)
            if (( valid_idx )); then
                bluetoothctl remove "${MACS[$idx]}" >/dev/null 2>&1 \
                    && STATUS="Forgot $(esc "${NAMES[$idx]}")" \
                    || STATUS="Could not forget $(esc "${NAMES[$idx]}")"
            else
                STATUS="Select a device to forget."
            fi
            continue ;;
        12) power_off; STATUS="Bluetooth disabled."; continue ;;
        0)  ;;
        *)  exit 0 ;;
    esac

    (( valid_idx )) || exit 0

    mac="${MACS[$idx]}"; name="${NAMES[$idx]}"; state="${STATES[$idx]}"

    if [[ "$state" == "connected" ]]; then
        bluetoothctl --timeout "$OP_TIMEOUT" disconnect "$mac" >/dev/null 2>&1
        if is_connected "$mac"; then
            STATUS="Failed to disconnect from $(esc "$name")"
        else
            STATUS="Disconnected from $(esc "$name")"
        fi
    else
        smart_connect "$mac" "$name"
    fi
done
