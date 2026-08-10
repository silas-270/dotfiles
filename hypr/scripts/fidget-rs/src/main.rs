use std::{
    fs,
    path::PathBuf,
};

use fontdue::{Font, FontSettings};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_layer, delegate_output, delegate_pointer, delegate_registry,
    delegate_seat, delegate_shm,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        pointer::{PointerEvent, PointerEventKind, PointerHandler},
        Capability, SeatHandler, SeatState,
    },
    shell::{
        wlr_layer::{
            Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
            LayerSurfaceConfigure,
        },
        WaylandSurface,
    },
    shm::{
        slot::{SlotPool},
        Shm, ShmHandler,
    },
};
use tiny_skia::{Color, Paint, PathBuilder, Pixmap, Rect, Stroke, Transform};
use wayland_client::{
    globals::registry_queue_init,
    protocol::{wl_output, wl_pointer, wl_seat, wl_surface},
    Connection, QueueHandle,
};

#[derive(Debug, Clone, Copy)]
struct ThemeColors {
    fill: Color,
    border: Color,
    badge_bg: Color,
    text_muted: Color,
}

impl Default for ThemeColors {
    fn default() -> Self {
        Self {
            fill: Color::from_rgba8(217, 119, 54, 64),
            border: Color::from_rgba8(84, 56, 43, 242),
            badge_bg: Color::from_rgba8(84, 56, 43, 217),
            text_muted: Color::from_rgba8(194, 170, 149, 204),
        }
    }
}

fn parse_hex_color(content: &str, key: &str) -> Option<(u8, u8, u8)> {
    for line in content.lines() {
        if line.contains(key) && line.contains('#') {
            if let Some(idx) = line.find('#') {
                let hex = &line[idx + 1..];
                if hex.len() >= 6 {
                    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                    return Some((r, g, b));
                }
            }
        }
    }
    None
}

fn load_theme_colors() -> ThemeColors {
    let mut colors = ThemeColors::default();
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/silas270".to_string());
    let path = PathBuf::from(home).join("dotfiles/theme/generated/colors.lua");

    if let Ok(content) = fs::read_to_string(path) {
        if let Some(c) = parse_hex_color(&content, "accent") {
            colors.fill = Color::from_rgba8(c.0, c.1, c.2, 64);
        }
        if let Some(c) = parse_hex_color(&content, "bg_base") {
            colors.border = Color::from_rgba8(c.0, c.1, c.2, 242);
            colors.badge_bg = Color::from_rgba8(c.0, c.1, c.2, 217);
        }
        if let Some(c) = parse_hex_color(&content, "fg_muted") {
            colors.text_muted = Color::from_rgba8(c.0, c.1, c.2, 204);
        }
    }

    colors
}

fn load_font() -> Option<Font> {
    let candidates = [
        "/usr/share/fonts/Adwaita/AdwaitaMono-Regular.ttf",
        "/usr/share/fonts/OTF/AtkynsonMonoNerdFont-Regular.otf",
        "/usr/share/fonts/gnu-free/FreeMono.otf",
    ];
    for path in candidates {
        if let Ok(bytes) = fs::read(path) {
            if let Ok(font) = Font::from_bytes(bytes, FontSettings::default()) {
                return Some(font);
            }
        }
    }
    None
}

fn draw_badge(
    pixmap: &mut Pixmap,
    text: &str,
    font: Option<&Font>,
    width: u32,
    height: u32,
    target_x: f32,
    target_y: f32,
    theme: &ThemeColors,
) {
    let font = match font {
        Some(f) => f,
        None => return,
    };

    let font_size = 11.0;
    let padding_x = 7.0;
    let padding_y = 3.0;

    let mut char_bitmaps = Vec::new();
    let mut total_width: f32 = 0.0;
    let mut max_height: f32 = 0.0;

    for c in text.chars() {
        let (metrics, bitmap) = font.rasterize(c, font_size);
        total_width += metrics.advance_width;
        max_height = max_height.max(metrics.height as f32);
        char_bitmaps.push((metrics, bitmap));
    }

    let bw = total_width + padding_x * 2.0;
    let bh = max_height.max(12.0) + padding_y * 2.0;

    let screen_w = width as f32;
    let screen_h = height as f32;

    let bx = (target_x + 6.0).min(screen_w - bw - 6.0).max(6.0);
    let by = (target_y + 6.0).min(screen_h - bh - 6.0).max(6.0);

    if let Some(badge_rect) = Rect::from_xywh(bx, by, bw, bh) {
        let mut bg_paint = Paint::default();
        bg_paint.set_color(theme.badge_bg);
        pixmap.fill_rect(badge_rect, &bg_paint, Transform::identity(), None);

        let mut border_paint = Paint::default();
        border_paint.set_color(theme.fill);
        let mut stroke = Stroke::default();
        stroke.width = 1.0;

        let path = PathBuilder::from_rect(badge_rect);
        pixmap.stroke_path(&path, &border_paint, &stroke, Transform::identity(), None);

        let mut cur_x = bx + padding_x;
        let text_color = theme.text_muted;

        for (metrics, bitmap) in char_bitmaps {
            let glyph_x = cur_x + metrics.bounds.xmin;
            let glyph_y = by + padding_y + 2.0;

            for gy in 0..metrics.height {
                for gx in 0..metrics.width {
                    let alpha_byte = bitmap[gy * metrics.width + gx];
                    if alpha_byte > 0 {
                        let px = (glyph_x + gx as f32) as u32;
                        let py = (glyph_y + gy as f32) as u32;

                        if px < pixmap.width() && py < pixmap.height() {
                            let mut color = text_color;
                            color.set_alpha(color.alpha() * (alpha_byte as f32 / 255.0));
                            pixmap.fill_rect(
                                Rect::from_xywh(px as f32, py as f32, 1.0, 1.0).unwrap(),
                                &Paint {
                                    shader: tiny_skia::Shader::SolidColor(color),
                                    ..Default::default()
                                },
                                Transform::identity(),
                                None,
                            );
                        }
                    }
                }
            }
            cur_x += metrics.advance_width;
        }
    }
}

struct AppState {
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,
    _compositor_state: CompositorState,
    shm: Shm,

    layer_surface: Option<LayerSurface>,
    pool: SlotPool,

    width: u32,
    height: u32,
    first_configure: bool,

    is_dragging: bool,
    start_pos: (f64, f64),
    current_pos: (f64, f64),

    dirty: bool,
    waiting_for_frame: bool,

    theme: ThemeColors,
    font: Option<Font>,
}

impl AppState {
    fn request_redraw(&mut self, qh: &QueueHandle<Self>) {
        self.dirty = true;
        if !self.waiting_for_frame {
            if let Some(layer_surface) = &self.layer_surface {
                let surface = layer_surface.wl_surface();
                surface.frame(qh, surface.clone());
                self.waiting_for_frame = true;
                surface.commit();
            }
        }
    }

    fn draw(&mut self, _qh: &QueueHandle<Self>) {
        self.dirty = false;
        let (width, height) = (self.width, self.height);
        if width == 0 || height == 0 {
            return;
        }

        let stride = (width * 4) as i32;
        let (buffer, canvas) = match self.pool.create_buffer(
            width as i32,
            height as i32,
            stride,
            wayland_client::protocol::wl_shm::Format::Argb8888,
        ) {
            Ok(res) => res,
            Err(e) => {
                eprintln!("Failed to create buffer: {:?}", e);
                return;
            }
        };

        // Render with tiny-skia
        let mut pixmap = match Pixmap::new(width, height) {
            Some(p) => p,
            None => return,
        };
        pixmap.fill(Color::TRANSPARENT);

        if self.is_dragging {
            let x1 = self.start_pos.0.min(self.current_pos.0);
            let y1 = self.start_pos.1.min(self.current_pos.1);
            let x2 = self.start_pos.0.max(self.current_pos.0);
            let y2 = self.start_pos.1.max(self.current_pos.1);

            let w = (x2 - x1) as f32;
            let h = (y2 - y1) as f32;

            if w > 1.0 && h > 1.0 {
                let rect = Rect::from_xywh(x1 as f32, y1 as f32, w, h);

                if let Some(r) = rect {
                    // Fill rectangle
                    let mut fill_paint = Paint::default();
                    fill_paint.set_color(self.theme.fill);
                    pixmap.fill_rect(r, &fill_paint, Transform::identity(), None);

                    // Border line (2px solid)
                    let mut border_paint = Paint::default();
                    border_paint.set_color(self.theme.border);
                    let mut stroke = Stroke::default();
                    stroke.width = 2.0;

                    let path = PathBuilder::from_rect(r);
                    pixmap.stroke_path(&path, &border_paint, &stroke, Transform::identity(), None);

                    // Dimension Badge
                    if w > 10.0 && h > 10.0 {
                        let text = format!("{} × {} px", w as u32, h as u32);
                        draw_badge(
                            &mut pixmap,
                            &text,
                            self.font.as_ref(),
                            width,
                            height,
                            x1 as f32 + w,
                            y1 as f32 + h,
                            &self.theme,
                        );
                    }
                }
            }
        }

        // Copy tiny-skia pixmap (RGBA) to Wayland ARGB8888 canvas buffer
        let data = pixmap.data();
        for (i, chunk) in data.chunks_exact(4).enumerate() {
            let r = chunk[0];
            let g = chunk[1];
            let b = chunk[2];
            let a = chunk[3];

            // ARGB8888 in little-endian is B G R A
            let offset = i * 4;
            canvas[offset] = b;
            canvas[offset + 1] = g;
            canvas[offset + 2] = r;
            canvas[offset + 3] = a;
        }

        if let Some(layer_surface) = &self.layer_surface {
            let surface = layer_surface.wl_surface();
            surface.damage_buffer(0, 0, width as i32, height as i32);
            buffer.attach_to(surface).unwrap();
            surface.commit();
        }
    }
}

impl CompositorHandler for AppState {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_scale: i32,
    ) {
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
        self.waiting_for_frame = false;
        if self.dirty {
            self.draw(qh);
        }
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }
}

impl OutputHandler for AppState {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }
}

impl LayerShellHandler for AppState {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {
        std::process::exit(0);
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        let (width, height) = configure.new_size;
        self.width = width;
        self.height = height;

        if self.first_configure {
            self.first_configure = false;
            self.draw(qh);
        }
    }
}

impl SeatHandler for AppState {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Pointer {
            self.seat_state.get_pointer(qh, &seat).unwrap();
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _seat: wl_seat::WlSeat,
        _capability: Capability,
    ) {
    }

    fn remove_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: wl_seat::WlSeat) {}
}

impl PointerHandler for AppState {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        for event in events {
            match event.kind {
                PointerEventKind::Press { button: 272, .. } => {
                    // Left click press (BTN_LEFT = 272 / 0x110)
                    self.is_dragging = true;
                    self.start_pos = event.position;
                    self.current_pos = event.position;
                    self.request_redraw(qh);
                }
                PointerEventKind::Release { button: 272, .. } => {
                    if self.is_dragging {
                        self.is_dragging = false;
                        self.request_redraw(qh);
                    }
                }
                PointerEventKind::Motion { .. } => {
                    if self.is_dragging {
                        self.current_pos = event.position;
                        self.request_redraw(qh);
                    }
                }
                _ => {}
            }
        }
    }
}

impl ShmHandler for AppState {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl ProvidesRegistryState for AppState {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    registry_handlers![OutputState, SeatState];
}

delegate_compositor!(AppState);
delegate_output!(AppState);
delegate_shm!(AppState);
delegate_layer!(AppState);
delegate_seat!(AppState);
delegate_pointer!(AppState);
delegate_registry!(AppState);

fn main() {
    let conn = Connection::connect_to_env().expect("Failed to connect to Wayland display");
    let (globals, mut event_queue) = registry_queue_init(&conn).expect("Failed to initialize registry");
    let qh = event_queue.handle();

    let compositor_state = CompositorState::bind(&globals, &qh).expect("Failed to bind compositor");
    let layer_shell = LayerShell::bind(&globals, &qh).expect("Failed to bind layer shell");
    let shm = Shm::bind(&globals, &qh).expect("Failed to bind shm");
    let seat_state = SeatState::new(&globals, &qh);
    let output_state = OutputState::new(&globals, &qh);

    let surface = compositor_state.create_surface(&qh);
    let layer_surface = layer_shell.create_layer_surface(
        &qh,
        surface,
        Layer::Background,
        Some("desktop-fidget"),
        None,
    );

    layer_surface.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
    layer_surface.set_exclusive_zone(-1);
    layer_surface.set_keyboard_interactivity(KeyboardInteractivity::None);
    layer_surface.commit();

    let pool = SlotPool::new(1920 * 1080 * 4, &shm).expect("Failed to create SHM slot pool");

    let mut app = AppState {
        registry_state: RegistryState::new(&globals),
        seat_state,
        output_state,
        _compositor_state: compositor_state,
        shm,
        layer_surface: Some(layer_surface),
        pool,
        width: 0,
        height: 0,
        first_configure: true,
        is_dragging: false,
        start_pos: (0.0, 0.0),
        current_pos: (0.0, 0.0),
        dirty: false,
        waiting_for_frame: false,
        theme: load_theme_colors(),
        font: load_font(),
    };

    loop {
        event_queue.blocking_dispatch(&mut app).unwrap();
    }
}
