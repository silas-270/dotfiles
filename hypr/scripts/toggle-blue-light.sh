#!/bin/bash
SHADER_PATH="/home/silas270/.config/hypr/shaders/blue-light.frag"
CURRENT_SHADER=$(hyprctl getoption decoration:screen_shader -j | jq -r '.str')

if [ "$CURRENT_SHADER" = "[[EMPTY]]" ] || [ -z "$CURRENT_SHADER" ]; then
    hyprctl keyword decoration:screen_shader "$SHADER_PATH"
else
    hyprctl keyword decoration:screen_shader "[[EMPTY]]"
fi
