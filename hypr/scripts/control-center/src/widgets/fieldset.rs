use tiny_skia::{Color, PixmapMut};
use crate::render;

/// Draws a fieldset container outline with a gap in the top horizontal border for title text.
pub fn draw_fieldset_outline(
    pixmap: &mut PixmapMut,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    thickness: f32,
    color: Color,
    gap_x: f32,
    gap_w: f32,
) {
    let top_y = y;
    let bottom_y = y + h - thickness;
    let left_x = x;
    let right_x = x + w - thickness;

    // Top left segment
    let left_seg_w = (gap_x - x).max(0.0);
    if left_seg_w > 0.0 {
        render::fill_rect(pixmap, x, top_y, left_seg_w, thickness, color);
    }

    // Top right segment
    let right_seg_start = gap_x + gap_w;
    let right_seg_w = ((x + w) - right_seg_start).max(0.0);
    if right_seg_w > 0.0 {
        render::fill_rect(pixmap, right_seg_start, top_y, right_seg_w, thickness, color);
    }

    // Bottom border segment
    render::fill_rect(pixmap, x, bottom_y, w, thickness, color);

    // Left border segment
    render::fill_rect(pixmap, left_x, y, thickness, h, color);

    // Right border segment
    render::fill_rect(pixmap, right_x, y, thickness, h, color);
}
