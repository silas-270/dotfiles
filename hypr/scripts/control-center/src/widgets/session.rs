use tiny_skia::PixmapMut;
use shell_common::font::FontCache;
use shell_common::theme::ThemeConfig;
use crate::api;
use super::fieldset::{draw_section_header, draw_inner_box};
use super::ActionResult;

pub struct SessionSection;

impl SessionSection {
    pub const HEIGHT: f32 = 81.0;

    pub fn draw(
        pixmap: &mut PixmapMut,
        font_cache: &mut FontCache,
        sec_x: f32,
        sec_y: f32,
        sec_w: f32,
        font_size: f32,
        theme: &ThemeConfig,
    ) {
        let accent_color = theme.accent_color;
        let sec_border = theme.sec_border;
        let inner_border = theme.inner_border;

        draw_section_header(
            pixmap, font_cache, "SESSION",
            sec_x, sec_y, sec_w, font_size,
            accent_color, sec_border,
        );

        let sess_x = sec_x;
        let sess_w = sec_w;
        let box_y = sec_y + 49.0;
        let box_h = 32.0;

        let icons = ["( 󰌾 )", "( 󰤄 )", "( 󰜉 )", "( 󰐥 )"];
        let num_btns = 4.0;
        let gap = 6.0;
        let btn_w = (sess_w - (num_btns - 1.0) * gap) / num_btns;

        for (i, icon) in icons.iter().enumerate() {
            let bx = sess_x + i as f32 * (btn_w + gap);
            draw_inner_box(pixmap, bx, box_y, btn_w, box_h, None, inner_border, 1.5);

            let icon_w = font_cache.measure_text(icon, font_size);
            let text_x = bx + (btn_w - icon_w) / 2.0;
            let text_y = box_y + 5.0;
            font_cache.draw_text(pixmap, icon, text_x, text_y, font_size, accent_color);
        }
    }

    pub fn handle_click(
        x: f64,
        y: f64,
        _font_cache: &mut FontCache,
        sec_x: f64,
        sec_y: f64,
        sec_w: f64,
        _font_size: f32,
    ) -> ActionResult {
        if y < sec_y || y > sec_y + Self::HEIGHT as f64 || x < sec_x || x > sec_x + sec_w {
            return ActionResult::None;
        }

        let sess_x = sec_x;
        let sess_w = sec_w;
        let box_y = sec_y + 49.0;
        let box_h = 32.0;

        if y < box_y || y > box_y + box_h {
            return ActionResult::None;
        }

        let num_btns = 4.0;
        let gap = 6.0;
        let btn_w = (sess_w - (num_btns - 1.0) * gap) / num_btns;

        for i in 0..4 {
            let bx = sess_x + i as f64 * (btn_w + gap);
            if x >= bx && x <= bx + btn_w {
                match i {
                    0 => api::session::lock(),
                    1 => api::session::suspend(),
                    2 => api::session::reboot(),
                    3 => api::session::poweroff(),
                    _ => {}
                }
                return ActionResult::HidePanel;
            }
        }

        ActionResult::None
    }
}
