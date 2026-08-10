#!/bin/bash
# Theme Switcher Script for Hyprland / Rofi / Wallust

THEMES_DIR="$HOME/dotfiles/themes"
ACTIVE_THEME_FILE="$HOME/.config/active_theme"
SWAYBG_LINK="$HOME/Bilder/Wallpaper/wallpaper-home.jpg"

mkdir -p "$HOME/.config" "$HOME/Bilder/Wallpaper"

declare -A THEME_PATHS
declare -A THEME_NAMES

if [ -d "$THEMES_DIR" ]; then
    for theme_dir in "$THEMES_DIR"/*/; do
        [ -d "$theme_dir" ] || continue
        folder_name=$(basename "$theme_dir")
        json_file="$theme_dir/theme.json"
        
        if [ -f "$json_file" ]; then
            display_name=$(python3 -c "import json; print(json.load(open('$json_file')).get('name', '$folder_name'))" 2>/dev/null)
        else
            display_name="$folder_name"
        fi
        
        [ -z "$display_name" ] && display_name="$folder_name"
        
        menu_key="🎨 $display_name"
        THEME_PATHS["$menu_key"]="$folder_name"
        THEME_NAMES["$menu_key"]="$display_name"
    done
fi

# Build menu list for Rofi
MENU_OPTIONS=""
for key in "${!THEME_PATHS[@]}"; do
    MENU_OPTIONS+="${key}\n"
done

# Show Rofi Menu
SELECTION=$(echo -e "$MENU_OPTIONS" | sort | rofi -dmenu -p "Select Theme" -i)

[ -z "$SELECTION" ] && exit 0

FOLDER_NAME="${THEME_PATHS[$SELECTION]}"
DISPLAY_NAME="${THEME_NAMES[$SELECTION]}"
THEME_DIR="$THEMES_DIR/$FOLDER_NAME"
JSON_FILE="$THEME_DIR/theme.json"

if [ -z "$FOLDER_NAME" ] || [ ! -d "$THEME_DIR" ]; then
    echo "Error: Selected theme '$SELECTION' not found."
    exit 1
fi

# Save active theme name
echo "$FOLDER_NAME" > "$ACTIVE_THEME_FILE"

# 1. Apply Wallust / Color palette
if command -v wallust &>/dev/null && [ -f "$JSON_FILE" ]; then
    wallust cs "$JSON_FILE" 2>/dev/null || wallust run "$JSON_FILE" 2>/dev/null || true
fi

# 2. Set Wallpaper (or solid black fallback if no wallpapers exist)
IFS=$'\n' read -r -d '' -a WPS < <(find "$THEME_DIR" -maxdepth 1 -type f \( -name "*.jpg" -o -name "*.png" -o -name "*.jpeg" -o -name "*.webp" \) | sort && printf '\0')

killall swaybg 2>/dev/null

if [ ${#WPS[@]} -gt 0 ]; then
    FIRST_WP="${WPS[0]}"
    ln -sf "$FIRST_WP" "$SWAYBG_LINK"
    nohup swaybg -i "$SWAYBG_LINK" -m fill >/dev/null 2>&1 &
else
    # Solid black screen fallback
    nohup swaybg -c "#000000" >/dev/null 2>&1 &
fi

# 3. Reload Waybar & Hyprland
killall -SIGUSR2 waybar 2>/dev/null || true
hyprctl reload 2>/dev/null || true

# 4. Desktop Notification
notify-send "Theme Changed" "Switched to ${DISPLAY_NAME}" -i preferences-desktop-theme 2>/dev/null || true
