use crate::render::FontCache;
use crate::theme::ThemeConfig;
use tiny_skia::PixmapMut;

pub fn render_network(
    pixmap: &mut PixmapMut,
    font_cache: &mut FontCache,
    theme: &ThemeConfig,
    start_x: f32,
    top_y: f32,
    font_size: f32,
) -> f32 {
    let content = "[ 󰖩 ]";
    font_cache.draw_gtk_box(pixmap, content, start_x, top_y, font_size, theme.fg_muted, theme.border, 0.0)
}

pub fn handle_click() {
    std::thread::spawn(|| {
        let _ = std::process::Command::new("/home/silas270/.config/hypr/scripts/rofi-wifi-menu.sh").status();
    });
}
