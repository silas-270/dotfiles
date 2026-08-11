use crate::render::FontCache;
use crate::theme::ThemeConfig;
use tiny_skia::PixmapMut;

pub fn render_bluetooth(
    pixmap: &mut PixmapMut,
    font_cache: &mut FontCache,
    theme: &ThemeConfig,
    start_x: f32,
    top_y: f32,
    font_size: f32,
) -> f32 {
    let content = "[  ]";
    font_cache.draw_module_box(pixmap, content, start_x, top_y, font_size, theme.fg_muted, theme.border)
}

pub fn handle_click() {
    std::thread::spawn(|| {
        let _ = std::process::Command::new("/home/silas270/.config/hypr/scripts/rofi-bluetooth-menu.sh").status();
    });
}
