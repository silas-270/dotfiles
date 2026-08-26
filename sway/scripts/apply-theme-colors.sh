#!/bin/bash
# Reads the active theme and applies window border colors to the running Sway session.

THEME_NAME=$(cat "$HOME/.config/active_theme" 2>/dev/null || echo "savanna-dusk")
THEME_JSON="$HOME/dotfiles/themes/$THEME_NAME/theme.json"

[ -f "$THEME_JSON" ] || exit 0

ACCENT=$(python3 -c "import json; t=json.load(open('$THEME_JSON')); print(t['colors']['accent'])")
INACTIVE=$(python3 -c "import json; t=json.load(open('$THEME_JSON')); print(t['colors']['bg_input'])")
BG=$(python3 -c "import json; t=json.load(open('$THEME_JSON')); print(t['colors']['bg_base'])")
INACTIVE_BORDER="${INACTIVE}33"

swaymsg "client.focused $ACCENT $BG #ffffff $ACCENT $ACCENT"
swaymsg "client.unfocused $INACTIVE_BORDER $BG #ffffff $INACTIVE_BORDER $INACTIVE_BORDER"
