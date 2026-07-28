#!/usr/bin/env bash

# Power Menu Options (Nur Icons)
shutdown="󰐥"
reboot="󰜉"
logout="󰍃"

options="$shutdown\n$reboot\n$logout"

# Show Rofi dmenu
chosen=$(echo -e "$options" | rofi -dmenu -i -theme ~/.config/rofi/powermenu.rasi)

case $chosen in
    $shutdown)
        systemctl poweroff
        ;;
    $reboot)
        systemctl reboot
        ;;
    $logout)
        hyprctl dispatch exit
        ;;
esac
