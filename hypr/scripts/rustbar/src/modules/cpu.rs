use crate::render::BarText;
use shell_common::font::FontCache;
use shell_common::theme::ThemeConfig;
use tiny_skia::PixmapMut;

pub fn render_cpu(
    pixmap: &mut PixmapMut,
    font_cache: &mut FontCache,
    theme: &ThemeConfig,
    start_x: f32,
    top_y: f32,
    font_size: f32,
    usage: u32,
) -> f32 {
    let text = format!("CPU {}%", usage);
    font_cache.draw_gtk_box(pixmap, &text, start_x, top_y, font_size, theme.fg_muted, theme.border, 131.0, false)
}
