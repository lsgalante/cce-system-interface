use cce_ui::widget::{hover_animation, Element};
use glyphon::{Buffer, FontSystem};

use cce_settings::app::{AppAction, AppState};
use cce_settings::pages::{self, Page};
mod input_handler;
mod renderer;

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
    page_buttons: Vec<(cce_ui::widget::Adapted<cce_ui::widget::Button>, AppAction)>,

    sidebar_width: f32,
    header_height: f32,
    status_height: f32,

    cursor_x: f32,
    cursor_y: f32,

    rx_audio: std::sync::mpsc::Receiver<pages::audio::AudioState>,
    rx_network: std::sync::mpsc::Receiver<pages::network::NetworkState>,
    pub rx_processes: std::sync::mpsc::Receiver<pages::processes::ProcessesState>,
    rx_system: std::sync::mpsc::Receiver<pages::system_info::SystemInfo>,
    rx_storage: std::sync::mpsc::Receiver<pages::storage::StorageState>,
    rx_notifications: std::sync::mpsc::Receiver<pages::notifications::NotificationsConfig>,
    rx_services: std::sync::mpsc::Receiver<Vec<pages::processes::ServiceInfo>>,
    rx_fonts: std::sync::mpsc::Receiver<pages::fonts::FontsState>,
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
    page_dropdown: cce_ui::widget::Adapted<cce_ui::widget::input::Dropdown>,
    switcher: cce_ui::widget::Adapted<cce_ui::widget::Switcher>,
    pages: Vec<cce_ui::widget::Page>,
    // Root Backplate + StatusBar DISSOLVED (Phase 6s): the window plate and the status
    // bar are emitted as tuples in rebuild_layout; this is the bar's text.
    status_text: String,
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
        let app = AppState {
            fonts: pages::fonts::read_typeface_config(),
            ..Default::default()
        };

        // ── Background refresh channels ──
        let initial_page_idx = INITIAL_PAGE_INDEX.load(std::sync::atomic::Ordering::SeqCst);
        let current_page_shared = std::sync::Arc::new(std::sync::atomic::AtomicU8::new(initial_page_idx as u8));

        let (watchers, tx_backup, rx_backup, tx_update, rx_update) =
            cce_settings::watchers::spawn_all(current_page_shared.clone());

        let (sans_family, serif_family, monospace_family, _, _, _, _) = pages::fonts::read_preferred_fonts();

        let pages_names = Page::ALL.iter().map(|p| p.label().to_string()).collect::<Vec<_>>();
        let page_dropdown = cce_ui::widget::input::Dropdown::new(pages_names, initial_page_idx)
            .with_open_upward(true)
            .with_auto_width(true);
        let sidebar_width = 0.0f32;

        let switcher = cce_ui::widget::Switcher::new(sidebar_width, 0.0, 820.0 - sidebar_width, 680.0);
        let mut pages = Vec::new();
        for page in Page::ALL.iter() {
            let mut page_widget = cce_ui::widget::Page::new(sidebar_width, 0.0, 820.0 - sidebar_width, 680.0)
                .with_label(page.label());
            if *page == Page::System {
                page_widget.layout = Box::new(cce_ui::widget::AdaptiveGridLayout {
                    min_col_width: 320.0,
                    gap: 20.0,
                    padding_x: 10.0,
                    padding_y: 10.0,
                    grid: None,
                });
            }
            pages.push(page_widget);
        }

        let mut app_state = app;
        app_state.current_page = Page::ALL[initial_page_idx];

        let font_system = cce_ui::create_font_system();

        let mut this = Self {
            app: app_state,
            font_system,
            widgets: Vec::new(),
            texts: Vec::new(),
            page_buttons: Vec::new(),
            sidebar_width,
            header_height: 0.0,
            status_height: 24.0,
            cursor_x: 0.0,
            cursor_y: 0.0,
            rx_audio: watchers.rx_audio,
            rx_network: watchers.rx_network,
            rx_processes: watchers.rx_processes,
            rx_system: watchers.rx_system,
            rx_storage: watchers.rx_storage,
            rx_notifications: watchers.rx_notifications,
            rx_services: watchers.rx_services,
            rx_fonts: watchers.rx_fonts,
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
            page_dropdown,
            switcher,
            pages,
            status_text: String::new(),
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

        for page in &mut this.pages {
            this.switcher.add_child(page.as_ptr(), &mut this.ui_context);
        }

        // Root Backplate dissolved: switcher/dropdown register parentless via
        // render_widget in rebuild_layout.
        this.update_status_text();

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
        for w in &self.widgets {
            let color = if w.hovering { w.hover_color } else { w.color };
            let rect = Rect { x: w.x, y: w.y, width: w.w, height: w.h };
            if w.radius > 0.1 {
                pc.rounded_rect(rect, w.radius, w.corners, color);
            } else {
                pc.quad(rect, color);
            }
        }
        cce_ui::widget::hover_animation::post_render_check();
        if let Some((qx, qy, qw, qh, qc)) = cce_ui::widget::hover_animation::get_quad() {
            pc.quad(Rect { x: qx, y: qy - self.scroll_y, width: qw, height: qh }, qc);
        }
        for (text, font_size, x, y, col, font, bounds) in &self.texts {
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
            if self.app.current_page == Page::Radios {
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
        while let Ok(s) = self.rx_fonts.try_recv() {
            self.sans_serif_family = s.sans_serif.clone();
            self.serif_family = s.serif.clone();
            self.monospace_family = s.monospace.clone();
            fonts::update(&mut self.app.fonts, fonts::FontsMessage::TypefaceRefreshed(s));
            if self.app.current_page == Page::Fonts {
                self.needs_rebuild = true;
            }
        }
        while let Ok(s) = self.rx_services.try_recv() {
            processes::update(&mut self.app.processes, processes::ProcessesMessage::ServicesRefreshed(s));
            if self.app.current_page == Page::Processes {
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
            AppAction::Radios(m) => network::update(&mut self.app.network, m.clone()),
            AppAction::SystemInfo(m) => system_info::update(&mut self.app.system_info, m.clone(), &mut self.ui_context),
            AppAction::Processes(m) => processes::update(&mut self.app.processes, m.clone()),
            AppAction::Notifications(m) => notifications::update(&mut self.app.notifications, m.clone()),
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


            AppAction::Fonts(m) => {
                fonts::update(&mut self.app.fonts, m.clone());
                self.sans_serif_family = self.app.fonts.sans_serif.clone();
                self.serif_family = self.app.fonts.serif.clone();
                self.monospace_family = self.app.fonts.monospace.clone();
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

    fn update_status_text(&mut self) {
        let msg = match self.app.current_page {
            Page::Accounts => "Accounts: Manage your online identities, keys, and credentials.",
            Page::Audio => "Audio Settings: Configure volume levels, inputs, and sound options.",
            Page::Fonts => "Fonts Settings: Adjust font family preferences, typography, and scaling.",
            Page::Packages => "Package Manager: Search, install, and update system packages.",
            Page::Processes => "System Monitor: Inspect running tasks, system resources, and services.",
            Page::Notifications => "Notifications: Configure system notifications, sound, and duration.",
            Page::Radios => "Network Settings: Configure wireless networks, radios, and connections.",
            Page::Storage => "Storage Settings: Manage local disks, partition structures, and backup runs.",
            Page::System => "System Settings: System properties, power management, and update checks.",
        };
        self.status_text = msg.to_string();
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
