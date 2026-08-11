use crate::api::hyprland::WorkspaceState;
use crate::render::{stroke_rect, FontCache};
use crate::theme::ThemeConfig;
use tiny_skia::PixmapMut;

pub struct WorkspaceButton {
    pub id: i32,
    pub x: f32,
    pub width: f32,
}

pub fn render_workspaces(
    pixmap: &mut PixmapMut,
    font_cache: &mut FontCache,
    state: &WorkspaceState,
    theme: &ThemeConfig,
    start_x: f32,
    top_y: f32,
    font_size: f32,
) -> (f32, Vec<WorkspaceButton>) {
    let mut x = start_x;
    let mut buttons = Vec::new();

    for &ws_id in &state.workspaces {
        let is_active = ws_id == state.active_id;
        let text_color = if is_active {
            theme.accent_color
        } else {
            theme.fg_muted
        };
        let border_color = if is_active {
            theme.waybar_border
        } else {
            theme.sec_border
        };

        let label = format!("[ {} ]", ws_id);
        let padding_x = 6.0;
        let text_w = font_cache.measure_text(&label, font_size);
        let box_w = text_w + 2.0 * padding_x;
        let box_h = font_size + 8.0;

        stroke_rect(pixmap, x, top_y, box_w, box_h, border_color, 2.0);
        font_cache.draw_text(pixmap, &label, x + padding_x, top_y + 4.0, font_size, text_color);

        buttons.push(WorkspaceButton {
            id: ws_id,
            x,
            width: box_w,
        });

        x += box_w + 6.0;
    }

    (x, buttons)
}
