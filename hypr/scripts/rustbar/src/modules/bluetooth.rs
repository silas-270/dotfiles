use crate::api::connectivity::bt_state;
use crate::render::BarText;
use shell_common::font::FontCache;
use shell_common::theme::ThemeConfig;
use tiny_skia::PixmapMut;

pub fn render_bluetooth(
    pixmap: &mut PixmapMut,
    font_cache: &mut FontCache,
    theme: &ThemeConfig,
    start_x: f32,
    top_y: f32,
    font_size: f32,
) -> f32 {
    // Cheap: reads a cached snapshot refreshed on a background thread.
    let state = bt_state();

    // Uniform with the CPU/RAM boxes; the glyph carries the state, not colour.
    let content = if !state.powered {
        "󰂲".to_string()
    } else if state.connected() {
        match state.battery {
            Some(b) => format!("󰂱 {}%", b),
            None => "󰂱".to_string(),
        }
    } else {
        "".to_string()
    };

    font_cache.draw_gtk_box(
        pixmap, &content, start_x, top_y, font_size, theme.fg_muted, theme.border, 0.0, false,
    )
}

pub fn handle_click() {
    std::thread::spawn(|| {
        let _ = std::process::Command::new("/home/silas270/.config/hypr/scripts/rofi-bluetooth-menu.sh").status();
        if let Ok(mut stream) = std::os::unix::net::UnixStream::connect("/home/silas270/dotfiles/hypr/scripts/rustbar/rustbar.sock") {
            use std::io::Write;
            let _ = stream.write_all(b"panel_closed");
        }
    });
}
