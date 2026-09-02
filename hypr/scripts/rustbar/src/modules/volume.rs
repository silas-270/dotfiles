use shell_common::api::audio::get_volume_and_mute;
use crate::render::BarText;
use shell_common::font::FontCache;
use shell_common::theme::ThemeConfig;
use tiny_skia::PixmapMut;
use std::sync::Mutex;
use std::time::{Instant, Duration};

static LAST_CHANGE: Mutex<Option<Instant>> = Mutex::new(None);
static LAST_PCT: Mutex<u32> = Mutex::new(u32::MAX);
static LAST_MUTED: Mutex<bool> = Mutex::new(false);
static SHOW_PERCENT: Mutex<bool> = Mutex::new(false);

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

    // Read manual toggle (separate, immediately-released lock)
    let manual = *SHOW_PERCENT.lock().unwrap();

    // Track auto-show when volume/mute changes
    let auto_show = {
        let prev_pct = *LAST_PCT.lock().unwrap();
        let prev_muted = *LAST_MUTED.lock().unwrap();
        if prev_pct != pct || prev_muted != muted {
            *LAST_PCT.lock().unwrap() = pct;
            *LAST_MUTED.lock().unwrap() = muted;
            *LAST_CHANGE.lock().unwrap() = Some(Instant::now());
            true
        } else {
            LAST_CHANGE.lock().unwrap()
                .map(|t| t.elapsed() < Duration::from_millis(1500))
                .unwrap_or(false)
        }
    };

    let show_pct = manual || auto_show;

    let icon = if muted {
        "󰝟"
    } else if pct == 0 {
        "󰕿"
    } else if pct < 50 {
        "󰖀"
    } else {
        "󰕾"
    };
    let text = if show_pct {
        format!("{} {}%", icon, pct)
    } else {
        icon.to_string()
    };

    font_cache.draw_module_box(pixmap, &text, start_x, top_y, font_size, theme.fg_muted, theme.border, false)
}

pub fn handle_click() {
    let mut show = SHOW_PERCENT.lock().unwrap();
    *show = !*show;
}
