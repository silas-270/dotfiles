use crate::api::stats::{get_battery_info, BatteryStatus};
use crate::render::FontCache;
use crate::theme::ThemeConfig;
use tiny_skia::PixmapMut;
use std::sync::Mutex;

static SHOW_PERCENT: Mutex<bool> = Mutex::new(false);

pub fn render_battery(
    pixmap: &mut PixmapMut,
    font_cache: &mut FontCache,
    theme: &ThemeConfig,
    start_x: f32,
    top_y: f32,
    font_size: f32,
) -> f32 {
    let info = get_battery_info();

    let (icon, color) = match info.status {
        BatteryStatus::Charging | BatteryStatus::Full => ("󰂄", theme.fg_muted),
        _ => {
            if info.capacity <= 15 {
                ("󰂎", theme.danger)
            } else if info.capacity <= 30 {
                ("󰁺", theme.warning)
            } else {
                ("󰁹", theme.fg_muted)
            }
        }
    };

    let show_pct = *SHOW_PERCENT.lock().unwrap();
    let text = if show_pct {
        format!("{} {}%", icon, info.capacity)
    } else {
        icon.to_string()
    };

    font_cache.draw_module_box(pixmap, &text, start_x, top_y, font_size, color, theme.border, false)
}

pub fn handle_click() {
    let mut show = SHOW_PERCENT.lock().unwrap();
    *show = !*show;
}
