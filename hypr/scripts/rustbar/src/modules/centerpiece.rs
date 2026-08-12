use crate::api::media::get_centerpiece_text;
use crate::render::FontCache;
use crate::theme::ThemeConfig;
use tiny_skia::PixmapMut;

pub fn render_centerpiece(
    pixmap: &mut PixmapMut,
    font_cache: &mut FontCache,
    theme: &ThemeConfig,
    start_x: f32,
    top_y: f32,
    font_size: f32,
) -> f32 {
    let (text, _is_playing) = get_centerpiece_text();
    let label = text;
    font_cache.draw_module_box(pixmap, &label, start_x, top_y, font_size, theme.accent_color, theme.border, false)
}

pub fn handle_click() {
    std::thread::spawn(|| {
        let _ = std::process::Command::new("/home/silas270/.config/hypr/scripts/rofi-power-profile-menu.sh").status();
        if let Ok(mut stream) = std::os::unix::net::UnixStream::connect("/home/silas270/dotfiles/hypr/scripts/rustbar/rustbar.sock") {
            use std::io::Write;
            let _ = stream.write_all(b"panel_closed");
        }
    });
}
