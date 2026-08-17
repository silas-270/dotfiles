#!/usr/bin/env bash

# Power Menu Options (Icons)
shutdown="󰐥"
reboot="󰜉"
logout="󰍃"

options="$shutdown\n$reboot\n$logout"

# Show Rofi dmenu
chosen=$(echo -e "$options" | rofi -dmenu -i -theme ~/.config/rofi/powermenu.rasi)

# Match selection robustly and execute
case "$chosen" in
    *"$shutdown"*)
        systemctl poweroff
        ;;
    *"$reboot"*)
        systemctl reboot
        ;;
    *"$logout"*)
        if [ -n "$SWAYSOCK" ]; then
            swaymsg exit
        else
            uwsm stop || hyprctl dispatch exit 0 || loginctl terminate-session "${XDG_SESSION_ID:-self}"
        fi
        ;;
esac
