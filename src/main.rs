use cce_ui::widget::Handle;
use cce_ui::cosmic_text::{Buffer, FontSystem};
use cce_ui::scene::layout::Rect;
use cce_ui::scene::paint::{DisplayList, PaintCtx, Prim};

use cce_settings::app::{AppAction, AppState};
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

/// A page button's dispatch clone: the context's copy of a button the page
/// drew, at its rect in the window (cut to its clip and the page's band), with
/// what a click on it does.
pub(crate) struct PageButton {
    pub handle: Handle<cce_ui::widget::Adapted<cce_ui::widget::Button>>,
    pub action: AppAction,
    /// Whether the pointer was over it at the rebuild that drew its face —
    /// the move handler rebuilds when this goes stale, which is how a page
    /// button lights under the pointer.
    pub hovered: bool,
}

/// The page's geometry, emitted under the scroll (`dy`): its fills cut to
/// `fill_band`, everything else (carves, plates) to `view`. The two are the
/// same rect but while the search band is open — the page view then stops
/// 42px higher, and the strip between kept showing the page's fills (and its
/// text) but none of its carves, as it always has.
fn emit_page_geo(pc: &mut PaintCtx, list: &DisplayList, dy: f32, fill_band: Rect, view: Rect) {
    if fill_band == view {
        pc.clip(view, |pc| pc.append_translated(list, 0.0, dy));
        return;
    }
    for item in &list.items {
        let clip = match item.prim {
            Prim::Quad { .. } | Prim::RoundedRect { .. } => fill_band,
            _ => view,
        };
        pc.clip(clip, |pc| pc.append_items(std::iter::once(item.clone().translated(0.0, dy))));
    }
}

static INITIAL_PAGE_INDEX: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

struct SystemInterface {
    app: AppState,
    font_system: FontSystem,
    /// What the last rebuild recorded, emitted every frame until the next
    /// (`renderer::CachedFrame`): the page in page coordinates, under the
    /// scroll; the popovers in window coordinates.
    frame: renderer::CachedFrame,
    page_buttons: Vec<PageButton>,

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
    rx_accounts: std::sync::mpsc::Receiver<pages::accounts::AccountsSnapshot>,
    tx_backup: std::sync::mpsc::Sender<pages::storage::StorageMessage>,
    rx_backup: std::sync::mpsc::Receiver<pages::storage::StorageMessage>,
    rx_packages: std::sync::mpsc::Receiver<pages::packages::PackagesState>,
    tx_update: std::sync::mpsc::Sender<pages::packages::PackagesMessage>,
    rx_update: std::sync::mpsc::Receiver<pages::packages::PackagesMessage>,


    scale_factor: f64,
    width: u32,
    height: u32,
    needs_rebuild: bool,
    /// The page's DRAWN offset — `page_scroll_motion` glides it (wheel) or
    /// coasts it (trackpad flick), and display_list emits the cached page
    /// under it, so a scroll needs no rebuild; direct writes (thumb drag,
    /// keyboard, search jump, clamp) are adopted by the motion on its next step.
    scroll_y: f32,
    page_scroll_motion: cce_ui::widget::ScrollMotion,
    max_scroll_y: f32,
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
    /// The rect of the focused widget at the start of a view pass: the page's
    /// buttons are per-rebuild clones, so the focused id dies with every
    /// rebuild and the clone at the same rect takes the focus back (lit as it
    /// is collected, focused at the pass's tail).
    refocus_rect: Option<(f32, f32, f32, f32)>,
    /// What the last rebuild's widgets claimed for text input (an editing
    /// TextBox, `cce_ui::text_input::capture`). Claimed again every frame:
    /// a frame that only replays the cached layout paints no widget, and
    /// would otherwise read as the field closing.
    text_claim: Option<[f32; 4]>,
    page_dropdown: Handle<cce_ui::widget::Adapted<cce_ui::widget::input::Dropdown>>,
    // Switcher + Page DISSOLVED (Phase 6u): the current page is app.current_page, page
    // scroll is scroll_y/max_scroll_y, and the page scrollbar is this app-owned widget
    // (rendered into the window assembly, evented directly). content_h feeds it — the
    // window pass reads last frame's value, exactly as the legacy Page did.
    page_scroll_bar: Handle<cce_ui::widget::Adapted<crate::scroll_bar::ScrollBar>>,
    content_h: f32,
    // root plate container + StatusBar DISSOLVED (Phase 6s): the window plate and the status
    // bar are emitted by display_list itself.
    sans_serif_family: String,
    serif_family: String,
    monospace_family: String,
    /// The page on screen, as the page workers see it (`watchers::PageWatch`).
    current_page_shared: tokio::sync::watch::Sender<u8>,
    /// A hash of the last snapshot each page worker sent, so an unchanged
    /// refresh is dropped instead of rebuilding the page (`fresh_snapshot`).
    seen_snapshots: std::collections::HashMap<&'static str, u64>,
    sender: calloop::channel::Sender<AppAction>,
    ui_context: cce_ui::context::UiContext,
    search_open: bool,
    search_query: String,
    search_box: Handle<cce_ui::widget::Adapted<cce_ui::widget::input::TextBox>>,

}

impl cce_ui::engine::Application for SystemInterface {
    type Message = AppAction;

    fn create(sender: cce_ui::engine::AppSender<Self::Message>) -> Self {
        // The app keeps calloop's sender; `AppSender` converts into it.
        let sender: calloop::channel::Sender<Self::Message> = sender.into();
        cce_ui::scale::set_scale_factor(1.0);
        // The context owns the widgets; the app keeps their handles.
        let mut ui_context = cce_ui::context::UiContext::new();
        let app = AppState::new(&mut ui_context);

        // ── Background refresh channels ──
        let initial_page_idx = INITIAL_PAGE_INDEX.load(std::sync::atomic::Ordering::SeqCst);
        let (current_page_shared, page_rx) = tokio::sync::watch::channel(initial_page_idx as u8);
        let wake: cce_settings::watchers::Wake = {
            let sender = sender.clone();
            std::sync::Arc::new(move || {
                let _ = sender.send(AppAction::Wake);
            })
        };

        let (watchers, tx_backup, rx_backup, tx_update, rx_update) =
            cce_settings::watchers::spawn_all(page_rx, wake);

        let (sans_family, serif_family, monospace_family, _) = cce_ui::layout::read_preferred_fonts();

        let pages_names = Page::ALL.iter().map(|p| p.label().to_string()).collect::<Vec<_>>();
        let page_dropdown = cce_ui::widget::input::Dropdown::new(pages_names, initial_page_idx)
            .with_open_upward(true)
            .with_auto_width(true)
            // The open menu is the page list alone, sat where the trigger was:
            // the current page already reads blue in the list, so the trigger
            // band repeating its title under the rows was noise.
            .with_menu_replaces_trigger(true);
        let sidebar_width = 0.0f32;

        let mut app_state = app;
        app_state.current_page = Page::ALL[initial_page_idx];

        let font_system = cce_ui::create_font_system();

        let search_box = ui_context.insert(
            cce_ui::widget::input::TextBox::new(String::new())
                .with_placeholder("Search sections & parameters...")
                .with_draw_bg_border(false),
        );
        let mut this = Self {
            app: app_state,
            font_system,
            frame: renderer::CachedFrame::default(),
            page_buttons: Vec::new(),
            sidebar_width,
            header_height: 0.0,
            // The page switcher at the toolkit's dropdown height, inset 3px
            // top and bottom: the bar grows with the control, not the reverse.
            status_height: cce_ui::layout::dropdown_height() + 6.0,
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
            page_scroll_motion: cce_ui::widget::ScrollMotion::new(),
            max_scroll_y: 0.0,
            focused_section: None,
            laid_out_page: None,
            refocus_rect: None,
            text_claim: None,
            page_dropdown: ui_context.insert(page_dropdown),
            page_scroll_bar: ui_context.insert(crate::scroll_bar::ScrollBar::new()),
            content_h: 0.0,
            sans_serif_family: sans_family,
            serif_family,
            monospace_family,
            current_page_shared,
            seen_snapshots: std::collections::HashMap::new(),
            sender,
            ui_context,
            search_open: false,
            search_query: String::new(),
            search_box,
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
        // Wakes the loop so `tick` drains the workers' channels; whether
        // anything is redrawn is `poll_background_updates`' call.
        if let AppAction::Wake = msg {
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
        // rebuild_layout records the page (in page coordinates) and the popovers into
        // `self.frame`; this emits them, the page under the scroll, around what is
        // drawn fresh every frame (the scrollbar, the window plate, the chrome).
        let (width, height) = (size.width, size.height);
        if self.needs_rebuild || self.ui_context.is_dirty() || self.width != width as u32 || self.height != height as u32 || self.scale_factor != scale {
            self.width = width as u32;
            self.height = height as u32;
            self.scale_factor = scale;
            cce_ui::scale::set_scale_factor(scale as f32);
            self.rebuild_layout(width, height);
        }
        let mut pc = PaintCtx::new();

        // The page scrollbar straddles the window plate (the designer
        // parameter-pane treatment) and is emitted fresh EVERY frame — never
        // baked into the rebuilt layout, so the thumb tracks the wheel fast
        // path's scrolls, which shift cached geometry without a rebuild and
        // used to leave the bar frozen until scrolling stopped. Sync first:
        // a thumb drag drives the page, anything else drives the thumb.
        if self.ui_context[self.page_scroll_bar].dragging {
            self.scroll_y = self.ui_context[self.page_scroll_bar].scroll_y;
        } else {
            self.ui_context[self.page_scroll_bar].scroll_y = self.scroll_y;
        }
        let page_bar = {
            use cce_ui::widget::WidgetHost;
            let (bx, by, bw, bh) = self.ui_context[self.page_scroll_bar].rect();
            self.ui_context[self.page_scroll_bar]
                .layer_quads(Rect { x: bx, y: by, width: bw, height: bh })
        };
        // The idle copy: under the translucent window plate, every frame, so
        // the bar reads as sunk INTO the window rather than gone, and the
        // plate occludes it from input. Track and thumb are pills (the
        // designer look). The fore copy fades in over it, below.
        for &(r, c) in &page_bar {
            pc.rounded_rect(r, r.width.min(r.height) * 0.5, (true, true, true, true), c);
        }
        // The frameless inner lists' idle copies go under the plate too: they
        // sit on wells carved into it, so this is the plate behind them. In
        // page coordinates, so shifted by the page scroll and clipped to the
        // page view as the wells are. Their fore copies are the pages'.
        {
            let view = self.page_view(width, height);
            let scroll_y = self.scroll_y;
            for list in self.app.get_current_page().frameless_lists() {
                let mut shown = list.clone();
                shown.y -= scroll_y;
                shown.viewport_y -= scroll_y;
                pc.clip(view, |pc| shown.push_scrollbar_prims(pc));
            }
        }

        // One glass slab (data-editor's idiom): the root plate, with the status
        // bar carved into it as a step — everything else paints on top. The
        // standard spec (cce-ui `PlateSpec::window`) in this app's own dark
        // green tint at the DE root opacity: a deliberate deviation from the
        // DE root colour, and the one thing here that is not the standard.
        {
            let mut plate = cce_ui::color::to_linear([0x0a as f32 / 255.0, 0x1a as f32 / 255.0, 0x0e as f32 / 255.0, 1.0]);
            if plate[3] > 0.001 {
                plate[3] = cce_ui::color::root_plate_opacity();
            }
            pc.plate_spec(
                &cce_ui::scene::paint::PlateSpec::window(width, height)
                    .with_material(cce_ui::scene::Material::opaque(plate)),
            );
            let sb_h = self.status_height;
            let depth = cce_ui::layout::bar_wall_width().min(sb_h * 0.6);
            pc.recess_edges(
                Rect { x: 0.0, y: height - sb_h, width, height: sb_h },
                (0.0, 0.0, 0.0, 0.0),
                depth,
                (true, false, false, false),
            );
        }

        // The page as the last rebuild recorded it, under the scroll: its
        // fills and carves in the widgets' OWN order (a TextBox's selection
        // and caret land on its well, not under its walls), the search's
        // highlights, the page buttons' faces and plates.
        let view = self.page_view(width, height);
        // The page's band above the status bar, what its fills and text were
        // always cut to (the page view also stops above the search band).
        let page_band = Rect { x: 0.0, y: 0.0, width, height: (height - self.status_height).max(0.0) };
        let dy = -self.scroll_y;
        emit_page_geo(&mut pc, &self.frame.geo, dy, page_band, view);
        // The search band, over the page's foot while open.
        if self.search_open {
            let (x, w) = (self.sidebar_width, width - self.sidebar_width);
            pc.quad(Rect { x, y: height - 42.0, width: w, height: 42.0 }, [0.08, 0.08, 0.12, 1.0]);
            pc.quad(Rect { x, y: height - 42.0, width: w, height: 1.0 }, [0.18, 0.18, 0.24, 1.0]);
        }
        // The section wells, carved after the page's flat quads so the walls shade
        // the fills they cross (the designer relief order), clipped to the page
        // viewport so a scrolled-off well can't shade the status bar or search row.
        if !self.frame.wells.is_empty() {
            pc.clip(view, |pc| pc.append_translated(&self.frame.wells, 0.0, dy));
        }
        // The page's glyphs — button icon faces and the page's own — over its
        // quads and carves, under the page viewport and each under the clip
        // it was placed with, which is what cuts a half-scrolled list row's
        // glyph at the list edge.
        if !self.frame.icons.is_empty() {
            pc.clip(view, |pc| pc.append_translated(&self.frame.icons, 0.0, dy));
        }

        // The window chrome, as it paints itself: the page dropdown in the status bar
        // and, while open, the search box in its band.
        cce_ui::widget::painter::paint_root_into(&self.ui_context, &self.ui_context[self.page_dropdown], &mut pc);
        if self.search_open {
            cce_ui::widget::painter::paint_root_into(&self.ui_context, &self.ui_context[self.search_box], &mut pc);
        }

        // The page scrollbar's fore copy: over the page content at the
        // raise's fade, while a scroll or drag holds it up and as it sinks
        // (popovers still stack above it).
        let fade = self.ui_context[self.page_scroll_bar].fade();
        if fade > 0.001 {
            for &(r, mut c) in &page_bar {
                c[3] *= fade;
                pc.rounded_rect(r, r.width.min(r.height) * 0.5, (true, true, true, true), c);
            }
        }

        // The popovers over everything on the page — their plates and rows in
        // the widget's own order (the hovered row a Dropdown draws after its
        // plate lands on the frosted face), then their glyphs.
        pc.append_translated(&self.frame.popover_geo, 0.0, 0.0);
        pc.append_translated(&self.frame.popover_icons, 0.0, 0.0);
        // The context menu last, over everything: the toolkit's lit plate,
        // its rows and its labels in the menu font, with the row marks and
        // page chevrons as cce-icons glyphs.
        cce_ui::widget::context_menu::paint_with_labels(&mut pc);
        // The page's text, under the scroll and cut to the page's band, then
        // the popovers' (the glyph pass draws all text after all geometry).
        pc.clip(page_band, |pc| pc.append_translated(&self.frame.text, 0.0, dy));
        pc.append_translated(&self.frame.popover_text, 0.0, 0.0);
        if let Some([x, y, w, h]) = self.text_claim {
            cce_ui::text_input::claim(x, y, w, h);
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

    // Page widgets are laid out unscrolled and drawn `scroll_y` higher (the
    // popover replay in `rebuild_layout` subtracts it); the chrome dropdown
    // is in window coords already.
    fn popover_offset(&self, id: cce_ui::widget::WidgetId) -> (f32, f32) {
        if id == self.page_dropdown.id() {
            (0.0, 0.0)
        } else {
            (0.0, -self.scroll_y)
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

    /// Tab walks the page's plates and wells (cce-ui's navigation in plate
    /// terms); the section chords in input.kdl keep their own walk.
    fn plate_navigation(&self) -> bool {
        true
    }

    /// The geometry is cached until the next rebuild — a moved focus ring needs
    /// one. The view pass itself carries the focus across the button clones it
    /// makes (see `refocus_rect`), on every rebuild.
    fn focus_stepped(&mut self) {
        self.needs_rebuild = true;
    }

    fn handle_key_input(&mut self, event: &cce_ui::widget::KeyEvent, needs_rebuild: &mut bool) -> Option<Self::Message> {
        if self.handle_key_input_internal(event) {
            *needs_rebuild = true;
        }
        None
    }
}

impl SystemInterface {
    /// The page viewport in window coords: everything under the header and
    /// above the status bar (and the search row while it is open).
    fn page_view(&self, width: f32, height: f32) -> cce_ui::scene::layout::Rect {
        use cce_ui::scene::layout::Rect;
        Rect {
            x: self.sidebar_width,
            y: self.header_height,
            width: width - self.sidebar_width,
            height: (height - self.header_height - self.status_height
                - if self.search_open { 42.0 } else { 0.0 }).max(0.0),
        }
    }


    fn tick_internal(&mut self, dt: f32) -> bool {
        let mut needs_redraw = false;
        // Raise/sink upkeep for the page scrollbar: true while the post-scroll
        // hold runs (keeps frames coming so the sink actually renders) and on
        // the raised flip itself. A redraw re-emits the bar at its new depth —
        // no layout rebuild needed, display_list draws it fresh each frame.
        if self.ui_context[self.page_scroll_bar].tick_activity(dt) {
            needs_redraw = true;
        }
        // The page's own wheel glide / flick coast: moves the cached page
        // like the wheel fast path, no rebuild.
        if self.tick_page_scroll(dt) {
            needs_redraw = true;
        }
        // The current page's inner lists (their glide/coast lives in the
        // region's tick): a moved list re-lays the page out.
        if self.app.get_current_page_mut().tick(dt) {
            needs_redraw = true;
            self.needs_rebuild = true;
        }
        if self.ui_context.tick(dt) {
            needs_redraw = true;
            self.needs_rebuild = true;
        }

        needs_redraw
    }

    /// Advance the page's wheel glide / flick coast; true while the offset is
    /// moving, so the frame loop keeps drawing until it settles.
    fn tick_page_scroll(&mut self, dt: f32) -> bool {
        use cce_ui::widget::Bounds;
        self.page_scroll_motion.reconcile(0.0, self.scroll_y);
        if !self.page_scroll_motion.is_animating() {
            return false;
        }
        let moved = self.page_scroll_motion.tick(dt, Bounds::max(0.0), Bounds::max(self.max_scroll_y));
        if moved {
            self.shift_page_to(self.page_scroll_motion.y.pos());
        }
        moved || self.page_scroll_motion.is_animating()
    }

    /// Whether a page worker's snapshot differs from the last one it sent.
    /// The workers resend the whole state on every period whether or not
    /// anything moved; applying an identical one changed nothing on screen
    /// but rebuilt the page and redrew the window — the settings app sat at
    /// ~3 fps at rest on its default page. Processes is not checked: its
    /// refresh also settles pending kills (`killing.clear()`), and its CPU
    /// column moves every time anyway.
    fn fresh_snapshot<T: std::fmt::Debug>(&mut self, key: &'static str, snapshot: &T) -> bool {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        format!("{snapshot:?}").hash(&mut h);
        let hash = h.finish();
        self.seen_snapshots.insert(key, hash) != Some(hash)
    }

    fn poll_background_updates(&mut self) {
        use pages::*;
        while let Ok(s) = self.rx_audio.try_recv() {
            if !self.fresh_snapshot("audio", &s) {
                continue;
            }
            audio::update(&mut self.app.audio, audio::AudioMessage::Refreshed(s), &mut self.ui_context);
            if self.app.current_page == Page::Audio {
                self.needs_rebuild = true;
            }
        }
        while let Ok(s) = self.rx_network.try_recv() {
            if !self.fresh_snapshot("network", &s) {
                continue;
            }
            network::update(&mut self.app.network, network::NetworkMessage::Refreshed(Box::new(s)), &mut self.ui_context);
            if self.app.current_page == Page::Network {
                self.needs_rebuild = true;
            }
        }
        while let Ok(s) = self.rx_timers.try_recv() {
            if !self.fresh_snapshot("timers", &s) {
                continue;
            }
            pages::timers::update(&mut self.app.timers, pages::timers::TimersMessage::Refreshed(s), &mut self.ui_context);
            if self.app.current_page == Page::Timers {
                self.needs_rebuild = true;
            }
        }
        while let Ok(s) = self.rx_bluetooth.try_recv() {
            if !self.fresh_snapshot("bluetooth", &s) {
                continue;
            }
            pages::bluetooth::update(&mut self.app.bluetooth, pages::bluetooth::BluetoothMessage::Refreshed(s), &mut self.ui_context);
            if self.app.current_page == Page::Bluetooth {
                self.needs_rebuild = true;
            }
        }
        while let Ok(s) = self.rx_power.try_recv() {
            if !self.fresh_snapshot("power", &s) {
                continue;
            }
            pages::power::update(&mut self.app.power, pages::power::PowerMessage::Refreshed(Box::new(s)), &mut self.ui_context);
            if self.app.current_page == Page::Power {
                self.needs_rebuild = true;
            }
        }
        while let Ok(s) = self.rx_system.try_recv() {
            if !self.fresh_snapshot("system", &s) {
                continue;
            }
            system_info::update(&mut self.app.system_info, system_info::SystemMessage::Refreshed(s), &mut self.ui_context);
            if self.app.current_page == Page::System {
                self.needs_rebuild = true;
            }
        }
        while let Ok(s) = self.rx_processes.try_recv() {
            processes::update(&mut self.app.processes, processes::ProcessesMessage::Refreshed(Box::new(s)));
            if self.app.current_page == Page::Processes {
                self.needs_rebuild = true;
            }
        }
        while let Ok(s) = self.rx_storage.try_recv() {
            if !self.fresh_snapshot("storage", &s) {
                continue;
            }
            storage::update(&mut self.app.storage, storage::StorageMessage::Refreshed(s), &mut self.ui_context);
            if self.app.current_page == Page::Storage {
                self.needs_rebuild = true;
            }
        }
        while let Ok(s) = self.rx_notifications.try_recv() {
            if !self.fresh_snapshot("notifications", &s) {
                continue;
            }
            pages::notifications::update(&mut self.app.notifications, pages::notifications::NotificationsMessage::Refreshed(s), &mut self.ui_context);
            if self.app.current_page == Page::Notifications {
                self.needs_rebuild = true;
            }
        }
        while let Ok(s) = self.rx_browser.try_recv() {
            if !self.fresh_snapshot("browser", &s) {
                continue;
            }
            pages::browser::update(&mut self.app.browser, pages::browser::BrowserMessage::Refreshed(s), &mut self.ui_context);
            if self.app.current_page == Page::Browser {
                self.needs_rebuild = true;
            }
        }
        while let Ok(s) = self.rx_default_apps.try_recv() {
            if !self.fresh_snapshot("default_apps", &s) {
                continue;
            }
            pages::default_apps::update(&mut self.app.default_apps, pages::default_apps::DefaultAppsMessage::Refreshed(s), &mut self.ui_context);
            if self.app.current_page == Page::DefaultApps {
                self.needs_rebuild = true;
            }
        }
        while let Ok(s) = self.rx_services.try_recv() {
            if !self.fresh_snapshot("services", &s) {
                continue;
            }
            services::update(&mut self.app.services, services::ServicesMessage::Refreshed(s), &mut self.ui_context);
            if self.app.current_page == Page::Services {
                self.needs_rebuild = true;
            }
        }

        while let Ok(s) = self.rx_accounts.try_recv() {
            if !self.fresh_snapshot("accounts", &s) {
                continue;
            }
            accounts::update(&mut self.app.accounts, accounts::AccountsMessage::Refreshed(s), &mut self.ui_context);
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
            if !self.fresh_snapshot("packages", &s) {
                continue;
            }
            pages::packages::update(&mut self.app.packages, pages::packages::PackagesMessage::Refreshed(Box::new(s)), &mut self.ui_context);
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
        // An action may change page state ahead of the system (a Wi-Fi
        // toggle flips `wifi_enabled` before nmcli answers). If it fails, the
        // next refresh equals the last one sent — and must still be applied
        // to put the page right, so forget what was seen.
        if !matches!(action, AppAction::Exit | AppAction::Wake) {
            self.seen_snapshots.clear();
        }
        if matches!(action, AppAction::Accounts(_)) {
            accounts::invalidate_keyring_probe();
        }
        match action {
            AppAction::Exit | AppAction::Wake => {}
            AppAction::Audio(m) => audio::update(&mut self.app.audio, m.clone(), &mut self.ui_context),
            AppAction::Network(m) => network::update(&mut self.app.network, m.clone(), &mut self.ui_context),
            AppAction::Bluetooth(m) => pages::bluetooth::update(&mut self.app.bluetooth, m.clone(), &mut self.ui_context),
            AppAction::Power(m) => pages::power::update(&mut self.app.power, m.clone(), &mut self.ui_context),
            AppAction::Timers(m) => pages::timers::update(&mut self.app.timers, m.clone(), &mut self.ui_context),
            AppAction::SystemInfo(m) => system_info::update(&mut self.app.system_info, m.clone(), &mut self.ui_context),
            AppAction::Processes(m) => processes::update(&mut self.app.processes, m.clone()),
            AppAction::Services(m) => services::update(&mut self.app.services, m.clone(), &mut self.ui_context),
            AppAction::DefaultApps(m) => pages::default_apps::update(&mut self.app.default_apps, m.clone(), &mut self.ui_context),
            AppAction::Notifications(m) => notifications::update(&mut self.app.notifications, m.clone(), &mut self.ui_context),
            AppAction::Browser(m) => pages::browser::update(&mut self.app.browser, m.clone(), &mut self.ui_context),
            AppAction::Storage(m) => match m {
                pages::storage::StorageMessage::StartBackup => {
                    pages::storage::update(&mut self.app.storage, pages::storage::StorageMessage::StartBackup, &mut self.ui_context);
                    let tx = self.tx_backup.clone();
                    let wake = self.sender.clone();
                    tokio::spawn(async move {
                        let res = pages::storage::run_backup().await;
                        let _ = tx.send(pages::storage::StorageMessage::BackupFinished(res));
                        let _ = wake.send(AppAction::Wake);
                    });
                }
                _ => pages::storage::update(&mut self.app.storage, m.clone(), &mut self.ui_context),
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
                            &mut self.ui_context,
                        );
                    } else {
                        pages::accounts::update(&mut self.app.accounts, m.clone(), &mut self.ui_context);
                        let sender = self.sender.clone();
                        tokio::spawn(async move {
                            pages::accounts::run_google_login(sender).await;
                        });
                    }
                }
                _ => pages::accounts::update(&mut self.app.accounts, m.clone(), &mut self.ui_context),
            },
            AppAction::Packages(m) => match m {
                pages::packages::PackagesMessage::StartUpdate => {
                    if self.app.packages.busy() {
                        return;
                    }
                    pages::packages::update(&mut self.app.packages, pages::packages::PackagesMessage::StartUpdate, &mut self.ui_context);
                    let tx = self.tx_update.clone();
                    let wake = self.sender.clone();
                    tokio::spawn(async move {
                        let res = pages::packages::run_update().await;
                        let _ = tx.send(pages::packages::PackagesMessage::UpdateFinished(res));
                        let _ = wake.send(AppAction::Wake);
                    });
                }
                pages::packages::PackagesMessage::UpdateFinished(res) => {
                    pages::packages::update(&mut self.app.packages, m.clone(), &mut self.ui_context);
                    if res.is_ok() {
                        let tx = self.tx_update.clone();
                        let wake = self.sender.clone();
                        tokio::spawn(async move {
                            let new_state = pages::packages::fetch_packages_state().await;
                            let _ = tx.send(pages::packages::PackagesMessage::Refreshed(Box::new(new_state)));
                            let _ = wake.send(AppAction::Wake);
                        });
                    }
                }
                pages::packages::PackagesMessage::SelectPackage(Some(ref name)) => {
                    let name_clone = name.clone();
                    let is_installed = self.app.packages.active_tab == pages::packages::PackageTab::Installed;
                    pages::packages::update(&mut self.app.packages, m.clone(), &mut self.ui_context);
                    let tx = self.tx_update.clone();
                    let wake = self.sender.clone();
                    tokio::spawn(async move {
                        let res = pages::packages::fetch_package_info(name_clone.clone(), is_installed).await;
                        let _ = tx.send(pages::packages::PackagesMessage::InfoFetched(name_clone, res));
                        let _ = wake.send(AppAction::Wake);
                    });
                }
                pages::packages::PackagesMessage::SelectAndScrollPackage(ref name) => {
                    let name_clone = name.clone();
                    pages::packages::update(&mut self.app.packages, m.clone(), &mut self.ui_context);
                    let tx = self.tx_update.clone();
                    let wake = self.sender.clone();
                    tokio::spawn(async move {
                        let res = pages::packages::fetch_package_info(name_clone.clone(), true).await;
                        let _ = tx.send(pages::packages::PackagesMessage::InfoFetched(name_clone, res));
                        let _ = wake.send(AppAction::Wake);
                    });
                }
                pages::packages::PackagesMessage::PreviewRemoval(ref targets) => {
                    if !targets.is_empty() && !self.app.packages.busy() {
                        pages::packages::update(&mut self.app.packages, m.clone(), &mut self.ui_context);
                        let targets = targets.clone();
                        let tx = self.tx_update.clone();
                        let wake = self.sender.clone();
                        tokio::spawn(async move {
                            let res = pages::packages::preview_removal(targets.clone()).await;
                            let _ = tx.send(pages::packages::PackagesMessage::RemovalPreviewed(targets, res));
                            let _ = wake.send(AppAction::Wake);
                        });
                    }
                }
                pages::packages::PackagesMessage::StartUninstall(ref targets) => {
                    if !targets.is_empty() && !self.app.packages.busy() {
                        pages::packages::update(&mut self.app.packages, m.clone(), &mut self.ui_context);
                        let targets = targets.clone();
                        let tx = self.tx_update.clone();
                        let wake = self.sender.clone();
                        tokio::spawn(async move {
                            let res = pages::packages::run_uninstall(targets.clone()).await;
                            let _ = tx.send(pages::packages::PackagesMessage::UninstallFinished(targets, res));
                            let _ = wake.send(AppAction::Wake);
                        });
                    }
                }
                pages::packages::PackagesMessage::SetInstallReason(ref targets, explicit) => {
                    if !targets.is_empty() && !self.app.packages.busy() {
                        pages::packages::update(&mut self.app.packages, m.clone(), &mut self.ui_context);
                        let (targets, explicit) = (targets.clone(), *explicit);
                        let tx = self.tx_update.clone();
                        let wake = self.sender.clone();
                        tokio::spawn(async move {
                            let res = pages::packages::run_set_install_reason(targets.clone(), explicit).await;
                            let _ = tx.send(pages::packages::PackagesMessage::InstallReasonSet(targets, explicit, res));
                            let _ = wake.send(AppAction::Wake);
                        });
                    }
                }
                // Both outcomes refresh, failure included: the list (and the
                // orphan set, which either can change) is pacman's to report.
                pages::packages::PackagesMessage::UninstallFinished(..)
                | pages::packages::PackagesMessage::InstallReasonSet(..) => {
                    pages::packages::update(&mut self.app.packages, m.clone(), &mut self.ui_context);
                    let tx = self.tx_update.clone();
                    let wake = self.sender.clone();
                    // The details pane shows the install reason; re-read it.
                    let reread = self.app.packages.selected_package.clone();
                    tokio::spawn(async move {
                        let new_state = pages::packages::fetch_packages_state().await;
                        let _ = tx.send(pages::packages::PackagesMessage::Refreshed(Box::new(new_state)));
                        if let Some(name) = reread {
                            let res = pages::packages::fetch_package_info(name.clone(), true).await;
                            let _ = tx.send(pages::packages::PackagesMessage::InfoFetched(name, res));
                        }
                        let _ = wake.send(AppAction::Wake);
                    });
                }
                _ => pages::packages::update(&mut self.app.packages, m.clone(), &mut self.ui_context),
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
    // gap IS the visual section gap. The sections stand on the root plate,
    // so it is the root rung's sibling gap — `PageFlow::init` reads the
    // toolkit's grid_gap, hence the set here.
    cce_ui::layout::set_grid_gap(cce_ui::layout::root_plate_gap());

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

