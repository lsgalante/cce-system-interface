use cce_ui::scene::layout::Rect;
use cce_ui::scene::paint::{DisplayList, PaintCtx, Prim, RenderTarget};
use cce_ui::scene::Material;

use crate::pages::audio;
use crate::pages::bluetooth;
use crate::pages::power;
use crate::pages::default_apps;
use crate::pages::network;
use crate::pages::processes;
use crate::pages::services;
use crate::pages::system_info;
use crate::pages::timers;
use crate::pages::storage;
use crate::pages::accounts;
use crate::pages::packages;
use crate::pages::notifications;
use crate::pages::browser;
use crate::pages::Page;

pub struct AppState {
    pub current_page: Page,
    pub audio: audio::AudioState,
    pub bluetooth: bluetooth::BluetoothState,
    pub power: power::PowerState,
    pub browser: browser::BrowserState,
    pub default_apps: default_apps::DefaultAppsState,
    pub network: network::NetworkState,
    pub processes: processes::ProcessesState,
    pub services: services::ServicesState,
    pub system_info: system_info::SystemState,
    pub timers: timers::TimersState,
    pub storage: storage::StorageState,
    pub accounts: accounts::AccountsState,
    pub packages: packages::PackagesState,
    pub notifications: notifications::NotificationsState,
}

impl AppState {
    /// Every page's state, their widgets inserted into `ctx`.
    pub fn new(ctx: &mut cce_ui::context::UiContext) -> Self {
        Self {
            current_page: Page::ALL[0],
            audio: audio::AudioState::default(),
            bluetooth: bluetooth::BluetoothState::new(ctx),
            power: power::PowerState::new(ctx),
            browser: browser::BrowserState::new(ctx),
            default_apps: default_apps::DefaultAppsState::new(ctx),
            network: network::NetworkState::new(ctx),
            processes: processes::ProcessesState::default(),
            services: services::ServicesState::new(ctx),
            system_info: system_info::SystemState::new(ctx),
            timers: timers::TimersState::new(ctx),
            storage: storage::StorageState::new(ctx),
            accounts: accounts::AccountsState::new(ctx),
            packages: packages::PackagesState::new(ctx),
            notifications: notifications::NotificationsState::new(ctx),
        }
    }
}

impl AppState {
    pub fn get_page(&self, page: Page) -> &dyn crate::pages::AppPage {
        match page {
            Page::Accounts => &self.accounts,
            Page::Audio => &self.audio,
            Page::Bluetooth => &self.bluetooth,
            Page::Power => &self.power,
            Page::Browser => &self.browser,
            Page::DefaultApps => &self.default_apps,
            Page::Packages => &self.packages,
            Page::Processes => &self.processes,
            Page::Services => &self.services,
            Page::Network => &self.network,
            Page::Storage => &self.storage,
            Page::System => &self.system_info,
            Page::Timers => &self.timers,
            Page::Notifications => &self.notifications,
        }
    }

    pub fn get_page_mut(&mut self, page: Page) -> &mut dyn crate::pages::AppPage {
        match page {
            Page::Accounts => &mut self.accounts,
            Page::Audio => &mut self.audio,
            Page::Bluetooth => &mut self.bluetooth,
            Page::Power => &mut self.power,
            Page::Browser => &mut self.browser,
            Page::DefaultApps => &mut self.default_apps,
            Page::Packages => &mut self.packages,
            Page::Processes => &mut self.processes,
            Page::Services => &mut self.services,
            Page::Network => &mut self.network,
            Page::Storage => &mut self.storage,
            Page::System => &mut self.system_info,
            Page::Timers => &mut self.timers,
            Page::Notifications => &mut self.notifications,
        }
    }

    pub fn get_current_page(&self) -> &dyn crate::pages::AppPage {
        self.get_page(self.current_page)
    }

    pub fn get_current_page_mut(&mut self) -> &mut dyn crate::pages::AppPage {
        self.get_page_mut(self.current_page)
    }
}

#[derive(Debug, Clone)]
pub enum AppAction {
    Exit,
    /// A page worker sent a snapshot; the next tick drains it. Carries
    /// nothing and rebuilds nothing by itself.
    Wake,
    Audio(audio::AudioMessage),
    Browser(browser::BrowserMessage),
    DefaultApps(default_apps::DefaultAppsMessage),
    Network(network::NetworkMessage),
    Bluetooth(bluetooth::BluetoothMessage),
    Power(power::PowerMessage),
    Processes(processes::ProcessesMessage),
    Services(services::ServicesMessage),
    SystemInfo(system_info::SystemMessage),
    Timers(timers::TimersMessage),
    Storage(storage::StorageMessage),
    Accounts(accounts::AccountsMessage),
    Packages(packages::PackagesMessage),
    Notifications(notifications::NotificationsMessage),
}


/// The width `text` takes at `size` in `font` (None = the default UI face,
/// what `PageContent::text` draws in), shaped as the renderer shapes it —
/// for placing a glyph right after a run of text.
pub fn text_width(text: &str, size: f32, font: Option<&str>) -> f32 {
    cce_ui::geometry_font_system()
        .lock()
        .ok()
        .and_then(|mut fs| {
            cce_ui::backend::text::shaped_cluster_offsets(&mut fs, text, size, font)
                .last()
                .map(|&(_, total)| total)
        })
        .unwrap_or(0.0)
}

/// The y a label of `size` in `font` takes to sit centred in a band of
/// height `h` at `y` — the renderer's own centring of a button label, so a
/// run placed beside one shares its line.
pub fn label_y_in(y: f32, h: f32, size: f32, font: Option<&str>) -> f32 {
    let lh = cce_ui::geometry_font_system()
        .lock()
        .map(|mut fs| cce_ui::backend::get_text_buffer(&mut fs, "Ag", size, font).metrics().line_height)
        .unwrap_or(size * 1.2)
        / cce_ui::scale::scale_factor();
    y + (h - lh) / 2.0
}

/// A page as its view pass records it: three recording layers in PAGE
/// coordinates (before the scroll), plus the page's buttons and its section
/// wells. `finish` hands the layers over as display lists; the host keeps them
/// until the next rebuild and emits them every frame under the scroll offset
/// (`PaintCtx::append_translated`), so a scroll moves the page without a
/// rebuild.
///
/// A widget reaches it through the toolkit's flat-host replay
/// (`compose::render_widget`, which `Form::widget_h` runs): every `RenderTarget`
/// call lands as a real prim, in the order the widget made it. A clip
/// (`push_clip_rect`) is a scissor on all three layers, as it is on a
/// `PaintCtx`: what it cuts is cut square at the clip, not shrunk to it —
/// except a control's carves, which take none (see `PageContent::carve`).
///
/// Until 2026-10-12 this was tuple vectors (rects shrunk to the innermost
/// clip, texts with the clip folded into their bounds, carves and the marks
/// that said where each fell between the rects) that the host flattened again
/// into its own tuples every rebuild and shifted in place on a scroll.
pub struct PageContent {
    /// Fills and carves — the page's own rects, each widget's replayed quads,
    /// rounded rects and borders, a Dropdown's inset plate, a TextBox's well,
    /// a Toggle's steps — in the order they were drawn, so a TextBox's
    /// selection and caret land on its well rather than under its walls.
    geo: PaintCtx,
    /// The text runs. Drawn in the glyph pass after all geometry wherever
    /// they sit in a list, so they are a layer of their own: the search dims
    /// and lights them as one.
    text: PaintCtx,
    /// The glyphs: [`PageContent::icon`] (a sort chevron, a row's in-use
    /// check) and a widget's own through `RenderTarget::icon` (a Dropdown's
    /// chevron). The host draws them over the section wells.
    icons: PaintCtx,
    /// (button, action, clip): clip is the innermost `push_clip_rect` at
    /// emission time (page coords, `[x, y, w, h]`) — the host cuts the drawn
    /// face, label bounds and the dispatch clone's hit rect to it.
    pub buttons: Vec<(cce_ui::widget::Adapted<cce_ui::widget::Button>, AppAction, Option<[f32; 4]>)>,
    /// Section wells claimed via `RenderTarget::section_relief` — the body box
    /// plus the title tab box (page coordinates), carved into the window
    /// plate by the host after the page's geometry.
    pub reliefs: Vec<((f32, f32, f32, f32), Option<(f32, f32, f32, f32)>)>,
    /// The clips pushed and not yet popped, as pushed — what `carve` lifts
    /// off the geometry layer and puts back.
    clips: Vec<Rect>,
}

/// A finished [`PageContent`]: its layers as display lists, still in page
/// coordinates, and what the host builds the rest of the page from.
pub struct PageLayers {
    pub geo: DisplayList,
    pub text: DisplayList,
    pub icons: DisplayList,
    pub buttons: Vec<(cce_ui::widget::Adapted<cce_ui::widget::Button>, AppAction, Option<[f32; 4]>)>,
    pub reliefs: Vec<((f32, f32, f32, f32), Option<(f32, f32, f32, f32)>)>,
}

/// One text run of a finished page, read back out of its text layer.
#[derive(Clone, Debug, PartialEq)]
pub struct PageText {
    pub text: String,
    pub size: f32,
    pub x: f32,
    pub y: f32,
    pub color: [u8; 3],
    pub font: Option<String>,
    /// `[l, t, r, b]`: the run's own bounds cut to the clip it was drawn
    /// under, what the glyph pass clamps it to. `None` = unbounded.
    pub bounds: Option<[f32; 4]>,
}

/// A text colour as the layers carry it: sRGB bytes, truncated as the
/// toolkit's flat-host bridge truncates them.
fn text_rgb(c: [f32; 4]) -> [u8; 3] {
    [(c[0] * 255.0) as u8, (c[1] * 255.0) as u8, (c[2] * 255.0) as u8]
}

fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect { x, y, width: w, height: h }
}

impl PageContent {
    pub fn new() -> Self {
        Self {
            geo: PaintCtx::new(),
            text: PaintCtx::new(),
            icons: PaintCtx::new(),
            buttons: Vec::new(),
            reliefs: Vec::new(),
            clips: Vec::new(),
        }
    }

    /// The layers, as built. The clips must be balanced.
    pub fn finish(self) -> PageLayers {
        PageLayers {
            geo: self.geo.finish(),
            text: self.text.finish(),
            icons: self.icons.finish(),
            buttons: self.buttons,
            reliefs: self.reliefs,
        }
    }

    /// The innermost clip, `[x, y, w, h]` — what a button records.
    fn clip(&self) -> Option<[f32; 4]> {
        self.geo.current_clip().map(|c| [c.x, c.y, c.width, c.height])
    }

    /// A fill: a plain quad, or rounded at `radius` on `corners` — a radius
    /// under 0.1 is no rounding (the host never drew a radius-0 rounded rect).
    fn fill(&mut self, color: [f32; 4], r: Rect, radius: f32, corners: (bool, bool, bool, bool)) {
        if radius > 0.1 {
            self.geo.rounded_rect(r, radius, corners, color);
        } else {
            self.geo.quad(r, color);
        }
    }

    /// A control's carve (its inset plate, a well, a step), drawn clear of
    /// the clips: the host cuts it to the page view alone, as it always has.
    /// A control carves inside its own rect (`layout::carve_inside`), so a
    /// clip has nothing of it to take — but the scissor truncates a clip's
    /// fractional left edge and width separately (`dl_batches_2d`), which
    /// shaved the last physical column off the right rim of every control
    /// that runs the width of its section.
    fn carve(&mut self, f: impl FnOnce(&mut PaintCtx)) {
        for _ in &self.clips {
            self.geo.pop_clip();
        }
        f(&mut self.geo);
        for &r in &self.clips {
            self.geo.push_clip(r);
        }
    }

    fn run(&mut self, content: &str, x: f32, y: f32, size: f32, color: [f32; 4], font: Option<&str>, bounds: Option<[f32; 4]>) {
        self.text.text_with(content, x, y, size, text_rgb(color), font.map(str::to_string), bounds);
    }

    pub fn rect(&mut self, color: [f32; 4], x: f32, y: f32, w: f32, h: f32) {
        self.fill(color, rect(x, y, w, h), 0.0, (true, true, true, true));
    }

    pub fn text(&mut self, content: &str, x: f32, y: f32, size: f32, color: [f32; 4]) {
        self.run(content, x, y, size, color, None, None);
    }

    pub fn text_with_font(&mut self, content: &str, x: f32, y: f32, size: f32, color: [f32; 4], font: &str) {
        self.run(content, x, y, size, color, Some(font), None);
    }

    /// A bundled cce-icons glyph at (x, y, w, h), tinted `color` as a text
    /// colour is (raw sRGB, alpha = the image's) — the ONE way a page draws a
    /// symbol, never a character in whatever face the font falls back to.
    /// Rasterized at twice the rect's longer side for a 2x output.
    ///
    /// `false` when the icon set lacks the glyph, so the caller can say it
    /// in a WORD instead.
    pub fn icon(&mut self, name: &str, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) -> bool {
        self.icons.icon(name, rect(x, y, w, h), color)
    }

    pub fn button(&mut self, label: &str, x: f32, y: f32, w: f32, h: f32,
                  bg: [f32; 4], hover_bg: [f32; 4], label_color: [f32; 4],
                  action: AppAction) {
        let btn = cce_ui::widget::Button::new(x, y, w, h)
            .with_label(label)
            .with_bg(bg)
            .with_hover_bg(hover_bg)
            .with_label_color(label_color);
        let clip = self.clip();
        self.buttons.push((btn, action, clip));
    }

    /// A button whose face is a bundled cce-icons glyph instead of a label.
    ///
    /// `label` stays as the FALLBACK: `upload_icon` returns `None` when the
    /// icon set is missing or unparsable, and a control that silently loses
    /// its face has no affordance left at all — so the button degrades to the
    /// word rather than to an empty box. `alpha` dims the glyph for a disabled
    /// control, which is the only state lever an icon has (images carry no
    /// color).
    pub fn button_icon(&mut self, icon: &str, label: &str, x: f32, y: f32, w: f32, h: f32,
                       bg: [f32; 4], hover_bg: [f32; 4], label_color: [f32; 4],
                       alpha: f32, action: AppAction) {
        let mut btn = cce_ui::widget::Button::new(x, y, w, h)
            .with_label(label)
            .with_bg(bg)
            .with_hover_bg(hover_bg)
            .with_label_color(label_color);
        // 32px on the longer side: the toolkit caches the upload per (name, px),
        // so every row's Start button shares one texture.
        if let Some((id, iw, ih)) = cce_ui::upload_icon(icon, 32) {
            btn = btn.with_icon(id, iw as f32, ih as f32).with_icon_alpha(alpha);
        }
        let clip = self.clip();
        self.buttons.push((btn, action, clip));
    }

    /// A row of a list, in the toolkit's list style (`Button::new_list_row`,
    /// what cce-mail, cce-fonts and cce-calendar rows wear): no plate,
    /// transparent until hovered, the shared row wash. A plain [`button`]
    /// with a transparent face is NOT that — under `control_relief` it still
    /// gets a control plate, edges only, so every idle row of a list wore a
    /// carved ring. No colours to pass: the renderer asks the row for its own.
    ///
    /// [`button`]: PageContent::button
    pub fn list_row(&mut self, x: f32, y: f32, w: f32, h: f32, action: AppAction) {
        let btn = cce_ui::widget::Button::new_list_row(x, y, w, h);
        let clip = self.clip();
        self.buttons.push((btn, action, clip));
    }

    /// [`button_icon`](Self::button_icon) with the glyph tinted
    /// `label_color` (its alpha the glyph's), where the face's colour is part
    /// of what the control says — the process list's red Kill. The white
    /// glyph `button_icon` uploads cannot carry a colour: an image has none.
    pub fn button_icon_tinted(&mut self, icon: &str, label: &str, x: f32, y: f32, w: f32, h: f32,
                              bg: [f32; 4], hover_bg: [f32; 4], label_color: [f32; 4],
                              action: AppAction) {
        let mut btn = cce_ui::widget::Button::new(x, y, w, h)
            .with_label(label)
            .with_bg(bg)
            .with_hover_bg(hover_bg)
            .with_label_color(label_color);
        if let Some((id, iw, ih)) = cce_ui::upload_icon_tinted(icon, 32, cce_ui::icon_tint(label_color)) {
            btn = btn.with_icon(id, iw as f32, ih as f32).with_icon_alpha(label_color[3]);
        }
        let clip = self.clip();
        self.buttons.push((btn, action, clip));
    }

    pub fn button_left(&mut self, label: &str, x: f32, y: f32, w: f32, h: f32,
                       bg: [f32; 4], hover_bg: [f32; 4], label_color: [f32; 4],
                       action: AppAction) {
        let btn = cce_ui::widget::Button::new(x, y, w, h)
            .with_label(label)
            .with_bg(bg)
            .with_hover_bg(hover_bg)
            .with_label_color(label_color)
            .with_left_align(true);
        let clip = self.clip();
        self.buttons.push((btn, action, clip));
    }
}

impl Default for PageContent {
    fn default() -> Self {
        Self::new()
    }
}

/// The flat-host surface the toolkit's replay and `compose` draw a page
/// through. Every call becomes a prim on a layer; `line`, `arc` and `circle`
/// keep the trait's no-op (this host never drew them — a widget's strokes
/// arrive with Stage B's real widget paint).
impl RenderTarget for PageContent {
    fn icon(&mut self, name: &str, r: Rect, color: [f32; 4]) {
        self.icons.icon(name, r, color);
    }

    fn rect(&mut self, color: [f32; 4], x: f32, y: f32, w: f32, h: f32) {
        self.fill(color, rect(x, y, w, h), 0.0, (true, true, true, true));
    }

    fn rect_with_radius(&mut self, color: [f32; 4], x: f32, y: f32, w: f32, h: f32, radius: f32) {
        self.fill(color, rect(x, y, w, h), radius, (true, true, true, true));
    }

    fn rect_with_radius_corners(&mut self, color: [f32; 4], x: f32, y: f32, w: f32, h: f32, radius: f32, corners: (bool, bool, bool, bool)) {
        self.fill(color, rect(x, y, w, h), radius, corners);
    }

    fn text(&mut self, content: &str, x: f32, y: f32, size: f32, color: [f32; 4]) {
        self.run(content, x, y, size, color, None, None);
    }

    fn text_with_font(&mut self, content: &str, x: f32, y: f32, size: f32, color: [f32; 4], font: &str) {
        self.run(content, x, y, size, color, Some(font), None);
    }

    fn text_with_bounds(&mut self, content: &str, x: f32, y: f32, size: f32, color: [f32; 4], bounds: Option<[f32; 4]>) {
        self.run(content, x, y, size, color, None, bounds);
    }

    fn text_with_font_and_bounds(&mut self, content: &str, x: f32, y: f32, size: f32, color: [f32; 4], font: &str, bounds: Option<[f32; 4]>) {
        self.run(content, x, y, size, color, Some(font), bounds);
    }

    fn section_relief_style(&self) -> bool {
        cce_ui::layout::control_relief()
    }

    /// A Dropdown's or a Button's flush inset trough, carved for real where
    /// the widget drew it.
    fn inset_plate(&mut self, color: [f32; 4], x: f32, y: f32, w: f32, h: f32, radius: f32, depth: f32) {
        let face = Material::face(color);
        self.carve(|geo| geo.inset_plate(rect(x, y, w, h), (radius, radius, radius, radius), face.as_ref(), depth));
    }

    /// [`inset_plate`](RenderTarget::inset_plate) with the rim lit — a
    /// focused control.
    fn inset_plate_tinted(&mut self, color: [f32; 4], x: f32, y: f32, w: f32, h: f32, radius: f32, depth: f32, tint: [f32; 3]) {
        let face = Material::face(color);
        self.carve(|geo| geo.inset_plate_tinted(rect(x, y, w, h), (radius, radius, radius, radius), face.as_ref(), depth, tint));
    }

    /// The step carves — a DIFFERENT shape from an inset plate, not one at
    /// another rect: a trough is a seam about the boundary with the face left
    /// level, a well drops the whole interior, a boss raises it.
    fn relief_carve(&mut self, carve: &cce_ui::scene::paint::ReliefCarve) {
        self.carve(|geo| geo.carve(carve));
    }

    fn section_relief(&mut self, f: &cce_ui::scene::paint::SectionFrame) -> bool {
        // Focused sections keep the legacy green outline (the ctrl-nav feedback);
        // relief-off styling keeps the outline everywhere.
        if f.focused || !cce_ui::layout::control_relief() {
            return false;
        }
        self.reliefs.push(((f.x, f.y, f.w, f.h), f.tab));
        true
    }

    fn push_clip_rect(&mut self, x: f32, y: f32, w: f32, h: f32) {
        let r = rect(x, y, w, h);
        self.clips.push(r);
        self.geo.push_clip(r);
        self.text.push_clip(r);
        self.icons.push_clip(r);
    }

    fn pop_clip_rect(&mut self) {
        self.clips.pop();
        self.geo.pop_clip();
        self.text.pop_clip();
        self.icons.pop_clip();
    }
}

impl PageLayers {
    /// Each fill (quad or rounded rect) where it shows: cut to the clip it
    /// was drawn under, and left out when that clip leaves none of it.
    pub fn fills(&self) -> Vec<Rect> {
        self.geo
            .items
            .iter()
            .filter_map(|item| {
                let r = match item.prim {
                    Prim::Quad { rect, .. } | Prim::RoundedRect { rect, .. } => rect,
                    _ => return None,
                };
                let Some(c) = item.clip else { return Some(r) };
                let (x0, y0) = (r.x.max(c.x), r.y.max(c.y));
                let (x1, y1) = ((r.x + r.width).min(c.x + c.width), (r.y + r.height).min(c.y + c.height));
                (x0 < x1 && y0 < y1).then(|| rect(x0, y0, x1 - x0, y1 - y0))
            })
            .collect()
    }

    /// The text runs, in the order they were drawn.
    pub fn texts(&self) -> Vec<PageText> {
        self.text
            .items
            .iter()
            .filter_map(|item| match &item.prim {
                Prim::Text { text, x, y, font_size, color, font, bounds, .. } => {
                    let clip = item.clip.map(|c| [c.x, c.y, c.x + c.width, c.y + c.height]);
                    let bounds = match (clip, *bounds) {
                        (Some(a), Some(b)) => Some([a[0].max(b[0]), a[1].max(b[1]), a[2].min(b[2]), a[3].min(b[3])]),
                        (a, b) => a.or(b),
                    };
                    Some(PageText {
                        text: text.clone(),
                        size: *font_size,
                        x: *x,
                        y: *y,
                        color: *color,
                        font: font.clone(),
                        bounds,
                    })
                }
                _ => None,
            })
            .collect()
    }

    /// Where each glyph sits.
    pub fn icons(&self) -> Vec<Rect> {
        self.icons
            .items
            .iter()
            .filter_map(|item| match item.prim {
                Prim::Image { rect, .. } => Some(rect),
                _ => None,
            })
            .collect()
    }

    /// How far down the page reaches: the lowest fill (as it shows), text
    /// line (its top plus its size) or button. Carves and wells do not
    /// count — a control's inset face sits inside the control's own rects.
    pub fn bottom(&self) -> f32 {
        let fills = self.fills().into_iter().map(|r| r.y + r.height);
        let texts = self.text.items.iter().filter_map(|item| match item.prim {
            Prim::Text { y, font_size, .. } => Some(y + font_size),
            _ => None,
        });
        let buttons = self.buttons.iter().map(|(btn, _, _)| {
            let base = cce_ui::widget::WidgetHost::base(btn);
            base.y + base.h
        });
        fills.chain(texts).chain(buttons).fold(0.0f32, f32::max)
    }
}

/// Width a label needs on a button plate. Feeds [`form_button_fit`], so a row of
/// buttons is divided by what is written on them rather than into equal
/// slices — "Reboot" and "Hibernate" are not the same size and a row that
/// pretends otherwise clips one and pads the other.
///
/// Asks the Button itself (`intrinsic_size`: its label shaped in the button
/// font, plus its 8px inset each side) rather than measuring here. This used
/// `measure_text_width` in the control-label font, an inked extent that runs
/// ~20% short of the shaped run under Berkeley Mono, so the row handed every
/// button less than its label and "Hibernate" / "Power Off" were cut off.
pub fn button_need(label: &str) -> f32 {
    cce_ui::widget::Button::new(0.0, 0.0, 0.0, 0.0)
        .with_label(label)
        .intrinsic_size()
        .map_or(0.0, |s| s.width)
}

/// A page button (`PageContent::button`) as a form piece, a button's height: `w` wide, or
/// the width of its column (and, in a row, the row's slack) when `w` is 0. `colors` are the
/// face, the hover face and the label.
pub fn form_button<'w>(
    g: &mut cce_ui::compose::FormGroup<'_, 'w, PageContent>,
    label: impl Into<String>,
    w: f32,
    colors: ([f32; 4], [f32; 4], [f32; 4]),
    action: AppAction,
) {
    let label = label.into();
    g.draw(w, cce_ui::layout::button_height(), w == 0.0, move |pc, r, _| {
        pc.button(&label, r.x, r.y, r.width, r.height, colors.0, colors.1, colors.2, action);
    });
}

/// A page button as a row cell that asks for `need` (its label's own width) and shares the
/// row's slack with the other cells — a row of buttons sized to their labels.
pub fn form_button_fit<'w>(
    g: &mut cce_ui::compose::FormGroup<'_, 'w, PageContent>,
    label: impl Into<String>,
    need: f32,
    colors: ([f32; 4], [f32; 4], [f32; 4]),
    action: AppAction,
) {
    let label = label.into();
    g.draw(need, cce_ui::layout::button_height(), true, move |pc, r, _| {
        pc.button(&label, r.x, r.y, r.width, r.height, colors.0, colors.1, colors.2, action);
    });
}

/// `text` broken into lines no wider than `width` at `size`, measured as the
/// renderer shapes it (the default UI face, as `PageContent::text` draws).
/// Breaks after a space or before a `/`, so a path splits on its separators;
/// a run with neither is cut wherever it overflows. A section's text is
/// clipped to its content box, so anything that does not fit has to wrap or
/// it is simply lost — a GPU name, the CPU model, a pending install path.
pub fn wrap_to_width(text: &str, width: f32, size: f32) -> Vec<String> {
    let offsets = cce_ui::geometry_font_system()
        .lock()
        .map(|mut fs| cce_ui::backend::text::shaped_cluster_offsets(&mut fs, text, size, None))
        .unwrap_or_default();
    if offsets.last().is_none_or(|&(_, total)| total <= width) {
        return vec![text.to_string()];
    }
    let mut lines = Vec::new();
    let (mut start, mut start_x) = (0usize, 0.0f32);
    // Where the next line would begin if this one broke at the last chance.
    let mut chance: Option<(usize, f32)> = None;
    for pair in offsets.windows(2) {
        let ((b, x), (next_b, next_x)) = (pair[0], pair[1]);
        if next_x - start_x > width && b > start {
            let (cut, cut_x) = chance.filter(|&(c, _)| c > start).unwrap_or((b, x));
            lines.push(text[start..cut].trim_end().to_string());
            (start, start_x, chance) = (cut, cut_x, None);
        }
        match &text[b..next_b] {
            " " => chance = Some((next_b, next_x)),
            "/" if b > start => chance = Some((b, x)),
            _ => {}
        }
    }
    lines.push(text[start..].trim_end().to_string());
    lines.retain(|l| !l.is_empty());
    lines
}

/// Label / value pairs as one block of text: the labels in a column as wide as the widest of
/// them, each value beside its label and wrapped to the room left of the content box, the
/// label on the value's first line. Each pair is (label, label colour, value, value colour).
pub fn form_pairs<'w>(
    col: &mut cce_ui::compose::FormGroup<'_, 'w, PageContent>,
    size: f32,
    pairs: Vec<(String, [f32; 4], String, [f32; 4])>,
) {
    use cce_ui::scene::layout::{CrossAlign, Style};
    let label_w = pairs.iter().map(|p| cce_ui::compose::form_text_width(&p.0, size)).fold(0.0, f32::max);
    let line_h = cce_ui::compose::form_line_height(size);
    let value_w = (col.form_width() - label_w - cce_ui::layout::control_gap()).max(1.0);
    // Lines of text, not controls: they stand a line apart, with no control gap between.
    col.group(Style::column().cross_align(CrossAlign::Stretch), false, |lines| {
        for (label, label_color, value, value_color) in pairs {
            lines.group(Style::controls_row(), true, |row| {
                row.draw(label_w, line_h, false, move |pc, r, _| pc.text(&label, r.x, r.y, size, label_color));
                row.lines(wrap_to_width(&value, value_w, size), size, value_color);
            });
        }
    });
}

/// The hairline that parts a well's zones, as a form piece.
pub fn form_divider(g: &mut cce_ui::compose::FormGroup<'_, '_, PageContent>) {
    g.rule([1.0, 1.0, 1.0, 0.06]);
}


#[cfg(test)]
mod page_content_tests {
    use super::*;

    /// A control's carve lands between the fills it was drawn between — the
    /// order is the picture (a TextBox's caret on its well, a Dropdown's
    /// hovered row on its plate) — and a FOCUSED control's lit plate is kept
    /// like any other. The marks this replaced had no entry for a lit plate,
    /// so every carve after one slid a place later and the last was dropped.
    #[test]
    fn carves_keep_their_place_and_a_lit_plate_is_kept() {
        let mut pc = PageContent::new();
        pc.rect([1.0; 4], 0.0, 0.0, 10.0, 10.0);
        RenderTarget::inset_plate_tinted(&mut pc, [0.0; 4], 0.0, 0.0, 10.0, 10.0, 2.0, 2.0, [0.0, 1.0, 0.0]);
        RenderTarget::rect_with_radius(&mut pc, [1.0; 4], 0.0, 20.0, 10.0, 10.0, 4.0);
        RenderTarget::inset_plate(&mut pc, [0.0; 4], 0.0, 20.0, 10.0, 10.0, 2.0, 2.0);
        let layers = pc.finish();
        let kinds: Vec<&str> = layers
            .geo
            .items
            .iter()
            .map(|i| match i.prim {
                Prim::Quad { .. } => "quad",
                Prim::RoundedRect { .. } => "rounded",
                Prim::Field { tint: Some(_), .. } => "lit run",
                Prim::Field { tint: None, .. } => "run",
                _ => "other",
            })
            .collect();
        assert_eq!(kinds, ["quad", "lit run", "rounded", "run"]);
    }

    /// A clip is a scissor on the fills and the text, and a fill under one
    /// counts (for the page's extent) only as far as it shows. A control's
    /// carve takes no clip, nested or not, and the clips stand again after it.
    #[test]
    fn a_clip_cuts_fills_and_text_but_not_carves() {
        let mut pc = PageContent::new();
        RenderTarget::push_clip_rect(&mut pc, 0.0, 0.0, 100.0, 50.0);
        pc.rect([1.0; 4], 0.0, 40.0, 10.0, 30.0);
        RenderTarget::push_clip_rect(&mut pc, 0.0, 0.0, 100.0, 45.0);
        RenderTarget::relief_carve(&mut pc, &cce_ui::scene::paint::ReliefCarve {
            kind: cce_ui::scene::paint::CarveKind::Recess { tint: None },
            x: 0.0,
            y: 40.0,
            w: 10.0,
            h: 10.0,
            radii: (0.0, 0.0, 0.0, 0.0),
            depth: 2.0,
            edges: (true, true, true, true),
        });
        RenderTarget::pop_clip_rect(&mut pc);
        pc.rect([1.0; 4], 0.0, 60.0, 10.0, 30.0);
        pc.text("a", 5.0, 45.0, 12.0, [1.0; 4]);
        RenderTarget::pop_clip_rect(&mut pc);
        pc.text("b", 5.0, 80.0, 12.0, [1.0; 4]);
        let layers = pc.finish();
        let clip = Some(Rect { x: 0.0, y: 0.0, width: 100.0, height: 50.0 });
        let clips: Vec<_> = layers.geo.items.iter().map(|i| i.clip).collect();
        assert_eq!(clips, [clip, None, clip]);
        assert_eq!(layers.fills(), [Rect { x: 0.0, y: 40.0, width: 10.0, height: 10.0 }]);
        let texts = layers.texts();
        assert_eq!(texts[0].bounds, Some([0.0, 0.0, 100.0, 50.0]));
        assert_eq!(texts[1].bounds, None);
        assert_eq!(layers.bottom(), 92.0, "the unclipped text's foot");
    }
}
