#!/usr/bin/env bash
# Dynamic RAM display for Waybar with U+2004 space brackets: Shows GB with 1 decimal if >= 1024MB, or MB if < 1024MB

used_mb=$(free -m | awk '/^Mem:/ {print $3}')

if [ "$used_mb" -ge 1024 ]; then
    used_gb=$(awk "BEGIN {printf \"%.1f\", $used_mb/1024}")
    echo "[ RAM ${used_gb}G ]"
else
    echo "[ RAM ${used_mb}M ]"
fi
