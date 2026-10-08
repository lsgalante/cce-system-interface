use cce_ui::widget::Owned;
use cce_ui::layout::RenderTarget;

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

impl Default for AppState {
    fn default() -> Self {
        Self {
            current_page: Page::ALL[0],
            audio: audio::AudioState::default(),
            bluetooth: bluetooth::BluetoothState::default(),
            power: power::PowerState::default(),
            browser: browser::BrowserState::default(),
            default_apps: default_apps::DefaultAppsState::default(),
            network: network::NetworkState::default(),
            processes: processes::ProcessesState::default(),
            services: services::ServicesState::default(),
            system_info: system_info::SystemState::default(),
            timers: timers::TimersState::default(),
            storage: storage::StorageState::default(),
            accounts: accounts::AccountsState::default_mock(),
            packages: packages::PackagesState::default(),
            notifications: notifications::NotificationsState::default(),
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


/// A control carve bridged out of a widget's `paint` for the flat path — the
/// relief prims the toolkit's controls draw for themselves and the legacy
/// `all_quads` stream cannot carry. The page collects them and `display_list`
/// re-emits them as real prims into the window plate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ControlCarve {
    /// `PaintCtx::inset_plate` — a flush inset face over a boundary seam
    /// (Dropdown, Button). `color` fills the face; transparent leaves the
    /// plate below.
    Plate { x: f32, y: f32, w: f32, h: f32, radius: f32, depth: f32, color: [f32; 4], tint: Option<[f32; 3]> },
    /// A step carve straight from the toolkit (TextBox well, a Toggle's well
    /// and glider) — it already carries its own rect, per-corner radii,
    /// depth and wall mask, so nothing here re-derives them.
    Step(cce_ui::layout::ReliefCarve),
}

impl ControlCarve {
    /// This carve shifted vertically — the page-scroll adjustment the popover
    /// collector applies when it lifts page carves to the popover layer.
    pub fn shifted_y(self, dy: f32) -> Self {
        match self {
            Self::Plate { x, y, w, h, radius, depth, color, tint } => {
                Self::Plate { x, y: y + dy, w, h, radius, depth, color, tint }
            }
            Self::Step(c) => Self::Step(c.shifted_y(dy)),
        }
    }
}

/// A bundled cce-icons glyph a page placed on its own (a sort chevron, a
/// row's in-use check) rather than as a button's face: the tinted upload's
/// image id, its rect and alpha, and the innermost clip at emission time —
/// all page coordinates, pre-scroll, like `texts`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PageIcon {
    pub image: u32,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub alpha: f32,
    pub clip: Option<[f32; 4]>,
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

pub struct PageContent {
    pub rects: Vec<([f32; 4], f32, f32, f32, f32, f32, (bool, bool, bool, bool))>,
    pub texts: Vec<(String, f32, f32, f32, [f32; 4], Option<String>, Option<[f32; 4]>)>,
    /// (button, action, clip): clip is the innermost push_clip_rect at emission
    /// time (page coords) — the renderer clamps the drawn quad, label bounds,
    /// and the dispatch clone's hit rect to it.
    pub buttons: Vec<(Owned<cce_ui::widget::Adapted<cce_ui::widget::Button>>, AppAction, Option<[f32; 4]>)>,
    /// Glyphs placed with [`PageContent::icon`] (and a widget's own, through
    /// `RenderTarget::icon` — the expanded Dropdown's chevron).
    pub icons: Vec<PageIcon>,
    /// Section wells claimed via `RenderTarget::section_relief` — the body box
    /// plus the title tab box, carved into the window plate by display_list as
    /// recess prims (page coordinates, pre-scroll).
    pub reliefs: Vec<((f32, f32, f32, f32), Option<(f32, f32, f32, f32)>)>,
    /// Control carves claimed via the flat-path relief hooks
    /// (`RenderTarget::inset_plate` for a Dropdown's flush inset chrome,
    /// `RenderTarget::recess` for a TextBox's well) — (x, y, w, h, radius,
    /// depth, carve), page coordinates, pre-scroll; display_list re-emits them
    /// as real relief prims.
    pub control_reliefs: Vec<ControlCarve>,
    /// Per carve, `rects.len()` at the moment it was claimed — the carve's
    /// place in the widget's own emission order. A Dropdown draws its
    /// hovered-row highlight AFTER the inset plate it claims for the menu
    /// face; replaying every rect and then every carve put the frosted plate
    /// over the highlight. The popover layer interleaves on these marks.
    pub control_relief_marks: Vec<usize>,
    pub clip_stack: Vec<[f32; 4]>,
}

impl PageContent {
    pub fn new() -> Self {
        Self {
            rects: Vec::new(),
            texts: Vec::new(),
            buttons: Vec::new(),
            icons: Vec::new(),
            reliefs: Vec::new(),
            control_reliefs: Vec::new(),
            control_relief_marks: Vec::new(),
            clip_stack: Vec::new(),
        }
    }

    fn get_clipped_rect(&self, x: f32, y: f32, w: f32, h: f32) -> Option<(f32, f32, f32, f32)> {
        if let Some(&clip) = self.clip_stack.last() {
            let cx = clip[0];
            let cy = clip[1];
            let cw = clip[2];
            let ch = clip[3];
            let rx1 = x.max(cx);
            let ry1 = y.max(cy);
            let rx2 = (x + w).min(cx + cw);
            let ry2 = (y + h).min(cy + ch);
            if rx1 < rx2 && ry1 < ry2 {
                Some((rx1, ry1, rx2 - rx1, ry2 - ry1))
            } else {
                None
            }
        } else {
            Some((x, y, w, h))
        }
    }

    fn get_clipped_bounds(&self, bounds: Option<[f32; 4]>) -> Option<[f32; 4]> {
        if let Some(&clip) = self.clip_stack.last() {
            if let Some(b) = bounds {
                let rx1 = b[0].max(clip[0]);
                let ry1 = b[1].max(clip[1]);
                let rx2 = b[2].min(clip[0] + clip[2]);
                let ry2 = b[3].min(clip[1] + clip[3]);
                Some([rx1, ry1, rx2.max(rx1), ry2.max(ry1)])
            } else {
                Some([clip[0], clip[1], clip[0] + clip[2], clip[1] + clip[3]])
            }
        } else {
            bounds
        }
    }

    pub fn rect(&mut self, color: [f32; 4], x: f32, y: f32, w: f32, h: f32) {
        if let Some((cx, cy, cw, ch)) = self.get_clipped_rect(x, y, w, h) {
            self.rects.push((color, cx, cy, cw, ch, 0.0, (true, true, true, true)));
        }
    }

    pub fn text(&mut self, content: &str, x: f32, y: f32, size: f32, color: [f32; 4]) {
        let cb = self.get_clipped_bounds(None);
        self.texts.push((content.to_string(), size, x, y, color, None, cb));
    }

    pub fn text_with_font(&mut self, content: &str, x: f32, y: f32, size: f32, color: [f32; 4], font: &str) {
        let cb = self.get_clipped_bounds(None);
        self.texts.push((content.to_string(), size, x, y, color, Some(font.to_string()), cb));
    }

    /// A bundled cce-icons glyph at (x, y, w, h), tinted `color` as a text
    /// colour is (raw sRGB, alpha = the image's) — the ONE way a page draws a
    /// symbol, never a character in whatever face the font falls back to.
    /// Rasterized at twice the rect's longer side for a 2x output.
    ///
    /// `false` when the icon set lacks the glyph, so the caller can say it
    /// in a WORD instead.
    pub fn icon(&mut self, name: &str, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) -> bool {
        let px = (w.max(h) * 2.0).ceil().max(1.0) as u32;
        let Some((image, _, _)) = cce_ui::upload_icon_tinted(name, px, cce_ui::icon_tint(color)) else {
            return false;
        };
        let clip = self.clip_stack.last().copied();
        self.icons.push(PageIcon { image, x, y, w, h, alpha: color[3], clip });
        true
    }

    pub fn button(&mut self, label: &str, x: f32, y: f32, w: f32, h: f32,
                  bg: [f32; 4], hover_bg: [f32; 4], label_color: [f32; 4],
                  action: AppAction) {
        let btn = cce_ui::widget::Button::new(x, y, w, h)
            .with_label(label)
            .with_bg(bg)
            .with_hover_bg(hover_bg)
            .with_label_color(label_color);
        let clip = self.clip_stack.last().copied();
        self.buttons.push((Owned::new(btn), action, clip));
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
        let clip = self.clip_stack.last().copied();
        self.buttons.push((Owned::new(btn), action, clip));
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
        let clip = self.clip_stack.last().copied();
        self.buttons.push((Owned::new(btn), action, clip));
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
        let clip = self.clip_stack.last().copied();
        self.buttons.push((Owned::new(btn), action, clip));
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
        let clip = self.clip_stack.last().copied();
        self.buttons.push((Owned::new(btn), action, clip));
    }
}

impl RenderTarget for PageContent {
    fn icon(&mut self, name: &str, rect: cce_ui::scene::layout::Rect, color: [f32; 4]) {
        PageContent::icon(self, name, rect.x, rect.y, rect.width, rect.height, color);
    }

    fn rect(&mut self, color: [f32; 4], x: f32, y: f32, w: f32, h: f32) {
        if let Some((cx, cy, cw, ch)) = self.get_clipped_rect(x, y, w, h) {
            self.rects.push((color, cx, cy, cw, ch, 0.0, (true, true, true, true)));
        }
    }

    fn rect_with_radius(&mut self, color: [f32; 4], x: f32, y: f32, w: f32, h: f32, radius: f32) {
        if let Some((cx, cy, cw, ch)) = self.get_clipped_rect(x, y, w, h) {
            self.rects.push((color, cx, cy, cw, ch, radius, (true, true, true, true)));
        }
    }

    fn rect_with_radius_corners(&mut self, color: [f32; 4], x: f32, y: f32, w: f32, h: f32, radius: f32, corners: (bool, bool, bool, bool)) {
        if let Some((cx, cy, cw, ch)) = self.get_clipped_rect(x, y, w, h) {
            self.rects.push((color, cx, cy, cw, ch, radius, corners));
        }
    }

    fn text(&mut self, content: &str, x: f32, y: f32, size: f32, color: [f32; 4]) {
        let cb = self.get_clipped_bounds(None);
        self.texts.push((content.to_string(), size, x, y, color, None, cb));
    }

    fn text_with_font(&mut self, content: &str, x: f32, y: f32, size: f32, color: [f32; 4], font: &str) {
        let cb = self.get_clipped_bounds(None);
        self.texts.push((content.to_string(), size, x, y, color, Some(font.to_string()), cb));
    }

    fn text_with_bounds(&mut self, content: &str, x: f32, y: f32, size: f32, color: [f32; 4], bounds: Option<[f32; 4]>) {
        let cb = self.get_clipped_bounds(bounds);
        self.texts.push((content.to_string(), size, x, y, color, None, cb));
    }

    fn text_with_font_and_bounds(&mut self, content: &str, x: f32, y: f32, size: f32, color: [f32; 4], font: &str, bounds: Option<[f32; 4]>) {
        let cb = self.get_clipped_bounds(bounds);
        self.texts.push((content.to_string(), size, x, y, color, Some(font.to_string()), cb));
    }

    fn section_relief_style(&self) -> bool {
        cce_ui::layout::control_relief()
    }

    /// The flat-path bridge offers each Dropdown's flush inset trough here;
    /// carve it for real (display_list turns these into inset plates).
    fn inset_plate(&mut self, color: [f32; 4], x: f32, y: f32, w: f32, h: f32, radius: f32, depth: f32) {
        self.control_relief_marks.push(self.rects.len());
        self.control_reliefs.push(ControlCarve::Plate { x, y, w, h, radius, depth, color, tint: None });
    }
    fn inset_plate_tinted(&mut self, color: [f32; 4], x: f32, y: f32, w: f32, h: f32, radius: f32, depth: f32, tint: [f32; 3]) {
        self.control_reliefs.push(ControlCarve::Plate { x, y, w, h, radius, depth, color, tint: Some(tint) });
    }

    /// The same bridge for the step carves — a DIFFERENT shape, not an inset
    /// plate at another rect: a trough is a seam about the boundary with the
    /// face left level, a well drops the whole interior, a boss raises it.
    /// Collapsing them would give this app controls no other relief host has.
    fn relief_carve(&mut self, carve: &cce_ui::layout::ReliefCarve) {
        self.control_relief_marks.push(self.rects.len());
        self.control_reliefs.push(ControlCarve::Step(*carve));
    }

    fn section_relief(&mut self, f: &cce_ui::layout::SectionFrame) -> bool {
        // Focused sections keep the legacy green outline (the ctrl-nav feedback);
        // relief-off styling keeps the outline everywhere.
        if f.focused || !cce_ui::layout::control_relief() {
            return false;
        }
        self.reliefs.push(((f.x, f.y, f.w, f.h), f.tab));
        true
    }

    fn push_clip_rect(&mut self, x: f32, y: f32, w: f32, h: f32) {
        let clip = if let Some(&parent_clip) = self.clip_stack.last() {
            let cx = x.max(parent_clip[0]);
            let cy = y.max(parent_clip[1]);
            let cw = (x + w).min(parent_clip[0] + parent_clip[2]) - cx;
            let ch = (y + h).min(parent_clip[1] + parent_clip[3]) - cy;
            [cx, cy, cw.max(0.0), ch.max(0.0)]
        } else {
            [x, y, w, h]
        };
        self.clip_stack.push(clip);
    }

    fn pop_clip_rect(&mut self) {
        self.clip_stack.pop();
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
    g: &mut cce_ui::layout::FormGroup<'_, 'w, PageContent>,
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
    g: &mut cce_ui::layout::FormGroup<'_, 'w, PageContent>,
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
    if offsets.last().map_or(true, |&(_, total)| total <= width) {
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
    col: &mut cce_ui::layout::FormGroup<'_, 'w, PageContent>,
    size: f32,
    pairs: Vec<(String, [f32; 4], String, [f32; 4])>,
) {
    use cce_ui::scene::layout::{CrossAlign, Style};
    let label_w = pairs.iter().map(|p| cce_ui::layout::form_text_width(&p.0, size)).fold(0.0, f32::max);
    let line_h = cce_ui::layout::form_line_height(size);
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
pub fn form_divider(g: &mut cce_ui::layout::FormGroup<'_, '_, PageContent>) {
    g.rule([1.0, 1.0, 1.0, 0.06]);
}

