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
    let mut buttons = Vec::new();

    // 1. Calculate total width for outer #workspaces container
    let mut content_w = 0.0;
    let button_padding_x = 4.0;
    let button_min_w = 24.0;
    let mut btn_widths = Vec::new();

    for &ws_id in &state.workspaces {
        let label = format!("[ {} ]", ws_id);
        let text_w = font_cache.measure_text(&label, font_size);
        let btn_w = (text_w + 2.0 * button_padding_x).max(button_min_w);
        btn_widths.push(btn_w);
        content_w += btn_w;
    }

    let outer_padding_x = 2.0; // GTK #workspaces padding: 0px 2px
    let border_w = 2.0;        // GTK border: 2px solid @border
    let box_w = content_w + 2.0 * (outer_padding_x + border_w);
    let box_h = font_size + 8.0; // 24px

    // 2. Draw outer 2px border container for #workspaces
    stroke_rect(pixmap, start_x, top_y, box_w, box_h, theme.border, border_w);

    // 3. Render workspace buttons inside container
    let mut curr_x = start_x + border_w + outer_padding_x;
    for (idx, &ws_id) in state.workspaces.iter().enumerate() {
        let btn_w = btn_widths[idx];
        let is_active = ws_id == state.active_id;

        let text_color = if is_active {
            theme.accent_color // GTK button.active: color: @accent (#00FF41)
        } else {
            theme.fg_muted     // GTK button: color: @fg-muted (#00B32D)
        };

        let label = format!("[ {} ]", ws_id);
        let text_w = font_cache.measure_text(&label, font_size);
        let text_x = curr_x + (btn_w - text_w) / 2.0;
        let text_y = top_y + 2.0 + border_w / 2.0;

        font_cache.draw_text(pixmap, &label, text_x, text_y, font_size, text_color);

        buttons.push(WorkspaceButton {
            id: ws_id,
            x: curr_x,
            width: btn_w,
        });

        curr_x += btn_w;
    }

    (start_x + box_w, buttons)
}
