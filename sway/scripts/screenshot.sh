#!/bin/bash
MODE=$1
FILE="$HOME/Pictures/screenshot_$(date +%Y%m%d_%H%M%S).png"
mkdir -p "$HOME/Pictures"

if [ "$MODE" = "region" ]; then
    grim -g "$(slurp)" "$FILE"
elif [ "$MODE" = "window" ]; then
    grim -g "$(slurp)" "$FILE"
elif [ "$MODE" = "output" ]; then
    grim -c "$FILE"
fi

if [ -f "$FILE" ]; then
    wl-copy < "$FILE"
    notify-send "Screenshot saved" "$FILE"
fi
