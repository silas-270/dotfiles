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
    pub waybar_border: Color,
    pub warning: Color,
    pub danger: Color,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        // Fallback savanna-dusk palette
        Self {
            bg_base: Color::from_rgba(84.0 / 255.0, 56.0 / 255.0, 43.0 / 255.0, 1.0).unwrap(),
            border: Color::from_rgba(122.0 / 255.0, 82.0 / 255.0, 61.0 / 255.0, 1.0).unwrap(),
            sec_border: Color::from_rgba(122.0 / 255.0, 82.0 / 255.0, 61.0 / 255.0, 0.6).unwrap(),
            text_color: Color::from_rgba(194.0 / 255.0, 170.0 / 255.0, 149.0 / 255.0, 1.0).unwrap(),
            accent_color: Color::from_rgba(217.0 / 255.0, 119.0 / 255.0, 54.0 / 255.0, 1.0).unwrap(),
            inner_border: Color::from_rgba(103.0 / 255.0, 69.0 / 255.0, 52.0 / 255.0, 0.75).unwrap(),
            waybar_border: Color::from_rgba(217.0 / 255.0, 119.0 / 255.0, 54.0 / 255.0, 1.0).unwrap(),
            warning: Color::from_rgba(255.0 / 255.0, 176.0 / 255.0, 0.0 / 255.0, 1.0).unwrap(),
            danger: Color::from_rgba(255.0 / 255.0, 51.0 / 255.0, 51.0 / 255.0, 1.0).unwrap(),
        }
    }
}

impl ThemeConfig {
    /// Loads dynamic theme colors from ~/dotfiles/theme/generated/colors.json.
    pub fn load() -> Self {
        let mut theme = Self::default();
        let colors_path = get_colors_path();
        let content = match fs::read_to_string(&colors_path) {
            Ok(c) => c,
            Err(_) => {
                eprintln!("[RustBar Theme] Could not read colors file at {:?}, using fallback theme", colors_path);
                return theme;
            }
        };

        theme.update_from_json(&content);
        eprintln!("[RustBar Theme] Loaded theme colors from {:?}", colors_path);
        theme
    }

    /// Parses JSON content string and updates theme tokens.
    pub fn update_from_json(&mut self, json_str: &str) {
        let json_val: Value = match serde_json::from_str(json_str) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[RustBar Theme] Failed to parse JSON: {}", e);
                return;
            }
        };

        let colors_map = if let Some(colors_obj) = json_val.get("colors").and_then(|v| v.as_object()) {
            colors_obj
        } else if let Some(flat_obj) = json_val.as_object() {
            flat_obj
        } else {
            eprintln!("[RustBar Theme] Invalid JSON color map structure");
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
        }
        if let Some(val) = colors_map.get("fg_muted").or_else(|| colors_map.get("text_color")).and_then(|v| v.as_str()).and_then(parse_color) {
            self.text_color = val;
        }
        if let Some(val) = colors_map.get("accent").or_else(|| colors_map.get("accent_color")).and_then(|v| v.as_str()).and_then(parse_color) {
            self.accent_color = val;
        }
        if let Some(val) = colors_map.get("waybar_border").or_else(|| colors_map.get("border")).and_then(|v| v.as_str()).and_then(parse_color) {
            self.waybar_border = val;
        }
        if let Some(val) = colors_map.get("warning").and_then(|v| v.as_str()).and_then(parse_color) {
            self.warning = val;
        }
        if let Some(val) = colors_map.get("danger").and_then(|v| v.as_str()).and_then(parse_color) {
            self.danger = val;
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
    } else {
        None
    }
}
