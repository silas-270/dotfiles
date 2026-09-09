#!/usr/bin/env bash
# Screenshot the active window and open an AI chat overlay about it.
set -euo pipefail

if [ "${1:-}" = "--demo" ]; then
    exec python3 "$HOME/.config/hypr/scripts/ask-ai-overlay.py" --demo
fi

DIR="/tmp/ask-ai"
mkdir -p "$DIR"
FILE="shot-$(date +%s%N).png"

hyprshot -m window -m active -s -o "$DIR" -f "$FILE" || true

IMG="$DIR/$FILE"
if [ ! -f "$IMG" ]; then
    notify-send "Ask AI" "Screenshot failed" || true
    exit 1
fi

exec python3 "$HOME/.config/hypr/scripts/ask-ai-overlay.py" "$IMG"

