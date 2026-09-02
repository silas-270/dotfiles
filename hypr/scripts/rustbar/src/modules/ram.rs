use crate::render::BarText;
use shell_common::font::FontCache;
use shell_common::theme::ThemeConfig;
use tiny_skia::PixmapMut;

pub fn render_ram(
    pixmap: &mut PixmapMut,
    font_cache: &mut FontCache,
    theme: &ThemeConfig,
    start_x: f32,
    top_y: f32,
    font_size: f32,
    ram_str: &str,
) -> f32 {
    font_cache.draw_gtk_box(pixmap, ram_str, start_x, top_y, font_size, theme.fg_muted, theme.border, 131.0, false)
}
