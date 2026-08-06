use tiny_skia::{Color, PixmapMut};
use crate::render::FontCache;
use crate::api;
use super::fieldset::draw_fieldset_outline;
use super::ActionResult;

pub struct ConnectionsSection;

impl ConnectionsSection {
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
        wifi_enabled: bool,
        wifi_ssid: &str,
        bt_enabled: bool,
        bt_device: &str,
    ) {
        let sec1_title = "CONNECTIONS";
        let sec1_title_w = font_cache.measure_text(sec1_title, font_size, false);
        let sec1_gap_x = sec_x + 12.0;
        let sec1_gap_w = sec1_title_w + 8.0;

        draw_fieldset_outline(
            pixmap,
            sec_x, sec_y, sec_w, Self::HEIGHT,
            2.0, sec_border,
            sec1_gap_x, sec1_gap_w,
        );
        font_cache.draw_text(pixmap, sec1_title, sec1_gap_x + 4.0, sec_y - 10.0, font_size, false, accent_color);

        let icon_x = sec_x + 12.0;
        let wifi_line1_y = sec_y + 14.0;
        let wifi_line2_y = sec_y + 36.0;

        // WiFi uses calibrated bracket tag (centered icon, exact standard bracket width)
        let wifi_icon = if wifi_enabled { "󰖩" } else { "󰖪" };
        let tag_w = font_cache.draw_calibrated_bracket_tag(pixmap, wifi_icon, icon_x, wifi_line1_y, font_size, accent_color);
        font_cache.draw_text(pixmap, " WIFI", icon_x + tag_w, wifi_line1_y, font_size, false, accent_color);

        let wifi_sub = if !wifi_enabled {
            "Disabled".to_string()
        } else if wifi_ssid.is_empty() {
            "Disconnected".to_string()
        } else {
            wifi_ssid.to_string()
        };
        font_cache.draw_text(pixmap, &wifi_sub, icon_x, wifi_line2_y, font_size, false, text_color);

        let bt_line1_y = sec_y + 74.0;
        let bt_line2_y = sec_y + 96.0;

        // Bluetooth uses standard space padding
        let bt_icon = if bt_enabled { "" } else { "󰂲" };
        let bt_header = format!("[ {} ] BLUETOOTH", bt_icon);
        font_cache.draw_text(pixmap, &bt_header, icon_x, bt_line1_y, font_size, false, accent_color);

        let bt_sub = if !bt_enabled {
            "Disabled".to_string()
        } else if bt_device.is_empty() {
            "Disconnected".to_string()
        } else {
            bt_device.to_string()
        };
        font_cache.draw_text(pixmap, &bt_sub, icon_x, bt_line2_y, font_size, false, text_color);
    }

    pub fn handle_click(
        x: f64,
        y: f64,
        font_cache: &mut FontCache,
        sec_x: f64,
        sec_y: f64,
        sec_w: f64,
        font_size: f32,
        wifi_enabled: &mut bool,
        bt_enabled: &mut bool,
    ) -> ActionResult {
        if y < sec_y || y > sec_y + Self::HEIGHT as f64 || x < sec_x || x > sec_x + sec_w {
            return ActionResult::None;
        }

        let icon_x = sec_x + 12.0;

        if y >= sec_y + 10.0 && y < sec_y + 64.0 {
            let wifi_icon = if *wifi_enabled { "󰖩" } else { "󰖪" };
            let tag_w = font_cache.measure_calibrated_bracket_tag(wifi_icon, font_size) as f64;
            let icon_hitbox_end = icon_x + tag_w;

            if x >= icon_x && x <= icon_hitbox_end {
                let new_state = !*wifi_enabled;
                api::network::set_wifi_enabled(new_state);
                *wifi_enabled = new_state;
                ActionResult::NeedsDraw
            } else {
                api::network::open_network_menu();
                ActionResult::HidePanel
            }
        } else if y >= sec_y + 64.0 && y < sec_y + 130.0 {
            let bt_icon = if *bt_enabled { "" } else { "󰂲" };
            let bt_tag = format!("[ {} ]", bt_icon);
            let tag_w = font_cache.measure_text(&bt_tag, font_size, false) as f64;
            let icon_hitbox_end = icon_x + tag_w;

            if x >= icon_x && x <= icon_hitbox_end {
                let new_state = !*bt_enabled;
                api::bluetooth::set_bluetooth_enabled(new_state);
                *bt_enabled = new_state;
                ActionResult::NeedsDraw
            } else {
                api::bluetooth::open_bluetooth_menu();
                ActionResult::HidePanel
            }
        } else {
            ActionResult::None
        }
    }
}
