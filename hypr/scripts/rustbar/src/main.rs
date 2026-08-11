//! ============================================================================
//! RUSTBAR - GTK-less Wayland Status Bar in Rust
//! ============================================================================

use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_keyboard, delegate_layer, delegate_output,
    delegate_pointer, delegate_registry, delegate_seat, delegate_shm,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers},
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
    reexports::calloop_wayland_source::WaylandSource,
};
use wayland_client::{
    globals::registry_queue_init,
    protocol::{wl_keyboard, wl_output, wl_pointer, wl_seat, wl_shm, wl_surface},
    Connection, QueueHandle,
};

use calloop::EventLoop;
use tiny_skia::Pixmap;
use std::time::Duration;

mod api;
mod modules;
mod render;
mod theme;

use api::hyprland::{get_workspace_state, listen_hyprland_events, switch_workspace, WorkspaceState};
use modules::{
    bluetooth, centerpiece, clock, controlcenter, cpu, network, ram, volume, workspaces::render_workspaces,
};
use render::{fill_rect, rgba_to_bgra, stroke_rect, FontCache};
use theme::ThemeConfig;

#[derive(Debug, Clone, Copy)]
enum ModuleClickAction {
    Workspace(i32),
    Network,
    Bluetooth,
    Centerpiece,
    Volume,
    Clock,
    ControlCenter,
}

struct ClickRegion {
    x_min: f32,
    x_max: f32,
    action: ModuleClickAction,
}

struct RustBar {
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,
    compositor_state: CompositorState,
    shm: Shm,
    layer_shell: LayerShell,
    _qh: QueueHandle<Self>,

    _seat: Option<wl_seat::WlSeat>,

    // Buffer pool
    pool: SlotPool,
    layer_surface: Option<LayerSurface>,

    width: u32,
    height: u32,
    configured: bool,
    needs_draw: bool,

    // App state
    theme: ThemeConfig,
    font_cache: FontCache,
    workspace_state: WorkspaceState,
    click_regions: Vec<ClickRegion>,
    mouse_x: f64,
    mouse_y: f64,
}

impl RustBar {
    pub fn redraw(&mut self) {
        if !self.configured {
            return;
        }

        let layer_surface = match self.layer_surface.as_ref() {
            Some(s) => s,
            None => return,
        };

        let width = if self.width > 10 { self.width } else { 1900 };
        let height = if self.height > 0 { self.height } else { 36 };
        let stride = width * 4;

        let (buffer, canvas) = match self.pool.create_buffer(
            width as i32,
            height as i32,
            stride as i32,
            wl_shm::Format::Argb8888,
        ) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("[RustBar] Failed to create SHM buffer ({}x{}): {:?}", width, height, e);
                return;
            }
        };

        let mut pixmap = match Pixmap::new(width, height) {
            Some(p) => p,
            None => return,
        };
        pixmap.fill(tiny_skia::Color::TRANSPARENT);

        // 1. Fill background box matching GTK window#waybar
        fill_rect(&mut pixmap.as_mut(), 0.0, 0.0, width as f32, height as f32, self.theme.bg_base);

        // 2. Draw 1px border around bar matching GTK border: 1px solid @waybar-border (if not transparent)
        if self.theme.waybar_border != tiny_skia::Color::TRANSPARENT {
            stroke_rect(&mut pixmap.as_mut(), 0.0, 0.0, width as f32, height as f32, self.theme.waybar_border, 1.0);
        }

        self.click_regions.clear();

        // GTK font-size: 16px, modules margin: 6px 6px
        let font_size = 16.0;
        let box_h = font_size + 8.0; // 24px box height
        let top_y = (height as f32 - box_h) / 2.0; // 6.0px top margin

        // ── Render Left Group (margin: 6px 6px) ──
        let mut left_x = 12.0;

        // Workspaces (#workspaces margin-right: 2px)
        let (next_x, buttons) = render_workspaces(
            &mut pixmap.as_mut(),
            &mut self.font_cache,
            &self.workspace_state,
            &self.theme,
            left_x,
            top_y,
            font_size,
        );
        for btn in buttons {
            self.click_regions.push(ClickRegion {
                x_min: btn.x,
                x_max: btn.x + btn.width,
                action: ModuleClickAction::Workspace(btn.id),
            });
        }
        left_x = next_x + 2.0;

        // Network (#network margin-left: 2px, margin-right: 2px)
        left_x += 2.0;
        let net_w = network::render_network(&mut pixmap.as_mut(), &mut self.font_cache, &self.theme, left_x, top_y, font_size);
        self.click_regions.push(ClickRegion {
            x_min: left_x,
            x_max: left_x + net_w,
            action: ModuleClickAction::Network,
        });
        left_x += net_w + 2.0;

        // Bluetooth (#bluetooth margin-left: 2px)
        left_x += 2.0;
        let bt_w = bluetooth::render_bluetooth(&mut pixmap.as_mut(), &mut self.font_cache, &self.theme, left_x, top_y, font_size);
        self.click_regions.push(ClickRegion {
            x_min: left_x,
            x_max: left_x + bt_w,
            action: ModuleClickAction::Bluetooth,
        });

        // ── Render Right Group ──
        let mut right_x = width as f32 - 12.0;

        // Control Center icon (rightmost, margin-left: 4px)
        let cc_w = self.font_cache.measure_gtk_box("[  ]", font_size, 0.0);
        right_x -= cc_w;
        controlcenter::render_controlcenter(&mut pixmap.as_mut(), &mut self.font_cache, &self.theme, right_x, top_y, font_size);
        self.click_regions.push(ClickRegion {
            x_min: right_x,
            x_max: right_x + cc_w,
            action: ModuleClickAction::ControlCenter,
        });
        right_x -= 4.0;

        // Clock (#clock margin-left: 4px)
        let clock_w = clock::render_clock(&mut pixmap.as_mut(), &mut self.font_cache, &self.theme, 0.0, -100.0, font_size);
        right_x -= clock_w;
        clock::render_clock(&mut pixmap.as_mut(), &mut self.font_cache, &self.theme, right_x, top_y, font_size);
        self.click_regions.push(ClickRegion {
            x_min: right_x,
            x_max: right_x + clock_w,
            action: ModuleClickAction::Clock,
        });
        right_x -= 4.0;

        // Battery (#battery margin-left: 4px)
        let bat_w = modules::battery::render_battery(&mut pixmap.as_mut(), &mut self.font_cache, &self.theme, 0.0, -100.0, font_size);
        right_x -= bat_w;
        modules::battery::render_battery(&mut pixmap.as_mut(), &mut self.font_cache, &self.theme, right_x, top_y, font_size);
        right_x -= 4.0;

        // Volume (#custom-volume margin-left: 4px)
        let vol_w = volume::render_volume(&mut pixmap.as_mut(), &mut self.font_cache, &self.theme, 0.0, -100.0, font_size);
        right_x -= vol_w;
        volume::render_volume(&mut pixmap.as_mut(), &mut self.font_cache, &self.theme, right_x, top_y, font_size);
        self.click_regions.push(ClickRegion {
            x_min: right_x,
            x_max: right_x + vol_w,
            action: ModuleClickAction::Volume,
        });

        // ── Render Center Group ──
        let center_x_mid = width as f32 / 2.0;

        // Measure centerpiece, cpu, ram for 1:1 GTK center layout
        let cp_w = centerpiece::render_centerpiece(&mut pixmap.as_mut(), &mut self.font_cache, &self.theme, 0.0, -100.0, font_size);
        let cpu_w = cpu::render_cpu(&mut pixmap.as_mut(), &mut self.font_cache, &self.theme, 0.0, -100.0, font_size);
        let ram_w = ram::render_ram(&mut pixmap.as_mut(), &mut self.font_cache, &self.theme, 0.0, -100.0, font_size);

        // margins: cpu (margin-right: 2px), centerpiece (margin-left: 2px, margin-right: 2px), memory (margin-left: 2px)
        let total_center_w = cpu_w + 4.0 + cp_w + 4.0 + ram_w;
        let mut center_start_x = center_x_mid - (total_center_w / 2.0);

        // Render CPU (#cpu margin-right: 2px)
        cpu::render_cpu(&mut pixmap.as_mut(), &mut self.font_cache, &self.theme, center_start_x, top_y, font_size);
        center_start_x += cpu_w + 4.0;

        // Render Centerpiece (#custom-centerpiece margin-left: 2px, margin-right: 2px)
        centerpiece::render_centerpiece(&mut pixmap.as_mut(), &mut self.font_cache, &self.theme, center_start_x, top_y, font_size);
        self.click_regions.push(ClickRegion {
            x_min: center_start_x,
            x_max: center_start_x + cp_w,
            action: ModuleClickAction::Centerpiece,
        });
        center_start_x += cp_w + 4.0;

        // Render RAM (#custom-memory margin-left: 2px)
        ram::render_ram(&mut pixmap.as_mut(), &mut self.font_cache, &self.theme, center_start_x, top_y, font_size);

        // Copy RGBA to BGRA buffer
        rgba_to_bgra(pixmap.data(), canvas);

        let wl_surface = layer_surface.wl_surface();
        wl_surface.attach(Some(buffer.wl_buffer()), 0, 0);
        wl_surface.damage_buffer(0, 0, width as i32, height as i32);
        wl_surface.commit();
        self.needs_draw = false;
    }

    pub fn handle_pointer_click(&mut self) {
        let x = self.mouse_x as f32;
        for region in &self.click_regions {
            if x >= region.x_min && x <= region.x_max {
                match region.action {
                    ModuleClickAction::Workspace(id) => switch_workspace(id),
                    ModuleClickAction::Network => network::handle_click(),
                    ModuleClickAction::Bluetooth => bluetooth::handle_click(),
                    ModuleClickAction::Centerpiece => centerpiece::handle_click(),
                    ModuleClickAction::Volume => volume::handle_click(),
                    ModuleClickAction::Clock => clock::handle_click(),
                    ModuleClickAction::ControlCenter => controlcenter::handle_click(),
                }
                break;
            }
        }
    }
}

// ── Wayland Handler Implementations ──────────────────────────────────────────

impl CompositorHandler for RustBar {
    fn scale_factor_changed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: i32) {}
    fn transform_changed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: wl_output::Transform) {}
    fn frame(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: u32) {}
    fn surface_enter(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: &wl_output::WlOutput) {}
    fn surface_leave(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: &wl_output::WlOutput) {}
}

impl OutputHandler for RustBar {
    fn output_state(&mut self) -> &mut OutputState { &mut self.output_state }
    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
}

impl ShmHandler for RustBar {
    fn shm_state(&mut self) -> &mut Shm { &mut self.shm }
}

impl SeatHandler for RustBar {
    fn seat_state(&mut self) -> &mut SeatState { &mut self.seat_state }
    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
    fn new_capability(&mut self, _conn: &Connection, qh: &QueueHandle<Self>, seat: wl_seat::WlSeat, capability: Capability) {
        if capability == Capability::Pointer {
            let _ = self.seat_state.get_pointer(qh, &seat);
            self._seat = Some(seat);
        }
    }
    fn remove_capability(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat, _: Capability) {}
    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl KeyboardHandler for RustBar {
    fn enter(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: &wl_surface::WlSurface, _: u32, _: &[u32], _: &[Keysym]) {}
    fn leave(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: &wl_surface::WlSurface, _: u32) {}
    fn press_key(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: u32, _: KeyEvent) {}
    fn release_key(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: u32, _: KeyEvent) {}
    fn update_modifiers(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: u32, _: Modifiers, _: u32) {}
}

impl PointerHandler for RustBar {
    fn pointer_frame(&mut self, _: &Connection, _qh: &QueueHandle<Self>, _: &wl_pointer::WlPointer, events: &[PointerEvent]) {
        for event in events {
            match event.kind {
                PointerEventKind::Motion { .. } => {
                    self.mouse_x = event.position.0;
                    self.mouse_y = event.position.1;
                }
                PointerEventKind::Press { button, .. } => {
                    if button == 0x110 { // BTN_LEFT
                        self.mouse_x = event.position.0;
                        self.mouse_y = event.position.1;
                        self.handle_pointer_click();
                        self.needs_draw = true;
                    }
                }
                _ => {}
            }
        }
    }
}

impl LayerShellHandler for RustBar {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &LayerSurface) {}
    fn configure(&mut self, _: &Connection, _qh: &QueueHandle<Self>, _layer_surface: &LayerSurface, configure: LayerSurfaceConfigure, _: u32) {
        let (mut w, mut h) = configure.new_size;
        if w == 0 {
            w = 1900;
        }
        if h == 0 {
            h = 36;
        }
        self.width = w;
        self.height = h;
        self.configured = true;
        self.needs_draw = true;
        eprintln!("[RustBar] Configured layer surface size: {}x{}", self.width, self.height);
    }
}

delegate_compositor!(RustBar);
delegate_output!(RustBar);
delegate_shm!(RustBar);
delegate_seat!(RustBar);
delegate_keyboard!(RustBar);
delegate_pointer!(RustBar);
delegate_layer!(RustBar);
delegate_registry!(RustBar);

impl ProvidesRegistryState for RustBar {
    fn registry(&mut self) -> &mut RegistryState { &mut self.registry_state }
    registry_handlers![OutputState, SeatState];
}

fn main() {
    let conn = Connection::connect_to_env().expect("Failed to connect to Wayland display");
    let (globals, event_queue) = registry_queue_init(&conn).expect("Failed to initialize Wayland registry");
    let qh = event_queue.handle();

    let mut event_loop: EventLoop<RustBar> = EventLoop::try_new().expect("Failed to create Calloop event loop");
    let loop_handle = event_loop.handle();

    WaylandSource::new(conn.clone(), event_queue)
        .insert(loop_handle.clone())
        .expect("Failed to insert WaylandSource into EventLoop");

    let compositor = CompositorState::bind(&globals, &qh).expect("wl_compositor unavailable");
    let layer_shell = LayerShell::bind(&globals, &qh).expect("zwlr_layer_shell_v1 unavailable");
    let shm = Shm::bind(&globals, &qh).expect("wl_shm unavailable");

    let pool = SlotPool::new(3840 * 100 * 4, &shm).expect("Failed to create SlotPool");

    let mut app = RustBar {
        registry_state: RegistryState::new(&globals),
        seat_state: SeatState::new(&globals, &qh),
        output_state: OutputState::new(&globals, &qh),
        compositor_state: compositor,
        shm,
        layer_shell,
        _qh: qh.clone(),
        _seat: None,
        pool,
        layer_surface: None,
        width: 1900,
        height: 36,
        configured: false,
        needs_draw: false,
        theme: ThemeConfig::load(),
        font_cache: FontCache::new(),
        workspace_state: get_workspace_state(),
        click_regions: Vec::new(),
        mouse_x: 0.0,
        mouse_y: 0.0,
    };

    // Bind pointer for all seats initialized in registry
    for seat in app.seat_state.seats() {
        let _ = app.seat_state.get_pointer(&qh, &seat);
        app._seat = Some(seat);
    }

    // Create LayerSurface on app instance
    let surface = app.compositor_state.create_surface(&qh);
    let layer_surface = app.layer_shell.create_layer_surface(
        &qh,
        surface,
        Layer::Top,
        Some("rustbar"),
        None,
    );

    layer_surface.set_anchor(Anchor::TOP | Anchor::LEFT | Anchor::RIGHT);
    layer_surface.set_size(1900, 36);
    layer_surface.set_margin(10, 10, 0, 10);
    layer_surface.set_exclusive_zone(36);
    layer_surface.set_keyboard_interactivity(KeyboardInteractivity::None);
    layer_surface.commit();

    app.layer_surface = Some(layer_surface);

    let (tx, rx) = calloop::channel::channel::<()>();
    loop_handle.insert_source(rx, |_, _, state: &mut RustBar| {
        state.workspace_state = get_workspace_state();
        state.theme = ThemeConfig::load();
        state.needs_draw = true;
    }).expect("Failed to insert channel source");

    let tx_clone = tx.clone();
    listen_hyprland_events(move || {
        let _ = tx_clone.send(());
    });

    // 1-second interval timer for stat updates and dynamic theme reloading
    let timer = calloop::timer::Timer::from_duration(Duration::from_secs(1));
    loop_handle.insert_source(timer, move |_, _, state: &mut RustBar| {
        let new_theme = ThemeConfig::load();
        if new_theme != state.theme {
            state.theme = new_theme;
            state.needs_draw = true;
        }
        calloop::timer::TimeoutAction::ToDuration(Duration::from_secs(1))
    }).expect("Failed to insert timer source");

    loop {
        let res = event_loop.dispatch(Duration::from_millis(16), &mut app);
        if let Err(e) = res {
            eprintln!("[RustBar] EventLoop error: {:?}", e);
            break;
        }

        if app.configured && app.needs_draw {
            app.redraw();
        }
    }
}
