use tiny_skia::{Color, PixmapMut};
use crate::render::FontCache;
use crate::api;
use super::fieldset::draw_fieldset_outline;
use super::ActionResult;

pub struct SessionSection;

impl SessionSection {
    pub const HEIGHT: f32 = 40.0;

    pub fn draw(
        pixmap: &mut PixmapMut,
        font_cache: &mut FontCache,
        sec_x: f32,
        sec_y: f32,
        sec_w: f32,
        font_size: f32,
        accent_color: Color,
        sec_border: Color,
    ) {
        let sec4_title = "SESSION";
        let sec4_title_w = font_cache.measure_text(sec4_title, font_size, false);
        let sec4_gap_x = sec_x + 12.0;
        let sec4_gap_w = sec4_title_w + 8.0;

        draw_fieldset_outline(
            pixmap,
            sec_x, sec_y, sec_w, Self::HEIGHT,
            2.0, sec_border,
            sec4_gap_x, sec4_gap_w,
        );
        font_cache.draw_text(pixmap, sec4_title, sec4_gap_x + 4.0, sec_y - 10.0, font_size, false, accent_color);

        let sess_x = sec_x + 12.0;
        let sess_w = sec_w - 24.0;
        let sess_line_y = sec_y + 12.0;

        let s_btn1 = "[ 󰌾 ]";
        let s_btn2 = "[ 󰤄 ]";
        let s_btn3 = "[ 󰜉 ]";
        let s_btn4 = "[ 󰐥 ]";

        let s_w1 = font_cache.measure_text(s_btn1, font_size, false);
        let s_w2 = font_cache.measure_text(s_btn2, font_size, false);
        let s_w3 = font_cache.measure_text(s_btn3, font_size, false);
        let s_w4 = font_cache.measure_text(s_btn4, font_size, false);

        let total_s_btn_w = s_w1 + s_w2 + s_w3 + s_w4;
        let s_gap = (sess_w - total_s_btn_w) / 5.0;

        let s_x1 = sess_x + s_gap;
        let s_x2 = s_x1 + s_w1 + s_gap;
        let s_x3 = s_x2 + s_w2 + s_gap;
        let s_x4 = s_x3 + s_w3 + s_gap;

        font_cache.draw_text(pixmap, s_btn1, s_x1, sess_line_y, font_size, false, accent_color);
        font_cache.draw_text(pixmap, s_btn2, s_x2, sess_line_y, font_size, false, accent_color);
        font_cache.draw_text(pixmap, s_btn3, s_x3, sess_line_y, font_size, false, accent_color);
        font_cache.draw_text(pixmap, s_btn4, s_x4, sess_line_y, font_size, false, accent_color);
    }

    pub fn handle_click(
        x: f64,
        y: f64,
        font_cache: &mut FontCache,
        sec_x: f64,
        sec_y: f64,
        sec_w: f64,
        font_size: f32,
    ) -> ActionResult {
        if y < sec_y || y > sec_y + Self::HEIGHT as f64 || x < sec_x || x > sec_x + sec_w {
            return ActionResult::None;
        }

        let sess_x = sec_x + 12.0;
        let sess_w = sec_w - 24.0;

        let s_btn1 = "[ 󰌾 ]";
        let s_btn2 = "[ 󰤄 ]";
        let s_btn3 = "[ 󰜉 ]";
        let s_btn4 = "[ 󰐥 ]";

        let s_w1 = font_cache.measure_text(s_btn1, font_size, false) as f64;
        let s_w2 = font_cache.measure_text(s_btn2, font_size, false) as f64;
        let s_w3 = font_cache.measure_text(s_btn3, font_size, false) as f64;
        let s_w4 = font_cache.measure_text(s_btn4, font_size, false) as f64;

        let total_s_btn_w = s_w1 + s_w2 + s_w3 + s_w4;
        let s_gap = (sess_w - total_s_btn_w) / 5.0;

        let s_x1 = sess_x + s_gap;
        let s_x2 = s_x1 + s_w1 + s_gap;
        let s_x3 = s_x2 + s_w2 + s_gap;
        let s_x4 = s_x3 + s_w3 + s_gap;

        if x >= s_x1 && x <= s_x1 + s_w1 {
            api::session::lock();
            ActionResult::HidePanel
        } else if x >= s_x2 && x <= s_x2 + s_w2 {
            api::session::suspend();
            ActionResult::HidePanel
        } else if x >= s_x3 && x <= s_x3 + s_w3 {
            api::session::reboot();
            ActionResult::HidePanel
        } else if x >= s_x4 && x <= s_x4 + s_w4 {
            api::session::poweroff();
            ActionResult::HidePanel
        } else {
            ActionResult::None
        }
    }
}
