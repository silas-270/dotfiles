#!/usr/bin/env bash
# rofi-power-profile-menu.sh - Vertical menu styled like Waybar centerpiece modules
# Controls system power profiles via powerprofilesctl, sysfs scaling governors, asusctl, or tlp.

set -euo pipefail

OPTION_PERF="[ 󰓅 Linux ]"
OPTION_BAL="[ 󰾅 MacOS ]"
OPTION_SAV="[ 󰾆 Windows ]"

options="${OPTION_PERF}\n${OPTION_BAL}\n${OPTION_SAV}"

get_current_profile() {
    if command -v powerprofilesctl >/dev/null 2>&1; then
        powerprofilesctl get 2>/dev/null || echo ""
    elif [ -f /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor ]; then
        local gov
        gov=$(cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor 2>/dev/null || echo "")
        case "$gov" in
            performance) echo "performance" ;;
            powersave) echo "power-saver" ;;
            *) echo "balanced" ;;
        esac
    else
        echo ""
    fi
}

current_profile=$(get_current_profile)
selected_row=1

case "$current_profile" in
    performance) selected_row=0 ;;
    balanced)    selected_row=1 ;;
    power-saver) selected_row=2 ;;
esac

chosen=$(echo -e "$options" | rofi -dmenu -i -p "Power Profile" -selected-row "$selected_row" -theme ~/.config/rofi/power-profile.rasi)

set_profile() {
    local profile_name="$1"
    local ppd_name="$2"
    local gov_name="$3"
    
    local success=0

    # 1. Update D-Bus state via powerprofilesctl if available
    if command -v powerprofilesctl >/dev/null 2>&1; then
        powerprofilesctl set "$ppd_name" 2>/dev/null || powerprofilesctl set balanced 2>/dev/null || true
    fi

    # 2. Enforce physical hardware CPU governor change via sysfs
    if [ -w /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor ]; then
        if echo "$gov_name" | tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor >/dev/null 2>&1; then
            success=1
        fi
    elif [ -d /sys/devices/system/cpu/cpufreq ]; then
        if echo "$gov_name" | pkexec tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor >/dev/null 2>&1; then
            success=1
        fi
    fi

    if [ "$success" -eq 1 ]; then
        notify-send -u normal "Power Profile" "Switched to ${profile_name} (${gov_name})"
    else
        notify-send -u normal "Power Profile" "Switched to ${profile_name}"
    fi
}

case "$chosen" in
    *"Performance"*)
        set_profile "Performance" "performance" "performance"
        ;;
    *"Balanced"*)
        set_profile "Balanced" "balanced" "schedutil"
        ;;
    *"Power Saver"*)
        set_profile "Power Saver" "power-saver" "powersave"
        ;;
esac

