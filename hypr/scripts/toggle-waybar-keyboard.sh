#!/usr/bin/env bash
# toggle-waybar-keyboard.sh - Toggle Waybar keyboard navigation mode

CONFIG_FILE="$HOME/dotfiles/waybar/config"
STATE_FILE="/tmp/waybar_kb_mode"

if [ -f "$STATE_FILE" ]; then
    rm -f "$STATE_FILE"
    sed -i 's/"keyboard-interactive": true,/"keyboard-interactive": false,/g' "$CONFIG_FILE"
else
    touch "$STATE_FILE"
    sed -i 's/"keyboard-interactive": false,/"keyboard-interactive": true,/g' "$CONFIG_FILE"
fi

pkill -USR2 waybar
