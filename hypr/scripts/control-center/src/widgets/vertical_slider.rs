use std::path::PathBuf;
use crate::grid;
use crate::render;
use super::text::FontCache;

pub struct VerticalSlider {
    pub rect: (i32, i32, i32, i32),
    value: f64,
    is_dragging: bool,
    is_locked: bool,
    icon_path: Option<PathBuf>,
    icon_pixmap: Option<tiny_skia::Pixmap>,
    icon_generator: Option<Box<dyn Fn(f64) -> Option<PathBuf>>>,
    on_change: Option<Box<dyn Fn(f64)>>,
    last_cb_time: Option<std::time::Instant>,
}

impl VerticalSlider {
    pub fn new(initial_value: f64) -> Self {
        Self {
            rect: (0, 0, 0, 0),
            value: initial_value.clamp(0.0, 1.0),
            is_dragging: false,
            is_locked: false,
            icon_path: None,
            icon_pixmap: None,
            icon_generator: None,
            on_change: None,
            last_cb_time: None,
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

    pub fn value(&self) -> f64 {
        self.value
    }

    pub fn set_value(&mut self, value: f64) {
        self.value = value.clamp(0.0, 1.0);
        self.update_dynamic_icon();
    }

    pub fn set_icon_generator<F: Fn(f64) -> Option<PathBuf> + 'static>(&mut self, f: F) {
        self.icon_generator = Some(Box::new(f));
        self.update_dynamic_icon();
    }

    pub fn connect_value_changed<F: Fn(f64) + 'static>(&mut self, f: F) {
        self.on_change = Some(Box::new(f));
    }

    pub fn is_dragging(&self) -> bool {
        self.is_dragging
    }

    pub fn locked(&self) -> bool {
        self.is_locked
    }

    pub fn set_locked(&mut self, locked: bool) {
        self.is_locked = locked;
    }

    pub fn contains(&self, px: f64, py: f64) -> bool {
        let (x, y, w, h) = self.rect;
        px >= x as f64 && px < (x + w) as f64 && py >= y as f64 && py < (y + h) as f64
    }

    /// Start drag – update value from absolute pointer position.
    pub fn on_pointer_press(&mut self, abs_x: f64, abs_y: f64) -> bool {
        if self.is_locked {
            return false;
        }
        self.is_dragging = true;
        self.update_value_from_pos(abs_x, abs_y);
        true
    }

    /// Continue drag – update value from absolute pointer position.
    pub fn on_pointer_motion(&mut self, abs_x: f64, abs_y: f64) -> bool {
        if !self.is_dragging || self.is_locked {
            return false;
        }
        self.update_value_from_pos(abs_x, abs_y);
        true
    }

    pub fn on_pointer_release(&mut self) -> bool {
        if self.is_dragging {
            self.is_dragging = false;
            if let Some(ref cb) = self.on_change {
                cb(self.value);
            }
            return true;
        }
        false
    }

    pub fn draw(&self, pixmap: &mut tiny_skia::PixmapMut, _fonts: &FontCache) {
        let (rx, ry, rw, rh) = self.rect;
        let (x, y, w, h) = (rx as f32, ry as f32, rw as f32, rh as f32);
        if w <= 0.0 || h <= 0.0 {
            return;
        }

        let is_horizontal = w > h;

        // Background
        let bg = tiny_skia::Color::from_rgba(0.77, 0.48, 0.06, 0.05).unwrap();
        render::fill_rounded_rect(pixmap, x, y, w, h, grid::WIDGET_RADIUS, bg);

        // Fill
        let val = self.value as f32;
        let fill_color = tiny_skia::Color::from_rgba(0.77, 0.48, 0.06, 0.12).unwrap();
        if is_horizontal {
            let fw = w * val;
            if fw > 0.0 {
                render::fill_rounded_rect(pixmap, x, y, fw, h, grid::WIDGET_RADIUS, fill_color);
            }
        } else {
            let fh = h * val;
            if fh > 0.0 {
                render::fill_rounded_rect(pixmap, x, y + h - fh, w, fh, grid::WIDGET_RADIUS, fill_color);
            }
        }

        // Icon
        if let Some(ref icon) = self.icon_pixmap {
            let cell = w.min(h);
            let icon_size = cell * 0.5;
            let (cell_x, cell_y) = if is_horizontal {
                (x, y)
            } else {
                (x, y + h - cell)
            };
            let ix = cell_x + (cell - icon_size) / 2.0;
            let iy = cell_y + (cell - icon_size) / 2.0;
            render::draw_pixmap_scaled(pixmap, icon.as_ref(), ix, iy, icon_size, icon_size);
        }
    }

    // ── Private helpers ─────────────────────────────────────────────────────

    fn update_value_from_pos(&mut self, abs_x: f64, abs_y: f64) {
        let (rx, ry, rw, rh) = self.rect;
        let w = rw as f64;
        let h = rh as f64;
        if w <= 0.0 || h <= 0.0 {
            return;
        }

        let local_x = abs_x - rx as f64;
        let local_y = abs_y - ry as f64;

        let raw = if w > h {
            local_x / w
        } else {
            1.0 - (local_y / h)
        };

        self.set_value(raw);

        let now = std::time::Instant::now();
        let should_call = match self.last_cb_time {
            Some(t) => now.duration_since(t).as_millis() > 50,
            None => true,
        };

        if should_call {
            self.last_cb_time = Some(now);
            if let Some(ref cb) = self.on_change {
                cb(self.value);
            }
        }
    }

    fn update_dynamic_icon(&mut self) {
        let maybe_path = if let Some(ref gen) = self.icon_generator {
            gen(self.value)
        } else {
            None
        };
        if self.icon_path != maybe_path {
            self.icon_path = maybe_path;
            self.reload_icon();
        }
    }

    fn reload_icon(&mut self) {
        let Some(ref path) = self.icon_path else {
            self.icon_pixmap = None;
            return;
        };
        self.icon_pixmap = super::svg_utils::load_svg_colored(path, "#3A2408", 128);
    }
}
