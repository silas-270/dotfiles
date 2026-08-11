use crate::api::audio::get_volume_and_mute;
use crate::render::FontCache;
use crate::theme::ThemeConfig;
use tiny_skia::PixmapMut;

pub fn render_volume(
    pixmap: &mut PixmapMut,
    font_cache: &mut FontCache,
    theme: &ThemeConfig,
    start_x: f32,
    top_y: f32,
    font_size: f32,
) -> f32 {
    let (vol, muted) = get_volume_and_mute();
    let content = if muted {
        "[ 󰝟 ]".to_string()
    } else {
        let pct = (vol * 100.0).round() as u32;
        let icon = if pct == 0 {
            "󰕿"
        } else if pct < 50 {
            "󰖀"
        } else {
            "󰕾"
        };
        format!("[ {} {}% ]", icon, pct)
    };

    font_cache.draw_gtk_box(pixmap, &content, start_x, top_y, font_size, theme.fg_muted, theme.border, 0.0)
}

pub fn handle_click() {
    std::thread::spawn(|| {
        let _ = std::process::Command::new("/home/silas270/.config/hypr/scripts/control-center/target/release/control-center").status();
    });
}
