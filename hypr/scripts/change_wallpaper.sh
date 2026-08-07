#!/bin/bash

# Configuration
WALLPAPER_DIR="$HOME/dotfiles/wallpapers"
TARGET_LINK="$WALLPAPER_DIR/wallpaper-home.jpg"
SWAYBG_LINK="$HOME/Bilder/Wallpaper/wallpaper-home.jpg"

# Ensure wallpaper1.jpg exists if it hasn't been backed up yet
if [ -f "$TARGET_LINK" ] && [ ! -L "$TARGET_LINK" ]; then
    mv "$TARGET_LINK" "$WALLPAPER_DIR/wallpaper1.jpg"
    ln -sf "$WALLPAPER_DIR/wallpaper1.jpg" "$TARGET_LINK"
fi

# Get available wallpapers in a sorted array
IFS=$'\n' read -r -d '' -a WPS < <(find "$WALLPAPER_DIR" -type f \( -name "*.jpg" -o -name "*.png" \) ! -name "wallpaper-home.jpg" | sort && printf '\0')

# Show usage if no argument
if [ -z "$1" ]; then
    echo "Usage: $0 <number|name|next|prev>"
    echo "Examples: $0 2     (to set wallpaper2.jpg)"
    echo "          $0 next  (to go to the next wallpaper)"
    echo "          $0 prev  (to go to the previous wallpaper)"
    echo ""
    echo "Available wallpapers:"
    for wp in "${WPS[@]}"; do
        basename "$wp"
    done
    exit 1
fi

INPUT="$1"

# Handle cycling (next/prev)
if [ "$INPUT" = "next" ] || [ "$INPUT" = "prev" ]; then
    # Get the current resolved wallpaper path
    CURRENT_TARGET=$(readlink -f "$TARGET_LINK")
    
    # Find current index
    CURRENT_INDEX=-1
    for i in "${!WPS[@]}"; do
        if [ "${WPS[$i]}" = "$CURRENT_TARGET" ]; then
            CURRENT_INDEX=$i
            break
        fi
    done
    
    # Calculate new index
    NUM_WPS=${#WPS[@]}
    if [ "$INPUT" = "next" ]; then
        NEW_INDEX=$(( (CURRENT_INDEX + 1) % NUM_WPS ))
    else
        NEW_INDEX=$(( (CURRENT_INDEX - 1 + NUM_WPS) % NUM_WPS ))
    fi
    
    FULL_PATH="${WPS[$NEW_INDEX]}"
else
    # Handle direct wallpaper selection (by number or name)
    if [[ "$INPUT" =~ ^[0-9]+$ ]]; then
        WP_FILE="wallpaper${INPUT}.jpg"
    else
        WP_FILE="$INPUT"
    fi

    FULL_PATH="$WALLPAPER_DIR/$WP_FILE"

    # Verify file exists, or try fallback extensions/prefixes
    if [ ! -f "$FULL_PATH" ]; then
        if [ -f "$WALLPAPER_DIR/${INPUT}.png" ]; then
            FULL_PATH="$WALLPAPER_DIR/${INPUT}.png"
        elif [ -f "$WALLPAPER_DIR/wallpaper${INPUT}.png" ]; then
            FULL_PATH="$WALLPAPER_DIR/wallpaper${INPUT}.png"
        else
            echo "Error: Wallpaper '$WP_FILE' not found in $WALLPAPER_DIR"
            exit 1
        fi
    fi
fi

# Update the symlink
ln -sf "$FULL_PATH" "$TARGET_LINK"

# Restart swaybg to apply the change immediately
killall swaybg 2>/dev/null
nohup swaybg -i "$SWAYBG_LINK" -m fill >/dev/null 2>&1 &

echo "Wallpaper successfully changed to $(basename "$FULL_PATH")!"
