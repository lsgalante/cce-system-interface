use std::sync::Arc;

use clear_ui::color;
use clear_ui::widget::{Spinbox, Widget, Finger, Slider, hover_animation, TextItem};
use glyphon::{
    Attrs, Buffer, Cache, FontSystem, Metrics, Resolution, SwashCache, TextArea, TextAtlas,
    TextBounds, TextRenderer, Viewport,
};

use clear_system_interface::app::{AppAction, AppState, ContentButton, PageContent};
use clear_system_interface::pages::{self, Page};

use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_keyboard, delegate_pointer, delegate_registry,
    delegate_seat, delegate_shm, delegate_xdg_shell, delegate_xdg_window, delegate_output,
    registry::{ProvidesRegistryState, RegistryState},
    output::{OutputHandler, OutputState},
    seat::{
        keyboard::KeyboardHandler,
        pointer::{PointerHandler, ThemedPointer, ThemeSpec, CursorIcon},
        Capability, SeatHandler, SeatState,
    },
    shell::{
        xdg::{
            window::{Window as XdgWindow, WindowConfigure, WindowHandler, WindowDecorations},
            XdgShell,
        },
        WaylandSurface,
    },
    shm::{Shm, ShmHandler},
};
use wayland_client::{
    globals::registry_queue_init,
    protocol::{wl_keyboard, wl_output, wl_pointer, wl_seat, wl_shm, wl_surface},
    Connection, QueueHandle, Proxy,
};
use calloop::EventLoop;
use calloop_wayland_source::WaylandSource;

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 4],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
        0 => Float32x2,
        1 => Float32x4,
    ];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

fn quad_vertices(x: f32, y: f32, w: f32, h: f32, sw: f32, sh: f32, c: [f32; 4]) -> [Vertex; 6] {
    let x0 = (x / sw) * 2.0 - 1.0;
    let y0 = 1.0 - (y / sh) * 2.0;
    let x1 = ((x + w) / sw) * 2.0 - 1.0;
    let y1 = 1.0 - ((y + h) / sh) * 2.0;
    [
        Vertex { position: [x0, y0], color: c },
        Vertex { position: [x1, y0], color: c },
        Vertex { position: [x0, y1], color: c },
        Vertex { position: [x1, y0], color: c },
        Vertex { position: [x1, y1], color: c },
        Vertex { position: [x0, y1], color: c },
    ]
}

fn make_text_buffer(fs: &mut FontSystem, text: &str, size: f32) -> Buffer {
    let metrics = Metrics::new(size, size * 1.4);
    let mut buf = Buffer::new(fs, metrics);
    buf.set_text(fs, text, Attrs::new(), glyphon::Shaping::Advanced);
    buf.shape_until_scroll(fs, true);
    buf
}

fn find_cased_family(fs: &FontSystem, name: &str) -> Option<String> {
    let lower_name = name.to_lowercase();
    for face in fs.db().faces() {
        for (family, _) in &face.families {
            if family.to_lowercase() == lower_name {
                return Some(family.clone());
            }
        }
    }
    None
}

fn make_text_buffer_with_font(
    fs: &mut FontSystem,
    text: &str,
    size: f32,
    font: Option<&str>,
    sans_fallback: &str,
    serif_fallback: &str,
    mono_fallback: &str,
) -> Buffer {
    let metrics = Metrics::new(size, size * 1.4);
    let mut buf = Buffer::new(fs, metrics);
    let mut attrs = Attrs::new();
    let mut resolved_storage = None;
    if let Some(font_name) = font {
        let family = match font_name {
            "monospace" => {
                if !mono_fallback.is_empty() {
                    resolved_storage = find_cased_family(fs, mono_fallback);
                    if let Some(ref cased) = resolved_storage {
                        glyphon::Family::Name(cased)
                    } else {
                        glyphon::Family::Name(mono_fallback)
                    }
                } else {
                    glyphon::Family::Monospace
                }
            }
            "sans-serif" => {
                if !sans_fallback.is_empty() {
                    resolved_storage = find_cased_family(fs, sans_fallback);
                    if let Some(ref cased) = resolved_storage {
                        glyphon::Family::Name(cased)
                    } else {
                        glyphon::Family::Name(sans_fallback)
                    }
                } else {
                    glyphon::Family::SansSerif
                }
            }
            "serif" => {
                if !serif_fallback.is_empty() {
                    resolved_storage = find_cased_family(fs, serif_fallback);
                    if let Some(ref cased) = resolved_storage {
                        glyphon::Family::Name(cased)
                    } else {
                        glyphon::Family::Name(serif_fallback)
                    }
                } else {
                    glyphon::Family::Serif
                }
            }
            _ => glyphon::Family::Name(font_name),
        };
        attrs = attrs.family(family);
    }
    buf.set_text(fs, text, attrs, glyphon::Shaping::Advanced);
    buf.shape_until_scroll(fs, true);
    buf
}

struct AppWidget {
    x: f32, y: f32, w: f32, h: f32,
    color: [f32; 4],
    hover_color: [f32; 4],
    hovering: bool,
    kind: WidgetKind,
}

#[derive(Clone)]
enum WidgetKind {
    ActionButton(AppAction),
    Static,
}

enum ColorSelectorAction {
    Background([u8; 3]),
    Border([u8; 3]),
}

static INITIAL_PAGE_INDEX: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

struct SystemInterface {
    app: AppState,
    font_system: FontSystem,
    widgets: Vec<AppWidget>,
    text_items: Vec<clear_ui::widget::TextItem>,
    page_buttons: Vec<ContentButton>,

    sidebar_width: f32,
    header_height: f32,
    status_height: f32,

    cursor_x: f32,
    cursor_y: f32,

    rx_audio: std::sync::mpsc::Receiver<pages::audio::AudioState>,
    rx_display: std::sync::mpsc::Receiver<pages::display::DisplayState>,
    rx_network: std::sync::mpsc::Receiver<pages::network::NetworkState>,
    rx_layout: std::sync::mpsc::Receiver<pages::layout::LayoutState>,
    rx_wm_events: std::sync::mpsc::Receiver<()>,
    rx_input: std::sync::mpsc::Receiver<pages::input::InputState>,
    rx_fingers: std::sync::mpsc::Receiver<Vec<Finger>>,
    rx_hardware: std::sync::mpsc::Receiver<pages::hardware::HardwareState>,
    rx_system: std::sync::mpsc::Receiver<pages::system_info::SystemState>,
    rx_status: std::sync::mpsc::Receiver<pages::status::StatusState>,
    rx_storage: std::sync::mpsc::Receiver<pages::storage::StorageState>,
    rx_notifications: std::sync::mpsc::Receiver<pages::notifications::NotificationsState>,
    rx_backup_state: std::sync::mpsc::Receiver<pages::backup::BackupState>,
    rx_typeface: std::sync::mpsc::Receiver<pages::typeface::TypefaceState>,
    rx_services: std::sync::mpsc::Receiver<Vec<pages::services::ServiceInfo>>,
    rx_colors: std::sync::mpsc::Receiver<pages::colors::ColorsState>,
    tx_backup: std::sync::mpsc::Sender<pages::backup::BackupMessage>,
    rx_backup: std::sync::mpsc::Receiver<pages::backup::BackupMessage>,
    tx_color_selector: std::sync::mpsc::Sender<ColorSelectorAction>,
    rx_color_selector: std::sync::mpsc::Receiver<ColorSelectorAction>,

    scale_factor: f64,
    width: u32,
    height: u32,
    needs_rebuild: bool,
    scroll_y: f32,
    max_scroll_y: f32,
    opacity_dragging: bool,
    page_root_container: clear_ui::widget::Container,
    page_sec_containers: Vec<clear_ui::widget::Container>,
    paginator: clear_ui::widget::Paginator,
    sans_serif_family: String,
    serif_family: String,
    monospace_family: String,
    sender: calloop::channel::Sender<AppAction>,
}

impl clear_ui::engine::Application for SystemInterface {
    type Message = AppAction;

    fn new(_qh: &wayland_client::QueueHandle<clear_ui::engine::EngineState<Self>>, sender: calloop::channel::Sender<Self::Message>) -> Self {
        let app = AppState {
            layout: pages::layout::read_layout_config(),
            input: pages::input::read_input_config(),
            ..Default::default()
        };

        // ── Background refresh channels ──
        fn spawn_bg<T, F>(period_secs: u64, f: fn() -> F) -> std::sync::mpsc::Receiver<T>
        where
            T: Send + 'static,
            F: std::future::Future<Output = T> + Send + 'static,
        {
            let (tx, rx) = std::sync::mpsc::channel::<T>();
            tokio::spawn(async move {
                loop {
                    let val = f().await;
                    if tx.send(val).is_err() { break; }
                    tokio::time::sleep(std::time::Duration::from_secs(period_secs)).await;
                }
            });
            rx
        }

        let rx_audio = spawn_bg(3, || pages::audio::fetch_audio_state());
        let rx_display = spawn_bg(10, || pages::display::fetch_display_state());
        let rx_network = spawn_bg(5, || pages::network::fetch_network_state());
        let rx_layout = {
            let (tx, rx) = std::sync::mpsc::channel::<pages::layout::LayoutState>();
            tokio::spawn(async move {
                loop {
                    let val = tokio::task::spawn_blocking(|| pages::layout::read_layout_config()).await;
                    if let Ok(val) = val { if tx.send(val).is_err() { break; } }
                    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                }
            });
            rx
        };
        let rx_wm_events = {
            let (tx, rx) = std::sync::mpsc::channel::<()>();
            tokio::spawn(async move {
                let display = std::env::var("WAYLAND_DISPLAY").unwrap_or_else(|_| "wayland-0".to_string());
                let windows_path = format!("/tmp/ccec-windows-{}", display);
                let tags_path = format!("/tmp/ccec-tags-{}", display);
                let title_path = format!("/tmp/ccec-title-{}", display);
                
                let mut last_mod = std::time::SystemTime::UNIX_EPOCH;
                
                let check_mtime = |path: &str| -> Option<std::time::SystemTime> {
                    std::fs::metadata(path).and_then(|m| m.modified()).ok()
                };

                loop {
                    let mut changed = false;
                    for p in &[&windows_path, &tags_path, &title_path] {
                        if let Some(mtime) = check_mtime(p) {
                            if mtime > last_mod {
                                last_mod = mtime;
                                changed = true;
                            }
                        }
                    }
                    if changed {
                        if tx.send(()).is_err() { break; }
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
            });
            rx
        };
        let rx_input = {
            let (tx, rx) = std::sync::mpsc::channel::<pages::input::InputState>();
            tokio::spawn(async move {
                loop {
                    let val = tokio::task::spawn_blocking(|| pages::input::read_input_config()).await;
                    if let Ok(val) = val { if tx.send(val).is_err() { break; } }
                    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                }
            });
            rx
        };
        let rx_fingers = {
            let (tx, rx) = std::sync::mpsc::channel::<Vec<Finger>>();
            tokio::spawn(async move {
                let socket_path = "/tmp/clear-input-coords.sock";
                loop {
                    if let Ok(stream) = tokio::net::UnixStream::connect(socket_path).await {
                        use tokio::io::AsyncBufReadExt;
                        let reader = tokio::io::BufReader::new(stream);
                        let mut lines = reader.lines();
                        while let Ok(Some(line)) = lines.next_line().await {
                            if let Ok(fingers) = serde_json::from_str::<Vec<Finger>>(&line) {
                                if tx.send(fingers).is_err() {
                                    return;
                                }
                            }
                        }
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
            });
            rx
        };
        let rx_system = spawn_bg(5, || pages::system_info::fetch_system_state());
        let rx_hardware = spawn_bg(3, || pages::hardware::fetch_hardware_state());
        let rx_status = spawn_bg(10, || pages::status::fetch_status_state());
        let rx_storage = spawn_bg(10, || pages::storage::fetch_storage_state());
        let rx_notifications = {
            let (tx, rx) = std::sync::mpsc::channel::<pages::notifications::NotificationsState>();
            tokio::spawn(async move {
                loop {
                    let val = tokio::task::spawn_blocking(|| pages::notifications::read_notifications_config()).await;
                    if let Ok(val) = val { if tx.send(val).is_err() { break; } }
                    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                }
            });
            rx
        };
        let rx_backup_state = spawn_bg(30, || pages::backup::fetch_backup_state());
        let rx_typeface = spawn_bg(30, || pages::typeface::fetch_typeface_state());
        let rx_services = spawn_bg(3, || pages::services::fetch_services());
        let rx_colors = {
            let (tx, rx) = std::sync::mpsc::channel::<pages::colors::ColorsState>();
            tokio::spawn(async move {
                loop {
                    let val = tokio::task::spawn_blocking(|| pages::colors::read_colors_config()).await;
                    if let Ok(val) = val { if tx.send(val).is_err() { break; } }
                    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                }
            });
            rx
        };
        let (tx_backup, rx_backup) = std::sync::mpsc::channel();
        let (tx_color_selector, rx_color_selector) = std::sync::mpsc::channel();

        let (sans_family, serif_family, monospace_family, _, _, _, _) = pages::typeface::read_preferred_fonts();

        let pages_names = Page::ALL.iter().map(|p| p.label().to_string()).collect::<Vec<_>>();
        let paginator = clear_ui::widget::Paginator::new(56.0, pages_names)
            .with_tabs_rotated(true);

        let initial_page_idx = INITIAL_PAGE_INDEX.load(std::sync::atomic::Ordering::SeqCst);
        let mut app_state = app;
        app_state.current_page = Page::ALL[initial_page_idx];

        let font_system = FontSystem::new();

        let mut this = Self {
            app: app_state,
            font_system,
            widgets: Vec::new(),
            text_items: Vec::new(),
            page_buttons: Vec::new(),
            sidebar_width: 56.0,
            header_height: 0.0,
            status_height: 0.0,
            cursor_x: 0.0,
            cursor_y: 0.0,
            rx_audio,
            rx_display,
            rx_network,
            rx_layout,
            rx_wm_events,
            rx_input,
            rx_fingers,
            rx_hardware,
            rx_system,
            rx_status,
            rx_storage,
            rx_notifications,
            rx_backup_state,
            rx_typeface,
            rx_services,
            rx_colors,
            tx_backup,
            rx_backup,
            tx_color_selector,
            rx_color_selector,
            scale_factor: 1.0,
            width: 820,
            height: 680,
            needs_rebuild: true,
            scroll_y: 0.0,
            max_scroll_y: 0.0,
            opacity_dragging: false,
            page_root_container: clear_ui::widget::Container::new(),
            page_sec_containers: Vec::new(),
            paginator,
            sans_serif_family: sans_family,
            serif_family,
            monospace_family,
            sender,
        };
        this.rebuild_layout(820.0, 680.0);
        this
    }

    fn settings(&self) -> clear_ui::engine::WindowSettings {
        clear_ui::engine::WindowSettings {
            title: "Clear System Interface".to_string(),
            app_id: "clear-system-interface".to_string(),
            width: 820,
            height: 680,
            fullscreen: false,
            min_size: Some((820, 680)),
        }
    }

    fn update(&mut self, msg: Self::Message, needs_rebuild: &mut bool, _exit: &mut bool) {
        self.handle_action(&msg);
        *needs_rebuild = true;
        self.needs_rebuild = true;
    }

    fn tick(&mut self, dt: f32, needs_rebuild: &mut bool) {
        self.poll_background_updates();
        if self.tick_internal(dt) {
            *needs_rebuild = true;
            self.needs_rebuild = true;
        }
        if self.needs_rebuild {
            *needs_rebuild = true;
        }
    }

    fn view(&mut self, quads: &mut Vec<(f32, f32, f32, f32, [f32; 4])>, size: clear_ui::engine::LogicalSize, scale: f64) {
        let (width, height) = (size.width, size.height);
        if self.needs_rebuild || self.width != width as u32 || self.height != height as u32 || self.scale_factor != scale {
            self.width = width as u32;
            self.height = height as u32;
            self.scale_factor = scale;
            self.rebuild_layout(width, height);
        }
        for w in &self.widgets {
            quads.push((w.x, w.y, w.w, w.h, w.color));
        }
    }

    fn text_items(&self) -> &[clear_ui::widget::TextItem] {
        &self.text_items
    }

    fn clear_color(&self) -> [f32; 4] {
        [
            self.app.colors.page_low_color[0] as f32 / 255.0,
            self.app.colors.page_low_color[1] as f32 / 255.0,
            self.app.colors.page_low_color[2] as f32 / 255.0,
            1.0,
        ]
    }

    fn handle_pointer_move(&mut self, pos: clear_ui::engine::LogicalPosition, needs_rebuild: &mut bool) {
        if self.handle_cursor_moved(pos.x, pos.y) {
            *needs_rebuild = true;
        }
    }

    fn handle_mouse_input(&mut self, button: clear_ui::widget::MouseButton, state: clear_ui::widget::ElementState, _pos: clear_ui::engine::LogicalPosition, needs_rebuild: &mut bool) -> Option<Self::Message> {
        if self.handle_mouse_input_internal(button, state) {
            *needs_rebuild = true;
        }
        None
    }

    fn handle_mouse_wheel(&mut self, delta: &clear_ui::widget::MouseScrollDelta, pos: clear_ui::engine::LogicalPosition, needs_rebuild: &mut bool) {
        if self.handle_mouse_wheel_internal(delta, pos.x, pos.y) {
            *needs_rebuild = true;
        }
    }

    fn handle_key_input(&mut self, event: &clear_ui::widget::KeyEvent, needs_rebuild: &mut bool) -> Option<Self::Message> {
        if self.handle_key_input_internal(event) {
            *needs_rebuild = true;
        }
        None
    }
}

impl SystemInterface {

fn collect_popover_rects(w: &dyn clear_ui::widget::Widget, popovers: &mut Vec<(f32, f32, f32, f32)>) {
    if let Some(rect) = w.popover_rect() {
        popovers.push(rect);
    }
    for child_ptr in w.children() {
        unsafe {
            if let Some(child) = child_ptr.as_ref() {
                Self::collect_popover_rects(child, popovers);
            }
        }
    }
}

    fn rebuild_layout(&mut self, sw: f32, sh: f32) {
        let s = 1.0f32;
        let mut widgets = Vec::new();
        let mut text_items = Vec::new();
        let mut page_buttons = Vec::new();

        clear_ui::widget::hover_animation::reset_frame_registration();
        clear_ui::widget::popovers::clear();
        clear_ui::widget::hover_animation::set_scroll_offset(self.scroll_y);
        clear_ui::widget::hover_animation::set_cursor_pos(self.cursor_x / s, self.cursor_y / s);

        let lcx = self.sidebar_width;
        let lcy = self.header_height;
        let lcw = sw / s - self.sidebar_width;
        let lch = sh / s - self.header_height - self.status_height;

        let page_idx = Page::ALL.iter().position(|&p| p == self.app.current_page).unwrap_or(0);
        self.paginator.set_selected_page(page_idx);

        let mut paginator_pc = PageContent::new();
        clear_ui::layout::render_widget(&mut paginator_pc, &mut self.paginator, 0.0, 0.0, sw / s, sh / s);

        for (c, x, y, w, h) in &paginator_pc.rects {
            widgets.push(AppWidget {
                x: *x * s, y: *y * s, w: *w * s, h: *h * s,
                color: *c, hover_color: *c,
                hovering: false, kind: WidgetKind::Static,
            });
        }
        for (t, size, x, y, tc, font_opt) in &paginator_pc.texts {
            text_items.push(TextItem {
                buffer: make_text_buffer_with_font(
                    &mut self.font_system,
                    t,
                    *size * s,
                    font_opt.as_deref(),
                    &self.sans_serif_family,
                    &self.serif_family,
                    &self.monospace_family,
                ),
                x: *x * s, y: *y * s,
                color: glyphon::Color::rgb(
                    (tc[0] * 255.0) as u8, (tc[1] * 255.0) as u8, (tc[2] * 255.0) as u8,
                ),
            });
        }

        // Page content in LOGICAL coordinates, then scale to physical
        let mut pc = self.render_page_content(lcx, lcy, lcw, lch);
        clear_ui::layout::render_popovers(&mut pc);

        // ── Rebuild Widget Focus Hierarchy ──
        self.page_root_container.clear_children();
        self.page_root_container.set_parent(None);
        self.page_sec_containers.clear();

        // Clear all widgets' hierarchy links
        self.app.typeface.sans_box.clear_children(); self.app.typeface.sans_box.set_parent(None);
        self.app.typeface.serif_box.clear_children(); self.app.typeface.serif_box.set_parent(None);
        self.app.typeface.mono_box.clear_children(); self.app.typeface.mono_box.set_parent(None);
        self.app.typeface.borders_menu.clear_children(); self.app.typeface.borders_menu.set_parent(None);
        self.app.typeface.borders_box.clear_children(); self.app.typeface.borders_box.set_parent(None);
        self.app.typeface.status_menu.clear_children(); self.app.typeface.status_menu.set_parent(None);
        self.app.typeface.status_box.clear_children(); self.app.typeface.status_box.set_parent(None);
        self.app.typeface.fuzzel_menu.clear_children(); self.app.typeface.fuzzel_menu.set_parent(None);
        self.app.typeface.fuzzel_box.clear_children(); self.app.typeface.fuzzel_box.set_parent(None);
        self.app.typeface.terminal_menu.clear_children(); self.app.typeface.terminal_menu.set_parent(None);
        self.app.typeface.terminal_box.clear_children(); self.app.typeface.terminal_box.set_parent(None);
        self.app.typeface.search_box.clear_children(); self.app.typeface.search_box.set_parent(None);
        self.app.typeface.list_box.scroll_box.clear_children(); self.app.typeface.list_box.scroll_box.set_parent(None);

        self.app.services.search_box.clear_children(); self.app.services.search_box.set_parent(None);

        self.app.services.list_box.scroll_box.clear_children(); self.app.services.list_box.scroll_box.set_parent(None);

        self.app.hardware.cpu_list_box.scroll_box.clear_children(); self.app.hardware.cpu_list_box.scroll_box.set_parent(None);

        self.app.network.wifi_list_box.scroll_box.clear_children(); self.app.network.wifi_list_box.scroll_box.set_parent(None);

        for sb in &mut self.app.layout.spinboxes {
            sb.clear_children();
            sb.set_parent(None);
        }
        self.app.layout.cascade_offset_spinbox.clear_children(); self.app.layout.cascade_offset_spinbox.set_parent(None);
        self.app.layout.edge_gap_spinbox.clear_children(); self.app.layout.edge_gap_spinbox.set_parent(None);
        self.app.layout.top_gap_spinbox.clear_children(); self.app.layout.top_gap_spinbox.set_parent(None);
        self.app.layout.grid_gap_spinbox.clear_children(); self.app.layout.grid_gap_spinbox.set_parent(None);
        self.app.layout.transition_duration_spinbox.clear_children(); self.app.layout.transition_duration_spinbox.set_parent(None);
        self.app.layout.status_height_spinbox.clear_children(); self.app.layout.status_height_spinbox.set_parent(None);

        for cs in &mut self.app.colors.color_selectors {
            cs.clear_children();
            cs.set_parent(None);
        }

        self.app.notifications.duration_spinbox.clear_children(); self.app.notifications.duration_spinbox.set_parent(None);
        self.app.notifications.opacity_slider.clear_children(); self.app.notifications.opacity_slider.set_parent(None);

        self.app.input.rate_spinbox.clear_children(); self.app.input.rate_spinbox.set_parent(None);
        self.app.input.delay_spinbox.clear_children(); self.app.input.delay_spinbox.set_parent(None);
        self.app.input.scroll_toggle.clear_children(); self.app.input.scroll_toggle.set_parent(None);
        self.app.input.scroll_friction_spinbox.clear_children(); self.app.input.scroll_friction_spinbox.set_parent(None);
        self.app.input.natural_toggle.clear_children(); self.app.input.natural_toggle.set_parent(None);
        self.app.input.scroll_speed_spinbox.clear_children(); self.app.input.scroll_speed_spinbox.set_parent(None);
        self.app.input.pointer_toggle.clear_children(); self.app.input.pointer_toggle.set_parent(None);
        self.app.input.pointer_friction_spinbox.clear_children(); self.app.input.pointer_friction_spinbox.set_parent(None);
        self.app.input.trackpad_toggle.clear_children(); self.app.input.trackpad_toggle.set_parent(None);
        self.app.input.trackpad_friction_spinbox.clear_children(); self.app.input.trackpad_friction_spinbox.set_parent(None);
        self.app.input.dwtp_toggle.clear_children(); self.app.input.dwtp_toggle.set_parent(None);
        self.app.input.trackpoint_accel_speed_spinbox.clear_children(); self.app.input.trackpoint_accel_speed_spinbox.set_parent(None);
        self.app.input.trackpoint_accel_profile_menu.clear_children(); self.app.input.trackpoint_accel_profile_menu.set_parent(None);
        self.app.input.cursor_theme_menu.clear_children(); self.app.input.cursor_theme_menu.set_parent(None);
        self.app.input.cursor_size_spinbox.clear_children(); self.app.input.cursor_size_spinbox.set_parent(None);

        for sb in &mut self.app.audio.sink_spinboxes {
            sb.clear_children();
            sb.set_parent(None);
        }
        for sb in &mut self.app.audio.source_spinboxes {
            sb.clear_children();
            sb.set_parent(None);
        }

        self.app.display.brightness_spinbox.clear_children(); self.app.display.brightness_spinbox.set_parent(None);
        self.app.status.padding_spinbox.clear_children(); self.app.status.padding_spinbox.set_parent(None);

        for menu in &mut self.app.layout.tag_layout_menus {
            menu.clear_children();
            menu.set_parent(None);
        }

        use clear_ui::widget::focus::link_parent_child;
        match self.app.current_page {
            Page::Typefaces => {
                self.page_sec_containers.resize_with(3, clear_ui::widget::Container::new);
                
                link_parent_child(&mut self.page_root_container, &mut self.page_sec_containers[0]);
                link_parent_child(&mut self.page_root_container, &mut self.page_sec_containers[1]);
                link_parent_child(&mut self.page_root_container, &mut self.page_sec_containers[2]);
                
                link_parent_child(&mut self.page_sec_containers[0], &mut self.app.typeface.sans_box);
                link_parent_child(&mut self.page_sec_containers[0], &mut self.app.typeface.serif_box);
                link_parent_child(&mut self.page_sec_containers[0], &mut self.app.typeface.mono_box);
                
                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.typeface.borders_menu);
                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.typeface.borders_box);
                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.typeface.status_menu);
                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.typeface.status_box);
                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.typeface.fuzzel_menu);
                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.typeface.fuzzel_box);
                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.typeface.terminal_menu);
                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.typeface.terminal_box);
                
                link_parent_child(&mut self.page_sec_containers[2], &mut self.app.typeface.search_box);
                link_parent_child(&mut self.page_sec_containers[2], &mut self.app.typeface.list_box.scroll_box);
            }
            Page::Services => {
                link_parent_child(&mut self.page_root_container, &mut self.app.services.search_box);
                link_parent_child(&mut self.page_root_container, &mut self.app.services.list_box.scroll_box);
            }
            Page::Hardware => {
                link_parent_child(&mut self.page_root_container, &mut self.app.hardware.cpu_list_box.scroll_box);
            }
            Page::Radios => {
                link_parent_child(&mut self.page_root_container, &mut self.app.network.wifi_list_box.scroll_box);
            }
            Page::Layout => {
                self.page_sec_containers.resize_with(5, clear_ui::widget::Container::new);
                
                for i in 0..5 {
                    link_parent_child(&mut self.page_root_container, &mut self.page_sec_containers[i]);
                }
                
                for sb in &mut self.app.layout.spinboxes {
                    link_parent_child(&mut self.page_sec_containers[0], sb);
                }
                
                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.layout.cascade_offset_spinbox);
                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.layout.edge_gap_spinbox);
                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.layout.top_gap_spinbox);
                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.layout.grid_gap_spinbox);
                
                link_parent_child(&mut self.page_sec_containers[2], &mut self.app.layout.transition_duration_spinbox);
                
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.layout.status_height_spinbox);

                for menu in &mut self.app.layout.tag_layout_menus {
                    link_parent_child(&mut self.page_sec_containers[4], menu);
                }
            }
            Page::Colors => {
                for cs in &mut self.app.colors.color_selectors {
                    link_parent_child(&mut self.page_root_container, cs);
                }
            }
            Page::Notifications => {
                link_parent_child(&mut self.page_root_container, &mut self.app.notifications.duration_spinbox);
                link_parent_child(&mut self.page_root_container, &mut self.app.notifications.opacity_slider);
            }
            Page::Input => {
                self.page_sec_containers.resize_with(5, clear_ui::widget::Container::new);
                link_parent_child(&mut self.page_root_container, &mut self.page_sec_containers[0]);
                link_parent_child(&mut self.page_root_container, &mut self.page_sec_containers[1]);
                link_parent_child(&mut self.page_root_container, &mut self.page_sec_containers[2]);
                link_parent_child(&mut self.page_root_container, &mut self.page_sec_containers[3]);
                link_parent_child(&mut self.page_root_container, &mut self.page_sec_containers[4]);
                
                link_parent_child(&mut self.page_sec_containers[0], &mut self.app.input.dwtp_toggle);
                link_parent_child(&mut self.page_sec_containers[0], &mut self.app.input.trackpoint_accel_speed_spinbox);
                link_parent_child(&mut self.page_sec_containers[0], &mut self.app.input.trackpoint_accel_profile_menu);
                
                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.input.rate_spinbox);
                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.input.delay_spinbox);

                link_parent_child(&mut self.page_sec_containers[2], &mut self.app.input.cursor_theme_menu);
                link_parent_child(&mut self.page_sec_containers[2], &mut self.app.input.cursor_size_spinbox);
                
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.input.scroll_toggle);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.input.scroll_friction_spinbox);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.input.natural_toggle);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.input.scroll_speed_spinbox);
                
                link_parent_child(&mut self.page_sec_containers[4], &mut self.app.input.pointer_toggle);
                link_parent_child(&mut self.page_sec_containers[4], &mut self.app.input.pointer_friction_spinbox);
                link_parent_child(&mut self.page_sec_containers[4], &mut self.app.input.trackpad_toggle);
                link_parent_child(&mut self.page_sec_containers[4], &mut self.app.input.trackpad_friction_spinbox);
            }
            Page::Audio => {
                self.page_sec_containers.resize_with(2, clear_ui::widget::Container::new);
                link_parent_child(&mut self.page_root_container, &mut self.page_sec_containers[0]);
                link_parent_child(&mut self.page_root_container, &mut self.page_sec_containers[1]);
                
                for sb in &mut self.app.audio.sink_spinboxes {
                    link_parent_child(&mut self.page_sec_containers[0], sb);
                }
                for sb in &mut self.app.audio.source_spinboxes {
                    link_parent_child(&mut self.page_sec_containers[1], sb);
                }
            }
            Page::Display => {
                link_parent_child(&mut self.page_root_container, &mut self.app.display.brightness_spinbox);
            }
            Page::Status => {
                link_parent_child(&mut self.page_root_container, &mut self.app.status.padding_spinbox);
            }
            _ => {}
        }

        let mut popovers = Vec::new();
        Self::collect_popover_rects(&self.page_root_container, &mut popovers);

        if !popovers.is_empty() {
            pc.texts.retain(|(text, size, tx, ty, _, _)| {
                let text_w = text.chars().count() as f32 * *size * 0.65;
                for &(px, py, pw, ph) in &popovers {
                    let x_overlap = *tx <= px + pw && (*tx + text_w) >= px;
                    let y_overlap = *ty <= py + ph && (*ty + *size) >= py;
                    if x_overlap && y_overlap {
                        return false;
                    }
                }
                true
            });
        }

        let mut max_y = 0.0f32;
        for (_, _, y, _, h) in &pc.rects {
            max_y = max_y.max(y + h);
        }
        for (_, size, _, y, _, _) in &pc.texts {
            max_y = max_y.max(y + size);
        }
        for btn in &pc.buttons {
            max_y = max_y.max(btn.y + btn.h);
        }
        self.max_scroll_y = (max_y - lch).max(0.0);
        self.scroll_y = self.scroll_y.min(self.max_scroll_y);

        let scroll_offset_y = self.scroll_y;

        for (c, x, y, w, h) in &pc.rects {
            widgets.push(AppWidget {
                x: *x * s, y: (*y - scroll_offset_y) * s, w: *w * s, h: *h * s,
                color: *c, hover_color: *c,
                hovering: false, kind: WidgetKind::Static,
            });
        }
        for (t, size, x, y, tc, font_opt) in &pc.texts {
            text_items.push(TextItem {
                buffer: make_text_buffer_with_font(
                    &mut self.font_system,
                    t,
                    *size * s,
                    font_opt.as_deref(),
                    &self.sans_serif_family,
                    &self.serif_family,
                    &self.monospace_family,
                ),
                x: *x * s, y: (*y - scroll_offset_y) * s,
                color: glyphon::Color::rgb(
                    (tc[0] * 255.0) as u8, (tc[1] * 255.0) as u8, (tc[2] * 255.0) as u8,
                ),
            });
        }
        for btn in &pc.buttons {
            widgets.push(AppWidget {
                x: btn.x * s, y: (btn.y - scroll_offset_y) * s, w: btn.w * s, h: btn.h * s,
                color: btn.bg, hover_color: btn.hover_bg,
                hovering: false,
                kind: WidgetKind::ActionButton(btn.action.clone()),
            });
            let buf = make_text_buffer(&mut self.font_system, &btn.label, btn.label_size * s);
            let tw = buf.layout_runs().next().map(|r| r.line_w).unwrap_or(0.0);
            let lh = btn.label_size * s * 1.4;
            let mut left_align = btn.left_align;

            // Auto-detect if inside a ScrollBox to apply left alignment by default
            if !left_align && btn.w >= 60.0 {
                if self.app.current_page == Page::Typefaces {
                    let sb = &self.app.typeface.list_box;
                    let (sb_x, sb_y, sb_w, sb_h) = sb.rect();
                    if btn.x >= sb_x - 1.0 && btn.x + btn.w <= sb_x + sb_w + 1.0
                       && btn.y >= sb_y - 1.0 && btn.y + btn.h <= sb_y + sb_h + 1.0 {
                        left_align = true;
                    }
                } else if self.app.current_page == Page::Hardware {
                    let sb = &self.app.hardware.cpu_list_box;
                    let (sb_x, sb_y, sb_w, sb_h) = sb.rect();
                    if btn.x >= sb_x - 1.0 && btn.x + btn.w <= sb_x + sb_w + 1.0
                       && btn.y >= sb_y - 1.0 && btn.y + btn.h <= sb_y + sb_h + 1.0 {
                        left_align = true;
                    }
                } else if self.app.current_page == Page::Services {
                    let sb = &self.app.services.list_box;
                    let (sb_x, sb_y, sb_w, sb_h) = sb.rect();
                    if btn.x >= sb_x - 1.0 && btn.x + btn.w <= sb_x + sb_w + 1.0
                       && btn.y >= sb_y - 1.0 && btn.y + btn.h <= sb_y + sb_h + 1.0 {
                        left_align = true;
                    }
                }
            }

            let text_x = if left_align {
                btn.x * s + 8.0 * s
            } else {
                btn.x * s + (btn.w * s - tw) / 2.0
            };

            text_items.push(TextItem {
                buffer: buf,
                x: text_x, y: (btn.y - scroll_offset_y) * s + (btn.h * s - lh) / 2.0,
                color: glyphon::Color::rgb(
                    (btn.label_color[0] * 255.0) as u8,
                    (btn.label_color[1] * 255.0) as u8,
                    (btn.label_color[2] * 255.0) as u8,
                ),
            });
            let mut cb = btn.clone();
            cb.x *= s; cb.y = (cb.y - scroll_offset_y) * s; cb.w *= s; cb.h *= s;
            page_buttons.push(cb);
        }

        // Draw global hover highlight if active
        clear_ui::widget::hover_animation::post_render_check();
        if let Some((qx, qy, qw, qh, qc)) = clear_ui::widget::hover_animation::get_quad() {
            widgets.push(AppWidget {
                x: qx * s,
                y: (qy - self.scroll_y) * s,
                w: qw * s,
                h: qh * s,
                color: qc,
                hover_color: qc,
                hovering: false,
                kind: WidgetKind::Static,
            });
        }

        // Render context menu overlay if visible
        if clear_ui::widget::context_menu::is_visible() {
            let cx = clear_ui::widget::context_menu::x();
            let cy = clear_ui::widget::context_menu::y();
            let cw = clear_ui::widget::context_menu::w();
            let ch = clear_ui::widget::context_menu::h();
            
            // Border
            widgets.push(AppWidget {
                x: cx * s, y: cy * s, w: cw * s, h: ch * s,
                color: [0.22, 0.22, 0.28, 1.0], hover_color: [0.22, 0.22, 0.28, 1.0],
                hovering: false, kind: WidgetKind::Static,
            });
            // Bg
            widgets.push(AppWidget {
                x: (cx + 1.0) * s, y: (cy + 1.0) * s, w: (cw - 2.0) * s, h: (ch - 2.0) * s,
                color: [0.06, 0.06, 0.09, 1.0], hover_color: [0.06, 0.06, 0.09, 1.0],
                hovering: false, kind: WidgetKind::Static,
            });
            
            // Hover highlight
            if let Some(h_idx) = clear_ui::widget::context_menu::hovered_item() {
                let iy = cy + h_idx as f32 * 24.0;
                widgets.push(AppWidget {
                    x: (cx + 2.0) * s, y: (iy + 2.0) * s, w: (cw - 4.0) * s, h: 20.0 * s,
                    color: [0.20, 0.40, 0.65, 0.6], hover_color: [0.20, 0.40, 0.65, 0.6],
                    hovering: false, kind: WidgetKind::Static,
                });
            }
            
            // Texts
            for (idx, opt) in clear_ui::widget::context_menu::options().iter().enumerate() {
                let iy = cy + idx as f32 * 24.0 + (24.0 - 12.0) / 2.0;
                let text_color = if clear_ui::widget::context_menu::hovered_item() == Some(idx) {
                    glyphon::Color::rgb(0xff, 0xff, 0xff)
                } else {
                    glyphon::Color::rgb(0xcc, 0xcc, 0xd4)
                };
                text_items.push(TextItem {
                    buffer: make_text_buffer(&mut self.font_system, opt, 12.0 * s),
                    x: (cx + 8.0) * s, y: iy * s,
                    color: text_color,
                });
            }
        }



        self.widgets = widgets;
        self.text_items = text_items;
        self.page_buttons = page_buttons;
        self.needs_rebuild = false;
    }

    fn render_page_content(&mut self, cx: f32, cy: f32, cw: f32, ch: f32) -> PageContent {
        use pages::*;
        let root_focused = clear_ui::widget::focus::is_focused(&self.page_root_container);
        let sec_focused: Vec<bool> = self.page_sec_containers.iter()
            .map(|c| clear_ui::widget::focus::is_focused(c))
            .collect();
        match self.app.current_page {
            Page::Audio => audio::view(&mut self.app.audio, cx, cy, cw, ch, &sec_focused),
            Page::Display => display::view(&mut self.app.display, cx, cy, cw, ch),
            Page::Radios => network::view(&mut self.app.network, cx, cy, cw, ch, root_focused),
            Page::Layout => layout::view(&mut self.app.layout, cx, cy, cw, ch, &sec_focused),
            Page::Hardware => hardware::view(&mut self.app.hardware, cx, cy, cw, ch, root_focused),
            Page::Input => input::view(&mut self.app.input, cx, cy, cw, ch, &sec_focused),
            Page::System => system_info::view(&self.app.system_info, cx, cy, cw, ch),
            Page::Status => status::view(&mut self.app.status, cx, cy, cw, ch),
            Page::Storage => storage::view(&self.app.storage, cx, cy, cw, ch),
            Page::Notifications => notifications::view(&mut self.app.notifications, cx, cy, cw, ch),
            Page::Backup => backup::view(&self.app.backup, cx, cy, cw, ch),
            Page::Typefaces => typeface::view(&mut self.app.typeface, cx, cy, cw, ch, &sec_focused),
            Page::Services => services::view(&mut self.app.services, cx, cy, cw, ch, root_focused),
            Page::Colors => colors::view(&mut self.app.colors, cx, cy, cw, ch),
        }
    }


    fn tick_internal(&mut self, dt: f32) -> bool {
        let mut needs_redraw = false;
        if hover_animation::tick(dt) {
            needs_redraw = true;
            self.needs_rebuild = true;
        }
        if self.paginator.tick(dt) {
            needs_redraw = true;
            self.needs_rebuild = true;
        }
        if let Some(root_ptr) = self.get_page_root_widget() {
            unsafe {
                if (*root_ptr).tick(dt) {
                    needs_redraw = true;
                    self.needs_rebuild = true;
                }
            }
        }

        // Asynchronously check color selector changes (e.g. Zenity process exit)
        let mut color_changed = false;
        let mut color_actions = Vec::new();
        for (i, cp) in self.app.colors.color_selectors.iter_mut().enumerate() {
            if cp.tick(dt) {
                needs_redraw = true;
                self.needs_rebuild = true;
            }
            let state_color = match i {
                0 => self.app.colors.page_low_color,
                1 => self.app.colors.high_color,
                2 => self.app.colors.visual_guides_color,
                3 => self.app.colors.disabled_color,
                4 => self.app.colors.separator_color,
                5 => self.app.colors.slider_track_color,
                6 => self.app.colors.color_borders_color,
                7 => self.app.colors.low_color,
                8 => self.app.colors.normal_color,
                9 => self.app.colors.paginator_sidebar_color,
                10 => self.app.colors.primary_highlight_color,
                11 => self.app.colors.paginator_tab_label_color,
                _ => self.app.colors.low_color,
            };
            if cp.color != state_color {
                color_actions.push(AppAction::Colors(match i {
                    0 => pages::colors::ColorsMessage::SetPageLowColor(cp.color),
                    1 => pages::colors::ColorsMessage::SetHighColor(cp.color),
                    2 => pages::colors::ColorsMessage::SetVisualGuidesColor(cp.color),
                    3 => pages::colors::ColorsMessage::SetDisabledColor(cp.color),
                    4 => pages::colors::ColorsMessage::SetSeparatorColor(cp.color),
                    5 => pages::colors::ColorsMessage::SetSliderTrackColor(cp.color),
                    6 => pages::colors::ColorsMessage::SetColorBordersColor(cp.color),
                    7 => pages::colors::ColorsMessage::SetLowColor(cp.color),
                    8 => pages::colors::ColorsMessage::SetNormalColor(cp.color),
                    9 => pages::colors::ColorsMessage::SetPaginatorSidebarColor(cp.color),
                    10 => pages::colors::ColorsMessage::SetPrimaryHighlightColor(cp.color),
                    11 => pages::colors::ColorsMessage::SetPaginatorTabLabelColor(cp.color),
                    _ => pages::colors::ColorsMessage::SetLowColor(cp.color),
                }));
                color_changed = true;
            }
        }
        for action in color_actions {
            self.handle_action(&action);
        }
        if color_changed {
            needs_redraw = true;
            self.needs_rebuild = true;
        }

        needs_redraw
    }

    fn poll_background_updates(&mut self) {
        use pages::*;
        while let Ok(s) = self.rx_audio.try_recv() {
            audio::update(&mut self.app.audio, audio::AudioMessage::Refreshed(s));
            self.needs_rebuild = true;
        }
        while let Ok(s) = self.rx_display.try_recv() {
            display::update(&mut self.app.display, display::DisplayMessage::Refreshed(s));
            self.needs_rebuild = true;
        }
        while let Ok(s) = self.rx_network.try_recv() {
            network::update(&mut self.app.network, network::NetworkMessage::Refreshed(s));
            self.needs_rebuild = true;
        }
        while let Ok(s) = self.rx_layout.try_recv() {
            layout::update(&mut self.app.layout, layout::LayoutMessage::Refreshed(s));
            self.needs_rebuild = true;
        }
        while let Ok(_) = self.rx_wm_events.try_recv() {
            self.needs_rebuild = true;
        }
        while let Ok(s) = self.rx_input.try_recv() {
            input::update(&mut self.app.input, input::InputMessage::Refreshed(s));
            self.needs_rebuild = true;
        }
        let mut got_fingers = None;
        while let Ok(s) = self.rx_fingers.try_recv() {
            got_fingers = Some(s);
        }
        if let Some(fingers) = got_fingers {
            input::update(&mut self.app.input, input::InputMessage::UpdateFingers(fingers));
            self.needs_rebuild = true;
        }
        while let Ok(s) = self.rx_system.try_recv() {
            system_info::update(&mut self.app.system_info, system_info::SystemMessage::Refreshed(s));
            self.needs_rebuild = true;
        }
        while let Ok(s) = self.rx_hardware.try_recv() {
            hardware::update(&mut self.app.hardware, hardware::HardwareMessage::Refreshed(s));
            self.needs_rebuild = true;
        }
        while let Ok(s) = self.rx_status.try_recv() {
            status::update(&mut self.app.status, status::StatusMessage::Refreshed(s));
            self.needs_rebuild = true;
        }
        while let Ok(s) = self.rx_storage.try_recv() {
            storage::update(&mut self.app.storage, storage::StorageMessage::Refreshed(s));
            self.needs_rebuild = true;
        }
        while let Ok(s) = self.rx_notifications.try_recv() {
            notifications::update(&mut self.app.notifications, notifications::NotificationsMessage::Refreshed(s));
            self.needs_rebuild = true;
        }
        while let Ok(s) = self.rx_backup_state.try_recv() {
            pages::backup::update(&mut self.app.backup, pages::backup::BackupMessage::Refreshed(s));
            self.needs_rebuild = true;
        }
        while let Ok(s) = self.rx_typeface.try_recv() {
            self.sans_serif_family = s.sans_serif.clone();
            self.serif_family = s.serif.clone();
            self.monospace_family = s.monospace.clone();
            typeface::update(&mut self.app.typeface, typeface::TypefaceMessage::Refreshed(s));
            self.needs_rebuild = true;
        }
        while let Ok(s) = self.rx_services.try_recv() {
            pages::services::update(&mut self.app.services, pages::services::ServicesMessage::Refreshed(s));
            self.needs_rebuild = true;
        }
        while let Ok(s) = self.rx_colors.try_recv() {
            colors::update(&mut self.app.colors, pages::colors::ColorsMessage::Refreshed(s));
            self.needs_rebuild = true;
        }
        while let Ok(m) = self.rx_backup.try_recv() {
            self.handle_action(&AppAction::Backup(m));
            self.needs_rebuild = true;
        }
        while let Ok(action) = self.rx_color_selector.try_recv() {
            match action {
                ColorSelectorAction::Background(rgb) => {
                    colors::update(&mut self.app.colors, pages::colors::ColorsMessage::SetLowColor(rgb));
                }
                ColorSelectorAction::Border(rgb) => {
                    colors::update(&mut self.app.colors, pages::colors::ColorsMessage::SetHighColor(rgb));
                }
            }
            self.needs_rebuild = true;
        }
    }

    fn handle_action(&mut self, action: &AppAction) {
        use pages::*;
        match action {
            AppAction::Audio(m) => audio::update(&mut self.app.audio, m.clone()),
            AppAction::Display(m) => display::update(&mut self.app.display, m.clone()),
            AppAction::Radios(m) => network::update(&mut self.app.network, m.clone()),
            AppAction::Layout(m) => layout::update(&mut self.app.layout, m.clone()),
            AppAction::Input(m) => input::update(&mut self.app.input, m.clone()),
            AppAction::SystemInfo(m) => system_info::update(&mut self.app.system_info, m.clone()),
            AppAction::Hardware(m) => hardware::update(&mut self.app.hardware, m.clone()),
            AppAction::Status(m) => status::update(&mut self.app.status, m.clone()),
            AppAction::Storage(m) => storage::update(&mut self.app.storage, m.clone()),
            AppAction::Notifications(m) => notifications::update(&mut self.app.notifications, m.clone()),
            AppAction::Typeface(m) => {
                typeface::update(&mut self.app.typeface, m.clone());
                self.sans_serif_family = self.app.typeface.sans_serif.clone();
                self.serif_family = self.app.typeface.serif.clone();
                self.monospace_family = self.app.typeface.monospace.clone();
            }
            AppAction::Services(m) => services::update(&mut self.app.services, m.clone()),
            AppAction::Colors(m) => colors::update(&mut self.app.colors, m.clone()),
            AppAction::Backup(m) => match m {
                pages::backup::BackupMessage::StartBackup => {
                    pages::backup::update(&mut self.app.backup, pages::backup::BackupMessage::StartBackup);
                    let tx = self.tx_backup.clone();
                    tokio::spawn(async move {
                        let res = pages::backup::run_backup().await;
                        let _ = tx.send(pages::backup::BackupMessage::BackupFinished(res));
                    });
                }
                _ => pages::backup::update(&mut self.app.backup, m.clone()),
            },
        }
    }

    fn handle_cursor_moved(&mut self, x: f32, y: f32) -> bool {
        self.cursor_x = x;
        self.cursor_y = y;
        let s = 1.0f32;
        let lx_no_scroll = x / s;
        let ly_no_scroll = y / s;
        
        if clear_ui::widget::context_menu::is_visible() {
            if clear_ui::widget::context_menu::cursor_moved(lx_no_scroll, ly_no_scroll) {
                self.needs_rebuild = true;
                return true;
            }
            return false;
        }

        let lx = self.cursor_x / s;
        let ly = self.cursor_y / s + self.scroll_y;
        clear_ui::widget::hover_animation::set_cursor_pos(lx, ly_no_scroll);
        let mut changed = false;
        if self.paginator.cursor_moved(lx_no_scroll, ly_no_scroll) {
            changed = true;
        }
        for w in &mut self.widgets {
            let was = w.hovering;
            w.hovering = self.cursor_x >= w.x && self.cursor_x <= w.x + w.w
                && self.cursor_y >= w.y && self.cursor_y <= w.y + w.h;
            if w.hovering != was {
                w.color = if w.hovering { w.hover_color } else { w.color };
                changed = true;
            }
        }
        if self.app.current_page == Page::Layout {
            for sb in &mut self.app.layout.spinboxes {
                if sb.cursor_moved(lx, ly) {
                    changed = true;
                }
            }
            if self.app.layout.cascade_offset_spinbox.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.layout.edge_gap_spinbox.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.layout.top_gap_spinbox.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.layout.grid_gap_spinbox.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.layout.transition_duration_spinbox.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.layout.status_height_spinbox.cursor_moved(lx, ly) {
                changed = true;
            }
            for menu in &mut self.app.layout.tag_layout_menus {
                if menu.cursor_moved(lx, ly) {
                    changed = true;
                }
            }
        }
        if self.app.current_page == Page::Colors {
            for cp in &mut self.app.colors.color_selectors {
                if cp.cursor_moved(lx, ly) {
                    changed = true;
                }
            }
        }
        if self.app.current_page == Page::Input {
            if self.app.input.rate_spinbox.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.input.delay_spinbox.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.input.tap_toggle.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.input.scroll_toggle.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.input.scroll_friction_spinbox.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.input.pointer_toggle.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.input.pointer_friction_spinbox.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.input.trackpad_toggle.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.input.trackpad_friction_spinbox.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.input.dwtp_toggle.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.input.trackpoint_accel_speed_spinbox.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.input.trackpoint_accel_profile_menu.cursor_moved(lx, ly) {
                changed = true;
            }
        }
        if self.app.current_page == Page::Audio {
            for sb in &mut self.app.audio.sink_spinboxes {
                if sb.cursor_moved(lx, ly) {
                    changed = true;
                }
            }
            for sb in &mut self.app.audio.source_spinboxes {
                if sb.cursor_moved(lx, ly) {
                    changed = true;
                }
            }
        }
        if self.app.current_page == Page::Display {
            if self.app.display.brightness_spinbox.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.display.night_light_label.cursor_moved(lx, ly) {
                changed = true;
            }
            for out in &mut self.app.display.outputs {
                if out.name_label.cursor_moved(lx, ly) {
                    changed = true;
                }
                if out.resolution_label.cursor_moved(lx, ly) {
                    changed = true;
                }
                if let Some(ref mut scale_lbl) = out.scale_label {
                    if scale_lbl.cursor_moved(lx, ly) {
                        changed = true;
                    }
                }
            }
        }
        if self.opacity_dragging && self.app.current_page == Page::Notifications {
            if self.app.notifications.opacity_slider.drag_update(lx, ly) {
                changed = true;
            }
        } else if self.app.current_page == Page::Notifications {
            if self.app.notifications.enable_toggle.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.notifications.bell_toggle.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.notifications.duration_spinbox.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.notifications.opacity_slider.cursor_moved(lx, ly) {
                changed = true;
            }
        }
        if self.app.current_page == Page::Hardware {
            if self.app.hardware.cpu_label.cursor_moved(lx, ly) {
                changed = true;
            }
            for gpu_lbl in &mut self.app.hardware.gpu_labels {
                if gpu_lbl.cursor_moved(lx, ly) {
                    changed = true;
                }
            }
            if self.app.hardware.cpu_list_box.cursor_moved(lx, ly) {
                changed = true;
            }
        }
        if self.app.current_page == Page::Status {
            if self.app.status.status_label.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.status.size_label.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.status.separators_toggle.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.status.underline_toggle.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.status.padding_spinbox.cursor_moved(lx, ly) {
                changed = true;
            }
        }
        if self.app.current_page == Page::Typefaces {
            if self.app.typeface.sans_box.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.typeface.serif_box.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.typeface.mono_box.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.typeface.borders_menu.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.typeface.borders_box.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.typeface.status_menu.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.typeface.status_box.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.typeface.fuzzel_menu.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.typeface.fuzzel_box.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.typeface.terminal_menu.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.typeface.terminal_box.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.typeface.search_box.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.typeface.list_box.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.typeface.borders_size_box.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.typeface.status_size_box.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.typeface.fuzzel_size_box.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.typeface.terminal_size_box.cursor_moved(lx, ly) {
                changed = true;
            }
            let query = self.app.typeface.search_box.text.to_lowercase();
            let matching_count = self.app.typeface.all_fonts.iter()
                .filter(|f| f.to_lowercase().contains(&query))
                .count();
            for i in 0..matching_count.min(self.app.typeface.font_buttons.len()) {
                if self.app.typeface.font_buttons[i].cursor_moved(lx, ly) {
                    changed = true;
                }
                if self.app.typeface.copy_buttons[i].cursor_moved(lx, ly) {
                    changed = true;
                }
            }
        }
        if self.app.current_page == Page::Services {
            if self.app.services.search_box.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.services.list_box.cursor_moved(lx, ly) {
                changed = true;
            }
        }
        if changed { self.needs_rebuild = true; }
        changed
    }

    fn handle_mouse_input_internal(&mut self, button: clear_ui::widget::MouseButton, state: clear_ui::widget::ElementState) -> bool {
        let s = 1.0f32;
        let lx_no_scroll = self.cursor_x / s;
        let ly_no_scroll = self.cursor_y / s;

        if clear_ui::widget::context_menu::is_visible() {
            if clear_ui::widget::context_menu::mouse_input(button, state, lx_no_scroll, ly_no_scroll) {
                self.needs_rebuild = true;
                return true;
            }
        }

        if self.paginator.mouse_input(button, state, lx_no_scroll, ly_no_scroll) {
            if self.paginator.take_click() {
                let idx = self.paginator.selected_page();
                if idx < Page::ALL.len() {
                    clear_ui::widget::focus::clear_focus();
                    self.app.current_page = Page::ALL[idx];
                    self.scroll_y = 0.0;
                }
            }
            self.needs_rebuild = true;
            return true;
        }

        if button != clear_ui::widget::MouseButton::Left && button != clear_ui::widget::MouseButton::Right { return false; }
        if button == clear_ui::widget::MouseButton::Left && state == clear_ui::widget::ElementState::Released {
            let (px, py) = (self.cursor_x, self.cursor_y);
            for btn in &self.page_buttons.clone() {
                if px >= btn.x && px <= btn.x + btn.w && py >= btn.y && py <= btn.y + btn.h {
                    self.handle_action(&btn.action);
                    self.needs_rebuild = true;
                    return true;
                }
            }
        }
        let lx = self.cursor_x / s;
        let ly = self.cursor_y / s + self.scroll_y;
        let mut actions = Vec::new();

        if state == clear_ui::widget::ElementState::Pressed {
            let mut clicked_any_focusable = false;
            match self.app.current_page {
                Page::Layout => {
                    for sb in &mut self.app.layout.spinboxes {
                        if sb.hit_test(lx, ly) { clicked_any_focusable = true; }
                    }
                    if self.app.layout.cascade_offset_spinbox.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if self.app.layout.edge_gap_spinbox.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if self.app.layout.top_gap_spinbox.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if self.app.layout.grid_gap_spinbox.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if self.app.layout.transition_duration_spinbox.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if self.app.layout.status_height_spinbox.hit_test(lx, ly) { clicked_any_focusable = true; }
                    for menu in &mut self.app.layout.tag_layout_menus {
                        if menu.hit_test(lx, ly) { clicked_any_focusable = true; }
                    }
                }
                Page::Colors => {
                    for cp in &mut self.app.colors.color_selectors {
                        if cp.hit_test(lx, ly) { clicked_any_focusable = true; }
                    }
                }
                Page::Input => {
                    if self.app.input.rate_spinbox.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if self.app.input.delay_spinbox.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if self.app.input.scroll_friction_spinbox.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if self.app.input.pointer_friction_spinbox.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if self.app.input.trackpad_friction_spinbox.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if self.app.input.trackpoint_accel_speed_spinbox.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if self.app.input.trackpoint_accel_profile_menu.hit_test(lx, ly) { clicked_any_focusable = true; }
                }
                Page::Notifications => {
                    if self.app.notifications.duration_spinbox.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if self.app.notifications.opacity_slider.hit_test(lx, ly) { clicked_any_focusable = true; }
                }
                Page::Audio => {
                    for sb in &mut self.app.audio.sink_spinboxes {
                        if sb.hit_test(lx, ly) { clicked_any_focusable = true; }
                    }
                    for sb in &mut self.app.audio.source_spinboxes {
                        if sb.hit_test(lx, ly) { clicked_any_focusable = true; }
                    }
                }
                Page::Display => {
                    if self.app.display.brightness_spinbox.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if self.app.display.night_light_label.hit_test(lx, ly) { clicked_any_focusable = true; }
                    for out in &mut self.app.display.outputs {
                        if out.name_label.hit_test(lx, ly) { clicked_any_focusable = true; }
                        if out.resolution_label.hit_test(lx, ly) { clicked_any_focusable = true; }
                        if let Some(ref mut scale_lbl) = out.scale_label {
                            if scale_lbl.hit_test(lx, ly) { clicked_any_focusable = true; }
                        }
                    }
                }
                Page::Status => {
                    if self.app.status.status_label.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if self.app.status.size_label.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if self.app.status.padding_spinbox.hit_test(lx, ly) { clicked_any_focusable = true; }
                }
                Page::Typefaces => {
                    let tf = &mut self.app.typeface;
                    if tf.sans_box.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if tf.serif_box.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if tf.mono_box.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if tf.borders_menu.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if tf.borders_box.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if tf.status_menu.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if tf.status_box.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if tf.fuzzel_menu.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if tf.fuzzel_box.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if tf.terminal_menu.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if tf.terminal_box.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if tf.search_box.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if tf.list_box.hit_test(lx, ly) { clicked_any_focusable = true; }
                }
                Page::Services => {
                    let srv = &mut self.app.services;
                    if srv.search_box.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if srv.list_box.hit_test(lx, ly) { clicked_any_focusable = true; }
                }
                Page::Hardware => {
                    let hw = &mut self.app.hardware;
                    if hw.cpu_list_box.hit_test(lx, ly) { clicked_any_focusable = true; }
                }
                Page::Radios => {
                    let net = &mut self.app.network;
                    if net.wifi_list_box.hit_test(lx, ly) { clicked_any_focusable = true; }
                }
                _ => {}
            }

            if !clicked_any_focusable {
                clear_ui::widget::focus::clear_focus();
            }
        }

        if state == clear_ui::widget::ElementState::Pressed && self.app.current_page == Page::Layout {
            for (i, sb) in self.app.layout.spinboxes.iter_mut().enumerate() {
                if !sb.hit_test(lx, ly) { sb.unfocus(); }
                let old = sb.value;
                if sb.mouse_input(button, state, lx, ly) && sb.value != old {
                    actions.push(AppAction::Layout(
                        pages::layout::LayoutMessage::SetWidth(
                            pages::layout::WidthParam::ALL[i],
                            sb.value as u16,
                        )
                    ));
                }
            }
            let sb = &mut self.app.layout.cascade_offset_spinbox;
            if !sb.hit_test(lx, ly) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly) && sb.value != old {
                actions.push(AppAction::Layout(
                    pages::layout::LayoutMessage::SetCascadeOffset(sb.value as u16)
                ));
            }
            let sb = &mut self.app.layout.edge_gap_spinbox;
            if !sb.hit_test(lx, ly) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly) && sb.value != old {
                actions.push(AppAction::Layout(
                    pages::layout::LayoutMessage::SetEdgeGap(sb.value as u16)
                ));
            }
            let sb = &mut self.app.layout.top_gap_spinbox;
            if !sb.hit_test(lx, ly) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly) && sb.value != old {
                actions.push(AppAction::Layout(
                    pages::layout::LayoutMessage::SetTopGap(sb.value as u16)
                ));
            }
            let sb = &mut self.app.layout.grid_gap_spinbox;
            if !sb.hit_test(lx, ly) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly) && sb.value != old {
                actions.push(AppAction::Layout(
                    pages::layout::LayoutMessage::SetGridGap(sb.value as u16)
                ));
            }
            let sb = &mut self.app.layout.transition_duration_spinbox;
            if !sb.hit_test(lx, ly) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly) && sb.value != old {
                actions.push(AppAction::Layout(
                    pages::layout::LayoutMessage::SetTransitionDuration(sb.value as u16)
                ));
            }
            let sb = &mut self.app.layout.status_height_spinbox;
            if !sb.hit_test(lx, ly) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly) && sb.value != old {
                actions.push(AppAction::Layout(
                    pages::layout::LayoutMessage::SetStatusHeight(sb.value as u16)
                ));
            }
        }
        if self.app.current_page == Page::Layout {
            for (idx, menu) in self.app.layout.tag_layout_menus.iter_mut().enumerate() {
                if state == clear_ui::widget::ElementState::Pressed && !menu.hit_test(lx, ly) { menu.unfocus(); }
                if menu.mouse_input(button, state, lx, ly) {
                    self.needs_rebuild = true;
                }
                if state == clear_ui::widget::ElementState::Pressed && menu.take_change() {
                    actions.push(AppAction::Layout(pages::layout::LayoutMessage::SetTagLayout(idx + 1, menu.selected)));
                }
            }
        }
        if state == clear_ui::widget::ElementState::Pressed && self.app.current_page == Page::Colors {
            for (i, cp) in self.app.colors.color_selectors.iter_mut().enumerate() {
                let old = cp.color;
                if !cp.hit_test(lx, ly) { cp.unfocus(); }
                cp.mouse_input(button, state, lx, ly);
                if cp.take_click() {
                    actions.push(AppAction::Colors(match i {
                        0 => pages::colors::ColorsMessage::PickPageLowColor,
                        1 => pages::colors::ColorsMessage::PickHighColor,
                        2 => pages::colors::ColorsMessage::PickVisualGuides,
                        3 => pages::colors::ColorsMessage::PickDisabledColor,
                        4 => pages::colors::ColorsMessage::PickSeparatorColor,
                        5 => pages::colors::ColorsMessage::PickSliderTrackColor,
                        6 => pages::colors::ColorsMessage::PickColorBordersColor,
                        7 => pages::colors::ColorsMessage::PickLowColor,
                        8 => pages::colors::ColorsMessage::PickNormalColor,
                        9 => pages::colors::ColorsMessage::PickPaginatorSidebarColor,
                        10 => pages::colors::ColorsMessage::PickPrimaryHighlightColor,
                        11 => pages::colors::ColorsMessage::PickPaginatorTabLabelColor,
                        _ => pages::colors::ColorsMessage::PickLowColor,
                    }));
                }
                if cp.color != old {
                    actions.push(AppAction::Colors(match i {
                        0 => pages::colors::ColorsMessage::SetPageLowColor(cp.color),
                        1 => pages::colors::ColorsMessage::SetHighColor(cp.color),
                        2 => pages::colors::ColorsMessage::SetVisualGuidesColor(cp.color),
                        3 => pages::colors::ColorsMessage::SetDisabledColor(cp.color),
                        4 => pages::colors::ColorsMessage::SetSeparatorColor(cp.color),
                        5 => pages::colors::ColorsMessage::SetSliderTrackColor(cp.color),
                        6 => pages::colors::ColorsMessage::SetColorBordersColor(cp.color),
                        7 => pages::colors::ColorsMessage::SetLowColor(cp.color),
                        8 => pages::colors::ColorsMessage::SetNormalColor(cp.color),
                        9 => pages::colors::ColorsMessage::SetPaginatorSidebarColor(cp.color),
                        10 => pages::colors::ColorsMessage::SetPrimaryHighlightColor(cp.color),
                        11 => pages::colors::ColorsMessage::SetPaginatorTabLabelColor(cp.color),
                        _ => pages::colors::ColorsMessage::SetLowColor(cp.color),
                    }));
                }
            }
        }
        if state == clear_ui::widget::ElementState::Pressed && self.app.current_page == Page::Input {
            let sb = &mut self.app.input.rate_spinbox;
            if !sb.hit_test(lx, ly) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly) && sb.value != old {
                actions.push(AppAction::Input(pages::input::InputMessage::ApplyRepeat));
            }
            let sb = &mut self.app.input.delay_spinbox;
            if !sb.hit_test(lx, ly) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly) && sb.value != old {
                actions.push(AppAction::Input(pages::input::InputMessage::ApplyRepeat));
            }
            let sb = &mut self.app.input.scroll_friction_spinbox;
            if !sb.hit_test(lx, ly) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly) && sb.value != old {
                actions.push(AppAction::Input(pages::input::InputMessage::ApplyScrollFriction));
            }
            let sb = &mut self.app.input.scroll_speed_spinbox;
            if !sb.hit_test(lx, ly) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly) && sb.value != old {
                actions.push(AppAction::Input(pages::input::InputMessage::ApplyScrollSpeed));
            }
            let sb = &mut self.app.input.pointer_friction_spinbox;
            if !sb.hit_test(lx, ly) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly) && sb.value != old {
                actions.push(AppAction::Input(pages::input::InputMessage::ApplyPointerFriction));
            }
            let sb = &mut self.app.input.trackpad_friction_spinbox;
            if !sb.hit_test(lx, ly) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly) && sb.value != old {
                actions.push(AppAction::Input(pages::input::InputMessage::ApplyTrackpadFriction));
            }
            let sb = &mut self.app.input.trackpoint_accel_speed_spinbox;
            if !sb.hit_test(lx, ly) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly) && sb.value != old {
                actions.push(AppAction::Input(pages::input::InputMessage::ApplyTrackpointAccelSpeed));
            }
            let sb = &mut self.app.input.cursor_size_spinbox;
            if !sb.hit_test(lx, ly) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly) && sb.value != old {
                actions.push(AppAction::Input(pages::input::InputMessage::ApplyCursorSize));
            }
        }
        if state == clear_ui::widget::ElementState::Pressed && self.app.current_page == Page::Notifications {
            let sb = &mut self.app.notifications.duration_spinbox;
            if !sb.hit_test(lx, ly) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly) && sb.value != old {
                actions.push(AppAction::Notifications(pages::notifications::NotificationsMessage::SetDuration(sb.value)));
            }
        }
        if self.app.current_page == Page::Input {
            let toggle = &mut self.app.input.tap_toggle;
            toggle.mouse_input(button, state, lx, ly);
            if toggle.take_click() {
                actions.push(AppAction::Input(pages::input::InputMessage::ToggleTapToClick));
            }
            let toggle = &mut self.app.input.scroll_toggle;
            toggle.mouse_input(button, state, lx, ly);
            if toggle.take_click() {
                actions.push(AppAction::Input(pages::input::InputMessage::ToggleInertialScroll));
            }
            let toggle = &mut self.app.input.natural_toggle;
            toggle.mouse_input(button, state, lx, ly);
            if toggle.take_click() {
                actions.push(AppAction::Input(pages::input::InputMessage::ToggleNaturalScroll));
            }
            let toggle = &mut self.app.input.pointer_toggle;
            toggle.mouse_input(button, state, lx, ly);
            if toggle.take_click() {
                actions.push(AppAction::Input(pages::input::InputMessage::ToggleInertialPointer));
            }
            let toggle = &mut self.app.input.trackpad_toggle;
            toggle.mouse_input(button, state, lx, ly);
            if toggle.take_click() {
                actions.push(AppAction::Input(pages::input::InputMessage::ToggleInertialTrackpad));
            }
            let toggle = &mut self.app.input.dwtp_toggle;
            toggle.mouse_input(button, state, lx, ly);
            if toggle.take_click() {
                actions.push(AppAction::Input(pages::input::InputMessage::ToggleDwtp));
            }
            let menu = &mut self.app.input.trackpoint_accel_profile_menu;
            if state == clear_ui::widget::ElementState::Pressed && !menu.hit_test(lx, ly) { menu.unfocus(); }
            if menu.mouse_input(button, state, lx, ly) {
                self.needs_rebuild = true;
            }
            if state == clear_ui::widget::ElementState::Pressed && menu.take_change() {
                actions.push(AppAction::Input(pages::input::InputMessage::ApplyTrackpointAccelProfile(menu.selected)));
            }
            let menu = &mut self.app.input.cursor_theme_menu;
            if state == clear_ui::widget::ElementState::Pressed && !menu.hit_test(lx, ly) { menu.unfocus(); }
            if menu.mouse_input(button, state, lx, ly) {
                self.needs_rebuild = true;
            }
            if state == clear_ui::widget::ElementState::Pressed && menu.take_change() {
                actions.push(AppAction::Input(pages::input::InputMessage::ApplyCursorTheme(menu.selected)));
            }
        }
        if self.app.current_page == Page::Notifications {
            let toggle = &mut self.app.notifications.enable_toggle;
            toggle.mouse_input(button, state, lx, ly);
            if toggle.take_click() {
                actions.push(AppAction::Notifications(pages::notifications::NotificationsMessage::ToggleEnable));
            }
            let toggle = &mut self.app.notifications.bell_toggle;
            toggle.mouse_input(button, state, lx, ly);
            if toggle.take_click() {
                actions.push(AppAction::Notifications(pages::notifications::NotificationsMessage::ToggleBell));
            }
            let slider = &mut self.app.notifications.opacity_slider;
            if button == clear_ui::widget::MouseButton::Left {
                if state == clear_ui::widget::ElementState::Pressed {
                    if slider.hit_test(lx, ly) {
                        slider.drag_begin(lx, ly);
                        self.opacity_dragging = true;
                        self.needs_rebuild = true;
                    }
                } else if state == clear_ui::widget::ElementState::Released {
                    if self.opacity_dragging {
                        self.opacity_dragging = false;
                        slider.drag_end();
                        let val = slider.value() as f32 / 100.0;
                        actions.push(AppAction::Notifications(pages::notifications::NotificationsMessage::SetOpacity(val)));
                        self.needs_rebuild = true;
                    }
                }
            }
        }
        if state == clear_ui::widget::ElementState::Pressed && self.app.current_page == Page::Audio {
            for (i, sb) in self.app.audio.sink_spinboxes.iter_mut().enumerate() {
                if !sb.hit_test(lx, ly) { sb.unfocus(); }
                let old = sb.value;
                if sb.mouse_input(button, state, lx, ly) && sb.value != old {
                    let id = self.app.audio.sinks[i].id;
                    actions.push(AppAction::Audio(pages::audio::AudioMessage::SinkVolume(id, sb.value as f32 / 100.0)));
                }
            }
            for (i, sb) in self.app.audio.source_spinboxes.iter_mut().enumerate() {
                if !sb.hit_test(lx, ly) { sb.unfocus(); }
                let old = sb.value;
                if sb.mouse_input(button, state, lx, ly) && sb.value != old {
                    let id = self.app.audio.sources[i].id;
                    actions.push(AppAction::Audio(pages::audio::AudioMessage::SourceVolume(id, sb.value as f32 / 100.0)));
                }
            }
        }
        if state == clear_ui::widget::ElementState::Pressed && self.app.current_page == Page::Display {
            let sb = &mut self.app.display.brightness_spinbox;
            if !sb.hit_test(lx, ly) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly) && sb.value != old {
                actions.push(AppAction::Display(pages::display::DisplayMessage::BrightnessSet(sb.value as u32)));
            }
            let lbl = &mut self.app.display.night_light_label;
            if !lbl.hit_test(lx, ly) { lbl.unfocus(); }
            lbl.mouse_input(button, state, lx, ly);

            for out in &mut self.app.display.outputs {
                let lbl = &mut out.name_label;
                if !lbl.hit_test(lx, ly) { lbl.unfocus(); }
                lbl.mouse_input(button, state, lx, ly);

                let lbl2 = &mut out.resolution_label;
                if !lbl2.hit_test(lx, ly) { lbl2.unfocus(); }
                lbl2.mouse_input(button, state, lx, ly);

                if let Some(ref mut scale_lbl) = out.scale_label {
                    if !scale_lbl.hit_test(lx, ly) { scale_lbl.unfocus(); }
                    scale_lbl.mouse_input(button, state, lx, ly);
                }
            }
        }
        if state == clear_ui::widget::ElementState::Pressed && self.app.current_page == Page::Status {
            let lbl1 = &mut self.app.status.status_label;
            if !lbl1.hit_test(lx, ly) { lbl1.unfocus(); }
            lbl1.mouse_input(button, state, lx, ly);

            let lbl2 = &mut self.app.status.size_label;
            if !lbl2.hit_test(lx, ly) { lbl2.unfocus(); }
            lbl2.mouse_input(button, state, lx, ly);

            let sb = &mut self.app.status.padding_spinbox;
            if !sb.hit_test(lx, ly) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly) && sb.value != old {
                actions.push(AppAction::Status(pages::status::StatusMessage::SetPadding(sb.value as u16)));
            }
        }
        if self.app.current_page == Page::Status {
            let toggle = &mut self.app.status.separators_toggle;
            toggle.mouse_input(button, state, lx, ly);
            if toggle.take_click() {
                actions.push(AppAction::Status(pages::status::StatusMessage::ToggleSeparators));
            }

            let toggle2 = &mut self.app.status.underline_toggle;
            toggle2.mouse_input(button, state, lx, ly);
            if toggle2.take_click() {
                actions.push(AppAction::Status(pages::status::StatusMessage::ToggleUnderline));
            }
        }
        if self.app.current_page == Page::Typefaces {
            let tb = &mut self.app.typeface.sans_box;
            if state == clear_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly) { tb.unfocus(); }
            if tb.mouse_input(button, state, lx, ly) {
                self.needs_rebuild = true;
            }
            if state == clear_ui::widget::ElementState::Pressed && tb.take_change() {
                actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetSans(tb.text.clone())));
            }

            let tb = &mut self.app.typeface.serif_box;
            if state == clear_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly) { tb.unfocus(); }
            if tb.mouse_input(button, state, lx, ly) {
                self.needs_rebuild = true;
            }
            if state == clear_ui::widget::ElementState::Pressed && tb.take_change() {
                actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetSerif(tb.text.clone())));
            }

            let tb = &mut self.app.typeface.mono_box;
            if state == clear_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly) { tb.unfocus(); }
            if tb.mouse_input(button, state, lx, ly) {
                self.needs_rebuild = true;
            }
            if state == clear_ui::widget::ElementState::Pressed && tb.take_change() {
                actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetMono(tb.text.clone())));
            }

            let menu = &mut self.app.typeface.borders_menu;
            if state == clear_ui::widget::ElementState::Pressed && !menu.hit_test(lx, ly) { menu.unfocus(); }
            if menu.mouse_input(button, state, lx, ly) {
                self.needs_rebuild = true;
            }
            if state == clear_ui::widget::ElementState::Pressed && menu.take_change() {
                actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetBordersMenu(menu.selected)));
            }

            let tb = &mut self.app.typeface.borders_box;
            if state == clear_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly) { tb.unfocus(); }
            if tb.mouse_input(button, state, lx, ly) {
                self.needs_rebuild = true;
            }
            if state == clear_ui::widget::ElementState::Pressed && tb.take_change() {
                actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetBorders(tb.text.clone())));
            }

            let menu = &mut self.app.typeface.status_menu;
            if state == clear_ui::widget::ElementState::Pressed && !menu.hit_test(lx, ly) { menu.unfocus(); }
            if menu.mouse_input(button, state, lx, ly) {
                self.needs_rebuild = true;
            }
            if state == clear_ui::widget::ElementState::Pressed && menu.take_change() {
                actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetStatusMenu(menu.selected)));
            }

            let tb = &mut self.app.typeface.status_box;
            if state == clear_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly) { tb.unfocus(); }
            if tb.mouse_input(button, state, lx, ly) {
                self.needs_rebuild = true;
            }
            if state == clear_ui::widget::ElementState::Pressed && tb.take_change() {
                actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetStatus(tb.text.clone())));
            }

            let menu = &mut self.app.typeface.fuzzel_menu;
            if state == clear_ui::widget::ElementState::Pressed && !menu.hit_test(lx, ly) { menu.unfocus(); }
            if menu.mouse_input(button, state, lx, ly) {
                self.needs_rebuild = true;
            }
            if state == clear_ui::widget::ElementState::Pressed && menu.take_change() {
                actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetFuzzelMenu(menu.selected)));
            }

            let tb = &mut self.app.typeface.fuzzel_box;
            if state == clear_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly) { tb.unfocus(); }
            if tb.mouse_input(button, state, lx, ly) {
                self.needs_rebuild = true;
            }
            if state == clear_ui::widget::ElementState::Pressed && tb.take_change() {
                actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetFuzzel(tb.text.clone())));
            }

            let menu = &mut self.app.typeface.terminal_menu;
            if state == clear_ui::widget::ElementState::Pressed && !menu.hit_test(lx, ly) { menu.unfocus(); }
            if menu.mouse_input(button, state, lx, ly) {
                self.needs_rebuild = true;
            }
            if state == clear_ui::widget::ElementState::Pressed && menu.take_change() {
                actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetTerminalMenu(menu.selected)));
            }

            let tb = &mut self.app.typeface.terminal_box;
            if state == clear_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly) { tb.unfocus(); }
            if tb.mouse_input(button, state, lx, ly) {
                self.needs_rebuild = true;
            }
            if state == clear_ui::widget::ElementState::Pressed && tb.take_change() {
                actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetTerminal(tb.text.clone())));
            }

            let tb = &mut self.app.typeface.search_box;
            if state == clear_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly) { tb.unfocus(); }
            if tb.mouse_input(button, state, lx, ly) {
                self.needs_rebuild = true;
            }
            if state == clear_ui::widget::ElementState::Pressed && tb.take_change() {
                actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetSearch(tb.text.clone())));
            }

            let sb = &mut self.app.typeface.borders_size_box;
            if state == clear_ui::widget::ElementState::Pressed && !sb.hit_test(lx, ly) { sb.unfocus(); }
            let old_val = sb.value;
            if sb.mouse_input(button, state, lx, ly) {
                self.needs_rebuild = true;
            }
            if sb.value != old_val {
                actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetBordersSize(sb.value)));
            }

            let sb = &mut self.app.typeface.status_size_box;
            if state == clear_ui::widget::ElementState::Pressed && !sb.hit_test(lx, ly) { sb.unfocus(); }
            let old_val = sb.value;
            if sb.mouse_input(button, state, lx, ly) {
                self.needs_rebuild = true;
            }
            if sb.value != old_val {
                actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetStatusSize(sb.value)));
            }

            let sb = &mut self.app.typeface.fuzzel_size_box;
            if state == clear_ui::widget::ElementState::Pressed && !sb.hit_test(lx, ly) { sb.unfocus(); }
            let old_val = sb.value;
            if sb.mouse_input(button, state, lx, ly) {
                self.needs_rebuild = true;
            }
            if sb.value != old_val {
                actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetFuzzelSize(sb.value)));
            }

            let sb = &mut self.app.typeface.terminal_size_box;
            if state == clear_ui::widget::ElementState::Pressed && !sb.hit_test(lx, ly) { sb.unfocus(); }
            let old_val = sb.value;
            if sb.mouse_input(button, state, lx, ly) {
                self.needs_rebuild = true;
            }
            if sb.value != old_val {
                actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetTerminalSize(sb.value)));
            }

            let tf = &mut self.app.typeface;
            let query = tf.search_box.text.to_lowercase();
            let matching_fonts: Vec<String> = tf.all_fonts.iter()
                .filter(|font| font.to_lowercase().contains(&query))
                .cloned()
                .collect();

            let mut clicked_idx = None;
            let mut is_copy = false;

            for (i, btn) in tf.font_buttons.iter_mut().enumerate() {
                if btn.mouse_input(button, state, lx, ly) {
                    self.needs_rebuild = true;
                }
                if btn.take_click() {
                    clicked_idx = Some(i);
                    is_copy = false;
                }
            }
            for (i, btn) in tf.copy_buttons.iter_mut().enumerate() {
                if btn.mouse_input(button, state, lx, ly) {
                    self.needs_rebuild = true;
                }
                if btn.take_click() {
                    clicked_idx = Some(i);
                    is_copy = true;
                }
            }

            if let Some(idx) = clicked_idx {
                if let Some(font_name) = matching_fonts.get(idx) {
                    let action = if is_copy {
                        AppAction::Typeface(pages::typeface::TypefaceMessage::CopyFontName(font_name.clone()))
                    } else {
                        AppAction::Typeface(pages::typeface::TypefaceMessage::SelectFont(font_name.clone()))
                    };
                    actions.push(action);
                }
            }

            if tf.list_box.mouse_input(button, state, lx, ly) {
                self.needs_rebuild = true;
            }
        }
        if self.app.current_page == Page::Services {
            let tb = &mut self.app.services.search_box;
            if state == clear_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly) { tb.unfocus(); }
            if tb.mouse_input(button, state, lx, ly) {
                self.needs_rebuild = true;
            }
            let srv = &mut self.app.services;
            if srv.list_box.mouse_input(button, state, lx, ly) {
                self.needs_rebuild = true;
            }
        }
        if state == clear_ui::widget::ElementState::Pressed && self.app.current_page == Page::Hardware {
            let hw = &mut self.app.hardware;
            if hw.cpu_list_box.mouse_input(button, state, lx, ly) {
                self.needs_rebuild = true;
            }
        }
        if state == clear_ui::widget::ElementState::Pressed && self.app.current_page == Page::Radios {
            let net = &mut self.app.network;
            if net.wifi_list_box.mouse_input(button, state, lx, ly) {
                self.needs_rebuild = true;
            }
        }
        for a in &actions {
            self.handle_action(a);
        }
        if !actions.is_empty() {
            self.needs_rebuild = true;
            return true;
        }
        self.needs_rebuild = true;
        true
    }

    fn handle_mouse_wheel_internal(&mut self, delta: &clear_ui::widget::MouseScrollDelta, px: f32, py: f32) -> bool {
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("/tmp/clear-scroll-debug.txt") {
            use std::io::Write;
            let _ = writeln!(file, "handle_mouse_wheel_internal: px={}, py={}, sidebar_w={}", px, py, self.sidebar_width);
        }
        let s = 1.0f32;
        if px >= self.sidebar_width * s {
            let lx = px / s;
            let ly = py / s + self.scroll_y;
            
            if self.app.current_page == Page::Input {
                let input = &self.app.input;
                if input.is_over_trackpad(lx, ly) {
                    return true;
                }
            }

            if self.app.current_page == Page::Typefaces {
                let tf = &mut self.app.typeface;
                if tf.list_box.mouse_wheel(delta, lx, ly) {
                    self.needs_rebuild = true;
                    return true;
                }
            }
            if self.app.current_page == Page::Services {
                let srv = &mut self.app.services;
                if srv.list_box.mouse_wheel(delta, lx, ly) {
                    self.needs_rebuild = true;
                    return true;
                }
            }
            if self.app.current_page == Page::Hardware {
                let hw = &mut self.app.hardware;
                if hw.cpu_list_box.mouse_wheel(delta, lx, ly) {
                    self.needs_rebuild = true;
                    return true;
                }
            }
            if self.app.current_page == Page::Radios {
                let net = &mut self.app.network;
                if net.wifi_list_box.mouse_wheel(delta, lx, ly) {
                    self.needs_rebuild = true;
                    return true;
                }
            }

            let scroll_speed = 24.0;
            let dy = match delta {
                clear_ui::widget::MouseScrollDelta::LineDelta(_, y) => -y * scroll_speed,
                clear_ui::widget::MouseScrollDelta::PixelDelta(pos) => -pos.y as f32,
            };
            let old_scroll = self.scroll_y;
            self.scroll_y = (self.scroll_y + dy).max(0.0).min(self.max_scroll_y);
            if (self.scroll_y - old_scroll).abs() > 0.01 {
                self.needs_rebuild = true;
                return true;
            }
        } else {
            let lx = px / s;
            let ly = py / s;
            if self.paginator.mouse_wheel(delta, lx, ly) {
                self.needs_rebuild = true;
                return true;
            }
        }
        false
    }

    fn get_page_root_widget(&mut self) -> Option<*mut (dyn clear_ui::widget::Widget + 'static)> {
        match self.app.current_page {
            Page::Typefaces | Page::Services | Page::Hardware | Page::Radios |
            Page::Layout | Page::Colors | Page::Notifications | Page::Input |
            Page::Audio | Page::Display => {
                let ptr = &mut self.page_root_container as &mut dyn clear_ui::widget::Widget as *mut dyn clear_ui::widget::Widget;
                let static_ptr = unsafe {
                    std::mem::transmute::<*mut dyn clear_ui::widget::Widget, *mut (dyn clear_ui::widget::Widget + 'static)>(ptr)
                };
                Some(static_ptr)
            }
            _ => None,
        }
    }

    fn handle_key_input_internal(&mut self, event: &clear_ui::widget::KeyEvent) -> bool {
        if event.state == clear_ui::widget::ElementState::Pressed && !event.repeat {
            let is_nav_key = match (&event.logical_key, event.ctrl) {
                (clear_ui::widget::Key::Character(c), true) if c == "j" || c == "J" || c == "k" || c == "K" || c == "u" || c == "U" || c == "i" || c == "I" => true,
                _ => false,
            };
            if is_nav_key {
                if clear_ui::widget::focus::has_focus() {
                    if clear_ui::widget::focus::navigate_focus(&event.logical_key, event.ctrl) {
                        self.needs_rebuild = true;
                        return true;
                    }
                } else {
                    if let Some(root_ptr) = self.get_page_root_widget() {
                        unsafe {
                            let root_ref = &mut *root_ptr;
                            clear_ui::widget::focus::set_focused(root_ref);
                            root_ref.focus();
                            self.needs_rebuild = true;
                            return true;
                        }
                    }
                }
            }
        }

        if self.app.current_page == Page::Layout {
            let mut changed = false;
            let mut actions = Vec::new();
            for (i, sb) in self.app.layout.spinboxes.iter_mut().enumerate() {
                let old = sb.value;
                if sb.keyboard_input(event) {
                    if sb.value != old {
                        actions.push(AppAction::Layout(
                            pages::layout::LayoutMessage::SetWidth(
                                pages::layout::WidthParam::ALL[i],
                                sb.value as u16,
                            )
                        ));
                    }
                    changed = true;
                }
            }
            let sb = &mut self.app.layout.cascade_offset_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event) {
                if sb.value != old {
                    actions.push(AppAction::Layout(
                        pages::layout::LayoutMessage::SetCascadeOffset(sb.value as u16)
                    ));
                }
                changed = true;
            }
            let sb = &mut self.app.layout.edge_gap_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event) {
                if sb.value != old {
                    actions.push(AppAction::Layout(
                        pages::layout::LayoutMessage::SetEdgeGap(sb.value as u16)
                    ));
                }
                changed = true;
            }
            let sb = &mut self.app.layout.top_gap_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event) {
                if sb.value != old {
                    actions.push(AppAction::Layout(
                        pages::layout::LayoutMessage::SetTopGap(sb.value as u16)
                    ));
                }
                changed = true;
            }
            let sb = &mut self.app.layout.grid_gap_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event) {
                if sb.value != old {
                    actions.push(AppAction::Layout(
                        pages::layout::LayoutMessage::SetGridGap(sb.value as u16)
                    ));
                }
                changed = true;
            }
            let sb = &mut self.app.layout.transition_duration_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event) {
                if sb.value != old {
                    actions.push(AppAction::Layout(
                        pages::layout::LayoutMessage::SetTransitionDuration(sb.value as u16)
                    ));
                }
                changed = true;
            }
            let sb = &mut self.app.layout.status_height_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event) {
                if sb.value != old {
                    actions.push(AppAction::Layout(
                        pages::layout::LayoutMessage::SetStatusHeight(sb.value as u16)
                    ));
                }
                changed = true;
            }
            for a in &actions {
                self.handle_action(a);
            }
            if changed {
                self.needs_rebuild = true;
                return true;
            }
        }
        if self.app.current_page == Page::Colors {
            let mut changed = false;
            let mut actions = Vec::new();
            for (i, cp) in self.app.colors.color_selectors.iter_mut().enumerate() {
                let old = cp.color;
                if cp.keyboard_input(event) {
                    if cp.color != old {
                        actions.push(AppAction::Colors(match i {
                            0 => pages::colors::ColorsMessage::SetPageLowColor(cp.color),
                            1 => pages::colors::ColorsMessage::SetHighColor(cp.color),
                            2 => pages::colors::ColorsMessage::SetVisualGuidesColor(cp.color),
                            3 => pages::colors::ColorsMessage::SetDisabledColor(cp.color),
                            4 => pages::colors::ColorsMessage::SetSeparatorColor(cp.color),
                            5 => pages::colors::ColorsMessage::SetSliderTrackColor(cp.color),
                            6 => pages::colors::ColorsMessage::SetColorBordersColor(cp.color),
                            7 => pages::colors::ColorsMessage::SetLowColor(cp.color),
                            8 => pages::colors::ColorsMessage::SetNormalColor(cp.color),
                            9 => pages::colors::ColorsMessage::SetPaginatorSidebarColor(cp.color),
                            10 => pages::colors::ColorsMessage::SetPrimaryHighlightColor(cp.color),
                            11 => pages::colors::ColorsMessage::SetPaginatorTabLabelColor(cp.color),
                            _ => pages::colors::ColorsMessage::SetLowColor(cp.color),
                        }));
                    }
                    changed = true;
                }
            }
            for a in &actions {
                self.handle_action(a);
            }
            if changed {
                self.needs_rebuild = true;
                return true;
            }
        }
        if self.app.current_page == Page::Notifications {
            let sb = &mut self.app.notifications.duration_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event) {
                let new_val = sb.value;
                drop(sb);
                if new_val != old {
                    self.handle_action(&AppAction::Notifications(pages::notifications::NotificationsMessage::SetDuration(new_val)));
                }
                self.needs_rebuild = true;
                return true;
            }
        }
        if self.app.current_page == Page::Input {
            if self.app.input.rate_spinbox.keyboard_input(event) {
                self.handle_action(&AppAction::Input(pages::input::InputMessage::ApplyRepeat));
                self.needs_rebuild = true;
                return true;
            }
            if self.app.input.delay_spinbox.keyboard_input(event) {
                self.handle_action(&AppAction::Input(pages::input::InputMessage::ApplyRepeat));
                self.needs_rebuild = true;
                return true;
            }
            if self.app.input.scroll_friction_spinbox.keyboard_input(event) {
                self.handle_action(&AppAction::Input(pages::input::InputMessage::ApplyScrollFriction));
                self.needs_rebuild = true;
                return true;
            }
            if self.app.input.scroll_speed_spinbox.keyboard_input(event) {
                self.handle_action(&AppAction::Input(pages::input::InputMessage::ApplyScrollSpeed));
                self.needs_rebuild = true;
                return true;
            }
            if self.app.input.pointer_friction_spinbox.keyboard_input(event) {
                self.handle_action(&AppAction::Input(pages::input::InputMessage::ApplyPointerFriction));
                self.needs_rebuild = true;
                return true;
            }
            if self.app.input.trackpad_friction_spinbox.keyboard_input(event) {
                self.handle_action(&AppAction::Input(pages::input::InputMessage::ApplyTrackpadFriction));
                self.needs_rebuild = true;
                return true;
            }
            if self.app.input.trackpoint_accel_speed_spinbox.keyboard_input(event) {
                self.handle_action(&AppAction::Input(pages::input::InputMessage::ApplyTrackpointAccelSpeed));
                self.needs_rebuild = true;
                return true;
            }
            if self.app.input.cursor_size_spinbox.keyboard_input(event) {
                self.handle_action(&AppAction::Input(pages::input::InputMessage::ApplyCursorSize));
                self.needs_rebuild = true;
                return true;
            }
        }
        if self.app.current_page == Page::Audio {
            let mut actions = Vec::new();
            for (i, sb) in self.app.audio.sink_spinboxes.iter_mut().enumerate() {
                let old = sb.value;
                if sb.keyboard_input(event) {
                    if sb.value != old {
                        let id = self.app.audio.sinks[i].id;
                        actions.push(AppAction::Audio(pages::audio::AudioMessage::SinkVolume(id, sb.value as f32 / 100.0)));
                    }
                }
            }
            for (i, sb) in self.app.audio.source_spinboxes.iter_mut().enumerate() {
                let old = sb.value;
                if sb.keyboard_input(event) {
                    if sb.value != old {
                        let id = self.app.audio.sources[i].id;
                        actions.push(AppAction::Audio(pages::audio::AudioMessage::SourceVolume(id, sb.value as f32 / 100.0)));
                    }
                }
            }
            for a in &actions {
                self.handle_action(a);
            }
            if !actions.is_empty() {
                self.needs_rebuild = true;
                return true;
            }
        }
        if self.app.current_page == Page::Display {
            let sb = &mut self.app.display.brightness_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event) {
                let new_val = sb.value;
                drop(sb);
                if new_val != old {
                    self.handle_action(&AppAction::Display(pages::display::DisplayMessage::BrightnessSet(new_val as u32)));
                }
                self.needs_rebuild = true;
                return true;
            }
        }
        if self.app.current_page == Page::Status {
            let sb = &mut self.app.status.padding_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event) {
                let new_val = sb.value;
                drop(sb);
                if new_val != old {
                    self.handle_action(&AppAction::Status(pages::status::StatusMessage::SetPadding(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
        }
        if self.app.current_page == Page::Typefaces {
            if event.state == clear_ui::widget::ElementState::Pressed {
                let is_down = match (&event.logical_key, event.ctrl) {
                    (clear_ui::widget::Key::Character(c), true) if c == "n" || c == "N" => true,
                    (clear_ui::widget::Key::Named(clear_ui::widget::NamedKey::ArrowDown), false) => true,
                    _ => false,
                };
                let is_up = match (&event.logical_key, event.ctrl) {
                    (clear_ui::widget::Key::Character(c), true) if c == "p" || c == "P" => true,
                    (clear_ui::widget::Key::Named(clear_ui::widget::NamedKey::ArrowUp), false) => true,
                    _ => false,
                };
                if is_down && clear_ui::widget::focus::is_focused(&self.app.typeface.list_box.scroll_box) {
                    let next_idx_font_scroll = {
                        let tf = &self.app.typeface;
                        let query = tf.search_box.text.to_lowercase();
                        let matching_fonts: Vec<&String> = tf.all_fonts.iter()
                            .filter(|font| font.to_lowercase().contains(&query))
                            .collect();
                        if !matching_fonts.is_empty() {
                            let current_idx = tf.selected_font.as_ref()
                                .and_then(|f| matching_fonts.iter().position(|&x| x == f));
                            let next_idx = match current_idx {
                                Some(idx) => (idx + 1).min(matching_fonts.len() - 1),
                                None => 0,
                            };
                            let font = matching_fonts[next_idx].clone();
                            
                            // Compute scroll
                            let btn_h = 24.0;
                            let btn_gap = 4.0;
                            let item_height_full = btn_h + btn_gap;
                            let item_y = next_idx as f32 * item_height_full;
                            let list_box_h = 320.0;
                            
                            let mut scroll_y = tf.list_box.scroll_y();
                            if item_y < scroll_y {
                                scroll_y = item_y;
                            } else if item_y + btn_h > scroll_y + list_box_h {
                                scroll_y = item_y + btn_h - list_box_h;
                            }
                            Some((font, scroll_y))
                        } else {
                            None
                        }
                    };

                    if let Some((font, scroll_y)) = next_idx_font_scroll {
                        self.app.typeface.list_box.set_scroll_y(scroll_y);
                        self.handle_action(&AppAction::Typeface(pages::typeface::TypefaceMessage::SelectFont(font)));
                        self.needs_rebuild = true;
                        return true;
                    }
                } else if is_up && clear_ui::widget::focus::is_focused(&self.app.typeface.list_box.scroll_box) {
                    let next_idx_font_scroll = {
                        let tf = &self.app.typeface;
                        let query = tf.search_box.text.to_lowercase();
                        let matching_fonts: Vec<&String> = tf.all_fonts.iter()
                            .filter(|font| font.to_lowercase().contains(&query))
                            .collect();
                        if !matching_fonts.is_empty() {
                            let current_idx = tf.selected_font.as_ref()
                                .and_then(|f| matching_fonts.iter().position(|&x| x == f));
                            let next_idx = match current_idx {
                                Some(idx) => idx.saturating_sub(1),
                                None => 0,
                            };
                            let font = matching_fonts[next_idx].clone();
                            
                            // Compute scroll
                            let btn_h = 24.0;
                            let btn_gap = 4.0;
                            let item_height_full = btn_h + btn_gap;
                            let item_y = next_idx as f32 * item_height_full;
                            let list_box_h = 320.0;
                            
                            let mut scroll_y = tf.list_box.scroll_y();
                            if item_y < scroll_y {
                                scroll_y = item_y;
                            } else if item_y + btn_h > scroll_y + list_box_h {
                                scroll_y = item_y + btn_h - list_box_h;
                            }
                            Some((font, scroll_y))
                        } else {
                            None
                        }
                    };

                    if let Some((font, scroll_y)) = next_idx_font_scroll {
                        self.app.typeface.list_box.set_scroll_y(scroll_y);
                        self.handle_action(&AppAction::Typeface(pages::typeface::TypefaceMessage::SelectFont(font)));
                        self.needs_rebuild = true;
                        return true;
                    }
                }
            }

            let mut actions = Vec::new();
            let mut consumed = false;
            
            let tf = &mut self.app.typeface;
            let tb = &mut tf.sans_box;
            if tb.keyboard_input(event) {
                if tb.take_change() {
                    actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetSans(tb.text.clone())));
                }
                consumed = true;
            }

            let tb = &mut self.app.typeface.serif_box;
            if tb.keyboard_input(event) {
                if tb.take_change() {
                    actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetSerif(tb.text.clone())));
                }
                consumed = true;
            }

            let tb = &mut self.app.typeface.mono_box;
            if tb.keyboard_input(event) {
                if tb.take_change() {
                    actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetMono(tb.text.clone())));
                }
                consumed = true;
            }

            let tb = &mut self.app.typeface.borders_box;
            if tb.keyboard_input(event) {
                if tb.take_change() {
                    actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetBorders(tb.text.clone())));
                }
                consumed = true;
            }

            let tb = &mut self.app.typeface.status_box;
            if tb.keyboard_input(event) {
                if tb.take_change() {
                    actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetStatus(tb.text.clone())));
                }
                consumed = true;
            }

            let tb = &mut self.app.typeface.fuzzel_box;
            if tb.keyboard_input(event) {
                if tb.take_change() {
                    actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetFuzzel(tb.text.clone())));
                }
                consumed = true;
            }

            let tb = &mut self.app.typeface.terminal_box;
            if tb.keyboard_input(event) {
                if tb.take_change() {
                    actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetTerminal(tb.text.clone())));
                }
                consumed = true;
            }

            let tb = &mut self.app.typeface.search_box;
            if tb.keyboard_input(event) {
                if tb.take_change() {
                    actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetSearch(tb.text.clone())));
                }
                consumed = true;
            }

            let sb = &mut self.app.typeface.borders_size_box;
            if sb.keyboard_input(event) {
                actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetBordersSize(sb.value)));
                consumed = true;
            }

            let sb = &mut self.app.typeface.status_size_box;
            if sb.keyboard_input(event) {
                actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetStatusSize(sb.value)));
                consumed = true;
            }

            let sb = &mut self.app.typeface.fuzzel_size_box;
            if sb.keyboard_input(event) {
                actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetFuzzelSize(sb.value)));
                consumed = true;
            }

            let sb = &mut self.app.typeface.terminal_size_box;
            if sb.keyboard_input(event) {
                actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetTerminalSize(sb.value)));
                consumed = true;
            }
            
            for a in &actions {
                self.handle_action(a);
            }
            if consumed {
                self.needs_rebuild = true;
                return true;
            }
        }
        if self.app.current_page == Page::Services {
            let srv = &mut self.app.services;
            if srv.list_box.keyboard_input(event) {
                self.needs_rebuild = true;
                return true;
            }
            let tb = &mut srv.search_box;
            if tb.keyboard_input(event) {
                tb.take_change();
                self.needs_rebuild = true;
                return true;
            }
        }
        if self.app.current_page == Page::Hardware {
            let hw = &mut self.app.hardware;
            if hw.cpu_list_box.keyboard_input(event) {
                self.needs_rebuild = true;
                return true;
            }
        }
        if self.app.current_page == Page::Radios {
            let net = &mut self.app.network;
            if net.wifi_list_box.keyboard_input(event) {
                self.needs_rebuild = true;
                return true;
            }
        }
        false
    }

}

fn main() {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let _guard = rt.enter();

    let mut initial_page = Page::ALL[0];
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        let arg = args.last().unwrap().to_lowercase();
        for page in Page::ALL {
            if page.label().to_lowercase() == arg {
                initial_page = page;
                break;
            }
        }
    }

    let initial_page_idx = Page::ALL.iter().position(|&p| p == initial_page).unwrap_or(0);
    INITIAL_PAGE_INDEX.store(initial_page_idx, std::sync::atomic::Ordering::SeqCst);

    clear_ui::engine::run::<SystemInterface>();
}
