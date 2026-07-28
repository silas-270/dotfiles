use std::fs::File;
use std::path::PathBuf;
use fontdue::{Font, FontSettings};
use memmap2::Mmap;

/// Pre-loaded fonts for text rendering.
pub struct FontCache {
    _bold_mmap: Option<Mmap>,
    _regular_mmap: Option<Mmap>,
    bold_font: Option<Font>,
    regular_font: Option<Font>,
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

        let (bold_mmap, bold_font) = bold_path
            .and_then(|p| {
                let file = File::open(&p).ok()?;
                let mmap = unsafe { Mmap::map(&file).ok()? };
                let font = Font::from_bytes(&mmap[..], FontSettings::default()).ok()?;
                Some((mmap, font))
            })
            .unzip();

        let (regular_mmap, regular_font) = regular_path
            .and_then(|p| {
                let file = File::open(&p).ok()?;
                let mmap = unsafe { Mmap::map(&file).ok()? };
                let font = Font::from_bytes(&mmap[..], FontSettings::default()).ok()?;
                Some((mmap, font))
            })
            .unzip();

        if bold_font.is_none() && regular_font.is_none() {
            eprintln!("[CC] Warning: No suitable monospace font found for text rendering!");
        }

        Self {
            _bold_mmap: bold_mmap,
            _regular_mmap: regular_mmap,
            bold_font,
            regular_font,
        }
    }

    fn get_font(&self, bold: bool) -> Option<&Font> {
        if bold {
            self.bold_font.as_ref().or(self.regular_font.as_ref())
        } else {
            self.regular_font.as_ref().or(self.bold_font.as_ref())
        }
    }

    /// Measure the horizontal advance width of `text` at `size` px.
    pub fn measure_text(&self, text: &str, size: f32, bold: bool) -> f32 {
        let Some(font) = self.get_font(bold) else {
            return 0.0;
        };
        text.chars()
            .map(|ch| {
                let (metrics, _) = font.rasterize(ch, size);
                metrics.advance_width
            })
            .sum()
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

        let line_metrics = font.horizontal_line_metrics(size);
        let ascent = line_metrics.map(|m| m.ascent).unwrap_or(size * 0.8);
        let baseline_y = y + ascent;

        let cr = color.red();
        let cg = color.green();
        let cb = color.blue();
        let ca = color.alpha();

        let pw = pixmap.width() as i32;
        let ph = pixmap.height() as i32;

        let mut cursor_x = x;

        for ch in text.chars() {
            let (metrics, bitmap) = font.rasterize(ch, size);

            if metrics.width > 0 && metrics.height > 0 {
                let gx = cursor_x as i32 + metrics.xmin;
                let gy = baseline_y as i32 - metrics.height as i32 - metrics.ymin;

                for row in 0..metrics.height {
                    let py = gy + row as i32;
                    if py < 0 || py >= ph {
                        continue;
                    }
                    for col in 0..metrics.width {
                        let px = gx + col as i32;
                        if px < 0 || px >= pw {
                            continue;
                        }

                        let coverage = bitmap[row * metrics.width + col];
                        if coverage == 0 {
                            continue;
                        }

                        let src_a = ca * (coverage as f32 / 255.0);
                        if src_a <= 0.0 {
                            continue;
                        }

                        let idx = ((py as usize) * (pw as usize) + (px as usize)) * 4;
                        let data = pixmap.data_mut();
                        if idx + 3 >= data.len() {
                            continue;
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
            }

            cursor_x += metrics.advance_width;
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
