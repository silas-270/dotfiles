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

// ── Font Cache for High-Performance Text Rendering ──────────────────────────

pub struct FontCache {
    font: Font<'static>,
    // Cached: (width, height, min_x, min_y, alpha_data)
    // min_x/min_y are the glyph offsets from a (0,0)-positioned glyph
    glyph_cache: HashMap<(char, u32), (u32, u32, i32, i32, Vec<u8>)>,
}

impl FontCache {
    pub fn new() -> Self {
        // font-weight: 400 (Regular) matches Waybar CSS: font-weight: 400
        let font_data: &[u8] = include_bytes!("../fonts/JetBrainsMonoNerdFont-Regular.ttf");
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
            let key = (c, font_size.to_bits());
            let font = &self.font;

        // Cache bitmap AND the (0,0)-relative offsets so Y is consistent across all chars
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
            // Use snapped baseline + cached offsets for pixel-consistent placement.
            // This ensures every character in a string lands on the same baseline row.
            let dest_x = (x + *min_x as f32).round() as i32;
            let dest_y = (baseline_y.round() + *min_y as f32) as i32;

            let alpha_data = alpha_data.clone();
            let (w, h) = (*w, *h);
            for py in 0..h {
                for px in 0..w {
                    let alpha = alpha_data[(py * w + px) as usize];
                    if alpha > 0 {
                        let sx = dest_x + px as i32;
                        let sy = dest_y + py as i32;
                        if sx >= 0 && sx < pixmap.width() as i32
                            && sy >= 0 && sy < pixmap.height() as i32
                        {
                            let px_alpha = (alpha as f32 / 255.0) * color.alpha();
                            if let Some(col) = Color::from_rgba(
                                color.red(), color.green(), color.blue(), px_alpha,
                            ) {
                                let mut paint = Paint::default();
                                paint.set_color(col);
                                if let Some(r) = Rect::from_xywh(sx as f32, sy as f32, 1.0, 1.0) {
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

    /// Same as `draw_text`, but skips any pixel outside [clip_top, clip_bottom) — used for
    /// slide-in/out line transitions so text doesn't bleed past its module box.
    pub fn draw_text_clipped(
        &mut self,
        pixmap: &mut PixmapMut,
        text: &str,
        start_x: f32,
        top_y: f32,
        font_size: f32,
        color: Color,
        clip_top: f32,
        clip_bottom: f32,
    ) -> f32 {
        let scale = Scale::uniform(font_size);
        let v_metrics = self.font.v_metrics(scale);
        let baseline_y = top_y + v_metrics.ascent;
        let mut x = start_x;

        for c in text.chars() {
            let key = (c, font_size.to_bits());
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
                let dest_x = (x + *min_x as f32).round() as i32;
                let dest_y = (baseline_y.round() + *min_y as f32) as i32;

                let alpha_data = alpha_data.clone();
                let (w, h) = (*w, *h);
                for py in 0..h {
                    for px in 0..w {
                        let alpha = alpha_data[(py * w + px) as usize];
                        if alpha > 0 {
                            let sx = dest_x + px as i32;
                            let sy = dest_y + py as i32;
                            if sx >= 0 && sx < pixmap.width() as i32
                                && sy >= 0 && sy < pixmap.height() as i32
                                && (sy as f32) >= clip_top && (sy as f32) < clip_bottom
                            {
                                let px_alpha = (alpha as f32 / 255.0) * color.alpha();
                                if let Some(col) = Color::from_rgba(
                                    color.red(), color.green(), color.blue(), px_alpha,
                                ) {
                                    let mut paint = Paint::default();
                                    paint.set_color(col);
                                    if let Some(r) = Rect::from_xywh(sx as f32, sy as f32, 1.0, 1.0) {
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

    pub fn text_height(&mut self, font_size: f32) -> f32 {
        let scale = Scale::uniform(font_size);
        let v = self.font.v_metrics(scale);
        v.ascent - v.descent
    }

    /// Measure a calibrated bracket tag to match exact Waybar width (e.g. [ icon ]).
    pub fn measure_calibrated_bracket_tag(&mut self, text: &str, font_size: f32, asymmetric: bool) -> f32 {
        let bracket_w = self.measure_text("[", font_size);
        let space_w = 5.33; // Locked to 16px nominal U+2004 width (16.0 / 3.0)
        let text_w = self.measure_text(text, font_size);
        if asymmetric {
            let right_space_w = self.measure_text(" ", font_size);
            bracket_w * 2.0 + space_w + right_space_w + text_w
        } else {
            bracket_w * 2.0 + space_w * 2.0 + text_w
        }
    }

    /// Draw a calibrated bracket tag [ icon ] with icon centered between brackets.
    pub fn draw_calibrated_bracket_tag(
        &mut self,
        pixmap: &mut PixmapMut,
        text: &str,
        start_x: f32,
        top_y: f32,
        font_size: f32,
        color: Color,
        asymmetric: bool,
    ) -> f32 {
        let target_w = self.measure_calibrated_bracket_tag(text, font_size, asymmetric);
        let bracket_w = self.measure_text("[", font_size);
        let text_w = self.measure_text(text, font_size);

        // 1. Draw "["
        self.draw_text(pixmap, "[", start_x, top_y, font_size, color);

        // 2. Draw inner text
        let text_x = if asymmetric {
            start_x + bracket_w + 5.33
        } else {
            start_x + (target_w - text_w) / 2.0
        };
        self.draw_text(pixmap, text, text_x, top_y, font_size, color);

        // 3. Draw "]" at exact end
        let right_bracket_x = start_x + target_w - bracket_w;
        self.draw_text(pixmap, "]", right_bracket_x, top_y, font_size, color);

        target_w
    }

    /// Measure GTK module box (padding: 6px, border: 2px solid)
    pub fn measure_gtk_box(&mut self, content: &str, font_size: f32, min_width: f32, asymmetric: bool) -> f32 {
        let padding_x = 6.0;
        let border_w = 2.0;
        let text_w = self.measure_calibrated_bracket_tag(content, font_size, asymmetric);
        (text_w + 2.0 * (padding_x + border_w)).max(min_width)
    }

    pub fn draw_module_box(
        &mut self,
        pixmap: &mut PixmapMut,
        content: &str,
        x: f32,
        y: f32,
        font_size: f32,
        text_color: Color,
        border_color: Color,
        asymmetric: bool,
    ) -> f32 {
        self.draw_gtk_box(pixmap, content, x, y, font_size, text_color, border_color, 0.0, asymmetric)
    }

    /// Draws a GTK module box tile matching GTK Waybar style.css (2px border, 6px padding)
    pub fn draw_gtk_box(
        &mut self,
        pixmap: &mut PixmapMut,
        inner_text: &str,
        x: f32,
        y: f32,
        font_size: f32,
        text_color: Color,
        border_color: Color,
        min_width: f32,
        asymmetric: bool,
    ) -> f32 {
        let padding_x = 6.0;
        let border_w = 2.0;
        let text_w = self.measure_calibrated_bracket_tag(inner_text, font_size, asymmetric);
        let box_w = (text_w + 2.0 * (padding_x + border_w)).max(min_width);
        let box_h = 32.0;

        stroke_rect(pixmap, x, y, box_w, box_h, border_color, border_w);
        let text_x = x + (box_w - text_w) / 2.0;
        let text_h = self.text_height(font_size);
        let text_y = y + (box_h - text_h) / 2.0;
        self.draw_calibrated_bracket_tag(pixmap, inner_text, text_x, text_y, font_size, text_color, asymmetric);

        box_w
    }
}
