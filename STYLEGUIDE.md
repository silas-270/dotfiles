# Savanna Dusk (African Sunset) Desktop Theme Style Guide

## 1. General Theme Concept

The **Savanna Dusk** theme is inspired by African sunset landscapes and savanna dusk horizons. It pairs deep, warm, earthy terracotta backgrounds (`#54382B`) with a vibrant sunset amber accent (`#D97736`) and soft, warm ivory foreground elements (`#F2E3D5`). 

The visual design combines a clean TUI/ASCII-inspired structure—utilizing fieldsets, inner rectangular boxes, and bracket enclosures (`[ ]`, `( )`)—with modern Wayland translucency, custom font calibration, and precision vector rendering.

---

## 2. Master Color Palette

The theme colors are centrally managed in a single JSON definition and compiled into format-specific config files for Waybar, Rofi, Hyprland, and Kitty.

* **Master Palette File:** [`~/dotfiles/theme/palette.json`](file:///home/silas270/dotfiles/theme/palette.json)
* **Generator Script:** [`~/dotfiles/theme/apply.py`](file:///home/silas270/dotfiles/theme/apply.py)

### Generated Target Exports
* **CSS (Waybar):** [`~/dotfiles/theme/generated/colors.css`](file:///home/silas270/dotfiles/theme/generated/colors.css)
* **Lua (Hyprland):** [`~/dotfiles/theme/generated/colors.lua`](file:///home/silas270/dotfiles/theme/generated/colors.lua)
* **Rasi (Rofi):** [`~/dotfiles/theme/generated/colors.rasi`](file:///home/silas270/dotfiles/theme/generated/colors.rasi)
* **Conf (Kitty / Shell):** [`~/dotfiles/theme/generated/colors.conf`](file:///home/silas270/dotfiles/theme/generated/colors.conf)
* **Hyprlock:** [`~/dotfiles/theme/generated/colors-hyprlock.conf`](file:///home/silas270/dotfiles/theme/generated/colors-hyprlock.conf)

### Color Tokens & Assignments

| Token Key | Hex / Value | Description & Primary Usage |
| :--- | :--- | :--- |
| `bg_base` | `#54382B` | Deep earthy savanna brown background for windows, surfaces, and control center. |
| `bg_input` | `#664839` | Slightly lighter brown for input boxes and active tile surfaces. |
| `bg_box` | `rgba(217, 119, 54, 0.15)` | Translucent amber highlight for active inner boxes and hovered tiles. |
| `fg_primary` | `#F2E3D5` | Warm ivory text for primary titles, values, and control buttons. |
| `fg_muted` | `#C2AA95` | Muted sand color for secondary text, timestamps, and subtitles. |
| `accent` | `#D97736` | Sunset orange accent for active toggles, section headers, badges, and slider thumbs (`●`). |
| `accent_hover` | `#E58544` | Lighter sunset orange for hover states and active highlights. |
| `border` | `#7A523D` | Terracotta outline for section fieldset borders and inner box dividers. |
| `hypr_active_border` | `0xffd97736` | Opaque sunset orange border for active Hyprland windows. |
| `hypr_inactive_border`| `0x3354382b` | Translucent brown border for inactive Hyprland windows. |

---

## 3. Design System & Typography Specifics

### Font & Metrics
* **Font Family:** `"JetBrainsMono Nerd Font"` (Monospaced, used across all applications and custom daemons).
* **Font Size:** Standard `12px` (calibrated in custom renderers for crisp layout calculations).

### Rectangles & Fieldset Outlines
* **Outer Section Fieldsets:** Outlined rectangles with a section title badge embedded directly into the top border line (e.g. `── CONNECTIONS ───`).
* **Inner Box Rects:** Drawn using `draw_inner_box` with a `1.5px` border of subtle terracotta (`#7A523D` / `rgba(103, 69, 52, 0.75)`).

### Bracket Enclosure Styling
All status badges, button icons, tags, and progress indicators use bracket enclosures:
* **Status Badges:** `[ ON ]`, `[ OFF ]`, `[ MUTE ]`
* **Icon Tags:** `( 󰤨 )`, `( 󰂯 )`, `( 󰕾 )`
* **Sliders & Track Thumbs:** `(========●--------)`
* **Media Seek Rail:** `0:42 (────●────────) 3:15`

---

## 4. Component Implementation Reference

Below is a sitemap of UI components created across the desktop ecosystem, along with their primary source code files:

### A. Control Center (Custom Rust / Wayland Layer-Shell App)
* **Main Application & Surface Management:** [`/home/silas270/dotfiles/hypr/scripts/control-center/src/main.rs`](file:///home/silas270/dotfiles/hypr/scripts/control-center/src/main.rs)
* **Grid & Layout Math:** [`/home/silas270/dotfiles/hypr/scripts/control-center/src/grid.rs`](file:///home/silas270/dotfiles/hypr/scripts/control-center/src/grid.rs)
* **Tiny-Skia Canvas & Font Renderer:** [`/home/silas270/dotfiles/hypr/scripts/control-center/src/render.rs`](file:///home/silas270/dotfiles/hypr/scripts/control-center/src/render.rs)
* **Fieldset & Inner Box Outlines:** [`/home/silas270/dotfiles/hypr/scripts/control-center/src/widgets/fieldset.rs`](file:///home/silas270/dotfiles/hypr/scripts/control-center/src/widgets/fieldset.rs)
* **Connections Section (Wi-Fi & Bluetooth Tiles):** [`/home/silas270/dotfiles/hypr/scripts/control-center/src/widgets/connections.rs`](file:///home/silas270/dotfiles/hypr/scripts/control-center/src/widgets/connections.rs)
* **Controls Section (Brightness & Volume Sliders with `●` Thumbs):** [`/home/silas270/dotfiles/hypr/scripts/control-center/src/widgets/controls.rs`](file:///home/silas270/dotfiles/hypr/scripts/control-center/src/widgets/controls.rs)
* **Media Player Section (`ARTIST - TITLE` & Interactive Seek Rail):** [`/home/silas270/dotfiles/hypr/scripts/control-center/src/widgets/media.rs`](file:///home/silas270/dotfiles/hypr/scripts/control-center/src/widgets/media.rs)
* **Session Section (`[ LOCK ]`, `[ REBOOT ]`, `[ SHUTDOWN ]` Action Tiles):** [`/home/silas270/dotfiles/hypr/scripts/control-center/src/widgets/session.rs`](file:///home/silas270/dotfiles/hypr/scripts/control-center/src/widgets/session.rs)

### B. Waybar Top Status Bar
* **Module Layout & IPC Handlers:** [`/home/silas270/dotfiles/waybar/config`](file:///home/silas270/dotfiles/waybar/config)
* **CSS Stylesheet & Theme Imports:** [`/home/silas270/dotfiles/waybar/style.css`](file:///home/silas270/dotfiles/waybar/style.css)
* **Key Modules:**
  - Arch Logo Badge: `[ 󰣇 ]`
  - Workspaces: `[ 1 ] [ 2 ] [ 3 ]`
  - Centerpiece Media Player: `( ARTIST - TITLE )` (Toggles Control Center)
  - Volume Module: `[ 󰕾 50% ]`
  - Control Center Launcher: `[  ]`

### C. Rofi Launcher & Power Menu
* **Rofi Configuration & Theme:** [`/home/silas270/dotfiles/rofi/config.rasi`](file:///home/silas270/dotfiles/rofi/config.rasi)
* **Power Menu Shell Script:** [`/home/silas270/dotfiles/hypr/scripts/power-menu.sh`](file:///home/silas270/dotfiles/hypr/scripts/power-menu.sh)

### D. Hyprland Compositor
* **Lua Configuration:** [`/home/silas270/dotfiles/hypr/hyprland.lua`](file:///home/silas270/dotfiles/hypr/hyprland.lua)
* **Key Features:** Active window border `0xffd97736`, inactive border `0x3354382b`, 10px outer gaps, `SUPER + F` fullscreen toggle.
