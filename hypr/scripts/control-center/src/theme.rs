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
    pub accent_color: Color,
    pub inner_border: Color,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        // Fallback Savanna Dusk warm dark palette
        Self {
            bg_base: Color::from_rgba(84.0 / 255.0, 56.0 / 255.0, 43.0 / 255.0, 1.0).unwrap(),
            border: Color::from_rgba(122.0 / 255.0, 82.0 / 255.0, 61.0 / 255.0, 1.0).unwrap(),
            sec_border: Color::from_rgba(122.0 / 255.0, 82.0 / 255.0, 61.0 / 255.0, 0.6).unwrap(),
            text_color: Color::from_rgba(194.0 / 255.0, 170.0 / 255.0, 149.0 / 255.0, 1.0).unwrap(),
            accent_color: Color::from_rgba(217.0 / 255.0, 119.0 / 255.0, 54.0 / 255.0, 1.0).unwrap(),
            inner_border: Color::from_rgba(103.0 / 255.0, 69.0 / 255.0, 52.0 / 255.0, 0.75).unwrap(),
        }
    }
}

impl ThemeConfig {
    /// Loads dynamic theme colors from ~/dotfiles/theme/generated/colors.json.
    /// Safely falls back to default Savanna Dusk colors if missing or invalid.
    pub fn load() -> Self {
        let mut theme = Self::default();

        let colors_path = get_colors_path();
        let content = match fs::read_to_string(&colors_path) {
            Ok(c) => c,
            Err(_) => {
                eprintln!("[CC Theme] Could not read colors file at {:?}, using default Savanna Dusk colors", colors_path);
                return theme;
            }
        };

        theme.update_from_json(&content);
        eprintln!("[CC Theme] Loaded theme colors from {:?}", colors_path);
        theme
    }

    /// Parses JSON content string and updates theme tokens where available.
    pub fn update_from_json(&mut self, json_str: &str) {
        let json_val: Value = match serde_json::from_str(json_str) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[CC Theme] Failed to parse JSON: {}", e);
                return;
            }
        };

        let colors_map = if let Some(colors_obj) = json_val.get("colors").and_then(|v| v.as_object()) {
            colors_obj
        } else if let Some(flat_obj) = json_val.as_object() {
            flat_obj
        } else {
            eprintln!("[CC Theme] Invalid JSON color map structure");
            return;
        };

        if let Some(val) = colors_map.get("bg_base").and_then(|v| v.as_str()).and_then(parse_color) {
            self.bg_base = val;
        }
        if let Some(val) = colors_map.get("border").and_then(|v| v.as_str()).and_then(parse_color) {
            self.border = val;
        }
        if let Some(val) = colors_map.get("border_subtle").or_else(|| colors_map.get("sec_border")).and_then(|v| v.as_str()).and_then(parse_color) {
            self.sec_border = val;
        } else if let Some(val) = colors_map.get("border").and_then(|v| v.as_str()).and_then(parse_color) {
            if let Some(col) = Color::from_rgba(val.red(), val.green(), val.blue(), 0.6) {
                self.sec_border = col;
            }
        }
        if let Some(val) = colors_map.get("fg_muted").or_else(|| colors_map.get("text_color")).and_then(|v| v.as_str()).and_then(parse_color) {
            self.text_color = val;
        }
        if let Some(val) = colors_map.get("accent").or_else(|| colors_map.get("accent_color")).and_then(|v| v.as_str()).and_then(parse_color) {
            self.accent_color = val;
        }
        if let Some(val) = colors_map.get("inner_border").and_then(|v| v.as_str()).and_then(parse_color) {
            self.inner_border = val;
        }
    }
}

fn get_colors_path() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        let path = PathBuf::from(home).join("dotfiles/theme/generated/colors.json");
        if path.exists() {
            return path;
        }
    }
    PathBuf::from("/home/silas270/dotfiles/theme/generated/colors.json")
}

/// Parses a color string (Hex or RGBA) into tiny_skia::Color.
pub fn parse_color(s: &str) -> Option<Color> {
    let s = s.trim();
    if s.starts_with('#') {
        let hex = &s[1..];
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
    } else if s.starts_with("rgba(") && s.ends_with(')') {
        let inner = &s[5..s.len() - 1];
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
    } else if s.starts_with("rgb(") && s.ends_with(')') {
        let inner = &s[4..s.len() - 1];
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
}
