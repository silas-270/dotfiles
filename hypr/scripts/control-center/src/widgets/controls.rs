use tiny_skia::PixmapMut;
use crate::render::FontCache;
use crate::theme::ThemeConfig;
use crate::api;
use super::fieldset::{draw_section_header, draw_inner_box};

pub enum DragTarget {
    None,
    Brightness,
    Volume,
}

pub struct ControlsSection;

impl ControlsSection {
    pub const HEIGHT: f32 = 195.0;

    pub fn draw(
        pixmap: &mut PixmapMut,
        font_cache: &mut FontCache,
        sec_x: f32,
        sec_y: f32,
        sec_w: f32,
        font_size: f32,
        theme: &ThemeConfig,
        brightness: f64,
        blue_light_active: bool,
        volume: f64,
        volume_muted: bool,
    ) {
        let accent_color = theme.accent_color;
        let text_color = theme.text_color;
        let sec_border = theme.sec_border;
        let inner_border = theme.inner_border;

        draw_section_header(
            pixmap, font_cache, "CONTROLS",
            sec_x, sec_y, sec_w, font_size,
            accent_color, sec_border,
        );

        // --- Brightness Box ---
        let bright_box_x = sec_x;
        let bright_box_y = sec_y + 49.0;
        let bright_box_w = sec_w;
        let bright_box_h = 66.0;

        draw_inner_box(pixmap, bright_box_x, bright_box_y, bright_box_w, bright_box_h, None, inner_border, 1.5);

        let icon_x = bright_box_x + 8.0;
        let bright_line1_y = bright_box_y + 8.0;
        let bright_line2_y = bright_box_y + 37.0;

        let b_icon = if blue_light_active { "󰈈" } else { "󰃠" };
        let tag_w1 = font_cache.draw_calibrated_bracket_tag(pixmap, b_icon, icon_x, bright_line1_y, font_size, accent_color);
        font_cache.draw_text(pixmap, " BRIGHTNESS", icon_x + tag_w1, bright_line1_y, font_size, false, accent_color);

        let char_w = font_cache.measure_text("=", font_size, false);
        let bracket_w = font_cache.measure_text("(", font_size, false);
        let available_slider_w = bright_box_w - 16.0;
        let available_inner_w = available_slider_w - bracket_w * 2.0;
        let slider_len = (available_inner_w / char_w).floor() as usize;

        let inner_len = slider_len.saturating_sub(1);
        let b_filled = (brightness.clamp(0.0, 1.0) * inner_len as f64).round() as usize;
        let b_empty = inner_len.saturating_sub(b_filled);
        let bright_slider_str = format!("({}●{})", "=".repeat(b_filled), "-".repeat(b_empty));
        font_cache.draw_text(pixmap, &bright_slider_str, icon_x, bright_line2_y, font_size, false, text_color);

        // --- Volume Box ---
        let vol_box_x = sec_x;
        let vol_box_y = sec_y + 129.0;
        let vol_box_w = sec_w;
        let vol_box_h = 66.0;

        draw_inner_box(pixmap, vol_box_x, vol_box_y, vol_box_w, vol_box_h, None, inner_border, 1.5);

        let vol_line1_y = vol_box_y + 8.0;
        let vol_line2_y = vol_box_y + 37.0;

        let v_icon = if volume_muted || volume == 0.0 { "󰖁" } else { "󰕾" };
        let vol_header = format!("( {} ) VOLUME", v_icon);
        font_cache.draw_text(pixmap, &vol_header, icon_x, vol_line1_y, font_size, false, accent_color);

        let eff_vol = if volume_muted { 0.0 } else { volume };
        let vol_filled = (eff_vol.clamp(0.0, 1.0) * inner_len as f64).round() as usize;
        let vol_empty = inner_len.saturating_sub(vol_filled);
        let vol_slider_str = format!("({}●{})", "=".repeat(vol_filled), "-".repeat(vol_empty));
        font_cache.draw_text(pixmap, &vol_slider_str, icon_x, vol_line2_y, font_size, false, text_color);
    }

    pub fn handle_click(
        x: f64,
        y: f64,
        font_cache: &mut FontCache,
        sec_x: f64,
        sec_y: f64,
        sec_w: f64,
        font_size: f32,
        _brightness: &mut f64,
        blue_light_active: &mut bool,
        _volume: &mut f64,
        volume_muted: &mut bool,
    ) -> DragTarget {
        if y < sec_y || y > sec_y + Self::HEIGHT as f64 || x < sec_x || x > sec_x + sec_w {
            return DragTarget::None;
        }

        let bright_box_y = sec_y + 49.0;
        let bright_box_h = 66.0;
        let vol_box_y = sec_y + 129.0;
        let vol_box_h = 66.0;

        let icon_x = sec_x + 8.0;

        if y >= bright_box_y && y <= bright_box_y + bright_box_h {
            let b_icon = if *blue_light_active { "󰈈" } else { "󰃠" };
            let tag_w = font_cache.measure_calibrated_bracket_tag(b_icon, font_size) as f64;
            if y <= bright_box_y + 35.0 && x >= icon_x && x <= icon_x + tag_w {
                let new_state = api::compositor::toggle_blue_light();
                *blue_light_active = new_state;
                DragTarget::None
            } else {
                DragTarget::Brightness
            }
        } else if y >= vol_box_y && y <= vol_box_y + vol_box_h {
            let v_icon = if *volume_muted { "󰖁" } else { "󰕾" };
            let vol_tag = format!("( {} )", v_icon);
            let tag_w = font_cache.measure_text(&vol_tag, font_size, false) as f64;
            if y <= vol_box_y + 35.0 && x >= icon_x && x <= icon_x + tag_w {
                let new_state = api::audio::toggle_mute();
                *volume_muted = new_state;
                DragTarget::None
            } else {
                DragTarget::Volume
            }
        } else {
            DragTarget::None
        }
    }
}
