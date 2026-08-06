use tiny_skia::{Color, PixmapMut};
use crate::render::FontCache;
use crate::api;
use super::fieldset::draw_fieldset_outline;

pub struct MediaSection;

fn format_time(secs: f64) -> String {
    let s = secs.max(0.0) as u64;
    let m = s / 60;
    let rem_s = s % 60;
    format!("{:02}:{:02}", m, rem_s)
}

impl MediaSection {
    pub const HEIGHT: f32 = 108.0;

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
        media_state: &api::media::MediaState,
    ) {
        let sec3_title = "MEDIA";
        let sec3_title_w = font_cache.measure_text(sec3_title, font_size, false);
        let sec3_gap_x = sec_x + 12.0;
        let sec3_gap_w = sec3_title_w + 8.0;

        draw_fieldset_outline(
            pixmap,
            sec_x, sec_y, sec_w, Self::HEIGHT,
            2.0, sec_border,
            sec3_gap_x, sec3_gap_w,
        );
        font_cache.draw_text(pixmap, sec3_title, sec3_gap_x + 4.0, sec_y - 10.0, font_size, false, accent_color);

        let media_x = sec_x + 12.0;
        let media_line1_y = sec_y + 14.0;
        let media_line2_y = sec_y + 36.0;
        let media_line3_y = sec_y + 58.0;
        let media_line4_y = sec_y + 80.0;

        // Line 1: TITLE
        let title_lbl = "TITLE  : ";
        font_cache.draw_text(pixmap, title_lbl, media_x, media_line1_y, font_size, false, accent_color);
        let title_lbl_w = font_cache.measure_text(title_lbl, font_size, false);
        let title_val = media_state.metadata.title.to_uppercase();
        font_cache.draw_text(pixmap, &title_val, media_x + title_lbl_w, media_line1_y, font_size, false, accent_color);

        // Line 2: ARTIST
        let artist_lbl = "ARTIST : ";
        font_cache.draw_text(pixmap, artist_lbl, media_x, media_line2_y, font_size, false, text_color);
        let artist_lbl_w = font_cache.measure_text(artist_lbl, font_size, false);
        let artist_val = media_state.metadata.artist.to_uppercase();
        font_cache.draw_text(pixmap, &artist_val, media_x + artist_lbl_w, media_line2_y, font_size, false, text_color);

        // Line 3: Seek Bar
        let elapsed_str = format_time(media_state.position_secs);
        let dur_str = format_time(media_state.metadata.length_secs);
        let left_lbl = format!("{} [", elapsed_str);
        let right_lbl = format!("] {}", dur_str);

        font_cache.draw_text(pixmap, &left_lbl, media_x, media_line3_y, font_size, false, text_color);
        let left_lbl_w = font_cache.measure_text(&left_lbl, font_size, false);
        let right_lbl_w = font_cache.measure_text(&right_lbl, font_size, false);

        let rail_start_x = media_x + left_lbl_w;
        let media_w = sec_w - 24.0;
        let right_lbl_x = media_x + media_w - right_lbl_w;
        font_cache.draw_text(pixmap, &right_lbl, right_lbl_x, media_line3_y, font_size, false, text_color);

        let available_rail_w = right_lbl_x - rail_start_x;
        let rail_char_w = font_cache.measure_text("─", font_size, false);
        let rail_len = (available_rail_w / rail_char_w).floor() as usize;

        let total_dur = media_state.metadata.length_secs;
        let pct = if total_dur > 0.0 { (media_state.position_secs / total_dur).clamp(0.0, 1.0) } else { 0.0 };
        let thumb_idx = if rail_len > 0 { (pct * (rail_len.saturating_sub(1) as f64)).round() as usize } else { 0 };
        let left_rail_len = thumb_idx;
        let right_rail_len = rail_len.saturating_sub(thumb_idx + 1);

        let rail_str = format!("{}|{}", "─".repeat(left_rail_len), "─".repeat(right_rail_len));
        font_cache.draw_text(pixmap, &rail_str, rail_start_x, media_line3_y, font_size, false, accent_color);

        // Line 4: Transport Controls
        let btn1_str = "[ 󰒮 PREV ]";
        let btn2_str = if media_state.status == api::media::PlaybackStatus::Playing { "[ 󰏤 PAUSE ]" } else { "[ 󰐊 PLAY ]" };
        let btn3_str = "[ 󰒵 NEXT ]";

        let btn2_w = font_cache.measure_text(btn2_str, font_size, false);
        let btn3_w = font_cache.measure_text(btn3_str, font_size, false);

        let btn1_x = media_x;
        let btn3_x = media_x + media_w - btn3_w;
        let btn2_x = media_x + (media_w - btn2_w) / 2.0;

        font_cache.draw_text(pixmap, btn1_str, btn1_x, media_line4_y, font_size, false, accent_color);
        font_cache.draw_text(pixmap, btn2_str, btn2_x, media_line4_y, font_size, false, accent_color);
        font_cache.draw_text(pixmap, btn3_str, btn3_x, media_line4_y, font_size, false, accent_color);
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
    ) -> bool {
        if y < sec_y || y > sec_y + Self::HEIGHT as f64 || x < sec_x || x > sec_x + sec_w {
            return false;
        }

        let media_x = sec_x + 12.0;
        let media_w = sec_w - 24.0;

        let btn1_str = "[ 󰒮 PREV ]";
        let btn2_str = if media_state.status == api::media::PlaybackStatus::Playing { "[ 󰏤 PAUSE ]" } else { "[ 󰐊 PLAY ]" };
        let btn3_str = "[ 󰒵 NEXT ]";

        let btn1_w = font_cache.measure_text(btn1_str, font_size, false) as f64;
        let btn2_w = font_cache.measure_text(btn2_str, font_size, false) as f64;
        let btn3_w = font_cache.measure_text(btn3_str, font_size, false) as f64;

        let btn1_x = media_x;
        let btn3_x = media_x + media_w - btn3_w;
        let btn2_x = media_x + (media_w - btn2_w) / 2.0;

        if y >= sec_y + 74.0 {
            if x >= btn1_x && x <= btn1_x + btn1_w {
                api::media::previous();
                media_state.position_secs = 0.0;
                true
            } else if x >= btn2_x && x <= btn2_x + btn2_w {
                api::media::play_pause();
                media_state.status = if media_state.status == api::media::PlaybackStatus::Playing {
                    api::media::PlaybackStatus::Paused
                } else {
                    api::media::PlaybackStatus::Playing
                };
                true
            } else if x >= btn3_x && x <= btn3_x + btn3_w {
                api::media::next();
                media_state.position_secs = 0.0;
                true
            } else {
                false
            }
        } else if y >= sec_y + 50.0 && y < sec_y + 74.0 {
            let elapsed_str = format_time(media_state.position_secs);
            let dur_str = format_time(media_state.metadata.length_secs);
            let left_lbl = format!("{} [", elapsed_str);
            let right_lbl = format!("] {}", dur_str);
            let left_lbl_w = font_cache.measure_text(&left_lbl, font_size, false) as f64;
            let right_lbl_w = font_cache.measure_text(&right_lbl, font_size, false) as f64;

            let rail_start_x = media_x + left_lbl_w;
            let right_lbl_x = media_x + media_w - right_lbl_w;
            let rail_w = right_lbl_x - rail_start_x;

            let pct = (x - rail_start_x) / rail_w;
            let val = pct.clamp(0.0, 1.0);
            let target_sec = val * media_state.metadata.length_secs;
            media_state.position_secs = target_sec;
            api::media::seek(target_sec);
            true
        } else {
            false
        }
    }
}
