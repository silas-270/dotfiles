use crate::api::stats::get_ram_display;
use crate::render::FontCache;
use crate::theme::ThemeConfig;
use tiny_skia::PixmapMut;

pub fn render_ram(
    pixmap: &mut PixmapMut,
    font_cache: &mut FontCache,
    theme: &ThemeConfig,
    start_x: f32,
    top_y: f32,
    font_size: f32,
) -> f32 {
    let ram_str = get_ram_display(); // returns e.g. "[ RAM 4.2G ]"
    font_cache.draw_gtk_box(pixmap, &ram_str, start_x, top_y, font_size, theme.fg_muted, theme.border, 115.0)
}
