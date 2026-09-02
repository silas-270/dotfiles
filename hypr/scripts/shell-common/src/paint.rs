//! Shared tiny-skia drawing primitives and the Wayland pixel-format bridge.

use tiny_skia::*;

/// Fill a rectangle with a solid color.
pub fn fill_rect(pixmap: &mut PixmapMut, x: f32, y: f32, w: f32, h: f32, color: Color) {
    if w <= 0.0 || h <= 0.0 {
        return;
    }
    if let Some(rect) = Rect::from_xywh(x, y, w, h) {
        let mut paint = Paint::default();
        paint.set_color(color);
        pixmap.fill_rect(rect, &paint, Transform::identity(), None);
    }
}

/// Stroke a rectangle with a solid color and line width, contained entirely inside the specified bounds.
pub fn stroke_rect(pixmap: &mut PixmapMut, x: f32, y: f32, w: f32, h: f32, color: Color, stroke_width: f32) {
    if w <= 0.0 || h <= 0.0 {
        return;
    }
    let half = stroke_width / 2.0;
    let x = x + half;
    let y = y + half;
    let w = w - stroke_width;
    let h = h - stroke_width;
    if w <= 0.0 || h <= 0.0 {
        return;
    }
    if let Some(rect) = Rect::from_xywh(x, y, w, h) {
        let mut paint = Paint::default();
        paint.set_color(color);
        let mut stroke = Stroke::default();
        stroke.width = stroke_width;
        let path = PathBuilder::from_rect(rect);
        pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    }
}

/// Copy tiny-skia premultiplied RGBA pixel data into a Wayland BGRA buffer.
pub fn rgba_to_bgra(src: &[u8], dst: &mut [u8]) {
    for (s, d) in src.chunks_exact(4).zip(dst.chunks_exact_mut(4)) {
        d[0] = s[2]; // B
        d[1] = s[1]; // G
        d[2] = s[0]; // R
        d[3] = s[3]; // A
    }
}
