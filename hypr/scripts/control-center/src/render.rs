use tiny_skia::*;

/// Build a rounded-rectangle path using cubic Bézier arcs.
pub fn rounded_rect_path(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<Path> {
    if w <= 0.0 || h <= 0.0 {
        return None;
    }
    let r = r.min(w / 2.0).min(h / 2.0);
    if r <= 0.0 {
        let rect = Rect::from_xywh(x, y, w, h)?;
        let mut pb = PathBuilder::new();
        pb.push_rect(rect);
        return pb.finish();
    }
    // Kappa constant for approximating quarter-circle arcs with cubic Béziers.
    let k: f32 = 0.55228;
    let kr = r * k;
    let mut pb = PathBuilder::new();

    pb.move_to(x + r, y);
    // Top edge → top-right corner
    pb.line_to(x + w - r, y);
    pb.cubic_to(x + w - r + kr, y, x + w, y + r - kr, x + w, y + r);
    // Right edge → bottom-right corner
    pb.line_to(x + w, y + h - r);
    pb.cubic_to(x + w, y + h - r + kr, x + w - r + kr, y + h, x + w - r, y + h);
    // Bottom edge → bottom-left corner
    pb.line_to(x + r, y + h);
    pb.cubic_to(x + r - kr, y + h, x, y + h - r + kr, x, y + h - r);
    // Left edge → top-left corner
    pb.line_to(x, y + r);
    pb.cubic_to(x, y + r - kr, x + r - kr, y, x + r, y);

    pb.close();
    pb.finish()
}

/// Fill a rounded rectangle with a solid colour.
pub fn fill_rounded_rect(
    pixmap: &mut PixmapMut,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    radius: f32,
    color: Color,
) {
    if let Some(path) = rounded_rect_path(x, y, w, h, radius) {
        let paint = Paint {
            shader: Shader::SolidColor(color),
            anti_alias: true,
            ..Paint::default()
        };
        pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
    }
}

/// Fill a plain (non-rounded) rectangle.
pub fn fill_rect(pixmap: &mut PixmapMut, x: f32, y: f32, w: f32, h: f32, color: Color) {
    fill_rounded_rect(pixmap, x, y, w, h, 0.0, color);
}

/// Draw a rectangle outline with specified thickness.
pub fn draw_rect_outline(
    pixmap: &mut PixmapMut,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    thickness: f32,
    color: Color,
) {
    // Top
    fill_rect(pixmap, x, y, w, thickness, color);
    // Bottom
    fill_rect(pixmap, x, y + h - thickness, w, thickness, color);
    // Left
    fill_rect(pixmap, x, y, thickness, h, color);
    // Right
    fill_rect(pixmap, x + w - thickness, y, thickness, h, color);
}


/// Composite a source `Pixmap` into `dest` at `(dx,dy)`, scaling it to
/// `(dw,dh)` pixels using bilinear filtering.
pub fn draw_pixmap_scaled(
    dest: &mut PixmapMut,
    src: PixmapRef,
    dx: f32,
    dy: f32,
    dw: f32,
    dh: f32,
) {
    let sw = src.width() as f32;
    let sh = src.height() as f32;
    if sw <= 0.0 || sh <= 0.0 || dw <= 0.0 || dh <= 0.0 {
        return;
    }
    let scale_x = dw / sw;
    let scale_y = dh / sh;
    let transform = Transform::from_scale(scale_x, scale_y).post_translate(dx, dy);

    dest.draw_pixmap(
        0,
        0,
        src,
        &PixmapPaint {
            opacity: 1.0,
            blend_mode: BlendMode::SourceOver,
            quality: FilterQuality::Bilinear,
        },
        transform,
        None,
    );
}

/// Copy tiny-skia premultiplied **RGBA** pixel data into a Wayland
/// `WL_SHM_FORMAT_ARGB8888` buffer (which is **BGRA** byte-order on
/// little-endian machines).
pub fn rgba_to_bgra(src: &[u8], dst: &mut [u8]) {
    for (s, d) in src.chunks_exact(4).zip(dst.chunks_exact_mut(4)) {
        d[0] = s[2]; // B
        d[1] = s[1]; // G
        d[2] = s[0]; // R
        d[3] = s[3]; // A
    }
}
