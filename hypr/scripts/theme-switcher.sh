#!/bin/bash
# Theme Switcher Script for Hyprland / Rofi / Wallust

PRESETS_DIR="$HOME/dotfiles/wallust/presets"
APPLY_SCRIPT="$HOME/dotfiles/theme/apply.py"
WALLPAPER_PATH="$HOME/Bilder/Wallpaper/wallpaper-home.jpg"

declare -A THEMES

# Populate themes map from JSON files in PRESETS_DIR
if [ -d "$PRESETS_DIR" ]; then
    for json_file in "$PRESETS_DIR"/*.json; do
        [ -e "$json_file" ] || continue
        filename=$(basename "$json_file")
        display_name=$(python3 -c "import json; print(json.load(open('$json_file')).get('name', '${filename%.json}'))" 2>/dev/null)
        if [ -z "$display_name" ]; then
            display_name="${filename%.json}"
        fi
        THEMES["󰏘 $display_name"]="$json_file"
    done
fi

# Additional Wallpaper Option
THEMES["🖼️ Auto-Generate from Wallpaper"]="WALLPAPER"

# Build menu list for Rofi
MENU_OPTIONS=""
for key in "${!THEMES[@]}"; do
    MENU_OPTIONS+="${key}\n"
done

# Show Rofi Menu
SELECTION=$(echo -e "$MENU_OPTIONS" | sort | rofi -dmenu -p "🎨 Select Theme" -i)

[ -z "$SELECTION" ] && exit 0

TARGET="${THEMES[$SELECTION]}"

if [ "$TARGET" = "WALLPAPER" ]; then
    # Wallpaper mode
    if command -v wallust &>/dev/null; then
        wallust run "$WALLPAPER_PATH" 2>/dev/null || true
    fi
    THEME_NAME="Wallpaper Palette"
elif [ -n "$TARGET" ] && [ -f "$TARGET" ]; then
    # Apply theme colors via apply.py
    if [ -f "$APPLY_SCRIPT" ]; then
        python3 "$APPLY_SCRIPT" "$TARGET"
    fi
    
    # Run wallust if available
    if command -v wallust &>/dev/null; then
        wallust cs "$TARGET" 2>/dev/null || wallust run "$TARGET" 2>/dev/null || true
    fi
    
    THEME_NAME=$(python3 -c "import json; print(json.load(open('$TARGET')).get('name', 'Preset'))" 2>/dev/null)
else
    echo "Unknown selection or file not found: $SELECTION"
    exit 1
fi

# Reload Waybar & Hyprland
killall -SIGUSR2 waybar 2>/dev/null || true
hyprctl reload 2>/dev/null || true

# Desktop Notification
notify-send "Theme Changed" "Switched to ${THEME_NAME}" -i preferences-desktop-theme 2>/dev/null || true
