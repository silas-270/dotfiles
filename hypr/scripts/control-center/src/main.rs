use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_keyboard, delegate_layer, delegate_output,
    delegate_pointer, delegate_registry, delegate_seat, delegate_shm,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        keyboard::{KeyEvent, KeyboardHandler, Modifiers},
        pointer::{PointerEvent, PointerEventKind, PointerHandler},
        Capability, SeatHandler, SeatState,
    },
    shell::{
        wlr_layer::{
            Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler,
            LayerSurface, LayerSurfaceConfigure,
        },
        WaylandSurface,
    },
    shm::{
        slot::SlotPool,
        Shm, ShmHandler,
    },
};
use wayland_client::{
    globals::registry_queue_init,
    protocol::{wl_keyboard, wl_output, wl_pointer, wl_seat, wl_shm, wl_surface},
    Connection, QueueHandle,
};

use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::time::Duration;

mod api;
mod grid;
mod render;
mod widgets;

use widgets::{FontCache, IconButton, MediaPlayer, QuickSettingButton, VerticalSlider};

/// Socket path unique per user so multiple sessions don't collide.
fn socket_path() -> std::path::PathBuf {
    let uid = unsafe { libc::getuid() };
    std::path::PathBuf::from(format!("/tmp/control-center-{}.sock", uid))
}

/// Try to send "toggle" to an already-running daemon.
fn try_toggle_existing() -> bool {
    let path = socket_path();
    if let Ok(mut stream) = UnixStream::connect(&path) {
        let _ = stream.write_all(b"toggle");
        let _ = stream.flush();
        let mut buf = [0u8; 2];
        let _ = stream.read(&mut buf);
        true
    } else {
        false
    }
}

// ---------------------------------------------------------------------------
// Sync messages from background threads
// ---------------------------------------------------------------------------

enum SyncMessage {
    WifiBluetooth {
        wifi: Option<(bool, String)>,
        bt: Option<bool>,
    },
    Brightness {
        brightness: f64,
        blue_active: bool,
    },
    Volume {
        volume: f64,
        muted: bool,
    },
    Media(api::media::MediaState),
}

// ---------------------------------------------------------------------------
// Slider drag tracking
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum DraggingSlider {
    Brightness,
    Volume,
}

// ---------------------------------------------------------------------------
// Main application state
// ---------------------------------------------------------------------------

struct ControlCenter {
    // SCTK protocol state
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,
    compositor_state: CompositorState,
    layer_shell: LayerShell,
    shm: Shm,

    // Buffer pool
    pool: SlotPool,
    backdrop_pool: SlotPool,

    // Surfaces (Some when visible, None when hidden)
    panel_surface: Option<LayerSurface>,
    backdrop_surface: Option<LayerSurface>,
    panel_width: u32,
    panel_height: u32,
    backdrop_width: u32,
    backdrop_height: u32,
    panel_configured: bool,
    backdrop_configured: bool,

    // Visibility
    visible: bool,
    needs_draw: bool,

    // Widgets
    wifi_btn: QuickSettingButton,
    bluetooth_btn: QuickSettingButton,
    brightness_btn: IconButton,
    brightness_slider: VerticalSlider,
    volume_btn: IconButton,
    volume_slider: VerticalSlider,
    media_player: MediaPlayer,

    // Sync state
    sync_sender: calloop::channel::Sender<SyncMessage>,
    wifi_toggle_time: Option<std::time::Instant>,
    bt_toggle_time: Option<std::time::Instant>,
    last_volume: f64,

    // Active drag
    dragging: Option<DraggingSlider>,

    // Font cache
    font_cache: FontCache,

    // For creating surfaces later
    qh: QueueHandle<Self>,
}

impl ControlCenter {
    // ── Show / Hide ─────────────────────────────────────────────────────

    fn show_panel(&mut self) {
        if self.visible {
            return;
        }
        eprintln!("[CC] Showing panel");
        self.visible = true;

        // Synchronously fetch media state to determine window height
        let media_state = api::media::get_media_state();
        self.media_player.update_from_state(&media_state);

        let panel_h = if self.media_player.visible {
            grid::GRID_HEIGHT_WITH_MEDIA
        } else {
            grid::GRID_HEIGHT
        };
        self.panel_height = panel_h as u32;
        self.panel_width = grid::GRID_WIDTH as u32;

        // Create backdrop (fullscreen click-catcher)
        let backdrop_wl = self.compositor_state.create_surface(&self.qh);
        let backdrop_layer = self.layer_shell.create_layer_surface(
            &self.qh,
            backdrop_wl,
            Layer::Overlay,
            Some("control-center-backdrop"),
            None,
        );
        backdrop_layer.set_anchor(Anchor::TOP | Anchor::RIGHT | Anchor::BOTTOM | Anchor::LEFT);
        backdrop_layer.set_exclusive_zone(-1);
        backdrop_layer.set_keyboard_interactivity(KeyboardInteractivity::None);
        backdrop_layer.commit();
        self.backdrop_surface = Some(backdrop_layer);
        self.backdrop_configured = false;

        // Create panel
        let panel_wl = self.compositor_state.create_surface(&self.qh);
        let panel_layer = self.layer_shell.create_layer_surface(
            &self.qh,
            panel_wl,
            Layer::Overlay,
            Some("control-center"),
            None,
        );
        panel_layer.set_anchor(Anchor::TOP | Anchor::RIGHT);
        panel_layer.set_size(self.panel_width, self.panel_height);
        panel_layer.set_margin(58, 10, 0, 0);
        panel_layer.set_exclusive_zone(-1);
        panel_layer.set_keyboard_interactivity(KeyboardInteractivity::OnDemand);
        panel_layer.commit();
        self.panel_surface = Some(panel_layer);
        self.panel_configured = false;

        // Trigger immediate background sync
        self.immediate_sync();
    }

    fn hide_panel(&mut self) {
        if !self.visible {
            return;
        }
        eprintln!("[CC] Hiding panel");
        self.visible = false;
        self.dragging = None;
        self.panel_surface = None;
        self.backdrop_surface = None;
        self.panel_configured = false;
        self.backdrop_configured = false;

        // Trim heap
        unsafe {
            extern "C" {
                fn malloc_trim(pad: usize) -> i32;
            }
            malloc_trim(0);
        }
    }

    fn toggle_panel(&mut self) {
        if self.visible {
            self.hide_panel();
        } else {
            self.show_panel();
        }
    }

    // ── Drawing ─────────────────────────────────────────────────────────

    fn draw_panel(&mut self) {
        let w = self.panel_width;
        let h = self.panel_height;
        if w == 0 || h == 0 {
            return;
        }

        let stride = w as i32 * 4;
        let (buffer, canvas) = match self.pool.create_buffer(
            w as i32,
            h as i32,
            stride,
            wl_shm::Format::Argb8888,
        ) {
            Ok(pair) => pair,
            Err(e) => {
                eprintln!("[CC] Failed to create panel buffer: {e}");
                return;
            }
        };

        // Render to tiny-skia pixmap
        let mut pixmap = match tiny_skia::Pixmap::new(w, h) {
            Some(p) => p,
            None => return,
        };
        pixmap.fill(tiny_skia::Color::TRANSPARENT);

        // Window background with rounded corners
        let win_bg = tiny_skia::Color::from_rgba(0.961, 0.910, 0.800, 0.85).unwrap();
        render::fill_rounded_rect(
            &mut pixmap.as_mut(), 0.0, 0.0, w as f32, h as f32,
            grid::WINDOW_RADIUS, win_bg,
        );

        // Draw widgets
        self.wifi_btn.draw(&mut pixmap.as_mut(), &self.font_cache);
        self.bluetooth_btn.draw(&mut pixmap.as_mut(), &self.font_cache);
        self.brightness_btn.draw(&mut pixmap.as_mut(), &self.font_cache);
        self.brightness_slider.draw(&mut pixmap.as_mut(), &self.font_cache);
        self.volume_btn.draw(&mut pixmap.as_mut(), &self.font_cache);
        self.volume_slider.draw(&mut pixmap.as_mut(), &self.font_cache);
        if self.media_player.visible {
            self.media_player.draw(&mut pixmap.as_mut(), &self.font_cache);
        }

        // Copy RGBA → BGRA
        render::rgba_to_bgra(pixmap.data(), canvas);

        // Submit
        if let Some(ref surface) = self.panel_surface {
            let wl = surface.wl_surface();
            wl.attach(Some(buffer.wl_buffer()), 0, 0);
            wl.damage_buffer(0, 0, w as i32, h as i32);
            wl.commit();
        }
        self.needs_draw = false;
    }

    fn draw_backdrop(&mut self) {
        let w = self.backdrop_width;
        let h = self.backdrop_height;
        if w == 0 || h == 0 {
            return;
        }

        let stride = w as i32 * 4;
        let (buffer, canvas) = match self.backdrop_pool.create_buffer(
            w as i32,
            h as i32,
            stride,
            wl_shm::Format::Argb8888,
        ) {
            Ok(pair) => pair,
            Err(e) => {
                eprintln!("[CC] Failed to create backdrop buffer: {e}");
                return;
            }
        };

        // Fully transparent background to catch clicks without visual artifacts
        canvas.fill(0);

        if let Some(ref surface) = self.backdrop_surface {
            let wl = surface.wl_surface();
            wl.attach(Some(buffer.wl_buffer()), 0, 0);
            wl.damage_buffer(0, 0, w as i32, h as i32);
            wl.commit();
        }
    }

    fn request_redraw(&mut self) {
        self.needs_draw = true;
    }

    // ── Pointer event dispatch ──────────────────────────────────────────

    fn handle_pointer_motion(&mut self, x: f64, y: f64) {
        let mut redraw = false;

        // If dragging a slider, forward to it regardless of hit test
        if let Some(slider_id) = self.dragging {
            match slider_id {
                DraggingSlider::Brightness => {
                    redraw |= self.brightness_slider.on_pointer_motion(x, y);
                }
                DraggingSlider::Volume => {
                    redraw |= self.volume_slider.on_pointer_motion(x, y);
                }
            }
            if redraw { self.request_redraw(); }
            return;
        }

        // Normal hit testing for hover state
        redraw |= if self.wifi_btn.contains(x, y) {
            self.wifi_btn.on_pointer_motion(x, y)
        } else {
            self.wifi_btn.on_pointer_leave()
        };
        redraw |= if self.bluetooth_btn.contains(x, y) {
            self.bluetooth_btn.on_pointer_motion(x, y)
        } else {
            self.bluetooth_btn.on_pointer_leave()
        };
        redraw |= if self.brightness_btn.contains(x, y) {
            self.brightness_btn.on_pointer_enter()
        } else {
            self.brightness_btn.on_pointer_leave()
        };
        redraw |= if self.volume_btn.contains(x, y) {
            self.volume_btn.on_pointer_enter()
        } else {
            self.volume_btn.on_pointer_leave()
        };
        // Sliders don't have hover state
        redraw |= if self.media_player.contains(x, y) {
            self.media_player.on_pointer_motion(x, y)
        } else {
            self.media_player.on_pointer_leave()
        };

        if redraw {
            self.request_redraw();
        }
    }

    fn handle_pointer_press(&mut self, x: f64, y: f64) {
        let mut redraw = false;

        if self.wifi_btn.contains(x, y) {
            redraw |= self.wifi_btn.on_pointer_press(x, y);
        } else if self.bluetooth_btn.contains(x, y) {
            redraw |= self.bluetooth_btn.on_pointer_press(x, y);
        } else if self.brightness_btn.contains(x, y) {
            redraw |= self.brightness_btn.on_pointer_press();
        } else if self.brightness_slider.contains(x, y) {
            redraw |= self.brightness_slider.on_pointer_press(x, y);
            self.dragging = Some(DraggingSlider::Brightness);
        } else if self.volume_btn.contains(x, y) {
            redraw |= self.volume_btn.on_pointer_press();
        } else if self.volume_slider.contains(x, y) {
            redraw |= self.volume_slider.on_pointer_press(x, y);
            self.dragging = Some(DraggingSlider::Volume);
        } else if self.media_player.contains(x, y) {
            redraw |= self.media_player.on_pointer_press(x, y);
        }

        if redraw {
            self.request_redraw();
        }
    }

    fn handle_pointer_release(&mut self, x: f64, y: f64) {
        let mut redraw = false;
        
        if let Some(slider_id) = self.dragging.take() {
            match slider_id {
                DraggingSlider::Brightness => redraw |= self.brightness_slider.on_pointer_release(),
                DraggingSlider::Volume => redraw |= self.volume_slider.on_pointer_release(),
            }
        }
        
        if self.wifi_btn.contains(x, y) {
            redraw |= self.wifi_btn.on_pointer_release();
            self.wifi_toggle_time = Some(std::time::Instant::now());
            self.wifi_btn.set_status(if self.wifi_btn.active() { "Enabling..." } else { "Disabling..." });
        } else if self.bluetooth_btn.contains(x, y) {
            redraw |= self.bluetooth_btn.on_pointer_release();
            self.bt_toggle_time = Some(std::time::Instant::now());
            self.bluetooth_btn.set_status(if self.bluetooth_btn.active() { "Enabling..." } else { "Disabling..." });
        } else if self.brightness_btn.contains(x, y) {
            redraw |= self.brightness_btn.on_pointer_release();
            api::compositor::set_blue_light_enabled(self.brightness_btn.active());
        } else if self.volume_btn.contains(x, y) {
            redraw |= self.volume_btn.on_pointer_release();
            if self.volume_btn.active() {
                self.volume_slider.set_locked(true);
                let current = self.volume_slider.value();
                if current > 0.0 { self.last_volume = current; }
                self.volume_slider.set_value(0.0);
                api::audio::set_mute(true);
            } else {
                self.volume_slider.set_locked(false);
                self.volume_slider.set_value(self.last_volume);
                api::audio::set_mute(false);
                api::audio::set_volume(self.last_volume);
            }
        } else if self.media_player.contains(x, y) {
            redraw |= self.media_player.on_pointer_release();
        }
        
        if redraw { self.request_redraw(); }
    }

    // ── Background sync ────────────────────────────────────────────────

    fn immediate_sync(&self) {
        let s = self.sync_sender.clone();
        std::thread::spawn(move || {
            let b = api::brightness::get_brightness();
            let bl = api::compositor::is_blue_light_active();
            let _ = s.send(SyncMessage::Brightness { brightness: b, blue_active: bl });
        });
        let s = self.sync_sender.clone();
        std::thread::spawn(move || {
            let wifi = Some((api::network::is_wifi_active(), api::network::get_wifi_status()));
            let bt = Some(api::bluetooth::is_bluetooth_active());
            let _ = s.send(SyncMessage::WifiBluetooth { wifi, bt });
        });
        let s = self.sync_sender.clone();
        std::thread::spawn(move || {
            let (vol, muted) = api::audio::get_volume();
            let _ = s.send(SyncMessage::Volume { volume: vol, muted });
        });
    }

    fn periodic_sync(&self) {
        if !self.visible { return; }
        let s = self.sync_sender.clone();
        std::thread::spawn(move || {
            let b = api::brightness::get_brightness();
            let bl = api::compositor::is_blue_light_active();
            let _ = s.send(SyncMessage::Brightness { brightness: b, blue_active: bl });
        });
        let s = self.sync_sender.clone();
        std::thread::spawn(move || {
            let (vol, muted) = api::audio::get_volume();
            let _ = s.send(SyncMessage::Volume { volume: vol, muted });
        });
        let s = self.sync_sender.clone();
        std::thread::spawn(move || {
            let state = api::media::get_media_state();
            let _ = s.send(SyncMessage::Media(state));
        });
    }

    fn periodic_wifi_bt_sync(&self) {
        if !self.visible { return; }
        let should_wifi = match self.wifi_toggle_time {
            Some(t) => t.elapsed().as_secs() >= 3,
            None => true,
        };
        let should_bt = match self.bt_toggle_time {
            Some(t) => t.elapsed().as_secs() >= 3,
            None => true,
        };
        let s = self.sync_sender.clone();
        std::thread::spawn(move || {
            let wifi = if should_wifi {
                Some((api::network::is_wifi_active(), api::network::get_wifi_status()))
            } else { None };
            let bt = if should_bt {
                Some(api::bluetooth::is_bluetooth_active())
            } else { None };
            let _ = s.send(SyncMessage::WifiBluetooth { wifi, bt });
        });
    }

    fn handle_sync_message(&mut self, msg: SyncMessage) {
        match msg {
            SyncMessage::WifiBluetooth { wifi, bt } => {
                if let Some((active, status)) = wifi {
                    if self.wifi_btn.active() != active {
                        self.wifi_btn.set_active(active);
                    }
                    self.wifi_btn.set_status(&status);
                }
                if let Some(active) = bt {
                    if self.bluetooth_btn.active() != active {
                        self.bluetooth_btn.set_active(active);
                    }
                    self.bluetooth_btn.set_status(if active { "On" } else { "Off" });
                }
            }
            SyncMessage::Brightness { brightness, blue_active } => {
                if !self.brightness_slider.is_dragging() {
                    if (self.brightness_slider.value() - brightness).abs() > 0.01 {
                        self.brightness_slider.set_value(brightness);
                    }
                }
                if self.brightness_btn.active() != blue_active {
                    self.brightness_btn.set_active(blue_active);
                }
            }
            SyncMessage::Volume { volume, muted } => {
                if self.volume_btn.active() != muted {
                    self.volume_btn.set_active(muted);
                }
                if !self.volume_slider.is_dragging() {
                    if muted {
                        self.volume_slider.set_locked(true);
                        if self.volume_slider.value() != 0.0 {
                            self.volume_slider.set_value(0.0);
                        }
                    } else {
                        self.volume_slider.set_locked(false);
                        if (self.volume_slider.value() - volume).abs() > 0.01 {
                            self.volume_slider.set_value(volume);
                        }
                        if volume > 0.0 {
                            self.last_volume = volume;
                        }
                    }
                }
            }
            SyncMessage::Media(state) => {
                let was_visible = self.media_player.visible;
                self.media_player.update_from_state(&state);
                let is_visible = self.media_player.visible;

                if was_visible != is_visible {
                    let target_h = if is_visible {
                        grid::GRID_HEIGHT_WITH_MEDIA
                    } else {
                        grid::GRID_HEIGHT
                    };
                    self.panel_height = target_h as u32;
                    // Recreate panel surface with new height
                    if self.visible {
                        if let Some(old) = self.panel_surface.take() {
                            drop(old);
                        }
                        let panel_wl = self.compositor_state.create_surface(&self.qh);
                        let panel_layer = self.layer_shell.create_layer_surface(
                            &self.qh, panel_wl, Layer::Overlay,
                            Some("control-center"), None,
                        );
                        panel_layer.set_anchor(Anchor::TOP | Anchor::RIGHT);
                        panel_layer.set_size(self.panel_width, self.panel_height);
                        panel_layer.set_margin(58, 10, 0, 0);
                        panel_layer.set_exclusive_zone(-1);
                        panel_layer.set_keyboard_interactivity(KeyboardInteractivity::OnDemand);
                        panel_layer.commit();
                        self.panel_surface = Some(panel_layer);
                        self.panel_configured = false;
                    }
                }
            }
        }
        self.request_redraw();
    }
}

// ---------------------------------------------------------------------------
// SCTK Handler Implementations
// ---------------------------------------------------------------------------

impl CompositorHandler for ControlCenter {
    fn scale_factor_changed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _surface: &wl_surface::WlSurface, _new_factor: i32) {}
    fn transform_changed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _surface: &wl_surface::WlSurface, _new_transform: wl_output::Transform) {}
    fn frame(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _surface: &wl_surface::WlSurface, _time: u32) {
        if self.needs_draw && self.panel_configured {
            self.draw_panel();
        }
    }
    fn surface_enter(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _surface: &wl_surface::WlSurface, _output: &wl_output::WlOutput) {}
    fn surface_leave(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _surface: &wl_surface::WlSurface, _output: &wl_output::WlOutput) {}
}

impl OutputHandler for ControlCenter {
    fn output_state(&mut self) -> &mut OutputState { &mut self.output_state }
    fn new_output(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: wl_output::WlOutput) {}
    fn update_output(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: wl_output::WlOutput) {}
    fn output_destroyed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: wl_output::WlOutput) {}
}

impl LayerShellHandler for ControlCenter {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {
        self.hide_panel();
    }

    fn configure(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, layer: &LayerSurface, configure: LayerSurfaceConfigure, _serial: u32) {
        let is_panel = self.panel_surface.as_ref()
            .map(|s| s.wl_surface() == layer.wl_surface())
            .unwrap_or(false);
        let is_backdrop = self.backdrop_surface.as_ref()
            .map(|s| s.wl_surface() == layer.wl_surface())
            .unwrap_or(false);

        if is_panel {
            // IGNORE compositor requested size to prevent stretching if Hyprland applies weird window rules
            self.panel_width = grid::GRID_WIDTH as u32;
            let panel_h = if self.media_player.visible { grid::GRID_HEIGHT_WITH_MEDIA } else { grid::GRID_HEIGHT };
            self.panel_height = panel_h as u32;
            self.panel_configured = true;
            self.draw_panel();
        } else if is_backdrop {
            let (w, h) = configure.new_size;
            // Use a massive fallback size (4K) to prevent 1x1 OpenGL texture scaling bugs
            // Because it's filled with 0s, Linux deduplicates it to the zero page (0 RAM usage)
            self.backdrop_width = if w == 0 { 4096 } else { w };
            self.backdrop_height = if h == 0 { 2160 } else { h };
            self.backdrop_configured = true;
            self.draw_backdrop();
        }
    }
}

impl SeatHandler for ControlCenter {
    fn seat_state(&mut self) -> &mut SeatState { &mut self.seat_state }
    fn new_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: wl_seat::WlSeat) {}
    fn new_capability(&mut self, _conn: &Connection, qh: &QueueHandle<Self>, seat: wl_seat::WlSeat, capability: Capability) {
        if capability == Capability::Pointer {
            let _ = self.seat_state.get_pointer(qh, &seat);
        }
        if capability == Capability::Keyboard {
            let _ = self.seat_state.get_keyboard(qh, &seat, None);
        }
    }
    fn remove_capability(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: wl_seat::WlSeat, _capability: Capability) {}
    fn remove_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: wl_seat::WlSeat) {}
}

impl PointerHandler for ControlCenter {
    fn pointer_frame(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _pointer: &wl_pointer::WlPointer, events: &[PointerEvent]) {
        for event in events {
            let is_panel = self.panel_surface.as_ref()
                .map(|s| *s.wl_surface() == event.surface)
                .unwrap_or(false);
            let is_backdrop = self.backdrop_surface.as_ref()
                .map(|s| *s.wl_surface() == event.surface)
                .unwrap_or(false);

            match event.kind {
                PointerEventKind::Enter { .. } | PointerEventKind::Motion { .. } => {
                    if is_panel {
                        self.handle_pointer_motion(event.position.0, event.position.1);
                    }
                }
                PointerEventKind::Press { button, .. } => {
                    if is_backdrop {
                        self.hide_panel();
                    } else if is_panel {
                        if button == 0x110 { // BTN_LEFT
                            self.handle_pointer_press(event.position.0, event.position.1);
                        }
                    }
                }
                PointerEventKind::Release { button, .. } => {
                    if is_panel && button == 0x110 {
                        self.handle_pointer_release(event.position.0, event.position.1);
                    }
                }
                PointerEventKind::Leave { .. } => {
                    if is_panel {
                        // Clear all hover states
                        let mut r = false;
                        r |= self.wifi_btn.on_pointer_leave();
                        r |= self.bluetooth_btn.on_pointer_leave();
                        r |= self.brightness_btn.on_pointer_leave();
                        r |= self.volume_btn.on_pointer_leave();
                        r |= self.media_player.on_pointer_leave();
                        if r { self.request_redraw(); }
                    }
                }
                _ => {}
            }
        }
    }
}

impl KeyboardHandler for ControlCenter {
    fn enter(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _keyboard: &wl_keyboard::WlKeyboard, _surface: &wl_surface::WlSurface, _serial: u32, _raw: &[u32], _keysyms: &[smithay_client_toolkit::seat::keyboard::Keysym]) {}
    fn leave(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _keyboard: &wl_keyboard::WlKeyboard, _surface: &wl_surface::WlSurface, _serial: u32) {}
    fn press_key(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _keyboard: &wl_keyboard::WlKeyboard, _serial: u32, event: KeyEvent) {
        // ESC key = 0xff1b
        if event.keysym.raw() == 0xff1b {
            self.hide_panel();
        }
    }
    fn release_key(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _keyboard: &wl_keyboard::WlKeyboard, _serial: u32, _event: KeyEvent) {}
    fn update_modifiers(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _keyboard: &wl_keyboard::WlKeyboard, _serial: u32, _modifiers: Modifiers, _layout: u32) {}
}

impl ShmHandler for ControlCenter {
    fn shm_state(&mut self) -> &mut Shm { &mut self.shm }
}

impl ProvidesRegistryState for ControlCenter {
    fn registry(&mut self) -> &mut RegistryState { &mut self.registry_state }
    registry_handlers![OutputState, SeatState,];
}

delegate_compositor!(ControlCenter);
delegate_output!(ControlCenter);
delegate_shm!(ControlCenter);
delegate_seat!(ControlCenter);
delegate_keyboard!(ControlCenter);
delegate_pointer!(ControlCenter);
delegate_layer!(ControlCenter);
delegate_registry!(ControlCenter);

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    if try_toggle_existing() {
        eprintln!("[CC] Sent toggle to running daemon");
        return;
    }
    eprintln!("[CC] No daemon found, starting up...");

    let conn = Connection::connect_to_env().expect("Failed to connect to Wayland display");
    let (globals, event_queue) = registry_queue_init(&conn).expect("Failed to init registry");
    let qh = event_queue.handle();

    // Initialize SCTK globals
    let compositor_state = CompositorState::bind(&globals, &qh).expect("wl_compositor");
    let layer_shell = LayerShell::bind(&globals, &qh).expect("zwlr_layer_shell_v1");
    let shm = Shm::bind(&globals, &qh).expect("wl_shm");
    let seat_state = SeatState::new(&globals, &qh);
    let output_state = OutputState::new(&globals, &qh);
    let pool = SlotPool::new(512 * 512 * 4, &shm).expect("SlotPool");

    // calloop event loop
    let mut event_loop: calloop::EventLoop<ControlCenter> =
        calloop::EventLoop::try_new().expect("calloop event loop");
    let loop_handle = event_loop.handle();

    // Wayland source
    smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource::new(
        conn.clone(), event_queue,
    )
    .insert(loop_handle.clone())
    .expect("Wayland calloop source");

    // IPC channel (toggle from external invocations)
    let (ipc_sender, ipc_recv) = calloop::channel::channel::<()>();
    loop_handle
        .insert_source(ipc_recv, |event, _, app: &mut ControlCenter| {
            if let calloop::channel::Event::Msg(()) = event {
                app.toggle_panel();
            }
        })
        .expect("IPC calloop source");

    // Sync channel (background thread results)
    let (sync_sender, sync_recv) = calloop::channel::channel::<SyncMessage>();
    loop_handle
        .insert_source(sync_recv, |event, _, app: &mut ControlCenter| {
            if let calloop::channel::Event::Msg(msg) = event {
                app.handle_sync_message(msg);
            }
        })
        .expect("Sync calloop source");

    // Periodic timers
    loop_handle
        .insert_source(
            calloop::timer::Timer::from_duration(Duration::from_secs(1)),
            |_, _, app: &mut ControlCenter| {
                app.periodic_sync();
                calloop::timer::TimeoutAction::ToDuration(Duration::from_secs(1))
            },
        )
        .expect("periodic sync timer");

    loop_handle
        .insert_source(
            calloop::timer::Timer::from_duration(Duration::from_secs(2)),
            |_, _, app: &mut ControlCenter| {
                app.periodic_wifi_bt_sync();
                calloop::timer::TimeoutAction::ToDuration(Duration::from_secs(2))
            },
        )
        .expect("wifi/bt sync timer");


    // ── Build widgets ──────────────────────────────────────────────────
    let assets = env!("CARGO_MANIFEST_DIR").to_string() + "/assets";

    let mut wifi_btn = QuickSettingButton::new();
    let (x, y, w, h) = grid::calc_rect((0, 0), (2, 0));
    wifi_btn.set_rect(x, y, w, h);
    wifi_btn.set_label("WiFi");
    wifi_btn.set_active(false);
    wifi_btn.set_status("Loading...");
    wifi_btn.set_icons(
        Some(format!("{}/wifi.svg", assets)),
        Some(format!("{}/wifi-off.svg", assets)),
    );
    {
        let s = sync_sender.clone();
        wifi_btn.connect_toggle(move |active| {
            api::network::set_wifi_enabled(active);
            // Force a re-sync after a short delay
            let s2 = s.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_secs(1));
                let wifi = Some((api::network::is_wifi_active(), api::network::get_wifi_status()));
                let _ = s2.send(SyncMessage::WifiBluetooth { wifi, bt: None });
            });
        });
    }
    wifi_btn.connect_detail_clicked(|| api::network::open_wifi_menu());

    let mut bluetooth_btn = QuickSettingButton::new();
    let (x, y, w, h) = grid::calc_rect((3, 0), (5, 0));
    bluetooth_btn.set_rect(x, y, w, h);
    bluetooth_btn.set_label("BT");
    bluetooth_btn.set_active(false);
    bluetooth_btn.set_status("Loading...");
    bluetooth_btn.set_icons(
        Some(format!("{}/bluetooth.svg", assets)),
        Some(format!("{}/bluetooth-off.svg", assets)),
    );
    {
        let s = sync_sender.clone();
        bluetooth_btn.connect_toggle(move |active| {
            api::bluetooth::set_bluetooth_enabled(active);
            let s2 = s.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_secs(1));
                let bt = Some(api::bluetooth::is_bluetooth_active());
                let _ = s2.send(SyncMessage::WifiBluetooth { wifi: None, bt });
            });
        });
    }
    bluetooth_btn.connect_detail_clicked(|| api::bluetooth::open_bluetooth_menu());

    // Brightness row
    let mut brightness_btn = IconButton::new();
    let (x, y, w, h) = grid::calc_rect((0, 1), (0, 1));
    brightness_btn.set_rect(x, y, w, h);
    brightness_btn.set_active(false);
    brightness_btn.set_icon_colors("#7A5020", "#D97706");
    brightness_btn.set_icon_from_file(Some(format!("{}/blue-filter.svg", assets)));

    let mut brightness_slider = VerticalSlider::new(1.0);
    let (x, y, w, h) = grid::calc_rect((1, 1), (5, 1));
    brightness_slider.set_rect(x, y, w, h);
    brightness_slider.set_icon_generator({
        let a = assets.clone();
        move |val| {
            let name = if val <= 0.33 { "brightness-0.svg" }
                else if val <= 0.66 { "brightness-1.svg" }
                else { "brightness-2.svg" };
            Some(std::path::PathBuf::from(format!("{}/{}", a, name)))
        }
    });
    brightness_slider.connect_value_changed(|value| api::brightness::set_brightness(value));

    // Volume row
    let mut volume_btn = IconButton::new();
    let (x, y, w, h) = grid::calc_rect((0, 2), (0, 2));
    volume_btn.set_rect(x, y, w, h);
    volume_btn.set_active(false);
    volume_btn.set_icon_colors("#7A5020", "#B02010");
    volume_btn.set_icon_from_file(Some(format!("{}/volume-0.svg", assets)));

    let mut volume_slider = VerticalSlider::new(0.0);
    let (x, y, w, h) = grid::calc_rect((1, 2), (5, 2));
    volume_slider.set_rect(x, y, w, h);
    volume_slider.set_icon_generator({
        let a = assets.clone();
        move |val| {
            let name = if val <= 0.01 { "volume-0.svg" }
                else if val <= 0.50 { "volume-1.svg" }
                else { "volume-2.svg" };
            Some(std::path::PathBuf::from(format!("{}/{}", a, name)))
        }
    });
    volume_slider.connect_value_changed(|value| {
        api::audio::set_volume(value);
        if value > 0.0 {
            api::audio::set_mute(false);
        }
    });

    // Media player
    let mut media_player = MediaPlayer::new();
    let (x, y, w, h) = grid::calc_rect((0, 3), (5, 4));
    media_player.set_rect(x, y, w, h);
    media_player.connect_play_pause(|| api::media::play_pause());
    media_player.connect_next(|| api::media::next());
    media_player.connect_previous(|| api::media::previous());

    // Initial media state
    let startup_media = api::media::get_media_state();
    media_player.update_from_state(&startup_media);

    // Font cache
    let font_cache = FontCache::new();

    // ── Assemble app state ─────────────────────────────────────────────
    let mut app = ControlCenter {
        registry_state: RegistryState::new(&globals),
        seat_state,
        output_state,
        compositor_state,
        layer_shell,
        backdrop_pool: SlotPool::new(4096 * 2160 * 4, &shm).expect("shm pool"),
        shm,
        pool,
        panel_surface: None,
        backdrop_surface: None,
        panel_width: grid::GRID_WIDTH as u32,
        panel_height: grid::GRID_HEIGHT as u32,
        backdrop_width: 0,
        backdrop_height: 0,
        panel_configured: false,
        backdrop_configured: false,
        visible: false,
        needs_draw: false,
        wifi_btn,
        bluetooth_btn,
        brightness_btn,
        brightness_slider,
        volume_btn,
        volume_slider,
        media_player,
        sync_sender: sync_sender.clone(),
        wifi_toggle_time: None,
        bt_toggle_time: None,
        last_volume: 0.0,
        dragging: None,
        font_cache,
        qh: qh.clone(),
    };

    // Initial background sync
    app.immediate_sync();

    // ── IPC socket listener ────────────────────────────────────────────
    let sock_path = socket_path();
    let _ = std::fs::remove_file(&sock_path);
    let listener = match UnixListener::bind(&sock_path) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("[CC] Failed to bind IPC socket at {:?}: {}", sock_path, e);
            return;
        }
    };
    eprintln!("[CC] IPC socket listening at {:?}", sock_path);

    std::thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(mut s) => {
                    let mut buf = [0u8; 64];
                    if let Ok(n) = s.read(&mut buf) {
                        let msg = String::from_utf8_lossy(&buf[..n]);
                        if msg.trim() == "toggle" {
                            let _ = ipc_sender.send(());
                            let _ = s.write_all(b"ok");
                        }
                    }
                }
                Err(e) => eprintln!("[CC] IPC accept error: {}", e),
            }
        }
    });

    // Clean up socket on exit
    ctrlc_cleanup();

    // ── Main loop ──────────────────────────────────────────────────────
    eprintln!("[CC] Entering main loop");
    loop {
        // Dispatch all pending Wayland events
        if let Err(e) = event_loop.dispatch(Duration::from_millis(16), &mut app) {
            eprintln!("[CC] Event loop error: {}", e);
            break;
        }
        
        // Immediately redraw if any event flagged a state change
        if app.needs_draw && app.panel_configured {
            app.draw_panel();
        }
    }

    let _ = std::fs::remove_file(socket_path());
}

fn ctrlc_cleanup() {
    unsafe {
        extern "C" fn atexit_handler() {
            let uid = unsafe { libc::getuid() };
            let path = format!("/tmp/control-center-{}.sock", uid);
            let _ = std::fs::remove_file(path);
        }
        libc::atexit(atexit_handler);
    }
}
