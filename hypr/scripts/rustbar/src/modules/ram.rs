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
    let text = get_ram_display();
    font_cache.draw_bracket_tag(pixmap, &text, start_x, top_y, font_size, theme.text_color)
}
