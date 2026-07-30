#!/bin/bash

# Kill background sleep timers on exit
trap 'kill $(jobs -p) 2>/dev/null' EXIT

get_vol() {
    out=$(wpctl get-volume @DEFAULT_AUDIO_SINK@)
    vol=$(echo "$out" | awk '{print int($2 * 100)}')
    muted=0
    if echo "$out" | grep -q "\[MUTED\]"; then
        muted=1
    fi
    echo "$vol $muted"
}

print_json() {
    local show_pct=$1
    read vol muted <<< $(get_vol)
    
    if [ "$muted" = "1" ]; then
        icon="󰝟"
        text="[ ${icon} ]"
    else
        if [ "$vol" = "0" ]; then
            icon="󰕿"
        elif [ "$vol" -lt "50" ]; then
            icon="󰖀"
        else
            icon="󰕾"
        fi
        
        if [ "$show_pct" = "1" ]; then
            text="[ ${icon} ${vol}% ]"
        else
            text="[ ${icon} ]"
        fi
    fi
    printf '{"text": "%s", "tooltip": "Volume: %s%%"}\n' "$text" "$vol"
}

# Initial print
print_json 0

TIMER_PID=""

# Listen for volume changes
pactl subscribe 2>/dev/null | grep --line-buffered -e "sink" -e "server" | while read -r line; do
    print_json 1
    
    # Kill previous timer if exists
    if [ -n "$TIMER_PID" ]; then
        kill "$TIMER_PID" 2>/dev/null
    fi
    
    # Start new timer in background
    (
        sleep 3
        print_json 0
    ) &
    TIMER_PID=$!
done
