use cce_ui::widget::hover_animation;
use cce_ui::cosmic_text::{Buffer, FontSystem};

use cce_settings::app::{AppAction, AppState, ControlCarve};
use cce_settings::pages::{self, Page};
mod input_handler;
mod renderer;
mod scroll_bar;

fn make_text_buffer_with_font(
    fs: &mut FontSystem,
    text: &str,
    size: f32,
    font: Option<&str>,
    _sans_fallback: &str,
    _serif_fallback: &str,
    _mono_fallback: &str,
) -> Buffer {
    cce_ui::backend::get_text_buffer(fs, text, size, font)
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
    // (content, font_size, x, y, color, font, bounds) — the frame's text, emitted as
    // display-list Text prims (scroll shift, search dim/highlight, and viewport clamps
    // already applied by rebuild_layout).
    texts: Vec<(String, f32, f32, f32, [f32; 4], Option<String>, Option<[f32; 4]>)>,
    // Popover + context-menu content, drawn INTO the frame on top of everything (Phase 6t —
    // no engine xdg popup). Separate from widgets/texts so the wheel fast-path never
    // scrolls them.
    popover_widgets: Vec<AppWidget>,
    popover_texts: Vec<(String, f32, f32, f32, [f32; 4], Option<String>, Option<[f32; 4]>)>,
    /// Popover-layer inset plates (window coords, scroll already applied) —
    /// the dropdown's grown-trigger surface via the inset_plate hook.
    popover_control_reliefs: Vec<ControlCarve>,
    page_buttons: Vec<(cce_ui::widget::Adapted<cce_ui::widget::Button>, AppAction)>,

    sidebar_width: f32,
    header_height: f32,
    status_height: f32,

    cursor_x: f32,
    cursor_y: f32,

    rx_audio: std::sync::mpsc::Receiver<pages::audio::AudioState>,
    rx_network: std::sync::mpsc::Receiver<pages::network::NetworkState>,
    rx_bluetooth: std::sync::mpsc::Receiver<pages::bluetooth::BluetoothState>,
    rx_power: std::sync::mpsc::Receiver<pages::power::PowerFacts>,
    pub rx_processes: std::sync::mpsc::Receiver<pages::processes::ProcessesState>,
    rx_system: std::sync::mpsc::Receiver<pages::system_info::SystemInfo>,
    rx_storage: std::sync::mpsc::Receiver<pages::storage::StorageInfo>,
    rx_notifications: std::sync::mpsc::Receiver<pages::notifications::NotificationsConfig>,
    rx_browser: std::sync::mpsc::Receiver<pages::browser::BrowserConfig>,
    rx_services: std::sync::mpsc::Receiver<Vec<pages::services::ServiceInfo>>,
    rx_default_apps: std::sync::mpsc::Receiver<pages::default_apps::DefaultAppsInfo>,
    rx_timers: std::sync::mpsc::Receiver<Vec<pages::timers::TimerInfo>>,
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
    // SectionContainer DISSOLVED (Phase 6w): section-level keyboard focus is this index
    // (single-slot with the global widget focus — descending clears it); the per-section
    // widget groups come from AppPage::section_widgets each time they're needed.
    focused_section: Option<usize>,
    // The page the last `rebuild_layout` actually laid out. Registration is a side effect
    // of the view pass (`render_widget`), and `clear_hierarchy` wipes the registry each
    // rebuild — so between a page switch and the next rebuild, the NEW page's
    // `section_widgets()` ids are not registered and every event to them is dropped with
    // a router warning. `dispatch_page_event` suppresses dispatch across that gap.
    laid_out_page: Option<Page>,
    page_dropdown: cce_ui::widget::Adapted<cce_ui::widget::input::Dropdown>,
    // Switcher + Page DISSOLVED (Phase 6u): the current page is app.current_page, page
    // scroll is scroll_y/max_scroll_y, and the page scrollbar is this app-owned widget
    // (rendered into the window assembly, evented directly). content_h feeds it — the
    // window pass reads last frame's value, exactly as the legacy Page did.
    page_scroll_bar: cce_ui::widget::Adapted<crate::scroll_bar::ScrollBar>,
    content_h: f32,
    // Section wells: body box + title tab (page coords, pre-scroll) — carved by
    // display_list.
    page_reliefs: Vec<((f32, f32, f32, f32), Option<(f32, f32, f32, f32)>)>,
    /// Control troughs from `PageContent::control_reliefs` (page coords,
    /// pre-scroll) — each carved as a flush inset plate after the section wells.
    page_control_reliefs: Vec<ControlCarve>,
    /// Icon faces for the page's buttons — `(image, x, y, w, h, alpha)`, page
    /// coords already scroll-shifted by the renderer. A flat host draws no
    /// images at all otherwise: `all_quads` carries quads and the text list
    /// carries labels, and an icon is neither.
    page_button_images: Vec<(u32, f32, f32, f32, f32, f32)>,
    // Root Backplate + StatusBar DISSOLVED (Phase 6s): the window plate and the status
    // bar are emitted as tuples in rebuild_layout.
    sans_serif_family: String,
    serif_family: String,
    monospace_family: String,
    current_page_shared: std::sync::Arc<std::sync::atomic::AtomicU8>,
    sender: calloop::channel::Sender<AppAction>,
    ui_context: cce_ui::context::UiContext,
    scroll_logs: Vec<String>,
    search_open: bool,
    search_query: String,
    search_box: cce_ui::widget::Adapted<cce_ui::widget::input::TextBox>,

}

impl cce_ui::engine::Application for SystemInterface {
    type Message = AppAction;

    fn new(_qh: &wayland_client::QueueHandle<cce_ui::engine::EngineState<Self>>, sender: calloop::channel::Sender<Self::Message>) -> Self {
        cce_ui::scale::set_scale_factor(1.0);
        let app = AppState::default();

        // ── Background refresh channels ──
        let initial_page_idx = INITIAL_PAGE_INDEX.load(std::sync::atomic::Ordering::SeqCst);
        let current_page_shared = std::sync::Arc::new(std::sync::atomic::AtomicU8::new(initial_page_idx as u8));

        let (watchers, tx_backup, rx_backup, tx_update, rx_update) =
            cce_settings::watchers::spawn_all(current_page_shared.clone());

        let (sans_family, serif_family, monospace_family, _) = cce_ui::layout::read_preferred_fonts();

        let pages_names = Page::ALL.iter().map(|p| p.label().to_string()).collect::<Vec<_>>();
        let page_dropdown = cce_ui::widget::input::Dropdown::new(pages_names, initial_page_idx)
            .with_open_upward(true)
            .with_auto_width(true);
        let sidebar_width = 0.0f32;

        let mut app_state = app;
        app_state.current_page = Page::ALL[initial_page_idx];

        let font_system = cce_ui::create_font_system();

        let mut this = Self {
            app: app_state,
            font_system,
            widgets: Vec::new(),
            texts: Vec::new(),
            popover_widgets: Vec::new(),
            popover_texts: Vec::new(),
            popover_control_reliefs: Vec::new(),
            page_buttons: Vec::new(),
            sidebar_width,
            header_height: 0.0,
            status_height: 24.0,
            cursor_x: 0.0,
            cursor_y: 0.0,
            rx_audio: watchers.rx_audio,
            rx_network: watchers.rx_network,
            rx_bluetooth: watchers.rx_bluetooth,
            rx_power: watchers.rx_power,
            rx_processes: watchers.rx_processes,
            rx_default_apps: watchers.rx_default_apps,
            rx_timers: watchers.rx_timers,
            rx_system: watchers.rx_system,
            rx_storage: watchers.rx_storage,
            rx_notifications: watchers.rx_notifications,
            rx_browser: watchers.rx_browser,
            rx_services: watchers.rx_services,
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
            focused_section: None,
            laid_out_page: None,
            page_dropdown,
            page_scroll_bar: crate::scroll_bar::ScrollBar::new(),
            content_h: 0.0,
            page_reliefs: Vec::new(),
            page_control_reliefs: Vec::new(),
            page_button_images: Vec::new(),
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
        this.app.system_info.sender = Some(this.sender.clone());

        this.rebuild_layout(820.0, 680.0);
        this.needs_rebuild = true;
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
        if let AppAction::Exit = msg {
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
        let mut actions = Vec::new();
        self.propagate_widget_changes(&mut actions);
        for action in actions {
            self.handle_action(&action);
        }
        if self.needs_rebuild || self.ui_context.is_dirty() {
            *needs_rebuild = true;
            self.needs_rebuild = true;
        }
    }

    fn display_list(&mut self, size: cce_ui::engine::LogicalSize, scale: f64) -> Option<cce_ui::scene::paint::DisplayList> {
        // Phase 6 single paint path: the whole frame — geometry and text — is this one list.
        // rebuild_layout flattens the UI into self.widgets/self.texts (scroll shift, search
        // dim/highlight, and viewport clamps already applied).
        let (width, height) = (size.width, size.height);
        if self.needs_rebuild || self.ui_context.is_dirty() || self.width != width as u32 || self.height != height as u32 || self.scale_factor != scale {
            self.width = width as u32;
            self.height = height as u32;
            self.scale_factor = scale;
            cce_ui::scale::set_scale_factor(scale as f32);
            self.rebuild_layout(width, height);
        }
        use cce_ui::scene::layout::Rect;
        let mut pc = cce_ui::scene::paint::PaintCtx::new();

        // One glass slab (data-editor's idiom): the beveled window plate, with the
        // status bar carved into it as a step — everything else paints on top.
        {
            let mut plate = cce_ui::color::to_linear([0x0a as f32 / 255.0, 0x1a as f32 / 255.0, 0x0e as f32 / 255.0, 1.0]);
            if plate[3] > 0.001 {
                plate[3] = cce_ui::color::root_plate_opacity();
            }
            // PlateSpec (cce-ui RFC 7b): the hardcoded r=12 detached this
            // plate's corners from the compositor's silhouette clip.
            pc.plate_spec(&cce_ui::scene::paint::PlateSpec {
                rect: Rect { x: 0.0, y: 0.0, width, height },
                color: plate,
                blur: false,
                window_corners: (true, true, true, true),
                depth: cce_ui::layout::bevel_width(),
            });
            let sb_h = self.status_height;
            let depth = cce_ui::layout::bar_wall_width().min(sb_h * 0.6);
            pc.recess_edges(
                Rect { x: 0.0, y: height - sb_h, width, height: sb_h },
                (0.0, 0.0, 0.0, 0.0),
                depth,
                (true, false, false, false),
            );
        }

        for w in &self.widgets {
            let color = if w.hovering { w.hover_color } else { w.color };
            let rect = Rect { x: w.x, y: w.y, width: w.w, height: w.h };
            if w.radius > 0.1 {
                pc.rounded_rect(rect, w.radius, w.corners, color);
            } else {
                pc.quad(rect, color);
            }
        }
        // The section wells, carved after the page's flat quads so the walls shade
        // the fills they cross (the designer relief order), clipped to the page
        // viewport so a scrolled-off well can't shade the status bar or search row.
        if !self.page_reliefs.is_empty() {
            let view = Rect {
                x: self.sidebar_width,
                y: self.header_height,
                width: width - self.sidebar_width,
                height: (height - self.header_height - self.status_height
                    - if self.search_open { 42.0 } else { 0.0 }).max(0.0),
            };
            let r = 20.0f32;
            let scroll_y = self.scroll_y;
            pc.clip(view, |pc| {
                for &((cx, cy, cw, ch), tab) in &self.page_reliefs {
                    let cy = cy - scroll_y;
                    let depth = cce_ui::layout::bevel_width().min(ch * 0.2);
                    match tab {
                        Some((tx, ty, tw, th)) => {
                            // The designer union carve: the title tab bottom-open, one
                            // piece owning the whole right run so its corners are real
                            // turns, a left piece carrying the left wall — pieces
                            // extend past their interior seam by `depth` so the walls
                            // crossfade there instead of notching — and the throat's
                            // inside corner rounded by a concave fillet.
                            let ty = ty - scroll_y;
                            let rt = r.min(th * 0.45);
                            let throat_r = tx + tw;
                            let rho = 10.0f32; // designer SECTION_FILLET_R
                            let body_lr = |x_run: f32, pc: &mut cce_ui::scene::paint::PaintCtx| {
                                pc.recess_edges(
                                    Rect { x: x_run, y: cy, width: cx + cw - x_run, height: ch },
                                    (0.0, r, r, 0.0),
                                    depth,
                                    (true, true, true, false),
                                );
                                pc.recess_edges(
                                    Rect { x: cx, y: cy, width: x_run + depth - cx, height: ch },
                                    (0.0, 0.0, 0.0, r),
                                    depth,
                                    (false, false, true, true),
                                );
                            };
                            if cx + cw > throat_r + 2.0 * rho {
                                // Filleted throat: the tab's right wall ends at the
                                // fillet's vertical tangent, a left-only bridge
                                // carries the left wall across the fillet span.
                                pc.recess_edges(
                                    Rect { x: tx, y: ty, width: tw, height: (cy - rho) - ty + depth },
                                    (rt, rt, 0.0, 0.0),
                                    depth,
                                    (true, true, false, true),
                                );
                                pc.recess_edges(
                                    Rect { x: tx, y: cy - rho, width: tw, height: rho + depth },
                                    (0.0, 0.0, 0.0, 0.0),
                                    depth,
                                    (false, false, false, true),
                                );
                                body_lr(throat_r + rho - depth, pc);
                                pc.concave_fillet(throat_r + rho, cy - rho, rho, depth, std::f32::consts::FRAC_PI_2, false);
                            } else if cx + cw > throat_r + 0.5 {
                                // Too narrow for the fillet: the plain square throat.
                                pc.recess_edges(
                                    Rect { x: tx, y: ty, width: tw, height: (cy - ty) + depth },
                                    (rt, rt, 0.0, 0.0),
                                    depth,
                                    (true, true, false, true),
                                );
                                body_lr(throat_r - depth, pc);
                            } else {
                                // The tab spans the body: no top wall at all.
                                pc.recess_edges(
                                    Rect { x: tx, y: ty, width: tw, height: (cy - ty) + depth },
                                    (rt, rt, 0.0, 0.0),
                                    depth,
                                    (true, true, false, true),
                                );
                                pc.recess_edges(
                                    Rect { x: cx, y: cy, width: cw, height: ch },
                                    (0.0, 0.0, r, r),
                                    depth,
                                    (false, true, true, true),
                                );
                            }
                        }
                        None => {
                            pc.recess_edges(
                                Rect { x: cx, y: cy, width: cw, height: ch },
                                (r, r, r, r),
                                depth,
                                (true, true, true, true),
                            );
                        }
                    }
                }
            });
        }

        // Control troughs (Dropdown flush inset chrome, offered by the flat
        // bridge): carved after the section wells so they shade the fills
        // beneath, clipped to the page viewport like the wells.
        if !self.page_control_reliefs.is_empty() {
            let view = Rect {
                x: self.sidebar_width,
                y: self.header_height,
                width: width - self.sidebar_width,
                height: (height - self.header_height - self.status_height
                    - if self.search_open { 42.0 } else { 0.0 }).max(0.0),
            };
            let scroll_y = self.scroll_y;
            pc.clip(view, |pc| {
                for &carve in &self.page_control_reliefs {
                    match carve.shifted_y(-scroll_y) {
                        ControlCarve::Plate { x, y, w, h, radius, depth, color } => pc.inset_plate(
                            Rect { x, y, width: w, height: h },
                            (radius, radius, radius, radius),
                            color,
                            depth,
                        ),
                        ControlCarve::Step(c) => pc.carve(&c),
                    }
                }
            });
        }

        // Button icon faces, over the page's quads and its carves — clipped to
        // the page viewport, which is what cuts a half-scrolled list row's icon
        // at the list edge (an image has no geometry to trim, only a clip).
        if !self.page_button_images.is_empty() {
            let view = Rect {
                x: self.sidebar_width,
                y: self.header_height,
                width: width - self.sidebar_width,
                height: (height - self.header_height - self.status_height
                    - if self.search_open { 42.0 } else { 0.0 }).max(0.0),
            };
            pc.clip(view, |pc| {
                for &(image, x, y, w, h, alpha) in &self.page_button_images {
                    pc.image(image, Rect { x, y, width: w, height: h }, alpha);
                }
            });
        }

        cce_ui::widget::hover_animation::post_render_check();
        if let Some((qx, qy, qw, qh, qc)) = cce_ui::widget::hover_animation::get_quad() {
            pc.quad(Rect { x: qx, y: qy - self.scroll_y, width: qw, height: qh }, qc);
        }
        for w in &self.popover_widgets {
            let rect = Rect { x: w.x, y: w.y, width: w.w, height: w.h };
            if w.radius > 0.1 {
                pc.rounded_rect(rect, w.radius, w.corners, w.color);
            } else {
                pc.quad(rect, w.color);
            }
        }
        // Popover surfaces claimed through the inset_plate hook (the dropdown's
        // grown-trigger plate): real relief prims at the popover layer, over
        // the page and its control troughs.
        for &carve in &self.popover_control_reliefs {
            match carve {
                ControlCarve::Plate { x, y, w, h, radius, depth, color } => pc.inset_plate(
                    Rect { x, y, width: w, height: h },
                    (radius, radius, radius, radius),
                    color,
                    depth,
                ),
                ControlCarve::Step(c) => pc.carve(&c),
            }
        }
        for (text, font_size, x, y, col, font, bounds) in self.texts.iter().chain(self.popover_texts.iter()) {
            pc.text_with(
                text.clone(),
                *x,
                *y,
                *font_size,
                [
                    (col[0] * 255.0) as u8,
                    (col[1] * 255.0) as u8,
                    (col[2] * 255.0) as u8,
                ],
                font.clone(),
                *bounds,
            );
        }
        Some(pc.finish())
    }

    fn display_list_text(&self) -> bool {
        true
    }

    fn ui_context(&self) -> Option<&cce_ui::context::UiContext> {
        Some(&self.ui_context)
    }

    // The engine ticks the exposed context each loop — this is what drives the
    // dropdown expand/contract animation frames.
    fn ui_context_mut(&mut self) -> Option<&mut cce_ui::context::UiContext> {
        Some(&mut self.ui_context)
    }

    fn clear_color(&self) -> [f32; 4] {
        [0.039, 0.102, 0.055, 1.0]
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

    fn tick_internal(&mut self, dt: f32) -> bool {
        let mut needs_redraw = false;
        if hover_animation::tick(dt) {
            needs_redraw = true;
        }
        if self.ui_context.tick(dt) {
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
        while let Ok(s) = self.rx_network.try_recv() {
            network::update(&mut self.app.network, network::NetworkMessage::Refreshed(s));
            if self.app.current_page == Page::Network {
                self.needs_rebuild = true;
            }
        }
        while let Ok(s) = self.rx_timers.try_recv() {
            pages::timers::update(&mut self.app.timers, pages::timers::TimersMessage::Refreshed(s));
            if self.app.current_page == Page::Timers {
                self.needs_rebuild = true;
            }
        }
        while let Ok(s) = self.rx_bluetooth.try_recv() {
            pages::bluetooth::update(&mut self.app.bluetooth, pages::bluetooth::BluetoothMessage::Refreshed(s));
            if self.app.current_page == Page::Bluetooth {
                self.needs_rebuild = true;
            }
        }
        while let Ok(s) = self.rx_power.try_recv() {
            pages::power::update(&mut self.app.power, pages::power::PowerMessage::Refreshed(s));
            if self.app.current_page == Page::Power {
                self.needs_rebuild = true;
            }
        }
        while let Ok(s) = self.rx_system.try_recv() {
            system_info::update(&mut self.app.system_info, system_info::SystemMessage::Refreshed(s), &mut self.ui_context);
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
            pages::notifications::update(&mut self.app.notifications, pages::notifications::NotificationsMessage::Refreshed(s));
            if self.app.current_page == Page::Notifications {
                self.needs_rebuild = true;
            }
        }
        while let Ok(s) = self.rx_browser.try_recv() {
            pages::browser::update(&mut self.app.browser, pages::browser::BrowserMessage::Refreshed(s));
            if self.app.current_page == Page::Browser {
                self.needs_rebuild = true;
            }
        }
        while let Ok(s) = self.rx_default_apps.try_recv() {
            pages::default_apps::update(&mut self.app.default_apps, pages::default_apps::DefaultAppsMessage::Refreshed(s));
            if self.app.current_page == Page::DefaultApps {
                self.needs_rebuild = true;
            }
        }
        while let Ok(s) = self.rx_services.try_recv() {
            services::update(&mut self.app.services, services::ServicesMessage::Refreshed(s));
            if self.app.current_page == Page::Services {
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
            AppAction::Network(m) => network::update(&mut self.app.network, m.clone()),
            AppAction::Bluetooth(m) => pages::bluetooth::update(&mut self.app.bluetooth, m.clone()),
            AppAction::Power(m) => pages::power::update(&mut self.app.power, m.clone()),
            AppAction::Timers(m) => pages::timers::update(&mut self.app.timers, m.clone()),
            AppAction::SystemInfo(m) => system_info::update(&mut self.app.system_info, m.clone(), &mut self.ui_context),
            AppAction::Processes(m) => processes::update(&mut self.app.processes, m.clone()),
            AppAction::Services(m) => services::update(&mut self.app.services, m.clone()),
            AppAction::DefaultApps(m) => pages::default_apps::update(&mut self.app.default_apps, m.clone()),
            AppAction::Notifications(m) => notifications::update(&mut self.app.notifications, m.clone()),
            AppAction::Browser(m) => pages::browser::update(&mut self.app.browser, m.clone()),
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


            AppAction::Accounts(m) => match m {
                pages::accounts::AccountsMessage::GoogleLoginInit => {
                    // One listener at a time: port 36137 is fixed, so a second
                    // flow could only fail to bind and report it as a broken app
                    // rather than "you already have a login in the browser".
                    if self.app.accounts.oauth_listener_running {
                        pages::accounts::update(
                            &mut self.app.accounts,
                            pages::accounts::AccountsMessage::StatusMessage(
                                "A Google sign-in is already waiting on the browser.".to_string(),
                            ),
                        );
                    } else {
                        pages::accounts::update(&mut self.app.accounts, m.clone());
                        let sender = self.sender.clone();
                        tokio::spawn(async move {
                            pages::accounts::run_google_login(sender).await;
                        });
                    }
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
    // cce-ui reports fatal event-loop errors through `log`; without a logger
    // installed they vanish (the cce-terminal connection-death lesson).
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let _guard = rt.enter();

    // One gap everywhere: the wells fill their grid allocations, so the grid
    // gap is the visual section gap — pin it to the page margin so
    // section-to-section and section-to-plate-edge spacing match.
    cce_ui::layout::set_grid_gap(cce_ui::layout::page_margin());

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
