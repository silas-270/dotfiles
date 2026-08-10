use tiny_skia::PixmapMut;
use crate::render::FontCache;
use crate::theme::ThemeConfig;
use crate::api;
use super::fieldset::{draw_section_header, draw_inner_box};

pub struct MediaSection;

fn format_time(secs: f64) -> String {
    let s = secs.max(0.0) as u64;
    let m = s / 60;
    let rem_s = s % 60;
    format!("{:02}:{:02}", m, rem_s)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaClickResult {
    None,
    Button,
    Seek,
}

impl MediaSection {
    pub const HEIGHT: f32 = 144.0;

    pub fn draw(
        pixmap: &mut PixmapMut,
        font_cache: &mut FontCache,
        sec_x: f32,
        sec_y: f32,
        sec_w: f32,
        font_size: f32,
        theme: &ThemeConfig,
        media_state: &api::media::MediaState,
    ) {
        let accent_color = theme.accent_color;
        let text_color = theme.text_color;
        let sec_border = theme.sec_border;
        let inner_border = theme.inner_border;

        draw_section_header(
            pixmap, font_cache, "MEDIA",
            sec_x, sec_y, sec_w, font_size,
            accent_color, sec_border,
        );

        let box_x = sec_x;
        let box_y = sec_y + 49.0;
        let box_w = sec_w;
        let box_h = 95.0;

        draw_inner_box(pixmap, box_x, box_y, box_w, box_h, None, inner_border, 1.5);

        let icon_x = box_x + 8.0;
        let content_w = box_w - 16.0;
        let media_line1_y = box_y + 8.0;
        let media_line2_y = box_y + 37.0;
        let media_line3_y = box_y + 66.0;

        // Line 1: ARTIST - TITLE
        let display_str = if media_state.metadata.artist.is_empty() {
            media_state.metadata.title.to_uppercase()
        } else if media_state.metadata.title.is_empty() {
            media_state.metadata.artist.to_uppercase()
        } else {
            format!("{} - {}", media_state.metadata.artist.to_uppercase(), media_state.metadata.title.to_uppercase())
        };
        font_cache.draw_text(pixmap, &display_str, icon_x, media_line1_y, font_size, false, accent_color);

        // Line 2: Seek Bar
        let elapsed_str = format_time(media_state.position_secs);
        let dur_str = format_time(media_state.metadata.length_secs);
        let left_lbl = format!("{} (", elapsed_str);
        let right_lbl = format!(") {}", dur_str);

        font_cache.draw_text(pixmap, &left_lbl, icon_x, media_line2_y, font_size, false, text_color);
        let left_lbl_w = font_cache.measure_text(&left_lbl, font_size, false);
        let right_lbl_w = font_cache.measure_text(&right_lbl, font_size, false);

        let rail_start_x = icon_x + left_lbl_w;
        let right_lbl_x = icon_x + content_w - right_lbl_w;
        font_cache.draw_text(pixmap, &right_lbl, right_lbl_x, media_line2_y, font_size, false, text_color);

        let available_rail_w = right_lbl_x - rail_start_x;
        let rail_char_w = font_cache.measure_text("─", font_size, false);
        let rail_len = (available_rail_w / rail_char_w).floor() as usize;

        let total_dur = media_state.metadata.length_secs;
        let pct = if total_dur > 0.0 { (media_state.position_secs / total_dur).clamp(0.0, 1.0) } else { 0.0 };
        let thumb_idx = if rail_len > 0 { (pct * (rail_len.saturating_sub(1) as f64)).round() as usize } else { 0 };
        let left_rail_len = thumb_idx;
        let right_rail_len = rail_len.saturating_sub(thumb_idx + 1);

        let rail_str = format!("{}|{}", "─".repeat(left_rail_len), "─".repeat(right_rail_len));
        font_cache.draw_text(pixmap, &rail_str, rail_start_x, media_line2_y, font_size, false, accent_color);

        // Line 3: Transport Controls
        let btn1_str = "( 󰒮 PREV )";
        let btn2_str = if media_state.status == api::media::PlaybackStatus::Playing { "( 󰏤 PAUSE )" } else { "( 󰐊 PLAY )" };
        let btn3_str = "( 󰒭 NEXT )";

        let btn2_w = font_cache.measure_text(btn2_str, font_size, false);
        let btn3_w = font_cache.measure_text(btn3_str, font_size, false);

        let btn1_x = icon_x;
        let btn3_x = icon_x + content_w - btn3_w;
        let btn2_x = icon_x + (content_w - btn2_w) / 2.0;

        font_cache.draw_text(pixmap, btn1_str, btn1_x, media_line3_y, font_size, false, text_color);
        font_cache.draw_text(pixmap, btn2_str, btn2_x, media_line3_y, font_size, false, text_color);
        font_cache.draw_text(pixmap, btn3_str, btn3_x, media_line3_y, font_size, false, text_color);
    }

    pub fn handle_click(
        x: f64,
        y: f64,
        font_cache: &mut FontCache,
        sec_x: f64,
        sec_y: f64,
        sec_w: f64,
        font_size: f32,
        media_state: &mut api::media::MediaState,
    ) -> MediaClickResult {
        let box_x = sec_x;
        let box_y = sec_y + 49.0;
        let box_w = sec_w;
        let box_h = 95.0;

        if y < box_y || y > box_y + box_h || x < box_x || x > box_x + box_w {
            return MediaClickResult::None;
        }

        let icon_x = box_x + 8.0;
        let content_w = box_w - 16.0;
        let line2_y = box_y + 37.0;
        let line3_y = box_y + 66.0;

        let btn1_str = "( 󰒮 PREV )";
        let btn2_str = if media_state.status == api::media::PlaybackStatus::Playing { "( 󰏤 PAUSE )" } else { "( 󰐊 PLAY )" };
        let btn3_str = "( 󰒭 NEXT )";

        let btn1_w = font_cache.measure_text(btn1_str, font_size, false) as f64;
        let btn2_w = font_cache.measure_text(btn2_str, font_size, false) as f64;
        let btn3_w = font_cache.measure_text(btn3_str, font_size, false) as f64;

        let btn1_x = icon_x;
        let btn3_x = icon_x + content_w - btn3_w;
        let btn2_x = icon_x + (content_w - btn2_w) / 2.0;

        if y >= line3_y - 4.0 {
            if x >= btn1_x && x <= btn1_x + btn1_w {
                api::media::previous();
                media_state.position_secs = 0.0;
                MediaClickResult::Button
            } else if x >= btn2_x && x <= btn2_x + btn2_w {
                api::media::play_pause();
                media_state.status = if media_state.status == api::media::PlaybackStatus::Playing {
                    api::media::PlaybackStatus::Paused
                } else {
                    api::media::PlaybackStatus::Playing
                };
                MediaClickResult::Button
            } else if x >= btn3_x && x <= btn3_x + btn3_w {
                api::media::next();
                media_state.position_secs = 0.0;
                MediaClickResult::Button
            } else {
                MediaClickResult::None
            }
        } else if y >= line2_y - 4.0 && y < line3_y - 4.0 {
            let elapsed_str = format_time(media_state.position_secs);
            let dur_str = format_time(media_state.metadata.length_secs);
            let left_lbl = format!("{} (", elapsed_str);
            let right_lbl = format!(") {}", dur_str);
            let left_lbl_w = font_cache.measure_text(&left_lbl, font_size, false) as f64;
            let right_lbl_w = font_cache.measure_text(&right_lbl, font_size, false) as f64;

            let rail_start_x = icon_x + left_lbl_w;
            let right_lbl_x = icon_x + content_w - right_lbl_w;
            let rail_w = right_lbl_x - rail_start_x;

            if rail_w > 0.0 {
                let pct = ((x - rail_start_x) / rail_w).clamp(0.0, 1.0);
                let target_sec = pct * media_state.metadata.length_secs;
                media_state.position_secs = target_sec;
                api::media::seek(target_sec);
                MediaClickResult::Seek
            } else {
                MediaClickResult::None
            }
        } else {
            MediaClickResult::None
        }
    }
}
