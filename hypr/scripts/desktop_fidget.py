#!/usr/bin/env python3
import gi
import os
import re

gi.require_version('Gtk', '3.0')
gi.require_version('GtkLayerShell', '0.1')
from gi.repository import Gtk, GtkLayerShell, Gdk
import cairo

def load_theme_colors():
    """Reads colors from ~/dotfiles/theme/generated/colors.lua matching Waybar and Hyprland setup"""
    # Fallback default colors
    fill_rgb = (217 / 255.0, 119 / 255.0, 54 / 255.0)       # #D97736 (Orange Accent)
    border_rgb = (84 / 255.0, 56 / 255.0, 43 / 255.0)       # #54382B (Waybar Background bg_base)
    badge_bg_rgb = (84 / 255.0, 56 / 255.0, 43 / 255.0)     # #54382B (bg_base)
    text_muted_rgb = (194 / 255.0, 170 / 255.0, 149 / 255.0) # #C2AA95 (fg_muted)

    colors_lua = os.path.expanduser("~/dotfiles/theme/generated/colors.lua")
    if os.path.exists(colors_lua):
        try:
            with open(colors_lua, "r") as f:
                content = f.read()

                # accent (Orange fill)
                m_acc = re.search(r'accent\s*=\s*"#([0-9a-fA-F]{6})"', content)
                if m_acc:
                    h = m_acc.group(1)
                    fill_rgb = (int(h[0:2], 16)/255.0, int(h[2:4], 16)/255.0, int(h[4:6], 16)/255.0)

                # bg_base (Waybar background -> used for border and badge background)
                m_bg = re.search(r'bg_base\s*=\s*"#([0-9a-fA-F]{6})"', content)
                if m_bg:
                    h = m_bg.group(1)
                    badge_bg_rgb = (int(h[0:2], 16)/255.0, int(h[2:4], 16)/255.0, int(h[4:6], 16)/255.0)
                    border_rgb = badge_bg_rgb

                # fg_muted (Subtle badge text)
                m_fg = re.search(r'fg_muted\s*=\s*"#([0-9a-fA-F]{6})"', content)
                if m_fg:
                    h = m_fg.group(1)
                    text_muted_rgb = (int(h[0:2], 16)/255.0, int(h[2:4], 16)/255.0, int(h[4:6], 16)/255.0)
        except Exception:
            pass

    return fill_rgb, border_rgb, badge_bg_rgb, text_muted_rgb


class DesktopFidget(Gtk.Window):
    def __init__(self):
        super().__init__()

        # Load Theme Colors
        colors = load_theme_colors()
        self.fill_r, self.fill_g, self.fill_b = colors[0]
        self.border_r, self.border_g, self.border_b = colors[1]
        self.badge_bg_r, self.badge_bg_g, self.badge_bg_b = colors[2]
        self.text_r, self.text_g, self.text_b = colors[3]

        # Register Layer Shell Surface for Hyprland
        GtkLayerShell.init_for_window(self)
        GtkLayerShell.set_layer(self, GtkLayerShell.Layer.BACKGROUND)
        GtkLayerShell.set_anchor(self, GtkLayerShell.Edge.TOP, True)
        GtkLayerShell.set_anchor(self, GtkLayerShell.Edge.BOTTOM, True)
        GtkLayerShell.set_anchor(self, GtkLayerShell.Edge.LEFT, True)
        GtkLayerShell.set_anchor(self, GtkLayerShell.Edge.RIGHT, True)
        GtkLayerShell.set_exclusive_zone(self, -1)
        GtkLayerShell.set_keyboard_mode(self, GtkLayerShell.KeyboardMode.NONE)

        # Make Background Transparent
        screen = self.get_screen()
        visual = screen.get_rgba_visual()
        if visual:
            self.set_visual(visual)
        self.set_app_paintable(True)

        self.start_x = None
        self.start_y = None
        self.current_x = None
        self.current_y = None
        self.is_dragging = False

        self.add_events(
            Gdk.EventMask.BUTTON_PRESS_MASK |
            Gdk.EventMask.BUTTON_RELEASE_MASK |
            Gdk.EventMask.POINTER_MOTION_MASK
        )

        self.connect("draw", self.on_draw)
        self.connect("button-press-event", self.on_button_press)
        self.connect("motion-notify-event", self.on_motion_notify)
        self.connect("button-release-event", self.on_button_release)

    def on_button_press(self, widget, event):
        if event.button == 1:  # Left mouse button
            self.start_x = event.x
            self.start_y = event.y
            self.current_x = event.x
            self.current_y = event.y
            self.is_dragging = True
            self.queue_draw()

    def on_motion_notify(self, widget, event):
        if self.is_dragging:
            self.current_x = event.x
            self.current_y = event.y
            self.queue_draw()

    def on_button_release(self, widget, event):
        if event.button == 1 and self.is_dragging:
            self.is_dragging = False
            self.start_x = None
            self.start_y = None
            self.queue_draw()

    def on_draw(self, widget, cr):
        # Clear background (transparent)
        cr.set_source_rgba(0, 0, 0, 0)
        cr.set_operator(cairo.OPERATOR_CLEAR)
        cr.paint()
        cr.set_operator(cairo.OPERATOR_OVER)

        # Draw Selection Rectangle
        if self.is_dragging and self.start_x is not None:
            x = min(self.start_x, self.current_x)
            y = min(self.start_y, self.current_y)
            w = abs(self.current_x - self.start_x)
            h = abs(self.current_y - self.start_y)

            # Orange Fill (25% Opacity)
            cr.set_source_rgba(self.fill_r, self.fill_g, self.fill_b, 0.25)
            cr.rectangle(x, y, w, h)
            cr.fill_preserve()

            # Waybar Background Brown Border (#54382B)
            cr.set_source_rgba(self.border_r, self.border_g, self.border_b, 0.95)
            cr.set_line_width(2.0)
            cr.stroke()

            # Draw Pixel-Dimension Badge (Subtle style)
            if w > 10 and h > 10:
                badge_text = f"{int(w)} × {int(h)} px"
                padding_x = 7
                padding_y = 3

                cr.select_font_face("JetBrainsMono Nerd Font", cairo.FONT_SLANT_NORMAL, cairo.FONT_WEIGHT_NORMAL)
                cr.set_font_size(10.5)
                extents = cr.text_extents(badge_text)

                bw = extents.width + padding_x * 2
                bh = extents.height + padding_y * 2

                win_w = self.get_allocated_width()
                win_h = self.get_allocated_height()

                # Position near bottom-right of selection box, clamped to screen bounds
                bx = min(x + w + 6, win_w - bw - 6)
                by = min(y + h + 6, win_h - bh - 6)
                if bx < 6:
                    bx = 6
                if by < 6:
                    by = 6

                # Badge background box (Waybar Background bg_base with subtle opacity)
                cr.set_source_rgba(self.badge_bg_r, self.badge_bg_g, self.badge_bg_b, 0.85)
                cr.rectangle(bx, by, bw, bh)
                cr.fill_preserve()

                # Subtle border around badge
                cr.set_source_rgba(self.fill_r, self.fill_g, self.fill_b, 0.35)
                cr.set_line_width(1.0)
                cr.stroke()

                # Subtle muted font color (fg_muted #C2AA95, 80% opacity)
                cr.move_to(bx + padding_x - extents.x_bearing, by + padding_y - extents.y_bearing)
                cr.set_source_rgba(self.text_r, self.text_g, self.text_b, 0.8)
                cr.show_text(badge_text)

if __name__ == "__main__":
    try:
        win = DesktopFidget()
        win.connect("destroy", Gtk.main_quit)
        win.show_all()
        Gtk.main()
    except KeyboardInterrupt:
        pass
