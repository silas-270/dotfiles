use std::path::Path;

/// Load an SVG file, replace stroke/fill colours with `color`, and rasterise
/// it into a square `tiny_skia::Pixmap` of the given `size`.
///
/// This reproduces the original GTK4 PixbufLoader + SVG colour-injection
/// pipeline exactly.
pub fn load_svg_colored(path: &Path, color: &str, size: u32) -> Option<tiny_skia::Pixmap> {
    let bytes = std::fs::read(path).ok()?;
    let svg_str = std::str::from_utf8(&bytes).ok()?;

    // Inject colours — same replacements as the GTK4 version
    let modified = svg_str
        .replace("currentColor", color)
        .replace("width=\"24\"", "")
        .replace("height=\"24\"", "")
        .replace("<path", &format!("<path stroke=\"{}\" fill=\"none\"", color))
        .replace("<line", &format!("<line stroke=\"{}\" fill=\"none\"", color))
        .replace("stroke=\"currentColor\"", &format!("stroke=\"{}\"", color))
        .replace("stroke=\"#ffffff\"", &format!("stroke=\"{}\"", color));

    let options = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_str(&modified, &options).ok()?;

    let mut pixmap = tiny_skia::Pixmap::new(size, size)?;
    let ts = tree.size();
    let scale = (size as f32 / ts.width()).min(size as f32 / ts.height());
    let tx = (size as f32 - ts.width() * scale) / 2.0;
    let ty = (size as f32 - ts.height() * scale) / 2.0;
    let transform =
        tiny_skia::Transform::from_scale(scale, scale).post_translate(tx, ty);

    resvg::render(&tree, transform, &mut pixmap.as_mut());
    Some(pixmap)
}

/// Load a raster image (PNG/JPEG) from `path` and scale it to `w × h`.
/// Returns a premultiplied-alpha tiny-skia `Pixmap`.
pub fn load_image_scaled(path: &str, w: u32, h: u32) -> Option<tiny_skia::Pixmap> {
    let img = image::open(path)
        .ok()?
        .resize_exact(w, h, image::imageops::FilterType::Lanczos3)
        .into_rgba8();

    let mut pixmap = tiny_skia::Pixmap::new(w, h)?;
    // `image` crate stores straight-alpha RGBA; convert to premultiplied for
    // tiny-skia.
    let src = img.as_raw();
    let dst = pixmap.data_mut();
    for (s, d) in src.chunks_exact(4).zip(dst.chunks_exact_mut(4)) {
        let a = s[3] as f32 / 255.0;
        d[0] = (s[0] as f32 * a) as u8; // R
        d[1] = (s[1] as f32 * a) as u8; // G
        d[2] = (s[2] as f32 * a) as u8; // B
        d[3] = s[3]; // A
    }
    Some(pixmap)
}
