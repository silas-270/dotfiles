#!/bin/bash
# Wallpaper Cycling Script for Hyprland (Theme-Aware)

THEMES_DIR="$HOME/.config/dotfiles/themes"
[ ! -d "$THEMES_DIR" ] && THEMES_DIR="$HOME/.config/themes"
ACTIVE_THEME_FILE="$HOME/.config/active_theme"
SWAYBG_LINK="$HOME/.cache/wallpaper-home.jpg"

mkdir -p "$HOME/.cache"

# Determine current active theme folder
if [ -f "$ACTIVE_THEME_FILE" ]; then
    ACTIVE_THEME=$(cat "$ACTIVE_THEME_FILE" | tr -d '\n\r')
else
    ACTIVE_THEME="savanna-dusk"
fi

THEME_DIR="$THEMES_DIR/$ACTIVE_THEME"

if [ ! -d "$THEME_DIR" ]; then
    THEME_DIR="$THEMES_DIR/savanna-dusk"
fi

# Get available wallpapers in current theme folder
IFS=$'\n' read -r -d '' -a WPS < <(find "$THEME_DIR" -maxdepth 1 -type f \( -name "*.jpg" -o -name "*.png" -o -name "*.jpeg" -o -name "*.webp" \) | sort && printf '\0')

# Handle startup initialization / restoration
if [ "$1" = "init" ] || [ "$1" = "restore" ]; then
    killall swaybg 2>/dev/null || true
    if [ ${#WPS[@]} -eq 0 ]; then
        rm -f "$SWAYBG_LINK"
        nohup swaybg -c "#000000" >/dev/null 2>&1 &
        exit 0
    fi

    # Check if current link target belongs to the active theme
    CURRENT_TARGET=$(readlink -f "$SWAYBG_LINK" 2>/dev/null)
    TARGET_DIR=$(dirname "$CURRENT_TARGET" 2>/dev/null)

    if [ "$TARGET_DIR" != "$THEME_DIR" ] || [ ! -f "$CURRENT_TARGET" ]; then
        # Link is missing, broken, or pointing to another theme -> default to first wallpaper in active theme
        ln -sf "${WPS[0]}" "$SWAYBG_LINK"
    fi

    nohup swaybg -i "$SWAYBG_LINK" -m fill >/dev/null 2>&1 &
    exit 0
fi

# Show usage / status if no wallpapers exist
if [ ${#WPS[@]} -eq 0 ]; then
    echo "No wallpapers found in theme folder: $THEME_DIR. Applying solid black screen."
    rm -f "$SWAYBG_LINK"
    killall swaybg 2>/dev/null || true
    nohup swaybg -c "#000000" >/dev/null 2>&1 &
    notify-send "Wallpaper" "No wallpapers in theme '${ACTIVE_THEME}'. Using solid black screen." 2>/dev/null || true
    exit 0
fi

# Show usage if no argument
if [ -z "$1" ]; then
    echo "Usage: $0 <next|prev|init|number>"
    echo "Theme: $ACTIVE_THEME"
    echo "Available wallpapers (${#WPS[@]}):"
    for wp in "${WPS[@]}"; do
        basename "$wp"
    done
    exit 1
fi

INPUT="$1"

# Handle cycling (next/prev)
if [ "$INPUT" = "next" ] || [ "$INPUT" = "prev" ]; then
    CURRENT_TARGET=$(readlink -f "$SWAYBG_LINK" 2>/dev/null)
    
    CURRENT_INDEX=-1
    for i in "${!WPS[@]}"; do
        if [ "${WPS[$i]}" = "$CURRENT_TARGET" ]; then
            CURRENT_INDEX=$i
            break
        fi
    done
    
    NUM_WPS=${#WPS[@]}
    if [ "$INPUT" = "next" ]; then
        NEW_INDEX=$(( (CURRENT_INDEX + 1) % NUM_WPS ))
    else
        NEW_INDEX=$(( (CURRENT_INDEX - 1 + NUM_WPS) % NUM_WPS ))
    fi
    
    FULL_PATH="${WPS[$NEW_INDEX]}"
else
    # Handle direct selection by index (1-based)
    if [[ "$INPUT" =~ ^[0-9]+$ ]]; then
        IDX=$((INPUT - 1))
        if [ $IDX -ge 0 ] && [ $IDX -lt ${#WPS[@]} ]; then
            FULL_PATH="${WPS[$IDX]}"
        else
            FULL_PATH="${WPS[0]}"
        fi
    else
        FULL_PATH="$THEME_DIR/$INPUT"
    fi
fi

if [ -f "$FULL_PATH" ]; then
    ln -sf "$FULL_PATH" "$SWAYBG_LINK"
    killall swaybg 2>/dev/null || true
    nohup swaybg -i "$SWAYBG_LINK" -m fill >/dev/null 2>&1 &
    echo "Wallpaper set to: $(basename "$FULL_PATH")"
    notify-send "Wallpaper Changed" "$(basename "$FULL_PATH")" 2>/dev/null || true
else
    echo "Error: Wallpaper '$FULL_PATH' not found."
    exit 1
fi
