use crate::render::{BarText, BracketSpacing};
use shell_common::font::FontCache;
use shell_common::theme::ThemeConfig;
use tiny_skia::PixmapMut;

pub fn is_fedora_mounted() -> bool {
    std::fs::read_to_string("/proc/mounts")
        .map(|content| content.lines().any(|line| line.contains("/mnt/fedora")))
        .unwrap_or(false)
}

pub fn render_fedora(
    pixmap: &mut PixmapMut,
    font_cache: &mut FontCache,
    theme: &ThemeConfig,
    start_x: f32,
    top_y: f32,
    font_size: f32,
) -> f32 {
    let content = "󰲝";
    font_cache.draw_gtk_box_with_spacing(
        pixmap,
        content,
        start_x,
        top_y,
        font_size,
        theme.fg_muted,
        theme.border,
        0.0,
        BracketSpacing::HalfAsymmetric,
    )
}

pub fn handle_click() {
    std::thread::spawn(|| {
        let _ = std::process::Command::new("/home/silas270/.local/bin/fedora-mount")
            .arg("toggle")
            .status();
    });
}
