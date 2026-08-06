//! ============================================================================
//! CONTROL CENTER DAEMON & TUI PANEL
//! ============================================================================
//!
//! ### BUILD & DAEMON RESTART WORKFLOW
//! If you modify any code in this project, ALWAYS ensure old daemon processes
//! are completely terminated before launching a new daemon:
//!
//! 1. Build release binary:
//!    $ cd ~/.config/hypr/scripts/control-center
//!    $ cargo build --release
//!
//! 2. Terminate all old control-center processes:
//!    $ pkill -9 -f control-center || true
//!    $ killall -9 control-center 2>/dev/null || true
//!    $ rm -f /tmp/control-center-*.sock
//!    $ ps aux | grep control-center | grep -v grep  # Verify 0 running processes
//!
//! 3. Restart background daemon:
//!    $ WAYLAND_DISPLAY=wayland-1 ~/.config/hypr/scripts/control-center/target/release/control-center >/dev/null 2>&1 &
//!
//! 4. Toggle Control Center overlay via IPC:
//!    $ ~/.config/hypr/scripts/control-center/target/release/control-center
//!    (Or trigger via Hyprland keybinding bound to this binary execution)
//! ============================================================================

use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_keyboard, delegate_layer, delegate_output,
    delegate_pointer, delegate_registry, delegate_seat, delegate_shm,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        keyboard::{KeyboardData, KeyEvent, KeyboardHandler, Keysym, Modifiers},
        pointer::{PointerData, PointerEvent, PointerEventKind, PointerHandler},
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

use render::FontCache;
use widgets::{ConnectionsSection, ControlsSection, DragTarget, MediaSection, SessionSection};

/// Socket path unique per user so multiple sessions don't collide.
fn socket_path() -> std::path::PathBuf {
    let uid = unsafe { libc::getuid() };
    std::path::PathBuf::from(format!("/tmp/control-center-{}.sock", uid))
}

/// Try to send "toggle" to an already-running daemon.
fn try_toggle_existing() -> bool {
    let path = socket_path();
    if let Ok(mut stream) = UnixStream::connect(&path) {
        let _ = stream.set_read_timeout(Some(Duration::from_millis(300)));
        let _ = stream.write_all(b"toggle");
        let _ = stream.flush();
        let mut buf = [0u8; 2];
        let _ = stream.read(&mut buf);
        drop(stream);
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
    RecreateBackdrop,
}

// ---------------------------------------------------------------------------
// Main application state
// ---------------------------------------------------------------------------

#[derive(PartialEq)]
pub enum DragState {
    None,
    Brightness,
    Volume,
    MediaSeek,
}

struct ControlCenter {
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,
    compositor_state: CompositorState,
    shm: Shm,
    layer_shell: LayerShell,

    _seat: Option<wl_seat::WlSeat>,

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

    // Sync state
    sync_sender: calloop::channel::Sender<SyncMessage>,
    wifi_toggle_time: Option<std::time::Instant>,
    bt_toggle_time: Option<std::time::Instant>,

    // Cached state from API syncs
    wifi_active: bool,
    wifi_status: String,
    bt_active: bool,
    bt_status: String,
    brightness: f64,
    blue_light_active: bool,
    volume: f64,
    volume_muted: bool,
    media_state: api::media::MediaState,

    // Interaction state
    drag_state: DragState,

    // Font cache
    font_cache: FontCache,

    qh: QueueHandle<Self>,
}

impl ControlCenter {
    fn show_panel(&mut self) {
        if self.visible {
            return;
        }
        eprintln!("[CC] Showing panel surface!");
        self.visible = true;
        self.needs_draw = true;

        let output = self.output_state.outputs().next();

        // 1. Create full-screen backdrop layer to capture clicks outside the panel
        let backdrop_wl = self.compositor_state.create_surface(&self.qh);
        let backdrop_layer = self.layer_shell.create_layer_surface(
            &self.qh,
            backdrop_wl,
            Layer::Top,
            Some("control-center-backdrop"),
            output.as_ref(),
        );
        backdrop_layer.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
        backdrop_layer.set_exclusive_zone(-1);
        backdrop_layer.set_keyboard_interactivity(KeyboardInteractivity::OnDemand);
        backdrop_layer.commit();
        self.backdrop_surface = Some(backdrop_layer);
        self.backdrop_configured = false;

        // 2. Create control center panel surface
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
        panel_layer.set_margin(61, 11, 0, 0);
        panel_layer.set_exclusive_zone(-1);
        panel_layer.set_keyboard_interactivity(KeyboardInteractivity::OnDemand);
        panel_layer.commit();
        self.panel_surface = Some(panel_layer);
        self.panel_configured = false;

        self.immediate_sync();
    }

    fn hide_panel(&mut self) {
        if !self.visible {
            return;
        }
        eprintln!("[CC] Hiding panel surface!");
        self.visible = false;

        self.panel_surface = None;
        self.backdrop_surface = None;

        self.panel_configured = false;
        self.backdrop_configured = false;
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
        let has_media = self.media_state.status == api::media::PlaybackStatus::Playing 
                     || self.media_state.status == api::media::PlaybackStatus::Paused;

        let target_h = if has_media { 462 } else { 362 };
        if self.panel_height != target_h as u32 {
            self.panel_height = target_h as u32;
            if let Some(ref surface) = self.panel_surface {
                surface.set_size(self.panel_width, self.panel_height);
                surface.commit();
            }
        }

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

        let mut pixmap = match tiny_skia::Pixmap::new(w, h) {
            Some(p) => p,
            None => return,
        };
        pixmap.fill(tiny_skia::Color::TRANSPARENT);

        // Background & border
        let bg = tiny_skia::Color::from_rgba(84.0 / 255.0, 56.0 / 255.0, 43.0 / 255.0, 1.0).unwrap();
        render::fill_rect(&mut pixmap.as_mut(), 0.0, 0.0, w as f32, h as f32, bg);

        let border_color = tiny_skia::Color::from_rgba(122.0 / 255.0, 82.0 / 255.0, 61.0 / 255.0, 1.0).unwrap();
        render::fill_rect(&mut pixmap.as_mut(), 0.0, h as f32 - 1.0, w as f32, 1.0, border_color);

        let outer_pad = 10.0;
        let sec_x = outer_pad;
        let sec_w = w as f32 - 2.0 * outer_pad;
        let font_size = 21.0;

        let text_color = tiny_skia::Color::from_rgba(194.0 / 255.0, 170.0 / 255.0, 149.0 / 255.0, 1.0).unwrap();
        let accent_color = tiny_skia::Color::from_rgba(217.0 / 255.0, 119.0 / 255.0, 54.0 / 255.0, 1.0).unwrap();
        let sec_border = tiny_skia::Color::from_rgba(122.0 / 255.0, 82.0 / 255.0, 61.0 / 255.0, 0.6).unwrap();

        // 1. CONNECTIONS
        let sec1_y = outer_pad + 10.0;
        ConnectionsSection::draw(
            &mut pixmap.as_mut(),
            &mut self.font_cache,
            sec_x, sec1_y, sec_w,
            font_size, accent_color, text_color, sec_border,
            self.wifi_active, &self.wifi_status, self.bt_active, &self.bt_status,
        );

        // 2. CONTROLS
        let sec2_y = sec1_y + ConnectionsSection::HEIGHT + 18.0;
        ControlsSection::draw(
            &mut pixmap.as_mut(),
            &mut self.font_cache,
            sec_x, sec2_y, sec_w,
            font_size, accent_color, text_color, sec_border,
            self.brightness, self.blue_light_active, self.volume, self.volume_muted,
        );

        // 3. MEDIA (conditional)
        let sec3_y = sec2_y + ControlsSection::HEIGHT + 18.0;
        if has_media {
            MediaSection::draw(
                &mut pixmap.as_mut(),
                &mut self.font_cache,
                sec_x, sec3_y, sec_w,
                font_size, accent_color, text_color, sec_border,
                &self.media_state,
            );
        }

        // 4. SESSION
        let sec4_y = if has_media {
            sec3_y + MediaSection::HEIGHT + 18.0
        } else {
            sec2_y + ControlsSection::HEIGHT + 18.0
        };
        SessionSection::draw(
            &mut pixmap.as_mut(),
            &mut self.font_cache,
            sec_x, sec4_y, sec_w,
            font_size, accent_color, sec_border,
        );

        // Submit to Wayland surface
        render::rgba_to_bgra(pixmap.data(), canvas);
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

        canvas.fill(0);

        if let Some(ref surface) = self.backdrop_surface {
            let wl = surface.wl_surface();
            wl.attach(Some(buffer.wl_buffer()), 0, 0);
            wl.damage_buffer(0, 0, w as i32, h as i32);
            wl.commit();
        }
    }

    // ── Input Handling ──────────────────────────────────────────────────

    fn handle_pointer_press(&mut self, x: f64, y: f64) {
        let outer_pad = 10.0;
        let sec_x = outer_pad as f64;
        let sec_w = (self.panel_width as f32 - 2.0 * outer_pad) as f64;
        let font_size = 21.0;

        let sec1_y = (outer_pad + 10.0) as f64;

        match ConnectionsSection::handle_click(
            x, y, &mut self.font_cache, sec_x, sec1_y, sec_w, font_size,
            &mut self.wifi_active, &mut self.bt_active,
        ) {
            widgets::ActionResult::NeedsDraw => {
                self.needs_draw = true;
                return;
            }
            widgets::ActionResult::HidePanel => {
                self.hide_panel();
                return;
            }
            widgets::ActionResult::None => {}
        }

        let sec2_y = sec1_y + ConnectionsSection::HEIGHT as f64 + 18.0;
        match ControlsSection::handle_click(
            x, y, &mut self.font_cache, sec_x, sec2_y, sec_w, font_size,
            &mut self.brightness, &mut self.blue_light_active, &mut self.volume, &mut self.volume_muted,
        ) {
            DragTarget::Brightness => {
                self.drag_state = DragState::Brightness;
                self.needs_draw = true;
                return;
            }
            DragTarget::Volume => {
                self.drag_state = DragState::Volume;
                self.needs_draw = true;
                return;
            }
            DragTarget::None => {}
        }

        let has_media = self.media_state.status == api::media::PlaybackStatus::Playing 
                     || self.media_state.status == api::media::PlaybackStatus::Paused;
        let sec3_y = sec2_y + ControlsSection::HEIGHT as f64 + 18.0;

        if has_media && MediaSection::handle_click(
            x, y, &mut self.font_cache, sec_x, sec3_y, sec_w, font_size,
            &mut self.media_state,
        ) {
            self.needs_draw = true;
            return;
        }

        let sec4_y = if has_media {
            sec3_y + MediaSection::HEIGHT as f64 + 18.0
        } else {
            sec2_y + ControlsSection::HEIGHT as f64 + 18.0
        };
        match SessionSection::handle_click(
            x, y, &mut self.font_cache, sec_x, sec4_y, sec_w, font_size,
        ) {
            widgets::ActionResult::HidePanel => {
                self.hide_panel();
            }
            _ => {}
        }
    }

    fn handle_pointer_motion(&mut self, x: f64, _y: f64) {
        let outer_pad = 10.0;
        let sec_x = outer_pad as f64;
        let sec_w = (self.panel_width as f32 - 2.0 * outer_pad) as f64;
        let font_size = 21.0;

        let char_w = self.font_cache.measure_text("#", font_size, false) as f64;
        let bracket_w = self.font_cache.measure_text("[", font_size, false) as f64;
        let available_inner_w = (sec_w - 24.0) - bracket_w * 2.0;
        let slider_len = (available_inner_w / char_w).floor();
        let slider_w = bracket_w * 2.0 + slider_len * char_w;
        let slider_start_x = sec_x + 12.0;
        let inner_start_x = slider_start_x + bracket_w;
        let inner_w = slider_w - bracket_w * 2.0;

        let process_slider = |val: &mut f64| {
            let pct = (x - inner_start_x) / inner_w;
            *val = pct.clamp(0.0, 1.0);
        };

        match self.drag_state {
            DragState::Brightness => {
                let old_pct = (self.brightness * 100.0).round() as i32;
                process_slider(&mut self.brightness);
                let new_pct = (self.brightness * 100.0).round() as i32;
                if old_pct != new_pct {
                    api::brightness::set_brightness(self.brightness);
                }
                self.needs_draw = true;
            }
            DragState::Volume => {
                let old_pct = (self.volume * 100.0).round() as i32;
                process_slider(&mut self.volume);
                let new_pct = (self.volume * 100.0).round() as i32;
                if old_pct != new_pct {
                    api::audio::set_volume(self.volume);
                }
                self.needs_draw = true;
            }
            DragState::MediaSeek => {
                let media_x = sec_x + 12.0;
                let media_w = sec_w - 24.0;
                let elapsed_str = format!("{:02}:{:02}", (self.media_state.position_secs as u64) / 60, (self.media_state.position_secs as u64) % 60);
                let dur_str = format!("{:02}:{:02}", (self.media_state.metadata.length_secs as u64) / 60, (self.media_state.metadata.length_secs as u64) % 60);
                let left_lbl = format!("{} [", elapsed_str);
                let right_lbl = format!("] {}", dur_str);
                let left_lbl_w = self.font_cache.measure_text(&left_lbl, font_size, false) as f64;
                let right_lbl_w = self.font_cache.measure_text(&right_lbl, font_size, false) as f64;

                let rail_start_x = media_x + left_lbl_w;
                let right_lbl_x = media_x + media_w - right_lbl_w;
                let rail_w = right_lbl_x - rail_start_x;

                let pct = (x - rail_start_x) / rail_w;
                let val = pct.clamp(0.0, 1.0);
                let target_sec = val * self.media_state.metadata.length_secs;
                self.media_state.position_secs = target_sec;
                api::media::seek(target_sec);
                self.needs_draw = true;
            }
            DragState::None => {}
        }
    }

    fn handle_pointer_release(&mut self, _x: f64, _y: f64) {
        self.drag_state = DragState::None;
    }

    // ── Background Sync Loop ───────────────────────────────────────────

    fn immediate_sync(&self) {
        let s = self.sync_sender.clone();
        std::thread::spawn(move || {
            let b = api::brightness::get_brightness();
            let bl = api::compositor::is_blue_light_active();
            let _ = s.send(SyncMessage::Brightness { brightness: b, blue_active: bl });
        });
        let s = self.sync_sender.clone();
        std::thread::spawn(move || {
            let v = api::audio::get_volume();
            let m = api::audio::is_muted();
            let _ = s.send(SyncMessage::Volume { volume: v, muted: m });
        });
        let s = self.sync_sender.clone();
        std::thread::spawn(move || {
            let w_enabled = api::network::is_wifi_enabled();
            let w_ssid = api::network::get_connected_ssid();
            let b_enabled = api::bluetooth::is_bluetooth_enabled();
            let _ = s.send(SyncMessage::WifiBluetooth {
                wifi: Some((w_enabled, w_ssid)),
                bt: Some(b_enabled),
            });
        });
        let s = self.sync_sender.clone();
        std::thread::spawn(move || {
            let media = api::media::get_media_state();
            let _ = s.send(SyncMessage::Media(media));
        });
    }

    fn spawn_sync_thread(sender: calloop::channel::Sender<SyncMessage>) {
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_millis(500));

            let w_enabled = api::network::is_wifi_enabled();
            let w_ssid = api::network::get_connected_ssid();
            let b_enabled = api::bluetooth::is_bluetooth_enabled();
            if sender.send(SyncMessage::WifiBluetooth {
                wifi: Some((w_enabled, w_ssid)),
                bt: Some(b_enabled),
            }).is_err() {
                break;
            }

            let b = api::brightness::get_brightness();
            let bl = api::compositor::is_blue_light_active();
            if sender.send(SyncMessage::Brightness { brightness: b, blue_active: bl }).is_err() {
                break;
            }

            let v = api::audio::get_volume();
            let m = api::audio::is_muted();
            if sender.send(SyncMessage::Volume { volume: v, muted: m }).is_err() {
                break;
            }

            let media = api::media::get_media_state();
            if sender.send(SyncMessage::Media(media)).is_err() {
                break;
            }
        });
    }
}

// ---------------------------------------------------------------------------
// SCTK Handlers Implementation
// ---------------------------------------------------------------------------

impl CompositorHandler for ControlCenter {
    fn scale_factor_changed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: i32) {}
    fn transform_changed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: wl_output::Transform) {}
    fn frame(&mut self, _: &Connection, _: &QueueHandle<Self>, surface: &wl_surface::WlSurface, _: u32) {
        if let Some(ref panel) = self.panel_surface {
            if panel.wl_surface() == surface && self.needs_draw {
                self.draw_panel();
            }
        }
    }
    fn surface_enter(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: &wl_output::WlOutput) {}
    fn surface_leave(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: &wl_output::WlOutput) {}
}

impl OutputHandler for ControlCenter {
    fn output_state(&mut self) -> &mut OutputState { &mut self.output_state }
    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
}

impl LayerShellHandler for ControlCenter {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, layer: &LayerSurface) {
        if let Some(ref panel) = self.panel_surface {
            if panel.wl_surface() == layer.wl_surface() {
                self.panel_surface = None;
                self.visible = false;
            }
        }
        if let Some(ref backdrop) = self.backdrop_surface {
            if backdrop.wl_surface() == layer.wl_surface() {
                self.backdrop_surface = None;
            }
        }
    }

    fn configure(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _: u32,
    ) {
        if let Some(ref backdrop) = self.backdrop_surface {
            if backdrop.wl_surface() == layer.wl_surface() {
                let (w, h) = configure.new_size;
                self.backdrop_width = if w > 0 { w } else { 1920 };
                self.backdrop_height = if h > 0 { h } else { 1080 };
                self.backdrop_configured = true;
                self.draw_backdrop();
                return;
            }
        }

        if let Some(ref panel) = self.panel_surface {
            if panel.wl_surface() == layer.wl_surface() {
                let (w, h) = configure.new_size;
                if w > 0 { self.panel_width = w; }
                if h > 0 { self.panel_height = h; }
                self.panel_configured = true;
                self.draw_panel();
            }
        }
    }
}

impl SeatHandler for ControlCenter {
    fn seat_state(&mut self) -> &mut SeatState { &mut self.seat_state }
    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
    fn new_capability(&mut self, _: &Connection, qh: &QueueHandle<Self>, seat: wl_seat::WlSeat, capability: Capability) {
        if capability == Capability::Pointer {
            seat.get_pointer(qh, PointerData::new(seat.clone()));
        }
        if capability == Capability::Keyboard {
            seat.get_keyboard(qh, KeyboardData::new(seat.clone()));
        }
    }
    fn remove_capability(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat, _: Capability) {}
    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl PointerHandler for ControlCenter {
    fn pointer_frame(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        for event in events {
            let is_panel = self.panel_surface.as_ref().map_or(false, |p| p.wl_surface() == &event.surface);
            let is_backdrop = self.backdrop_surface.as_ref().map_or(false, |b| b.wl_surface() == &event.surface);

            match event.kind {
                PointerEventKind::Press { button, .. } => {
                    if button == 0x110 {
                        if is_backdrop {
                            self.hide_panel();
                        } else if is_panel {
                            self.handle_pointer_press(event.position.0, event.position.1);
                        }
                    }
                }
                PointerEventKind::Release { button, .. } => {
                    if button == 0x110 && is_panel {
                        self.handle_pointer_release(event.position.0, event.position.1);
                    }
                }
                PointerEventKind::Motion { .. } => {
                    if is_panel && self.drag_state != DragState::None {
                        self.handle_pointer_motion(event.position.0, event.position.1);
                    }
                }
                _ => {}
            }
        }
    }
}

impl KeyboardHandler for ControlCenter {
    fn enter(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: &wl_surface::WlSurface, _: u32, _: &[u32], _: &[Keysym]) {}
    fn leave(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: &wl_surface::WlSurface, _: u32) {}
    fn press_key(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: u32, event: KeyEvent) {
        if event.raw_code == 1 { // Escape key
            self.hide_panel();
        }
    }
    fn release_key(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: u32, _: KeyEvent) {}
    fn update_modifiers(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: u32, _: Modifiers, _: Layout) {}
}

type Layout = u32;

impl ShmHandler for ControlCenter {
    fn shm_state(&mut self) -> &mut Shm { &mut self.shm }
}

delegate_compositor!(ControlCenter);
delegate_output!(ControlCenter);
delegate_layer!(ControlCenter);
delegate_seat!(ControlCenter);
delegate_pointer!(ControlCenter);
delegate_keyboard!(ControlCenter);
delegate_shm!(ControlCenter);
delegate_registry!(ControlCenter);

impl ProvidesRegistryState for ControlCenter {
    fn registry(&mut self) -> &mut RegistryState { &mut self.registry_state }
    registry_handlers![OutputState, SeatState];
}

// ---------------------------------------------------------------------------
// Entry Point
// ---------------------------------------------------------------------------

fn main() {
    if try_toggle_existing() {
        eprintln!("[CC] Toggled existing instance via IPC socket.");
        return;
    }

    let socket_p = socket_path();
    let _ = std::fs::remove_file(&socket_p);

    let ipc_listener = match UnixListener::bind(&socket_p) {
        Ok(l) => {
            l.set_nonblocking(true).expect("Cannot set non-blocking on IPC socket");
            l
        }
        Err(e) => {
            eprintln!("[CC] Warning: Failed to bind IPC socket {:?}: {}", socket_p, e);
            return;
        }
    };

    let conn = Connection::connect_to_env().expect("Failed to connect to Wayland display");
    let (globals, mut event_queue) = registry_queue_init(&conn).expect("Failed to init registry");
    let qh = event_queue.handle();

    let compositor_state = CompositorState::bind(&globals, &qh).expect("wl_compositor unavailable");
    let layer_shell = LayerShell::bind(&globals, &qh).expect("zwlr_layer_shell_v1 unavailable");
    let shm = Shm::bind(&globals, &qh).expect("wl_shm unavailable");

    let pool = SlotPool::new(grid::PANEL_WIDTH as usize * grid::PANEL_HEIGHT as usize * 4, &shm)
        .expect("Failed to create panel slot pool");

    let backdrop_pool = SlotPool::new(1920 * 1080 * 4, &shm)
        .expect("Failed to create backdrop slot pool");

    let (sync_tx, sync_rx) = calloop::channel::channel::<SyncMessage>();
    ControlCenter::spawn_sync_thread(sync_tx.clone());

    let font_cache = FontCache::new();

    let mut app = ControlCenter {
        registry_state: RegistryState::new(&globals),
        seat_state: SeatState::new(&globals, &qh),
        output_state: OutputState::new(&globals, &qh),
        compositor_state,
        shm,
        layer_shell,
        _seat: None,
        pool,
        backdrop_pool,
        panel_surface: None,
        backdrop_surface: None,
        panel_width: grid::PANEL_WIDTH as u32,
        panel_height: grid::PANEL_HEIGHT as u32,
        backdrop_width: 1920,
        backdrop_height: 1080,
        panel_configured: false,
        backdrop_configured: false,
        visible: false,
        needs_draw: true,
        sync_sender: sync_tx,
        wifi_toggle_time: None,
        bt_toggle_time: None,
        wifi_active: false,
        wifi_status: "Disabled".to_string(),
        bt_active: false,
        bt_status: "Disconnected".to_string(),
        brightness: 0.8,
        blue_light_active: false,
        volume: 0.5,
        volume_muted: false,
        media_state: api::media::MediaState {
            status: api::media::PlaybackStatus::None,
            metadata: api::media::MediaMetadata::default(),
            position_secs: 0.0,
        },
        drag_state: DragState::None,
        font_cache,
        qh,
    };

    app.show_panel();

    let mut event_loop: calloop::EventLoop<ControlCenter> =
        calloop::EventLoop::try_new().expect("Failed to create calloop EventLoop");
    let loop_handle = event_loop.handle();

    smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource::new(conn.clone(), event_queue)
        .insert(loop_handle.clone())
        .expect("Failed to insert WaylandSource into EventLoop");

    let _ = loop_handle.insert_source(
        sync_rx,
        |event, _, state| {
            if let calloop::channel::Event::Msg(msg) = event {
                match msg {
                    SyncMessage::WifiBluetooth { wifi, bt } => {
                        let now = std::time::Instant::now();
                        let wifi_grace = state.wifi_toggle_time.map_or(false, |t| now.duration_since(t) < Duration::from_millis(3000));
                        let bt_grace = state.bt_toggle_time.map_or(false, |t| now.duration_since(t) < Duration::from_millis(3000));

                        if let Some((w_active, w_ssid)) = wifi {
                            if !wifi_grace {
                                state.wifi_active = w_active;
                                state.wifi_status = w_ssid;
                            }
                        }
                        if let Some(b_active) = bt {
                            if !bt_grace {
                                state.bt_active = b_active;
                                state.bt_status = if b_active { "Connected".to_string() } else { "Disabled".to_string() };
                            }
                        }
                        state.needs_draw = true;
                    }
                    SyncMessage::Brightness { brightness, blue_active } => {
                        if state.drag_state != DragState::Brightness {
                            state.brightness = brightness;
                        }
                        state.blue_light_active = blue_active;
                        state.needs_draw = true;
                    }
                    SyncMessage::Volume { volume, muted } => {
                        if state.drag_state != DragState::Volume {
                            state.volume = volume;
                        }
                        state.volume_muted = muted;
                        state.needs_draw = true;
                    }
                    SyncMessage::Media(media_state) => {
                        if state.drag_state != DragState::MediaSeek {
                            state.media_state = media_state;
                        }
                        state.needs_draw = true;
                    }
                    SyncMessage::RecreateBackdrop => {
                        if state.visible {
                            state.hide_panel();
                            state.show_panel();
                        }
                    }
                }
            }
        },
    );

    let ipc_fd = calloop::generic::Generic::new(
        ipc_listener,
        calloop::Interest::READ,
        calloop::Mode::Level,
    );
    let _ = loop_handle.insert_source(
        ipc_fd,
        |_event, socket, state| {
            if let Ok((mut stream, _)) = unsafe { socket.get_mut() }.accept() {
                let mut buf = [0u8; 128];
                if let Ok(n) = stream.read(&mut buf) {
                    let msg = String::from_utf8_lossy(&buf[..n]);
                    if msg.trim() == "toggle" {
                        state.toggle_panel();
                        let _ = stream.write_all(b"OK");
                        let _ = stream.flush();
                    }
                }
                drop(stream);
            }
            Ok(calloop::PostAction::Continue)
        },
    );

    loop {
        event_loop
            .dispatch(Duration::from_millis(16), &mut app)
            .expect("Error during EventLoop dispatch");
    }
}
