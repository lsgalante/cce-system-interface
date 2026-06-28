use cce_ui::widget::{Finger, hover_animation, Element, PageSelector};
use glyphon::{Attrs, Buffer, FontSystem, Metrics};

use cce_system_settings::app::{AppAction, AppState};
use cce_system_settings::pages::{self, Page};
mod input_handler;
mod renderer;

#[derive(Hash, PartialEq, Eq, Clone)]
struct BufferCacheKey {
    text: String,
    size_milli: u32,
    font: Option<String>,
    sans_fallback: String,
    serif_fallback: String,
    mono_fallback: String,
}

std::thread_local! {
    static BUFFER_CACHE: std::cell::RefCell<std::collections::HashMap<BufferCacheKey, Buffer>> = std::cell::RefCell::new(std::collections::HashMap::new());
}

fn make_text_buffer(fs: &mut FontSystem, text: &str, size: f32) -> Buffer {
    make_text_buffer_with_font(fs, text, size, None, "", "", "")
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
    let size_key = (physical_size * 1000.0).round() as u32;

    let key = BufferCacheKey {
        text: text.to_string(),
        size_milli: size_key,
        font: family_name.clone(),
        sans_fallback: sans_fallback.to_string(),
        serif_fallback: serif_fallback.to_string(),
        mono_fallback: mono_fallback.to_string(),
    };

    let cached = BUFFER_CACHE.with(|cache| {
        cache.borrow().get(&key).cloned()
    });

    if let Some(buf) = cached {
        return buf;
    }

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

    BUFFER_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.len() > 3000 {
            cache.clear();
        }
        cache.insert(key, buf.clone());
    });

    buf
}

#[allow(dead_code)]
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
    rx_layout: std::sync::mpsc::Receiver<pages::interface::WindowsState>,
    rx_input: std::sync::mpsc::Receiver<pages::input::InputState>,
    rx_fingers: std::sync::mpsc::Receiver<Vec<Finger>>,
    rx_processes: std::sync::mpsc::Receiver<pages::processes::ProcessesState>,
    rx_system: std::sync::mpsc::Receiver<pages::system_info::SystemState>,
    rx_storage: std::sync::mpsc::Receiver<pages::storage::StorageState>,
    rx_notifications: std::sync::mpsc::Receiver<pages::system_info::NotificationsConfig>,
    rx_typeface: std::sync::mpsc::Receiver<pages::interface::InterfaceState>,
    rx_services: std::sync::mpsc::Receiver<Vec<pages::processes::ServiceInfo>>,
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
    scrollable_widgets_start_idx: usize,
    scrollable_text_items_start_idx: usize,
    scrollable_buttons_start_idx: usize,
    last_scroll_y: f32,
    page_sec_containers: Vec<cce_ui::widget::SectionContainer>,
    root_window: cce_ui::widget::Backplate,
    menubar: cce_ui::widget::Paginator,
    switcher: cce_ui::widget::Switcher,
    pages: Vec<cce_ui::widget::Page>,
    sans_serif_family: String,
    serif_family: String,
    monospace_family: String,
    current_page_shared: std::sync::Arc<std::sync::atomic::AtomicU8>,
    sender: calloop::channel::Sender<AppAction>,
    ui_context: cce_ui::context::UiContext,
    scroll_logs: Vec<String>,
    search_open: bool,
    search_query: String,
    search_box: cce_ui::widget::input::TextBox,
}

impl cce_ui::engine::Application for SystemInterface {
    type Message = AppAction;

    fn new(_qh: &wayland_client::QueueHandle<cce_ui::engine::EngineState<Self>>, sender: calloop::channel::Sender<Self::Message>) -> Self {
        cce_ui::scale::set_scale_factor(1.0);
        let app = AppState {
            input: pages::input::read_input_config(),
            interface: pages::interface::read_interface_config(),
            ..Default::default()
        };

        // ── Background refresh channels ──
        let initial_page_idx = INITIAL_PAGE_INDEX.load(std::sync::atomic::Ordering::SeqCst);
        let current_page_shared = std::sync::Arc::new(std::sync::atomic::AtomicU8::new(initial_page_idx as u8));

        let (watchers, tx_backup, rx_backup, tx_update, rx_update) =
            cce_system_settings::watchers::spawn_all(current_page_shared.clone());

        let (sans_family, serif_family, monospace_family, _, _, _, _) = pages::interface::read_preferred_fonts();

        let pages_names = Page::ALL.iter().map(|p| p.label().to_string()).collect::<Vec<_>>();
        let menubar = cce_ui::widget::Paginator::new(pages_names);
        let sidebar_width = menubar.sidebar_w();

        let switcher = cce_ui::widget::Switcher::new(sidebar_width, 0.0, 820.0 - sidebar_width, 680.0);
        let mut pages = Vec::new();
        for page in Page::ALL.iter() {
            let page_widget = cce_ui::widget::Page::new(sidebar_width, 0.0, 820.0 - sidebar_width, 680.0)
                .with_label(page.label());
            pages.push(page_widget);
        }

        let mut app_state = app;
        app_state.current_page = Page::ALL[initial_page_idx];

        let win_color = app_state.interface.backplate_color;
        let win_opacity = 1.0f32;
        let win_radius = app_state.interface.backplate_corner_radius;

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
            rx_audio: watchers.rx_audio,
            rx_display: watchers.rx_display,
            rx_network: watchers.rx_network,
            rx_layout: watchers.rx_layout,
            rx_input: watchers.rx_input,
            rx_fingers: watchers.rx_fingers,
            rx_processes: watchers.rx_processes,
            rx_system: watchers.rx_system,
            rx_storage: watchers.rx_storage,
            rx_notifications: watchers.rx_notifications,
            rx_typeface: watchers.rx_typeface,
            rx_services: watchers.rx_services,
            rx_interface: watchers.rx_interface,
            rx_accounts: watchers.rx_accounts,
            tx_backup,
            rx_backup,
            rx_packages: watchers.rx_packages,
            tx_update,
            rx_update,

            scale_factor: 1.0,
            width: 820,
            height: 680,
            needs_rebuild: true,
            scroll_y: 0.0,
            max_scroll_y: 0.0,
            scrollable_widgets_start_idx: 0,
            scrollable_text_items_start_idx: 0,
            scrollable_buttons_start_idx: 0,
            last_scroll_y: 0.0,
            page_sec_containers: Vec::new(),
            root_window: cce_ui::widget::Backplate::new(0.0, 0.0, 820.0, 680.0)
                .with_movable(false)
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
            pages,
            sans_serif_family: sans_family,
            serif_family,
            monospace_family,
            current_page_shared,
            sender,
            ui_context: cce_ui::context::UiContext::new(),
            scroll_logs: Vec::new(),
            search_open: false,
            search_query: String::new(),
            search_box: cce_ui::widget::input::TextBox::new(String::new())
                .with_placeholder("Search sections & parameters...")
                .with_draw_bg_border(false),
        };

        for page in &mut this.pages {
            this.switcher.add_child(page.as_ptr(), &mut this.ui_context);
        }

        this.root_window.add_child(this.menubar.as_ptr(), &mut this.ui_context);
        this.root_window.add_child(this.switcher.as_ptr(), &mut this.ui_context);

        this.rebuild_layout(820.0, 680.0);
        this.needs_rebuild = true;
        this
    }

    fn settings(&self) -> cce_ui::engine::WindowSettings {
        cce_ui::engine::WindowSettings {
            title: "CCE System Settings".to_string(),
            app_id: "cce-system-settings".to_string(),
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
        if self.needs_rebuild {
            self.rebuild_layout(self.width as f32, self.height as f32);
        }
        self.poll_background_updates();
        if self.tick_internal(dt) {
            *needs_rebuild = true;
        }
        if self.needs_rebuild || self.ui_context.is_dirty() {
            *needs_rebuild = true;
            self.needs_rebuild = true;
        }
    }

    fn view(&mut self, _quads: &mut Vec<(f32, f32, f32, f32, [f32; 4])>, size: cce_ui::engine::LogicalSize, scale: f64) {
        let (width, height) = (size.width, size.height);
        if self.needs_rebuild || self.ui_context.is_dirty() || self.width != width as u32 || self.height != height as u32 || self.scale_factor != scale {
            self.width = width as u32;
            self.height = height as u32;
            self.scale_factor = scale;
            cce_ui::scale::set_scale_factor(scale as f32);
            self.rebuild_layout(width, height);
        }
    }

    fn view_rounded_quads(&mut self, quads: &mut Vec<(f32, f32, f32, f32, f32, [f32; 4], (bool, bool, bool, bool))>, size: cce_ui::engine::LogicalSize, scale: f64) {
        let (width, height) = (size.width, size.height);
        if self.needs_rebuild || self.ui_context.is_dirty() || self.width != width as u32 || self.height != height as u32 || self.scale_factor != scale {
            self.width = width as u32;
            self.height = height as u32;
            self.scale_factor = scale;
            cce_ui::scale::set_scale_factor(scale as f32);
            self.rebuild_layout(width, height);
        }
        for w in &self.widgets {
            quads.push((w.x, w.y, w.w, w.h, w.radius, w.color, w.corners));
        }

        // Draw global hover highlight if active
        let s = scale as f32;
        cce_ui::widget::hover_animation::post_render_check();
        if let Some((qx, qy, qw, qh, qc)) = cce_ui::widget::hover_animation::get_quad() {
            quads.push((
                qx * s,
                (qy - self.scroll_y) * s,
                qw * s,
                qh * s,
                0.0,
                qc,
                (true, true, true, true),
            ));
        }
    }

    fn text_items(&self) -> &[cce_ui::widget::TextItem] {
        &self.text_items
    }

    fn ui_context(&self) -> Option<&cce_ui::context::UiContext> {
        Some(&self.ui_context)
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


    fn tick_internal(&mut self, dt: f32) -> bool {
        let mut needs_redraw = false;
        if hover_animation::tick(dt) {
            needs_redraw = true;
        }
        if self.ui_context.tick(dt) {
            needs_redraw = true;
            self.needs_rebuild = true;
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
                4 => self.app.interface.slider_track_color,
                5 => self.app.interface.color_borders_color,
                6 => self.app.interface.desktop_background_color,
                7 => self.app.interface.normal_color,
                8 => self.app.interface.paginator_sidebar_color,
                9 => self.app.interface.primary_highlight_color,
                10 => self.app.interface.menubar_tab_label_color,
                11 => self.app.interface.toggle_enabled_color,
                12 => self.app.interface.toggle_disabled_color,
                13 => self.app.interface.scrollinglist_bg_color,
                14 => self.app.interface.breadcrumb_bg_color,
                15 => self.app.interface.popover_bg_color,
                16 => self.app.interface.notification_bg_color,
                17 => self.app.interface.backplate_color,
                18 => self.app.interface.page_color,
                19 => self.app.interface.layer_color,
                22 => self.app.interface.status_box_background_color,
                _ => self.app.interface.desktop_background_color,
            };
            if i == 20 || i == 21 || i == 23 {
                let (state_rgb, state_alpha) = if i == 20 {
                    (
                        [
                            self.app.interface.scrollinglist_entry_bg_color[0],
                            self.app.interface.scrollinglist_entry_bg_color[1],
                            self.app.interface.scrollinglist_entry_bg_color[2],
                        ],
                        self.app.interface.scrollinglist_entry_bg_color[3],
                    )
                } else if i == 21 {
                    (
                        [
                            self.app.interface.scrollinglist_entry_highlight_color[0],
                            self.app.interface.scrollinglist_entry_highlight_color[1],
                            self.app.interface.scrollinglist_entry_highlight_color[2],
                        ],
                        self.app.interface.scrollinglist_entry_highlight_color[3],
                    )
                } else {
                    (
                        [
                            self.app.interface.desktop_grid_color[0],
                            self.app.interface.desktop_grid_color[1],
                            self.app.interface.desktop_grid_color[2],
                        ],
                        self.app.interface.desktop_grid_color[3],
                    )
                };
                if cp.color != state_rgb || cp.alpha != state_alpha {
                    color_actions.push(AppAction::Interface(if i == 20 {
                        pages::interface::InterfaceMessage::SetScrollingListEntryBgColor([cp.color[0], cp.color[1], cp.color[2], cp.alpha])
                    } else if i == 21 {
                        pages::interface::InterfaceMessage::SetScrollingListEntryHighlightColor([cp.color[0], cp.color[1], cp.color[2], cp.alpha])
                    } else {
                        pages::interface::InterfaceMessage::SetDesktopGridColor([cp.color[0], cp.color[1], cp.color[2], cp.alpha])
                    }));
                    color_changed = true;
                }
            } else if cp.color != state_color {
                color_actions.push(AppAction::Interface(match i {
                    0 => pages::interface::InterfaceMessage::SetPageLowColor(cp.color),
                    1 => pages::interface::InterfaceMessage::SetHighColor(cp.color),
                    2 => pages::interface::InterfaceMessage::SetVisualGuidesColor(cp.color),
                    3 => pages::interface::InterfaceMessage::SetDisabledColor(cp.color),
                    4 => pages::interface::InterfaceMessage::SetSliderTrackColor(cp.color),
                    5 => pages::interface::InterfaceMessage::SetColorBordersColor(cp.color),
                    6 => pages::interface::InterfaceMessage::SetDesktopBackground(cp.color),
                    7 => pages::interface::InterfaceMessage::SetNormalColor(cp.color),
                    8 => pages::interface::InterfaceMessage::SetPaginatorSidebarColor(cp.color),
                    9 => pages::interface::InterfaceMessage::SetPrimaryHighlightColor(cp.color),
                    10 => pages::interface::InterfaceMessage::SetMenubarTabLabelColor(cp.color),
                    11 => pages::interface::InterfaceMessage::SetToggleEnabledColor(cp.color),
                    12 => pages::interface::InterfaceMessage::SetToggleDisabledColor(cp.color),
                    13 => pages::interface::InterfaceMessage::SetScrollingListBgColor(cp.color),
                    14 => pages::interface::InterfaceMessage::SetBreadcrumbBgColor(cp.color),
                    15 => pages::interface::InterfaceMessage::SetPopoverBgColor(cp.color),
                    16 => pages::interface::InterfaceMessage::SetNotificationBgColor(cp.color),
                    17 => pages::interface::InterfaceMessage::SetBackplateColor(cp.color),
                    18 => pages::interface::InterfaceMessage::SetPageColor(cp.color),
                    19 => pages::interface::InterfaceMessage::SetLayerColor(cp.color),
                    22 => pages::interface::InterfaceMessage::SetStatusBoxBackgroundColor(cp.color),
                    _ => pages::interface::InterfaceMessage::SetDesktopBackground(cp.color),
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
            interface::update_windows(&mut self.app.interface.windows, interface::WindowsMessage::Refreshed(s));
            if self.app.current_page == Page::Interface {
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
        while let Ok(s) = self.rx_processes.try_recv() {
            processes::update(&mut self.app.processes, processes::ProcessesMessage::Refreshed(s));
            if self.app.current_page == Page::Processes {
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
            system_info::update(&mut self.app.system_info, system_info::SystemMessage::NotificationsRefreshed(s));
            if self.app.current_page == Page::System {
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
            processes::update(&mut self.app.processes, processes::ProcessesMessage::ServicesRefreshed(s));
            if self.app.current_page == Page::Processes {
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
            AppAction::Input(m) => input::update(&mut self.app.input, m.clone()),
            AppAction::SystemInfo(m) => system_info::update(&mut self.app.system_info, m.clone()),
            AppAction::Processes(m) => processes::update(&mut self.app.processes, m.clone()),
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

}

fn main() {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let _guard = rt.enter();

    let mut initial_page = Page::ALL[0];

    // Try to load last_page from config
    let config_path = "/home/lsgalante/.config/cce/config.json";
    if let Ok(content) = std::fs::read_to_string(config_path) {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(last_page_val) = val.pointer("/layout/last_page").and_then(|v| v.as_str()) {
                let last_page_val = last_page_val.trim_matches('"').trim_matches('\'').trim().to_lowercase();
                for page in Page::ALL {
                    if page.label().to_lowercase() == last_page_val {
                        initial_page = page;
                        break;
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

impl Drop for SystemInterface {
    fn drop(&mut self) {
        if !self.scroll_logs.is_empty() {
            if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("/tmp/cce-scroll-debug.log") {
                use std::io::Write;
                for log in &self.scroll_logs {
                    let _ = writeln!(file, "{}", log);
                }
            }
        }
    }
}
