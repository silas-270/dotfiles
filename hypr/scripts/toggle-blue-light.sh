#!/bin/bash
SHADER_PATH="/home/silas270/.config/hypr/shaders/blue-light.frag"
CURRENT_SHADER=$(hyprctl getoption decoration:screen_shader -j | grep -oP '"str": "\K[^"]+')

if [ "$CURRENT_SHADER" = "[[EMPTY]]" ] || [ -z "$CURRENT_SHADER" ] || [ "$CURRENT_SHADER" = "null" ]; then
    hyprctl eval "hl.config({ decoration = { screen_shader = '$SHADER_PATH' } })"
else
    hyprctl eval "hl.config({ decoration = { screen_shader = '[[EMPTY]]' } })"
fi
