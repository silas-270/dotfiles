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

/// Stroke a rectangle with a solid color and line width.
pub fn stroke_rect(pixmap: &mut PixmapMut, x: f32, y: f32, w: f32, h: f32, color: Color, stroke_width: f32) {
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

// ── Font Cache for High-Performance Text Rendering ──────────────────────────

pub struct FontCache {
    font: Font<'static>,
    // Cached tuple: (width, height, min_x, min_y, alpha_data)
    glyph_cache: HashMap<(char, u32, bool), (u32, u32, i32, i32, Vec<u8>)>,
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

    pub fn measure_text(&mut self, text: &str, font_size: f32) -> f32 {
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
        color: Color,
    ) -> f32 {
        let scale = Scale::uniform(font_size);
        let v_metrics = self.font.v_metrics(scale);
        let baseline_y = top_y + v_metrics.ascent;
        let mut x = start_x;

        for c in text.chars() {
            let key = (c, font_size.to_bits(), false);
            let font = &self.font;

            let (w, h, min_x, min_y, alpha_data) = self.glyph_cache.entry(key).or_insert_with(|| {
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
                    (gw, gh, bb.min.x, bb.min.y, data)
                } else {
                    (0, 0, 0, 0, Vec::new())
                }
            });

            if *w > 0 && *h > 0 {
                let base_x = (x + *min_x as f32).round() as i32;
                let base_y = (baseline_y + *min_y as f32).round() as i32;

                for py in 0..*h {
                    for px in 0..*w {
                        let alpha = alpha_data[(py * *w + px) as usize];
                        if alpha > 0 {
                            let dest_x = base_x + px as i32;
                            let dest_y = base_y + py as i32;
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

            let g_scaled = font.glyph(c).scaled(scale);
            x += g_scaled.h_metrics().advance_width;
        }

        x
    }

    /// Measure a module box tile.
    pub fn measure_module_box(&mut self, content: &str, font_size: f32) -> f32 {
        let padding_x = 8.0;
        let text_w = self.measure_text(content, font_size);
        text_w + 2.0 * padding_x
    }

    pub fn measure_gtk_box(&mut self, content: &str, font_size: f32, min_width: f32) -> f32 {
        let padding_x = 8.0;
        let text_w = self.measure_text(content, font_size);
        (text_w + 2.0 * padding_x).max(min_width)
    }

    /// Draws a module box tile with 2px border and centered text padding.
    pub fn draw_module_box(
        &mut self,
        pixmap: &mut PixmapMut,
        content: &str,
        x: f32,
        y: f32,
        font_size: f32,
        text_color: Color,
        border_color: Color,
    ) -> f32 {
        let padding_x = 8.0;
        let text_w = self.measure_text(content, font_size);
        let box_w = text_w + 2.0 * padding_x;
        let box_h = font_size + 10.0; // 30px for 20px font

        stroke_rect(pixmap, x, y, box_w, box_h, border_color, 2.0);
        let text_x = x + padding_x;
        let text_y = y + 3.0;
        self.draw_text(pixmap, content, text_x, text_y, font_size, text_color);

        box_w
    }

    pub fn draw_gtk_box(
        &mut self,
        pixmap: &mut PixmapMut,
        content: &str,
        x: f32,
        y: f32,
        font_size: f32,
        text_color: Color,
        border_color: Color,
        min_width: f32,
    ) -> f32 {
        let padding_x = 8.0;
        let text_w = self.measure_text(content, font_size);
        let box_w = (text_w + 2.0 * padding_x).max(min_width);
        let box_h = font_size + 10.0;

        stroke_rect(pixmap, x, y, box_w, box_h, border_color, 2.0);
        let text_x = x + (box_w - text_w) / 2.0;
        let text_y = y + 3.0;
        self.draw_text(pixmap, content, text_x, text_y, font_size, text_color);

        box_w
    }
}
