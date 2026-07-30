#!/usr/bin/env bash
# Change volume raise/lower/toggle

set -euo pipefail

ACTION="${1:-toggle}"

if command -v wpctl >/dev/null 2>&1; then
    case "$ACTION" in
        raise)  wpctl set-volume -l 1.0 @DEFAULT_AUDIO_SINK@ 5%+ ;;
        lower)  wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%- ;;
        toggle) wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle ;;
    esac
    exit 0
fi

if command -v pactl >/dev/null 2>&1; then
    case "$ACTION" in
        raise)  pactl set-sink-volume @DEFAULT_SINK@ +5% ;;
        lower)  pactl set-sink-volume @DEFAULT_SINK@ -5% ;;
        toggle) pactl set-sink-mute @DEFAULT_SINK@ toggle ;;
    esac
    exit 0
fi

if command -v amixer >/dev/null 2>&1; then
    case "$ACTION" in
        raise)  amixer sset Master 5%+ ;;
        lower)  amixer sset Master 5%- ;;
        toggle) amixer sset Master toggle ;;
    esac
    exit 0
fi
