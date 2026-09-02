use crate::api::connectivity::{net_state, NetKind};
use crate::render::{BracketSpacing, FontCache};
use crate::theme::ThemeConfig;
use tiny_skia::PixmapMut;

fn signal_icon(signal: u8) -> &'static str {
    match signal {
        0..=20 => "󰤟",
        21..=40 => "󰤢",
        41..=60 => "󰤥",
        _ => "󰤨",
    }
}

pub fn render_network(
    pixmap: &mut PixmapMut,
    font_cache: &mut FontCache,
    theme: &ThemeConfig,
    start_x: f32,
    top_y: f32,
    font_size: f32,
) -> f32 {
    // Cheap: reads a cached snapshot refreshed on a background thread.
    let state = net_state();

    // The asymmetric bracket spacing is a calibration for the Wi-Fi glyphs,
    // whose ink sits off-centre. The ethernet glyphs need only half that
    // offset, so they use the HalfAsymmetric step.
    let (icon, spacing) = match state.kind {
        NetKind::Ethernet => ("󰈀", BracketSpacing::HalfAsymmetric),
        NetKind::Wifi => (signal_icon(state.signal), BracketSpacing::Asymmetric),
        NetKind::None => {
            if state.wifi_enabled {
                ("󰤯", BracketSpacing::Asymmetric)
            } else if state.eth_present {
                ("󰈂", BracketSpacing::HalfAsymmetric)
            } else {
                ("󰖪", BracketSpacing::Asymmetric)
            }
        }
    };

    font_cache.draw_gtk_box_with_spacing(
        pixmap, icon, start_x, top_y, font_size,
        theme.fg_muted, theme.border, 0.0, spacing,
    )
}

pub fn handle_click() {
    std::thread::spawn(|| {
        let _ = std::process::Command::new("/home/silas270/.config/hypr/scripts/rofi-wifi-menu.sh").status();
        if let Ok(mut stream) = std::os::unix::net::UnixStream::connect("/home/silas270/dotfiles/hypr/scripts/rustbar/rustbar.sock") {
            use std::io::Write;
            let _ = stream.write_all(b"panel_closed");
        }
    });
}
