#!/usr/bin/env bash
# waybar-sound.sh - Combined MPRIS track title & volume status for Waybar

# Read volume and mute state
vol_raw=$(wpctl get-volume @DEFAULT_AUDIO_SINK@ 2>/dev/null)
vol_val=$(echo "$vol_raw" | awk '{print $2}')
vol_percent=$(python3 -c "print(int(float('${vol_val:-0}') * 100))" 2>/dev/null || echo "0")

if [[ "$vol_raw" == *"[MUTED]"* ]]; then
    is_muted=true
else
    is_muted=false
fi

# Determine icon
if [ "$is_muted" = true ]; then
    icon="󰝟"
elif [ "$vol_percent" -ge 50 ]; then
    icon="󰕾"
elif [ "$vol_percent" -gt 0 ]; then
    icon="󰖀"
else
    icon="󰕿"
fi

# Volume text
if [ "$is_muted" = true ]; then
    vol_text=""
else
    vol_text="${vol_percent}%"
fi

# Check MPRIS media status
player_status=$(playerctl status 2>/dev/null || echo "Stopped")
title=""

if [[ "$player_status" == "Playing" ]]; then
    raw_title=$(playerctl metadata --format '{{title}}' 2>/dev/null)
    if [[ -n "$raw_title" ]]; then
        # Truncate title if longer than 22 characters
        if [ ${#raw_title} -gt 22 ]; then
            title="${raw_title:0:20}..."
        else
            title="$raw_title"
        fi
    fi
fi

# Format output string
if [[ -n "$title" ]]; then
    if [ "$is_muted" = true ]; then
        output="[ ${title} | ${icon} ]"
    else
        output="[ ${title} | ${icon} ${vol_text} ]"
    fi
else
    if [ "$is_muted" = true ]; then
        output="[ ${icon} ]"
    else
        output="[ ${icon} ${vol_text} ]"
    fi
fi

echo "$output"
