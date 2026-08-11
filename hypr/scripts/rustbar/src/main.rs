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
};
use wayland_client::{
    globals::registry_queue_init,
    protocol::{wl_keyboard, wl_output, wl_pointer, wl_seat, wl_shm, wl_surface},
    Connection, QueueHandle,
};

use calloop::{EventLoop, LoopHandle};
use tiny_skia::{Pixmap, PixmapMut};
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

    _seat: Option<wl_seat::WlSeat>,

    // Buffer pool
    pool: SlotPool,
    layer_surface: Option<LayerSurface>,

    width: u32,
    height: u32,
    configured: bool,

    // App state
    theme: ThemeConfig,
    font_cache: FontCache,
    workspace_state: WorkspaceState,
    click_regions: Vec<ClickRegion>,
    mouse_x: f64,
    mouse_y: f64,
}

impl RustBar {
    pub fn redraw(&mut self, _qh: &QueueHandle<Self>) {
        if !self.configured || self.width == 0 || self.height == 0 {
            return;
        }

        let layer_surface = match self.layer_surface.as_ref() {
            Some(s) => s,
            None => return,
        };

        let width = self.width;
        let height = self.height;
        let stride = width * 4;

        let (buffer, canvas) = match self.pool.create_buffer(
            width as i32,
            height as i32,
            stride as i32,
            wl_shm::Format::Bgra8888,
        ) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("[RustBar] Failed to create SHM buffer: {:?}", e);
                return;
            }
        };

        let mut pixmap = match Pixmap::new(width, height) {
            Some(p) => p,
            None => return,
        };
        pixmap.fill(tiny_skia::Color::TRANSPARENT);

        // 1. Clear background
        fill_rect(&mut pixmap.as_mut(), 0.0, 0.0, width as f32, height as f32, self.theme.bg_base);

        // 2. Draw 1px border around bar
        stroke_rect(&mut pixmap.as_mut(), 0.0, 0.0, width as f32, height as f32, self.theme.waybar_border, 1.0);

        self.click_regions.clear();

        let font_size = 16.0;
        let top_y = (height as f32 - font_size) / 2.0 - 2.0;

        // ── Render Left Group ──
        let mut left_x = 10.0;

        // Workspaces
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
        left_x = next_x + 6.0;

        // Network
        let net_w = network::render_network(&mut pixmap.as_mut(), &mut self.font_cache, &self.theme, left_x, top_y, font_size);
        self.click_regions.push(ClickRegion {
            x_min: left_x,
            x_max: left_x + net_w,
            action: ModuleClickAction::Network,
        });
        left_x += net_w + 6.0;

        // Bluetooth
        let bt_w = bluetooth::render_bluetooth(&mut pixmap.as_mut(), &mut self.font_cache, &self.theme, left_x, top_y, font_size);
        self.click_regions.push(ClickRegion {
            x_min: left_x,
            x_max: left_x + bt_w,
            action: ModuleClickAction::Bluetooth,
        });

        // ── Render Right Group ──
        let mut right_x = width as f32 - 10.0;

        // Control Center icon (rightmost)
        let cc_w = self.font_cache.measure_bracket_tag("", font_size);
        right_x -= cc_w;
        controlcenter::render_controlcenter(&mut pixmap.as_mut(), &mut self.font_cache, &self.theme, right_x, top_y, font_size);
        self.click_regions.push(ClickRegion {
            x_min: right_x,
            x_max: right_x + cc_w,
            action: ModuleClickAction::ControlCenter,
        });
        right_x -= 6.0;

        // Clock
        let clock_w = self.font_cache.measure_bracket_tag("00:00", font_size);
        right_x -= clock_w;
        clock::render_clock(&mut pixmap.as_mut(), &mut self.font_cache, &self.theme, right_x, top_y, font_size);
        self.click_regions.push(ClickRegion {
            x_min: right_x,
            x_max: right_x + clock_w,
            action: ModuleClickAction::Clock,
        });
        right_x -= 6.0;

        // Battery
        let bat_w = self.font_cache.measure_bracket_tag("󰁹 100%", font_size);
        right_x -= bat_w;
        modules::battery::render_battery(&mut pixmap.as_mut(), &mut self.font_cache, &self.theme, right_x, top_y, font_size);
        right_x -= 6.0;

        // Volume
        let vol_w = self.font_cache.measure_bracket_tag("󰕾 100%", font_size);
        right_x -= vol_w;
        volume::render_volume(&mut pixmap.as_mut(), &mut self.font_cache, &self.theme, right_x, top_y, font_size);
        self.click_regions.push(ClickRegion {
            x_min: right_x,
            x_max: right_x + vol_w,
            action: ModuleClickAction::Volume,
        });

        // ── Render Center Group ──
        let center_x_mid = width as f32 / 2.0;

        // Centerpiece
        let cp_w = centerpiece::render_centerpiece(&mut pixmap.as_mut(), &mut self.font_cache, &self.theme, center_x_mid - 80.0, top_y, font_size);
        self.click_regions.push(ClickRegion {
            x_min: center_x_mid - 80.0,
            x_max: center_x_mid - 80.0 + cp_w,
            action: ModuleClickAction::Centerpiece,
        });

        // CPU (Left of centerpiece)
        let _cpu_w = cpu::render_cpu(&mut pixmap.as_mut(), &mut self.font_cache, &self.theme, center_x_mid - 200.0, top_y, font_size);

        // RAM (Right of centerpiece)
        let _ram_w = ram::render_ram(&mut pixmap.as_mut(), &mut self.font_cache, &self.theme, center_x_mid + 90.0, top_y, font_size);

        // Convert RGBA to BGRA for Wayland SHM
        let raw_pixels = pixmap.data();
        let mut bgra_buf = vec![0u8; raw_pixels.len()];
        rgba_to_bgra(raw_pixels, &mut bgra_buf);
        canvas.copy_from_slice(&bgra_buf);

        let wl_surface = layer_surface.wl_surface();
        wl_surface.damage_buffer(0, 0, width as i32, height as i32);
        buffer.attach_to(wl_surface).expect("Failed to attach buffer");
        wl_surface.commit();
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
        if capability == Capability::Pointer && self._seat.is_none() {
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
    fn pointer_frame(&mut self, _: &Connection, qh: &QueueHandle<Self>, _: &wl_pointer::WlPointer, events: &[PointerEvent]) {
        for event in events {
            match event.kind {
                PointerEventKind::Motion { .. } => {
                    self.mouse_x = event.position.0;
                    self.mouse_y = event.position.1;
                }
                PointerEventKind::Press { button, .. } => {
                    if button == 0x110 { // BTN_LEFT
                        self.handle_pointer_click();
                        self.redraw(qh);
                    }
                }
                _ => {}
            }
        }
    }
}

impl LayerShellHandler for RustBar {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &LayerSurface) {}
    fn configure(&mut self, _: &Connection, qh: &QueueHandle<Self>, _layer_surface: &LayerSurface, configure: LayerSurfaceConfigure, _: u32) {
        self.width = configure.new_size.0.max(1);
        self.height = configure.new_size.1.max(34);
        self.configured = true;
        self.redraw(qh);
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
    let (globals, mut event_queue) = registry_queue_init(&conn).expect("Failed to initialize Wayland registry");
    let qh = event_queue.handle();

    let compositor = CompositorState::bind(&globals, &qh).expect("wl_compositor unavaliable");
    let layer_shell = LayerShell::bind(&globals, &qh).expect("zwlr_layer_shell_v1 unavailable");
    let shm = Shm::bind(&globals, &qh).expect("wl_shm unavailable");

    let pool = SlotPool::new(1920 * 40 * 4, &shm).expect("Failed to create SlotPool");

    let surface = compositor.create_surface(&qh);
    let layer_surface = layer_shell.create_layer_surface(
        &qh,
        surface,
        Layer::Top,
        Some("rustbar"),
        None,
    );

    layer_surface.set_anchor(Anchor::TOP | Anchor::LEFT | Anchor::RIGHT);
    layer_surface.set_size(0, 36);
    layer_surface.set_margin(10, 10, 0, 10);
    layer_surface.set_exclusive_zone(36);
    layer_surface.set_keyboard_interactivity(KeyboardInteractivity::None);
    layer_surface.commit();

    let mut app = RustBar {
        registry_state: RegistryState::new(&globals),
        seat_state: SeatState::new(&globals, &qh),
        output_state: OutputState::new(&globals, &qh),
        compositor_state: compositor,
        shm,
        layer_shell,
        _seat: None,
        pool,
        layer_surface: Some(layer_surface),
        width: 0,
        height: 36,
        configured: false,
        theme: ThemeConfig::load(),
        font_cache: FontCache::new(),
        workspace_state: get_workspace_state(),
        click_regions: Vec::new(),
        mouse_x: 0.0,
        mouse_y: 0.0,
    };

    let mut event_loop: EventLoop<RustBar> = EventLoop::try_new().expect("Failed to create Calloop event loop");
    let loop_handle = event_loop.handle();

    let (tx, rx) = calloop::channel::channel::<()>();
    loop_handle.insert_source(rx, |_, _, state: &mut RustBar| {
        state.workspace_state = get_workspace_state();
        // Dynamic theme reload check
        state.theme = ThemeConfig::load();
    }).expect("Failed to insert channel source");

    let tx_clone = tx.clone();
    listen_hyprland_events(move || {
        let _ = tx_clone.send(());
    });

    // 1-second interval timer for stat updates
    let timer = calloop::timer::Timer::from_duration(Duration::from_secs(1));
    loop_handle.insert_source(timer, move |_, _, state: &mut RustBar| {
        state.redraw(&qh);
        calloop::timer::TimeoutAction::ToDuration(Duration::from_secs(1))
    }).expect("Failed to insert timer source");

    loop {
        event_queue.dispatch_pending(&mut app).unwrap();
        event_loop.dispatch(Some(Duration::from_millis(50)), &mut app).unwrap();
        event_queue.flush().unwrap();
    }
}
