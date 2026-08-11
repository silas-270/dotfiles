use crate::api::hyprland::WorkspaceState;
use crate::render::FontCache;
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
        let color = if is_active {
            theme.accent_color
        } else {
            theme.text_color
        };

        let label = ws_id.to_string();
        let width = font_cache.measure_bracket_tag(&label, font_size);
        font_cache.draw_bracket_tag(pixmap, &label, x, top_y, font_size, color);

        buttons.push(WorkspaceButton {
            id: ws_id,
            x,
            width,
        });

        x += width + 4.0;
    }

    (x, buttons)
}
