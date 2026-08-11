use crate::render::FontCache;
use crate::theme::ThemeConfig;
use chrono::Local;
use std::sync::atomic::{AtomicBool, Ordering};
use tiny_skia::PixmapMut;

static ALT_MODE: AtomicBool = AtomicBool::new(false);

pub fn render_clock(
    pixmap: &mut PixmapMut,
    font_cache: &mut FontCache,
    theme: &ThemeConfig,
    start_x: f32,
    top_y: f32,
    font_size: f32,
) -> f32 {
    let now = Local::now();
    let text = if ALT_MODE.load(Ordering::Relaxed) {
        now.format("%a %d.%m.%Y").to_string()
    } else {
        now.format("%H:%M").to_string()
    };

    font_cache.draw_bracket_tag(pixmap, &text, start_x, top_y, font_size, theme.accent_color)
}

pub fn handle_click() {
    let current = ALT_MODE.load(Ordering::Relaxed);
    ALT_MODE.store(!current, Ordering::Relaxed);
}
