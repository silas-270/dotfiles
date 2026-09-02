#!/usr/bin/env bash
# Toggles mouse/touchpad input devices and cursor visibility on/off in Hyprland

STATE_FILE="/tmp/hypr_mouse_disabled"

if [ -f "$STATE_FILE" ]; then
    rm -f "$STATE_FILE"
    NEW_STATE="true"
    CURSOR_INVISIBLE="false"
    TITLE="Keyboard & Mouse Mode"
    COLOR="rgb(a6e3a1)"
    ICON_ID=6
else
    touch "$STATE_FILE"
    NEW_STATE="false"
    CURSOR_INVISIBLE="true"
    TITLE="Keyboard Only Mode"
    COLOR="rgb(f38ba8)"
    ICON_ID=1
fi

# 1. Update cursor visibility in Hyprland
hyprctl eval "hl.config({ cursor = { invisible = $CURSOR_INVISIBLE } })" >/dev/null 2>&1 || true

# 2. If entering keyboard-only mode, warp cursor off active content so hover styles don't stay active
if [ "$NEW_STATE" = "false" ]; then
    hyprctl eval "hl.dispatch(hl.dsp.cursor.move(99999, 99999))" >/dev/null 2>&1 || true
fi

# 3. Disable/Enable all detected pointing devices
mapfile -t MICE < <(hyprctl devices -j 2>/dev/null | jq -r '.mice[].name' 2>/dev/null)

if [ ${#MICE[@]} -eq 0 ]; then
    MICE=("dell092a:00-06cb:cca6-mouse" "dell092a:00-06cb:cca6-touchpad")
fi

for dev in "${MICE[@]}"; do
    hyprctl eval "hl.device({ name = \"$dev\", enabled = $NEW_STATE })" >/dev/null 2>&1 || true
done

# 4. Show on-screen notification
hyprctl notify "$ICON_ID" 1500 "$COLOR" "$TITLE" >/dev/null 2>&1 || true
