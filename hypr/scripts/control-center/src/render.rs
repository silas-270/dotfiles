use tiny_skia::*;
use rusttype::{Font, Scale};
use std::collections::HashMap;

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

/// Copy tiny-skia premultiplied RGBA pixel data into a Wayland BGRA buffer.
pub fn rgba_to_bgra(src: &[u8], dst: &mut [u8]) {
    for (s, d) in src.chunks_exact(4).zip(dst.chunks_exact_mut(4)) {
        d[0] = s[2]; // B
        d[1] = s[1]; // G
        d[2] = s[0]; // R
        d[3] = s[3]; // A
    }
}

// ── Font Cache for High-Performance Text Rendering ──────────────────────────

pub struct FontCache {
    font: Font<'static>,
    glyph_cache: HashMap<(char, u32, bool), (u32, u32, Vec<u8>)>,
}

impl FontCache {
    pub fn new() -> Self {
        let font_data: &[u8] = include_bytes!("/home/silas270/.local/share/fonts/JetBrainsMono/JetBrainsMonoNerdFont-Medium.ttf");
        let font = Font::try_from_bytes(font_data)
            .expect("Failed to load JetBrainsMonoNerdFont-Medium font");

        Self {
            font,
            glyph_cache: HashMap::new(),
        }
    }

    pub fn measure_text(&mut self, text: &str, font_size: f32, _bold: bool) -> f32 {
        let scale = Scale::uniform(font_size);
        let mut x = 0.0;
        for c in text.chars() {
            let glyph = self.font.glyph(c).scaled(scale);
            x += glyph.h_metrics().advance_width;
        }
        x
    }

    pub fn draw_text(
        &mut self,
        pixmap: &mut PixmapMut,
        text: &str,
        start_x: f32,
        top_y: f32,
        font_size: f32,
        bold: bool,
        color: Color,
    ) -> f32 {
        let scale = Scale::uniform(font_size);
        let v_metrics = self.font.v_metrics(scale);
        let baseline_y = top_y + v_metrics.ascent;
        let mut x = start_x;

        for c in text.chars() {
            let key = (c, font_size.to_bits(), bold);
            let font = &self.font;

            let (w, h, alpha_data) = self.glyph_cache.entry(key).or_insert_with(|| {
                let g = font.glyph(c).scaled(scale).positioned(rusttype::point(0.0, 0.0));
                if let Some(bb) = g.pixel_bounding_box() {
                    let gw = bb.width() as u32;
                    let gh = bb.height() as u32;
                    let mut data = vec![0u8; (gw * gh) as usize];
                    g.draw(|px, py, v| {
                        let idx = (py * gw + px) as usize;
                        if idx < data.len() {
                            data[idx] = (v * 255.0) as u8;
                        }
                    });
                    (gw, gh, data)
                } else {
                    (0, 0, Vec::new())
                }
            });

            if *w > 0 && *h > 0 {
                let g = font.glyph(c).scaled(scale).positioned(rusttype::point(x, baseline_y));
                if let Some(bb) = g.pixel_bounding_box() {
                    let gx = bb.min.x as f32;
                    let gy = bb.min.y as f32;

                    for py in 0..*h {
                        for px in 0..*w {
                            let alpha = alpha_data[(py * *w + px) as usize];
                            if alpha > 0 {
                                let dest_x = (gx + px as f32) as i32;
                                let dest_y = (gy + py as f32) as i32;
                                if dest_x >= 0 && dest_x < pixmap.width() as i32 && dest_y >= 0 && dest_y < pixmap.height() as i32 {
                                    let px_alpha = (alpha as f32 / 255.0) * color.alpha();
                                    if let Some(col) = Color::from_rgba(color.red(), color.green(), color.blue(), px_alpha) {
                                        let mut paint = Paint::default();
                                        paint.set_color(col);
                                        if let Some(r) = Rect::from_xywh(dest_x as f32, dest_y as f32, 1.0, 1.0) {
                                            pixmap.fill_rect(r, &paint, Transform::identity(), None);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            let g_scaled = font.glyph(c).scaled(scale);
            x += g_scaled.h_metrics().advance_width;
        }

        x
    }

    /// Measure a calibrated bracket tag to match exact standard bracket tag width (e.g. (  )).
    pub fn measure_calibrated_bracket_tag(&mut self, _icon: &str, font_size: f32) -> f32 {
        self.measure_text("(  )", font_size, false)
    }

    /// Draw a calibrated bracket tag ( icon ) with icon position at the visual sweet spot (75% towards center).
    pub fn draw_calibrated_bracket_tag(
        &mut self,
        pixmap: &mut PixmapMut,
        icon: &str,
        start_x: f32,
        top_y: f32,
        font_size: f32,
        color: Color,
    ) -> f32 {
        let target_w = self.measure_calibrated_bracket_tag(icon, font_size);
        let bracket_w = self.measure_text("(", font_size, false);
        let icon_w = self.measure_text(icon, font_size, false);

        // 1. Draw "("
        self.draw_text(pixmap, "(", start_x, top_y, font_size, false, color);

        // 2. Draw Icon at the visual sweet spot (halfway between mid_offset and center_offset)
        let inner_w = target_w - 2.0 * bracket_w;
        let center_offset = (inner_w - icon_w) / 2.0;
        let mid_offset = (2.0 + center_offset) / 2.0;
        let sweet_spot_offset = (center_offset + mid_offset) / 2.0;
        let icon_x = start_x + bracket_w + sweet_spot_offset;
        self.draw_text(pixmap, icon, icon_x, top_y, font_size, false, color);

        // 3. Draw ")" at exact target_w end position to preserve vertical alignment
        let right_bracket_x = start_x + target_w - bracket_w;
        self.draw_text(pixmap, ")", right_bracket_x, top_y, font_size, false, color);

        target_w
    }
}
