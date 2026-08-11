use crate::api::stats::get_cpu_usage;
use crate::render::FontCache;
use crate::theme::ThemeConfig;
use tiny_skia::PixmapMut;

pub fn render_cpu(
    pixmap: &mut PixmapMut,
    font_cache: &mut FontCache,
    theme: &ThemeConfig,
    start_x: f32,
    top_y: f32,
    font_size: f32,
) -> f32 {
    let usage = get_cpu_usage();
    let text = format!("[ CPU {}% ]", usage);
    font_cache.draw_module_box(pixmap, &text, start_x, top_y, font_size, theme.text_color, theme.sec_border)
}
