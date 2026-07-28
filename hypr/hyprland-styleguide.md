# Hyprland Ecosystem Style Guide

This style guide is based on your current configurations. Your setup features a warm, earthy, "latte"-style aesthetic with soft, rounded corners, snappy animations, and a cohesive gap structure. Use this guide as a reference when styling new components (like application launchers, notification daemons, or lock screens) to ensure a unified desktop experience.

## 1. Color Palette

Your setup relies on a warm, cream-and-brown color scheme with translucent elements for a modern feel.

### Base Backgrounds
* **Solid:** `#F5E8CC` (Used as the primary Waybar background)
* **Translucent Highlight:** `rgba(196, 122, 16, 0.13)` to `0.18` (Used for active modules and logos)

### Text & Foreground
* **Primary Text:** `#3A2408` (Deep brown)
* **Secondary/Muted Text:** `#5A3A14` and `#7A5020`

### Accents
* **Primary Accent (Orange/Amber):** `#C47A10` (Used for the clock, logo, and active workspaces)
* **Secondary Accent/Hover:** `#B85510` (Used for network info and hover states)

### Window Borders
* **Active Window:** `0xffffffaa` (Translucent white)
* **Inactive Window:** `0x00000000` (Fully transparent)
* **UI Borders/Dividers:** `rgba(180, 110, 30, 0.25)` (Used for Waybar separators and bottom border)

### State Colors
* **Warning:** `#B06010` (Used for low battery)
* **Critical/Urgent:** `#B02010` (Used for urgent workspaces and critical battery)

## 2. Geometry & Spacing

A consistent gap and border-radius structure is critical to making different UI elements look like they belong together.

### Gaps & Margins
* **Outer Gaps:** `10px`
* **Inner Gaps:** `5px`
* **Integration Note:** Top panels (like Waybar) should have a left and right margin of `10px` to perfectly align with the outer window gaps.

### Border Radius (Rounding)
* **Main Windows:** `12px`
* **Main UI Containers:** `14px` (e.g., Waybar outer edge)
* **Inner UI Elements:** `10px` (e.g., buttons, pill modules)

### Border Thickness
* **Global border width:** `1px`
* **UI element borders:** `1px solid`

## 3. Typography

* **Primary Font Family:** `"JetBrainsMono Nerd Font"`, falling back to `"JetBrains Mono"`, `monospace`
* **Base Font Size:** `12px`
* **Font Weights:** Use standard weights for normal text, and `700` (bold) for highlighted elements like the logo, active workspaces, and the clock.

## 4. Visual Effects & Animations

Your aesthetic leans into a flat, fast, and slightly frosted look, entirely eschewing drop shadows.

### Blur
* **Settings:** Enabled globally with `size = 8` and `passes = 3`
* **Integration Note:** Ensure layer rules apply blur to floating UI elements (like Rofi and Waybar).

### Shadows
* Disabled completely for a cleaner, flatter look.

### Animations
* **Feel:** Snappy and responsive.
* **Bezier Curve:** `snappy, 0.05, 0.9, 0.1, 1.05`
* **Speed/Duration:** Quick, ranging between `1` and `2`
