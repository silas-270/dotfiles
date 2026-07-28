#!/usr/bin/env bash
# Script for Waybar custom module: outputs keyboard icon when keyd is active

if systemctl is-active --quiet keyd; then
    echo '{"text": "󰌌", "class": "active", "tooltip": "keyd ist aktiv"}'
else
    echo '{"text": "", "class": "inactive", "tooltip": "keyd ist inaktiv"}'
fi
