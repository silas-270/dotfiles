use std::path::PathBuf;
use crate::grid;
use crate::render;
use super::text::FontCache;

pub struct IconButton {
    pub rect: (i32, i32, i32, i32),
    active: bool,
    is_hovered: bool,
    is_pressed: bool,
    icon_path: Option<PathBuf>,
    icon_pixmap: Option<tiny_skia::Pixmap>,
    icon_color_inactive: String,
    icon_color_active: String,
    click_handler: Option<Box<dyn Fn()>>,
}

impl IconButton {
    pub fn new() -> Self {
        Self {
            rect: (0, 0, 0, 0),
            active: false,
            is_hovered: false,
            is_pressed: false,
            icon_path: None,
            icon_pixmap: None,
            icon_color_inactive: "#7A5020".to_string(),
            icon_color_active: "#C47A10".to_string(),
            click_handler: None,
        }
    }

    pub fn set_rect(&mut self, x: i32, y: i32, w: i32, h: i32) {
        self.rect = (x, y, w, h);
    }

    pub fn set_icon_from_file<P: AsRef<std::path::Path>>(&mut self, path: Option<P>) {
        let new_path = path.map(|p| p.as_ref().to_path_buf());
        if self.icon_path != new_path {
            self.icon_path = new_path;
            self.reload_icon();
        }
    }

    pub fn set_icon_colors(&mut self, inactive: &str, active: &str) {
        self.icon_color_inactive = inactive.to_string();
        self.icon_color_active = active.to_string();
        self.reload_icon();
    }

    pub fn active(&self) -> bool {
        self.active
    }

    pub fn set_active(&mut self, active: bool) {
        if self.active != active {
            self.active = active;
            self.reload_icon();
        }
    }

    pub fn connect_clicked<F: Fn() + 'static>(&mut self, f: F) {
        self.click_handler = Some(Box::new(f));
    }

    pub fn contains(&self, px: f64, py: f64) -> bool {
        let (x, y, w, h) = self.rect;
        px >= x as f64 && px < (x + w) as f64 && py >= y as f64 && py < (y + h) as f64
    }

    pub fn on_pointer_enter(&mut self) -> bool {
        if !self.is_hovered {
            self.is_hovered = true;
            return true;
        }
        false
    }

    pub fn on_pointer_leave(&mut self) -> bool {
        let changed = self.is_hovered || self.is_pressed;
        self.is_hovered = false;
        self.is_pressed = false;
        changed
    }

    pub fn on_pointer_press(&mut self) -> bool {
        self.is_pressed = true;
        true
    }

    pub fn on_pointer_release(&mut self) -> bool {
        self.is_pressed = false;
        let next = !self.active;
        self.set_active(next);
        if let Some(ref cb) = self.click_handler {
            cb();
        }
        true
    }

    pub fn draw(&self, pixmap: &mut tiny_skia::PixmapMut, _fonts: &FontCache) {
        let (x, y, w, h) = self.rect;
        let (x, y, w, h) = (x as f32, y as f32, w as f32, h as f32);
        if w <= 0.0 || h <= 0.0 {
            return;
        }

        // Uniform translucent card background matching all grid widgets
        let bg_color = if self.is_pressed {
            tiny_skia::Color::from_rgba(0.77, 0.48, 0.06, 0.24).unwrap()
        } else if self.is_hovered {
            tiny_skia::Color::from_rgba(0.77, 0.48, 0.06, 0.16).unwrap()
        } else {
            tiny_skia::Color::from_rgba(0.77, 0.48, 0.06, 0.08).unwrap()
        };
        render::fill_rounded_rect(pixmap, x, y, w, h, grid::WIDGET_RADIUS, bg_color);

        // Draw icon centered at 1/2 of the cell size
        if let Some(ref icon) = self.icon_pixmap {
            let cell = w.min(h);
            let icon_size = cell * 0.5;
            let ix = x + (w - icon_size) / 2.0;
            let iy = y + (h - icon_size) / 2.0;
            render::draw_pixmap_scaled(pixmap, icon.as_ref(), ix, iy, icon_size, icon_size);
        }
    }

    fn reload_icon(&mut self) {
        let Some(ref path) = self.icon_path else {
            self.icon_pixmap = None;
            return;
        };
        let color = if self.active {
            &self.icon_color_active
        } else {
            &self.icon_color_inactive
        };
        self.icon_pixmap = super::svg_utils::load_svg_colored(path, color, 128);
    }
}

impl Default for IconButton {
    fn default() -> Self {
        Self::new()
    }
}
