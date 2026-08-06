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

/// Draws a section header with title text followed by a horizontal divider line extending to the right edge.
/// Upper margin (space above title text from upper rect bottom) = 14px (matching inter-box gap).
/// Lower margin (space below title text to top of lower rect) = 14px (matching inter-box gap).
pub fn draw_section_header(
    pixmap: &mut PixmapMut,
    font_cache: &mut crate::render::FontCache,
    title: &str,
    sec_x: f32,
    sec_y: f32,
    sec_w: f32,
    font_size: f32,
    accent_color: Color,
    line_color: Color,
) {
    let text_top_y = sec_y + 14.0;
    font_cache.draw_text(pixmap, title, sec_x, text_top_y, font_size, false, accent_color);
    let title_w = font_cache.measure_text(title, font_size, false);
    let line_start_x = sec_x + title_w + 8.0;
    let line_w = (sec_x + sec_w - line_start_x).max(0.0);
    let line_y = text_top_y + 11.0;
    render::fill_rect(pixmap, line_start_x, line_y, line_w, 2.0, line_color);
}

/// Manually renders a tree corner symbol (└) with exact dimensions:
/// - Vertical part: short stem (starting at y + 5.0 down to y + font_size / 2.0)
/// - Horizontal part: at exact vertical middle height of character (y + font_size / 2.0)
pub fn draw_tree_corner(
    pixmap: &mut PixmapMut,
    x: f32,
    y: f32,
    font_size: f32,
    color: Color,
) {
    let half_h = font_size / 2.0; // 10.5px
    let stroke = 1.5;
    let mid_y = y + half_h;
    let start_y = y + 5.0;
    let v_height = mid_y - start_y;
    // Vertical part: short stem down to middle height
    render::fill_rect(pixmap, x + 2.0, start_y, stroke, v_height, color);
    // Horizontal part: exact vertical middle height
    render::fill_rect(pixmap, x + 2.0, mid_y, 7.0, stroke, color);
}



/// Draws an inner control tile with background fill and subtle outline.
pub fn draw_inner_box(
    pixmap: &mut PixmapMut,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    fill_color: Option<Color>,
    border_color: Color,
    border_thickness: f32,
) {
    if w <= 0.0 || h <= 0.0 {
        return;
    }
    if let Some(fill) = fill_color {
        render::fill_rect(pixmap, x, y, w, h, fill);
    }
    if border_thickness > 0.0 {
        render::fill_rect(pixmap, x, y, w, border_thickness, border_color);
        render::fill_rect(pixmap, x, y + h - border_thickness, w, border_thickness, border_color);
        render::fill_rect(pixmap, x, y, border_thickness, h, border_color);
        render::fill_rect(pixmap, x + w - border_thickness, y, border_thickness, h, border_color);
    }
}

