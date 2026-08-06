use tiny_skia::{Color, PixmapMut};
use crate::render::FontCache;
use crate::api;
use super::fieldset::draw_fieldset_outline;

pub enum DragTarget {
    None,
    Brightness,
    Volume,
}

pub struct ControlsSection;

impl ControlsSection {
    pub const HEIGHT: f32 = 124.0;

    pub fn draw(
        pixmap: &mut PixmapMut,
        font_cache: &mut FontCache,
        sec_x: f32,
        sec_y: f32,
        sec_w: f32,
        font_size: f32,
        accent_color: Color,
        text_color: Color,
        sec_border: Color,
        brightness: f64,
        blue_light_active: bool,
        volume: f64,
        volume_muted: bool,
    ) {
        let sec2_title = "CONTROLS";
        let sec2_title_w = font_cache.measure_text(sec2_title, font_size, false);
        let sec2_gap_x = sec_x + 12.0;
        let sec2_gap_w = sec2_title_w + 8.0;

        draw_fieldset_outline(
            pixmap,
            sec_x, sec_y, sec_w, Self::HEIGHT,
            2.0, sec_border,
            sec2_gap_x, sec2_gap_w,
        );
        font_cache.draw_text(pixmap, sec2_title, sec2_gap_x + 4.0, sec_y - 10.0, font_size, false, accent_color);

        let icon_x = sec_x + 12.0;

        // Brightness (uses calibrated bracket tag offset)
        let bright_line1_y = sec_y + 14.0;
        let bright_line2_y = sec_y + 36.0;

        let b_icon = if blue_light_active { "󰈈" } else { "󰃠" };
        let tag_w1 = font_cache.draw_calibrated_bracket_tag(pixmap, b_icon, icon_x, bright_line1_y, font_size, accent_color);
        font_cache.draw_text(pixmap, " BRIGHTNESS", icon_x + tag_w1, bright_line1_y, font_size, false, accent_color);

        let char_w = font_cache.measure_text("#", font_size, false);
        let bracket_w = font_cache.measure_text("[", font_size, false);
        let available_inner_w = (sec_w - 24.0) - bracket_w * 2.0;
        let slider_len = (available_inner_w / char_w).floor() as usize;

        let b_filled = (brightness.clamp(0.0, 1.0) * slider_len as f64).round() as usize;
        let b_empty = slider_len.saturating_sub(b_filled);
        let bright_slider_str = format!("[{}{}]", "#".repeat(b_filled), "-".repeat(b_empty));
        font_cache.draw_text(pixmap, &bright_slider_str, icon_x, bright_line2_y, font_size, false, text_color);

        // Volume (uses standard space padding)
        let vol_line1_y = sec_y + 74.0;
        let vol_line2_y = sec_y + 96.0;

        let v_icon = if volume_muted || volume == 0.0 { "󰖁" } else { "󰕾" };
        let vol_header = format!("[ {} ] VOLUME", v_icon);
        font_cache.draw_text(pixmap, &vol_header, icon_x, vol_line1_y, font_size, false, accent_color);

        let eff_vol = if volume_muted { 0.0 } else { volume };
        let vol_filled = (eff_vol.clamp(0.0, 1.0) * slider_len as f64).round() as usize;
        let vol_empty = slider_len.saturating_sub(vol_filled);
        let vol_slider_str = format!("[{}{}]", "#".repeat(vol_filled), "-".repeat(vol_empty));
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

        let icon_x = sec_x + 12.0;

        if y >= sec_y + 10.0 && y < sec_y + 36.0 {
            let b_icon = if *blue_light_active { "󰈈" } else { "󰃠" };
            let tag_w = font_cache.measure_calibrated_bracket_tag(b_icon, font_size) as f64;
            if x >= icon_x && x <= icon_x + tag_w {
                let new_state = api::compositor::toggle_blue_light();
                *blue_light_active = new_state;
                DragTarget::None
            } else {
                DragTarget::None
            }
        } else if y >= sec_y + 36.0 && y < sec_y + 64.0 {
            DragTarget::Brightness
        } else if y >= sec_y + 70.0 && y < sec_y + 96.0 {
            let v_icon = if *volume_muted { "󰖁" } else { "󰕾" };
            let vol_tag = format!("[ {} ]", v_icon);
            let tag_w = font_cache.measure_text(&vol_tag, font_size, false) as f64;
            if x >= icon_x && x <= icon_x + tag_w {
                let new_state = api::audio::toggle_mute();
                *volume_muted = new_state;
                DragTarget::None
            } else {
                DragTarget::None
            }
        } else if y >= sec_y + 96.0 && y < sec_y + 130.0 {
            DragTarget::Volume
        } else {
            DragTarget::None
        }
    }
}
