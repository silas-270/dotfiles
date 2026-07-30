use std::fs::File;
use std::path::PathBuf;
use memmap2::Mmap;
use rusttype::{Font, Scale, point};

/// Pre-loaded fonts for text rendering.
pub struct FontCache {
    bold_mmap: Option<Mmap>,
    regular_mmap: Option<Mmap>,
}

impl FontCache {
    /// Discover and load system fonts using memory-mapping for low RAM usage.
    pub fn new() -> Self {
        let bold_path = find_font("JetBrainsMono Nerd Font:style=Bold")
            .or_else(|| find_font("JetBrains Mono:style=Bold"))
            .or_else(|| find_font("monospace:style=Bold"));

        let regular_path = find_font("JetBrainsMono Nerd Font:style=Regular")
            .or_else(|| find_font("JetBrains Mono:style=Regular"))
            .or_else(|| find_font("monospace"));

        let bold_mmap = bold_path.and_then(|p| {
            let file = File::open(&p).ok()?;
            unsafe { Mmap::map(&file).ok() }
        });

        let regular_mmap = regular_path.and_then(|p| {
            let file = File::open(&p).ok()?;
            unsafe { Mmap::map(&file).ok() }
        });

        if bold_mmap.is_none() && regular_mmap.is_none() {
            eprintln!("[CC] Warning: No suitable monospace font found for text rendering!");
        }

        Self {
            bold_mmap,
            regular_mmap,
        }
    }

    fn get_font(&self, bold: bool) -> Option<Font<'_>> {
        let mmap = if bold {
            self.bold_mmap.as_ref().or(self.regular_mmap.as_ref())
        } else {
            self.regular_mmap.as_ref().or(self.bold_mmap.as_ref())
        }?;
        Font::try_from_bytes(mmap)
    }

    /// Measure the horizontal advance width of `text` at `size` px.
    pub fn measure_text(&self, text: &str, size: f32, bold: bool) -> f32 {
        let Some(font) = self.get_font(bold) else {
            return 0.0;
        };
        let scale = Scale::uniform(size);
        let v_metrics = font.v_metrics(scale);
        
        let glyphs: Vec<_> = font.layout(text, scale, point(0.0, v_metrics.ascent)).collect();
        if let Some(last) = glyphs.last() {
            last.position().x + last.unpositioned().h_metrics().advance_width
        } else {
            0.0
        }
    }

    /// Render `text` into `pixmap` at `(x, y)` (top-left of the em box).
    ///
    /// `size` is in pixels.  `bold` selects the bold variant.  `color` uses
    /// premultiplied-alpha compositing to blend onto existing content.
    pub fn draw_text(
        &self,
        pixmap: &mut tiny_skia::PixmapMut,
        text: &str,
        x: f32,
        y: f32,
        size: f32,
        bold: bool,
        color: tiny_skia::Color,
    ) {
        let Some(font) = self.get_font(bold) else {
            return;
        };
        let scale = Scale::uniform(size);
        let v_metrics = font.v_metrics(scale);

        let cr = color.red();
        let cg = color.green();
        let cb = color.blue();
        let ca = color.alpha();

        let pw = pixmap.width() as i32;
        let ph = pixmap.height() as i32;

        for glyph in font.layout(text, scale, point(x, y + v_metrics.ascent)) {
            if let Some(bounding_box) = glyph.pixel_bounding_box() {
                glyph.draw(|gx, gy, v| {
                    let px = bounding_box.min.x + gx as i32;
                    let py = bounding_box.min.y + gy as i32;
                    if px >= 0 && px < pw && py >= 0 && py < ph {
                        let src_a = ca * v;
                        if src_a > 0.0 {
                            let idx = ((py as usize) * (pw as usize) + (px as usize)) * 4;
                            let data = pixmap.data_mut();
                            if idx + 3 >= data.len() {
                                return;
                            }
                            
                            let inv = 1.0 - src_a;
                            let dst_r = data[idx] as f32 / 255.0;
                            let dst_g = data[idx + 1] as f32 / 255.0;
                            let dst_b = data[idx + 2] as f32 / 255.0;
                            let dst_a = data[idx + 3] as f32 / 255.0;

                            data[idx] = ((cr * src_a + dst_r * inv) * 255.0).min(255.0) as u8;
                            data[idx + 1] = ((cg * src_a + dst_g * inv) * 255.0).min(255.0) as u8;
                            data[idx + 2] = ((cb * src_a + dst_b * inv) * 255.0).min(255.0) as u8;
                            data[idx + 3] = ((src_a + dst_a * inv) * 255.0).min(255.0) as u8;
                        }
                    }
                });
            }
        }
    }
}

impl Default for FontCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Ask fontconfig for the file path of the best-matching font.
fn find_font(name: &str) -> Option<PathBuf> {
    let output = std::process::Command::new("fc-match")
        .args(["-f", "%{file}", name])
        .output()
        .ok()?;
    let path = String::from_utf8(output.stdout).ok()?;
    let path = path.trim();
    if path.is_empty() {
        None
    } else {
        Some(PathBuf::from(path))
    }
}
