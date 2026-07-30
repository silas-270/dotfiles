#!/usr/bin/env bash
# waybar-centerpiece.sh - Dynamic centerpiece: displays [ arch ] when idle, switches to track title when playing

player_status=$(playerctl status 2>/dev/null || echo "Stopped")

if [[ "$player_status" == "Playing" || "$player_status" == "Paused" ]]; then
    artist=$(playerctl metadata --format '{{artist}}' 2>/dev/null)
    title=$(playerctl metadata --format '{{title}}' 2>/dev/null)
    
    if [[ -n "$artist" && -n "$title" ]]; then
        track="${artist} - ${title}"
    elif [[ -n "$title" ]]; then
        track="$title"
    else
        track=""
    fi
    
    if [[ -n "$track" ]]; then
        # Truncate if longer than 32 characters
        if [ ${#track} -gt 32 ]; then
            track="${track:0:30}..."
        fi
        
        # Escape XML/Pango markup special characters
        track=$(echo "$track" | sed 's/&/\&amp;/g; s/</\&lt;/g; s/>/\&gt;/g')
        
        if [[ "$player_status" == "Paused" ]]; then
            echo "[ 󰏤 ${track} ]"
        else
            echo "[ 󰎈 ${track} ]"
        fi
        exit 0
    fi
fi

# Fallback when no media is playing
echo "[ arch ]"
