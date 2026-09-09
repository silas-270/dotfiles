//! Shared glyph-caching text renderer.
//!
//! Both fonts ship in `shell-common/fonts/` and are embedded at build time, so
//! neither binary depends on a font installed outside the repo.

use tiny_skia::*;
use rusttype::{Font, Scale};
use std::collections::HashMap;

/// Which embedded JetBrainsMono Nerd Font weight a `FontCache` renders with.
///
/// rustbar uses `Regular` to match Waybar's `font-weight: 400`;
/// control-center uses `Medium`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontWeight {
    Regular,
    Medium,
}

pub struct FontCache {
    font: Font<'static>,
    // Cached: (width, height, min_x, min_y, alpha_data)
    // min_x/min_y are the glyph offsets from a (0,0)-positioned glyph
    glyph_cache: HashMap<(char, u32), (u32, u32, i32, i32, Vec<u8>)>,
}

impl FontCache {
    pub fn new(weight: FontWeight) -> Self {
        let font_data: &[u8] = match weight {
            FontWeight::Regular => include_bytes!("../fonts/JetBrainsMonoNerdFont-Regular.ttf"),
            FontWeight::Medium => include_bytes!("../fonts/JetBrainsMonoNerdFont-Medium.ttf"),
        };
        let font = Font::try_from_bytes(font_data)
            .expect("Failed to load embedded JetBrainsMono Nerd Font");

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
        self.draw_text_clipped(pixmap, text, start_x, top_y, font_size, color, f32::NEG_INFINITY, f32::INFINITY)
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

    /// Ascent (positive, above baseline) and descent (positive, below
    /// baseline) — needed to align non-text content (e.g. a formula image)
    /// to the same baseline `draw_text`/`draw_text_clipped` use internally
    /// (`top_y + ascent`), rather than to the full line box.
    pub fn ascent_descent(&mut self, font_size: f32) -> (f32, f32) {
        let scale = Scale::uniform(font_size);
        let v = self.font.v_metrics(scale);
        (v.ascent, -v.descent)
    }
}
