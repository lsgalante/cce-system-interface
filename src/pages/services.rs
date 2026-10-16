use crate::app::{button_need, form_button, form_divider, wrap_to_width, AppAction, PageCanvas};
use cce_ui::context::UiContext;
use cce_ui::widget::Handle;
use cce_ui::widget::ScrollRegion;
use cce_ui::compose::{lay_row, Cell, PageLayoutBuilder, PageFlow};
use cce_ui::scene::layout::Rect;
use cce_ui::widget::{TextBox, InteractiveListItem};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ServiceInfo {
    pub name: String,
    pub description: String,
    /// systemd's LOAD column (`loaded`, `not-found`, `masked`, …), or
    /// [`UNLOADED`] for a unit file systemd has not loaded.
    pub load_state: String,
    pub active_state: String,
    pub sub_state: String,
    pub is_system: bool,
    /// `list-unit-files`' STATE (`enabled`, `disabled`, `static`, …): whether
    /// it starts at boot. None for a unit with no file of its own.
    pub file_state: Option<String>,
}

/// The load state given to a unit file `list-units` did not list: installed,
/// but nothing has pulled it in, so systemd has not loaded it. Startable all
/// the same — `systemctl start` loads it.
pub const UNLOADED: &str = "unloaded";

impl ServiceInfo {
    pub fn failed(&self) -> bool {
        self.active_state == "failed" || self.sub_state == "failed"
    }

    /// Confirmed running, as the Running filter counts it (the transport
    /// button reads [`transport_running`], which also counts where a click
    /// is heading).
    pub fn running(&self) -> bool {
        self.active_state == "active" || self.sub_state == "running"
    }

    fn controls(&self) -> RowControls {
        match self.load_state.as_str() {
            // A unit whose file is gone: nothing to start, but a failure
            // record to clear (what `systemctl reset-failed` is for).
            "not-found" if self.failed() => RowControls::Clear,
            "not-found" | "masked" => RowControls::None,
            _ => RowControls::Transport,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RowControls {
    Transport,
    Clear,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ServiceTab {
    #[default]
    System,
    User,
}

/// Which of the tab's units the list shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ServiceFilter {
    #[default]
    All,
    Running,
    Failed,
}

impl ServiceFilter {
    fn admits(self, s: &ServiceInfo) -> bool {
        match self {
            ServiceFilter::All => true,
            ServiceFilter::Running => s.running(),
            ServiceFilter::Failed => s.failed(),
        }
    }
}

/// What a click asks systemctl to do to a unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitAction {
    Start,
    Stop,
    Restart,
    Enable,
    Disable,
    /// `reset-failed`: forget a failure (a unit whose file is gone keeps
    /// showing as failed until it is cleared).
    ResetFailed,
}

impl UnitAction {
    fn verb(self) -> &'static str {
        match self {
            UnitAction::Start => "start",
            UnitAction::Stop => "stop",
            UnitAction::Restart => "restart",
            UnitAction::Enable => "enable",
            UnitAction::Disable => "disable",
            UnitAction::ResetFailed => "reset-failed",
        }
    }

    fn label(self) -> &'static str {
        match self {
            UnitAction::Start => "Start",
            UnitAction::Stop => "Stop",
            UnitAction::Restart => "Restart",
            UnitAction::Enable => "Enable",
            UnitAction::Disable => "Disable",
            UnitAction::ResetFailed => "Clear",
        }
    }

    /// Stop and Restart take a running service away. A user unit runs them
    /// with no password prompt, and the buttons sit on every row's edge —
    /// one stray click on the user bus (`dbus-broker`) would end the session.
    fn needs_confirm(self) -> bool {
        matches!(self, UnitAction::Stop | UnitAction::Restart)
    }

    /// The state written the instant the action starts, before systemd
    /// answers, so the transport button flips under the pointer.
    fn optimistic(self) -> Option<(&'static str, &'static str)> {
        match self {
            UnitAction::Start => Some(STARTING),
            UnitAction::Stop => Some(STOPPING),
            UnitAction::Restart => Some(RESTARTING),
            _ => None,
        }
    }
}

/// Units the session itself rides on: stopping (or restarting) one ends it.
/// Named in the confirmation, so it is not just one more "Stop x?".
fn session_critical(name: &str, is_system: bool) -> bool {
    const USER: &[&str] = &["dbus-broker.service", "dbus.service"];
    const SYSTEM: &[&str] =
        &["dbus-broker.service", "dbus.service", "systemd-logind.service", "cce-display-manager.service", "display-manager.service"];
    if is_system { SYSTEM.contains(&name) } else { USER.contains(&name) }
}

/// An action waiting on a click of its confirmation.
#[derive(Debug, Clone, PartialEq)]
pub struct Confirm {
    pub action: UnitAction,
    pub name: String,
    pub is_system: bool,
}

#[derive(Debug, Clone)]
pub struct ServicesState {
    pub loaded: bool,
    pub services: Vec<ServiceInfo>,
    pub active_tab: ServiceTab,
    pub filter: ServiceFilter,
    pub search_box: Handle<cce_ui::widget::Adapted<TextBox>>,
    pub list: ScrollRegion,
    pub items: Vec<Handle<cce_ui::widget::Adapted<cce_ui::widget::InteractiveListItem>>>,
    /// Actions sent and not yet answered. Their optimistic states are put
    /// back over each refresh: a system unit's action waits on the password
    /// prompt, and a 3 s refresh used to flip its button back under it.
    pub pending: Vec<(UnitAction, String, bool)>,
    pub confirm: Option<Confirm>,
    /// The last failed action, shown until dismissed or the next one.
    pub error: Option<String>,
    /// The unit whose details pane is open, and its `systemctl status`
    /// (None while it is being read).
    pub selected: Option<(String, bool)>,
    pub status: Option<String>,
    /// How results come back from the spawned commands (set by the host).
    pub sender: Option<calloop::channel::Sender<AppAction>>,
}

impl Default for ServicesState {
    fn default() -> Self {
        Self {
            loaded: false,
            services: Vec::new(),
            active_tab: ServiceTab::System,
            filter: ServiceFilter::All,
            search_box: Handle::none(),
            list: ScrollRegion::new(36.0, 6.0).with_frame(false).with_sink_behind(true),
            items: Vec::new(),
            pending: Vec::new(),
            confirm: None,
            error: None,
            selected: None,
            status: None,
            sender: None,
        }
    }
}

impl ServicesState {
    /// The page's state, its widgets inserted into `ctx`.
    pub fn new(ctx: &mut UiContext) -> Self {
        Self {
            search_box: ctx.insert(TextBox::new(String::new()).with_label("Filter Services")),
            ..Self::default()
        }
    }

    fn query(&self, ctx: &UiContext) -> String {
        if ctx[self.search_box].editing {
            ctx[self.search_box].edit_buffer.to_lowercase()
        } else {
            ctx[self.search_box].text.to_lowercase()
        }
    }

    fn on_tab(&self, s: &ServiceInfo) -> bool {
        s.is_system == (self.active_tab == ServiceTab::System)
    }

    /// Indices into `services` of the rows the list shows: the tab, the
    /// filter, then the search. The view paints exactly these and the click
    /// mapping resolves against them.
    pub fn visible(&self, ctx: &UiContext) -> Vec<usize> {
        let query = self.query(ctx);
        self.services
            .iter()
            .enumerate()
            .filter(|(_, s)| self.on_tab(s) && self.filter.admits(s))
            .filter(|(_, s)| s.name.to_lowercase().contains(&query) || s.description.to_lowercase().contains(&query))
            .map(|(i, _)| i)
            .collect()
    }

    fn find(&self, name: &str, is_system: bool) -> Option<&ServiceInfo> {
        self.services.iter().find(|s| s.name == name && s.is_system == is_system)
    }

    fn apply_optimistic(&mut self, action: UnitAction, name: &str, is_system: bool) {
        if let Some((active, sub)) = action.optimistic() {
            if let Some(srv) = self.services.iter_mut().find(|s| s.name == name && s.is_system == is_system) {
                srv.active_state = active.to_string();
                srv.sub_state = sub.to_string();
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum ServicesMessage {
    Refreshed(Vec<ServiceInfo>),
    SetTab(ServiceTab),
    SetFilter(ServiceFilter),
    /// A button asked for an action; Stop and Restart wait on [`Confirm`].
    Request(UnitAction, String, bool),
    ConfirmAction,
    CancelConfirm,
    Finished(UnitAction, String, bool, Result<(), String>),
    DismissError,
    /// Open (or, with None, close) a unit's details.
    Select(Option<(String, bool)>),
    StatusFetched(String, bool, String),
}

const TEXT_FG: [f32; 4] = [0.83, 0.83, 0.83, 1.0];
const TEXT_DIM: [f32; 4] = [0.53, 0.53, 0.60, 1.0];
/// Error text: light enough for small type on the dark plate (7.6:1; the
/// old [0.85, 0.25, 0.25] was 4.1:1, under AA for text this size).
const RED: [f32; 4] = [0.95, 0.55, 0.55, 1.0];
const WARN: [f32; 4] = [0.95, 0.75, 0.55, 1.0];
const NEUTRAL: ([f32; 4], [f32; 4], [f32; 4]) = ([0.16, 0.16, 0.24, 1.0], [0.25, 0.30, 0.26, 1.0], TEXT_FG);
const DANGER: ([f32; 4], [f32; 4], [f32; 4]) = ([0.30, 0.14, 0.14, 1.0], [0.45, 0.20, 0.20, 1.0], [0.98, 0.65, 0.65, 1.0]);
/// The least room the list keeps on a short window.
const LIST_MIN_H: f32 = 120.0;
/// The most status lines the details pane shows; the list keeps the rest.
const STATUS_LINES: usize = 12;
/// The fewest it shows however short the window: the unit line, Active.
const STATUS_MIN_LINES: usize = 2;

pub fn view(state: &mut ServicesState, cx: f32, cy: f32, cw: f32, ch: f32, _root_focused: bool, sec_focused: &[bool], layout: &mut PageFlow, ctx: &mut cce_ui::context::UiContext) -> PageCanvas {
    let mut final_pc = PageCanvas::new();
    let sec_w = 320.0f32;
    let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(1);

    builder.add_section_spanned(&mut final_pc, "", 1, sec_focused.first().copied().unwrap_or(false), |sec| {
        let sec_w = sec.cw;
        let mut form = sec.form();
        if !state.loaded {
            form.column().text("Loading systemd services...", 12.0, TEXT_DIM);
            sec.place(form, ctx);
            return;
        }
        // The list fills the page: the well's floor lands at the page's bottom.
        form.fill_height((cy + ch) - form.top() - sec.bottom_inset());
        let wrap_w = form.width();
        let (form_top, bottom_inset) = (form.top(), sec.bottom_inset());

        let tab_colors = |on: bool| {
            let face = if on { [0.20, 0.40, 0.65, 0.4] } else { [0.10, 0.10, 0.16, 0.3] };
            (face, [0.20, 0.20, 0.25, 0.15], [0.90, 0.90, 0.95, 1.0])
        };
        let (label1, label2) = if sec_w < 250.0 { ("System", "User") } else { ("System Services", "User Services") };

        let visible = state.visible(ctx);
        if state.items.len() != visible.len() {
            for h in state.items.drain(..) {
                ctx.remove(h);
            }
            for _ in 0..visible.len() {
                state.items.push(ctx.insert(InteractiveListItem::new("")));
            }
        }

        let active_tab = state.active_tab;
        let mut col = form.column();
        col.row(|r| {
            form_button(r, label1, 0.0, tab_colors(active_tab == ServiceTab::System),
                AppAction::Services(ServicesMessage::SetTab(ServiceTab::System)));
            form_button(r, label2, 0.0, tab_colors(active_tab == ServiceTab::User),
                AppAction::Services(ServicesMessage::SetTab(ServiceTab::User)));
        });

        // ── All / Running / Failed, counted on this tab ──
        let on_tab: Vec<&ServiceInfo> = state.services.iter().filter(|s| state.on_tab(s)).collect();
        let filters = [
            (ServiceFilter::All, format!("All ({})", on_tab.len())),
            (ServiceFilter::Running, format!("Running ({})", on_tab.iter().filter(|s| s.running()).count())),
            (ServiceFilter::Failed, format!("Failed ({})", on_tab.iter().filter(|s| s.failed()).count())),
        ];
        let filter = state.filter;
        col.row(|r| {
            for (f, label) in filters {
                form_button(r, label, 0.0, tab_colors(filter == f), AppAction::Services(ServicesMessage::SetFilter(f)));
            }
        });
        col.widget_h(ctx, state.search_box, cce_ui::layout::textbox_height());

        // ── Confirmation for Stop / Restart ──
        if let Some(c) = state.confirm.clone() {
            form_divider(&mut col);
            let question = format!("{} {}?", c.action.label(), c.name);
            col.block(|b| {
                b.lines(wrap_to_width(&question, wrap_w, 12.0), 12.0, WARN);
                if session_critical(&c.name, c.is_system) {
                    b.lines(wrap_to_width("Your session runs on this unit: this logs you out.", wrap_w, 11.0), 11.0, RED);
                }
            });
            let go = c.action.label();
            col.row(|r| {
                form_button(r, go, button_need(go), DANGER, AppAction::Services(ServicesMessage::ConfirmAction));
                form_button(r, "Cancel", button_need("Cancel"), NEUTRAL, AppAction::Services(ServicesMessage::CancelConfirm));
                r.space(0.0, true);
            });
        }

        // ── The last failed action ──
        if let Some(err) = state.error.clone() {
            form_divider(&mut col);
            let lines: Vec<String> = err.lines().flat_map(|l| wrap_to_width(l, wrap_w, 11.0)).take(6).collect();
            col.lines(lines, 11.0, RED);
            col.row(|r| {
                form_button(r, "Dismiss", button_need("Dismiss"), NEUTRAL, AppAction::Services(ServicesMessage::DismissError));
                r.space(0.0, true);
            });
        }

        // ── Details: enablement and `systemctl status` ──
        // Sized to the room the list leaves: a full status ran the list off
        // the page's bottom. Everything above the status is counted from the
        // toolkit's own control metrics.
        let status_budget = {
            let line_h = cce_ui::compose::form_line_height(11.0);
            let row_h = cce_ui::layout::button_height() + cce_ui::layout::control_gap();
            let divider = 2.0 * cce_ui::layout::control_gap();
            let mut above = 3.0 * row_h + cce_ui::layout::textbox_height() + line_h + 3.0 * divider + row_h;
            if state.confirm.is_some() {
                above += 2.0 * line_h + row_h + divider;
            }
            if state.error.is_some() {
                above += 6.0 * line_h + row_h + divider;
            }
            let room = (cy + ch) - form_top - bottom_inset - above - LIST_MIN_H;
            ((room / line_h).floor().max(0.0) as usize).clamp(STATUS_MIN_LINES, STATUS_LINES)
        };
        if let Some((name, is_system)) = state.selected.clone() {
            form_divider(&mut col);
            let unit = state.find(&name, is_system).cloned();
            let busy = state.pending.iter().any(|(_, n, s)| *n == name && *s == is_system);
            col.row(|r| {
                r.text_fill(name.clone(), 14.0, [0.35, 0.65, 0.90, 1.0]);
                if let Some(u) = &unit {
                    let colors = if busy { (NEUTRAL.0, NEUTRAL.0, TEXT_DIM) } else { NEUTRAL };
                    let toggle = match u.file_state.as_deref() {
                        Some("enabled") => Some(UnitAction::Disable),
                        Some("disabled") => Some(UnitAction::Enable),
                        _ => None,
                    };
                    if let Some(a) = toggle {
                        form_button(r, a.label(), button_need(a.label()), colors,
                            AppAction::Services(ServicesMessage::Request(a, name.clone(), is_system)));
                    }
                    if u.failed() && u.load_state != "not-found" {
                        form_button(r, "Clear Failure", button_need("Clear Failure"), colors,
                            AppAction::Services(ServicesMessage::Request(UnitAction::ResetFailed, name.clone(), is_system)));
                    }
                }
                form_button(r, "Close", button_need("Close"), NEUTRAL, AppAction::Services(ServicesMessage::Select(None)));
            });
            match &state.status {
                None => {
                    col.text("Reading status...", 11.0, TEXT_DIM);
                }
                Some(text) => {
                    col.lines(fit_status(text, status_budget, |l| wrap_to_width(l, wrap_w, 11.0)), 11.0, TEXT_FG);
                }
            }
        }
        form_divider(&mut col);

        let empty_msg = match state.filter {
            ServiceFilter::Failed if state.query(ctx).is_empty() => "No failed services",
            ServiceFilter::Running if state.query(ctx).is_empty() => "No running services",
            _ => "No services match the query",
        };
        let rows: Vec<ServiceInfo> = visible.iter().map(|&i| state.services[i].clone()).collect();
        let selected = state.selected.clone();
        let list = &mut state.list;
        let items = &state.items;
        col.fill(LIST_MIN_H, move |pc, rect, ctx| {
            let (list_box_x, list_box_y, list_box_w, list_box_h) = (rect.x, rect.y, rect.width, rect.height);
            // Dissolved List (Phase 6v): scroll state + frame prims are app-owned.
            list.set_rect(list_box_x, list_box_y, list_box_w, list_box_h);
            list.update_bounds(rows.len(), list_box_y, list_box_h);
            list.push_prims(pc.as_mut());
            let item_h = list.item_height;

            pc.push_clip(crate::app::rect(list_box_x, list_box_y, list_box_w, list_box_h));
                for (idx, service) in rows.iter().enumerate() {
                    if let Some(draw_y) = list.get_item_draw_y(idx, 4.0) {
                        // Transport + Restart, on the LEFT where the status dot
                        // used to be: the dot was reporting what the transport icon
                        // already says (play = stopped, stop = running), so the
                        // controls take the column it was using and the name/
                        // description run from there to the row's right inset.
                        //
                        // Sized on whether the icon set is actually THERE:
                        // `button_icon` falls back to the labels when it isn't, and
                        // a square button doesn't clip a label so much as replace it
                        // — the text centers, so both ends cut and "Restart" reads
                        // "sta". `upload_icon` caches per (name, px), so asking
                        // every row costs one hash lookup.
                        let icons_ok = cce_ui::upload_icon("play", 32).is_some();
                        let btn_h = cce_ui::layout::button_height();
                        // Without the icon set the controls are word buttons, as wide as
                        // their words.
                        let (btn_w, r_btn_w) = if icons_ok {
                            (btn_h, btn_h)
                        } else {
                            (button_need("Start").max(button_need("Stop")), button_need("Restart"))
                        };
                        // The controls, then the name and description to the row's end
                        // (no gutter for the scrollbar, which rides the list's centre line).
                        let row = Rect { x: list_box_x, y: draw_y, width: list_box_w, height: item_h };
                        let cells = lay_row(row, &[Cell::fixed(btn_w, btn_h), Cell::fixed(r_btn_w, btn_h), Cell::grow(item_h)]);
                        let (toggle_x, restart_x, btn_y) = (cells[0].x, cells[1].x, cells[0].y);
                        let (item_x, item_w) = (cells[2].x, cells[2].width);

                        // The second line, truncated to the room the row's text
                        // column has (the item insets its labels from its walls).
                        // Char-boundary safe: unit descriptions are free text.
                        let text_max_w = item_w - 2.0 * cce_ui::layout::CONTROL_TEXT_INSET;
                        let max_chars = ((text_max_w / 6.0) as usize).max(10);
                        let desc_truncated = cce_ui::widget::display::truncate_tail(&subtitle(service), max_chars);

                        // Rows dispatch as extra roots (the dissolved list is no parent).
                        let item = items[idx];
                        let item_btn = &mut ctx[item];
                        item_btn.title = service.name.clone();
                        item_btn.subtitle = Some(desc_truncated);
                        item_btn.selected = selected.as_ref().is_some_and(|(n, s)| *n == service.name && *s == service.is_system);
                        crate::app::place_and_paint(pc, item, crate::app::rect(item_x, draw_y, item_w, item_h), ctx);

                        let active_txt = [0.90, 0.90, 0.95, 1.0];
                        // No per-action tints: both controls wear the DE's themed
                        // button face, and what they DO is carried by the icon.
                        let face = cce_ui::color::button_background_color();
                        let face_hover = cce_ui::color::button_hover_color();
                        let request = |a: UnitAction| {
                            AppAction::Services(ServicesMessage::Request(a, service.name.clone(), service.is_system))
                        };

                        match service.controls() {
                            RowControls::Transport => {
                                // ONE transport button, showing the action it will
                                // take: play on a stopped service, stop on a running
                                // one. `transport_running` counts a transitional
                                // state as where it is heading, so a click flips it.
                                let running = transport_running(&service.active_state, &service.sub_state);
                                pc.button_icon(
                                    if running { "stop" } else { "play" },
                                    if running { "Stop" } else { "Start" },
                                    toggle_x, btn_y, btn_w, btn_h, face, face_hover, active_txt, 1.0,
                                    request(if running { UnitAction::Stop } else { UnitAction::Start }),
                                );
                                pc.button_icon(
                                    "refresh", "Restart",
                                    restart_x, btn_y, r_btn_w, btn_h, face, face_hover, active_txt, 1.0,
                                    request(UnitAction::Restart),
                                );
                            }
                            // A failed unit whose file is gone: one word button
                            // over both cells, clearing the failure record.
                            RowControls::Clear => {
                                let w = cells[1].x + cells[1].width - toggle_x;
                                pc.button("Clear", toggle_x, btn_y, w, btn_h, face, face_hover, active_txt,
                                    request(UnitAction::ResetFailed));
                            }
                            RowControls::None => {}
                        }
                    }
                }
            pc.pop_clip();
            // The scrollbar's fore copy, over the rows at the raise's fade.
            list.push_scrollbar_fore(pc.as_mut());

            if rows.is_empty() {
                let inset = cce_ui::layout::plate_padding();
                pc.text(empty_msg, list_box_x + inset, list_box_y + inset, 12.0, crate::app::rgb(TEXT_DIM));
            }
        });
        sec.place(form, ctx);
    });

    final_pc
}

/// The optimistic states written the instant a transport button is clicked,
/// before systemd has said anything back. Named because
/// [`transport_running`] has to agree with them: the whole point of writing
/// them is that the button's icon flips under the pointer instead of staying
/// stale until the next refresh lands.
const STARTING: (&str, &str) = ("activating", "starting");
const STOPPING: (&str, &str) = ("deactivating", "stopping");
const RESTARTING: (&str, &str) = ("activating", "restarting");

/// The row's second line: `failed` first when it is (composed before the
/// caller truncates, so a long description can never push it off the end —
/// a failed unit offers the same play button a stopped one does, and this
/// is the only sign it died), then why it has no controls, then whether it
/// starts at boot, then the description.
pub fn subtitle(s: &ServiceInfo) -> String {
    let mut parts: Vec<&str> = Vec::new();
    if s.failed() {
        parts.push("failed");
    }
    match s.load_state.as_str() {
        "not-found" => parts.push("not installed"),
        "masked" => parts.push("masked"),
        _ => {}
    }
    if let Some(f @ ("enabled" | "disabled")) = s.file_state.as_deref() {
        parts.push(f);
    }
    parts.push(match (s.description.is_empty(), s.load_state == UNLOADED) {
        (false, _) => &s.description,
        (true, true) => "not loaded",
        (true, false) => "No description",
    });
    parts.join("  ·  ")
}

/// `systemctl status` cut to what says how the unit is doing, in at most
/// `budget` lines (after `wrap`): the unit line and the header fields that
/// matter, then the NEWEST journal lines in the room left. The process tree
/// and resource counters go — they took half the pane.
pub fn fit_status(text: &str, budget: usize, wrap: impl Fn(&str) -> Vec<String>) -> Vec<String> {
    const KEEP: &[&str] = &["Loaded:", "Active:", "TriggeredBy:", "Main PID:", "Status:"];
    let (head, journal) = match text.split_once("\n\n") {
        Some((h, j)) => (h, j),
        None => (text, ""),
    };
    let mut lines: Vec<String> = head
        .lines()
        .enumerate()
        .filter(|(i, l)| *i == 0 || KEEP.iter().any(|k| l.trim_start().starts_with(k)))
        .flat_map(|(_, l)| wrap(l.trim_end()))
        .collect();
    lines.truncate(budget);
    let room = budget - lines.len();
    let tail: Vec<String> = journal.lines().filter(|l| !l.trim().is_empty()).flat_map(|l| wrap(l.trim_end())).collect();
    lines.extend(tail[tail.len().saturating_sub(room)..].iter().cloned());
    lines
}

/// Whether the row's single transport button offers Stop (`true`) or Start
/// (`false`) — which is also which icon it wears.
///
/// A transitional state counts as the state it is heading FOR, not the one it
/// is leaving. That is what makes the click feel like a toggle: a Start marks
/// the unit `activating` the moment it is pressed, and this reads that as
/// running, so the icon becomes Stop immediately.
pub fn transport_running(active_state: &str, sub_state: &str) -> bool {
    match active_state {
        "active" | "activating" | "reloading" => true,
        "deactivating" => false,
        _ => sub_state == "running",
    }
}

pub fn update(state: &mut ServicesState, msg: ServicesMessage, ctx: &mut UiContext) {
    match msg {
        ServicesMessage::Refreshed(new_services) => {
            state.loaded = true;
            let watched = state.selected.clone().and_then(|(n, s)| state.find(&n, s).cloned());
            state.services = new_services;
            // An action still in flight keeps its optimistic state: systemd's
            // answer, not the refresh, settles it.
            for (action, name, is_system) in state.pending.clone() {
                state.apply_optimistic(action, &name, is_system);
            }
            for h in state.items.drain(..) {
                ctx.remove(h);
            }
            if let Some((name, is_system)) = state.selected.clone() {
                match state.find(&name, is_system).cloned() {
                    None => {
                        state.selected = None;
                        state.status = None;
                    }
                    // Re-read the status when the unit changed under it.
                    Some(now) if Some(&now) != watched.as_ref() => fetch_status(state, &name, is_system),
                    Some(_) => {}
                }
            }
        }
        ServicesMessage::SetTab(tab) => {
            state.active_tab = tab;
            state.list.set_scroll_y(0.0);
            state.selected = None;
            state.status = None;
            for h in state.items.drain(..) {
                ctx.remove(h);
            }
        }
        ServicesMessage::SetFilter(filter) => {
            if state.filter != filter {
                state.filter = filter;
                state.list.set_scroll_y(0.0);
                for h in state.items.drain(..) {
                    ctx.remove(h);
                }
            }
        }
        ServicesMessage::Request(action, name, is_system) => {
            if action.needs_confirm() {
                state.confirm = Some(Confirm { action, name, is_system });
            } else {
                run_action(state, action, name, is_system);
            }
        }
        ServicesMessage::ConfirmAction => {
            if let Some(c) = state.confirm.take() {
                run_action(state, c.action, c.name, c.is_system);
            }
        }
        ServicesMessage::CancelConfirm => state.confirm = None,
        ServicesMessage::Finished(action, name, is_system, res) => {
            state.pending.retain(|(a, n, s)| !(*a == action && *n == name && *s == is_system));
            match res {
                Ok(()) => {
                    // Shown now; the refresh below confirms it.
                    if let Some(srv) = state.services.iter_mut().find(|s| s.name == name && s.is_system == is_system) {
                        match action {
                            UnitAction::Enable => srv.file_state = Some("enabled".into()),
                            UnitAction::Disable => srv.file_state = Some("disabled".into()),
                            _ => {}
                        }
                    }
                }
                Err(err) => {
                    state.error = Some(format!("Couldn't {} {name}: {err}", action.verb()));
                    // Its status usually says why: open it.
                    state.selected = Some((name.clone(), is_system));
                }
            }
            if state.selected.as_ref().is_some_and(|(n, s)| *n == name && *s == is_system) {
                fetch_status(state, &name, is_system);
            }
            refresh(state);
        }
        ServicesMessage::DismissError => state.error = None,
        ServicesMessage::Select(sel) => match sel {
            Some((name, is_system)) if state.selected.as_ref() != Some(&(name.clone(), is_system)) => {
                state.selected = Some((name.clone(), is_system));
                fetch_status(state, &name, is_system);
            }
            Some(_) => {}
            None => {
                state.selected = None;
                state.status = None;
            }
        },
        ServicesMessage::StatusFetched(name, is_system, text) => {
            if state.selected.as_ref() == Some(&(name, is_system)) {
                state.status = Some(text);
            }
        }
    }
}

/// Send `action`, unless the same unit already has one in flight (a second
/// click on a system unit used to queue a second password prompt).
fn run_action(state: &mut ServicesState, action: UnitAction, name: String, is_system: bool) {
    if state.pending.iter().any(|(_, n, s)| *n == name && *s == is_system) {
        return;
    }
    state.error = None;
    state.apply_optimistic(action, &name, is_system);
    state.pending.push((action, name.clone(), is_system));
    let Some(tx) = state.sender.clone() else { return };
    tokio::spawn(async move {
        let res = service_action(&name, action, is_system).await;
        let _ = tx.send(AppAction::Services(ServicesMessage::Finished(action, name, is_system, res)));
    });
}

fn fetch_status(state: &mut ServicesState, name: &str, is_system: bool) {
    state.status = None;
    let Some(tx) = state.sender.clone() else { return };
    let name = name.to_string();
    tokio::spawn(async move {
        let text = unit_status(&name, is_system).await;
        let _ = tx.send(AppAction::Services(ServicesMessage::StatusFetched(name, is_system, text)));
    });
}

/// A refresh now rather than at the next 3 s tick, after an action.
fn refresh(state: &ServicesState) {
    // An action may have enabled, disabled or masked a unit.
    crate::unit_files::invalidate();
    let Some(tx) = state.sender.clone() else { return };
    tokio::spawn(async move {
        let _ = tx.send(AppAction::Services(ServicesMessage::Refreshed(fetch_services().await)));
    });
}

// ── Background Fetching ──

fn systemctl(is_system: bool) -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new("systemctl");
    if !is_system {
        cmd.arg("--user");
    }
    cmd.env("SYSTEMD_COLORS", "0");
    cmd
}

async fn stdout_of(mut cmd: tokio::process::Command) -> String {
    match crate::output_within(&mut cmd, crate::FETCH_TIMEOUT).await {
        Ok(o) => String::from_utf8_lossy(&o.stdout).into_owned(),
        Err(_) => String::new(),
    }
}

pub async fn fetch_services() -> Vec<ServiceInfo> {
    let units = |is_system: bool| {
        let mut c = systemctl(is_system);
        c.args(["list-units", "--type=service", "--all", "--no-legend", "--plain"]);
        stdout_of(c)
    };
    // Cached: ~280 ms for the system scope, and it changes only when a unit
    // is installed, removed, enabled or disabled (`crate::unit_files`).
    let files = |is_system: bool| crate::unit_files::list(is_system, "service");
    let (sys_units, sys_files, user_units, user_files) = tokio::join!(units(true), files(true), units(false), files(false));
    let mut services = merge_units(&sys_units, &sys_files, true);
    services.extend(merge_units(&user_units, &user_files, false));
    services.sort_by_key(|a| a.name.to_lowercase());
    services
}

/// `list-units --all` (what systemd has LOADED) joined with `list-unit-files`
/// (what is installed, and whether it starts at boot). The first alone left
/// out every unit nothing had pulled in — 134 system units here, disabled
/// ones like avahi-dnsconfd among them — so they could not be started.
///
/// Dropped: template files (`foo@.service`, nothing to start without an
/// instance), aliases (another name for a unit already listed), and
/// not-found units that are not failed (a reference to a file that does not
/// exist; a FAILED one stays, to be cleared).
pub fn merge_units(units: &str, files: &str, is_system: bool) -> Vec<ServiceInfo> {
    let file_states: std::collections::HashMap<&str, &str> = files
        .lines()
        .filter_map(|l| {
            let mut p = l.split_whitespace();
            Some((p.next()?, p.next()?))
        })
        .collect();
    let file_state = |name: &str| -> Option<String> {
        file_states
            .get(name)
            .or_else(|| {
                // An instance takes its template's state.
                let (prefix, _) = name.split_once('@')?;
                file_states.get(format!("{prefix}@.service").as_str())
            })
            .map(|s| s.to_string())
    };
    let mut out: Vec<ServiceInfo> = units
        .lines()
        .filter_map(|line| parse_service_line(line, is_system))
        .filter(|s| s.load_state != "not-found" || s.failed())
        .map(|mut s| {
            s.file_state = file_state(&s.name);
            s
        })
        .collect();
    for (name, st) in &file_states {
        if name.ends_with("@.service") || *st == "alias" || out.iter().any(|s| s.name == *name) {
            continue;
        }
        out.push(ServiceInfo {
            name: name.to_string(),
            description: String::new(),
            load_state: UNLOADED.to_string(),
            active_state: "inactive".to_string(),
            sub_state: "dead".to_string(),
            is_system,
            file_state: Some(st.to_string()),
        });
    }
    out
}

fn parse_service_line(line: &str, is_system: bool) -> Option<ServiceInfo> {
    let cleaned = line.trim_start_matches('●').trim();
    let parts: Vec<&str> = cleaned.split_whitespace().collect();
    if parts.len() >= 4 && parts[0].ends_with(".service") {
        Some(ServiceInfo {
            name: parts[0].to_string(),
            load_state: parts[1].to_string(),
            active_state: parts[2].to_string(),
            sub_state: parts[3].to_string(),
            description: parts[4..].join(" "),
            is_system,
            file_state: None,
        })
    } else {
        None
    }
}

/// Run one action and say how it went. A system unit goes through pkexec
/// (126/127 are pkexec's own: not authorized, or the dialog dismissed).
async fn service_action(name: &str, action: UnitAction, is_system: bool) -> Result<(), String> {
    let mut cmd = if is_system {
        let mut c = tokio::process::Command::new("pkexec");
        c.arg("systemctl");
        c
    } else {
        let mut c = tokio::process::Command::new("systemctl");
        c.arg("--user");
        c
    };
    cmd.args([action.verb(), "--", name]);
    let out = cmd.output().await.map_err(|e| format!("could not run systemctl: {e}"))?;
    if out.status.success() {
        return Ok(());
    }
    let text = String::from_utf8_lossy(&out.stderr).trim().to_string();
    Err(match out.status.code() {
        Some(126) | Some(127) if is_system => "authorization was cancelled or denied".to_string(),
        _ if text.is_empty() => format!("systemctl exited with {}", out.status),
        _ => text,
    })
}

/// `systemctl status` with its last journal lines. It exits non-zero for a
/// unit that is not running, which is still an answer worth showing.
async fn unit_status(name: &str, is_system: bool) -> String {
    let mut cmd = systemctl(is_system);
    cmd.args(["status", "--no-pager", "--full", "--lines=6", "--", name]);
    match cmd.output().await {
        Ok(o) => {
            let out = String::from_utf8_lossy(&o.stdout).trim_end().to_string();
            if out.is_empty() { String::from_utf8_lossy(&o.stderr).trim().to_string() } else { out }
        }
        Err(e) => format!("could not run systemctl: {e}"),
    }
}

impl crate::pages::AppPage for ServicesState {
    // Sections: [Services]
    fn section_widgets(&mut self) -> Vec<Vec<cce_ui::widget::WidgetId>> {
        // Mirrors the view's `!loaded` branch: the search box is only painted (and
        // so only registered) once the unit list has landed. The group count stays
        // 1 either way — an empty outer Vec would kill the ctrl-nav entry point.
        if self.loaded {
            vec![vec![self.search_box.id()]]
        } else {
            vec![Vec::new()]
        }
    }

    fn view(
        &mut self,
        cx: f32,
        cy: f32,
        cw: f32,
        ch: f32,
        root_focused: bool,
        sec_focused: &[bool],
        layout: &mut PageFlow,
        ctx: &mut cce_ui::context::UiContext,
    ) -> crate::app::PageCanvas {
        view(self, cx, cy, cw, ch, root_focused, sec_focused, layout, ctx)
    }

    /// A row click opens that unit's details.
    fn propagate_widget_changes(
        &mut self,
        events: &[(cce_ui::widget::WidgetId, cce_ui::widget::WidgetEvent)],
        actions: &mut Vec<crate::app::AppAction>,
        ctx: &mut UiContext,
    ) {
        let clicked = events
            .iter()
            .filter(|(_, ev)| *ev == cce_ui::widget::WidgetEvent::Clicked)
            .filter_map(|(id, _)| self.items.iter().position(|h| h.id() == *id))
            .min();
        let Some(idx) = clicked else { return };
        if let Some(&i) = self.visible(ctx).get(idx) {
            let s = &self.services[i];
            actions.push(AppAction::Services(ServicesMessage::Select(Some((s.name.clone(), s.is_system)))));
        }
    }

    // Filtered by `get_item_draw_y`, the same predicate the view's paint loop virtualizes
    // on — a scrolled-out row keeps its last-drawn rect and would otherwise win the
    // hit-test against the row actually on screen. See the note in packages.rs.
    fn extra_dispatch_roots(&mut self) -> Vec<cce_ui::widget::WidgetId> {
        let (list, items) = (&self.list, &self.items);
        items
            .iter()
            .enumerate()
            .filter(|(idx, _)| list.get_item_draw_y(*idx, 4.0).is_some())
            .map(|(_, i)| i.id())
            .collect()
    }


    fn handle_pointer_move(
        &mut self,
        lx: f32,
        ly: f32,
        _actions: &mut Vec<crate::app::AppAction>,
        _ctx: &mut cce_ui::context::UiContext,
    ) -> bool {
        self.loaded && self.list.cursor_moved(lx, ly)
    }

    fn handle_pointer_down(&mut self, lx: f32, ly: f32, _ctx: &mut cce_ui::context::UiContext) -> bool {
        self.loaded && self.list.press(lx, ly)
    }

    fn handle_pointer_up(&mut self, _ctx: &mut cce_ui::context::UiContext) -> bool {
        self.list.release()
    }

    fn handle_mouse_wheel(&mut self, delta: &cce_ui::widget::MouseScrollDelta, lx: f32, ly: f32) -> bool {
        self.loaded && self.list.wheel(delta, lx, ly)
    }

    fn handle_key_input(&mut self, event: &cce_ui::widget::KeyEvent) -> bool {
        self.loaded && self.list.keyboard(event)
    }

    fn tick(&mut self, dt: f32) -> bool {
        self.list.tick(dt)
    }

    fn frameless_lists(&self) -> Vec<&ScrollRegion> {
        vec![&self.list]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(name: &str, load: &str, active: &str, sub: &str, is_system: bool) -> ServiceInfo {
        ServiceInfo {
            name: name.into(),
            description: format!("{name} desc"),
            load_state: load.into(),
            active_state: active.into(),
            sub_state: sub.into(),
            is_system,
            file_state: None,
        }
    }

    fn loaded(ui: &mut UiContext) -> ServicesState {
        let mut st = ServicesState::new(ui);
        update(&mut st, ServicesMessage::Refreshed(vec![
            unit("a.service", "loaded", "active", "running", false),
            unit("b.service", "loaded", "failed", "failed", false),
            unit("c.service", "loaded", "inactive", "dead", false),
            unit("dbus-broker.service", "loaded", "active", "running", false),
            unit("s.service", "loaded", "active", "running", true),
        ]), ui);
        st.active_tab = ServiceTab::User;
        st
    }

    fn texts(st: &mut ServicesState, ui: &mut UiContext) -> Vec<String> {
        let mut layout = cce_ui::compose::PageFlow::new();
        let pc = view(st, 10.0, 20.0, 800.0, 600.0, false, &[false], &mut layout, ui).finish();
        pc.texts().into_iter().map(|t| t.text).collect()
    }

    #[test]
    fn settled_states_pick_the_opposite_action() {
        assert!(transport_running("active", "running"));
        assert!(!transport_running("inactive", "dead"));
        assert!(!transport_running("failed", "failed"));
        // sub_state alone is enough — some units report it without
        // active_state catching up.
        assert!(transport_running("something-else", "running"));
    }

    #[test]
    fn a_click_flips_the_icon_before_any_refresh() {
        assert!(transport_running(STARTING.0, STARTING.1), "Start must show Stop immediately");
        assert!(!transport_running(STOPPING.0, STOPPING.1), "Stop must show Start immediately");
        // Restart keeps the unit running throughout, so the transport button
        // must not flicker to Start while it cycles.
        assert!(transport_running(RESTARTING.0, RESTARTING.1), "Restart must keep showing Stop");
    }

    #[test]
    fn the_failed_marker_leads_and_survives_truncation() {
        let mut u = unit("x.service", "loaded", "failed", "failed", true);
        u.description = "x".repeat(500);
        u.file_state = Some("enabled".into());
        let line = subtitle(&u);
        assert!(line.starts_with("failed  ·  enabled"), "{line}");
        let shown = cce_ui::widget::display::truncate_tail(&line, 20);
        assert!(shown.starts_with("failed"), "got {shown:?}");
        let mut quiet = unit("y.service", UNLOADED, "inactive", "dead", true);
        quiet.description.clear();
        quiet.file_state = Some("static".into());
        assert_eq!(subtitle(&quiet), "not loaded", "static is not worth a word");
        let gone = unit("gitsite.service", "not-found", "failed", "failed", false);
        assert!(subtitle(&gone).starts_with("failed  ·  not installed"));
    }

    /// Unit files nothing loaded are listed and startable; templates,
    /// aliases and harmless not-found references are not.
    #[test]
    fn unit_files_join_the_loaded_units() {
        let units = "a.service loaded active running A\n\
                     ghost.service not-found inactive dead ghost.service\n\
                     gitsite.service not-found failed failed gitsite.service\n\
                     getty@tty1.service loaded active running Getty on tty1\n";
        let files = "a.service enabled enabled\n\
                     avahi-dnsconfd.service disabled disabled\n\
                     getty@.service enabled enabled\n\
                     dbus.service alias -\n\
                     a-template@.service disabled -\n";
        let list = merge_units(units, files, true);
        let names: Vec<&str> = list.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["a.service", "gitsite.service", "getty@tty1.service", "avahi-dnsconfd.service"]);
        assert_eq!(list[0].file_state.as_deref(), Some("enabled"));
        assert_eq!(list[2].file_state.as_deref(), Some("enabled"), "an instance takes its template's state");
        assert_eq!(list[3].load_state, UNLOADED);
        assert_eq!(list[3].controls(), RowControls::Transport, "an unloaded unit can be started");
        assert_eq!(list[1].controls(), RowControls::Clear, "a failed not-found unit can be cleared");
        let masked = unit("m.service", "masked", "inactive", "dead", true);
        assert_eq!(masked.controls(), RowControls::None);
    }

    #[test]
    fn stop_and_restart_wait_for_a_confirmation_and_name_the_risk() {
        let mut ui = UiContext::new();
        let mut st = loaded(&mut ui);
        update(&mut st, ServicesMessage::Request(UnitAction::Stop, "dbus-broker.service".into(), false), &mut ui);
        assert!(st.pending.is_empty(), "nothing runs before the confirmation");
        assert!(st.confirm.is_some());
        let t = texts(&mut st, &mut ui);
        assert!(t.iter().any(|s| s.contains("Stop dbus-broker.service?")), "{t:?}");
        assert!(t.iter().any(|s| s.contains("logs you out")), "{t:?}");
        update(&mut st, ServicesMessage::CancelConfirm, &mut ui);
        assert!(st.confirm.is_none() && st.pending.is_empty());

        update(&mut st, ServicesMessage::Request(UnitAction::Restart, "a.service".into(), false), &mut ui);
        update(&mut st, ServicesMessage::ConfirmAction, &mut ui);
        assert_eq!(st.pending, [(UnitAction::Restart, "a.service".to_string(), false)]);
        // Start needs no confirmation.
        update(&mut st, ServicesMessage::Request(UnitAction::Start, "c.service".into(), false), &mut ui);
        assert_eq!(st.pending.len(), 2);
        assert!(st.confirm.is_none());
    }

    /// A refresh while an action waits (a password prompt) keeps the
    /// button where the click put it, and a second click sends nothing.
    #[test]
    fn a_pending_action_outlives_refreshes_and_blocks_a_second() {
        let mut ui = UiContext::new();
        let mut st = loaded(&mut ui);
        update(&mut st, ServicesMessage::Request(UnitAction::Start, "c.service".into(), false), &mut ui);
        update(&mut st, ServicesMessage::Refreshed(vec![unit("c.service", "loaded", "inactive", "dead", false)]), &mut ui);
        let c = st.find("c.service", false).unwrap();
        assert!(transport_running(&c.active_state, &c.sub_state), "still showing Stop");
        update(&mut st, ServicesMessage::Request(UnitAction::Start, "c.service".into(), false), &mut ui);
        assert_eq!(st.pending.len(), 1);
        update(&mut st, ServicesMessage::Finished(UnitAction::Start, "c.service".into(), false, Ok(())), &mut ui);
        assert!(st.pending.is_empty());
    }

    /// A failure is reported, and opens the unit's status.
    #[test]
    fn a_failed_action_is_shown() {
        let mut ui = UiContext::new();
        let mut st = loaded(&mut ui);
        update(&mut st, ServicesMessage::Request(UnitAction::Start, "c.service".into(), false), &mut ui);
        let err = "Job for c.service failed because the control process exited with error code.".to_string();
        update(&mut st, ServicesMessage::Finished(UnitAction::Start, "c.service".into(), false, Err(err)), &mut ui);
        assert_eq!(st.selected, Some(("c.service".to_string(), false)));
        let t = texts(&mut st, &mut ui);
        assert!(t.iter().any(|s| s.contains("Couldn't start c.service")), "{t:?}");
        update(&mut st, ServicesMessage::StatusFetched("c.service".into(), false, "× c.service\n     Active: failed".into()), &mut ui);
        let t = texts(&mut st, &mut ui);
        assert!(t.iter().any(|s| s.contains("Active: failed")), "{t:?}");
    }

    /// The status keeps what says how the unit is doing and the newest
    /// journal lines, within the budget.
    #[test]
    fn status_is_condensed_to_its_budget() {
        let text = "● a.service - A\n     Loaded: loaded (/x; static)\n     Active: active (running) since x\n Invocation: d36\n   Main PID: 1 (a)\n      Tasks: 7\n     Memory: 2.3M\n     CGroup: /user.slice/a\n             └─1 /usr/bin/a\n\nOct 14 j1\nOct 14 j2\nOct 14 j3\n";
        let one = |l: &str| vec![l.to_string()];
        let all = fit_status(text, 12, one);
        assert_eq!(all.len(), 7, "{all:?}");
        assert!(all[0].starts_with("● a.service"));
        assert!(!all.iter().any(|l| l.contains("Tasks") || l.contains("CGroup") || l.contains("└─")));
        let tight = fit_status(text, 5, one);
        assert_eq!(tight.len(), 5);
        assert_eq!(tight.last().map(String::as_str), Some("Oct 14 j3"), "the newest journal line stays");
        assert_eq!(fit_status(text, 2, one).len(), 2);
    }

    #[test]
    fn the_filter_counts_and_narrows_the_tab() {
        let mut ui = UiContext::new();
        let mut st = loaded(&mut ui);
        assert_eq!(st.visible(&ui).len(), 4, "the user tab only");
        update(&mut st, ServicesMessage::SetFilter(ServiceFilter::Failed), &mut ui);
        assert_eq!(st.visible(&ui).iter().map(|&i| st.services[i].name.as_str()).collect::<Vec<_>>(), ["b.service"]);
        update(&mut st, ServicesMessage::SetFilter(ServiceFilter::Running), &mut ui);
        assert_eq!(st.visible(&ui).len(), 2);
    }

    /// A row click names the row painted there, through the filter.
    #[test]
    fn a_row_click_opens_that_units_details() {
        let mut ui = UiContext::new();
        let mut st = loaded(&mut ui);
        update(&mut st, ServicesMessage::SetFilter(ServiceFilter::Running), &mut ui);
        texts(&mut st, &mut ui);
        ui[st.items[1]].just_clicked = true;
        let events = ui.take_events();
        let mut actions = Vec::new();
        crate::pages::AppPage::propagate_widget_changes(&mut st, &events, &mut actions, &mut ui);
        assert!(
            matches!(actions.as_slice(), [AppAction::Services(ServicesMessage::Select(Some((n, false))))] if n == "dbus-broker.service"),
            "{actions:?}"
        );
    }
}
