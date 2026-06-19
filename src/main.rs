use cce_ui::widget::{Finger, hover_animation, TextItem, Element, PageSelector};
use glyphon::{Attrs, Buffer, FontSystem, Metrics};

use cce_system_interface::app::{AppAction, AppState, PageContent};
use cce_system_interface::pages::{self, Page};

fn make_text_buffer(fs: &mut FontSystem, text: &str, size: f32) -> Buffer {
    let scale = cce_ui::scale::scale_factor();
    let physical_size = size * scale;
    let metrics = Metrics::new(physical_size, physical_size * 1.4);
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
    let scale = cce_ui::scale::scale_factor();
    let mut font_size = size;
    let mut family_name = None;

    if let Some(font_str) = font {
        let (parsed_family, parsed_size) = cce_ui::layout::parse_font_string(font_str);
        if let Some(ps) = parsed_size {
            font_size = ps;
        }
        family_name = Some(parsed_family);
    }

    let physical_size = font_size * scale;
    let metrics = Metrics::new(physical_size, physical_size * 1.4);
    let mut buf = Buffer::new(fs, metrics);
    let mut attrs = Attrs::new();

    let resolved_storage = family_name.as_deref().and_then(|font_name| match font_name {
        "monospace" if !mono_fallback.is_empty() => find_cased_family(fs, mono_fallback),
        "sans-serif" if !sans_fallback.is_empty() => find_cased_family(fs, sans_fallback),
        "serif" if !serif_fallback.is_empty() => find_cased_family(fs, serif_fallback),
        _ => None,
    });

    if let Some(font_name) = family_name.as_deref() {
        let family = match font_name {
            "monospace" => {
                if !mono_fallback.is_empty() {
                    if let Some(ref cased) = resolved_storage {
                        glyphon::Family::Name(cased)
                    } else {
                        glyphon::Family::Name(mono_fallback)
                    }
                } else {
                    glyphon::Family::Name(cce_ui::layout::get_system_monospace_font())
                }
            }
            "sans-serif" => {
                if !sans_fallback.is_empty() {
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
    radius: f32,
    corners: (bool, bool, bool, bool),
}

static INITIAL_PAGE_INDEX: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

struct SystemInterface {
    app: AppState,
    font_system: FontSystem,
    widgets: Vec<AppWidget>,
    text_items: Vec<cce_ui::widget::TextItem>,
    page_buttons: Vec<(cce_ui::widget::Button, AppAction)>,

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
    rx_status: std::sync::mpsc::Receiver<pages::services::StatusData>,
    rx_storage: std::sync::mpsc::Receiver<pages::storage::StorageState>,
    rx_notifications: std::sync::mpsc::Receiver<pages::services::NotificationsConfig>,
    rx_typeface: std::sync::mpsc::Receiver<pages::interface::InterfaceState>,
    rx_services: std::sync::mpsc::Receiver<Vec<pages::services::ServiceInfo>>,
    rx_interface: std::sync::mpsc::Receiver<pages::interface::InterfaceState>,
    rx_accounts: std::sync::mpsc::Receiver<Vec<pages::accounts::AccountInfo>>,
    tx_backup: std::sync::mpsc::Sender<pages::storage::StorageMessage>,
    rx_backup: std::sync::mpsc::Receiver<pages::storage::StorageMessage>,
    rx_packages: std::sync::mpsc::Receiver<pages::packages::PackagesState>,
    tx_update: std::sync::mpsc::Sender<pages::packages::PackagesMessage>,
    rx_update: std::sync::mpsc::Receiver<pages::packages::PackagesMessage>,


    scale_factor: f64,
    width: u32,
    height: u32,
    needs_rebuild: bool,
    scroll_y: f32,
    max_scroll_y: f32,
    audio_sink_dragging: Option<usize>,
    audio_source_dragging: Option<usize>,
    display_brightness_dragging: bool,
    page_sec_containers: Vec<cce_ui::widget::Container>,
    root_window: cce_ui::widget::Window,
    menubar: cce_ui::widget::MenuBar,
    switcher: cce_ui::widget::Switcher,
    plates: Vec<cce_ui::widget::Plate>,
    sans_serif_family: String,
    serif_family: String,
    monospace_family: String,
    current_page_shared: std::sync::Arc<std::sync::atomic::AtomicU8>,
    sender: calloop::channel::Sender<AppAction>,
    ui_context: cce_ui::context::UiContext,
}

impl cce_ui::engine::Application for SystemInterface {
    type Message = AppAction;

    fn new(_qh: &wayland_client::QueueHandle<cce_ui::engine::EngineState<Self>>, sender: calloop::channel::Sender<Self::Message>) -> Self {
        cce_ui::scale::set_scale_factor(1.0);
        let app = AppState {
            layout: pages::layout::read_layout_config(),
            input: pages::input::read_input_config(),
            interface: pages::interface::read_interface_config(),
            ..Default::default()
        };

        // ── Background refresh channels ──
        let initial_page_idx = INITIAL_PAGE_INDEX.load(std::sync::atomic::Ordering::SeqCst);
        let current_page_shared = std::sync::Arc::new(std::sync::atomic::AtomicU8::new(initial_page_idx as u8));

        fn spawn_bg_active<T, F>(
            current_page_shared: std::sync::Arc<std::sync::atomic::AtomicU8>,
            target_page_idx: u8,
            period_secs: u64,
            f: fn() -> F,
        ) -> std::sync::mpsc::Receiver<T>
        where
            T: Send + 'static,
            F: std::future::Future<Output = T> + Send + 'static,
        {
            let (tx, rx) = std::sync::mpsc::channel::<T>();
            tokio::spawn(async move {
                let mut last_fetch: Option<std::time::Instant> = None;
                loop {
                    let current_page = current_page_shared.load(std::sync::atomic::Ordering::SeqCst);
                    if current_page == target_page_idx {
                        let should_fetch = match last_fetch {
                            None => true,
                            Some(t) => t.elapsed() >= std::time::Duration::from_secs(period_secs),
                        };
                        if should_fetch {
                            let val = f().await;
                            if tx.send(val).is_err() { break; }
                            last_fetch = Some(std::time::Instant::now());
                        }
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                }
            });
            rx
        }

        let rx_audio = spawn_bg_active(current_page_shared.clone(), 1, 3, || pages::audio::fetch_audio_state());
        let rx_display = spawn_bg_active(current_page_shared.clone(), 2, 10, || pages::display::fetch_display_state());
        let rx_network = spawn_bg_active(current_page_shared.clone(), 7, 5, || pages::network::fetch_network_state());
        let rx_layout = {
            let (tx, rx) = std::sync::mpsc::channel::<pages::layout::LayoutState>();
            let current_page_shared = current_page_shared.clone();
            tokio::spawn(async move {
                let mut last_fetch: Option<std::time::Instant> = None;
                loop {
                    let current_page = current_page_shared.load(std::sync::atomic::Ordering::SeqCst);
                    if current_page == 11 { // Layout is index 11
                        let should_fetch = match last_fetch {
                            None => true,
                            Some(t) => t.elapsed() >= std::time::Duration::from_secs(30),
                        };
                        if should_fetch {
                            let val = tokio::task::spawn_blocking(|| pages::layout::read_layout_config()).await;
                            if let Ok(val) = val {
                                if tx.send(val).is_err() { break; }
                            }
                            last_fetch = Some(std::time::Instant::now());
                        }
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                }
            });
            rx
        };
        let rx_wm_events = {
            let (tx, rx) = std::sync::mpsc::channel::<()>();
            let current_page_shared = current_page_shared.clone();
            tokio::spawn(async move {
                let display = std::env::var("WAYLAND_DISPLAY").unwrap_or_else(|_| "wayland-0".to_string());
                let windows_path = format!("/tmp/cce-windows-{}", display);
                let tags_path = format!("/tmp/cce-tags-{}", display);
                let title_path = format!("/tmp/cce-title-{}", display);
                
                let mut last_mod = std::time::SystemTime::UNIX_EPOCH;
                
                let check_mtime = |path: &str| -> Option<std::time::SystemTime> {
                    std::fs::metadata(path).and_then(|m| m.modified()).ok()
                };

                loop {
                    let current_page = current_page_shared.load(std::sync::atomic::Ordering::SeqCst);
                    if current_page == 11 { // Layout is index 11
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
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
            });
            rx
        };
        let rx_input = {
            let (tx, rx) = std::sync::mpsc::channel::<pages::input::InputState>();
            let current_page_shared = current_page_shared.clone();
            tokio::spawn(async move {
                let mut last_fetch: Option<std::time::Instant> = None;
                loop {
                    let current_page = current_page_shared.load(std::sync::atomic::Ordering::SeqCst);
                    if current_page == 4 { // Input is index 4
                        let should_fetch = match last_fetch {
                            None => true,
                            Some(t) => t.elapsed() >= std::time::Duration::from_secs(30),
                        };
                        if should_fetch {
                            let val = tokio::task::spawn_blocking(|| pages::input::read_input_config()).await;
                            if let Ok(val) = val {
                                if tx.send(val).is_err() { break; }
                            }
                            last_fetch = Some(std::time::Instant::now());
                        }
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                }
            });
            rx
        };
        let rx_fingers = {
            let (tx, rx) = std::sync::mpsc::channel::<Vec<Finger>>();
            let current_page_shared = current_page_shared.clone();
            tokio::spawn(async move {
                let socket_path = match std::env::var("WAYLAND_DISPLAY") {
                    Ok(display) => format!("/tmp/cce-input-coords-{}.sock", display),
                    Err(_) => "/tmp/cce-input-coords.sock".to_string(),
                };
                loop {
                    let current_page = current_page_shared.load(std::sync::atomic::Ordering::SeqCst);
                    if current_page == 4 { // Input is index 4
                        if let Ok(stream) = tokio::net::UnixStream::connect(&socket_path).await {
                            use tokio::io::AsyncBufReadExt;
                            let reader = tokio::io::BufReader::new(stream);
                            let mut lines = reader.lines();
                            while let Ok(Some(line)) = lines.next_line().await {
                                if current_page_shared.load(std::sync::atomic::Ordering::SeqCst) != 4 {
                                    break;
                                }
                                if let Ok(fingers) = serde_json::from_str::<Vec<Finger>>(&line) {
                                    if tx.send(fingers).is_err() {
                                        return;
                                    }
                                }
                            }
                        }
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
            });
            rx
        };
        let rx_system = spawn_bg_active(current_page_shared.clone(), 10, 5, || pages::system_info::fetch_system_state());
        let rx_hardware = spawn_bg_active(current_page_shared.clone(), 3, 3, || pages::hardware::fetch_hardware_state());
        let rx_status = spawn_bg_active(current_page_shared.clone(), 8, 10, || pages::services::fetch_status_state());
        let rx_storage = spawn_bg_active(current_page_shared.clone(), 9, 10, || pages::storage::fetch_storage_state());
        let rx_notifications = {
            let (tx, rx) = std::sync::mpsc::channel::<pages::services::NotificationsConfig>();
            let current_page_shared = current_page_shared.clone();
            tokio::spawn(async move {
                let mut last_fetch: Option<std::time::Instant> = None;
                loop {
                    let current_page = current_page_shared.load(std::sync::atomic::Ordering::SeqCst);
                    if current_page == 8 { // Services is index 8
                        let should_fetch = match last_fetch {
                            None => true,
                            Some(t) => t.elapsed() >= std::time::Duration::from_secs(30),
                        };
                        if should_fetch {
                            let val = tokio::task::spawn_blocking(|| pages::services::read_notifications_config()).await;
                            if let Ok(val) = val {
                                if tx.send(val).is_err() { break; }
                            }
                            last_fetch = Some(std::time::Instant::now());
                        }
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                }
            });
            rx
        };
        let rx_typeface = spawn_bg_active(current_page_shared.clone(), 5, 30, || pages::interface::fetch_typeface_state());
        let rx_services = spawn_bg_active(current_page_shared.clone(), 8, 3, || pages::services::fetch_services());
        let rx_accounts = spawn_bg_active(current_page_shared.clone(), 0, 3, || pages::accounts::fetch_accounts());
        let rx_interface = {
            let (tx, rx) = std::sync::mpsc::channel::<pages::interface::InterfaceState>();
            let current_page_shared = current_page_shared.clone();
            tokio::spawn(async move {
                let mut last_fetch: Option<std::time::Instant> = None;
                loop {
                    let current_page = current_page_shared.load(std::sync::atomic::Ordering::SeqCst);
                    if current_page == 5 { // Interface is index 5
                        let should_fetch = match last_fetch {
                            None => true,
                            Some(t) => t.elapsed() >= std::time::Duration::from_secs(30),
                        };
                        if should_fetch {
                            let val = tokio::task::spawn_blocking(|| pages::interface::read_interface_config()).await;
                            if let Ok(val) = val {
                                if tx.send(val).is_err() { break; }
                            }
                            last_fetch = Some(std::time::Instant::now());
                        }
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                }
            });
            rx
        };
        let (tx_backup, rx_backup) = std::sync::mpsc::channel();
        let rx_packages = spawn_bg_active(current_page_shared.clone(), 6, 30, || pages::packages::fetch_packages_state());
        let (tx_update, rx_update) = std::sync::mpsc::channel();

        let (sans_family, serif_family, monospace_family, _, _, _, _, _) = pages::interface::read_preferred_fonts();

        let pages_names = Page::ALL.iter().map(|p| p.label().to_string()).collect::<Vec<_>>();
        let mut menubar = cce_ui::widget::MenuBar::new(0.0, 0.0, 180.0, 680.0)
            .with_vertical(true);
        menubar.set_pages(pages_names);
        let sidebar_width = menubar.sidebar_w();

        let switcher = cce_ui::widget::Switcher::new(sidebar_width, 0.0, 820.0 - sidebar_width, 680.0);
        let mut plates = Vec::new();
        for page in Page::ALL.iter() {
            let plate = cce_ui::widget::Plate::new(sidebar_width, 0.0, 820.0 - sidebar_width, 680.0)
                .with_label(page.label())
                .with_draggable(false);
            plates.push(plate);
        }

        let mut app_state = app;
        app_state.current_page = Page::ALL[initial_page_idx];

        let win_color = app_state.interface.window_color;
        let win_opacity = app_state.interface.window_opacity;
        let win_radius = app_state.interface.window_corner_radius;

        let mut font_system = FontSystem::new();
        font_system.db_mut().load_fonts_dir("/home/lsgalante/Dropbox/Fonts");

        let mut this = Self {
            app: app_state,
            font_system,
            widgets: Vec::new(),
            text_items: Vec::new(),
            page_buttons: Vec::new(),
            sidebar_width,
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
            rx_typeface,
            rx_services,
            rx_interface,
            rx_accounts,
            tx_backup,
            rx_backup,
            rx_packages,
            tx_update,
            rx_update,

            scale_factor: 1.0,
            width: 552,
            height: 1128,
            needs_rebuild: true,
            scroll_y: 4000.0,
            max_scroll_y: 0.0,
            audio_sink_dragging: None,
            audio_source_dragging: None,
            display_brightness_dragging: false,
            page_sec_containers: Vec::new(),
            root_window: cce_ui::widget::Window::new(0.0, 0.0, 820.0, 680.0)
                .with_background([
                    win_color[0] as f32 / 255.0,
                    win_color[1] as f32 / 255.0,
                    win_color[2] as f32 / 255.0,
                    win_opacity,
                ])
                .with_border([0.22, 0.22, 0.28, 1.0], 1.5)
                .with_radius(win_radius as f32),
            menubar,
            switcher,
            plates,
            sans_serif_family: sans_family,
            serif_family,
            monospace_family,
            current_page_shared,
            sender,
            ui_context: cce_ui::context::UiContext::new(),
        };

        for plate in &mut this.plates {
            this.switcher.add_child(plate.as_ptr(), &mut this.ui_context);
        }

        this.root_window.add_child(this.menubar.as_ptr(), &mut this.ui_context);
        this.root_window.add_child(this.switcher.as_ptr(), &mut this.ui_context);

        this.rebuild_layout(820.0, 680.0);
        this
    }

    fn settings(&self) -> cce_ui::engine::WindowSettings {
        cce_ui::engine::WindowSettings {
            title: "CCE System Interface".to_string(),
            app_id: "cce-system-interface".to_string(),
            width: 820,
            height: 680,
            fullscreen: false,
            min_size: Some((400, 680)),
        }
    }

    fn update(&mut self, msg: Self::Message, needs_rebuild: &mut bool, exit: &mut bool) {
        if matches!(msg, AppAction::Exit) {
            *exit = true;
            return;
        }
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

    fn view(&mut self, _quads: &mut Vec<(f32, f32, f32, f32, [f32; 4])>, size: cce_ui::engine::LogicalSize, scale: f64) {
        let (width, height) = (size.width, size.height);
        if self.needs_rebuild || self.width != width as u32 || self.height != height as u32 || self.scale_factor != scale {
            self.width = width as u32;
            self.height = height as u32;
            self.scale_factor = scale;
            cce_ui::scale::set_scale_factor(scale as f32);
            self.rebuild_layout(width, height);
        }
    }

    fn view_rounded_quads(&mut self, quads: &mut Vec<(f32, f32, f32, f32, f32, [f32; 4], (bool, bool, bool, bool))>, size: cce_ui::engine::LogicalSize, scale: f64) {
        let (width, height) = (size.width, size.height);
        if self.needs_rebuild || self.width != width as u32 || self.height != height as u32 || self.scale_factor != scale {
            self.width = width as u32;
            self.height = height as u32;
            self.scale_factor = scale;
            cce_ui::scale::set_scale_factor(scale as f32);
            self.rebuild_layout(width, height);
        }
        for w in &self.widgets {
            quads.push((w.x, w.y, w.w, w.h, w.radius, w.color, w.corners));
        }
    }

    fn text_items(&self) -> &[cce_ui::widget::TextItem] {
        &self.text_items
    }

    fn render_popovers(&self, pc: &mut dyn cce_ui::layout::RenderTarget) {
        cce_ui::layout::render_popovers(pc, &self.ui_context);

        if cce_ui::widget::context_menu::is_visible() {
            let cx = cce_ui::widget::context_menu::x();
            let cy = cce_ui::widget::context_menu::y();
            let cw = cce_ui::widget::context_menu::w();
            let ch = cce_ui::widget::context_menu::h();

            // Border
            pc.rect([0.22, 0.22, 0.28, 1.0], cx, cy, cw, ch);
            // Bg
            pc.rect([0.06, 0.06, 0.09, 1.0], cx + 1.0, cy + 1.0, cw - 2.0, ch - 2.0);

            // Hover highlight
            if let Some(h_idx) = cce_ui::widget::context_menu::hovered_item() {
                let iy = cy + h_idx as f32 * 24.0;
                pc.rect([0.20, 0.40, 0.65, 0.6], cx + 2.0, iy + 2.0, cw - 4.0, 20.0);
            }

            // Texts
            for (idx, opt) in cce_ui::widget::context_menu::options().iter().enumerate() {
                let iy = cy + idx as f32 * 24.0 + (24.0 - 12.0) / 2.0;
                let text_color = if idx == 0 {
                    [0.44, 0.44, 0.47, 1.0]
                } else if cce_ui::widget::context_menu::hovered_item() == Some(idx) {
                    [1.0, 1.0, 1.0, 1.0]
                } else {
                    [0.80, 0.80, 0.83, 1.0]
                };
                pc.text_with_bounds(opt, cx + 8.0, iy, 12.0, text_color, Some([cx, cy, cx + cw, cy + ch]));
            }
        }
    }

    fn clear_color(&self) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn handle_pointer_move(&mut self, pos: cce_ui::engine::LogicalPosition, needs_rebuild: &mut bool) {
        if self.handle_cursor_moved(pos.x, pos.y) {
            *needs_rebuild = true;
        }
    }

    fn handle_mouse_input(&mut self, button: cce_ui::widget::MouseButton, state: cce_ui::widget::ElementState, pos: cce_ui::engine::LogicalPosition, needs_rebuild: &mut bool) -> Option<Self::Message> {
        self.cursor_x = pos.x;
        self.cursor_y = pos.y;
        if self.handle_mouse_input_internal(button, state) {
            *needs_rebuild = true;
        }
        None
    }

    fn handle_mouse_wheel(&mut self, delta: &cce_ui::widget::MouseScrollDelta, pos: cce_ui::engine::LogicalPosition, needs_rebuild: &mut bool) {
        if self.handle_mouse_wheel_internal(delta, pos.x, pos.y) {
            *needs_rebuild = true;
        }
    }

    fn handle_key_input(&mut self, event: &cce_ui::widget::KeyEvent, needs_rebuild: &mut bool) -> Option<Self::Message> {
        if self.handle_key_input_internal(event) {
            *needs_rebuild = true;
        }
        None
    }
}

impl SystemInterface {

fn collect_popover_rects(w: &dyn cce_ui::widget::Element, popovers: &mut Vec<(f32, f32, f32, f32)>, ctx: &cce_ui::context::UiContext) {
    if let Some(rect) = w.popover_rect() {
        popovers.push(rect);
    }
    for child_ptr in w.children(ctx) {
        unsafe {
            if let Some(child) = child_ptr.as_ref() {
                Self::collect_popover_rects(child, popovers, ctx);
            }
        }
    }
}

    fn rebuild_layout(&mut self, sw: f32, sh: f32) {
        self.switcher.clear_children(&mut self.ui_context);
        for plate in &mut self.plates {
            self.switcher.add_child(plate.as_ptr(), &mut self.ui_context);
        }

        let page_idx = Page::ALL.iter().position(|&p| p == self.app.current_page).unwrap_or(0);

        // ── Rebuild Element Focus Hierarchy ──
        for plate in &mut self.plates {
            plate.clear_children(&mut self.ui_context);
        }
        let page_root = &mut self.plates[page_idx];
        self.page_sec_containers.clear();

        // Clear all widgets' hierarchy links
        self.app.services.search_box.clear_children(&mut self.ui_context); self.app.services.search_box.set_parent(None, &mut self.ui_context);
        self.app.packages.search_box.clear_children(&mut self.ui_context); self.app.packages.search_box.set_parent(None, &mut self.ui_context);
        self.app.packages.installed_list_box.scroll_box.clear_children(&mut self.ui_context); self.app.packages.installed_list_box.scroll_box.set_parent(None, &mut self.ui_context);
        self.app.packages.updates_list_box.scroll_box.clear_children(&mut self.ui_context); self.app.packages.updates_list_box.scroll_box.set_parent(None, &mut self.ui_context);
        self.app.accounts.email_box.clear_children(&mut self.ui_context); self.app.accounts.email_box.set_parent(None, &mut self.ui_context);
        self.app.accounts.password_box.clear_children(&mut self.ui_context); self.app.accounts.password_box.set_parent(None, &mut self.ui_context);
        self.app.accounts.imap_box.clear_children(&mut self.ui_context); self.app.accounts.imap_box.set_parent(None, &mut self.ui_context);
        self.app.accounts.smtp_box.clear_children(&mut self.ui_context); self.app.accounts.smtp_box.set_parent(None, &mut self.ui_context);
        self.app.accounts.oauth_client_id_box.clear_children(&mut self.ui_context); self.app.accounts.oauth_client_id_box.set_parent(None, &mut self.ui_context);
        self.app.accounts.oauth_client_secret_box.clear_children(&mut self.ui_context); self.app.accounts.oauth_client_secret_box.set_parent(None, &mut self.ui_context);

        self.app.services.list_box.scroll_box.clear_children(&mut self.ui_context); self.app.services.list_box.scroll_box.set_parent(None, &mut self.ui_context);

        self.app.hardware.cpu_list_box.scroll_box.clear_children(&mut self.ui_context); self.app.hardware.cpu_list_box.scroll_box.set_parent(None, &mut self.ui_context);

        self.app.network.wifi_list_box.scroll_box.clear_children(&mut self.ui_context); self.app.network.wifi_list_box.scroll_box.set_parent(None, &mut self.ui_context);

        for sb in &mut self.app.layout.spinboxes {
            sb.clear_children(&mut self.ui_context);
            sb.set_parent(None, &mut self.ui_context);
        }
        self.app.layout.cascade_offset_spinbox.clear_children(&mut self.ui_context); self.app.layout.cascade_offset_spinbox.set_parent(None, &mut self.ui_context);
        self.app.layout.edge_gap_spinbox.clear_children(&mut self.ui_context); self.app.layout.edge_gap_spinbox.set_parent(None, &mut self.ui_context);
        self.app.layout.top_gap_spinbox.clear_children(&mut self.ui_context); self.app.layout.top_gap_spinbox.set_parent(None, &mut self.ui_context);
        self.app.layout.grid_gap_spinbox.clear_children(&mut self.ui_context); self.app.layout.grid_gap_spinbox.set_parent(None, &mut self.ui_context);
        self.app.layout.transition_duration_spinbox.clear_children(&mut self.ui_context); self.app.layout.transition_duration_spinbox.set_parent(None, &mut self.ui_context);
        self.app.layout.status_height_spinbox.clear_children(&mut self.ui_context); self.app.layout.status_height_spinbox.set_parent(None, &mut self.ui_context);

        for cs in &mut self.app.interface.color_selectors {
            cs.clear_children(&mut self.ui_context);
            cs.set_parent(None, &mut self.ui_context);
        }
        self.app.interface.tab_margin_spinbox_x.clear_children(&mut self.ui_context);
        self.app.interface.tab_margin_spinbox_x.set_parent(None, &mut self.ui_context);
        self.app.interface.menubar_opacity_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.menubar_opacity_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.tab_margin_spinbox_y.clear_children(&mut self.ui_context);
        self.app.interface.tab_margin_spinbox_y.set_parent(None, &mut self.ui_context);
        self.app.interface.button_padding_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.button_padding_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.section_padding_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.section_padding_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.label_alignment_menu.clear_children(&mut self.ui_context);
        self.app.interface.label_alignment_menu.set_parent(None, &mut self.ui_context);
        self.app.interface.label_offset_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.label_offset_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.label_margin_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.label_margin_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.plate_padding_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.plate_padding_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.graph_show_grid_toggle.clear_children(&mut self.ui_context);
        self.app.interface.graph_show_grid_toggle.set_parent(None, &mut self.ui_context);
        self.app.interface.graph_snap_enabled_toggle.clear_children(&mut self.ui_context);
        self.app.interface.graph_snap_enabled_toggle.set_parent(None, &mut self.ui_context);
        self.app.interface.graph_uniform_background_toggle.clear_children(&mut self.ui_context);
        self.app.interface.graph_uniform_background_toggle.set_parent(None, &mut self.ui_context);
        self.app.interface.graph_cell_opacity_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.graph_cell_opacity_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.graph_gap_opacity_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.graph_gap_opacity_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.graph_gap_width_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.graph_gap_width_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.page_margin_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.page_margin_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.grid_min_col_width_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.grid_min_col_width_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.spinbox_height_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.spinbox_height_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.spinbox_corner_radius_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.spinbox_corner_radius_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.toggle_height_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.toggle_height_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.toggle_corner_radius_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.toggle_corner_radius_spinbox.set_parent(None, &mut self.ui_context);

        self.app.interface.color_selector_height_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.color_selector_height_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.color_selector_corner_radius_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.color_selector_corner_radius_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.plate_opacity_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.plate_opacity_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.plate_corner_radius_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.plate_corner_radius_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.page_opacity_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.page_opacity_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.layer_opacity_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.layer_opacity_spinbox.set_parent(None, &mut self.ui_context);

        self.app.interface.color_selector_preview_corner_radius_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.color_selector_preview_corner_radius_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.color_selector_preview_margin_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.color_selector_preview_margin_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.textbox_height_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.textbox_height_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.textbox_corner_radius_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.textbox_corner_radius_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.slider_height_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.slider_height_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.color_selector_font_selector.clear_children(&mut self.ui_context);
        self.app.interface.color_selector_font_selector.set_parent(None, &mut self.ui_context);
        self.app.interface.menubar_font_selector.clear_children(&mut self.ui_context);
        self.app.interface.menubar_font_selector.set_parent(None, &mut self.ui_context);
        self.app.interface.section_label_font_selector.clear_children(&mut self.ui_context);
        self.app.interface.section_label_font_selector.set_parent(None, &mut self.ui_context);
        self.app.interface.nested_section_label_font_selector.clear_children(&mut self.ui_context);
        self.app.interface.nested_section_label_font_selector.set_parent(None, &mut self.ui_context);
        self.app.interface.font_selector_height_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.font_selector_height_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.font_selector_corner_radius_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.font_selector_corner_radius_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.dropdown_height_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.dropdown_height_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.dropdown_corner_radius_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.dropdown_corner_radius_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.button_corner_radius_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.button_corner_radius_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.notification_opacity_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.notification_opacity_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.window_opacity_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.window_opacity_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.window_corner_radius_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.window_corner_radius_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.custom_multicontrol.clear_children(&mut self.ui_context);
        self.app.interface.custom_multicontrol.set_parent(None, &mut self.ui_context);

        self.app.interface.sans_box.clear_children(&mut self.ui_context); self.app.interface.sans_box.set_parent(None, &mut self.ui_context);
        self.app.interface.serif_box.clear_children(&mut self.ui_context); self.app.interface.serif_box.set_parent(None, &mut self.ui_context);
        self.app.interface.mono_box.clear_children(&mut self.ui_context); self.app.interface.mono_box.set_parent(None, &mut self.ui_context);
        self.app.interface.borders_menu.clear_children(&mut self.ui_context); self.app.interface.borders_menu.set_parent(None, &mut self.ui_context);
        self.app.interface.borders_box.clear_children(&mut self.ui_context); self.app.interface.borders_box.set_parent(None, &mut self.ui_context);
        self.app.interface.status_menu.clear_children(&mut self.ui_context); self.app.interface.status_menu.set_parent(None, &mut self.ui_context);
        self.app.interface.status_box.clear_children(&mut self.ui_context); self.app.interface.status_box.set_parent(None, &mut self.ui_context);
        self.app.interface.fuzzel_menu.clear_children(&mut self.ui_context); self.app.interface.fuzzel_menu.set_parent(None, &mut self.ui_context);
        self.app.interface.fuzzel_box.clear_children(&mut self.ui_context); self.app.interface.fuzzel_box.set_parent(None, &mut self.ui_context);
        self.app.interface.terminal_menu.clear_children(&mut self.ui_context); self.app.interface.terminal_menu.set_parent(None, &mut self.ui_context);
        self.app.interface.terminal_box.clear_children(&mut self.ui_context); self.app.interface.terminal_box.set_parent(None, &mut self.ui_context);

        self.app.services.notifications_enable_toggle.clear_children(&mut self.ui_context); self.app.services.notifications_enable_toggle.set_parent(None, &mut self.ui_context);
        self.app.services.notifications_bell_toggle.clear_children(&mut self.ui_context); self.app.services.notifications_bell_toggle.set_parent(None, &mut self.ui_context);
        self.app.services.notifications_duration_spinbox.clear_children(&mut self.ui_context); self.app.services.notifications_duration_spinbox.set_parent(None, &mut self.ui_context);

        self.app.input.rate_spinbox.clear_children(&mut self.ui_context); self.app.input.rate_spinbox.set_parent(None, &mut self.ui_context);
        self.app.input.delay_spinbox.clear_children(&mut self.ui_context); self.app.input.delay_spinbox.set_parent(None, &mut self.ui_context);
        self.app.input.scroll_toggle.clear_children(&mut self.ui_context); self.app.input.scroll_toggle.set_parent(None, &mut self.ui_context);
        self.app.input.scroll_friction_spinbox.clear_children(&mut self.ui_context); self.app.input.scroll_friction_spinbox.set_parent(None, &mut self.ui_context);
        self.app.input.natural_toggle.clear_children(&mut self.ui_context); self.app.input.natural_toggle.set_parent(None, &mut self.ui_context);
        self.app.input.scroll_speed_spinbox.clear_children(&mut self.ui_context); self.app.input.scroll_speed_spinbox.set_parent(None, &mut self.ui_context);
        self.app.input.pointer_toggle.clear_children(&mut self.ui_context); self.app.input.pointer_toggle.set_parent(None, &mut self.ui_context);
        self.app.input.pointer_friction_spinbox.clear_children(&mut self.ui_context); self.app.input.pointer_friction_spinbox.set_parent(None, &mut self.ui_context);
        self.app.input.trackpad_toggle.clear_children(&mut self.ui_context); self.app.input.trackpad_toggle.set_parent(None, &mut self.ui_context);
        self.app.input.trackpad_friction_spinbox.clear_children(&mut self.ui_context); self.app.input.trackpad_friction_spinbox.set_parent(None, &mut self.ui_context);
        self.app.input.dwtp_toggle.clear_children(&mut self.ui_context); self.app.input.dwtp_toggle.set_parent(None, &mut self.ui_context);
        self.app.input.trackpoint_accel_speed_spinbox.clear_children(&mut self.ui_context); self.app.input.trackpoint_accel_speed_spinbox.set_parent(None, &mut self.ui_context);
        self.app.input.trackpoint_accel_profile_menu.clear_children(&mut self.ui_context); self.app.input.trackpoint_accel_profile_menu.set_parent(None, &mut self.ui_context);
        self.app.input.cursor_theme_menu.clear_children(&mut self.ui_context); self.app.input.cursor_theme_menu.set_parent(None, &mut self.ui_context);
        self.app.input.cursor_size_spinbox.clear_children(&mut self.ui_context); self.app.input.cursor_size_spinbox.set_parent(None, &mut self.ui_context);
        self.app.input.zoom_in_box.clear_children(&mut self.ui_context); self.app.input.zoom_in_box.set_parent(None, &mut self.ui_context);
        self.app.input.zoom_out_box.clear_children(&mut self.ui_context); self.app.input.zoom_out_box.set_parent(None, &mut self.ui_context);

        for sb in &mut self.app.audio.sink_spinboxes {
            sb.clear_children(&mut self.ui_context);
            sb.set_parent(None, &mut self.ui_context);
        }
        for sb in &mut self.app.audio.source_spinboxes {
            sb.clear_children(&mut self.ui_context);
            sb.set_parent(None, &mut self.ui_context);
        }
        for slider in &mut self.app.audio.sink_sliders {
            slider.clear_children(&mut self.ui_context);
            slider.set_parent(None, &mut self.ui_context);
        }
        for slider in &mut self.app.audio.source_sliders {
            slider.clear_children(&mut self.ui_context);
            slider.set_parent(None, &mut self.ui_context);
        }

        self.app.display.brightness_spinbox.clear_children(&mut self.ui_context); self.app.display.brightness_spinbox.set_parent(None, &mut self.ui_context);
        self.app.display.brightness_slider.clear_children(&mut self.ui_context); self.app.display.brightness_slider.set_parent(None, &mut self.ui_context);
        self.app.display.night_light_label.clear_children(&mut self.ui_context); self.app.display.night_light_label.set_parent(None, &mut self.ui_context);
        self.app.display.screensaver_enable_toggle.clear_children(&mut self.ui_context); self.app.display.screensaver_enable_toggle.set_parent(None, &mut self.ui_context);
        self.app.display.screensaver_lock_screen_toggle.clear_children(&mut self.ui_context); self.app.display.screensaver_lock_screen_toggle.set_parent(None, &mut self.ui_context);
        self.app.display.screensaver_timeout_spinbox.clear_children(&mut self.ui_context); self.app.display.screensaver_timeout_spinbox.set_parent(None, &mut self.ui_context);
        self.app.display.screensaver_style_menu.clear_children(&mut self.ui_context); self.app.display.screensaver_style_menu.set_parent(None, &mut self.ui_context);
        for out in &mut self.app.display.outputs {
            out.name_label.clear_children(&mut self.ui_context); out.name_label.set_parent(None, &mut self.ui_context);
            out.resolution_label.clear_children(&mut self.ui_context); out.resolution_label.set_parent(None, &mut self.ui_context);
            if let Some(ref mut scale_lbl) = out.scale_label {
                scale_lbl.clear_children(&mut self.ui_context); scale_lbl.set_parent(None, &mut self.ui_context);
            }
        }
        self.app.services.status_separators_toggle.clear_children(&mut self.ui_context); self.app.services.status_separators_toggle.set_parent(None, &mut self.ui_context);
        self.app.services.status_underline_toggle.clear_children(&mut self.ui_context); self.app.services.status_underline_toggle.set_parent(None, &mut self.ui_context);
        self.app.services.status_padding_spinbox.clear_children(&mut self.ui_context); self.app.services.status_padding_spinbox.set_parent(None, &mut self.ui_context);

        for menu in &mut self.app.layout.tag_layout_menus {
            menu.clear_children(&mut self.ui_context);
            menu.set_parent(None, &mut self.ui_context);
        }
        self.app.layout.side_panel_behavior_menu.clear_children(&mut self.ui_context);
        self.app.layout.side_panel_behavior_menu.set_parent(None, &mut self.ui_context);
        self.app.layout.side_panel_position_menu.clear_children(&mut self.ui_context);
        self.app.layout.side_panel_position_menu.set_parent(None, &mut self.ui_context);
        self.app.layout.side_panel_width_spinbox.clear_children(&mut self.ui_context);
        self.app.layout.side_panel_width_spinbox.set_parent(None, &mut self.ui_context);
        self.app.layout.side_panel_border_gap_spinbox.clear_children(&mut self.ui_context);
        self.app.layout.side_panel_border_gap_spinbox.set_parent(None, &mut self.ui_context);
        self.app.layout.side_panel_border_opacity_spinbox.clear_children(&mut self.ui_context);
        self.app.layout.side_panel_border_opacity_spinbox.set_parent(None, &mut self.ui_context);

        use cce_ui::widget::focus::link_parent_child;
        match self.app.current_page {
            Page::Accounts => {
                self.page_sec_containers.resize_with(2, cce_ui::widget::Container::new);
                for i in 0..2 {
                    link_parent_child(page_root, &mut self.page_sec_containers[i], &mut self.ui_context);
                }
                if self.app.accounts.editing_oauth_creds {
                    link_parent_child(&mut self.page_sec_containers[1], &mut self.app.accounts.oauth_client_id_box, &mut self.ui_context);
                    link_parent_child(&mut self.page_sec_containers[1], &mut self.app.accounts.oauth_client_secret_box, &mut self.ui_context);
                } else if self.app.accounts.adding_new {
                    link_parent_child(&mut self.page_sec_containers[1], &mut self.app.accounts.email_box, &mut self.ui_context);
                    link_parent_child(&mut self.page_sec_containers[1], &mut self.app.accounts.password_box, &mut self.ui_context);
                    link_parent_child(&mut self.page_sec_containers[1], &mut self.app.accounts.imap_box, &mut self.ui_context);
                    link_parent_child(&mut self.page_sec_containers[1], &mut self.app.accounts.smtp_box, &mut self.ui_context);
                }
            }

            Page::Services => {
                self.page_sec_containers.resize_with(3, cce_ui::widget::Container::new);
                for i in 0..3 {
                    link_parent_child(page_root, &mut self.page_sec_containers[i], &mut self.ui_context);
                }
                link_parent_child(&mut self.page_sec_containers[0], &mut self.app.services.search_box, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[0], &mut self.app.services.list_box.scroll_box, &mut self.ui_context);

                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.services.notifications_enable_toggle, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.services.notifications_bell_toggle, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.services.notifications_duration_spinbox, &mut self.ui_context);

                link_parent_child(&mut self.page_sec_containers[2], &mut self.app.services.status_separators_toggle, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[2], &mut self.app.services.status_underline_toggle, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[2], &mut self.app.services.status_padding_spinbox, &mut self.ui_context);
            }
            Page::Hardware => {
                self.page_sec_containers.resize_with(5, cce_ui::widget::Container::new);
                for i in 0..5 {
                    link_parent_child(page_root, &mut self.page_sec_containers[i], &mut self.ui_context);
                }
                link_parent_child(&mut self.page_sec_containers[0], &mut self.app.hardware.cpu_list_box.scroll_box, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.hardware.cpu_gov_menu, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[4], &mut self.app.hardware.gpu_gov_menu, &mut self.ui_context);
            }
            Page::Radios => {
                self.page_sec_containers.resize_with(2, cce_ui::widget::Container::new);
                for i in 0..2 {
                    link_parent_child(page_root, &mut self.page_sec_containers[i], &mut self.ui_context);
                }
                link_parent_child(&mut self.page_sec_containers[0], &mut self.app.network.wifi_list_box.scroll_box, &mut self.ui_context);
            }
            Page::Layout => {
                self.page_sec_containers.resize_with(7, cce_ui::widget::Container::new);
                
                for i in 0..7 {
                    link_parent_child(page_root, &mut self.page_sec_containers[i], &mut self.ui_context);
                }
                
                // Fullscreen (Section 0)
                link_parent_child(&mut self.page_sec_containers[0], &mut self.app.layout.spinboxes[0], &mut self.ui_context);
                
                // Cascade (Section 1)
                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.layout.spinboxes[1], &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.layout.cascade_offset_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.layout.edge_gap_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.layout.top_gap_spinbox, &mut self.ui_context);
                
                // Grid (Section 2)
                link_parent_child(&mut self.page_sec_containers[2], &mut self.app.layout.grid_gap_spinbox, &mut self.ui_context);
                
                // Floating (Section 3)
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.layout.spinboxes[3], &mut self.ui_context);
                
                // Movement (Section 4)
                link_parent_child(&mut self.page_sec_containers[4], &mut self.app.layout.transition_duration_spinbox, &mut self.ui_context);
                
                // Default Layouts (Section 5)
                for menu in &mut self.app.layout.tag_layout_menus {
                    link_parent_child(&mut self.page_sec_containers[5], menu, &mut self.ui_context);
                }

                // Side Panel (Section 6)
                link_parent_child(&mut self.page_sec_containers[6], &mut self.app.layout.side_panel_behavior_menu, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[6], &mut self.app.layout.side_panel_position_menu, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[6], &mut self.app.layout.side_panel_width_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[6], &mut self.app.layout.side_panel_border_gap_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[6], &mut self.app.layout.side_panel_border_opacity_spinbox, &mut self.ui_context);
            }
            Page::Interface => {
                self.page_sec_containers.resize_with(9, cce_ui::widget::Container::new);
                
                for i in 0..9 {
                    link_parent_child(page_root, &mut self.page_sec_containers[i], &mut self.ui_context);
                }
                
                // Section 0: Custom Parameters
                link_parent_child(&mut self.page_sec_containers[0], &mut self.app.interface.custom_multicontrol, &mut self.ui_context);
                
                // Section 1: Layout (parent of Grid Layout)
                // (Layout widgets)
                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.interface.color_selectors[7], &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.interface.color_selectors[1], &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.interface.color_selectors[2], &mut self.ui_context);
                // (Grid Layout child widgets)
                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.interface.grid_min_col_width_spinbox, &mut self.ui_context);

                
                // Section 2: Status
                link_parent_child(&mut self.page_sec_containers[2], &mut self.app.interface.color_selectors[8], &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[2], &mut self.app.interface.color_selectors[3], &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[2], &mut self.app.interface.color_selectors[4], &mut self.ui_context);
                
                // Section 3: Controls (parent of: Slider, MenuBar, Toggles, Spinbox, ColorSelector, Textbox, FontSelector)
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.color_selectors[6], &mut self.ui_context);
                // (Slider child widgets)
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.color_selectors[5], &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.slider_height_spinbox, &mut self.ui_context);
                
                // (MenuBar child widgets)
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.color_selectors[9], &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.color_selectors[11], &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.tab_margin_spinbox_x, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.tab_margin_spinbox_y, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.button_padding_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.menubar_opacity_spinbox, &mut self.ui_context);
                 // (Toggles child widgets)
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.color_selectors[12], &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.color_selectors[13], &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.toggle_height_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.toggle_corner_radius_spinbox, &mut self.ui_context);
                
                // (Breadcrumb child widgets)
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.color_selectors[15], &mut self.ui_context);
 
                // (Spinbox child widgets)
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.spinbox_height_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.spinbox_corner_radius_spinbox, &mut self.ui_context);
                
                // (ColorSelector child widgets)
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.color_selector_height_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.color_selector_preview_corner_radius_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.color_selector_preview_margin_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.color_selector_corner_radius_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.color_selector_font_selector, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.menubar_font_selector, &mut self.ui_context);
                
                // (Textbox child widgets)
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.textbox_height_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.textbox_corner_radius_spinbox, &mut self.ui_context);
                
                // (FontSelector child widgets)
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.font_selector_height_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.font_selector_corner_radius_spinbox, &mut self.ui_context);
                
                // (Dropdown child widgets)
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.dropdown_height_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.dropdown_corner_radius_spinbox, &mut self.ui_context);
                
                // (Button child widgets)
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.button_corner_radius_spinbox, &mut self.ui_context);
                
                // (Labels child widgets)
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.label_margin_spinbox, &mut self.ui_context);
                
                // Section 4: Indicators (parent of Primary Highlight)
                link_parent_child(&mut self.page_sec_containers[4], &mut self.app.interface.color_selectors[10], &mut self.ui_context);
                
                // Section 5: Notification
                link_parent_child(&mut self.page_sec_containers[5], &mut self.app.interface.color_selectors[17], &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[5], &mut self.app.interface.notification_opacity_spinbox, &mut self.ui_context);
                
                // Section 6: Surfaces
                link_parent_child(&mut self.page_sec_containers[6], &mut self.app.interface.color_selectors[18], &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[6], &mut self.app.interface.window_opacity_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[6], &mut self.app.interface.window_corner_radius_spinbox, &mut self.ui_context);
                // (Plate child widgets)
                link_parent_child(&mut self.page_sec_containers[6], &mut self.app.interface.color_selectors[0], &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[6], &mut self.app.interface.plate_padding_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[6], &mut self.app.interface.plate_opacity_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[6], &mut self.app.interface.plate_corner_radius_spinbox, &mut self.ui_context);
                // (Popover child widgets)
                link_parent_child(&mut self.page_sec_containers[6], &mut self.app.interface.color_selectors[16], &mut self.ui_context);
                
                // Section 7: Fonts (parent of System Fonts and Program Fonts)
                // (System Fonts)
                link_parent_child(&mut self.page_sec_containers[7], &mut self.app.interface.sans_box, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[7], &mut self.app.interface.serif_box, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[7], &mut self.app.interface.mono_box, &mut self.ui_context);
                
                // (Program Fonts)
                link_parent_child(&mut self.page_sec_containers[7], &mut self.app.interface.borders_menu, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[7], &mut self.app.interface.borders_box, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[7], &mut self.app.interface.status_menu, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[7], &mut self.app.interface.status_box, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[7], &mut self.app.interface.fuzzel_menu, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[7], &mut self.app.interface.fuzzel_box, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[7], &mut self.app.interface.terminal_menu, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[7], &mut self.app.interface.terminal_box, &mut self.ui_context);

                // Section 8: Containers (parent of ScrollingList, Sections, Graph)
                // (Sections child widgets)
                link_parent_child(&mut self.page_sec_containers[8], &mut self.app.interface.section_padding_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[8], &mut self.app.interface.page_margin_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[8], &mut self.app.interface.section_label_font_selector, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[8], &mut self.app.interface.label_alignment_menu, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[8], &mut self.app.interface.label_offset_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[8], &mut self.app.interface.nested_section_label_font_selector, &mut self.ui_context);
                // (ScrollingList child widgets)
                link_parent_child(&mut self.page_sec_containers[8], &mut self.app.interface.color_selectors[14], &mut self.ui_context);
                // (Page child widgets)
                link_parent_child(&mut self.page_sec_containers[8], &mut self.app.interface.color_selectors[19], &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[8], &mut self.app.interface.page_opacity_spinbox, &mut self.ui_context);
                // (Layer child widgets)
                link_parent_child(&mut self.page_sec_containers[8], &mut self.app.interface.color_selectors[20], &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[8], &mut self.app.interface.layer_opacity_spinbox, &mut self.ui_context);
                // (Graph child widgets)
                link_parent_child(&mut self.page_sec_containers[8], &mut self.app.interface.graph_show_grid_toggle, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[8], &mut self.app.interface.graph_snap_enabled_toggle, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[8], &mut self.app.interface.graph_uniform_background_toggle, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[8], &mut self.app.interface.graph_cell_opacity_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[8], &mut self.app.interface.graph_gap_opacity_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[8], &mut self.app.interface.graph_gap_width_spinbox, &mut self.ui_context);
            }


            Page::Input => {
                self.page_sec_containers.resize_with(6, cce_ui::widget::Container::new);
                link_parent_child(page_root, &mut self.page_sec_containers[0], &mut self.ui_context);
                link_parent_child(page_root, &mut self.page_sec_containers[1], &mut self.ui_context);
                link_parent_child(page_root, &mut self.page_sec_containers[2], &mut self.ui_context);
                link_parent_child(page_root, &mut self.page_sec_containers[3], &mut self.ui_context);
                link_parent_child(page_root, &mut self.page_sec_containers[4], &mut self.ui_context);
                link_parent_child(page_root, &mut self.page_sec_containers[5], &mut self.ui_context);
                
                link_parent_child(&mut self.page_sec_containers[0], &mut self.app.input.dwtp_toggle, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[0], &mut self.app.input.trackpoint_accel_speed_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[0], &mut self.app.input.trackpoint_accel_profile_menu, &mut self.ui_context);
                
                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.input.rate_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.input.delay_spinbox, &mut self.ui_context);

                link_parent_child(&mut self.page_sec_containers[2], &mut self.app.input.cursor_theme_menu, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[2], &mut self.app.input.cursor_size_spinbox, &mut self.ui_context);
                
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.input.scroll_toggle, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.input.scroll_friction_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.input.natural_toggle, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.input.scroll_speed_spinbox, &mut self.ui_context);
                
                link_parent_child(&mut self.page_sec_containers[4], &mut self.app.input.pointer_toggle, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[4], &mut self.app.input.pointer_friction_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[4], &mut self.app.input.trackpad_toggle, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[4], &mut self.app.input.trackpad_friction_spinbox, &mut self.ui_context);

                link_parent_child(&mut self.page_sec_containers[5], &mut self.app.input.zoom_in_box, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[5], &mut self.app.input.zoom_out_box, &mut self.ui_context);
            }
            Page::Audio => {
                self.page_sec_containers.resize_with(2, cce_ui::widget::Container::new);
                link_parent_child(page_root, &mut self.page_sec_containers[0], &mut self.ui_context);
                link_parent_child(page_root, &mut self.page_sec_containers[1], &mut self.ui_context);
                
                for sb in &mut self.app.audio.sink_spinboxes {
                    link_parent_child(&mut self.page_sec_containers[0], &mut **sb, &mut self.ui_context);
                }
                for sb in &mut self.app.audio.source_spinboxes {
                    link_parent_child(&mut self.page_sec_containers[1], &mut **sb, &mut self.ui_context);
                }
                for slider in &mut self.app.audio.sink_sliders {
                    link_parent_child(&mut self.page_sec_containers[0], &mut **slider, &mut self.ui_context);
                }
                for slider in &mut self.app.audio.source_sliders {
                    link_parent_child(&mut self.page_sec_containers[1], &mut **slider, &mut self.ui_context);
                }
            }
            Page::Display => {
                self.page_sec_containers.resize_with(4, cce_ui::widget::Container::new);
                link_parent_child(page_root, &mut self.page_sec_containers[0], &mut self.ui_context);
                link_parent_child(page_root, &mut self.page_sec_containers[1], &mut self.ui_context);
                link_parent_child(page_root, &mut self.page_sec_containers[2], &mut self.ui_context);
                link_parent_child(page_root, &mut self.page_sec_containers[3], &mut self.ui_context);

                // Section 0: Brightness
                link_parent_child(&mut self.page_sec_containers[0], &mut self.app.display.brightness_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[0], &mut self.app.display.brightness_slider, &mut self.ui_context);

                // Section 1: Night Light
                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.display.night_light_label, &mut self.ui_context);

                // Section 2: Outputs
                for out in &mut self.app.display.outputs {
                    link_parent_child(&mut self.page_sec_containers[2], &mut out.name_label, &mut self.ui_context);
                    link_parent_child(&mut self.page_sec_containers[2], &mut out.resolution_label, &mut self.ui_context);
                    if let Some(ref mut scale_lbl) = out.scale_label {
                        link_parent_child(&mut self.page_sec_containers[2], scale_lbl, &mut self.ui_context);
                    }
                }

                // Section 3: Screensaver Settings
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.display.screensaver_enable_toggle, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.display.screensaver_lock_screen_toggle, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.display.screensaver_timeout_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.display.screensaver_style_menu, &mut self.ui_context);
            }
            Page::Packages => {
                self.page_sec_containers.resize_with(2, cce_ui::widget::Container::new);
                for i in 0..2 {
                    link_parent_child(page_root, &mut self.page_sec_containers[i], &mut self.ui_context);
                }
                link_parent_child(&mut self.page_sec_containers[0], &mut self.app.packages.search_box, &mut self.ui_context);
                match self.app.packages.active_tab {
                    pages::packages::PackageTab::Installed => {
                        link_parent_child(&mut self.page_sec_containers[0], &mut self.app.packages.installed_list_box.scroll_box, &mut self.ui_context);
                    }
                    pages::packages::PackageTab::Updates => {
                        link_parent_child(&mut self.page_sec_containers[0], &mut self.app.packages.updates_list_box.scroll_box, &mut self.ui_context);
                    }
                }
            }
            _ => {}
        }

        self.sidebar_width = self.menubar.sidebar_w();
        self.header_height = 0.0; // No CSD Titlebar
        let s = 1.0f32;
        let mut widgets = Vec::new();
        let mut text_items = Vec::new();
        let mut page_buttons = Vec::new();

        cce_ui::widget::hover_animation::reset_frame_registration();
        cce_ui::widget::popovers::clear();
        cce_ui::widget::hover_animation::set_scroll_offset(self.scroll_y);
        cce_ui::widget::hover_animation::set_cursor_pos(self.cursor_x / s, self.cursor_y / s);

        let lcx = self.sidebar_width;
        let lcy = self.header_height;
        let lcw = sw / s - self.sidebar_width;
        let lch = sh / s - self.header_height - self.status_height;

        let page_idx = Page::ALL.iter().position(|&p| p == self.app.current_page).unwrap_or(0);
        self.menubar.set_selected_page(page_idx);
        self.switcher.set_active_index(Some(page_idx));

        // Update root window size, background color, opacity, corner radius, and children
        self.root_window.set_rect(0.0, 0.0, sw / s, sh / s);
        let win_r = self.app.interface.window_color[0] as f32 / 255.0;
        let win_g = self.app.interface.window_color[1] as f32 / 255.0;
        let win_b = self.app.interface.window_color[2] as f32 / 255.0;
        let win_a = self.app.interface.window_opacity;
        self.root_window.background_color = Some(cce_ui::color::to_linear([win_r, win_g, win_b, win_a]));
        self.root_window.radius = self.app.interface.window_corner_radius as f32;
        self.root_window.clear_children(&mut self.ui_context);
        self.root_window.add_child(self.menubar.as_ptr(), &mut self.ui_context);
        self.root_window.add_child(self.switcher.as_ptr(), &mut self.ui_context);

        // Position sidebar and switcher below the titlebar
        let mut dummy_pc = PageContent::new();
        cce_ui::layout::render_widget(&mut dummy_pc, &mut self.menubar, 0.0, self.header_height, self.sidebar_width, sh / s - self.header_height, &mut self.ui_context);
        cce_ui::layout::render_widget(&mut dummy_pc, &mut self.switcher, self.sidebar_width, self.header_height, sw / s - self.sidebar_width, sh / s - self.header_height, &mut self.ui_context);

        // Render root window recursively
        let mut window_pc = PageContent::new();
        cce_ui::layout::render_widget(&mut window_pc, &mut self.root_window, 0.0, 0.0, sw / s, sh / s, &mut self.ui_context);

        // CSD Titlebar removed

        for pc_part in &[window_pc] {
            for (c, x, y, w, h, r, corners) in &pc_part.rects {
                widgets.push(AppWidget {
                    x: *x * s, y: *y * s, w: *w * s, h: *h * s,
                    color: *c, hover_color: *c,
                    hovering: false,
                    radius: *r * s,
                    corners: *corners,
                });
            }
            for (t, size, x, y, tc, font_opt, bounds) in &pc_part.texts {
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
                    bounds: *bounds,
                });
            }
        }

        // Page content in LOGICAL coordinates, then scale to physical
        let pc = self.render_page_content(lcx, lcy, lcw, lch);

        let mut popovers = Vec::new();
        Self::collect_popover_rects(&self.plates[page_idx], &mut popovers, &self.ui_context);

        let mut max_y = 0.0f32;
        for (_, _, y, _, h, _, _) in &pc.rects {
            max_y = max_y.max(y + h);
        }
        for (_, size, _, y, _, _, _) in &pc.texts {
            max_y = max_y.max(y + size);
        }
        for (btn, _) in &pc.buttons {
            let base = btn.base().unwrap();
            max_y = max_y.max(base.y + base.h);
        }
        self.max_scroll_y = (max_y - lch).max(0.0);
        eprintln!("SCROLL_DEBUG: max_y={}, lch={}, max_scroll_y={}, scroll_y={}", max_y, lch, self.max_scroll_y, self.scroll_y);
        static mut FRAME_COUNT: usize = 0;
        unsafe {
            FRAME_COUNT += 1;
            if FRAME_COUNT > 5 {
                self.scroll_y = self.scroll_y.min(self.max_scroll_y);
            }
        }

        let scroll_offset_y = self.scroll_y;

        for (c, x, y, w, h, r, corners) in &pc.rects {
            widgets.push(AppWidget {
                x: *x * s, y: (*y - scroll_offset_y) * s, w: *w * s, h: *h * s,
                color: *c, hover_color: *c,
                hovering: false,
                radius: *r * s,
                corners: *corners,
            });
        }
        for (t, size, x, y, tc, font_opt, bounds) in &pc.texts {
            let shifted_bounds = bounds.map(|[bl, bt, br, bb]| {
                [bl, bt - scroll_offset_y, br, bb - scroll_offset_y]
            });
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
                bounds: shifted_bounds,
            });
        }
        for (btn, action) in &pc.buttons {
            let base = btn.base().unwrap();
            let bg = btn.bg.unwrap_or([0.16, 0.16, 0.24, 1.0]);
            let hover_bg = btn.hover_bg.unwrap_or([0.25, 0.30, 0.26, 1.0]);
            widgets.push(AppWidget {
                x: base.x * s, y: (base.y - scroll_offset_y) * s, w: base.w * s, h: base.h * s,
                color: bg, hover_color: hover_bg,
                hovering: false,
                radius: cce_ui::layout::button_corner_radius() * s,
                corners: (true, true, true, true),
            });
            let label = base.label.as_deref().unwrap_or("");
            let label_size = 12.0;
            let buf = make_text_buffer(&mut self.font_system, label, label_size * s);
            let scale = cce_ui::scale::scale_factor();
            let tw = buf.layout_runs().next().map(|r| r.line_w).unwrap_or(0.0) / scale;
            let lh = label_size * s * 1.4;
            let mut left_align = btn.left_align;

            // Auto-detect if inside a ScrollBox to apply left alignment by default
            if !left_align && base.w >= 60.0 {
                if self.app.current_page == Page::Hardware {
                    let sb = &self.app.hardware.cpu_list_box;
                    let (sb_x, sb_y, sb_w, sb_h) = sb.rect();
                    if base.x >= sb_x - 1.0 && base.x + base.w <= sb_x + sb_w + 1.0
                       && base.y >= sb_y - 1.0 && base.y + base.h <= sb_y + sb_h + 1.0 {
                        left_align = true;
                    }
                } else if self.app.current_page == Page::Services {
                    let sb = &self.app.services.list_box;
                    let (sb_x, sb_y, sb_w, sb_h) = sb.rect();
                    if base.x >= sb_x - 1.0 && base.x + base.w <= sb_x + sb_w + 1.0
                       && base.y >= sb_y - 1.0 && base.y + base.h <= sb_y + sb_h + 1.0 {
                        left_align = true;
                    }
                }
            }

            let text_x = if left_align {
                base.x * s + 8.0 * s
            } else {
                base.x * s + (base.w * s - tw) / 2.0
            };

            let label_color = btn.label_color.unwrap_or([0.83, 0.83, 0.83, 1.0]);
            text_items.push(TextItem {
                buffer: buf,
                x: text_x, y: (base.y - scroll_offset_y) * s + (base.h * s - lh) / 2.0,
                color: glyphon::Color::rgb(
                    (label_color[0] * 255.0) as u8,
                    (label_color[1] * 255.0) as u8,
                    (label_color[2] * 255.0) as u8,
                ),
                bounds: None,
            });
            let mut btn_clone = btn.clone();
            if let Some(base_mut) = btn_clone.base_mut() {
                base_mut.x *= s;
                base_mut.y = (base_mut.y - scroll_offset_y) * s;
                base_mut.w *= s;
                base_mut.h *= s;
            }
            page_buttons.push((btn_clone, action.clone()));
        }

        // Render popovers on top of everything (both backgrounds and texts)
        let mut popover_pc = PageContent::new();
        cce_ui::layout::render_popovers(&mut popover_pc, &mut self.ui_context);

        for (c, x, y, w, h, r, corners) in &popover_pc.rects {
            widgets.push(AppWidget {
                x: *x * s, y: (*y - scroll_offset_y) * s, w: *w * s, h: *h * s,
                color: *c, hover_color: *c,
                hovering: false,
                radius: *r * s,
                corners: *corners,
            });
        }
        for (t, size, x, y, tc, font_opt, bounds) in &popover_pc.texts {
            let shifted_bounds = bounds.map(|[bl, bt, br, bb]| {
                [bl, bt - scroll_offset_y, br, bb - scroll_offset_y]
            });
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
                bounds: shifted_bounds,
            });
        }

        // Draw global hover highlight if active
        cce_ui::widget::hover_animation::post_render_check();
        if let Some((qx, qy, qw, qh, qc)) = cce_ui::widget::hover_animation::get_quad() {
            widgets.push(AppWidget {
                x: qx * s,
                y: (qy - self.scroll_y) * s,
                w: qw * s,
                h: qh * s,
                color: qc,
                hover_color: qc,
                hovering: false,
                radius: 0.0,
                corners: (true, true, true, true),
            });
        }





        self.widgets = widgets;
        self.text_items = text_items;
        self.page_buttons = page_buttons;
        self.needs_rebuild = false;
    }

    fn render_page_content(&mut self, cx: f32, cy: f32, cw: f32, ch: f32) -> PageContent {
        use pages::*;
        use cce_ui::layout::AdaptiveGrid;
        let margin = cce_ui::layout::page_margin();
        let cx = cx + margin;
        let cy = cy + margin;
        let cw = (cw - 2.0 * margin).max(1.0);
        let ch = (ch - 2.0 * margin).max(1.0);
        let mut layout = AdaptiveGrid::new(260.0, 20.0);
        let page_idx = Page::ALL.iter().position(|&p| p == self.app.current_page).unwrap_or(0);
        let root_focused = cce_ui::widget::focus::is_focused(&self.plates[page_idx]);
        let sec_focused: Vec<bool> = self.page_sec_containers.iter()
            .map(|c| cce_ui::widget::focus::is_focused(c))
            .collect();
        match self.app.current_page {
            Page::Accounts => accounts::view(&mut self.app.accounts, cx, cy, cw, ch, &mut layout, &mut self.ui_context),
            Page::Audio => audio::view(&mut self.app.audio, cx, cy, cw, ch, &sec_focused, &mut layout, &mut self.ui_context),
            Page::Display => display::view(&mut self.app.display, cx, cy, cw, ch, &mut layout, &mut self.ui_context),
            Page::Radios => network::view(&mut self.app.network, cx, cy, cw, ch, root_focused, &mut layout, &mut self.ui_context),
            Page::Layout => layout::view(&mut self.app.layout, cx, cy, cw, ch, &sec_focused, &mut layout, &mut self.ui_context),
            Page::Hardware => hardware::view(&mut self.app.hardware, cx, cy, cw, ch, root_focused, &mut layout, &mut self.ui_context),
            Page::Input => input::view(&mut self.app.input, cx, cy, cw, ch, &sec_focused, &mut layout, &mut self.ui_context),
            Page::System => system_info::view(&self.app.system_info, cx, cy, cw, ch, &mut layout, &mut self.ui_context),
            Page::Storage => storage::view(&self.app.storage, cx, cy, cw, ch, &mut layout, &mut self.ui_context),
            Page::Services => services::view(&mut self.app.services, cx, cy, cw, ch, &sec_focused, &mut layout, &mut self.ui_context),
            Page::Interface => interface::view(&mut self.app.interface, cx, cy, cw, ch, &sec_focused, &mut layout, &mut self.ui_context),
            Page::Packages => packages::view(&mut self.app.packages, cx, cy, cw, ch, &sec_focused, &mut layout, &mut self.ui_context),
        }
    }


    fn tick_internal(&mut self, dt: f32) -> bool {
        let mut needs_redraw = false;
        if hover_animation::tick(dt) {
            needs_redraw = true;
            self.needs_rebuild = true;
        }
        let menubar_changed = self.menubar.tick(dt, &mut self.ui_context);
        let switcher_changed = self.switcher.tick(dt, &mut self.ui_context);
        if menubar_changed || switcher_changed {
            needs_redraw = true;
            self.needs_rebuild = true;
        }
        if let Some(root_ptr) = self.get_page_root_widget() {
            unsafe {
                if (*root_ptr).tick(dt, &mut self.ui_context) {
                    needs_redraw = true;
                    self.needs_rebuild = true;
                }
            }
        }

        // Asynchronously check color selector changes (e.g. Zenity process exit)
        let mut color_changed = false;
        let mut color_actions = Vec::new();
        for (i, cp) in self.app.interface.color_selectors.iter_mut().enumerate() {
            if cp.tick(dt, &mut self.ui_context) {
                needs_redraw = true;
                self.needs_rebuild = true;
            }
            let state_color = match i {
                0 => self.app.interface.page_low_color,
                1 => self.app.interface.high_color,
                2 => self.app.interface.visual_guides_color,
                3 => self.app.interface.disabled_color,
                4 => self.app.interface.separator_color,
                5 => self.app.interface.slider_track_color,
                6 => self.app.interface.color_borders_color,
                7 => self.app.interface.low_color,
                8 => self.app.interface.normal_color,
                9 => self.app.interface.paginator_sidebar_color,
                10 => self.app.interface.primary_highlight_color,
                11 => self.app.interface.menubar_tab_label_color,
                12 => self.app.interface.toggle_enabled_color,
                13 => self.app.interface.toggle_disabled_color,
                14 => self.app.interface.scrollinglist_bg_color,
                15 => self.app.interface.breadcrumb_bg_color,
                16 => self.app.interface.popover_bg_color,
                17 => self.app.interface.notification_bg_color,
                18 => self.app.interface.window_color,
                _ => self.app.interface.low_color,
            };
            if cp.color != state_color {
                color_actions.push(AppAction::Interface(match i {
                    0 => pages::interface::InterfaceMessage::SetPageLowColor(cp.color),
                    1 => pages::interface::InterfaceMessage::SetHighColor(cp.color),
                    2 => pages::interface::InterfaceMessage::SetVisualGuidesColor(cp.color),
                    3 => pages::interface::InterfaceMessage::SetDisabledColor(cp.color),
                    4 => pages::interface::InterfaceMessage::SetSeparatorColor(cp.color),
                    5 => pages::interface::InterfaceMessage::SetSliderTrackColor(cp.color),
                    6 => pages::interface::InterfaceMessage::SetColorBordersColor(cp.color),
                    7 => pages::interface::InterfaceMessage::SetLowColor(cp.color),
                    8 => pages::interface::InterfaceMessage::SetNormalColor(cp.color),
                    9 => pages::interface::InterfaceMessage::SetPaginatorSidebarColor(cp.color),
                    10 => pages::interface::InterfaceMessage::SetPrimaryHighlightColor(cp.color),
                    11 => pages::interface::InterfaceMessage::SetMenubarTabLabelColor(cp.color),
                    12 => pages::interface::InterfaceMessage::SetToggleEnabledColor(cp.color),
                    13 => pages::interface::InterfaceMessage::SetToggleDisabledColor(cp.color),
                    14 => pages::interface::InterfaceMessage::SetScrollingListBgColor(cp.color),
                    15 => pages::interface::InterfaceMessage::SetBreadcrumbBgColor(cp.color),
                    16 => pages::interface::InterfaceMessage::SetPopoverBgColor(cp.color),
                    17 => pages::interface::InterfaceMessage::SetNotificationBgColor(cp.color),
                    18 => pages::interface::InterfaceMessage::SetWindowColor(cp.color),
                    _ => pages::interface::InterfaceMessage::SetLowColor(cp.color),
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
            if self.app.current_page == Page::Audio {
                self.needs_rebuild = true;
            }
        }
        while let Ok(s) = self.rx_display.try_recv() {
            display::update(&mut self.app.display, display::DisplayMessage::Refreshed(s));
            if self.app.current_page == Page::Display {
                self.needs_rebuild = true;
            }
        }
        while let Ok(s) = self.rx_network.try_recv() {
            network::update(&mut self.app.network, network::NetworkMessage::Refreshed(s));
            if self.app.current_page == Page::Radios {
                self.needs_rebuild = true;
            }
        }
        while let Ok(s) = self.rx_layout.try_recv() {
            layout::update(&mut self.app.layout, layout::LayoutMessage::Refreshed(s));
            if self.app.current_page == Page::Layout {
                self.needs_rebuild = true;
            }
        }
        while let Ok(_) = self.rx_wm_events.try_recv() {
            if self.app.current_page == Page::Layout {
                self.needs_rebuild = true;
            }
        }
        while let Ok(s) = self.rx_input.try_recv() {
            input::update(&mut self.app.input, input::InputMessage::Refreshed(s));
            if self.app.current_page == Page::Input {
                self.needs_rebuild = true;
            }
        }
        let mut got_fingers = None;
        while let Ok(s) = self.rx_fingers.try_recv() {
            got_fingers = Some(s);
        }
        if let Some(fingers) = got_fingers {
            input::update(&mut self.app.input, input::InputMessage::UpdateFingers(fingers));
            if self.app.current_page == Page::Input {
                self.needs_rebuild = true;
            }
        }
        while let Ok(s) = self.rx_system.try_recv() {
            system_info::update(&mut self.app.system_info, system_info::SystemMessage::Refreshed(s));
            if self.app.current_page == Page::System {
                self.needs_rebuild = true;
            }
        }
        while let Ok(s) = self.rx_hardware.try_recv() {
            hardware::update(&mut self.app.hardware, hardware::HardwareMessage::Refreshed(s));
            if self.app.current_page == Page::Hardware {
                self.needs_rebuild = true;
            }
        }
        while let Ok(s) = self.rx_status.try_recv() {
            services::update(&mut self.app.services, services::ServicesMessage::StatusRefreshed(s));
            if self.app.current_page == Page::Services {
                self.needs_rebuild = true;
            }
        }
        while let Ok(s) = self.rx_storage.try_recv() {
            storage::update(&mut self.app.storage, storage::StorageMessage::Refreshed(s));
            if self.app.current_page == Page::Storage {
                self.needs_rebuild = true;
            }
        }
        while let Ok(s) = self.rx_notifications.try_recv() {
            services::update(&mut self.app.services, services::ServicesMessage::NotificationsRefreshed(s));
            if self.app.current_page == Page::Services {
                self.needs_rebuild = true;
            }
        }
        while let Ok(s) = self.rx_typeface.try_recv() {
            self.sans_serif_family = s.sans_serif.clone();
            self.serif_family = s.serif.clone();
            self.monospace_family = s.monospace.clone();
            interface::update(&mut self.app.interface, interface::InterfaceMessage::TypefaceRefreshed(s));
            if self.app.current_page == Page::Interface {
                self.needs_rebuild = true;
            }
        }
        while let Ok(s) = self.rx_services.try_recv() {
            pages::services::update(&mut self.app.services, pages::services::ServicesMessage::Refreshed(s));
            if self.app.current_page == Page::Services {
                self.needs_rebuild = true;
            }
        }
        while let Ok(s) = self.rx_interface.try_recv() {
            interface::update(&mut self.app.interface, pages::interface::InterfaceMessage::Refreshed(s));
            if self.app.current_page == Page::Interface {
                self.needs_rebuild = true;
            }
        }
        while let Ok(s) = self.rx_accounts.try_recv() {
            accounts::update(&mut self.app.accounts, accounts::AccountsMessage::Refreshed(s));
            if self.app.current_page == Page::Accounts {
                self.needs_rebuild = true;
            }
        }
        while let Ok(m) = self.rx_backup.try_recv() {
            self.handle_action(&AppAction::Storage(m));
            if self.app.current_page == Page::Storage {
                self.needs_rebuild = true;
            }
        }
        while let Ok(s) = self.rx_packages.try_recv() {
            pages::packages::update(&mut self.app.packages, pages::packages::PackagesMessage::Refreshed(s));
            if self.app.current_page == Page::Packages {
                self.needs_rebuild = true;
            }
        }
        while let Ok(m) = self.rx_update.try_recv() {
            self.handle_action(&AppAction::Packages(m));
            if self.app.current_page == Page::Packages {
                self.needs_rebuild = true;
            }
        }
    }

    fn handle_action(&mut self, action: &AppAction) {
        use pages::*;
        match action {
            AppAction::Exit => {}
            AppAction::Audio(m) => audio::update(&mut self.app.audio, m.clone()),
            AppAction::Display(m) => display::update(&mut self.app.display, m.clone()),
            AppAction::Radios(m) => network::update(&mut self.app.network, m.clone()),
            AppAction::Layout(m) => layout::update(&mut self.app.layout, m.clone()),
            AppAction::Input(m) => input::update(&mut self.app.input, m.clone()),
            AppAction::SystemInfo(m) => system_info::update(&mut self.app.system_info, m.clone()),
            AppAction::Hardware(m) => hardware::update(&mut self.app.hardware, m.clone()),
            AppAction::Storage(m) => match m {
                pages::storage::StorageMessage::StartBackup => {
                    pages::storage::update(&mut self.app.storage, pages::storage::StorageMessage::StartBackup);
                    let tx = self.tx_backup.clone();
                    tokio::spawn(async move {
                        let res = pages::storage::run_backup().await;
                        let _ = tx.send(pages::storage::StorageMessage::BackupFinished(res));
                    });
                }
                _ => pages::storage::update(&mut self.app.storage, m.clone()),
            },

            AppAction::Services(m) => services::update(&mut self.app.services, m.clone()),
            AppAction::Interface(m) => {
                interface::update(&mut self.app.interface, m.clone());
                self.sans_serif_family = self.app.interface.sans_serif.clone();
                self.serif_family = self.app.interface.serif.clone();
                self.monospace_family = self.app.interface.monospace.clone();
            }
            AppAction::Accounts(m) => match m {
                pages::accounts::AccountsMessage::GoogleLoginInit => {
                    pages::accounts::update(&mut self.app.accounts, m.clone());
                    let sender = self.sender.clone();
                    tokio::spawn(async move {
                        pages::accounts::run_google_login(sender).await;
                    });
                }
                _ => pages::accounts::update(&mut self.app.accounts, m.clone()),
            },
            AppAction::Packages(m) => match m {
                pages::packages::PackagesMessage::StartUpdate => {
                    pages::packages::update(&mut self.app.packages, pages::packages::PackagesMessage::StartUpdate);
                    let tx = self.tx_update.clone();
                    tokio::spawn(async move {
                        let res = pages::packages::run_update().await;
                        let _ = tx.send(pages::packages::PackagesMessage::UpdateFinished(res));
                    });
                }
                pages::packages::PackagesMessage::UpdateFinished(res) => {
                    pages::packages::update(&mut self.app.packages, m.clone());
                    if res.is_ok() {
                        let tx = self.tx_update.clone();
                        tokio::spawn(async move {
                            let new_state = pages::packages::fetch_packages_state().await;
                            let _ = tx.send(pages::packages::PackagesMessage::Refreshed(new_state));
                        });
                    }
                }
                pages::packages::PackagesMessage::SelectPackage(Some(ref name)) => {
                    let name_clone = name.clone();
                    let is_installed = self.app.packages.active_tab == pages::packages::PackageTab::Installed;
                    pages::packages::update(&mut self.app.packages, m.clone());
                    let tx = self.tx_update.clone();
                    tokio::spawn(async move {
                        let res = pages::packages::fetch_package_info(name_clone.clone(), is_installed).await;
                        let _ = tx.send(pages::packages::PackagesMessage::InfoFetched(name_clone, res));
                    });
                }
                pages::packages::PackagesMessage::SelectAndScrollPackage(ref name) => {
                    let name_clone = name.clone();
                    pages::packages::update(&mut self.app.packages, m.clone());
                    let tx = self.tx_update.clone();
                    tokio::spawn(async move {
                        let res = pages::packages::fetch_package_info(name_clone.clone(), true).await;
                        let _ = tx.send(pages::packages::PackagesMessage::InfoFetched(name_clone, res));
                    });
                }
                pages::packages::PackagesMessage::StartUninstall(ref name) => {
                    if !self.app.packages.uninstalling {
                        pages::packages::update(&mut self.app.packages, m.clone());
                        let name_clone = name.clone();
                        let tx = self.tx_update.clone();
                        tokio::spawn(async move {
                            let res = pages::packages::run_uninstall(name_clone).await;
                            let _ = tx.send(pages::packages::PackagesMessage::UninstallFinished(res));
                        });
                    }
                }
                pages::packages::PackagesMessage::UninstallFinished(res) => {
                    pages::packages::update(&mut self.app.packages, m.clone());
                    if res.is_ok() {
                        let tx = self.tx_update.clone();
                        tokio::spawn(async move {
                            let new_state = pages::packages::fetch_packages_state().await;
                            let _ = tx.send(pages::packages::PackagesMessage::Refreshed(new_state));
                        });
                    }
                }
                _ => pages::packages::update(&mut self.app.packages, m.clone()),
            },
        }
    }

    fn handle_cursor_moved(&mut self, x: f32, y: f32) -> bool {
        self.cursor_x = x;
        self.cursor_y = y;
        let s = 1.0f32;
        let lx_no_scroll = x / s;
        let ly_no_scroll = y / s;
        
        if cce_ui::widget::context_menu::is_visible() {
            if cce_ui::widget::context_menu::cursor_moved(lx_no_scroll, ly_no_scroll) {
                self.needs_rebuild = true;
                return true;
            }
            return false;
        }

        let lx = self.cursor_x / s;
        let ly = self.cursor_y / s + self.scroll_y;
        cce_ui::widget::hover_animation::set_cursor_pos(lx, ly_no_scroll);
        let mut changed = false;
        if lx_no_scroll < self.sidebar_width {
            if self.menubar.cursor_moved(lx_no_scroll, ly_no_scroll, &mut self.ui_context) {
                changed = true;
            }
        } else {
            if self.switcher.cursor_moved(lx_no_scroll, ly_no_scroll, &mut self.ui_context) {
                changed = true;
            }
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
                if sb.cursor_moved(lx, ly, &mut self.ui_context) {
                    changed = true;
                }
            }
            if self.app.layout.cascade_offset_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.layout.edge_gap_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.layout.top_gap_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.layout.grid_gap_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.layout.transition_duration_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.layout.status_height_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            for menu in &mut self.app.layout.tag_layout_menus {
                if menu.cursor_moved(lx, ly, &mut self.ui_context) {
                    changed = true;
                }
            }
            if self.app.layout.side_panel_behavior_menu.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.layout.side_panel_position_menu.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.layout.side_panel_width_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.layout.side_panel_border_gap_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.layout.side_panel_border_opacity_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.layout.transparency_toggle.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.layout.blur_toggle.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
        }
        if self.app.current_page == Page::Interface {
            for cp in &mut self.app.interface.color_selectors {
                if cp.cursor_moved(lx, ly, &mut self.ui_context) {
                    changed = true;
                }
            }
            if self.app.interface.tab_margin_spinbox_x.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.menubar_opacity_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.tab_margin_spinbox_y.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.button_padding_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.section_padding_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.label_alignment_menu.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.label_offset_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.label_margin_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.plate_padding_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.graph_show_grid_toggle.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.graph_snap_enabled_toggle.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.graph_uniform_background_toggle.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.graph_cell_opacity_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.graph_gap_opacity_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.graph_gap_width_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.page_margin_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.grid_min_col_width_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.spinbox_height_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.spinbox_corner_radius_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.toggle_height_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.toggle_corner_radius_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.plate_opacity_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.plate_corner_radius_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.page_opacity_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.layer_opacity_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }


            if self.app.interface.color_selector_height_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.color_selector_corner_radius_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.color_selector_preview_corner_radius_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.color_selector_preview_margin_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.color_selector_font_selector.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.menubar_font_selector.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.section_label_font_selector.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.nested_section_label_font_selector.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.textbox_height_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.textbox_corner_radius_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.slider_height_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.font_selector_height_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.font_selector_corner_radius_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.dropdown_height_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.dropdown_corner_radius_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.button_corner_radius_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.notification_opacity_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.window_opacity_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.window_corner_radius_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.sans_box.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.serif_box.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.mono_box.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.borders_menu.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.borders_box.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.status_menu.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.status_box.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.fuzzel_menu.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.fuzzel_box.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.terminal_menu.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.terminal_box.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.borders_size_box.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.status_size_box.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.fuzzel_size_box.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.terminal_size_box.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.custom_multicontrol.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
        }
        if self.app.current_page == Page::Input {
            if self.app.input.rate_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.input.delay_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.input.tap_toggle.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.input.scroll_toggle.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.input.scroll_friction_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.input.pointer_toggle.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.input.pointer_friction_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.input.trackpad_toggle.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.input.trackpad_friction_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.input.dwtp_toggle.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.input.trackpoint_accel_speed_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.input.trackpoint_accel_profile_menu.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.input.zoom_in_box.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.input.zoom_out_box.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
        }
        if self.app.current_page == Page::Audio {
            if let Some(idx) = self.audio_sink_dragging {
                if idx < self.app.audio.sink_sliders.len() {
                    if self.app.audio.sink_sliders[idx].drag_update(lx, ly) {
                        changed = true;
                        let val = self.app.audio.sink_sliders[idx].value();
                        if idx < self.app.audio.sink_spinboxes.len() {
                            self.app.audio.sink_spinboxes[idx].value = (val * 100.0).round() as i32;
                        }
                        if idx < self.app.audio.sinks.len() {
                            let id = self.app.audio.sinks[idx].id;
                            self.handle_action(&AppAction::Audio(pages::audio::AudioMessage::SinkVolume(id, val)));
                        }
                    }
                }
            } else if let Some(idx) = self.audio_source_dragging {
                if idx < self.app.audio.source_sliders.len() {
                    if self.app.audio.source_sliders[idx].drag_update(lx, ly) {
                        changed = true;
                        let val = self.app.audio.source_sliders[idx].value();
                        if idx < self.app.audio.source_spinboxes.len() {
                            self.app.audio.source_spinboxes[idx].value = (val * 100.0).round() as i32;
                        }
                        if idx < self.app.audio.sources.len() {
                            let id = self.app.audio.sources[idx].id;
                            self.handle_action(&AppAction::Audio(pages::audio::AudioMessage::SourceVolume(id, val)));
                        }
                    }
                }
            } else {
                for sb in &mut self.app.audio.sink_spinboxes {
                    if sb.cursor_moved(lx, ly, &mut self.ui_context) {
                        changed = true;
                    }
                }
                for sb in &mut self.app.audio.source_spinboxes {
                    if sb.cursor_moved(lx, ly, &mut self.ui_context) {
                        changed = true;
                    }
                }
                for slider in &mut self.app.audio.sink_sliders {
                    if slider.cursor_moved(lx, ly, &mut self.ui_context) {
                        changed = true;
                    }
                }
                for slider in &mut self.app.audio.source_sliders {
                    if slider.cursor_moved(lx, ly, &mut self.ui_context) {
                        changed = true;
                    }
                }
            }
        }
        if self.app.current_page == Page::Display {
            if self.display_brightness_dragging {
                if self.app.display.brightness_slider.drag_update(lx, ly) {
                    changed = true;
                    let val = self.app.display.brightness_slider.value();
                    let pct = (val * 100.0).round() as u32;
                    self.app.display.brightness_spinbox.value = pct as i32;
                    self.handle_action(&AppAction::Display(pages::display::DisplayMessage::BrightnessSet(pct)));
                }
            } else {
                if self.app.display.brightness_slider.cursor_moved(lx, ly, &mut self.ui_context) {
                    changed = true;
                }
                if self.app.display.brightness_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                    changed = true;
                }
            }
            if self.app.display.night_light_label.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            for out in &mut self.app.display.outputs {
                if out.name_label.cursor_moved(lx, ly, &mut self.ui_context) {
                    changed = true;
                }
                if out.resolution_label.cursor_moved(lx, ly, &mut self.ui_context) {
                    changed = true;
                }
                if let Some(ref mut scale_lbl) = out.scale_label {
                    if scale_lbl.cursor_moved(lx, ly, &mut self.ui_context) {
                        changed = true;
                    }
                }
            }
            if self.app.display.screensaver_enable_toggle.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.display.screensaver_lock_screen_toggle.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.display.screensaver_timeout_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.display.screensaver_style_menu.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
        }

        if self.app.current_page == Page::Hardware {
            if self.app.hardware.cpu_label.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.hardware.cpu_usage_label.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.hardware.cpu_temp_label.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            for gpu_lbl in &mut self.app.hardware.gpu_labels {
                if gpu_lbl.cursor_moved(lx, ly, &mut self.ui_context) {
                    changed = true;
                }
            }
            if self.app.hardware.cpu_list_box.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.hardware.cpu_gov_menu.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.hardware.gpu_gov_menu.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
        }

        if self.app.current_page == Page::Packages {
            let pkgs = &mut self.app.packages;
            if pkgs.search_box.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            match pkgs.active_tab {
                pages::packages::PackageTab::Installed => {
                    if pkgs.installed_list_box.cursor_moved(lx, ly, &mut self.ui_context) {
                        changed = true;
                    }
                    let query = if pkgs.search_box.editing {
                        pkgs.search_box.edit_buffer.to_lowercase()
                    } else {
                        pkgs.search_box.text.to_lowercase()
                    };
                    let matching = pkgs.installed.iter()
                        .filter(|p| p.name.to_lowercase().contains(&query) || p.version.to_lowercase().contains(&query))
                        .count();
                    for i in 0..matching.min(pkgs.installed_items.len()) {
                        if pkgs.installed_items[i].cursor_moved(lx, ly, &mut self.ui_context) {
                            changed = true;
                        }
                    }
                }
                pages::packages::PackageTab::Updates => {
                    if pkgs.updates_list_box.cursor_moved(lx, ly, &mut self.ui_context) {
                        changed = true;
                    }
                    let query = if pkgs.search_box.editing {
                        pkgs.search_box.edit_buffer.to_lowercase()
                    } else {
                        pkgs.search_box.text.to_lowercase()
                    };
                    let matching = pkgs.updates.iter()
                        .filter(|p| p.name.to_lowercase().contains(&query))
                        .count();
                    for i in 0..matching.min(pkgs.updates_items.len()) {
                        if pkgs.updates_items[i].cursor_moved(lx, ly, &mut self.ui_context) {
                            changed = true;
                        }
                    }
                }
            }
        }



        if self.app.current_page == Page::Services {
            if self.app.services.search_box.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.services.list_box.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            let query = if self.app.services.search_box.editing {
                self.app.services.search_box.edit_buffer.to_lowercase()
            } else {
                self.app.services.search_box.text.to_lowercase()
            };
            let matching_count = self.app.services.services.iter()
                .filter(|s| s.is_system == (self.app.services.active_tab == pages::services::ServiceTab::System))
                .filter(|s| s.name.to_lowercase().contains(&query) || s.description.to_lowercase().contains(&query))
                .count();
            for i in 0..matching_count.min(self.app.services.service_items.len()) {
                if self.app.services.service_items[i].cursor_moved(lx, ly, &mut self.ui_context) {
                    changed = true;
                }
            }
            if self.app.services.notifications_enable_toggle.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.services.notifications_bell_toggle.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.services.notifications_duration_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.services.status_label.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.services.status_separators_toggle.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.services.status_underline_toggle.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.services.status_padding_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
        }
        if self.app.current_page == Page::Accounts {
            if self.app.accounts.editing_oauth_creds {
                if self.app.accounts.oauth_client_id_box.cursor_moved(lx, ly, &mut self.ui_context) { changed = true; }
                if self.app.accounts.oauth_client_secret_box.cursor_moved(lx, ly, &mut self.ui_context) { changed = true; }
            } else if self.app.accounts.adding_new {
                if self.app.accounts.email_box.cursor_moved(lx, ly, &mut self.ui_context) { changed = true; }
                if self.app.accounts.password_box.cursor_moved(lx, ly, &mut self.ui_context) { changed = true; }
                if self.app.accounts.imap_box.cursor_moved(lx, ly, &mut self.ui_context) { changed = true; }
                if self.app.accounts.smtp_box.cursor_moved(lx, ly, &mut self.ui_context) { changed = true; }
            }
        }
        if changed { self.needs_rebuild = true; }
        changed
    }

    fn handle_mouse_input_internal(&mut self, button: cce_ui::widget::MouseButton, state: cce_ui::widget::ElementState) -> bool {
        let s = 1.0f32;
        let lx_no_scroll = self.cursor_x / s;
        let ly_no_scroll = self.cursor_y / s;

        // CSD Close Button Interaction removed

        if cce_ui::widget::context_menu::is_visible() {
            if cce_ui::widget::context_menu::mouse_input(button, state, lx_no_scroll, ly_no_scroll) {
                let mut actions = Vec::new();
                self.propagate_widget_changes(&mut actions);
                for action in actions {
                    self.handle_action(&action);
                }
                self.needs_rebuild = true;
                return true;
            }
        }

        if lx_no_scroll < self.sidebar_width {
            if self.menubar.mouse_input(button, state, lx_no_scroll, ly_no_scroll, &mut self.ui_context) {
                use cce_ui::widget::MenuController;
                if let Some((idx, _)) = self.menubar.menu_click() {
                    if idx < Page::ALL.len() {
                        cce_ui::widget::focus::clear_focus();
                        let new_page = Page::ALL[idx];
                        self.app.current_page = new_page;
                        self.current_page_shared.store(idx as u8, std::sync::atomic::Ordering::SeqCst);
                        self.scroll_y = 0.0;
                        pages::interface::write_config_value("last_page", &format!("\"{}\"", new_page.label().to_lowercase()));
                    }
                }
                self.needs_rebuild = true;
                return true;
            }
        } else {
            if self.switcher.mouse_input(button, state, lx_no_scroll, ly_no_scroll, &mut self.ui_context) {
                self.needs_rebuild = true;
                return true;
            }
        }

        if button != cce_ui::widget::MouseButton::Left && button != cce_ui::widget::MouseButton::Right { return false; }
        if button == cce_ui::widget::MouseButton::Left && state == cce_ui::widget::ElementState::Released {
            if self.app.current_page == Page::Audio {
                let mut ended = false;
                if let Some(idx) = self.audio_sink_dragging {
                    if idx < self.app.audio.sink_sliders.len() {
                        self.app.audio.sink_sliders[idx].drag_end();
                    }
                    self.audio_sink_dragging = None;
                    ended = true;
                }
                if let Some(idx) = self.audio_source_dragging {
                    if idx < self.app.audio.source_sliders.len() {
                        self.app.audio.source_sliders[idx].drag_end();
                    }
                    self.audio_source_dragging = None;
                    ended = true;
                }
                if ended {
                    self.needs_rebuild = true;
                }
            }
            if self.app.current_page == Page::Display {
                if self.display_brightness_dragging {
                    self.app.display.brightness_slider.drag_end();
                    self.display_brightness_dragging = false;
                    self.needs_rebuild = true;
                }
            }

            let (px, py) = (self.cursor_x, self.cursor_y);
            for (btn, action) in &self.page_buttons.clone() {
                let base = btn.base().unwrap();
                if px >= base.x && px <= base.x + base.w && py >= base.y && py <= base.y + base.h {
                    self.handle_action(action);
                    self.needs_rebuild = true;
                    return true;
                }
            }
        }
        let lx = self.cursor_x / s;
        let ly = self.cursor_y / s + self.scroll_y;
        let mut actions = Vec::new();

        if state == cce_ui::widget::ElementState::Pressed {
            let mut clicked_any_focusable = false;
            match self.app.current_page {
                Page::Accounts => {
                    let accs = &mut self.app.accounts;
                    if accs.editing_oauth_creds {
                        if accs.oauth_client_id_box.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                        if accs.oauth_client_secret_box.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    } else if accs.adding_new {
                        if accs.email_box.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                        if accs.password_box.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                        if accs.imap_box.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                        if accs.smtp_box.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    }
                }
                Page::Layout => {
                    for sb in &mut self.app.layout.spinboxes {
                        if sb.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    }
                    if self.app.layout.cascade_offset_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.layout.edge_gap_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.layout.top_gap_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.layout.grid_gap_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.layout.transition_duration_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.layout.status_height_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    for menu in &mut self.app.layout.tag_layout_menus {
                        if menu.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    }
                    if self.app.layout.side_panel_behavior_menu.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.layout.side_panel_position_menu.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.layout.side_panel_width_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.layout.side_panel_border_gap_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.layout.side_panel_border_opacity_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.layout.transparency_toggle.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.layout.blur_toggle.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                }
                Page::Interface => {
                    if self.app.interface.custom_multicontrol.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    for cp in &mut self.app.interface.color_selectors {
                        if cp.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    }
                    if self.app.interface.tab_margin_spinbox_x.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.menubar_opacity_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.tab_margin_spinbox_y.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.button_padding_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.section_padding_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.label_alignment_menu.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.label_offset_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.label_margin_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.plate_padding_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.graph_show_grid_toggle.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.graph_snap_enabled_toggle.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.graph_uniform_background_toggle.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.graph_cell_opacity_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.graph_gap_opacity_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.graph_gap_width_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.page_margin_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.grid_min_col_width_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.spinbox_height_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.spinbox_corner_radius_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.toggle_height_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.toggle_corner_radius_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.plate_opacity_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.plate_corner_radius_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.page_opacity_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.layer_opacity_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }


                    if self.app.interface.color_selector_height_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.color_selector_corner_radius_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.color_selector_preview_corner_radius_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.color_selector_preview_margin_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.color_selector_font_selector.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.menubar_font_selector.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.section_label_font_selector.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.nested_section_label_font_selector.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.textbox_height_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.textbox_corner_radius_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.slider_height_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.font_selector_height_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.font_selector_corner_radius_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.dropdown_height_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.dropdown_corner_radius_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.button_corner_radius_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.notification_opacity_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.window_opacity_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.window_corner_radius_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    let tf = &mut self.app.interface;
                    if tf.sans_box.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if tf.serif_box.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if tf.mono_box.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if tf.borders_menu.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if tf.borders_box.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if tf.status_menu.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if tf.status_box.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if tf.fuzzel_menu.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if tf.fuzzel_box.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if tf.terminal_menu.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if tf.terminal_box.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                }
                Page::Input => {
                    if self.app.input.rate_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.input.delay_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.input.scroll_friction_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.input.pointer_friction_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.input.trackpad_friction_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.input.trackpoint_accel_speed_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.input.trackpoint_accel_profile_menu.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.input.zoom_in_box.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.input.zoom_out_box.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                }

                Page::Audio => {
                    for sb in &mut self.app.audio.sink_spinboxes {
                        if sb.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    }
                    for sb in &mut self.app.audio.source_spinboxes {
                        if sb.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    }
                    for slider in &mut self.app.audio.sink_sliders {
                        if slider.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    }
                    for slider in &mut self.app.audio.source_sliders {
                        if slider.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    }
                }
                Page::Display => {
                    if self.app.display.brightness_slider.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.display.brightness_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.display.night_light_label.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    for out in &mut self.app.display.outputs {
                        if out.name_label.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                        if out.resolution_label.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                        if let Some(ref mut scale_lbl) = out.scale_label {
                            if scale_lbl.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                        }
                    }
                    if self.app.display.screensaver_timeout_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.display.screensaver_style_menu.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                }
                Page::Services => {
                    let srv = &mut self.app.services;
                    if srv.search_box.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if srv.list_box.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if srv.notifications_duration_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if srv.status_label.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if srv.status_padding_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                }
                Page::Packages => {
                    let pkgs = &mut self.app.packages;
                    if pkgs.search_box.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    match pkgs.active_tab {
                        pages::packages::PackageTab::Installed => {
                            if pkgs.installed_list_box.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                        }
                        pages::packages::PackageTab::Updates => {
                            if pkgs.updates_list_box.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                        }
                    }
                }
                Page::Hardware => {
                    let hw = &mut self.app.hardware;
                    if hw.cpu_list_box.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if hw.cpu_gov_menu.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if hw.gpu_gov_menu.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                }
                Page::Radios => {
                    let net = &mut self.app.network;
                    if net.wifi_list_box.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                }
                _ => {}
            }

            if !clicked_any_focusable {
                cce_ui::widget::focus::clear_focus();
            }
        }

        if state == cce_ui::widget::ElementState::Pressed && self.app.current_page == Page::Layout {
            for (i, sb) in self.app.layout.spinboxes.iter_mut().enumerate() {
                if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
                let old = sb.value;
                if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                    actions.push(AppAction::Layout(
                        pages::layout::LayoutMessage::SetWidth(
                            pages::layout::WidthParam::ALL[i],
                            sb.value as u16,
                        )
                    ));
                }
            }
            let sb = &mut self.app.layout.cascade_offset_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Layout(
                    pages::layout::LayoutMessage::SetCascadeOffset(sb.value as u16)
                ));
            }
            let sb = &mut self.app.layout.edge_gap_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Layout(
                    pages::layout::LayoutMessage::SetEdgeGap(sb.value as u16)
                ));
            }
            let sb = &mut self.app.layout.top_gap_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Layout(
                    pages::layout::LayoutMessage::SetTopGap(sb.value as u16)
                ));
            }
            let sb = &mut self.app.layout.grid_gap_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Layout(
                    pages::layout::LayoutMessage::SetGridGap(sb.value as u16)
                ));
            }
            let sb = &mut self.app.layout.transition_duration_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Layout(
                    pages::layout::LayoutMessage::SetTransitionDuration(sb.value as u16)
                ));
            }
            let sb = &mut self.app.layout.status_height_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Layout(
                    pages::layout::LayoutMessage::SetStatusHeight(sb.value as u16)
                ));
            }
        }
        if self.app.current_page == Page::Layout {
            for (idx, menu) in self.app.layout.tag_layout_menus.iter_mut().enumerate() {
                if state == cce_ui::widget::ElementState::Pressed && !menu.hit_test(lx, ly, &self.ui_context) { menu.unfocus(); }
                if menu.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                    self.needs_rebuild = true;
                }
                if state == cce_ui::widget::ElementState::Pressed && menu.take_change() {
                    actions.push(AppAction::Layout(pages::layout::LayoutMessage::SetTagLayout(idx + 1, menu.selected)));
                }
            }
            let menu = &mut self.app.layout.side_panel_behavior_menu;
            if state == cce_ui::widget::ElementState::Pressed && !menu.hit_test(lx, ly, &self.ui_context) { menu.unfocus(); }
            if menu.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if state == cce_ui::widget::ElementState::Pressed && menu.take_change() {
                actions.push(AppAction::Layout(pages::layout::LayoutMessage::SetSidePanelBehavior(menu.selected)));
            }
            let menu = &mut self.app.layout.side_panel_position_menu;
            if state == cce_ui::widget::ElementState::Pressed && !menu.hit_test(lx, ly, &self.ui_context) { menu.unfocus(); }
            if menu.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if state == cce_ui::widget::ElementState::Pressed && menu.take_change() {
                actions.push(AppAction::Layout(pages::layout::LayoutMessage::SetSidePanelPosition(menu.selected)));
            }
            let sb = &mut self.app.layout.side_panel_width_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Layout(
                    pages::layout::LayoutMessage::SetSidePanelWidth(sb.value as u16)
                ));
            }
            let sb = &mut self.app.layout.side_panel_border_gap_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Layout(
                    pages::layout::LayoutMessage::SetSidePanelBorderGap(sb.value as u16)
                ));
            }
            let sb = &mut self.app.layout.side_panel_border_opacity_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Layout(
                    pages::layout::LayoutMessage::SetSidePanelBorderOpacity(sb.value as u16)
                ));
            }
            let toggle = &mut self.app.layout.transparency_toggle;
            toggle.mouse_input(button, state, lx, ly, &mut self.ui_context);
            if toggle.take_click() {
                actions.push(AppAction::Layout(pages::layout::LayoutMessage::ToggleTransparency));
            }
            let toggle2 = &mut self.app.layout.blur_toggle;
            toggle2.mouse_input(button, state, lx, ly, &mut self.ui_context);
            if toggle2.take_click() {
                actions.push(AppAction::Layout(pages::layout::LayoutMessage::ToggleBlur));
            }
        }
        if state == cce_ui::widget::ElementState::Pressed && self.app.current_page == Page::Interface {
            for (i, cp) in self.app.interface.color_selectors.iter_mut().enumerate() {
                let old = cp.color;
                if !cp.hit_test(lx, ly, &self.ui_context) { cp.unfocus(); }
                cp.mouse_input(button, state, lx, ly, &mut self.ui_context);
                if cp.take_click() {
                    actions.push(AppAction::Interface(match i {
                        0 => pages::interface::InterfaceMessage::PickPageLowColor,
                        1 => pages::interface::InterfaceMessage::PickHighColor,
                        2 => pages::interface::InterfaceMessage::PickVisualGuides,
                        3 => pages::interface::InterfaceMessage::PickDisabledColor,
                        4 => pages::interface::InterfaceMessage::PickSeparatorColor,
                        5 => pages::interface::InterfaceMessage::PickSliderTrackColor,
                        6 => pages::interface::InterfaceMessage::PickColorBordersColor,
                        7 => pages::interface::InterfaceMessage::PickLowColor,
                        8 => pages::interface::InterfaceMessage::PickNormalColor,
                        9 => pages::interface::InterfaceMessage::PickPaginatorSidebarColor,
                        10 => pages::interface::InterfaceMessage::PickPrimaryHighlightColor,
                        11 => pages::interface::InterfaceMessage::PickMenubarTabLabelColor,
                        12 => pages::interface::InterfaceMessage::PickToggleEnabledColor,
                        13 => pages::interface::InterfaceMessage::PickToggleDisabledColor,
                        14 => pages::interface::InterfaceMessage::PickScrollingListBgColor,
                        15 => pages::interface::InterfaceMessage::PickBreadcrumbBgColor,
                        16 => pages::interface::InterfaceMessage::PickPopoverBgColor,
                        17 => pages::interface::InterfaceMessage::PickNotificationBgColor,
                        18 => pages::interface::InterfaceMessage::PickWindowColor,
                        19 => pages::interface::InterfaceMessage::PickPageColor,
                        20 => pages::interface::InterfaceMessage::PickLayerColor,
                        _ => pages::interface::InterfaceMessage::PickLowColor,
                    }));
                }
                if cp.color != old {
                    actions.push(AppAction::Interface(match i {
                        0 => pages::interface::InterfaceMessage::SetPageLowColor(cp.color),
                        1 => pages::interface::InterfaceMessage::SetHighColor(cp.color),
                        2 => pages::interface::InterfaceMessage::SetVisualGuidesColor(cp.color),
                        3 => pages::interface::InterfaceMessage::SetDisabledColor(cp.color),
                        4 => pages::interface::InterfaceMessage::SetSeparatorColor(cp.color),
                        5 => pages::interface::InterfaceMessage::SetSliderTrackColor(cp.color),
                        6 => pages::interface::InterfaceMessage::SetColorBordersColor(cp.color),
                        7 => pages::interface::InterfaceMessage::SetLowColor(cp.color),
                        8 => pages::interface::InterfaceMessage::SetNormalColor(cp.color),
                        9 => pages::interface::InterfaceMessage::SetPaginatorSidebarColor(cp.color),
                        10 => pages::interface::InterfaceMessage::SetPrimaryHighlightColor(cp.color),
                        11 => pages::interface::InterfaceMessage::SetMenubarTabLabelColor(cp.color),
                        12 => pages::interface::InterfaceMessage::SetToggleEnabledColor(cp.color),
                        13 => pages::interface::InterfaceMessage::SetToggleDisabledColor(cp.color),
                        14 => pages::interface::InterfaceMessage::SetScrollingListBgColor(cp.color),
                        15 => pages::interface::InterfaceMessage::SetBreadcrumbBgColor(cp.color),
                        16 => pages::interface::InterfaceMessage::SetPopoverBgColor(cp.color),
                        17 => pages::interface::InterfaceMessage::SetNotificationBgColor(cp.color),
                        18 => pages::interface::InterfaceMessage::SetWindowColor(cp.color),
                        19 => pages::interface::InterfaceMessage::SetPageColor(cp.color),
                        20 => pages::interface::InterfaceMessage::SetLayerColor(cp.color),
                        _ => pages::interface::InterfaceMessage::SetLowColor(cp.color),
                    }));
                }
            }
            let sb = &mut self.app.interface.menubar_opacity_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetMenubarOpacity(sb.value as f32 / 100.0)));
            }
            let sb = &mut self.app.interface.notification_opacity_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetNotificationOpacity(sb.value as f32 / 100.0)));
            }
            let sb = &mut self.app.interface.window_opacity_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetWindowOpacity(sb.value as f32 / 100.0)));
            }
            let sb = &mut self.app.interface.window_corner_radius_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetWindowCornerRadius(sb.value as u16)));
            }
            let sb = &mut self.app.interface.tab_margin_spinbox_x;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetTabMarginX(sb.value as u16)));
            }
            let sb = &mut self.app.interface.tab_margin_spinbox_y;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetTabMarginY(sb.value as u16)));
            }
            let sb = &mut self.app.interface.button_padding_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetButtonPadding(sb.value as u16)));
            }
            let sb = &mut self.app.interface.section_padding_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetSectionPadding(sb.value as u16)));
            }
            let menu = &mut self.app.interface.label_alignment_menu;
            if state == cce_ui::widget::ElementState::Pressed && !menu.hit_test(lx, ly, &self.ui_context) { menu.unfocus(); }
            if menu.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if state == cce_ui::widget::ElementState::Pressed && menu.take_change() {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetNestedSectionLabelAlignment(menu.selected)));
            }
            let sb = &mut self.app.interface.label_offset_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetNestedSectionLabelOffset(sb.value as i16)));
            }
            let sb = &mut self.app.interface.label_margin_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetLabelMargin(sb.value as u16)));
            }
            let sb = &mut self.app.interface.plate_padding_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetPlatePadding(sb.value as u16)));
            }
            let toggle = &mut self.app.interface.graph_show_grid_toggle;
            toggle.mouse_input(button, state, lx, ly, &mut self.ui_context);
            if toggle.take_click() {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetGraphShowGrid(toggle.toggled())));
            }
            let toggle = &mut self.app.interface.graph_snap_enabled_toggle;
            toggle.mouse_input(button, state, lx, ly, &mut self.ui_context);
            if toggle.take_click() {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetGraphSnapEnabled(toggle.toggled())));
            }
            let toggle = &mut self.app.interface.graph_uniform_background_toggle;
            toggle.mouse_input(button, state, lx, ly, &mut self.ui_context);
            if toggle.take_click() {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetGraphUniformBackground(toggle.toggled())));
            }
            let sb = &mut self.app.interface.graph_gap_width_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetGraphGapWidth(sb.value as u16)));
            }
            let sb = &mut self.app.interface.graph_cell_opacity_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetGraphCellOpacity(sb.value as f32 / 100.0)));
            }
            let sb = &mut self.app.interface.graph_gap_opacity_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetGraphGapOpacity(sb.value as f32 / 100.0)));
            }
            let sb = &mut self.app.interface.page_margin_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetPageMargin(sb.value as u16)));
            }
            let sb = &mut self.app.interface.grid_min_col_width_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetGridMinColWidth(sb.value as u16)));
            }
            let sb = &mut self.app.interface.spinbox_height_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetSpinboxHeight(sb.value as u16)));
            }
            let sb = &mut self.app.interface.spinbox_corner_radius_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetSpinboxCornerRadius(sb.value as u16)));
            }
            let sb = &mut self.app.interface.toggle_height_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetToggleHeight(sb.value as u16)));
            }
            let sb = &mut self.app.interface.toggle_corner_radius_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetToggleCornerRadius(sb.value as u16)));
            }
            let sb = &mut self.app.interface.plate_opacity_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetPlateOpacity(sb.value as f32 / 100.0)));
            }
            let sb = &mut self.app.interface.plate_corner_radius_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetPlateCornerRadius(sb.value as u16)));
            }
            let sb = &mut self.app.interface.page_opacity_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetPageOpacity(sb.value as f32 / 100.0)));
            }
            let sb = &mut self.app.interface.layer_opacity_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetLayerOpacity(sb.value as f32 / 100.0)));
            }


            let sb = &mut self.app.interface.color_selector_height_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetColorSelectorHeight(sb.value as u16)));
            }
            let sb = &mut self.app.interface.color_selector_corner_radius_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetColorSelectorCornerRadius(sb.value as u16)));
            }
            let sb = &mut self.app.interface.color_selector_preview_corner_radius_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetColorSelectorPreviewCornerRadius(sb.value as u16)));
            }
            let sb = &mut self.app.interface.color_selector_preview_margin_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetColorSelectorPreviewMargin(sb.value as u16)));
            }
            let sb = &mut self.app.interface.textbox_height_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetTextboxHeight(sb.value as u16)));
            }
            let sb = &mut self.app.interface.textbox_corner_radius_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetTextboxCornerRadius(sb.value as u16)));
            }
            let sb = &mut self.app.interface.slider_height_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetSliderHeight(sb.value as u16)));
            }
            let sb = &mut self.app.interface.font_selector_height_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetFontSelectorHeight(sb.value as u16)));
            }
            let sb = &mut self.app.interface.font_selector_corner_radius_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetFontSelectorCornerRadius(sb.value as u16)));
            }
            let sb = &mut self.app.interface.dropdown_height_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetDropdownHeight(sb.value as u16)));
            }
            let sb = &mut self.app.interface.dropdown_corner_radius_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetDropdownCornerRadius(sb.value as u16)));
            }
            let sb = &mut self.app.interface.button_corner_radius_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetButtonCornerRadius(sb.value as u16)));
            }
        }

        if self.app.current_page == Page::Interface {
            let mc = &mut self.app.interface.custom_multicontrol;
            if state == cce_ui::widget::ElementState::Pressed && !mc.hit_test(lx, ly, &self.ui_context) {
                mc.unfocus();
            }
            if mc.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
        }
        if state == cce_ui::widget::ElementState::Pressed && self.app.current_page == Page::Input {
            let sb = &mut self.app.input.rate_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Input(pages::input::InputMessage::ApplyRepeat));
            }
            let sb = &mut self.app.input.delay_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Input(pages::input::InputMessage::ApplyRepeat));
            }
            let sb = &mut self.app.input.scroll_friction_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Input(pages::input::InputMessage::ApplyScrollFriction));
            }
            let sb = &mut self.app.input.scroll_speed_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Input(pages::input::InputMessage::ApplyScrollSpeed));
            }
            let sb = &mut self.app.input.pointer_friction_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Input(pages::input::InputMessage::ApplyPointerFriction));
            }
            let sb = &mut self.app.input.trackpad_friction_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Input(pages::input::InputMessage::ApplyTrackpadFriction));
            }
            let sb = &mut self.app.input.trackpoint_accel_speed_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Input(pages::input::InputMessage::ApplyTrackpointAccelSpeed));
            }
            let sb = &mut self.app.input.cursor_size_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Input(pages::input::InputMessage::ApplyCursorSize));
            }
        }
        if state == cce_ui::widget::ElementState::Pressed && self.app.current_page == Page::Services {
            let sb = &mut self.app.services.notifications_duration_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Services(pages::services::ServicesMessage::SetNotificationsDuration(sb.value)));
            }
            let sb2 = &mut self.app.services.status_padding_spinbox;
            if !sb2.hit_test(lx, ly, &self.ui_context) { sb2.unfocus(); }
            let old2 = sb2.value;
            if sb2.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb2.value != old2 {
                actions.push(AppAction::Services(pages::services::ServicesMessage::StatusSetPadding(sb2.value as u16)));
            }
        }
        if self.app.current_page == Page::Input {
            let toggle = &mut self.app.input.tap_toggle;
            toggle.mouse_input(button, state, lx, ly, &mut self.ui_context);
            if toggle.take_click() {
                actions.push(AppAction::Input(pages::input::InputMessage::ToggleTapToClick));
            }
            let toggle = &mut self.app.input.scroll_toggle;
            toggle.mouse_input(button, state, lx, ly, &mut self.ui_context);
            if toggle.take_click() {
                actions.push(AppAction::Input(pages::input::InputMessage::ToggleInertialScroll));
            }
            let toggle = &mut self.app.input.natural_toggle;
            toggle.mouse_input(button, state, lx, ly, &mut self.ui_context);
            if toggle.take_click() {
                actions.push(AppAction::Input(pages::input::InputMessage::ToggleNaturalScroll));
            }
            let toggle = &mut self.app.input.pointer_toggle;
            toggle.mouse_input(button, state, lx, ly, &mut self.ui_context);
            if toggle.take_click() {
                actions.push(AppAction::Input(pages::input::InputMessage::ToggleInertialPointer));
            }
            let toggle = &mut self.app.input.trackpad_toggle;
            toggle.mouse_input(button, state, lx, ly, &mut self.ui_context);
            if toggle.take_click() {
                actions.push(AppAction::Input(pages::input::InputMessage::ToggleInertialTrackpad));
            }
            let toggle = &mut self.app.input.dwtp_toggle;
            toggle.mouse_input(button, state, lx, ly, &mut self.ui_context);
            if toggle.take_click() {
                actions.push(AppAction::Input(pages::input::InputMessage::ToggleDwtp));
            }
            let menu = &mut self.app.input.trackpoint_accel_profile_menu;
            if state == cce_ui::widget::ElementState::Pressed && !menu.hit_test(lx, ly, &self.ui_context) { menu.unfocus(); }
            if menu.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if state == cce_ui::widget::ElementState::Pressed && menu.take_change() {
                actions.push(AppAction::Input(pages::input::InputMessage::ApplyTrackpointAccelProfile(menu.selected)));
            }
            let menu = &mut self.app.input.cursor_theme_menu;
            if state == cce_ui::widget::ElementState::Pressed && !menu.hit_test(lx, ly, &self.ui_context) { menu.unfocus(); }
            if menu.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if state == cce_ui::widget::ElementState::Pressed && menu.take_change() {
                actions.push(AppAction::Input(pages::input::InputMessage::ApplyCursorTheme(menu.selected)));
            }

            let tb = &mut self.app.input.zoom_in_box;
            if state == cce_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly, &self.ui_context) { tb.unfocus(); }
            if tb.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if state == cce_ui::widget::ElementState::Pressed && tb.take_change() {
                actions.push(AppAction::Input(pages::input::InputMessage::ApplyZoomIn));
            }

            let tb = &mut self.app.input.zoom_out_box;
            if state == cce_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly, &self.ui_context) { tb.unfocus(); }
            if tb.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if state == cce_ui::widget::ElementState::Pressed && tb.take_change() {
                actions.push(AppAction::Input(pages::input::InputMessage::ApplyZoomOut));
            }
        }
        if self.app.current_page == Page::Services {
            let toggle = &mut self.app.services.notifications_enable_toggle;
            toggle.mouse_input(button, state, lx, ly, &mut self.ui_context);
            if toggle.take_click() {
                actions.push(AppAction::Services(pages::services::ServicesMessage::ToggleNotificationsEnable));
            }
            let toggle = &mut self.app.services.notifications_bell_toggle;
            toggle.mouse_input(button, state, lx, ly, &mut self.ui_context);
            if toggle.take_click() {
                actions.push(AppAction::Services(pages::services::ServicesMessage::ToggleNotificationsBell));
            }
            if state == cce_ui::widget::ElementState::Pressed {
                let lbl1 = &mut self.app.services.status_label;
                if !lbl1.hit_test(lx, ly, &self.ui_context) { lbl1.unfocus(); }
                lbl1.mouse_input(button, state, lx, ly, &mut self.ui_context);
            }
            let toggle = &mut self.app.services.status_separators_toggle;
            toggle.mouse_input(button, state, lx, ly, &mut self.ui_context);
            if toggle.take_click() {
                actions.push(AppAction::Services(pages::services::ServicesMessage::StatusToggleSeparators));
            }
            let toggle2 = &mut self.app.services.status_underline_toggle;
            toggle2.mouse_input(button, state, lx, ly, &mut self.ui_context);
            if toggle2.take_click() {
                actions.push(AppAction::Services(pages::services::ServicesMessage::StatusToggleUnderline));
            }
        }
        if self.app.current_page == Page::Display {
            let toggle = &mut self.app.display.screensaver_enable_toggle;
            toggle.mouse_input(button, state, lx, ly, &mut self.ui_context);
            if toggle.take_click() {
                actions.push(AppAction::Display(pages::display::DisplayMessage::ToggleScreensaverEnable));
            }

            let toggle = &mut self.app.display.screensaver_lock_screen_toggle;
            toggle.mouse_input(button, state, lx, ly, &mut self.ui_context);
            if toggle.take_click() {
                actions.push(AppAction::Display(pages::display::DisplayMessage::ToggleScreensaverLockScreen));
            }

            let menu = &mut self.app.display.screensaver_style_menu;
            if state == cce_ui::widget::ElementState::Pressed && !menu.hit_test(lx, ly, &self.ui_context) { menu.unfocus(); }
            if menu.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if state == cce_ui::widget::ElementState::Pressed && menu.take_change() {
                actions.push(AppAction::Display(pages::display::DisplayMessage::SetScreensaverStyle(menu.selected)));
            }
        }
        if self.app.current_page == Page::Hardware {
            let menu = &mut self.app.hardware.cpu_gov_menu;
            if state == cce_ui::widget::ElementState::Pressed && !menu.hit_test(lx, ly, &self.ui_context) { menu.unfocus(); }
            if menu.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if state == cce_ui::widget::ElementState::Pressed && menu.take_change() {
                if menu.selected == 0 {
                    actions.push(AppAction::Hardware(pages::hardware::HardwareMessage::SetCpuPerformance));
                } else {
                    actions.push(AppAction::Hardware(pages::hardware::HardwareMessage::SetCpuPowersave));
                }
            }

            let menu = &mut self.app.hardware.gpu_gov_menu;
            if state == cce_ui::widget::ElementState::Pressed && !menu.hit_test(lx, ly, &self.ui_context) { menu.unfocus(); }
            if menu.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if state == cce_ui::widget::ElementState::Pressed && menu.take_change() {
                if menu.selected == 0 {
                    actions.push(AppAction::Hardware(pages::hardware::HardwareMessage::SetGpuDefault));
                } else {
                    actions.push(AppAction::Hardware(pages::hardware::HardwareMessage::SetGpuPowersave));
                }
            }
        }

        if state == cce_ui::widget::ElementState::Pressed && self.app.current_page == Page::Audio {
            if button == cce_ui::widget::MouseButton::Left {
                for (i, slider) in self.app.audio.sink_sliders.iter_mut().enumerate() {
                    if slider.hit_test(lx, ly, &self.ui_context) {
                        slider.drag_begin(lx, ly);
                        self.audio_sink_dragging = Some(i);
                        self.needs_rebuild = true;
                    }
                }
                for (i, slider) in self.app.audio.source_sliders.iter_mut().enumerate() {
                    if slider.hit_test(lx, ly, &self.ui_context) {
                        slider.drag_begin(lx, ly);
                        self.audio_source_dragging = Some(i);
                        self.needs_rebuild = true;
                    }
                }
            }
            for (i, sb) in self.app.audio.sink_spinboxes.iter_mut().enumerate() {
                if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
                let old = sb.value;
                if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                    let id = self.app.audio.sinks[i].id;
                    actions.push(AppAction::Audio(pages::audio::AudioMessage::SinkVolume(id, sb.value as f32 / 100.0)));
                }
            }
            for (i, sb) in self.app.audio.source_spinboxes.iter_mut().enumerate() {
                if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
                let old = sb.value;
                if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                    let id = self.app.audio.sources[i].id;
                    actions.push(AppAction::Audio(pages::audio::AudioMessage::SourceVolume(id, sb.value as f32 / 100.0)));
                }
            }
        }
        if state == cce_ui::widget::ElementState::Pressed && self.app.current_page == Page::Display {
            if button == cce_ui::widget::MouseButton::Left {
                let slider = &mut self.app.display.brightness_slider;
                if slider.hit_test(lx, ly, &self.ui_context) {
                    slider.drag_begin(lx, ly);
                    self.display_brightness_dragging = true;
                    self.needs_rebuild = true;
                }
            }
            let sb = &mut self.app.display.brightness_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Display(pages::display::DisplayMessage::BrightnessSet(sb.value as u32)));
            }
            let lbl = &mut self.app.display.night_light_label;
            if !lbl.hit_test(lx, ly, &self.ui_context) { lbl.unfocus(); }
            lbl.mouse_input(button, state, lx, ly, &mut self.ui_context);

            for out in &mut self.app.display.outputs {
                let lbl = &mut out.name_label;
                if !lbl.hit_test(lx, ly, &self.ui_context) { lbl.unfocus(); }
                lbl.mouse_input(button, state, lx, ly, &mut self.ui_context);

                let lbl2 = &mut out.resolution_label;
                if !lbl2.hit_test(lx, ly, &self.ui_context) { lbl2.unfocus(); }
                lbl2.mouse_input(button, state, lx, ly, &mut self.ui_context);

                if let Some(ref mut scale_lbl) = out.scale_label {
                    if !scale_lbl.hit_test(lx, ly, &self.ui_context) { scale_lbl.unfocus(); }
                    scale_lbl.mouse_input(button, state, lx, ly, &mut self.ui_context);
                }
            }

            let sb = &mut self.app.display.screensaver_timeout_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Display(pages::display::DisplayMessage::SetScreensaverTimeout(sb.value)));
            }
        }

        if self.app.current_page == Page::Accounts {
            if self.app.accounts.editing_oauth_creds {
                let tb = &mut self.app.accounts.oauth_client_id_box;
                if state == cce_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly, &self.ui_context) { tb.unfocus(); }
                if tb.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                    self.needs_rebuild = true;
                }

                let tb = &mut self.app.accounts.oauth_client_secret_box;
                if state == cce_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly, &self.ui_context) { tb.unfocus(); }
                if tb.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                    self.needs_rebuild = true;
                }
            } else if self.app.accounts.adding_new {
                let tb = &mut self.app.accounts.email_box;
                if state == cce_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly, &self.ui_context) { tb.unfocus(); }
                if tb.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                    self.needs_rebuild = true;
                }
                if state == cce_ui::widget::ElementState::Pressed && tb.take_change() {
                    let email_val = tb.text.trim().to_lowercase();
                    if email_val.ends_with("@gmail.com") {
                        self.app.accounts.imap_box.text = "imap.gmail.com:993".to_string();
                        self.app.accounts.imap_box.edit_buffer = "imap.gmail.com:993".to_string();
                        self.app.accounts.smtp_box.text = "smtp.gmail.com:465".to_string();
                        self.app.accounts.smtp_box.edit_buffer = "smtp.gmail.com:465".to_string();
                    } else if email_val.ends_with("@icloud.com") {
                        self.app.accounts.imap_box.text = "imap.mail.me.com:993".to_string();
                        self.app.accounts.imap_box.edit_buffer = "imap.mail.me.com:993".to_string();
                        self.app.accounts.smtp_box.text = "smtp.mail.me.com:587".to_string();
                        self.app.accounts.smtp_box.edit_buffer = "smtp.mail.me.com:587".to_string();
                    } else if email_val.ends_with("@outlook.com") || email_val.ends_with("@hotmail.com") {
                        self.app.accounts.imap_box.text = "outlook.office365.com:993".to_string();
                        self.app.accounts.imap_box.edit_buffer = "outlook.office365.com:993".to_string();
                        self.app.accounts.smtp_box.text = "smtp.office365.com:587".to_string();
                        self.app.accounts.smtp_box.edit_buffer = "smtp.office365.com:587".to_string();
                    }
                }

                let tb = &mut self.app.accounts.password_box;
                if state == cce_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly, &self.ui_context) { tb.unfocus(); }
                if tb.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                    self.needs_rebuild = true;
                }

                let tb = &mut self.app.accounts.imap_box;
                if state == cce_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly, &self.ui_context) { tb.unfocus(); }
                if tb.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                    self.needs_rebuild = true;
                }

                let tb = &mut self.app.accounts.smtp_box;
                if state == cce_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly, &self.ui_context) { tb.unfocus(); }
                if tb.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                    self.needs_rebuild = true;
                }
            }
        }
        if self.app.current_page == Page::Interface {
            let tb = &mut self.app.interface.sans_box;
            if state == cce_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly, &self.ui_context) { tb.unfocus(); }
            if tb.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if state == cce_ui::widget::ElementState::Pressed && tb.take_change() {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetSans(tb.text.clone())));
            }

            let tb = &mut self.app.interface.serif_box;
            if state == cce_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly, &self.ui_context) { tb.unfocus(); }
            if tb.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if state == cce_ui::widget::ElementState::Pressed && tb.take_change() {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetSerif(tb.text.clone())));
            }

            let tb = &mut self.app.interface.mono_box;
            if state == cce_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly, &self.ui_context) { tb.unfocus(); }
            if tb.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if state == cce_ui::widget::ElementState::Pressed && tb.take_change() {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetMono(tb.text.clone())));
            }

            let menu = &mut self.app.interface.borders_menu;
            if state == cce_ui::widget::ElementState::Pressed && !menu.hit_test(lx, ly, &self.ui_context) { menu.unfocus(); }
            if menu.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if state == cce_ui::widget::ElementState::Pressed && menu.take_change() {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetBordersMenu(menu.selected)));
            }

            let tb = &mut self.app.interface.borders_box;
            if state == cce_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly, &self.ui_context) { tb.unfocus(); }
            if tb.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if state == cce_ui::widget::ElementState::Pressed && tb.take_change() {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetBorders(tb.text.clone())));
            }

            let menu = &mut self.app.interface.status_menu;
            if state == cce_ui::widget::ElementState::Pressed && !menu.hit_test(lx, ly, &self.ui_context) { menu.unfocus(); }
            if menu.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if state == cce_ui::widget::ElementState::Pressed && menu.take_change() {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetStatusMenu(menu.selected)));
            }

            let tb = &mut self.app.interface.status_box;
            if state == cce_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly, &self.ui_context) { tb.unfocus(); }
            if tb.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if state == cce_ui::widget::ElementState::Pressed && tb.take_change() {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetStatus(tb.text.clone())));
            }

            let menu = &mut self.app.interface.fuzzel_menu;
            if state == cce_ui::widget::ElementState::Pressed && !menu.hit_test(lx, ly, &self.ui_context) { menu.unfocus(); }
            if menu.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if state == cce_ui::widget::ElementState::Pressed && menu.take_change() {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetFuzzelMenu(menu.selected)));
            }

            let tb = &mut self.app.interface.fuzzel_box;
            if state == cce_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly, &self.ui_context) { tb.unfocus(); }
            if tb.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if state == cce_ui::widget::ElementState::Pressed && tb.take_change() {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetFuzzel(tb.text.clone())));
            }

            let menu = &mut self.app.interface.terminal_menu;
            if state == cce_ui::widget::ElementState::Pressed && !menu.hit_test(lx, ly, &self.ui_context) { menu.unfocus(); }
            if menu.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if state == cce_ui::widget::ElementState::Pressed && menu.take_change() {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetTerminalMenu(menu.selected)));
            }

            let tb = &mut self.app.interface.terminal_box;
            if state == cce_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly, &self.ui_context) { tb.unfocus(); }
            if tb.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if state == cce_ui::widget::ElementState::Pressed && tb.take_change() {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetTerminal(tb.text.clone())));
            }





            let fs = &mut self.app.interface.color_selector_font_selector;
            if state == cce_ui::widget::ElementState::Pressed && !fs.hit_test(lx, ly, &self.ui_context) { fs.unfocus(); }
            if fs.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if fs.take_change() {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetColorSelectorFont(fs.font_family.clone())));
            }

            let fs = &mut self.app.interface.menubar_font_selector;
            if state == cce_ui::widget::ElementState::Pressed && !fs.hit_test(lx, ly, &self.ui_context) { fs.unfocus(); }
            if fs.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if fs.take_change() {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetMenubarFont(fs.font_family.clone())));
            }

            let fs = &mut self.app.interface.section_label_font_selector;
            if state == cce_ui::widget::ElementState::Pressed && !fs.hit_test(lx, ly, &self.ui_context) { fs.unfocus(); }
            if fs.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if fs.take_change() {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetSectionLabelFont(fs.font_family.clone())));
            }

            let fs = &mut self.app.interface.nested_section_label_font_selector;
            if state == cce_ui::widget::ElementState::Pressed && !fs.hit_test(lx, ly, &self.ui_context) { fs.unfocus(); }
            if fs.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if fs.take_change() {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetNestedSectionLabelFont(fs.font_family.clone())));
            }

            let sb = &mut self.app.interface.borders_size_box;
            if state == cce_ui::widget::ElementState::Pressed && !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old_val = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if sb.value != old_val {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetBordersSize(sb.value)));
            }

            let sb = &mut self.app.interface.status_size_box;
            if state == cce_ui::widget::ElementState::Pressed && !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old_val = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if sb.value != old_val {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetStatusSize(sb.value)));
            }

            let sb = &mut self.app.interface.fuzzel_size_box;
            if state == cce_ui::widget::ElementState::Pressed && !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old_val = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if sb.value != old_val {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetFuzzelSize(sb.value)));
            }

            let sb = &mut self.app.interface.terminal_size_box;
            if state == cce_ui::widget::ElementState::Pressed && !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old_val = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if sb.value != old_val {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetTerminalSize(sb.value)));
            }




        }
        if self.app.current_page == Page::Services {
            let tb = &mut self.app.services.search_box;
            if state == cce_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly, &self.ui_context) { tb.unfocus(); }
            if tb.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            let srv = &mut self.app.services;
            if srv.list_box.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            let query = if srv.search_box.editing {
                srv.search_box.edit_buffer.to_lowercase()
            } else {
                srv.search_box.text.to_lowercase()
            };
            let matching_count = srv.services.iter()
                .filter(|s| s.is_system == (srv.active_tab == pages::services::ServiceTab::System))
                .filter(|s| s.name.to_lowercase().contains(&query) || s.description.to_lowercase().contains(&query))
                .count();
            for i in 0..matching_count.min(srv.service_items.len()) {
                if srv.service_items[i].mouse_input(button, state, lx, ly, &mut self.ui_context) {
                    self.needs_rebuild = true;
                }
            }
        }
        if self.app.current_page == Page::Packages {
            let pkgs = &mut self.app.packages;
            let tb = &mut pkgs.search_box;
            if state == cce_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly, &self.ui_context) {
                tb.unfocus();
            }
            if tb.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            match pkgs.active_tab {
                pages::packages::PackageTab::Installed => {
                    if pkgs.installed_list_box.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                        self.needs_rebuild = true;
                    }
                    let query = if pkgs.search_box.editing {
                        pkgs.search_box.edit_buffer.to_lowercase()
                    } else {
                        pkgs.search_box.text.to_lowercase()
                    };
                    let filtered: Vec<&pages::packages::PackageInfo> = pkgs.installed.iter()
                        .filter(|p| p.name.to_lowercase().contains(&query) || p.version.to_lowercase().contains(&query))
                        .collect();
                    let matching = filtered.len();
                    for i in 0..matching.min(pkgs.installed_items.len()) {
                        if pkgs.installed_items[i].mouse_input(button, state, lx, ly, &mut self.ui_context) {
                            self.needs_rebuild = true;
                        }
                        if pkgs.installed_items[i].take_click() {
                            let pkg_name = filtered[i].name.clone();
                            actions.push(AppAction::Packages(pages::packages::PackagesMessage::SelectPackage(Some(pkg_name))));
                        }
                    }
                }
                pages::packages::PackageTab::Updates => {
                    if pkgs.updates_list_box.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                        self.needs_rebuild = true;
                    }
                    let query = if pkgs.search_box.editing {
                        pkgs.search_box.edit_buffer.to_lowercase()
                    } else {
                        pkgs.search_box.text.to_lowercase()
                    };
                    let filtered: Vec<&pages::packages::UpdateInfo> = pkgs.updates.iter()
                        .filter(|p| p.name.to_lowercase().contains(&query))
                        .collect();
                    let matching = filtered.len();
                    for i in 0..matching.min(pkgs.updates_items.len()) {
                        if pkgs.updates_items[i].mouse_input(button, state, lx, ly, &mut self.ui_context) {
                            self.needs_rebuild = true;
                        }
                        if pkgs.updates_items[i].take_click() {
                            let pkg_name = filtered[i].name.clone();
                            actions.push(AppAction::Packages(pages::packages::PackagesMessage::SelectPackage(Some(pkg_name))));
                        }
                    }
                }
            }
        }
        if state == cce_ui::widget::ElementState::Pressed && self.app.current_page == Page::Hardware {
            let hw = &mut self.app.hardware;
            if hw.cpu_list_box.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
        }
        if state == cce_ui::widget::ElementState::Pressed && self.app.current_page == Page::Radios {
            let net = &mut self.app.network;
            if net.wifi_list_box.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            net.wifi_toggle.mouse_input(button, state, lx, ly, &mut self.ui_context);
            if net.wifi_toggle.take_click() {
                actions.push(AppAction::Radios(pages::network::NetworkMessage::ToggleWifi));
            }
            net.bt_toggle.mouse_input(button, state, lx, ly, &mut self.ui_context);
            if net.bt_toggle.take_click() {
                actions.push(AppAction::Radios(pages::network::NetworkMessage::ToggleBluetooth));
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

    fn propagate_widget_changes(&mut self, actions: &mut Vec<AppAction>) {
        match self.app.current_page {
            Page::Layout => {
                for (i, sb) in self.app.layout.spinboxes.iter_mut().enumerate() {
                    if sb.take_change() {
                        actions.push(AppAction::Layout(pages::layout::LayoutMessage::SetWidth(pages::layout::WidthParam::ALL[i], sb.value as u16)));
                    }
                }
                if self.app.layout.cascade_offset_spinbox.take_change() {
                    actions.push(AppAction::Layout(pages::layout::LayoutMessage::SetCascadeOffset(self.app.layout.cascade_offset_spinbox.value as u16)));
                }
                if self.app.layout.edge_gap_spinbox.take_change() {
                    actions.push(AppAction::Layout(pages::layout::LayoutMessage::SetEdgeGap(self.app.layout.edge_gap_spinbox.value as u16)));
                }
                if self.app.layout.top_gap_spinbox.take_change() {
                    actions.push(AppAction::Layout(pages::layout::LayoutMessage::SetTopGap(self.app.layout.top_gap_spinbox.value as u16)));
                }
                if self.app.layout.grid_gap_spinbox.take_change() {
                    actions.push(AppAction::Layout(pages::layout::LayoutMessage::SetGridGap(self.app.layout.grid_gap_spinbox.value as u16)));
                }
                if self.app.layout.transition_duration_spinbox.take_change() {
                    actions.push(AppAction::Layout(pages::layout::LayoutMessage::SetTransitionDuration(self.app.layout.transition_duration_spinbox.value as u16)));
                }
                if self.app.layout.status_height_spinbox.take_change() {
                    actions.push(AppAction::Layout(pages::layout::LayoutMessage::SetStatusHeight(self.app.layout.status_height_spinbox.value as u16)));
                }
                for (idx, menu) in self.app.layout.tag_layout_menus.iter_mut().enumerate() {
                    if menu.take_change() {
                        actions.push(AppAction::Layout(pages::layout::LayoutMessage::SetTagLayout(idx + 1, menu.selected)));
                    }
                }
                if self.app.layout.side_panel_behavior_menu.take_change() {
                    actions.push(AppAction::Layout(pages::layout::LayoutMessage::SetSidePanelBehavior(self.app.layout.side_panel_behavior_menu.selected)));
                }
                if self.app.layout.side_panel_position_menu.take_change() {
                    actions.push(AppAction::Layout(pages::layout::LayoutMessage::SetSidePanelPosition(self.app.layout.side_panel_position_menu.selected)));
                }
                if self.app.layout.side_panel_width_spinbox.take_change() {
                    actions.push(AppAction::Layout(pages::layout::LayoutMessage::SetSidePanelWidth(self.app.layout.side_panel_width_spinbox.value as u16)));
                }
                if self.app.layout.side_panel_border_gap_spinbox.take_change() {
                    actions.push(AppAction::Layout(pages::layout::LayoutMessage::SetSidePanelBorderGap(self.app.layout.side_panel_border_gap_spinbox.value as u16)));
                }
                if self.app.layout.side_panel_border_opacity_spinbox.take_change() {
                    actions.push(AppAction::Layout(pages::layout::LayoutMessage::SetSidePanelBorderOpacity(self.app.layout.side_panel_border_opacity_spinbox.value as u16)));
                }
            }
            Page::Interface => {
                for (i, cp) in self.app.interface.color_selectors.iter_mut().enumerate() {
                    if cp.take_change() {
                        actions.push(AppAction::Interface(match i {
                            0 => pages::interface::InterfaceMessage::SetPageLowColor(cp.color),
                            1 => pages::interface::InterfaceMessage::SetHighColor(cp.color),
                            2 => pages::interface::InterfaceMessage::SetVisualGuidesColor(cp.color),
                            3 => pages::interface::InterfaceMessage::SetDisabledColor(cp.color),
                            4 => pages::interface::InterfaceMessage::SetSeparatorColor(cp.color),
                            5 => pages::interface::InterfaceMessage::SetSliderTrackColor(cp.color),
                            6 => pages::interface::InterfaceMessage::SetColorBordersColor(cp.color),
                            7 => pages::interface::InterfaceMessage::SetLowColor(cp.color),
                            8 => pages::interface::InterfaceMessage::SetNormalColor(cp.color),
                            9 => pages::interface::InterfaceMessage::SetPaginatorSidebarColor(cp.color),
                            10 => pages::interface::InterfaceMessage::SetPrimaryHighlightColor(cp.color),
                            11 => pages::interface::InterfaceMessage::SetMenubarTabLabelColor(cp.color),
                            12 => pages::interface::InterfaceMessage::SetToggleEnabledColor(cp.color),
                            13 => pages::interface::InterfaceMessage::SetToggleDisabledColor(cp.color),
                            14 => pages::interface::InterfaceMessage::SetScrollingListBgColor(cp.color),
                            15 => pages::interface::InterfaceMessage::SetBreadcrumbBgColor(cp.color),
                            16 => pages::interface::InterfaceMessage::SetPopoverBgColor(cp.color),
                            17 => pages::interface::InterfaceMessage::SetNotificationBgColor(cp.color),
                            _ => pages::interface::InterfaceMessage::SetLowColor(cp.color),
                        }));
                    }
                }
                if self.app.interface.notification_opacity_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetNotificationOpacity(self.app.interface.notification_opacity_spinbox.value as f32 / 100.0)));
                }
                if self.app.interface.menubar_opacity_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetMenubarOpacity(self.app.interface.menubar_opacity_spinbox.value as f32 / 100.0)));
                }
                if self.app.interface.tab_margin_spinbox_x.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetTabMarginX(self.app.interface.tab_margin_spinbox_x.value as u16)));
                }
                if self.app.interface.tab_margin_spinbox_y.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetTabMarginY(self.app.interface.tab_margin_spinbox_y.value as u16)));
                }
                if self.app.interface.button_padding_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetButtonPadding(self.app.interface.button_padding_spinbox.value as u16)));
                }
                if self.app.interface.section_padding_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetSectionPadding(self.app.interface.section_padding_spinbox.value as u16)));
                }
                if self.app.interface.label_alignment_menu.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetNestedSectionLabelAlignment(self.app.interface.label_alignment_menu.selected)));
                }
                if self.app.interface.label_offset_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetNestedSectionLabelOffset(self.app.interface.label_offset_spinbox.value as i16)));
                }
                if self.app.interface.label_margin_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetLabelMargin(self.app.interface.label_margin_spinbox.value as u16)));
                }
                if self.app.interface.plate_padding_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetPlatePadding(self.app.interface.plate_padding_spinbox.value as u16)));
                }
                if self.app.interface.graph_show_grid_toggle.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetGraphShowGrid(self.app.interface.graph_show_grid_toggle.toggled())));
                }
                if self.app.interface.graph_snap_enabled_toggle.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetGraphSnapEnabled(self.app.interface.graph_snap_enabled_toggle.toggled())));
                }
                if self.app.interface.graph_uniform_background_toggle.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetGraphUniformBackground(self.app.interface.graph_uniform_background_toggle.toggled())));
                }
                if self.app.interface.graph_cell_opacity_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetGraphCellOpacity(self.app.interface.graph_cell_opacity_spinbox.value as f32 / 100.0)));
                }
                if self.app.interface.graph_gap_opacity_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetGraphGapOpacity(self.app.interface.graph_gap_opacity_spinbox.value as f32 / 100.0)));
                }
                if self.app.interface.page_margin_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetPageMargin(self.app.interface.page_margin_spinbox.value as u16)));
                }
                if self.app.interface.grid_min_col_width_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetGridMinColWidth(self.app.interface.grid_min_col_width_spinbox.value as u16)));
                }
                if self.app.interface.spinbox_height_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetSpinboxHeight(self.app.interface.spinbox_height_spinbox.value as u16)));
                }
                if self.app.interface.spinbox_corner_radius_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetSpinboxCornerRadius(self.app.interface.spinbox_corner_radius_spinbox.value as u16)));
                }
                if self.app.interface.toggle_height_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetToggleHeight(self.app.interface.toggle_height_spinbox.value as u16)));
                }
                if self.app.interface.toggle_corner_radius_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetToggleCornerRadius(self.app.interface.toggle_corner_radius_spinbox.value as u16)));
                }
                if self.app.interface.plate_opacity_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetPlateOpacity(self.app.interface.plate_opacity_spinbox.value as f32 / 100.0)));
                }
                if self.app.interface.plate_corner_radius_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetPlateCornerRadius(self.app.interface.plate_corner_radius_spinbox.value as u16)));
                }
                if self.app.interface.page_opacity_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetPageOpacity(self.app.interface.page_opacity_spinbox.value as f32 / 100.0)));
                }
                if self.app.interface.layer_opacity_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetLayerOpacity(self.app.interface.layer_opacity_spinbox.value as f32 / 100.0)));
                }


                if self.app.interface.color_selector_height_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetColorSelectorHeight(self.app.interface.color_selector_height_spinbox.value as u16)));
                }
                if self.app.interface.color_selector_corner_radius_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetColorSelectorCornerRadius(self.app.interface.color_selector_corner_radius_spinbox.value as u16)));
                }
                if self.app.interface.color_selector_preview_corner_radius_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetColorSelectorPreviewCornerRadius(self.app.interface.color_selector_preview_corner_radius_spinbox.value as u16)));
                }
                if self.app.interface.color_selector_preview_margin_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetColorSelectorPreviewMargin(self.app.interface.color_selector_preview_margin_spinbox.value as u16)));
                }
                if self.app.interface.textbox_height_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetTextboxHeight(self.app.interface.textbox_height_spinbox.value as u16)));
                }
                if self.app.interface.textbox_corner_radius_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetTextboxCornerRadius(self.app.interface.textbox_corner_radius_spinbox.value as u16)));
                }
                if self.app.interface.slider_height_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetSliderHeight(self.app.interface.slider_height_spinbox.value as u16)));
                }
                if self.app.interface.font_selector_height_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetFontSelectorHeight(self.app.interface.font_selector_height_spinbox.value as u16)));
                }
                if self.app.interface.font_selector_corner_radius_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetFontSelectorCornerRadius(self.app.interface.font_selector_corner_radius_spinbox.value as u16)));
                }
                if self.app.interface.dropdown_height_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetDropdownHeight(self.app.interface.dropdown_height_spinbox.value as u16)));
                }
                if self.app.interface.dropdown_corner_radius_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetDropdownCornerRadius(self.app.interface.dropdown_corner_radius_spinbox.value as u16)));
                }
                if self.app.interface.button_corner_radius_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetButtonCornerRadius(self.app.interface.button_corner_radius_spinbox.value as u16)));
                }
                if self.app.interface.sans_box.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetSans(self.app.interface.sans_box.text.clone())));
                }
                if self.app.interface.serif_box.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetSerif(self.app.interface.serif_box.text.clone())));
                }
                if self.app.interface.mono_box.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetMono(self.app.interface.mono_box.text.clone())));
                }
                if self.app.interface.borders_menu.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetBordersMenu(self.app.interface.borders_menu.selected)));
                }
                if self.app.interface.borders_box.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetBorders(self.app.interface.borders_box.text.clone())));
                }
                if self.app.interface.status_menu.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetStatusMenu(self.app.interface.status_menu.selected)));
                }
                if self.app.interface.status_box.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetStatus(self.app.interface.status_box.text.clone())));
                }
                if self.app.interface.fuzzel_menu.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetFuzzelMenu(self.app.interface.fuzzel_menu.selected)));
                }
                if self.app.interface.fuzzel_box.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetFuzzel(self.app.interface.fuzzel_box.text.clone())));
                }
                if self.app.interface.terminal_menu.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetTerminalMenu(self.app.interface.terminal_menu.selected)));
                }
                if self.app.interface.terminal_box.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetTerminal(self.app.interface.terminal_box.text.clone())));
                }
                if self.app.interface.color_selector_font_selector.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetColorSelectorFont(self.app.interface.color_selector_font_selector.font_family.clone())));
                }
                if self.app.interface.menubar_font_selector.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetMenubarFont(self.app.interface.menubar_font_selector.font_family.clone())));
                }
                if self.app.interface.section_label_font_selector.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetSectionLabelFont(self.app.interface.section_label_font_selector.font_family.clone())));
                }
                if self.app.interface.nested_section_label_font_selector.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetNestedSectionLabelFont(self.app.interface.nested_section_label_font_selector.font_family.clone())));
                }
                if self.app.interface.borders_size_box.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetBordersSize(self.app.interface.borders_size_box.value)));
                }
                if self.app.interface.status_size_box.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetStatusSize(self.app.interface.status_size_box.value)));
                }
            }
            Page::Input => {
                if self.app.input.rate_spinbox.take_change() {
                    actions.push(AppAction::Input(pages::input::InputMessage::ApplyRepeat));
                }
                if self.app.input.delay_spinbox.take_change() {
                    actions.push(AppAction::Input(pages::input::InputMessage::ApplyRepeat));
                }
                if self.app.input.scroll_friction_spinbox.take_change() {
                    actions.push(AppAction::Input(pages::input::InputMessage::ApplyScrollFriction));
                }
                if self.app.input.scroll_speed_spinbox.take_change() {
                    actions.push(AppAction::Input(pages::input::InputMessage::ApplyScrollSpeed));
                }
                if self.app.input.pointer_friction_spinbox.take_change() {
                    actions.push(AppAction::Input(pages::input::InputMessage::ApplyPointerFriction));
                }
                if self.app.input.trackpad_friction_spinbox.take_change() {
                    actions.push(AppAction::Input(pages::input::InputMessage::ApplyTrackpadFriction));
                }
                if self.app.input.trackpoint_accel_speed_spinbox.take_change() {
                    actions.push(AppAction::Input(pages::input::InputMessage::ApplyTrackpointAccelSpeed));
                }
                if self.app.input.cursor_size_spinbox.take_change() {
                    actions.push(AppAction::Input(pages::input::InputMessage::ApplyCursorSize));
                }
                if self.app.input.tap_toggle.take_change() {
                    actions.push(AppAction::Input(pages::input::InputMessage::ToggleTapToClick));
                }
                if self.app.input.scroll_toggle.take_change() {
                    actions.push(AppAction::Input(pages::input::InputMessage::ToggleInertialScroll));
                }
                if self.app.input.natural_toggle.take_change() {
                    actions.push(AppAction::Input(pages::input::InputMessage::ToggleNaturalScroll));
                }
                if self.app.input.pointer_toggle.take_change() {
                    actions.push(AppAction::Input(pages::input::InputMessage::ToggleInertialPointer));
                }
                if self.app.input.trackpad_toggle.take_change() {
                    actions.push(AppAction::Input(pages::input::InputMessage::ToggleInertialTrackpad));
                }
                if self.app.input.dwtp_toggle.take_change() {
                    actions.push(AppAction::Input(pages::input::InputMessage::ToggleDwtp));
                }
                if self.app.input.trackpoint_accel_profile_menu.take_change() {
                    actions.push(AppAction::Input(pages::input::InputMessage::ApplyTrackpointAccelProfile(self.app.input.trackpoint_accel_profile_menu.selected)));
                }
                if self.app.input.cursor_theme_menu.take_change() {
                    actions.push(AppAction::Input(pages::input::InputMessage::ApplyCursorTheme(self.app.input.cursor_theme_menu.selected)));
                }
                if self.app.input.zoom_in_box.take_change() {
                    actions.push(AppAction::Input(pages::input::InputMessage::ApplyZoomIn));
                }
                if self.app.input.zoom_out_box.take_change() {
                    actions.push(AppAction::Input(pages::input::InputMessage::ApplyZoomOut));
                }
            }
            Page::Services => {
                if self.app.services.notifications_duration_spinbox.take_change() {
                    actions.push(AppAction::Services(pages::services::ServicesMessage::SetNotificationsDuration(self.app.services.notifications_duration_spinbox.value)));
                }
                if self.app.services.status_padding_spinbox.take_change() {
                    actions.push(AppAction::Services(pages::services::ServicesMessage::StatusSetPadding(self.app.services.status_padding_spinbox.value as u16)));
                }
                if self.app.services.notifications_enable_toggle.take_change() {
                    actions.push(AppAction::Services(pages::services::ServicesMessage::ToggleNotificationsEnable));
                }
                if self.app.services.notifications_bell_toggle.take_change() {
                    actions.push(AppAction::Services(pages::services::ServicesMessage::ToggleNotificationsBell));
                }
                if self.app.services.status_separators_toggle.take_change() {
                    actions.push(AppAction::Services(pages::services::ServicesMessage::StatusToggleSeparators));
                }
                if self.app.services.status_underline_toggle.take_change() {
                    actions.push(AppAction::Services(pages::services::ServicesMessage::StatusToggleUnderline));
                }
            }
            Page::Display => {
                if self.app.display.screensaver_enable_toggle.take_change() {
                    actions.push(AppAction::Display(pages::display::DisplayMessage::ToggleScreensaverEnable));
                }
                if self.app.display.screensaver_lock_screen_toggle.take_change() {
                    actions.push(AppAction::Display(pages::display::DisplayMessage::ToggleScreensaverLockScreen));
                }
                if self.app.display.screensaver_style_menu.take_change() {
                    actions.push(AppAction::Display(pages::display::DisplayMessage::SetScreensaverStyle(self.app.display.screensaver_style_menu.selected)));
                }
                if self.app.display.brightness_spinbox.take_change() {
                    actions.push(AppAction::Display(pages::display::DisplayMessage::BrightnessSet(self.app.display.brightness_spinbox.value as u32)));
                }
                if self.app.display.brightness_slider.take_change() {
                    actions.push(AppAction::Display(pages::display::DisplayMessage::BrightnessSet((self.app.display.brightness_slider.value() * 100.0).round() as u32)));
                }
                if self.app.display.screensaver_timeout_spinbox.take_change() {
                    actions.push(AppAction::Display(pages::display::DisplayMessage::SetScreensaverTimeout(self.app.display.screensaver_timeout_spinbox.value)));
                }
            }
            Page::Hardware => {
                if self.app.hardware.cpu_gov_menu.take_change() {
                    if self.app.hardware.cpu_gov_menu.selected == 0 {
                        actions.push(AppAction::Hardware(pages::hardware::HardwareMessage::SetCpuPerformance));
                    } else {
                        actions.push(AppAction::Hardware(pages::hardware::HardwareMessage::SetCpuPowersave));
                    }
                }
                if self.app.hardware.gpu_gov_menu.take_change() {
                    if self.app.hardware.gpu_gov_menu.selected == 0 {
                        actions.push(AppAction::Hardware(pages::hardware::HardwareMessage::SetGpuDefault));
                    } else {
                        actions.push(AppAction::Hardware(pages::hardware::HardwareMessage::SetGpuPowersave));
                    }
                }
            }
            Page::Accounts => {
                if self.app.accounts.adding_new && self.app.accounts.email_box.take_change() {
                    let email_val = self.app.accounts.email_box.text.trim().to_lowercase();
                    if email_val.ends_with("@gmail.com") {
                        self.app.accounts.imap_box.text = "imap.gmail.com:993".to_string();
                        self.app.accounts.imap_box.edit_buffer = "imap.gmail.com:993".to_string();
                        self.app.accounts.smtp_box.text = "smtp.gmail.com:465".to_string();
                        self.app.accounts.smtp_box.edit_buffer = "smtp.gmail.com:465".to_string();
                    } else if email_val.ends_with("@icloud.com") {
                        self.app.accounts.imap_box.text = "imap.mail.me.com:993".to_string();
                        self.app.accounts.imap_box.edit_buffer = "imap.mail.me.com:993".to_string();
                        self.app.accounts.smtp_box.text = "smtp.mail.me.com:587".to_string();
                        self.app.accounts.smtp_box.edit_buffer = "smtp.mail.me.com:587".to_string();
                    } else if email_val.ends_with("@outlook.com") || email_val.ends_with("@hotmail.com") {
                        self.app.accounts.imap_box.text = "outlook.office365.com:993".to_string();
                        self.app.accounts.imap_box.edit_buffer = "outlook.office365.com:993".to_string();
                        self.app.accounts.smtp_box.text = "smtp.office365.com:587".to_string();
                        self.app.accounts.smtp_box.edit_buffer = "smtp.office365.com:587".to_string();
                    }
                }
            }
            Page::Audio => {
                for (i, sb) in self.app.audio.sink_spinboxes.iter_mut().enumerate() {
                    if sb.take_change() {
                        let id = self.app.audio.sinks[i].id;
                        actions.push(AppAction::Audio(pages::audio::AudioMessage::SinkVolume(id, sb.value as f32 / 100.0)));
                    }
                }
                for (i, sb) in self.app.audio.source_spinboxes.iter_mut().enumerate() {
                    if sb.take_change() {
                        let id = self.app.audio.sources[i].id;
                        actions.push(AppAction::Audio(pages::audio::AudioMessage::SourceVolume(id, sb.value as f32 / 100.0)));
                    }
                }
                for (i, slider) in self.app.audio.sink_sliders.iter_mut().enumerate() {
                    if slider.take_change() {
                        let id = self.app.audio.sinks[i].id;
                        actions.push(AppAction::Audio(pages::audio::AudioMessage::SinkVolume(id, slider.value() as f32 / 100.0)));
                    }
                }
                for (i, slider) in self.app.audio.source_sliders.iter_mut().enumerate() {
                    if slider.take_change() {
                        let id = self.app.audio.sources[i].id;
                        actions.push(AppAction::Audio(pages::audio::AudioMessage::SourceVolume(id, slider.value() as f32 / 100.0)));
                    }
                }
            }
            Page::Radios => {
                let net = &mut self.app.network;
                if net.wifi_toggle.take_change() {
                    actions.push(AppAction::Radios(pages::network::NetworkMessage::ToggleWifi));
                }
                if net.bt_toggle.take_change() {
                    actions.push(AppAction::Radios(pages::network::NetworkMessage::ToggleBluetooth));
                }
            }
            _ => {}
        }
    }

    fn handle_mouse_wheel_internal(&mut self, delta: &cce_ui::widget::MouseScrollDelta, px: f32, py: f32) -> bool {
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("/tmp/cce-scroll-debug.log") {
            use std::io::Write;
            let _ = writeln!(file, "handle_mouse_wheel_internal: px={}, py={}, delta={:?}, sidebar_w={}", px, py, delta, self.sidebar_width);
        }
        let s = 1.0f32;
        if px >= self.sidebar_width * s {
            let lx = px / s;
            let ly = py / s + self.scroll_y;
            
            if self.app.current_page == Page::Input {
                let input = &self.app.input;
                if input.is_over_trackpad(lx, ly, &self.ui_context) {
                    return true;
                }
            }
            if self.app.current_page == Page::Audio {
                let mut actions = Vec::new();
                for (i, sb) in self.app.audio.sink_spinboxes.iter_mut().enumerate() {
                    let old = sb.value;
                    if sb.mouse_wheel(delta, lx, ly, &mut self.ui_context) {
                        if sb.value != old {
                            let id = self.app.audio.sinks[i].id;
                            actions.push(AppAction::Audio(pages::audio::AudioMessage::SinkVolume(id, sb.value as f32 / 100.0)));
                        }
                    }
                }
                for (i, sb) in self.app.audio.source_spinboxes.iter_mut().enumerate() {
                    let old = sb.value;
                    if sb.mouse_wheel(delta, lx, ly, &mut self.ui_context) {
                        if sb.value != old {
                            let id = self.app.audio.sources[i].id;
                            actions.push(AppAction::Audio(pages::audio::AudioMessage::SourceVolume(id, sb.value as f32 / 100.0)));
                        }
                    }
                }
                for (i, slider) in self.app.audio.sink_sliders.iter_mut().enumerate() {
                    let old = slider.value();
                    if slider.mouse_wheel(delta, lx, ly, &mut self.ui_context) {
                        let new_val = slider.value();
                        if new_val != old {
                            let id = self.app.audio.sinks[i].id;
                            if i < self.app.audio.sink_spinboxes.len() {
                                self.app.audio.sink_spinboxes[i].value = (new_val * 100.0).round() as i32;
                            }
                            actions.push(AppAction::Audio(pages::audio::AudioMessage::SinkVolume(id, new_val)));
                        }
                    }
                }
                for (i, slider) in self.app.audio.source_sliders.iter_mut().enumerate() {
                    let old = slider.value();
                    if slider.mouse_wheel(delta, lx, ly, &mut self.ui_context) {
                        let new_val = slider.value();
                        if new_val != old {
                            let id = self.app.audio.sources[i].id;
                            if i < self.app.audio.source_spinboxes.len() {
                                self.app.audio.source_spinboxes[i].value = (new_val * 100.0).round() as i32;
                            }
                            actions.push(AppAction::Audio(pages::audio::AudioMessage::SourceVolume(id, new_val)));
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
                let mut actions = Vec::new();
                let slider = &mut self.app.display.brightness_slider;
                let old_val = slider.value();
                if slider.mouse_wheel(delta, lx, ly, &mut self.ui_context) {
                    let new_val = slider.value();
                    if new_val != old_val {
                        let pct = (new_val * 100.0).round() as u32;
                        self.app.display.brightness_spinbox.value = pct as i32;
                        actions.push(AppAction::Display(pages::display::DisplayMessage::BrightnessSet(pct)));
                    }
                }
                let sb = &mut self.app.display.brightness_spinbox;
                let old = sb.value;
                if sb.mouse_wheel(delta, lx, ly, &mut self.ui_context) {
                    if sb.value != old {
                        actions.push(AppAction::Display(pages::display::DisplayMessage::BrightnessSet(sb.value as u32)));
                    }
                }
                let sb2 = &mut self.app.display.screensaver_timeout_spinbox;
                let old2 = sb2.value;
                if sb2.mouse_wheel(delta, lx, ly, &mut self.ui_context) {
                    if sb2.value != old2 {
                        actions.push(AppAction::Display(pages::display::DisplayMessage::SetScreensaverTimeout(sb2.value)));
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


            if self.app.current_page == Page::Interface {
                let mut actions = Vec::new();
                let sb = &mut self.app.interface.graph_cell_opacity_spinbox;
                let old = sb.value;
                if sb.mouse_wheel(delta, lx, ly, &mut self.ui_context) {
                    if sb.value != old {
                        actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetGraphCellOpacity(sb.value as f32 / 100.0)));
                    }
                }
                let sb2 = &mut self.app.interface.graph_gap_opacity_spinbox;
                let old2 = sb2.value;
                if sb2.mouse_wheel(delta, lx, ly, &mut self.ui_context) {
                    if sb2.value != old2 {
                        actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetGraphGapOpacity(sb2.value as f32 / 100.0)));
                    }
                }
                let sb3 = &mut self.app.interface.window_opacity_spinbox;
                let old3 = sb3.value;
                if sb3.mouse_wheel(delta, lx, ly, &mut self.ui_context) {
                    if sb3.value != old3 {
                        actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetWindowOpacity(sb3.value as f32 / 100.0)));
                    }
                }
                let sb4 = &mut self.app.interface.window_corner_radius_spinbox;
                let old4 = sb4.value;
                if sb4.mouse_wheel(delta, lx, ly, &mut self.ui_context) {
                    if sb4.value != old4 {
                        actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetWindowCornerRadius(sb4.value as u16)));
                    }
                }
                let sb5 = &mut self.app.interface.plate_opacity_spinbox;
                let old5 = sb5.value;
                if sb5.mouse_wheel(delta, lx, ly, &mut self.ui_context) {
                    if sb5.value != old5 {
                        actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetPlateOpacity(sb5.value as f32 / 100.0)));
                    }
                }
                let sb6 = &mut self.app.interface.plate_corner_radius_spinbox;
                let old6 = sb6.value;
                if sb6.mouse_wheel(delta, lx, ly, &mut self.ui_context) {
                    if sb6.value != old6 {
                        actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetPlateCornerRadius(sb6.value as u16)));
                    }
                }
                let sb7 = &mut self.app.interface.page_opacity_spinbox;
                let old7 = sb7.value;
                if sb7.mouse_wheel(delta, lx, ly, &mut self.ui_context) {
                    if sb7.value != old7 {
                        actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetPageOpacity(sb7.value as f32 / 100.0)));
                    }
                }
                let sb8 = &mut self.app.interface.layer_opacity_spinbox;
                let old8 = sb8.value;
                if sb8.mouse_wheel(delta, lx, ly, &mut self.ui_context) {
                    if sb8.value != old8 {
                        actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetLayerOpacity(sb8.value as f32 / 100.0)));
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

            if self.app.current_page == Page::Services {
                let srv = &mut self.app.services;
                if srv.list_box.mouse_wheel(delta, lx, ly, &mut self.ui_context) {
                    self.needs_rebuild = true;
                    return true;
                }
            }
            if self.app.current_page == Page::Packages {
                let pkgs = &mut self.app.packages;
                match pkgs.active_tab {
                    pages::packages::PackageTab::Installed => {
                        if pkgs.installed_list_box.mouse_wheel(delta, lx, ly, &mut self.ui_context) {
                            self.needs_rebuild = true;
                            return true;
                        }
                    }
                    pages::packages::PackageTab::Updates => {
                        if pkgs.updates_list_box.mouse_wheel(delta, lx, ly, &mut self.ui_context) {
                            self.needs_rebuild = true;
                            return true;
                        }
                    }
                }
            }
            if self.app.current_page == Page::Hardware {
                let hw = &mut self.app.hardware;
                if hw.cpu_list_box.mouse_wheel(delta, lx, ly, &mut self.ui_context) {
                    self.needs_rebuild = true;
                    return true;
                }
            }
            if self.app.current_page == Page::Radios {
                let net = &mut self.app.network;
                if net.wifi_list_box.mouse_wheel(delta, lx, ly, &mut self.ui_context) {
                    self.needs_rebuild = true;
                    return true;
                }
            }

            let scroll_speed = 24.0;
            let dy = match delta {
                cce_ui::widget::MouseScrollDelta::LineDelta(_, y) => -y * scroll_speed,
                cce_ui::widget::MouseScrollDelta::PixelDelta(pos) => -pos.y as f32,
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
            if self.menubar.mouse_wheel(delta, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
                return true;
            }
        }
        false
    }

    fn get_page_root_widget(&mut self) -> Option<*mut (dyn cce_ui::widget::Element + 'static)> {
        let page_idx = Page::ALL.iter().position(|&p| p == self.app.current_page).unwrap_or(0);
        let ptr = &mut self.plates[page_idx] as &mut dyn cce_ui::widget::Element as *mut dyn cce_ui::widget::Element;
        let static_ptr = unsafe {
            std::mem::transmute::<*mut dyn cce_ui::widget::Element, *mut (dyn cce_ui::widget::Element + 'static)>(ptr)
        };
        Some(static_ptr)
    }

    fn handle_key_input_internal(&mut self, event: &cce_ui::widget::KeyEvent) -> bool {
        if cce_ui::widget::context_menu::is_visible() {
            if event.state == cce_ui::widget::ElementState::Pressed
                && event.logical_key == cce_ui::widget::Key::Named(cce_ui::widget::NamedKey::Escape)
            {
                cce_ui::widget::context_menu::hide();
                self.needs_rebuild = true;
                return true;
            }
        }

        if event.state == cce_ui::widget::ElementState::Pressed && !event.repeat {
            let is_nav_key = match (&event.logical_key, event.ctrl) {
                (cce_ui::widget::Key::Character(c), true) if c == "j" || c == "J" || c == "k" || c == "K" || c == "u" || c == "U" || c == "i" || c == "I" => true,
                _ => false,
            };
            if is_nav_key {
                if cce_ui::widget::focus::has_focus() {
                    if cce_ui::widget::focus::navigate_focus(&event.logical_key, event.ctrl) {
                        self.needs_rebuild = true;
                        return true;
                    }
                } else {
                    if let Some(root_ptr) = self.get_page_root_widget() {
                        unsafe {
                            let root_ref = &mut *root_ptr;
                            cce_ui::widget::focus::set_focused(root_ref);
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
                if sb.keyboard_input(event, &mut self.ui_context) {
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
            if sb.keyboard_input(event, &mut self.ui_context) {
                if sb.value != old {
                    actions.push(AppAction::Layout(
                        pages::layout::LayoutMessage::SetCascadeOffset(sb.value as u16)
                    ));
                }
                changed = true;
            }
            let sb = &mut self.app.layout.edge_gap_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                if sb.value != old {
                    actions.push(AppAction::Layout(
                        pages::layout::LayoutMessage::SetEdgeGap(sb.value as u16)
                    ));
                }
                changed = true;
            }
            let sb = &mut self.app.layout.top_gap_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                if sb.value != old {
                    actions.push(AppAction::Layout(
                        pages::layout::LayoutMessage::SetTopGap(sb.value as u16)
                    ));
                }
                changed = true;
            }
            let sb = &mut self.app.layout.grid_gap_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                if sb.value != old {
                    actions.push(AppAction::Layout(
                        pages::layout::LayoutMessage::SetGridGap(sb.value as u16)
                    ));
                }
                changed = true;
            }
            let sb = &mut self.app.layout.transition_duration_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                if sb.value != old {
                    actions.push(AppAction::Layout(
                        pages::layout::LayoutMessage::SetTransitionDuration(sb.value as u16)
                    ));
                }
                changed = true;
            }
            let sb = &mut self.app.layout.status_height_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                if sb.value != old {
                    actions.push(AppAction::Layout(
                        pages::layout::LayoutMessage::SetStatusHeight(sb.value as u16)
                    ));
                }
                changed = true;
            }
            let (menu_changed, old_selected, new_selected) = {
                let menu = &mut self.app.layout.side_panel_behavior_menu;
                let old = menu.selected;
                let changed = menu.keyboard_input(event, &mut self.ui_context);
                (changed, old, menu.selected)
            };
            if menu_changed {
                if new_selected != old_selected {
                    actions.push(AppAction::Layout(pages::layout::LayoutMessage::SetSidePanelBehavior(new_selected)));
                }
                changed = true;
            }
            let (menu_changed, old_selected, new_selected) = {
                let menu = &mut self.app.layout.side_panel_position_menu;
                let old = menu.selected;
                let changed = menu.keyboard_input(event, &mut self.ui_context);
                (changed, old, menu.selected)
            };
            if menu_changed {
                if new_selected != old_selected {
                    actions.push(AppAction::Layout(pages::layout::LayoutMessage::SetSidePanelPosition(new_selected)));
                }
                changed = true;
            }
            let sb = &mut self.app.layout.side_panel_width_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                if sb.value != old {
                    actions.push(AppAction::Layout(
                        pages::layout::LayoutMessage::SetSidePanelWidth(sb.value as u16)
                    ));
                }
                changed = true;
            }
            let sb = &mut self.app.layout.side_panel_border_gap_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                if sb.value != old {
                    actions.push(AppAction::Layout(
                        pages::layout::LayoutMessage::SetSidePanelBorderGap(sb.value as u16)
                    ));
                }
                changed = true;
            }
            let sb = &mut self.app.layout.side_panel_border_opacity_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                if sb.value != old {
                    actions.push(AppAction::Layout(
                        pages::layout::LayoutMessage::SetSidePanelBorderOpacity(sb.value as u16)
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
        if self.app.current_page == Page::Interface {
            let mc = &mut self.app.interface.custom_multicontrol;
            if mc.keyboard_input(event, &mut self.ui_context) {
                self.needs_rebuild = true;
                return true;
            }
            let mut changed = false;
            let mut actions = Vec::new();
            for (i, cp) in self.app.interface.color_selectors.iter_mut().enumerate() {
                let old = cp.color;
                if cp.keyboard_input(event, &mut self.ui_context) {
                    if cp.color != old {
                        actions.push(AppAction::Interface(match i {
                            0 => pages::interface::InterfaceMessage::SetPageLowColor(cp.color),
                            1 => pages::interface::InterfaceMessage::SetHighColor(cp.color),
                            2 => pages::interface::InterfaceMessage::SetVisualGuidesColor(cp.color),
                            3 => pages::interface::InterfaceMessage::SetDisabledColor(cp.color),
                            4 => pages::interface::InterfaceMessage::SetSeparatorColor(cp.color),
                            5 => pages::interface::InterfaceMessage::SetSliderTrackColor(cp.color),
                            6 => pages::interface::InterfaceMessage::SetColorBordersColor(cp.color),
                            7 => pages::interface::InterfaceMessage::SetLowColor(cp.color),
                            8 => pages::interface::InterfaceMessage::SetNormalColor(cp.color),
                            9 => pages::interface::InterfaceMessage::SetPaginatorSidebarColor(cp.color),
                            10 => pages::interface::InterfaceMessage::SetPrimaryHighlightColor(cp.color),
                            11 => pages::interface::InterfaceMessage::SetMenubarTabLabelColor(cp.color),
                            12 => pages::interface::InterfaceMessage::SetToggleEnabledColor(cp.color),
                            13 => pages::interface::InterfaceMessage::SetToggleDisabledColor(cp.color),
                            14 => pages::interface::InterfaceMessage::SetScrollingListBgColor(cp.color),
                            15 => pages::interface::InterfaceMessage::SetBreadcrumbBgColor(cp.color),
                            16 => pages::interface::InterfaceMessage::SetPopoverBgColor(cp.color),
                            17 => pages::interface::InterfaceMessage::SetNotificationBgColor(cp.color),
                            18 => pages::interface::InterfaceMessage::SetWindowColor(cp.color),
                            _ => pages::interface::InterfaceMessage::SetLowColor(cp.color),
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
            let sb = &mut self.app.interface.notification_opacity_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetNotificationOpacity(new_val as f32 / 100.0)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.window_opacity_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetWindowOpacity(new_val as f32 / 100.0)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.window_corner_radius_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetWindowCornerRadius(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.menubar_opacity_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetMenubarOpacity(new_val as f32 / 100.0)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.graph_gap_width_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetGraphGapWidth(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.tab_margin_spinbox_x;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetTabMarginX(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.tab_margin_spinbox_y;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetTabMarginY(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.button_padding_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetButtonPadding(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.section_padding_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetSectionPadding(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let (menu_changed, old_selected, new_selected) = {
                let menu = &mut self.app.interface.label_alignment_menu;
                let old = menu.selected;
                let changed = menu.keyboard_input(event, &mut self.ui_context);
                (changed, old, menu.selected)
            };
            if menu_changed {
                if new_selected != old_selected {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetNestedSectionLabelAlignment(new_selected)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.label_offset_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetNestedSectionLabelOffset(new_val as i16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.label_margin_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetLabelMargin(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.plate_padding_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetPlatePadding(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.page_margin_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetPageMargin(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.grid_min_col_width_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetGridMinColWidth(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.spinbox_height_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetSpinboxHeight(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.spinbox_corner_radius_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetSpinboxCornerRadius(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.toggle_height_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetToggleHeight(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.toggle_corner_radius_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetToggleCornerRadius(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.plate_opacity_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetPlateOpacity(new_val as f32 / 100.0)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.plate_corner_radius_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetPlateCornerRadius(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.page_opacity_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetPageOpacity(new_val as f32 / 100.0)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.layer_opacity_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetLayerOpacity(new_val as f32 / 100.0)));
                }
                self.needs_rebuild = true;
                return true;
            }


            let sb = &mut self.app.interface.color_selector_height_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetColorSelectorHeight(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.color_selector_corner_radius_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetColorSelectorCornerRadius(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.color_selector_preview_corner_radius_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetColorSelectorPreviewCornerRadius(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.color_selector_preview_margin_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetColorSelectorPreviewMargin(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.textbox_height_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetTextboxHeight(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.textbox_corner_radius_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetTextboxCornerRadius(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.slider_height_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetSliderHeight(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.font_selector_height_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetFontSelectorHeight(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.font_selector_corner_radius_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetFontSelectorCornerRadius(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.dropdown_height_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetDropdownHeight(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.dropdown_corner_radius_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetDropdownCornerRadius(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.button_corner_radius_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetButtonCornerRadius(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }



            let mut actions = Vec::new();
            let mut consumed = false;
            
            let tf = &mut self.app.interface;
            let tb = &mut tf.sans_box;
            if tb.keyboard_input(event, &mut self.ui_context) {
                if tb.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetSans(tb.text.clone())));
                }
                consumed = true;
            }

            let tb = &mut self.app.interface.serif_box;
            if tb.keyboard_input(event, &mut self.ui_context) {
                if tb.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetSerif(tb.text.clone())));
                }
                consumed = true;
            }

            let tb = &mut self.app.interface.mono_box;
            if tb.keyboard_input(event, &mut self.ui_context) {
                if tb.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetMono(tb.text.clone())));
                }
                consumed = true;
            }

            let tb = &mut self.app.interface.borders_box;
            if tb.keyboard_input(event, &mut self.ui_context) {
                if tb.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetBorders(tb.text.clone())));
                }
                consumed = true;
            }

            let tb = &mut self.app.interface.status_box;
            if tb.keyboard_input(event, &mut self.ui_context) {
                if tb.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetStatus(tb.text.clone())));
                }
                consumed = true;
            }

            let tb = &mut self.app.interface.fuzzel_box;
            if tb.keyboard_input(event, &mut self.ui_context) {
                if tb.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetFuzzel(tb.text.clone())));
                }
                consumed = true;
            }

            let tb = &mut self.app.interface.terminal_box;
            if tb.keyboard_input(event, &mut self.ui_context) {
                if tb.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetTerminal(tb.text.clone())));
                }
                consumed = true;
            }





            let sb = &mut self.app.interface.borders_size_box;
            if sb.keyboard_input(event, &mut self.ui_context) {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetBordersSize(sb.value)));
                consumed = true;
            }

            let sb = &mut self.app.interface.status_size_box;
            if sb.keyboard_input(event, &mut self.ui_context) {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetStatusSize(sb.value)));
                consumed = true;
            }

            let sb = &mut self.app.interface.fuzzel_size_box;
            if sb.keyboard_input(event, &mut self.ui_context) {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetFuzzelSize(sb.value)));
                consumed = true;
            }

            let sb = &mut self.app.interface.terminal_size_box;
            if sb.keyboard_input(event, &mut self.ui_context) {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetTerminalSize(sb.value)));
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
            let sb = &mut self.app.services.notifications_duration_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Services(pages::services::ServicesMessage::SetNotificationsDuration(new_val)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.services.status_padding_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Services(pages::services::ServicesMessage::StatusSetPadding(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
        }
        if self.app.current_page == Page::Input {
            if self.app.input.rate_spinbox.keyboard_input(event, &mut self.ui_context) {
                self.handle_action(&AppAction::Input(pages::input::InputMessage::ApplyRepeat));
                self.needs_rebuild = true;
                return true;
            }
            if self.app.input.delay_spinbox.keyboard_input(event, &mut self.ui_context) {
                self.handle_action(&AppAction::Input(pages::input::InputMessage::ApplyRepeat));
                self.needs_rebuild = true;
                return true;
            }
            if self.app.input.scroll_friction_spinbox.keyboard_input(event, &mut self.ui_context) {
                self.handle_action(&AppAction::Input(pages::input::InputMessage::ApplyScrollFriction));
                self.needs_rebuild = true;
                return true;
            }
            if self.app.input.scroll_speed_spinbox.keyboard_input(event, &mut self.ui_context) {
                self.handle_action(&AppAction::Input(pages::input::InputMessage::ApplyScrollSpeed));
                self.needs_rebuild = true;
                return true;
            }
            if self.app.input.pointer_friction_spinbox.keyboard_input(event, &mut self.ui_context) {
                self.handle_action(&AppAction::Input(pages::input::InputMessage::ApplyPointerFriction));
                self.needs_rebuild = true;
                return true;
            }
            if self.app.input.trackpad_friction_spinbox.keyboard_input(event, &mut self.ui_context) {
                self.handle_action(&AppAction::Input(pages::input::InputMessage::ApplyTrackpadFriction));
                self.needs_rebuild = true;
                return true;
            }
            if self.app.input.trackpoint_accel_speed_spinbox.keyboard_input(event, &mut self.ui_context) {
                self.handle_action(&AppAction::Input(pages::input::InputMessage::ApplyTrackpointAccelSpeed));
                self.needs_rebuild = true;
                return true;
            }
            if self.app.input.cursor_size_spinbox.keyboard_input(event, &mut self.ui_context) {
                self.handle_action(&AppAction::Input(pages::input::InputMessage::ApplyCursorSize));
                self.needs_rebuild = true;
                return true;
            }
            if self.app.input.zoom_in_box.keyboard_input(event, &mut self.ui_context) {
                self.handle_action(&AppAction::Input(pages::input::InputMessage::ApplyZoomIn));
                self.needs_rebuild = true;
                return true;
            }
            if self.app.input.zoom_out_box.keyboard_input(event, &mut self.ui_context) {
                self.handle_action(&AppAction::Input(pages::input::InputMessage::ApplyZoomOut));
                self.needs_rebuild = true;
                return true;
            }
        }
        if self.app.current_page == Page::Accounts {
            let mut consumed = false;
            if self.app.accounts.editing_oauth_creds {
                let tb = &mut self.app.accounts.oauth_client_id_box;
                if tb.keyboard_input(event, &mut self.ui_context) { consumed = true; }
                let tb = &mut self.app.accounts.oauth_client_secret_box;
                if tb.keyboard_input(event, &mut self.ui_context) { consumed = true; }
            } else if self.app.accounts.adding_new {
                let tb = &mut self.app.accounts.email_box;
                if tb.keyboard_input(event, &mut self.ui_context) {
                    consumed = true;
                    let email_val = tb.edit_buffer.trim().to_lowercase();
                    if email_val.ends_with("@gmail.com") {
                        self.app.accounts.imap_box.text = "imap.gmail.com:993".to_string();
                        self.app.accounts.imap_box.edit_buffer = "imap.gmail.com:993".to_string();
                        self.app.accounts.smtp_box.text = "smtp.gmail.com:465".to_string();
                        self.app.accounts.smtp_box.edit_buffer = "smtp.gmail.com:465".to_string();
                    } else if email_val.ends_with("@icloud.com") {
                        self.app.accounts.imap_box.text = "imap.mail.me.com:993".to_string();
                        self.app.accounts.imap_box.edit_buffer = "imap.mail.me.com:993".to_string();
                        self.app.accounts.smtp_box.text = "smtp.mail.me.com:587".to_string();
                        self.app.accounts.smtp_box.edit_buffer = "smtp.mail.me.com:587".to_string();
                    } else if email_val.ends_with("@outlook.com") || email_val.ends_with("@hotmail.com") {
                        self.app.accounts.imap_box.text = "outlook.office365.com:993".to_string();
                        self.app.accounts.imap_box.edit_buffer = "outlook.office365.com:993".to_string();
                        self.app.accounts.smtp_box.text = "smtp.office365.com:587".to_string();
                        self.app.accounts.smtp_box.edit_buffer = "smtp.office365.com:587".to_string();
                    }
                }
                let tb = &mut self.app.accounts.password_box;
                if tb.keyboard_input(event, &mut self.ui_context) { consumed = true; }
                let tb = &mut self.app.accounts.imap_box;
                if tb.keyboard_input(event, &mut self.ui_context) { consumed = true; }
                let tb = &mut self.app.accounts.smtp_box;
                if tb.keyboard_input(event, &mut self.ui_context) { consumed = true; }
            }
            
            if consumed {
                self.needs_rebuild = true;
                return true;
            }
        }
        if self.app.current_page == Page::Audio {
            let mut actions = Vec::new();
            for (i, sb) in self.app.audio.sink_spinboxes.iter_mut().enumerate() {
                let old = sb.value;
                if sb.keyboard_input(event, &mut self.ui_context) {
                    if sb.value != old {
                        let id = self.app.audio.sinks[i].id;
                        actions.push(AppAction::Audio(pages::audio::AudioMessage::SinkVolume(id, sb.value as f32 / 100.0)));
                    }
                }
            }
            for (i, sb) in self.app.audio.source_spinboxes.iter_mut().enumerate() {
                let old = sb.value;
                if sb.keyboard_input(event, &mut self.ui_context) {
                    if sb.value != old {
                        let id = self.app.audio.sources[i].id;
                        actions.push(AppAction::Audio(pages::audio::AudioMessage::SourceVolume(id, sb.value as f32 / 100.0)));
                    }
                }
            }
            for (i, slider) in self.app.audio.sink_sliders.iter_mut().enumerate() {
                let old = slider.value();
                if slider.keyboard_input(event, &mut self.ui_context) {
                    let new_val = slider.value();
                    if new_val != old {
                        let id = self.app.audio.sinks[i].id;
                        if i < self.app.audio.sink_spinboxes.len() {
                            self.app.audio.sink_spinboxes[i].value = (new_val * 100.0).round() as i32;
                        }
                        actions.push(AppAction::Audio(pages::audio::AudioMessage::SinkVolume(id, new_val)));
                    }
                }
            }
            for (i, slider) in self.app.audio.source_sliders.iter_mut().enumerate() {
                let old = slider.value();
                if slider.keyboard_input(event, &mut self.ui_context) {
                    let new_val = slider.value();
                    if new_val != old {
                        let id = self.app.audio.sources[i].id;
                        if i < self.app.audio.source_spinboxes.len() {
                            self.app.audio.source_spinboxes[i].value = (new_val * 100.0).round() as i32;
                        }
                        actions.push(AppAction::Audio(pages::audio::AudioMessage::SourceVolume(id, new_val)));
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
            let slider = &mut self.app.display.brightness_slider;
            let old_slider = slider.value();
            if slider.keyboard_input(event, &mut self.ui_context) {
                let new_slider = slider.value();
                if new_slider != old_slider {
                    let pct = (new_slider * 100.0).round() as u32;
                    self.app.display.brightness_spinbox.value = pct as i32;
                    self.handle_action(&AppAction::Display(pages::display::DisplayMessage::BrightnessSet(pct)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.display.brightness_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Display(pages::display::DisplayMessage::BrightnessSet(new_val as u32)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.display.screensaver_timeout_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Display(pages::display::DisplayMessage::SetScreensaverTimeout(new_val)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let (menu_changed, old_selected, new_selected) = {
                let menu = &mut self.app.display.screensaver_style_menu;
                let old = menu.selected;
                let changed = menu.keyboard_input(event, &mut self.ui_context);
                (changed, old, menu.selected)
            };
            if menu_changed {
                if new_selected != old_selected {
                    self.handle_action(&AppAction::Display(pages::display::DisplayMessage::SetScreensaverStyle(new_selected)));
                }
                self.needs_rebuild = true;
                return true;
            }
        }


        if self.app.current_page == Page::Services {
            let srv = &mut self.app.services;
            if srv.list_box.keyboard_input(event, &mut self.ui_context) {
                self.needs_rebuild = true;
                return true;
            }
            let tb = &mut srv.search_box;
            if tb.keyboard_input(event, &mut self.ui_context) {
                tb.take_change();
                self.needs_rebuild = true;
                return true;
            }
        }
        if self.app.current_page == Page::Packages {
            let pkgs = &mut self.app.packages;
            match pkgs.active_tab {
                pages::packages::PackageTab::Installed => {
                    if pkgs.installed_list_box.keyboard_input(event, &mut self.ui_context) {
                        self.needs_rebuild = true;
                        return true;
                    }
                }
                pages::packages::PackageTab::Updates => {
                    if pkgs.updates_list_box.keyboard_input(event, &mut self.ui_context) {
                        self.needs_rebuild = true;
                        return true;
                    }
                }
            }
            let tb = &mut pkgs.search_box;
            if tb.keyboard_input(event, &mut self.ui_context) {
                tb.take_change();
                self.needs_rebuild = true;
                return true;
            }
        }
        if self.app.current_page == Page::Hardware {
            let hw = &mut self.app.hardware;
            if hw.cpu_list_box.keyboard_input(event, &mut self.ui_context) {
                self.needs_rebuild = true;
                return true;
            }
            let (cpu_changed, old_cpu, new_cpu) = {
                let menu = &mut hw.cpu_gov_menu;
                let old = menu.selected;
                let changed = menu.keyboard_input(event, &mut self.ui_context);
                (changed, old, menu.selected)
            };
            if cpu_changed {
                if new_cpu != old_cpu {
                    if new_cpu == 0 {
                        self.handle_action(&AppAction::Hardware(pages::hardware::HardwareMessage::SetCpuPerformance));
                    } else {
                        self.handle_action(&AppAction::Hardware(pages::hardware::HardwareMessage::SetCpuPowersave));
                    }
                }
                self.needs_rebuild = true;
                return true;
            }
            let (gpu_changed, old_gpu, new_gpu) = {
                let menu = &mut hw.gpu_gov_menu;
                let old = menu.selected;
                let changed = menu.keyboard_input(event, &mut self.ui_context);
                (changed, old, menu.selected)
            };
            if gpu_changed {
                if new_gpu != old_gpu {
                    if new_gpu == 0 {
                        self.handle_action(&AppAction::Hardware(pages::hardware::HardwareMessage::SetGpuDefault));
                    } else {
                        self.handle_action(&AppAction::Hardware(pages::hardware::HardwareMessage::SetGpuPowersave));
                    }
                }
                self.needs_rebuild = true;
                return true;
            }
        }
        if self.app.current_page == Page::Radios {
            let net = &mut self.app.network;
            if net.wifi_list_box.keyboard_input(event, &mut self.ui_context) {
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

    // Try to load last_page from config
    let config_path = "/home/lsgalante/.config/cce/config.toml";
    if let Ok(content) = std::fs::read_to_string(config_path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("last_page") {
                if let Some(val_str) = trimmed.split('=').nth(1) {
                    let last_page_val = val_str.trim().trim_matches('"').trim_matches('\'').trim().to_lowercase();
                    for page in Page::ALL {
                        if page.label().to_lowercase() == last_page_val {
                            initial_page = page;
                            break;
                        }
                    }
                }
            }
        }
    }

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

    cce_ui::engine::run::<SystemInterface>();
}
