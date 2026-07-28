#!/usr/bin/env bash
# Script to toggle keyd system service and update waybar icon

if systemctl is-active --quiet keyd; then
    sudo systemctl stop keyd
else
    sudo systemctl start keyd
fi

# Send SIGRTMIN+8 to Waybar to refresh the custom/keyd module immediately
pkill -RTMIN+8 waybar
