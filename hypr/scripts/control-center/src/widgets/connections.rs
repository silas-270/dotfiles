use tiny_skia::PixmapMut;
use crate::render::FontCache;
use crate::theme::ThemeConfig;
use crate::api;
use super::fieldset::{draw_section_header, draw_inner_box, draw_tree_corner};
use super::ActionResult;

pub struct ConnectionsSection;

impl ConnectionsSection {
    pub const HEIGHT: f32 = 195.0;

    pub fn draw(
        pixmap: &mut PixmapMut,
        font_cache: &mut FontCache,
        sec_x: f32,
        sec_y: f32,
        sec_w: f32,
        font_size: f32,
        theme: &ThemeConfig,
        wifi_enabled: bool,
        wifi_ssid: &str,
        bt_enabled: bool,
        bt_device: &str,
    ) {
        let accent_color = theme.accent_color;
        let text_color = theme.text_color;
        let sec_border = theme.sec_border;
        let inner_border = theme.inner_border;

        draw_section_header(
            pixmap, font_cache, "CONNECTIONS",
            sec_x, sec_y, sec_w, font_size,
            accent_color, sec_border,
        );

        // --- WiFi Box ---
        let wifi_box_x = sec_x;
        let wifi_box_y = sec_y + 49.0;
        let wifi_box_w = sec_w;
        let wifi_box_h = 66.0;

        draw_inner_box(pixmap, wifi_box_x, wifi_box_y, wifi_box_w, wifi_box_h, None, inner_border, 1.5);

        let icon_x = wifi_box_x + 8.0;
        let wifi_line1_y = wifi_box_y + 8.0;
        let wifi_line2_y = wifi_box_y + 37.0;

        // WiFi uses calibrated bracket tag (centered icon, exact standard bracket width)
        let wifi_icon = if wifi_enabled { "󰖩" } else { "󰖪" };
        let tag_w = font_cache.draw_calibrated_bracket_tag(pixmap, wifi_icon, icon_x, wifi_line1_y, font_size, accent_color);
        font_cache.draw_text(pixmap, " WIFI", icon_x + tag_w, wifi_line1_y, font_size, false, accent_color);

        let wifi_sub_text = if !wifi_enabled {
            "Disabled"
        } else if wifi_ssid.is_empty() {
            "Disconnected"
        } else {
            wifi_ssid
        };
        draw_tree_corner(pixmap, icon_x, wifi_line2_y, font_size, text_color);
        font_cache.draw_text(pixmap, wifi_sub_text, icon_x + 14.0, wifi_line2_y, font_size, false, text_color);

        // --- Bluetooth Box ---
        let bt_box_x = sec_x;
        let bt_box_y = sec_y + 129.0;
        let bt_box_w = sec_w;
        let bt_box_h = 66.0;

        draw_inner_box(pixmap, bt_box_x, bt_box_y, bt_box_w, bt_box_h, None, inner_border, 1.5);

        let bt_line1_y = bt_box_y + 8.0;
        let bt_line2_y = bt_box_y + 37.0;

        // Bluetooth uses standard space padding
        let bt_icon = if bt_enabled { "" } else { "󰂲" };
        let bt_header = format!("( {} ) BLUETOOTH", bt_icon);
        font_cache.draw_text(pixmap, &bt_header, icon_x, bt_line1_y, font_size, false, accent_color);

        let bt_sub_text = if !bt_enabled {
            "Disabled"
        } else if bt_device.is_empty() {
            "Disconnected"
        } else {
            bt_device
        };
        draw_tree_corner(pixmap, icon_x, bt_line2_y, font_size, text_color);
        font_cache.draw_text(pixmap, bt_sub_text, icon_x + 14.0, bt_line2_y, font_size, false, text_color);
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

        let wifi_box_y = sec_y + 49.0;
        let wifi_box_h = 66.0;
        let bt_box_y = sec_y + 129.0;
        let bt_box_h = 66.0;

        let icon_x = sec_x + 8.0;

        if y >= wifi_box_y && y <= wifi_box_y + wifi_box_h {
            let wifi_icon = if *wifi_enabled { "󰖩" } else { "󰖪" };
            let tag_w = font_cache.measure_calibrated_bracket_tag(wifi_icon, font_size) as f64;
            let icon_hitbox_end = icon_x + tag_w;

            if y <= wifi_box_y + 35.0 && x >= icon_x && x <= icon_hitbox_end {
                let new_state = !*wifi_enabled;
                api::network::set_wifi_enabled(new_state);
                *wifi_enabled = new_state;
                ActionResult::NeedsDraw
            } else {
                api::network::open_network_menu();
                ActionResult::HidePanel
            }
        } else if y >= bt_box_y && y <= bt_box_y + bt_box_h {
            let bt_icon = if *bt_enabled { "" } else { "󰂲" };
            let bt_tag = format!("( {} )", bt_icon);
            let tag_w = font_cache.measure_text(&bt_tag, font_size, false) as f64;
            let icon_hitbox_end = icon_x + tag_w;

            if y <= bt_box_y + 24.0 && x >= icon_x && x <= icon_hitbox_end {
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
