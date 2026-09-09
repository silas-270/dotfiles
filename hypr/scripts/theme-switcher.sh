#!/usr/bin/env bash
# Theme Switcher Script for Hyprland / Rofi / Wallust

export PATH="$HOME/.cargo/bin:$PATH"

THEMES_DIR="$HOME/.config/dotfiles/themes"
[ ! -d "$THEMES_DIR" ] && THEMES_DIR="$HOME/.config/themes"
ACTIVE_THEME_FILE="$HOME/.config/active_theme"
SWAYBG_LINK="$HOME/.cache/wallpaper-home.jpg"
ROFI_THEME="$HOME/.config/rofi/theme.rasi"

mkdir -p "$HOME/.config" "$HOME/.cache"

# Determine current active theme
ACTIVE_THEME=""
if [ -f "$ACTIVE_THEME_FILE" ]; then
    ACTIVE_THEME=$(tr -d '\n\r' < "$ACTIVE_THEME_FILE")
fi

DISPLAY_LIST=()
FOLDER_LIST=()
ACTIVE_INDEX=0
CURRENT_INDEX=0

if [ -d "$THEMES_DIR" ]; then
    # Read theme directories sorted alphabetically
    while IFS= read -r theme_dir; do
        [ -d "$theme_dir" ] || continue
        folder_name=$(basename "$theme_dir")
        json_file="$theme_dir/theme.json"
        
        display_name=""
        if [ -f "$json_file" ] && command -v jq >/dev/null 2>&1; then
            display_name=$(jq -r '.name // empty' "$json_file" 2>/dev/null)
        fi
        
        [ -z "$display_name" ] && display_name="$folder_name"
        
        if [ "$folder_name" = "$ACTIVE_THEME" ]; then
            label="󰄬  ${display_name}  (active)"
            ACTIVE_INDEX=$CURRENT_INDEX
        else
            label="󰏘  ${display_name}"
        fi
        
        DISPLAY_LIST+=("$label")
        FOLDER_LIST+=("$folder_name")
        ((CURRENT_INDEX++))
    done < <(find "$THEMES_DIR" -maxdepth 1 -mindepth 1 -type d | sort)
fi

if [ ${#DISPLAY_LIST[@]} -eq 0 ]; then
    notify-send "Theme Switcher" "No themes found in $THEMES_DIR" -u critical
    exit 1
fi

# Build Rofi command
ROFI_CMD=(rofi -dmenu -i -p "󰏘 Theme" -selected-row "$ACTIVE_INDEX")
if [ -f "$ROFI_THEME" ]; then
    ROFI_CMD+=(-theme "$ROFI_THEME")
fi

# Show Rofi menu
SELECTION=$(printf '%s\n' "${DISPLAY_LIST[@]}" | "${ROFI_CMD[@]}")

[ -z "$SELECTION" ] && exit 0

# Match selection to folder name
SELECTED_FOLDER=""
SELECTED_DISPLAY=""
for i in "${!DISPLAY_LIST[@]}"; do
    if [ "${DISPLAY_LIST[$i]}" = "$SELECTION" ]; then
        SELECTED_FOLDER="${FOLDER_LIST[$i]}"
        SELECTED_DISPLAY=$(echo "${DISPLAY_LIST[$i]}" | sed -E 's/^(󰄬|󰏘)[[:space:]]+//; s/[[:space:]]+\(active\)$//')
        break
    fi
done

if [ -z "$SELECTED_FOLDER" ]; then
    echo "Error: Selected theme '$SELECTION' not found."
    exit 1
fi

THEME_DIR="$THEMES_DIR/$SELECTED_FOLDER"
JSON_FILE="$THEME_DIR/theme.json"

if [ ! -d "$THEME_DIR" ]; then
    echo "Error: Theme directory '$THEME_DIR' not found."
    exit 1
fi

# Save active theme name
echo "$SELECTED_FOLDER" > "$ACTIVE_THEME_FILE"

# 1. Apply Wallust Color Palette
WALLUST_BIN=$(command -v wallust || echo "$HOME/.cargo/bin/wallust")
if [ -f "$WALLUST_BIN" ] && [ -f "$JSON_FILE" ]; then
    "$WALLUST_BIN" cs "$JSON_FILE" >/dev/null 2>&1
fi

# 1b. Compile theme colors via apply.py to preserve custom fields
python3 "$HOME/.config/dotfiles/theme/apply.py" "$JSON_FILE" >/dev/null 2>&1 || true

# 2. Set Wallpaper (or solid black fallback if no wallpapers exist)
IFS=$'\n' read -r -d '' -a WPS < <(find "$THEME_DIR" -maxdepth 1 -type f \( -name "*.jpg" -o -name "*.png" -o -name "*.jpeg" -o -name "*.webp" \) | sort && printf '\0')

pkill -x swaybg 2>/dev/null || true

if [ ${#WPS[@]} -gt 0 ]; then
    FIRST_WP="${WPS[0]}"
    ln -sf "$FIRST_WP" "$SWAYBG_LINK"
    nohup swaybg -i "$SWAYBG_LINK" -m fill >/dev/null 2>&1 &
else
    # Solid black screen fallback
    rm -f "$SWAYBG_LINK"
    nohup swaybg -c "#000000" >/dev/null 2>&1 &
fi

# 3. Reload Waybar, Rustbar & Hyprland/Sway
killall -SIGUSR2 waybar 2>/dev/null || true

# Restart Rustbar on Sway & Hyprland
pkill -x rustbar 2>/dev/null || true
sleep 0.1
RUSTBAR_BIN="$HOME/.config/hypr/scripts/target/release/rustbar"
if [ ! -x "$RUSTBAR_BIN" ]; then
    RUSTBAR_BIN="$HOME/.config/dotfiles/hypr/scripts/target/release/rustbar"
fi
if [ -x "$RUSTBAR_BIN" ]; then
    setsid "$RUSTBAR_BIN" >/dev/null 2>&1 &
fi

if [ -n "$SWAYSOCK" ]; then
    swaymsg reload 2>/dev/null || true
else
    hyprctl reload 2>/dev/null || true
fi

# 4. Desktop Notification
notify-send "Theme Changed" "Switched to ${SELECTED_DISPLAY}" -i preferences-desktop-theme 2>/dev/null || true
