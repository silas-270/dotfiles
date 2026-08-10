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
    compositor::{CompositorHandler, CompositorState, Region},
    delegate_compositor, delegate_keyboard, delegate_layer, delegate_output,
    delegate_pointer, delegate_registry, delegate_seat, delegate_shm,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        keyboard::{KeyboardData, KeyEvent, KeyboardHandler, Keysym, Modifiers},
        pointer::{PointerData, PointerEvent, PointerEventKind, PointerHandler, CursorIcon, ThemeSpec, ThemedPointer},
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
mod theme;
mod widgets;

use render::FontCache;
use theme::ThemeConfig;
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
    backdrop_pressed: bool,

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
    themed_pointer: Option<ThemedPointer>,

    // Font cache
    font_cache: FontCache,

    // Theme configuration
    theme: ThemeConfig,

    qh: QueueHandle<Self>,
}

impl ControlCenter {
    fn show_panel(&mut self) {
        if self.visible {
            return;
        }
        self.theme = ThemeConfig::load();
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

        // Exclude the top 50px (Waybar zone) from the backdrop's input region.
        // The backdrop still renders full-screen, but clicks in the Waybar area
        // pass through to Waybar so its on-click toggle works without needing
        // the compositor to refocus the pointer.
        let waybar_h: i32 = 50;
        if let Ok(region) = Region::new(&self.compositor_state) {
            region.add(0, waybar_h, 1920, 1080 - waybar_h);
            backdrop_layer.wl_surface().set_input_region(Some(region.wl_region()));
        }

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

    fn refresh_backdrop(&mut self) {
        if self.visible && self.backdrop_configured {
            self.draw_backdrop();
            if let Some(ref backdrop) = self.backdrop_surface {
                backdrop.commit();
            }
        }
    }

    fn hide_panel(&mut self) {
        if !self.visible {
            return;
        }
        eprintln!("[CC] Hiding panel surface!");
        self.visible = false;
        self.backdrop_pressed = false;

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
        if !self.visible || !self.panel_configured {
            return;
        }
        let has_media = self.media_state.status == api::media::PlaybackStatus::Playing 
                     || self.media_state.status == api::media::PlaybackStatus::Paused;

        let target_h = if has_media { 629 } else { 485 };
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
        render::fill_rect(&mut pixmap.as_mut(), 0.0, 0.0, w as f32, h as f32, self.theme.bg_base);
        render::fill_rect(&mut pixmap.as_mut(), 0.0, h as f32 - 1.0, w as f32, 1.0, self.theme.border);

        let outer_pad = 10.0;
        let sec_x = outer_pad;
        let sec_w = w as f32 - 2.0 * outer_pad;
        let font_size = 21.0;

        // 1. CONNECTIONS
        let sec1_y = 0.0;
        ConnectionsSection::draw(
            &mut pixmap.as_mut(),
            &mut self.font_cache,
            sec_x, sec1_y, sec_w,
            font_size, &self.theme,
            self.wifi_active, &self.wifi_status, self.bt_active, &self.bt_status,
        );

        // 2. CONTROLS
        let sec2_y = sec1_y + ConnectionsSection::HEIGHT;
        ControlsSection::draw(
            &mut pixmap.as_mut(),
            &mut self.font_cache,
            sec_x, sec2_y, sec_w,
            font_size, &self.theme,
            self.brightness, self.blue_light_active, self.volume, self.volume_muted,
        );

        // 3. MEDIA (conditional)
        let sec3_y = sec2_y + ControlsSection::HEIGHT;
        if has_media {
            MediaSection::draw(
                &mut pixmap.as_mut(),
                &mut self.font_cache,
                sec_x, sec3_y, sec_w,
                font_size, &self.theme,
                &self.media_state,
            );
        }

        // 4. SESSION
        let sec4_y = if has_media {
            sec3_y + MediaSection::HEIGHT
        } else {
            sec2_y + ControlsSection::HEIGHT
        };
        SessionSection::draw(
            &mut pixmap.as_mut(),
            &mut self.font_cache,
            sec_x, sec4_y, sec_w,
            font_size, &self.theme,
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
        if !self.visible || !self.backdrop_configured {
            return;
        }
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

        let sec1_y = 0.0;

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

        let old_blue_active = self.blue_light_active;
        let sec2_y = sec1_y + ConnectionsSection::HEIGHT as f64;
        match ControlsSection::handle_click(
            x, y, &mut self.font_cache, sec_x, sec2_y, sec_w, font_size,
            &mut self.brightness, &mut self.blue_light_active, &mut self.volume, &mut self.volume_muted,
        ) {
            DragTarget::Brightness => {
                self.drag_state = DragState::Brightness;
                self.handle_pointer_motion(x, y);
                return;
            }
            DragTarget::Volume => {
                self.drag_state = DragState::Volume;
                self.handle_pointer_motion(x, y);
                return;
            }
            DragTarget::None => {}
        }

        if self.blue_light_active != old_blue_active {
            self.refresh_backdrop();
            self.needs_draw = true;
        }

        let has_media = self.media_state.status == api::media::PlaybackStatus::Playing 
                     || self.media_state.status == api::media::PlaybackStatus::Paused;
        let sec3_y = sec2_y + ControlsSection::HEIGHT as f64;

        if has_media {
            match MediaSection::handle_click(
                x, y, &mut self.font_cache, sec_x, sec3_y, sec_w, font_size,
                &mut self.media_state,
            ) {
                widgets::MediaClickResult::Seek => {
                    self.drag_state = DragState::MediaSeek;
                    self.needs_draw = true;
                    return;
                }
                widgets::MediaClickResult::Button => {
                    self.needs_draw = true;
                    return;
                }
                widgets::MediaClickResult::None => {}
            }
        }

        let sec4_y = if has_media {
            sec3_y + MediaSection::HEIGHT as f64
        } else {
            sec2_y + ControlsSection::HEIGHT as f64
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

        let char_w = self.font_cache.measure_text("=", font_size, false) as f64;
        let bracket_w = self.font_cache.measure_text("(", font_size, false) as f64;
        let box_x = sec_x + 8.0;
        let box_w = sec_w - 16.0;
        let icon_x = box_x + 8.0;
        let available_slider_w = box_w - 16.0;
        let available_inner_w = available_slider_w - bracket_w * 2.0;
        let slider_len = (available_inner_w / char_w).floor();
        let slider_w = bracket_w * 2.0 + slider_len * char_w;
        let inner_start_x = icon_x + bracket_w;
        let inner_w = slider_w - bracket_w * 2.0;

        match self.drag_state {
            DragState::Brightness => {
                let old_val = self.brightness;
                let pct = ((x - inner_start_x) / inner_w).clamp(0.0, 1.0);
                self.brightness = pct;
                let old_pct = (old_val * slider_len).round() as usize;
                let new_pct = (pct * slider_len).round() as usize;
                if old_pct != new_pct {
                    api::brightness::set_brightness(self.brightness);
                }
                self.needs_draw = true;
            }
            DragState::Volume => {
                let old_val = self.volume;
                let pct = ((x - inner_start_x) / inner_w).clamp(0.0, 1.0);
                self.volume = pct;
                let old_pct = (old_val * slider_len).round() as usize;
                let new_pct = (pct * slider_len).round() as usize;
                if old_pct != new_pct {
                    api::audio::set_volume(self.volume);
                }
                self.needs_draw = true;
            }
            DragState::MediaSeek => {
                let m_box_x = sec_x;
                let m_box_w = sec_w;
                let m_icon_x = m_box_x + 8.0;
                let m_content_w = m_box_w - 16.0;
                let elapsed_str = format!("{:02}:{:02}", (self.media_state.position_secs as u64) / 60, (self.media_state.position_secs as u64) % 60);
                let dur_str = format!("{:02}:{:02}", (self.media_state.metadata.length_secs as u64) / 60, (self.media_state.metadata.length_secs as u64) % 60);
                let left_lbl = format!("{} (", elapsed_str);
                let right_lbl = format!(") {}", dur_str);
                let left_lbl_w = self.font_cache.measure_text(&left_lbl, font_size, false) as f64;
                let right_lbl_w = self.font_cache.measure_text(&right_lbl, font_size, false) as f64;

                let rail_start_x = m_icon_x + left_lbl_w;
                let right_lbl_x = m_icon_x + m_content_w - right_lbl_w;
                let rail_w = right_lbl_x - rail_start_x;

                if rail_w > 0.0 {
                    let pct = ((x - rail_start_x) / rail_w).clamp(0.0, 1.0);
                    let target_sec = pct * self.media_state.metadata.length_secs;
                    self.media_state.position_secs = target_sec;
                    self.needs_draw = true;
                }
            }
            DragState::None => {}
        }
    }

    fn handle_pointer_release(&mut self, _x: f64, _y: f64) {
        if self.drag_state == DragState::MediaSeek {
            api::media::seek(self.media_state.position_secs);
        }
        self.drag_state = DragState::None;
    }

    fn is_hover_interactive(&self, x: f64, y: f64) -> bool {
        let outer_pad = 10.0;
        let sec_x = outer_pad as f64;
        let sec_w = (self.panel_width as f32 - 2.0 * outer_pad) as f64;
        if x < sec_x || x > sec_x + sec_w {
            return false;
        }

        let sec1_y = 0.0;
        let wifi_box_y = sec1_y + 49.0;
        let wifi_box_h = 66.0;
        if y >= wifi_box_y && y <= wifi_box_y + wifi_box_h {
            return true;
        }

        let bt_box_y = sec1_y + 129.0;
        let bt_box_h = 66.0;
        if y >= bt_box_y && y <= bt_box_y + bt_box_h {
            return true;
        }

        let sec2_y = sec1_y + ConnectionsSection::HEIGHT as f64;
        let bright_box_y = sec2_y + 49.0;
        let bright_box_h = 66.0;
        if y >= bright_box_y && y <= bright_box_y + bright_box_h {
            return true;
        }

        let vol_box_y = sec2_y + 129.0;
        let vol_box_h = 66.0;
        if y >= vol_box_y && y <= vol_box_y + vol_box_h {
            return true;
        }

        let has_media = self.media_state.status == api::media::PlaybackStatus::Playing 
                     || self.media_state.status == api::media::PlaybackStatus::Paused;
        let sec3_y = sec2_y + ControlsSection::HEIGHT as f64;

        if has_media {
            let media_interactive_top = sec3_y + 49.0;
            let media_interactive_bottom = sec3_y + 144.0;
            if y >= media_interactive_top && y <= media_interactive_bottom {
                return true;
            }
        }

        let sec4_y = if has_media {
            sec3_y + MediaSection::HEIGHT as f64
        } else {
            sec2_y + ControlsSection::HEIGHT as f64
        };

        let sess_box_y = sec4_y + 49.0;
        let sess_box_h = 32.0;
        if y >= sess_box_y && y <= sess_box_y + sess_box_h {
            return true;
        }

        false
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

                // Force Hyprland to re-evaluate pointer focus after both
                // surfaces are mapped. Without this, Hyprland sends
                // wl_pointer::leave to Waybar when our surfaces map but
                // never re-enters until the mouse physically moves.
                // Jiggle 1px right then back (imperceptible) using the
                // Hyprland 0.56+ Lua dispatch syntax.
                std::thread::spawn(|| {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    if let Ok(pos) = std::process::Command::new("hyprctl")
                        .args(["cursorpos"])
                        .output()
                    {
                        let pos_str = String::from_utf8_lossy(&pos.stdout);
                        if let Some((x_str, y_str)) = pos_str.trim().split_once(", ") {
                            if let (Ok(x), Ok(y)) = (x_str.parse::<i32>(), y_str.parse::<i32>()) {
                                let cmd1 = format!("hl.dsp.cursor.move({{x={}, y={}}})", x + 1, y);
                                let cmd2 = format!("hl.dsp.cursor.move({{x={}, y={}}})", x, y);
                                let _ = std::process::Command::new("hyprctl")
                                    .args(["dispatch", &cmd1])
                                    .output();
                                let _ = std::process::Command::new("hyprctl")
                                    .args(["dispatch", &cmd2])
                                    .output();
                            }
                        }
                    }
                });
            }
        }
    }
}

impl SeatHandler for ControlCenter {
    fn seat_state(&mut self) -> &mut SeatState { &mut self.seat_state }
    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
    fn new_capability(&mut self, _: &Connection, qh: &QueueHandle<Self>, seat: wl_seat::WlSeat, capability: Capability) {
        if capability == Capability::Pointer {
            let surface = self.compositor_state.create_surface(qh);
            if let Ok(themed) = self.seat_state.get_pointer_with_theme(qh, &seat, self.shm.wl_shm(), surface, ThemeSpec::System) {
                self.themed_pointer = Some(themed);
            } else {
                seat.get_pointer(qh, PointerData::new(seat.clone()));
            }
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
        conn: &Connection,
        _: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        for event in events {
            let is_panel = self.panel_surface.as_ref().map_or(false, |p| p.wl_surface() == &event.surface);
            let is_backdrop = self.backdrop_surface.as_ref().map_or(false, |b| b.wl_surface() == &event.surface);

            let (px, py) = event.position;
            let is_interactive = is_panel && self.is_hover_interactive(px, py);

            if let Some(ref mut themed) = self.themed_pointer {
                if self.drag_state != DragState::None {
                    let _ = themed.set_cursor(conn, CursorIcon::Grabbing);
                } else if is_interactive {
                    let _ = themed.set_cursor(conn, CursorIcon::Pointer);
                } else {
                    let _ = themed.set_cursor(conn, CursorIcon::Default);
                }
            }

            match event.kind {
                PointerEventKind::Press { button, .. } => {
                    if button == 0x110 {
                        if is_backdrop {
                            self.backdrop_pressed = true;
                        } else if is_panel {
                            self.handle_pointer_press(px, py);
                        }
                    }
                }
                PointerEventKind::Release { button, .. } => {
                    if button == 0x110 {
                        if is_backdrop && self.backdrop_pressed {
                            self.backdrop_pressed = false;
                            self.hide_panel();
                        }
                        if self.drag_state != DragState::None {
                            self.handle_pointer_release(px, py);
                        }
                    }
                }
                PointerEventKind::Motion { .. } => {
                    if self.drag_state != DragState::None {
                        self.handle_pointer_motion(px, py);
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
        backdrop_pressed: false,
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
        themed_pointer: None,
        font_cache,
        theme: ThemeConfig::load(),
        qh,
    };

    let start_daemon_only = std::env::args().any(|a| a == "--daemon");
    if !start_daemon_only {
        app.show_panel();
    }

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
                        if state.blue_light_active != blue_active {
                            state.blue_light_active = blue_active;
                            state.refresh_backdrop();
                        }
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
                        state.refresh_backdrop();
                        state.needs_draw = true;
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

        if app.visible && app.needs_draw {
            app.draw_panel();
        }
    }
}
