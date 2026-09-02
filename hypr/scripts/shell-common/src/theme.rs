//! Shared theme loading for rustbar and control-center.
//!
//! Both surfaces read the same generated palette at
//! `~/dotfiles/theme/generated/colors.json`, so they share one struct, one
//! JSON mapping and one color parser. `waybar_border` is only consumed by
//! rustbar and `cc_border` only by control-center, but they live here so a
//! single `ThemeConfig` covers both.

use std::fs;
use std::path::PathBuf;
use tiny_skia::Color;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThemeConfig {
    pub bg_base: Color,
    pub border: Color,
    pub sec_border: Color,
    pub text_color: Color,
    pub fg_muted: Color,
    pub accent_color: Color,
    pub inner_border: Color,
    /// rustbar only.
    pub waybar_border: Color,
    /// control-center only.
    pub cc_border: Color,
    pub warning: Color,
    pub danger: Color,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        // Fallback Savanna Dusk warm dark palette
        Self {
            bg_base: Color::from_rgba(84.0 / 255.0, 56.0 / 255.0, 43.0 / 255.0, 1.0).unwrap(),
            border: Color::from_rgba(122.0 / 255.0, 82.0 / 255.0, 61.0 / 255.0, 1.0).unwrap(),
            sec_border: Color::from_rgba(122.0 / 255.0, 82.0 / 255.0, 61.0 / 255.0, 0.6).unwrap(),
            text_color: Color::from_rgba(242.0 / 255.0, 227.0 / 255.0, 213.0 / 255.0, 1.0).unwrap(),
            fg_muted: Color::from_rgba(194.0 / 255.0, 170.0 / 255.0, 149.0 / 255.0, 1.0).unwrap(),
            accent_color: Color::from_rgba(255.0 / 255.0, 140.0 / 255.0, 0.0, 1.0).unwrap(),
            inner_border: Color::from_rgba(103.0 / 255.0, 69.0 / 255.0, 52.0 / 255.0, 0.75).unwrap(),
            waybar_border: Color::TRANSPARENT,
            cc_border: Color::TRANSPARENT,
            warning: Color::from_rgba(255.0 / 255.0, 152.0 / 255.0, 0.0, 1.0).unwrap(),
            danger: Color::from_rgba(231.0 / 255.0, 76.0 / 255.0, 60.0 / 255.0, 1.0).unwrap(),
        }
    }
}

impl ThemeConfig {
    /// Loads dynamic theme colors from `~/dotfiles/theme/generated/colors.json`,
    /// falling back to the default palette if the file is missing or invalid.
    pub fn load() -> Self {
        let mut theme = Self::default();
        let colors_path = get_colors_path();
        let content = match fs::read_to_string(&colors_path) {
            Ok(c) => c,
            Err(_) => return theme,
        };

        theme.update_from_json(&content);
        theme
    }

    /// Parses JSON content and updates theme tokens where available.
    pub fn update_from_json(&mut self, json_str: &str) {
        let json_val: Value = match serde_json::from_str(json_str) {
            Ok(v) => v,
            Err(_) => return,
        };

        let colors_map = if let Some(colors_obj) = json_val.get("colors").and_then(|v| v.as_object()) {
            colors_obj
        } else if let Some(flat_obj) = json_val.as_object() {
            flat_obj
        } else {
            return;
        };

        let get = |keys: &[&str]| -> Option<Color> {
            keys.iter()
                .find_map(|k| colors_map.get(*k).and_then(|v| v.as_str()).and_then(parse_color))
        };

        if let Some(val) = get(&["bg_base"]) {
            self.bg_base = val;
        }
        if let Some(val) = get(&["border"]) {
            self.border = val;
            // Default sec_border to the same stroke; overridden below if the
            // palette defines a distinct subtle border.
            self.sec_border = val;
        }
        if let Some(val) = get(&["border_subtle", "sec_border"]) {
            self.sec_border = val;
        }
        if let Some(val) = get(&["fg_primary", "text_color"]) {
            self.text_color = val;
        }
        if let Some(val) = get(&["fg_muted"]) {
            self.fg_muted = val;
        }
        if let Some(val) = get(&["accent", "accent_color"]) {
            self.accent_color = val;
        }
        if let Some(val) = get(&["inner_border"]) {
            self.inner_border = val;
        }
        if let Some(val) = get(&["waybar_border"]) {
            self.waybar_border = val;
        }
        if let Some(val) = get(&["cc_border"]) {
            self.cc_border = val;
        }
        if let Some(val) = get(&["warning"]) {
            self.warning = val;
        }
        if let Some(val) = get(&["danger"]) {
            self.danger = val;
        }
    }
}

pub fn get_colors_path() -> PathBuf {
    crate::paths::colors_json()
}

/// Parses a color string into a `tiny_skia::Color`.
///
/// Accepts every form the generated palette emits: `transparent`, `#RRGGBB`,
/// `#RRGGBBAA`, `rgb(r, g, b)`, `rgba(r, g, b, a)` and `0xAARRGGBB`.
pub fn parse_color(s: &str) -> Option<Color> {
    let s = s.trim();
    if s == "transparent" {
        return Some(Color::TRANSPARENT);
    }
    if let Some(hex) = s.strip_prefix('#') {
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f32 / 255.0;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f32 / 255.0;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f32 / 255.0;
            Color::from_rgba(r, g, b, 1.0)
        } else if hex.len() == 8 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f32 / 255.0;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f32 / 255.0;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f32 / 255.0;
            let a = u8::from_str_radix(&hex[6..8], 16).ok()? as f32 / 255.0;
            Color::from_rgba(r, g, b, a)
        } else {
            None
        }
    } else if let Some(inner) = s.strip_prefix("rgba(").and_then(|r| r.strip_suffix(')')) {
        let parts: Vec<&str> = inner.split(',').map(|p| p.trim()).collect();
        if parts.len() == 4 {
            let r = parts[0].parse::<f32>().ok()? / 255.0;
            let g = parts[1].parse::<f32>().ok()? / 255.0;
            let b = parts[2].parse::<f32>().ok()? / 255.0;
            let a = parts[3].parse::<f32>().ok()?;
            Color::from_rgba(r, g, b, a)
        } else {
            None
        }
    } else if let Some(inner) = s.strip_prefix("rgb(").and_then(|r| r.strip_suffix(')')) {
        let parts: Vec<&str> = inner.split(',').map(|p| p.trim()).collect();
        if parts.len() == 3 {
            let r = parts[0].parse::<f32>().ok()? / 255.0;
            let g = parts[1].parse::<f32>().ok()? / 255.0;
            let b = parts[2].parse::<f32>().ok()? / 255.0;
            Color::from_rgba(r, g, b, 1.0)
        } else {
            None
        }
    } else if s.starts_with("0x") || s.starts_with("0X") {
        let hex = &s[2..];
        if hex.len() == 8 {
            let val = u32::from_str_radix(hex, 16).ok()?;
            let a = ((val >> 24) & 0xff) as f32 / 255.0;
            let r = ((val >> 16) & 0xff) as f32 / 255.0;
            let g = ((val >> 8) & 0xff) as f32 / 255.0;
            let b = (val & 0xff) as f32 / 255.0;
            Color::from_rgba(r, g, b, a)
        } else {
            None
        }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex() {
        let c = parse_color("#54382B").unwrap();
        assert!((c.red() - 84.0 / 255.0).abs() < 0.01);
        assert!((c.green() - 56.0 / 255.0).abs() < 0.01);
        assert!((c.blue() - 43.0 / 255.0).abs() < 0.01);
        assert_eq!(c.alpha(), 1.0);
    }

    #[test]
    fn test_parse_rgba() {
        let c = parse_color("rgba(217, 119, 54, 0.15)").unwrap();
        assert!((c.red() - 217.0 / 255.0).abs() < 0.01);
        assert!((c.green() - 119.0 / 255.0).abs() < 0.01);
        assert!((c.blue() - 54.0 / 255.0).abs() < 0.01);
        assert!((c.alpha() - 0.15).abs() < 0.01);
    }

    #[test]
    fn test_fallback_defaults() {
        let mut theme = ThemeConfig::default();
        theme.update_from_json("invalid json");
        assert_eq!(theme, ThemeConfig::default());
    }

    #[test]
    fn test_parse_transparent() {
        assert_eq!(parse_color("transparent").unwrap(), Color::TRANSPARENT);
    }

    #[test]
    fn test_parse_rgb() {
        let c = parse_color("rgb(255, 140, 0)").unwrap();
        assert!((c.red() - 1.0).abs() < 0.01);
        assert!((c.green() - 140.0 / 255.0).abs() < 0.01);
        assert_eq!(c.blue(), 0.0);
        assert_eq!(c.alpha(), 1.0);
    }

    #[test]
    fn test_parse_0x_argb() {
        let c = parse_color("0xffff8c00").unwrap();
        assert_eq!(c.alpha(), 1.0);
        assert!((c.red() - 1.0).abs() < 0.01);
        assert!((c.green() - 140.0 / 255.0).abs() < 0.01);
        assert_eq!(c.blue(), 0.0);
    }

    #[test]
    fn test_hex_alpha() {
        let c = parse_color("#7A523D80").unwrap();
        assert!((c.alpha() - 0.5).abs() < 0.01);
    }

    /// The real generated palette must populate every token, including the two
    /// that each surface previously failed to read (`waybar_border` was
    /// unparseable in control-center, `inner_border` was never read by rustbar).
    #[test]
    fn test_generated_palette_shape() {
        let json = r##"{
          "colors": {
            "bg_base": "#54382B", "fg_primary": "#F2E3D5", "fg_muted": "#C2AA95",
            "accent": "#FF8C00", "border": "#7A523D", "border_subtle": "#7A523D",
            "inner_border": "rgba(103, 69, 52, 0.75)",
            "waybar_border": "transparent", "cc_border": "transparent",
            "warning": "#FF9800", "danger": "#E74C3C"
          }
        }"##;
        let mut theme = ThemeConfig::default();
        theme.update_from_json(json);

        assert_eq!(theme.text_color, parse_color("#F2E3D5").unwrap());
        assert_eq!(theme.fg_muted, parse_color("#C2AA95").unwrap());
        assert_eq!(theme.waybar_border, Color::TRANSPARENT);
        assert_eq!(theme.cc_border, Color::TRANSPARENT);
        assert_eq!(theme.inner_border, parse_color("rgba(103, 69, 52, 0.75)").unwrap());
        assert_eq!(theme.accent_color, parse_color("#FF8C00").unwrap());
    }
}
