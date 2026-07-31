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

// We keep the widgets module around for FontCache (text rendering).
mod widgets;
use widgets::FontCache;

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
}

struct ControlCenter {
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,
    compositor_state: CompositorState,
    shm: Shm,
    layer_shell: LayerShell,
    
    seat: Option<wl_seat::WlSeat>,

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

    // Cached state from API syncs (to be used by future widgets)
    wifi_active: bool,
    wifi_status: String,
    bt_active: bool,
    brightness: f64,
    blue_light_active: bool,
    volume: f64,
    volume_muted: bool,

    // Interaction state
    drag_state: DragState,

    // Font cache
    font_cache: FontCache,

    // For creating surfaces later
    qh: QueueHandle<Self>,
}

impl ControlCenter {
    // ── Show / Hide ─────────────────────────────────────────────────────

    fn show_panel(&mut self) {
        if self.visible {
            eprintln!("[CC] show_panel called but already visible");
            return;
        }
        eprintln!("[CC] Showing panel surface...");
        self.visible = true;

        self.panel_height = grid::PANEL_HEIGHT as u32;
        self.panel_width = grid::PANEL_WIDTH as u32;

        // Create backdrop (fullscreen click-catcher)
        self.create_backdrop_surface();

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
        eprintln!("[CC] Hiding panel surface!");
        self.visible = false;
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

    fn create_backdrop_surface(&mut self) {
        if !self.visible { return; }
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
    }

    // ── Drawing ─────────────────────────────────────────────────────────

    fn draw_panel(&mut self) {
        let w = self.panel_width;
        let h = self.panel_height;
        eprintln!("[CC] draw_panel called with size {}x{}", w, h);
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

        // ── Background: flat @bg-base (#382319) — same as Waybar ──
        // R=0x38=56  G=0x23=35  B=0x19=25 → normalized
        let bg = tiny_skia::Color::from_rgba(
            56.0 / 255.0,   // R = 0.2196
            35.0 / 255.0,   // G = 0.1373
            25.0 / 255.0,   // B = 0.0980
            1.0,
        ).unwrap();
        render::fill_rect(
            &mut pixmap.as_mut(),
            0.0, 0.0,
            w as f32, h as f32,
            bg,
        );

        // ── Border: 1px @border (#7A523D) on bottom edge — same as Waybar ──
        let border_color = tiny_skia::Color::from_rgba(
            122.0 / 255.0,  // R
            82.0 / 255.0,   // G
            61.0 / 255.0,   // B
            1.0,
        ).unwrap();
        render::fill_rect(
            &mut pixmap.as_mut(),
            0.0, h as f32 - grid::BORDER as f32,
            w as f32, grid::BORDER as f32,
            border_color,
        );
        // Left border
        render::fill_rect(
            &mut pixmap.as_mut(),
            0.0, 0.0,
            grid::BORDER as f32, h as f32,
            border_color,
        );
        // Right border
        render::fill_rect(
            &mut pixmap.as_mut(),
            w as f32 - grid::BORDER as f32, 0.0,
            grid::BORDER as f32, h as f32,
            border_color,
        );
        // Top border
        render::fill_rect(
            &mut pixmap.as_mut(),
            0.0, 0.0,
            w as f32, grid::BORDER as f32,
            border_color,
        );

        let outer_pad = 10.0;
        let rect_x = outer_pad;
        let rect_y = outer_pad;
        let rect_w = w as f32 - 2.0 * outer_pad;
        let wifi_rect_h = 54.0;
        let bt_rect_h = 54.0;
        let bright_rect_h = 54.0;
        let vol_rect_h = 54.0;

        // 2px solid @border (#7A523D) - brownish-orange
        let box_border = tiny_skia::Color::from_rgba(
            122.0 / 255.0,  // R (#7A523D)
            82.0 / 255.0,   // G
            61.0 / 255.0,   // B
            1.0,
        ).unwrap();

        // ── WiFi Row ──
        render::draw_rect_outline(
            &mut pixmap.as_mut(),
            rect_x, rect_y,
            rect_w, wifi_rect_h,
            2.0,
            box_border,
        );

        // Text color (@fg-primary)
        let text_color = tiny_skia::Color::from_rgba(
            242.0 / 255.0,  // R (#F2E3D5)
            227.0 / 255.0,  // G
            213.0 / 255.0,  // B
            1.0,
        ).unwrap();

        // Accent color (Orange #D97736 - same as Waybar clock)
        let accent_color = tiny_skia::Color::from_rgba(
            217.0 / 255.0,  // R (#D97736)
            119.0 / 255.0,  // G
            54.0 / 255.0,   // B
            1.0,
        ).unwrap();

        let font_size = 21.0;
        let icon_x = rect_x + 6.0; // Left padding inside box

        let wifi_line1_y = rect_y + 5.0;
        let wifi_line2_y = rect_y + 29.0;

        // 1. Draw "["
        self.font_cache.draw_text(&mut pixmap.as_mut(), "[", icon_x, wifi_line1_y, font_size, false, accent_color);
        let bracket_w = self.font_cache.measure_text("[", font_size, false);

        // Pre-calculate Bluetooth inner width to force WiFi bracket alignment
        let bt_icon_str = if self.bt_active { "" } else { "󰂲" };
        let bt_icon_w = self.font_cache.measure_text(bt_icon_str, font_size, false);
        let bt_inner_w = (font_size / 3.0) * 2.0 + bt_icon_w;

        // Mathematical alignment:
        let icon_str = if self.wifi_active { "󰖩" } else { "󰖪" };
        let icon_w = self.font_cache.measure_text(icon_str, font_size, false);
        
        let base_left = font_size / 3.0; 
        let base_right = self.font_cache.measure_text(" ", font_size, false);
        
        let available_pad = bt_inner_w - icon_w;
        let excess_pad = (base_left + base_right) - available_pad;
        
        // Subtract half the excess from both sides so the total width matches BT
        // Then apply the 1.5px left shift
        let left_pad = base_left - (excess_pad / 2.0) - 1.5;
        let right_pad = base_right - (excess_pad / 2.0) + 1.5;

        // 2. Draw Icon with exact calculated left padding
        let icon_pos_x = icon_x + bracket_w + left_pad; 
        self.font_cache.draw_text(&mut pixmap.as_mut(), icon_str, icon_pos_x, wifi_line1_y, font_size, false, accent_color);

        // 3. Draw "]" with exact calculated right padding
        let right_bracket_x = icon_pos_x + icon_w + right_pad;
        self.font_cache.draw_text(&mut pixmap.as_mut(), "]", right_bracket_x, wifi_line1_y, font_size, false, accent_color);
        let right_bracket_w = self.font_cache.measure_text("]", font_size, false);

        let total_icon_box_w = (right_bracket_x + right_bracket_w) - icon_x;

        // 4. Line 1: State text (WIFI ON / WIFI OFF)
        let wifi_state_str = if self.wifi_active { "WIFI ON" } else { "WIFI OFF" };
        let space_w = self.font_cache.measure_text(" ", font_size, false);
        let name_x = icon_x + total_icon_box_w + space_w;
        
        self.font_cache.draw_text(
            &mut pixmap.as_mut(),
            wifi_state_str,
            name_x,
            wifi_line1_y,
            font_size,
            false,
            accent_color,
        );

        // 5. Line 2: WiFi Name
        let wifi_name_str = if self.wifi_active {
            if self.wifi_status == "On" { "Disconnected" } else { self.wifi_status.as_str() }
        } else {
            "Disabled"
        };
        self.font_cache.draw_text(
            &mut pixmap.as_mut(),
            wifi_name_str,
            icon_x,
            wifi_line2_y,
            font_size,
            false,
            text_color,
        );

        // ── Bluetooth Row ──
        let bt_rect_y = rect_y + wifi_rect_h + 10.0;
        render::draw_rect_outline(
            &mut pixmap.as_mut(),
            rect_x, bt_rect_y,
            rect_w, bt_rect_h,
            2.0,
            box_border,
        );

        let bt_line1_y = bt_rect_y + 5.0;
        let bt_line2_y = bt_rect_y + 29.0;
        let bt_state_str = if self.bt_active { "BLUETOOTH ON" } else { "BLUETOOTH OFF" };
        let bt_name_str = if self.bt_active { "Disconnected" } else { "Disabled" };
        
        let bt_pad = font_size / 3.0;
        self.font_cache.draw_text(&mut pixmap.as_mut(), "[", icon_x, bt_line1_y, font_size, false, accent_color);
        let bt_icon_pos_x = icon_x + bracket_w + bt_pad; 
        self.font_cache.draw_text(&mut pixmap.as_mut(), bt_icon_str, bt_icon_pos_x, bt_line1_y, font_size, false, accent_color);
        let bt_right_bracket_x = bt_icon_pos_x + bt_icon_w + bt_pad;
        self.font_cache.draw_text(&mut pixmap.as_mut(), "]", bt_right_bracket_x, bt_line1_y, font_size, false, accent_color);
        let bt_right_bracket_w = self.font_cache.measure_text("]", font_size, false);
        let bt_total_icon_box_w = (bt_right_bracket_x + bt_right_bracket_w) - icon_x;
        
        let bt_name_x = icon_x + bt_total_icon_box_w + space_w;
        // Line 1: state text
        self.font_cache.draw_text(&mut pixmap.as_mut(), bt_state_str, bt_name_x, bt_line1_y, font_size, false, accent_color);
        // Line 2: bluetooth name
        self.font_cache.draw_text(&mut pixmap.as_mut(), bt_name_str, icon_x, bt_line2_y, font_size, false, text_color);

        // Dynamic full-width slider length calculation
        let char_w = self.font_cache.measure_text("#", font_size, false);
        let available_inner_w = (rect_w - 12.0) - bracket_w - right_bracket_w;
        let slider_len = (available_inner_w / char_w).floor() as usize;

        // ── Brightness Row ──
        let bright_rect_y = bt_rect_y + bt_rect_h + 10.0;
        render::draw_rect_outline(&mut pixmap.as_mut(), rect_x, bright_rect_y, rect_w, bright_rect_h, 2.0, box_border);
        let bright_line1_y = bright_rect_y + 5.0;
        let bright_line2_y = bright_rect_y + 29.0;
        let bright_icon_str = if self.blue_light_active { "󰃟" } else { "󰃠" };
        
        // Line 1: Icon box [󰃠] + Header "BRIGHTNESS" in accent_color
        self.font_cache.draw_text(&mut pixmap.as_mut(), "[", icon_x, bright_line1_y, font_size, false, accent_color);
        let bright_icon_w = self.font_cache.measure_text(bright_icon_str, font_size, false);
        let available_pad = bt_inner_w - bright_icon_w;
        let excess_pad = (base_left + base_right) - available_pad;
        let bright_left_pad = base_left - (excess_pad / 2.0) - 1.5;
        let bright_right_pad = base_right - (excess_pad / 2.0) + 1.5;
        
        let bright_icon_pos_x = icon_x + bracket_w + bright_left_pad;
        self.font_cache.draw_text(&mut pixmap.as_mut(), bright_icon_str, bright_icon_pos_x, bright_line1_y, font_size, false, accent_color);
        let bright_right_bracket_x = bright_icon_pos_x + bright_icon_w + bright_right_pad;
        self.font_cache.draw_text(&mut pixmap.as_mut(), "]", bright_right_bracket_x, bright_line1_y, font_size, false, accent_color);
        let bright_total_icon_box_w = (bright_right_bracket_x + right_bracket_w) - icon_x;
        
        let bright_name_x = icon_x + bright_total_icon_box_w + space_w;
        self.font_cache.draw_text(&mut pixmap.as_mut(), "BRIGHTNESS", bright_name_x, bright_line1_y, font_size, false, accent_color);

        // Line 2: Full-width Slider
        let bright_filled = (self.brightness * slider_len as f64).round() as usize;
        let bright_empty = slider_len.saturating_sub(bright_filled);
        let bright_slider_str = format!("[{}{}]", "#".repeat(bright_filled), "-".repeat(bright_empty));
        self.font_cache.draw_text(&mut pixmap.as_mut(), &bright_slider_str, icon_x, bright_line2_y, font_size, false, text_color);

        // ── Volume Row ──
        let vol_rect_y = bright_rect_y + bright_rect_h + 10.0;
        render::draw_rect_outline(&mut pixmap.as_mut(), rect_x, vol_rect_y, rect_w, vol_rect_h, 2.0, box_border);
        let vol_line1_y = vol_rect_y + 5.0;
        let vol_line2_y = vol_rect_y + 29.0;
        let vol_icon_str = if self.volume_muted { "󰖁" } else { "󰕾" };
        
        // Line 1: Icon box [󰕾] + Header "VOLUME" in accent_color
        self.font_cache.draw_text(&mut pixmap.as_mut(), "[", icon_x, vol_line1_y, font_size, false, accent_color);
        let vol_icon_w = self.font_cache.measure_text(vol_icon_str, font_size, false);
        let available_pad = bt_inner_w - vol_icon_w;
        let excess_pad = (base_left + base_right) - available_pad;
        let vol_left_pad = base_left - (excess_pad / 2.0) - 1.5;
        let vol_right_pad = base_right - (excess_pad / 2.0) + 1.5;
        
        let vol_icon_pos_x = icon_x + bracket_w + vol_left_pad;
        self.font_cache.draw_text(&mut pixmap.as_mut(), vol_icon_str, vol_icon_pos_x, vol_line1_y, font_size, false, accent_color);
        let vol_right_bracket_x = vol_icon_pos_x + vol_icon_w + vol_right_pad;
        self.font_cache.draw_text(&mut pixmap.as_mut(), "]", vol_right_bracket_x, vol_line1_y, font_size, false, accent_color);
        let vol_total_icon_box_w = (vol_right_bracket_x + right_bracket_w) - icon_x;
        
        let vol_name_x = icon_x + vol_total_icon_box_w + space_w;
        self.font_cache.draw_text(&mut pixmap.as_mut(), "VOLUME", vol_name_x, vol_line1_y, font_size, false, accent_color);

        // Line 2: Full-width Slider
        let vol_filled = (self.volume * slider_len as f64).round() as usize;
        let vol_empty = slider_len.saturating_sub(vol_filled);
        let vol_slider_str = format!("[{}{}]", "#".repeat(vol_filled), "-".repeat(vol_empty));
        self.font_cache.draw_text(&mut pixmap.as_mut(), &vol_slider_str, icon_x, vol_line2_y, font_size, false, text_color);


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

        // Fully transparent background to catch clicks without visual artifacts.
        // We MUST zero it out, because the memory pool might return uninitialized memory!
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

    // ── Pointer event dispatch (stub — no widgets yet) ─────────────────

    fn handle_pointer_motion(&mut self, x: f64, y: f64) {
        if self.drag_state == DragState::None {
            return;
        }

        let x = x as f32;
        let outer_pad = 10.0;
        let rect_x = outer_pad;
        
        let font_size = 21.0;
        let bracket_w = self.font_cache.measure_text("[", font_size, false);
        let bt_icon_str = if self.bt_active { "" } else { "󰂲" };
        let bt_icon_w = self.font_cache.measure_text(bt_icon_str, font_size, false);
        let right_bracket_w = self.font_cache.measure_text("]", font_size, false);
        let bt_inner_w = (font_size / 3.0) * 2.0 + bt_icon_w;

        let rect_w = grid::PANEL_WIDTH as f32 - 2.0 * outer_pad;
        let process_slider = |val: &mut f64, x: f32, font_cache: &mut FontCache| -> Option<f64> {
            let slider_start_x = rect_x + 6.0;
            let char_w = font_cache.measure_text("#", font_size, false);
            let available_inner_w = (rect_w - 12.0) - bracket_w * 2.0;
            let slider_len = (available_inner_w / char_w).floor() as usize;
            let slider_w = bracket_w * 2.0 + (slider_len as f32) * char_w;
            
            let inner_start_x = slider_start_x + bracket_w;
            let inner_w = slider_w - bracket_w * 2.0;
            
            let pct = (x - inner_start_x) / inner_w;
            Some(pct.clamp(0.0, 1.0) as f64)
        };

        if self.drag_state == DragState::Brightness {
            if let Some(val) = process_slider(&mut self.brightness, x, &mut self.font_cache) {
                let old_pct = (self.brightness * 100.0).round() as i32;
                let new_pct = (val * 100.0).round() as i32;
                if old_pct != new_pct {
                    api::brightness::set_brightness(val);
                }
                self.brightness = val;
                self.needs_draw = true;
            }
        } else if self.drag_state == DragState::Volume {
            if let Some(val) = process_slider(&mut self.volume, x, &mut self.font_cache) {
                let old_pct = (self.volume * 100.0).round() as i32;
                let new_pct = (val * 100.0).round() as i32;
                if old_pct != new_pct {
                    api::audio::set_volume(val);
                }
                self.volume = val;
                self.needs_draw = true;
            }
        }
    }

    fn handle_pointer_press(&mut self, x: f64, y: f64) {
        let x = x as f32;
        let y = y as f32;

        let outer_pad = 10.0;
        let rect_x = outer_pad;
        let rect_y = outer_pad;
        let rect_w = self.panel_width as f32 - 2.0 * outer_pad;
        let wifi_rect_h = 54.0;
        let bt_rect_h = 54.0;
        let slider_rect_h = 32.0;

        // Check if inside the WiFi row
        if y >= rect_y && y <= rect_y + wifi_rect_h && x >= rect_x && x <= rect_x + rect_w {
            let font_size = 21.0;
            let bracket_w = self.font_cache.measure_text("[", font_size, false);
            
            // 1. Calculate Bluetooth inner width to force alignment
            let bt_icon_str = if self.bt_active { "" } else { "󰂲" };
            let bt_icon_w = self.font_cache.measure_text(bt_icon_str, font_size, false);
            let bt_inner_w = (font_size / 3.0) * 2.0 + bt_icon_w;

            // 2. Calculate WiFi padding to match BT width, keeping the 1.5px left shift
            let icon_str = if self.wifi_active { "󰖩" } else { "󰖪" };
            let icon_w = self.font_cache.measure_text(icon_str, font_size, false);
            
            let base_left = font_size / 3.0; 
            let base_right = self.font_cache.measure_text(" ", font_size, false);
            
            let available_pad = bt_inner_w - icon_w;
            let excess_pad = (base_left + base_right) - available_pad;
            
            let left_pad = base_left - (excess_pad / 2.0) - 1.5;
            let right_pad = base_right - (excess_pad / 2.0) + 1.5;

            let right_bracket_w = self.font_cache.measure_text("]", font_size, false);
            let total_icon_box_w = bracket_w + left_pad + icon_w + right_pad + right_bracket_w;
            
            if x <= rect_x + 6.0 + total_icon_box_w || y < rect_y + 27.0 {
                // Clicked icon or Line 1 -> toggle WiFi
                let new_state = !self.wifi_active;
                api::network::set_wifi_enabled(new_state);
                self.wifi_active = new_state; // Optimistic update
                if !new_state {
                    self.wifi_status = "Off".to_string();
                } else {
                    self.wifi_status = "On".to_string();
                }
                self.needs_draw = true;
            } else {
                // Clicked Line 2 / name -> open wifi menu
                api::network::open_wifi_menu();
                self.hide_panel();
            }
        }

        // Check if inside the Bluetooth row
        let bt_rect_y = rect_y + wifi_rect_h + 10.0;
        if y >= bt_rect_y && y <= bt_rect_y + bt_rect_h && x >= rect_x && x <= rect_x + rect_w {
            let font_size = 21.0;
            let bracket_w = self.font_cache.measure_text("[", font_size, false);
            let bt_icon_str = if self.bt_active { "" } else { "󰂲" };
            let bt_icon_w = self.font_cache.measure_text(bt_icon_str, font_size, false);
            let right_bracket_w = self.font_cache.measure_text("]", font_size, false);
            let bt_pad = font_size / 3.0; 
            let bt_total_icon_box_w = bracket_w + bt_pad + bt_icon_w + bt_pad + right_bracket_w;
            
            if x <= rect_x + 6.0 + bt_total_icon_box_w || y < bt_rect_y + 27.0 {
                // Clicked icon or Line 1 -> toggle bluetooth
                let new_state = !self.bt_active;
                api::bluetooth::set_bluetooth_enabled(new_state);
                self.bt_active = new_state;
                self.needs_draw = true;
            } else {
                // Clicked Line 2 / name -> open bluetooth menu
                api::bluetooth::open_bluetooth_menu();
                self.hide_panel();
            }
        }

        // Check if inside the Brightness row
        let bright_rect_y = bt_rect_y + bt_rect_h + 10.0;
        let bright_rect_h = 54.0;
        let rect_w = grid::PANEL_WIDTH as f32 - 2.0 * outer_pad;
        if y >= bright_rect_y && y <= bright_rect_y + bright_rect_h && x >= rect_x && x <= rect_x + rect_w {
            let font_size = 21.0;
            let bracket_w = self.font_cache.measure_text("[", font_size, false);
            let bt_icon_str = if self.bt_active { "" } else { "󰂲" };
            let bt_icon_w = self.font_cache.measure_text(bt_icon_str, font_size, false);
            let right_bracket_w = self.font_cache.measure_text("]", font_size, false);
            let bt_inner_w = (font_size / 3.0) * 2.0 + bt_icon_w;

            let bright_icon_str = if self.blue_light_active { "󰃟" } else { "󰃠" };
            let bright_icon_w = self.font_cache.measure_text(bright_icon_str, font_size, false);
            let base_left = font_size / 3.0;
            let base_right = self.font_cache.measure_text(" ", font_size, false);
            let available_pad = bt_inner_w - bright_icon_w;
            let excess_pad = (base_left + base_right) - available_pad;
            let bright_left_pad = base_left - (excess_pad / 2.0) - 1.5;
            let bright_right_pad = base_right - (excess_pad / 2.0) + 1.5;
            
            let bright_total_icon_box_w = bracket_w + bright_left_pad + bright_icon_w + bright_right_pad + right_bracket_w;
            
            if y <= bright_rect_y + 24.0 && x <= rect_x + 6.0 + bright_total_icon_box_w {
                // Clicked the Line 1 icon -> toggle blue light
                let new_state = !self.blue_light_active;
                self.blue_light_active = new_state;
                self.backdrop_surface = None;
                self.backdrop_configured = false;
                self.needs_draw = true;
                let s = self.sync_sender.clone();
                std::thread::spawn(move || {
                    api::compositor::set_blue_light_enabled(new_state);
                    std::thread::sleep(std::time::Duration::from_millis(80));
                    let _ = s.send(SyncMessage::RecreateBackdrop);
                });
            } else {
                // Clicked the Line 2 slider
                self.drag_state = DragState::Brightness;
                let char_w = self.font_cache.measure_text("#", font_size, false);
                let available_inner_w = (rect_w - 12.0) - bracket_w * 2.0;
                let slider_len = (available_inner_w / char_w).floor() as usize;
                let slider_w = bracket_w * 2.0 + (slider_len as f32) * char_w;
                let slider_start_x = rect_x + 6.0;
                
                let inner_start_x = slider_start_x + bracket_w;
                let inner_w = slider_w - bracket_w * 2.0;
                
                let pct = (x - inner_start_x) / inner_w;
                let val = pct.clamp(0.0, 1.0) as f64;
                let old_pct = (self.brightness * 100.0).round() as i32;
                let new_pct = (val * 100.0).round() as i32;
                if old_pct != new_pct {
                    api::brightness::set_brightness(val);
                }
                self.brightness = val;
                self.needs_draw = true;
            }
        }

        // Check if inside the Volume row
        let vol_rect_y = bright_rect_y + bright_rect_h + 10.0;
        let vol_rect_h = 54.0;
        if y >= vol_rect_y && y <= vol_rect_y + vol_rect_h && x >= rect_x && x <= rect_x + rect_w {
            let font_size = 21.0;
            let bracket_w = self.font_cache.measure_text("[", font_size, false);
            let bt_icon_str = if self.bt_active { "" } else { "󰂲" };
            let bt_icon_w = self.font_cache.measure_text(bt_icon_str, font_size, false);
            let right_bracket_w = self.font_cache.measure_text("]", font_size, false);
            let bt_inner_w = (font_size / 3.0) * 2.0 + bt_icon_w;

            let vol_icon_str = if self.volume_muted { "󰖁" } else { "󰕾" };
            let vol_icon_w = self.font_cache.measure_text(vol_icon_str, font_size, false);
            let base_left = font_size / 3.0;
            let base_right = self.font_cache.measure_text(" ", font_size, false);
            let available_pad = bt_inner_w - vol_icon_w;
            let excess_pad = (base_left + base_right) - available_pad;
            let vol_left_pad = base_left - (excess_pad / 2.0) - 1.5;
            let vol_right_pad = base_right - (excess_pad / 2.0) + 1.5;
            
            let vol_total_icon_box_w = bracket_w + vol_left_pad + vol_icon_w + vol_right_pad + right_bracket_w;
            
            if y <= vol_rect_y + 24.0 && x <= rect_x + 6.0 + vol_total_icon_box_w {
                // Clicked the Line 1 icon -> toggle mute
                let new_state = !self.volume_muted;
                api::audio::set_mute(new_state);
                self.volume_muted = new_state;
                self.needs_draw = true;
            } else {
                // Clicked the Line 2 slider
                self.drag_state = DragState::Volume;
                let char_w = self.font_cache.measure_text("#", font_size, false);
                let available_inner_w = (rect_w - 12.0) - bracket_w * 2.0;
                let slider_len = (available_inner_w / char_w).floor() as usize;
                let slider_w = bracket_w * 2.0 + (slider_len as f32) * char_w;
                let slider_start_x = rect_x + 6.0;
                
                let inner_start_x = slider_start_x + bracket_w;
                let inner_w = slider_w - bracket_w * 2.0;
                
                let pct = (x - inner_start_x) / inner_w;
                let val = pct.clamp(0.0, 1.0) as f64;
                let old_pct = (self.volume * 100.0).round() as i32;
                let new_pct = (val * 100.0).round() as i32;
                if old_pct != new_pct {
                    api::audio::set_volume(val);
                }
                self.volume = val;
                self.needs_draw = true;
            }
        }
    }

    fn handle_pointer_release(&mut self, _x: f64, _y: f64) {
        self.drag_state = DragState::None;
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
        let mut changed = false;
        match msg {
            SyncMessage::WifiBluetooth { wifi, bt } => {
                if let Some((active, status)) = wifi {
                    if self.wifi_active != active || self.wifi_status != status {
                        self.wifi_active = active;
                        self.wifi_status = status;
                        changed = true;
                    }
                }
                if let Some(active) = bt {
                    if self.bt_active != active {
                        self.bt_active = active;
                        changed = true;
                    }
                }
            }
            SyncMessage::Brightness { brightness, blue_active } => {
                if self.drag_state != DragState::Brightness {
                    if self.brightness != brightness || self.blue_light_active != blue_active {
                        self.brightness = brightness;
                        self.blue_light_active = blue_active;
                        changed = true;
                    }
                }
            }
            SyncMessage::Volume { volume, muted } => {
                if self.drag_state != DragState::Volume {
                    if self.volume != volume || self.volume_muted != muted {
                        self.volume = volume;
                        self.volume_muted = muted;
                        changed = true;
                    }
                }
            }
            SyncMessage::Media(_state) => {
                // Media player will be added later
            }
            SyncMessage::RecreateBackdrop => {
                self.create_backdrop_surface();
            }
        }
        if changed {
            self.request_redraw();
        }
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
            self.panel_width = grid::PANEL_WIDTH as u32;
            self.panel_height = grid::PANEL_HEIGHT as u32;
            self.panel_configured = true;
            self.draw_panel();
        } else if is_backdrop {
            let (w, h) = configure.new_size;
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
                    // No widget hover states to clear yet
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
        panel_width: grid::PANEL_WIDTH as u32,
        panel_height: grid::PANEL_HEIGHT as u32,
        backdrop_width: 0,
        backdrop_height: 0,
        panel_configured: false,
        backdrop_configured: false,
        visible: false,
        needs_draw: false,
        sync_sender: sync_sender.clone(),
        wifi_toggle_time: None,
        bt_toggle_time: None,
        wifi_active: false,
        wifi_status: String::new(),
        bt_active: false,
        brightness: 0.0,
        blue_light_active: false,
        volume: 0.0,
        volume_muted: false,
        drag_state: DragState::None,
        seat: None,
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
