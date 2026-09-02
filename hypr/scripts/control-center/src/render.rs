//! control-center-specific drawing on top of `shell_common`.
//!
//! The panel draws a paren-style tag `( icon )` with the icon nudged to a
//! visual sweet spot; the bar draws square bracket boxes instead. This extends
//! the shared `FontCache` through the `PanelText` trait, which must be in scope
//! at the call site.

use shell_common::font::FontCache;
use tiny_skia::{Color, PixmapMut};

/// Panel-only tag drawing layered onto the shared `FontCache`.
pub trait PanelText {
    fn measure_calibrated_bracket_tag(&mut self, icon: &str, font_size: f32) -> f32;
    fn draw_calibrated_bracket_tag(
        &mut self,
        pixmap: &mut PixmapMut,
        icon: &str,
        start_x: f32,
        top_y: f32,
        font_size: f32,
        color: Color,
    ) -> f32;
}

impl PanelText for FontCache {
    /// Measure a calibrated bracket tag to match exact standard bracket tag width (e.g. (  )).
    fn measure_calibrated_bracket_tag(&mut self, _icon: &str, font_size: f32) -> f32 {
        self.measure_text("(  )", font_size)
    }

    /// Draw a calibrated bracket tag ( icon ) with icon position at the visual sweet spot (75% towards center).
    fn draw_calibrated_bracket_tag(
        &mut self,
        pixmap: &mut PixmapMut,
        icon: &str,
        start_x: f32,
        top_y: f32,
        font_size: f32,
        color: Color,
    ) -> f32 {
        let target_w = self.measure_calibrated_bracket_tag(icon, font_size);
        let bracket_w = self.measure_text("(", font_size);
        let icon_w = self.measure_text(icon, font_size);

        // 1. Draw "("
        self.draw_text(pixmap, "(", start_x, top_y, font_size, color);

        // 2. Draw Icon at the visual sweet spot (halfway between mid_offset and center_offset)
        let inner_w = target_w - 2.0 * bracket_w;
        let center_offset = (inner_w - icon_w) / 2.0;
        let mid_offset = (2.0 + center_offset) / 2.0;
        let sweet_spot_offset = (center_offset + mid_offset) / 2.0;
        let icon_x = start_x + bracket_w + sweet_spot_offset;
        self.draw_text(pixmap, icon, icon_x, top_y, font_size, color);

        // 3. Draw ")" at exact target_w end position to preserve vertical alignment
        let right_bracket_x = start_x + target_w - bracket_w;
        self.draw_text(pixmap, ")", right_bracket_x, top_y, font_size, color);

        target_w
    }
}
