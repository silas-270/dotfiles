#!/bin/bash
# Toggles eDP-1 on/off when keybind is pressed. Runs once and exits (no background daemon).

LAPTOP="eDP-1"
WALLPAPER="/home/silas270/.cache/wallpaper-home.jpg"

if [ -n "$SWAYSOCK" ]; then
    is_active=$(swaymsg -t get_outputs -r | jq -r ".[] | select(.name==\"$LAPTOP\") | .active")
    if [ "$is_active" = "true" ]; then
        swaymsg output "$LAPTOP" disable
    else
        swaymsg output "$LAPTOP" enable
        sleep 0.3
        if [ -f "$WALLPAPER" ]; then
            pkill swaybg 2>/dev/null || true
            nohup swaybg -i "$WALLPAPER" -m fill >/dev/null 2>&1 &
        fi
        killall waybar 2>/dev/null || true
        sleep 0.2
        nohup waybar >/dev/null 2>&1 &
    fi
else
    # Check if eDP-1 is currently active (not disabled)
    is_active=$(hyprctl monitors -j | jq -r ".[] | select(.name==\"$LAPTOP\") | .name")

    if [ "$is_active" = "$LAPTOP" ]; then
        # Currently enabled -> disable laptop screen
        hyprctl eval "hl.monitor({ output = \"$LAPTOP\", disabled = true })"
    else
        # Currently disabled -> enable laptop screen & refresh swaybg/waybar
        hyprctl eval "hl.monitor({ output = \"$LAPTOP\", mode = \"preferred\", position = \"auto\", scale = \"1\", disabled = false })"
        sleep 0.3
        if [ -f "$WALLPAPER" ]; then
            pkill swaybg 2>/dev/null || true
            nohup swaybg -i "$WALLPAPER" -m fill >/dev/null 2>&1 &
        fi
        killall waybar 2>/dev/null || true
        sleep 0.2
        nohup waybar >/dev/null 2>&1 &
    fi
fi
