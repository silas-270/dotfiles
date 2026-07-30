#!/usr/bin/env bash
# Change brightness up or down by percentage step (e.g. +5% or -5%)

set -euo pipefail

DIRECTION="${1:-+5%}"

if command -v brightnessctl >/dev/null 2>&1; then
    brightnessctl set "$DIRECTION" && exit 0
fi

if command -v light >/dev/null 2>&1; then
    if [[ "$DIRECTION" == *"+"* ]]; then
        light -A 5 && exit 0
    else
        light -U 5 && exit 0
    fi
fi

# Fallback: systemd-logind DBus via busctl
DEV=$(ls /sys/class/backlight/ 2>/dev/null | head -n1)
if [[ -n "$DEV" && -f "/sys/class/backlight/$DEV/brightness" ]]; then
    CUR=$(cat "/sys/class/backlight/$DEV/brightness")
    MAX=$(cat "/sys/class/backlight/$DEV/max_brightness")
    STEP=$((MAX / 20))

    if [[ "$DIRECTION" == *"-"* ]]; then
        NEW=$((CUR - STEP))
        [[ $NEW -lt 0 ]] && NEW=0
    else
        NEW=$((CUR + STEP))
        [[ $NEW -gt $MAX ]] && NEW=$MAX
    fi

    busctl call org.freedesktop.login1 /org/freedesktop/login1/session/auto org.freedesktop.login1.Session SetBrightness ssu "backlight" "$DEV" "$NEW" >/dev/null 2>&1 || true
fi
