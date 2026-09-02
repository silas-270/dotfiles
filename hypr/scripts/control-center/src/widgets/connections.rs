use tiny_skia::PixmapMut;
use crate::render::FontCache;
use crate::theme::ThemeConfig;
use crate::api;
use crate::api::network::{NetInfo, NetKind};
use super::fieldset::{draw_section_header, draw_inner_box, draw_tree_corner};
use super::ActionResult;

pub struct ConnectionsSection;

impl ConnectionsSection {
    pub const HEIGHT: f32 = 195.0;

    fn wifi_signal_icon(signal: u8) -> &'static str {
        match signal {
            0..=20 => "󰤟",
            21..=40 => "󰤢",
            41..=60 => "󰤥",
            _ => "󰤨",
        }
    }

    /// The Wi-Fi radio marker is only needed when Wi-Fi is not the primary
    /// link; otherwise the main icon already conveys the radio state.
    fn shows_wifi_marker(net: &NetInfo) -> bool {
        net.kind == NetKind::Ethernet
            || (net.kind == NetKind::None && !net.wifi_enabled && net.eth_present)
    }

    /// True when the primary icon is an ethernet glyph rather than a Wi-Fi one.
    fn primary_is_ethernet(net: &NetInfo) -> bool {
        Self::shows_wifi_marker(net)
    }

    fn wifi_marker_text(net: &NetInfo) -> &'static str {
        if net.wifi_enabled { "󰖩 on" } else { "󰖪 off" }
    }

    pub fn draw(
        pixmap: &mut PixmapMut,
        font_cache: &mut FontCache,
        sec_x: f32,
        sec_y: f32,
        sec_w: f32,
        font_size: f32,
        theme: &ThemeConfig,
        net: &NetInfo,
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

        // The tile follows whatever is actually carrying traffic: a plugged-in
        // cable outranks Wi-Fi.
        let (primary_icon, primary_label) = match net.kind {
            NetKind::Ethernet => ("󰈀", " ETHERNET"),
            NetKind::Wifi => (Self::wifi_signal_icon(net.signal), " WIFI"),
            NetKind::None => {
                if net.wifi_enabled {
                    ("󰤯", " WIFI")
                } else if net.eth_present {
                    ("󰈂", " ETHERNET")
                } else {
                    ("󰖪", " WIFI")
                }
            }
        };

        // The calibrated tag exists to re-centre the Wi-Fi glyphs, whose ink sits
        // off-centre. The ethernet glyphs are centred already and use plain
        // symmetric brackets, like the Bluetooth row.
        let tag_w = if Self::primary_is_ethernet(net) {
            let tag = format!("( {} )", primary_icon);
            font_cache.draw_text(pixmap, &tag, icon_x, wifi_line1_y, font_size, false, accent_color);
            font_cache.measure_text(&tag, font_size, false)
        } else {
            font_cache.draw_calibrated_bracket_tag(pixmap, primary_icon, icon_x, wifi_line1_y, font_size, accent_color)
        };
        font_cache.draw_text(pixmap, primary_label, icon_x + tag_w, wifi_line1_y, font_size, false, accent_color);

        // When the cable is the live route the Wi-Fi radio state would
        // otherwise be invisible, so show it as a marker on the right. That
        // marker doubles as the radio toggle (see handle_click).
        if Self::shows_wifi_marker(net) {
            let marker = Self::wifi_marker_text(net);
            let marker_color = if net.wifi_enabled { accent_color } else { text_color };
            let marker_w = font_cache.measure_text(marker, font_size, false);
            let marker_x = wifi_box_x + wifi_box_w - marker_w - 8.0;
            font_cache.draw_text(pixmap, marker, marker_x, wifi_line1_y, font_size, false, marker_color);
        }

        let net_sub_text: &str = if net.kind == NetKind::None {
            if net.wifi_enabled {
                "Disconnected"
            } else if net.eth_present {
                // Labelled ETHERNET in this state, so describe the cable.
                "Unplugged"
            } else {
                "Disabled"
            }
        } else {
            &net.name
        };
        draw_tree_corner(pixmap, icon_x, wifi_line2_y, font_size, text_color);
        font_cache.draw_text(pixmap, net_sub_text, icon_x + 14.0, wifi_line2_y, font_size, false, text_color);

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
        net: &NetInfo,
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
            // The Wi-Fi glyph is always the radio toggle. It sits on the left
            // when Wi-Fi is the primary link, and moves to the right-hand
            // marker when a cable has taken over.
            let toggles_wifi = if Self::shows_wifi_marker(net) {
                let marker_w = font_cache.measure_text(Self::wifi_marker_text(net), font_size, false) as f64;
                let marker_x = sec_x + sec_w - marker_w - 8.0;
                y <= wifi_box_y + 35.0 && x >= marker_x && x <= marker_x + marker_w
            } else {
                let icon = if *wifi_enabled { "󰖩" } else { "󰖪" };
                let tag_w = font_cache.measure_calibrated_bracket_tag(icon, font_size) as f64;
                y <= wifi_box_y + 35.0 && x >= icon_x && x <= icon_x + tag_w
            };

            if toggles_wifi {
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
