use std::path::PathBuf;
use crate::grid;
use crate::render;
use super::text::FontCache;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PressZone {
    CircleZone,
    TextZone,
}

pub struct QuickSettingButton {
    pub rect: (i32, i32, i32, i32),
    active: bool,
    label: String,
    status: String,
    icon_active_path: Option<PathBuf>,
    icon_inactive_path: Option<PathBuf>,
    icon_active_pixmap: Option<tiny_skia::Pixmap>,
    icon_inactive_pixmap: Option<tiny_skia::Pixmap>,
    pub is_hovered: bool,
    pub mouse_x: f64,
    pub mouse_y: f64,
    pub press_zone: Option<PressZone>,
    toggle_handler: Option<Box<dyn Fn(bool)>>,
    detail_handler: Option<Box<dyn Fn()>>,
}

impl QuickSettingButton {
    pub fn new() -> Self {
        Self {
            rect: (0, 0, 0, 0),
            active: false,
            label: String::new(),
            status: String::new(),
            icon_active_path: None,
            icon_inactive_path: None,
            icon_active_pixmap: None,
            icon_inactive_pixmap: None,
            is_hovered: false,
            mouse_x: 0.0,
            mouse_y: 0.0,
            press_zone: None,
            toggle_handler: None,
            detail_handler: None,
        }
    }

    pub fn set_rect(&mut self, x: i32, y: i32, w: i32, h: i32) {
        self.rect = (x, y, w, h);
    }

    pub fn set_label(&mut self, text: &str) {
        self.label = text.to_string();
    }

    pub fn set_status(&mut self, text: &str) {
        self.status = text.to_string();
    }

    pub fn active(&self) -> bool {
        self.active
    }

    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    pub fn set_icons<P: AsRef<std::path::Path>>(&mut self, active_path: Option<P>, inactive_path: Option<P>) {
        self.icon_active_path = active_path.map(|p| p.as_ref().to_path_buf());
        self.icon_inactive_path = inactive_path.map(|p| p.as_ref().to_path_buf());
        self.reload_icons();
    }

    pub fn connect_toggle<F: Fn(bool) + 'static>(&mut self, f: F) {
        self.toggle_handler = Some(Box::new(f));
    }

    pub fn connect_detail_clicked<F: Fn() + 'static>(&mut self, f: F) {
        self.detail_handler = Some(Box::new(f));
    }

    pub fn contains(&self, px: f64, py: f64) -> bool {
        let (x, y, w, h) = self.rect;
        px >= x as f64 && px < (x + w) as f64 && py >= y as f64 && py < (y + h) as f64
    }

    pub fn on_pointer_enter(&mut self, x: f64, y: f64) -> bool {
        self.is_hovered = true;
        self.mouse_x = x;
        self.mouse_y = y;
        true
    }

    pub fn on_pointer_leave(&mut self) -> bool {
        let changed = self.is_hovered || self.press_zone.is_some();
        self.is_hovered = false;
        self.press_zone = None;
        changed
    }

    pub fn on_pointer_motion(&mut self, x: f64, y: f64) -> bool {
        self.mouse_x = x;
        self.mouse_y = y;
        self.is_hovered = true;
        true
    }

    pub fn on_pointer_press(&mut self, x: f64, _y: f64) -> bool {
        let h = self.rect.3 as f64;
        let local_x = x - self.rect.0 as f64;
        if local_x < h {
            self.press_zone = Some(PressZone::CircleZone);
        } else {
            self.press_zone = Some(PressZone::TextZone);
        }
        true
    }

    pub fn on_pointer_release(&mut self) -> bool {
        let zone = self.press_zone;
        self.press_zone = None;

        match zone {
            Some(PressZone::CircleZone) => {
                let next = !self.active;
                self.set_active(next);
                if let Some(ref cb) = self.toggle_handler {
                    cb(next);
                }
            }
            Some(PressZone::TextZone) => {
                if let Some(ref cb) = self.detail_handler {
                    cb();
                }
            }
            None => {}
        }
        true
    }

    pub fn draw(&self, pixmap: &mut tiny_skia::PixmapMut, fonts: &FontCache) {
        let (rx, ry, rw, rh) = self.rect;
        let (x, y, w, h) = (rx as f32, ry as f32, rw as f32, rh as f32);
        if w <= 0.0 || h <= 0.0 {
            return;
        }

        // 1. Card background
        let is_text_hovered = self.is_hovered && (self.mouse_x - rx as f64) as f32 >= h;
        let card_bg = if is_text_hovered {
            if self.press_zone == Some(PressZone::TextZone) {
                tiny_skia::Color::from_rgba(0.77, 0.48, 0.06, 0.24).unwrap()
            } else {
                tiny_skia::Color::from_rgba(0.77, 0.48, 0.06, 0.16).unwrap()
            }
        } else {
            tiny_skia::Color::from_rgba(0.77, 0.48, 0.06, 0.08).unwrap()
        };
        render::fill_rounded_rect(pixmap, x, y, w, h, grid::WIDGET_RADIUS, card_bg);

        // 2. Circle toggle on the left (rounded square matching WIDGET_RADIUS)
        let circle_size = h;
        let is_circle_hovered = self.is_hovered && ((self.mouse_x - rx as f64) as f32) < h;
        let is_circle_pressed = self.press_zone == Some(PressZone::CircleZone);

        let circle_bg = if self.active {
            if is_circle_pressed {
                tiny_skia::Color::from_rgba(0.82, 0.60, 0.33, 1.0).unwrap()
            } else if is_circle_hovered {
                tiny_skia::Color::from_rgba(0.87, 0.67, 0.40, 1.0).unwrap()
            } else {
                tiny_skia::Color::from_rgba(0.92, 0.72, 0.46, 1.0).unwrap()
            }
        } else {
            if is_circle_pressed {
                tiny_skia::Color::from_rgba(0.77, 0.48, 0.06, 0.28).unwrap()
            } else if is_circle_hovered {
                tiny_skia::Color::from_rgba(0.77, 0.48, 0.06, 0.20).unwrap()
            } else {
                tiny_skia::Color::from_rgba(0.77, 0.48, 0.06, 0.12).unwrap()
            }
        };
        render::fill_rounded_rect(pixmap, x, y, circle_size, circle_size, grid::WIDGET_RADIUS, circle_bg);

        // SVG icon inside circle
        let icon_size = h / 2.0;
        let icon_x = x + (circle_size - icon_size) / 2.0;
        let icon_y = y + (circle_size - icon_size) / 2.0;
        let pixmap_to_draw = if self.active {
            self.icon_active_pixmap.as_ref()
        } else {
            self.icon_inactive_pixmap.as_ref()
        };
        if let Some(icon) = pixmap_to_draw {
            render::draw_pixmap_scaled(pixmap, icon.as_ref(), icon_x, icon_y, icon_size, icon_size);
        }

        // 3. Title text
        let text_x = x + h + 8.0;
        let title_y = y + h * 0.22;
        let title_color = tiny_skia::Color::from_rgba(0.227, 0.141, 0.031, 1.0).unwrap();
        fonts.draw_text(pixmap, &self.label, text_x, title_y, 13.0, true, title_color);

        // 4. Status text
        let status_y = y + h * 0.55;
        let status_color = tiny_skia::Color::from_rgba(0.353, 0.227, 0.078, 0.8).unwrap();
        fonts.draw_text(pixmap, &self.status, text_x, status_y, 11.0, false, status_color);
    }

    fn reload_icons(&mut self) {
        if let Some(ref path) = self.icon_active_path {
            self.icon_active_pixmap = super::svg_utils::load_svg_colored(path, "#3A2408", 64);
        }
        if let Some(ref path) = self.icon_inactive_path {
            self.icon_inactive_pixmap = super::svg_utils::load_svg_colored(path, "#7A5020", 64);
        }
    }
}

impl Default for QuickSettingButton {
    fn default() -> Self {
        Self::new()
    }
}
