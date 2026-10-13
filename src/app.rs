use cce_ui::scene::layout::Rect;
use cce_ui::scene::paint::{DisplayList, PaintCtx, Prim};

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

/// A page as its view pass paints it: ONE display list in call order, in
/// PAGE coordinates (before the scroll) — each section's title, its content
/// as the widgets paint themselves (`cce_ui::compose` places and paints them,
/// `place_and_paint` the rows a list draws), and its well, carved by the
/// section's `finish` — plus the page buttons declared on it. The host keeps
/// the finished list until the next rebuild and emits it every frame under
/// the scroll offset (`PaintCtx::append_translated`), so a scroll moves the
/// page without a rebuild.
///
/// A thin wrapper: the page's `PaintCtx`, which it dereferences to (and is
/// `AsMut` of, for `compose`), and the [`PageButtonSpec`]s. A page button is
/// declared here, not painted: the host keeps a widget for each and paints it
/// into the page's list where it was declared, under the clip in force there.
///
/// Until 2026-10-12 this was tuple vectors the host flattened again every
/// rebuild; until 2026-10-13, three recording layers (geometry, text,
/// glyphs) a toolkit bridge replayed each widget onto, losing what the flat
/// surface could not say (grooves, bevels, per-corner radii, the widgets'
/// own clips).
pub struct PageContent {
    pc: PaintCtx,
    pub buttons: Vec<PageButtonSpec>,
}

/// A button a page declared ([`PageContent::button`] and its siblings): the
/// button as made, what a click on it does, where in the page's list it was
/// declared and the clip in force there (page coordinates) — where and under
/// what the host paints it, and what a click outside it misses — and what it
/// was made from.
#[derive(Clone)]
pub struct PageButtonSpec {
    pub button: cce_ui::widget::Adapted<cce_ui::widget::Button>,
    pub action: AppAction,
    /// The page list's length when it was declared: the host paints it there,
    /// over what the page drew before it and under what it drew after (a
    /// glyph the page puts on a row, the list's raised scrollbar).
    pub at: usize,
    pub clip: Option<Rect>,
    /// Everything the button was made from but its rect, its action
    /// included: while a page declares a button with the same key, the host
    /// keeps the widget it made for it (its focus, its press) and only moves
    /// it. An icon's key carries the uploaded image's id, so a renderer
    /// rebuild, which hands out new ids, makes a new widget.
    pub key: String,
}

/// A page button as the host keeps it: the widget the context holds for a
/// button the page declared, in PAGE coordinates like the page's other
/// widgets, with what a click on it does.
pub struct PageButton {
    pub handle: cce_ui::widget::Handle<cce_ui::widget::Adapted<cce_ui::widget::Button>>,
    pub action: AppAction,
    /// Where in the page's list it is painted ([`PageButtonSpec::at`]).
    pub at: usize,
    /// The clip it was declared under (page coordinates): it is painted cut
    /// to it, and a point outside it misses the button.
    pub clip: Option<Rect>,
    /// What it was made from ([`PageButtonSpec::key`]): the next rebuild
    /// keeps this widget for a button declared the same way.
    pub key: String,
    /// Whether the pointer was over it at the rebuild that painted it — the
    /// host's move handler rebuilds when this goes stale, which is how a page
    /// button lights under the pointer.
    pub hovered: bool,
}

/// Bring the page buttons in `ctx` in line with what a page declared this
/// rebuild, in the order it declared them: a button declared as before (the
/// same key: face, label, colours, action) keeps its widget — its focus, a
/// press not yet released — moved to where it was declared now; a new one
/// gets a widget; one the page no longer declares loses its. A button its
/// clip leaves nothing of gets none.
pub fn sync_page_buttons(buttons: &mut Vec<PageButton>, specs: Vec<PageButtonSpec>, ctx: &mut cce_ui::context::UiContext) {
    use cce_ui::widget::WidgetHost;
    let mut old: Vec<Option<PageButton>> = std::mem::take(buttons).into_iter().map(Some).collect();
    for spec in specs {
        let base = spec.button.base();
        let r = rect(base.x, base.y, base.w, base.h);
        if let Some(c) = spec.clip {
            if r.x.max(c.x) >= (r.x + r.width).min(c.x + c.width) || r.y.max(c.y) >= (r.y + r.height).min(c.y + c.height) {
                continue;
            }
        }
        let kept = old.iter_mut().find(|o| o.as_ref().is_some_and(|b| b.key == spec.key)).and_then(Option::take);
        let handle = match kept {
            Some(b) => {
                ctx[b.handle].set_rect(r.x, r.y, r.width, r.height);
                b.handle
            }
            None => ctx.insert(spec.button),
        };
        buttons.push(PageButton { handle, action: spec.action, at: spec.at, clip: spec.clip, key: spec.key, hovered: false });
    }
    for b in old.into_iter().flatten() {
        ctx.remove(b.handle);
    }
}

/// A finished [`PageContent`]: the page's list, still in page coordinates,
/// and its buttons.
pub struct PageLayers {
    pub list: DisplayList,
    pub buttons: Vec<PageButtonSpec>,
}

/// One text run of a finished page, read back out of its list.
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

/// A text colour given as `[r, g, b, a]` in 0..1, as a `PaintCtx` takes it:
/// sRGB bytes, truncated.
pub fn rgb(c: [f32; 4]) -> [u8; 3] {
    [(c[0] * 255.0) as u8, (c[1] * 255.0) as u8, (c[2] * 255.0) as u8]
}

/// A rect from its corner and size.
pub fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect { x, y, width: w, height: h }
}

/// Place the widget `h` in the block `(x, y, w, h)` and paint it there, as a
/// `compose::Form` does its widgets: what a page does for the rows of a list
/// it draws itself.
pub fn place_and_paint<T: cce_ui::widget::WidgetHost + 'static>(
    pc: &mut PaintCtx,
    h: cce_ui::widget::Handle<T>,
    r: Rect,
    ctx: &mut cce_ui::context::UiContext,
) {
    cce_ui::compose::place_h(h, r.x, r.y, r.width, r.height, ctx);
    if let Some(w) = ctx.get(h) {
        cce_ui::widget::painter::paint_root_into(ctx, w, pc);
    }
}

impl PageContent {
    pub fn new() -> Self {
        Self { pc: PaintCtx::new(), buttons: Vec::new() }
    }

    /// The page's list and its buttons. The clips must be balanced.
    pub fn finish(self) -> PageLayers {
        PageLayers { list: self.pc.finish(), buttons: self.buttons }
    }

    /// Declare a button, under the clip in force.
    fn declare(&mut self, button: cce_ui::widget::Adapted<cce_ui::widget::Button>, action: AppAction, made_of: String) {
        let key = format!("{made_of}|{action:?}");
        let (at, clip) = (self.pc.len(), self.pc.current_clip());
        self.buttons.push(PageButtonSpec { button, action, at, clip, key });
    }

    pub fn button(&mut self, label: &str, x: f32, y: f32, w: f32, h: f32,
                  bg: [f32; 4], hover_bg: [f32; 4], label_color: [f32; 4],
                  action: AppAction) {
        let btn = cce_ui::widget::Button::new(x, y, w, h)
            .with_label(label)
            .with_bg(bg)
            .with_hover_bg(hover_bg)
            .with_label_color(label_color);
        self.declare(btn, action, format!("label|{label}|{bg:?}|{hover_bg:?}|{label_color:?}"));
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
        let image = cce_ui::upload_icon(icon, 32);
        if let Some((id, iw, ih)) = image {
            btn = btn.with_icon(id, iw as f32, ih as f32).with_icon_alpha(alpha);
        }
        self.declare(btn, action, format!("icon|{icon}|{image:?}|{label}|{bg:?}|{hover_bg:?}|{label_color:?}|{alpha}"));
    }

    /// A row of a list, in the toolkit's list style (`Button::new_list_row`,
    /// what cce-mail, cce-fonts and cce-calendar rows wear): no plate,
    /// transparent until hovered, the shared row wash. A plain [`button`]
    /// with a transparent face is NOT that — under `control_relief` it still
    /// gets a control plate, edges only, so every idle row of a list wore a
    /// carved ring. No colours to pass: the row asks for its own.
    ///
    /// [`button`]: PageContent::button
    pub fn list_row(&mut self, x: f32, y: f32, w: f32, h: f32, action: AppAction) {
        let btn = cce_ui::widget::Button::new_list_row(x, y, w, h);
        self.declare(btn, action, "row".to_string());
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
        let image = cce_ui::upload_icon_tinted(icon, 32, cce_ui::icon_tint(label_color));
        if let Some((id, iw, ih)) = image {
            btn = btn.with_icon(id, iw as f32, ih as f32).with_icon_alpha(label_color[3]);
        }
        self.declare(btn, action, format!("tinted|{icon}|{image:?}|{label}|{bg:?}|{hover_bg:?}|{label_color:?}"));
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
        self.declare(btn, action, format!("left|{label}|{bg:?}|{hover_bg:?}|{label_color:?}"));
    }
}

impl Default for PageContent {
    fn default() -> Self {
        Self::new()
    }
}

impl std::ops::Deref for PageContent {
    type Target = PaintCtx;
    fn deref(&self) -> &PaintCtx {
        &self.pc
    }
}

impl std::ops::DerefMut for PageContent {
    fn deref_mut(&mut self) -> &mut PaintCtx {
        &mut self.pc
    }
}

impl AsMut<PaintCtx> for PageContent {
    fn as_mut(&mut self) -> &mut PaintCtx {
        &mut self.pc
    }
}

impl PageLayers {
    /// Each fill where it shows: cut to the clip it was drawn under, and left
    /// out when that clip leaves none of it. A fill is a quad, a rounded rect
    /// or a bevel's plate — what a page's extent has always been measured by;
    /// a control's inset face and the carves are not fills.
    pub fn fills(&self) -> Vec<Rect> {
        self.list
            .items
            .iter()
            .filter_map(|item| {
                let r = match item.prim {
                    Prim::Quad { rect, .. } | Prim::RoundedRect { rect, .. } | Prim::Bevel { rect, .. } => rect,
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
        self.list
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
        self.list
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
        let texts = self.list.items.iter().filter_map(|item| match item.prim {
            Prim::Text { y, font_size, .. } => Some(y + font_size),
            _ => None,
        });
        let buttons = self.buttons.iter().map(|b| {
            let base = cce_ui::widget::WidgetHost::base(&b.button);
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
                row.draw(label_w, line_h, false, move |pc, r, _| pc.text(label, r.x, r.y, size, rgb(label_color)));
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
    use cce_ui::widget::WidgetHost;

    /// A page paints into its one list in call order, and declares its
    /// buttons beside it: a button is not painted by the page, and it carries
    /// where it was declared and the clip in force there.
    #[test]
    fn a_page_paints_in_call_order_and_declares_its_buttons() {
        let mut pc = PageContent::new();
        pc.quad(rect(0.0, 0.0, 10.0, 10.0), [1.0; 4]);
        pc.push_clip(rect(0.0, 0.0, 100.0, 50.0));
        pc.text("a", 5.0, 5.0, 12.0, [255; 3]);
        pc.button("Go", 0.0, 20.0, 40.0, 20.0, [0.0; 4], [0.0; 4], [1.0; 4], AppAction::Exit);
        pc.pop_clip();
        pc.list_row(0.0, 60.0, 40.0, 20.0, AppAction::Wake);
        let layers = pc.finish();
        let kinds: Vec<&str> = layers
            .list
            .items
            .iter()
            .map(|i| match i.prim {
                Prim::Quad { .. } => "quad",
                Prim::Text { .. } => "text",
                _ => "other",
            })
            .collect();
        assert_eq!(kinds, ["quad", "text"], "the buttons are not painted by the page");
        assert_eq!(layers.buttons[0].clip, Some(rect(0.0, 0.0, 100.0, 50.0)));
        assert_eq!(layers.buttons[1].clip, None);
        assert_eq!((layers.buttons[0].at, layers.buttons[1].at), (2, 2), "where they were declared");
        assert_eq!(layers.bottom(), 80.0, "the row's foot");
    }

    fn declared(label: &str, y: f32, action: AppAction) -> PageButtonSpec {
        let mut pc = PageContent::new();
        pc.push_clip(rect(0.0, 0.0, 200.0, 100.0));
        pc.button(label, 0.0, y, 40.0, 20.0, [0.0; 4], [0.0; 4], [1.0; 4], action);
        pc.pop_clip();
        pc.finish().buttons.remove(0)
    }

    /// A button declared as before keeps its widget, moved; one declared
    /// differently (its label, or what it does) gets a new widget and the old
    /// one goes; one its clip leaves nothing of gets none.
    #[test]
    fn a_button_declared_as_before_keeps_its_widget() {
        let mut ctx = cce_ui::context::UiContext::new();
        let mut buttons = Vec::new();
        sync_page_buttons(&mut buttons, vec![declared("Go", 10.0, AppAction::Exit), declared("Stop", 40.0, AppAction::Exit)], &mut ctx);
        let (go, stop) = (buttons[0].handle, buttons[1].handle);

        sync_page_buttons(
            &mut buttons,
            vec![declared("Go", 30.0, AppAction::Exit), declared("Stop", 40.0, AppAction::Wake), declared("Off", 150.0, AppAction::Exit)],
            &mut ctx,
        );
        assert_eq!(buttons.len(), 2, "the clipped-out button gets no widget");
        assert_eq!(buttons[0].handle, go, "declared as before: the same widget");
        assert_eq!(ctx[go].rect().1, 30.0, "moved to where it was declared now");
        assert_ne!(buttons[1].handle, stop, "another action: another widget");
        assert!(ctx.get(stop).is_none(), "the old one is gone");
    }
}
