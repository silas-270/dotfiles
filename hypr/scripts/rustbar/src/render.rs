//! rustbar-specific drawing on top of `shell_common`.
//!
//! The bracket-tag and GTK-box renderers below match Waybar's CSS metrics and
//! are used only by the bar; control-center draws a different tag shape. They
//! extend the shared `FontCache` through the `BarText` trait, which must be in
//! scope at the call site.

use shell_common::font::FontCache;
use shell_common::paint::stroke_rect;
use tiny_skia::{Color, PixmapMut};


// ── Bracket Spacing Calibration ──────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BracketSpacing {
    Symmetric,
    Asymmetric,
    HalfAsymmetric,
}

/// Bar-only text and box drawing layered onto the shared `FontCache`.
pub trait BarText {
    fn measure_calibrated_bracket_tag(&mut self, text: &str, font_size: f32, asymmetric: bool) -> f32;
    fn measure_calibrated_bracket_tag_with_spacing(&mut self, text: &str, font_size: f32, spacing: BracketSpacing) -> f32;
    fn draw_calibrated_bracket_tag(
        &mut self,
        pixmap: &mut PixmapMut,
        text: &str,
        start_x: f32,
        top_y: f32,
        font_size: f32,
        color: Color,
        asymmetric: bool,
    ) -> f32;
    fn draw_calibrated_bracket_tag_with_spacing(
        &mut self,
        pixmap: &mut PixmapMut,
        text: &str,
        start_x: f32,
        top_y: f32,
        font_size: f32,
        color: Color,
        spacing: BracketSpacing,
    ) -> f32;
    fn measure_gtk_box(&mut self, content: &str, font_size: f32, min_width: f32, asymmetric: bool) -> f32;
    fn draw_module_box(
        &mut self,
        pixmap: &mut PixmapMut,
        inner_text: &str,
        x: f32,
        y: f32,
        font_size: f32,
        text_color: Color,
        border_color: Color,
        asymmetric: bool,
    ) -> f32;
    fn draw_gtk_box(
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
    ) -> f32;
    fn draw_gtk_box_with_spacing(
        &mut self,
        pixmap: &mut PixmapMut,
        inner_text: &str,
        x: f32,
        y: f32,
        font_size: f32,
        text_color: Color,
        border_color: Color,
        min_width: f32,
        spacing: BracketSpacing,
    ) -> f32;
}

impl BarText for FontCache {
    /// Measure a calibrated bracket tag to match exact Waybar width (e.g. [ icon ]).
    fn measure_calibrated_bracket_tag(&mut self, text: &str, font_size: f32, asymmetric: bool) -> f32 {
        let spacing = if asymmetric { BracketSpacing::Asymmetric } else { BracketSpacing::Symmetric };
        self.measure_calibrated_bracket_tag_with_spacing(text, font_size, spacing)
    }

    fn measure_calibrated_bracket_tag_with_spacing(&mut self, text: &str, font_size: f32, spacing: BracketSpacing) -> f32 {
        let bracket_w = self.measure_text("[", font_size);
        let space_w = 5.33; // Locked to 16px nominal U+2004 width (16.0 / 3.0)
        let text_w = self.measure_text(text, font_size);
        let right_space_w = match spacing {
            BracketSpacing::Symmetric => space_w,
            BracketSpacing::Asymmetric => self.measure_text(" ", font_size),
            BracketSpacing::HalfAsymmetric => (space_w + self.measure_text(" ", font_size)) / 2.0,
        };
        bracket_w * 2.0 + space_w + right_space_w + text_w
    }

    /// Draw a calibrated bracket tag [ icon ] with icon centered between brackets.
    fn draw_calibrated_bracket_tag(
        &mut self,
        pixmap: &mut PixmapMut,
        text: &str,
        start_x: f32,
        top_y: f32,
        font_size: f32,
        color: Color,
        asymmetric: bool,
    ) -> f32 {
        let spacing = if asymmetric { BracketSpacing::Asymmetric } else { BracketSpacing::Symmetric };
        self.draw_calibrated_bracket_tag_with_spacing(pixmap, text, start_x, top_y, font_size, color, spacing)
    }

    fn draw_calibrated_bracket_tag_with_spacing(
        &mut self,
        pixmap: &mut PixmapMut,
        text: &str,
        start_x: f32,
        top_y: f32,
        font_size: f32,
        color: Color,
        spacing: BracketSpacing,
    ) -> f32 {
        let target_w = self.measure_calibrated_bracket_tag_with_spacing(text, font_size, spacing);
        let bracket_w = self.measure_text("[", font_size);
        let text_w = self.measure_text(text, font_size);

        // 1. Draw "["
        self.draw_text(pixmap, "[", start_x, top_y, font_size, color);

        // 2. Draw inner text
        let text_x = match spacing {
            BracketSpacing::Asymmetric | BracketSpacing::HalfAsymmetric => start_x + bracket_w + 5.33,
            BracketSpacing::Symmetric => start_x + (target_w - text_w) / 2.0,
        };
        self.draw_text(pixmap, text, text_x, top_y, font_size, color);

        // 3. Draw "]" at exact end
        let right_bracket_x = start_x + target_w - bracket_w;
        self.draw_text(pixmap, "]", right_bracket_x, top_y, font_size, color);

        target_w
    }

    /// Measure GTK module box (padding: 6px, border: 2px solid)
    fn measure_gtk_box(&mut self, content: &str, font_size: f32, min_width: f32, asymmetric: bool) -> f32 {
        let spacing = if asymmetric { BracketSpacing::Asymmetric } else { BracketSpacing::Symmetric };
        let padding_x = 6.0;
        let border_w = 2.0;
        let text_w = self.measure_calibrated_bracket_tag_with_spacing(content, font_size, spacing);
        (text_w + 2.0 * (padding_x + border_w)).max(min_width)
    }

    fn draw_module_box(
        &mut self,
        pixmap: &mut PixmapMut,
        inner_text: &str,
        x: f32,
        y: f32,
        font_size: f32,
        text_color: Color,
        border_color: Color,
        asymmetric: bool,
    ) -> f32 {
        self.draw_gtk_box(pixmap, inner_text, x, y, font_size, text_color, border_color, 0.0, asymmetric)
    }

    /// Draws a GTK module box tile matching GTK Waybar style.css (2px border, 6px padding)
    fn draw_gtk_box(
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
        let spacing = if asymmetric {
            BracketSpacing::Asymmetric
        } else {
            BracketSpacing::Symmetric
        };
        self.draw_gtk_box_with_spacing(pixmap, inner_text, x, y, font_size, text_color, border_color, min_width, spacing)
    }

    fn draw_gtk_box_with_spacing(
        &mut self,
        pixmap: &mut PixmapMut,
        inner_text: &str,
        x: f32,
        y: f32,
        font_size: f32,
        text_color: Color,
        border_color: Color,
        min_width: f32,
        spacing: BracketSpacing,
    ) -> f32 {
        let padding_x = 6.0;
        let border_w = 2.0;
        let text_w = self.measure_calibrated_bracket_tag_with_spacing(inner_text, font_size, spacing);
        let box_w = (text_w + 2.0 * (padding_x + border_w)).max(min_width);
        let box_h = 32.0;

        stroke_rect(pixmap, x, y, box_w, box_h, border_color, border_w);
        let text_x = x + (box_w - text_w) / 2.0;
        let text_h = self.text_height(font_size);
        let text_y = y + (box_h - text_h) / 2.0;
        self.draw_calibrated_bracket_tag_with_spacing(pixmap, inner_text, text_x, text_y, font_size, text_color, spacing);

        box_w
    }
}
