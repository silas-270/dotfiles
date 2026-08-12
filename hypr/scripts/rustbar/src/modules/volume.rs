use crate::api::audio::get_volume_and_mute;
use crate::render::FontCache;
use crate::theme::ThemeConfig;
use tiny_skia::PixmapMut;
use std::sync::Mutex;
use std::time::{Instant, Duration};

static LAST_STATE: Mutex<Option<(u32, bool, Instant)>> = Mutex::new(None);

pub fn render_volume(
    pixmap: &mut PixmapMut,
    font_cache: &mut FontCache,
    theme: &ThemeConfig,
    start_x: f32,
    top_y: f32,
    font_size: f32,
) -> f32 {
    let (vol, muted) = get_volume_and_mute();
    let pct = (vol * 100.0).round() as u32;

    let show_pct = {
        let mut last = LAST_STATE.lock().unwrap();
        if let Some((last_pct, last_muted, last_time)) = *last {
            if last_pct != pct || last_muted != muted {
                *last = Some((pct, muted, Instant::now()));
                true
            } else {
                last_time.elapsed() < Duration::from_secs(1)
            }
        } else {
            *last = Some((pct, muted, Instant::now()));
            false // initially hide percentage (matching waybar-volume.sh print_json 0)
        }
    };

    let text = if muted {
        "󰝟".to_string()
    } else {
        let icon = if pct == 0 {
            "󰕿"
        } else if pct < 50 {
            "󰖀"
        } else {
            "󰕾"
        };
        if show_pct {
            format!("{} {}%", icon, pct)
        } else {
            icon.to_string()
        }
    };

    font_cache.draw_module_box(pixmap, &text, start_x, top_y, font_size, theme.fg_muted, theme.border, false)
}

pub fn handle_click() {
    std::thread::spawn(|| {
        let _ = std::process::Command::new("/home/silas270/.config/hypr/scripts/control-center/target/release/control-center").status();
    });
}
