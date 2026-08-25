use crate::app::{PageContent, SectionContextExt};
use crate::scroll_region::ScrollRegion;
use cce_ui::layout::{render_widget, PageLayoutBuilder, LayoutStrategy, RenderTarget};
use cce_ui::widget::{TextBox, StatusDot, DotStatus, InteractiveListItem, WidgetHost};

#[derive(Debug, Clone)]
pub struct ServiceInfo {
    pub name: String,
    pub description: String,
    pub active_state: String,
    pub sub_state: String,
    pub is_system: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceTab {
    System,
    User,
}

impl Default for ServiceTab {
    fn default() -> Self {
        ServiceTab::System
    }
}

#[derive(Debug, Clone)]
pub struct ServicesState {
    pub loaded: bool,
    pub services: Vec<ServiceInfo>,
    pub active_tab: ServiceTab,
    pub search_box: cce_ui::widget::Adapted<TextBox>,
    pub list: ScrollRegion,
    pub items: Vec<cce_ui::widget::Adapted<cce_ui::widget::InteractiveListItem>>,
}

impl Default for ServicesState {
    fn default() -> Self {
        Self {
            loaded: false,
            services: Vec::new(),
            active_tab: ServiceTab::System,
            search_box: TextBox::new(String::new()).with_label("Filter Services"),
            list: ScrollRegion::new(36.0, 6.0).with_frame(false),
            items: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ServicesMessage {
    Refreshed(Vec<ServiceInfo>),
    SetTab(ServiceTab),
    Start(String, bool),
    Stop(String, bool),
    Restart(String, bool),
}

const TEXT_DIM: [f32; 4] = [0.53, 0.53, 0.60, 1.0];

pub fn view(state: &mut ServicesState, cx: f32, cy: f32, cw: f32, ch: f32, _root_focused: bool, sec_focused: &[bool], layout: &mut dyn LayoutStrategy, ctx: &mut cce_ui::context::UiContext) -> PageContent {
    let mut final_pc = PageContent::new();
    let sec_w = 320.0f32;
    let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(1);

    builder.add_section_spanned(&mut final_pc, "", 1, sec_focused.first().copied().unwrap_or(false), |sec| {
        let sec_w = sec.cw;
        if !state.loaded {
            sec.text("Loading systemd services...", 12.0, 0.0, 12.0, TEXT_DIM);
        } else {
            // Tab header buttons: System Services, User Services
            let mut stack = sec.vstack(8.0);
            let tab_h = 28.0;
            let active_bg = [0.20, 0.40, 0.65, 0.4];
            let inactive_bg = [0.10, 0.10, 0.16, 0.3];
            let hover_bg = [0.20, 0.20, 0.25, 0.15];

            let label1 = if stack.context.cw < 250.0 { "System" } else { "System Services" };
            let label2 = if stack.context.cw < 250.0 { "User" } else { "User Services" };

            stack.add_row(2, 8.0, tab_h, |ctx, i, x, w| {
                if i == 0 {
                    ctx.button(
                        label1,
                        x,
                        ctx.ay(),
                        w,
                        tab_h,
                        if state.active_tab == ServiceTab::System { active_bg } else { inactive_bg },
                        hover_bg,
                        [0.90, 0.90, 0.95, 1.0],
                        crate::app::AppAction::Services(ServicesMessage::SetTab(ServiceTab::System)),
                    );
                } else {
                    ctx.button(
                        label2,
                        x,
                        ctx.ay(),
                        w,
                        tab_h,
                        if state.active_tab == ServiceTab::User { active_bg } else { inactive_bg },
                        hover_bg,
                        [0.90, 0.90, 0.95, 1.0],
                        crate::app::AppAction::Services(ServicesMessage::SetTab(ServiceTab::User)),
                    );
                }
            });

            stack.context.spacing(4.0);

            // Search textbox
            let search_w = sec_w - 24.0;
            let search_h = 46.0;

            state.search_box.set_row_rect(stack.context.left + 12.0, search_w);
            stack.add_widget(&mut state.search_box, search_w, search_h, ctx);
            stack.context.spacing(8.0);

            // Scroll box list
            let list_box_x = sec.left + 12.0;
            let list_box_y = sec.ay();
            let list_box_w = sec_w - 24.0;
            // Fill the page: the well's bottom wall lands at the page bottom,
            // the list keeps a 12px inset above it.
            let list_box_h = ((cy + ch) - 12.0 - list_box_y).max(120.0);

            // Filter services
            let query = if state.search_box.editing {
                state.search_box.edit_buffer.to_lowercase()
            } else {
                state.search_box.text.to_lowercase()
            };
            let filtered_services: Vec<&ServiceInfo> = state.services.iter()
                .filter(|s| s.is_system == (state.active_tab == ServiceTab::System))
                .filter(|s| s.name.to_lowercase().contains(&query) || s.description.to_lowercase().contains(&query))
                .collect();

            // Dissolved List (Phase 6v): scroll state + frame prims are app-owned.
            state.list.set_rect(list_box_x, list_box_y, list_box_w, list_box_h);
            state.list.update_bounds(filtered_services.len(), list_box_y, list_box_h);
            state.list.push_prims(sec.pc);

            let item_h = state.list.item_height;

            if state.items.len() != filtered_services.len() {
                state.items.clear();
                for _ in 0..filtered_services.len() {
                    state.items.push(InteractiveListItem::new(""));
                }
            }

            sec.pc.push_clip_rect(list_box_x, list_box_y, list_box_w, list_box_h);
            for (idx, service) in filtered_services.iter().enumerate() {
                if let Some(draw_y) = state.list.get_item_draw_y(idx, 4.0) {
                    let is_active = service.active_state == "active" || service.sub_state == "running";

                    // Control buttons: Start, Stop, Restart on the right. With
                    // icon faces all three are the same square, and the extra
                    // room "Restart" needed goes back to the description.
                    //
                    // Sized on whether the icon set is actually THERE:
                    // `button_icon` falls back to the labels when it isn't, and
                    // a 24px button doesn't clip a label so much as replace it
                    // — the text centers, so both ends cut and "Restart" reads
                    // "sta". `upload_icon` caches per (name, px), so asking
                    // every row costs one hash lookup.
                    let icons_ok = cce_ui::upload_icon("play", 32).is_some();
                    let is_small = sec_w < 350.0;
                    let (btn_w, r_btn_w) = match (icons_ok, is_small) {
                        (true, _) => (24.0, 24.0),
                        (false, true) => (24.0, 24.0),
                        (false, false) => (46.0, 54.0),
                    };
                    let btn_gap = if is_small { 4.0 } else { 6.0 };
                    let right_edge = list_box_x + list_box_w - 24.0 - 8.0;

                    let restart_x = right_edge - r_btn_w;
                    let toggle_x = restart_x - btn_gap - btn_w;

                    let btn_y = draw_y + (item_h - 22.0) / 2.0;
                    let btn_h = 22.0;

                    // Service Description (Truncate dynamically based on remaining space before Start button)
                    let text_max_w = (toggle_x - 8.0) - (list_box_x + 32.0);
                    let max_chars = ((text_max_w / 6.0) as usize).max(10);
                    let desc = if service.description.is_empty() { "No description" } else { &service.description };
                    let desc_truncated = if desc.len() > max_chars {
                        format!("{}...", &desc[..max_chars.saturating_sub(3)])
                    } else {
                        desc.to_string()
                    };

                    // Render InteractiveListItem background and text labels
                    // Rows dispatch as extra roots (the dissolved list is no parent).
                    let item_btn = &mut state.items[idx];
                    item_btn.title = service.name.clone();
                    item_btn.subtitle = Some(desc_truncated);
                    render_widget(sec.pc, item_btn, list_box_x + 24.0, draw_y, list_box_w - 44.0, item_h, ctx);

                    // Render StatusDot
                    let status_dot_state = if service.active_state == "failed" {
                        DotStatus::Error
                    } else if is_active {
                        DotStatus::Active
                    } else {
                        DotStatus::Inactive
                    };
                    let mut dot = StatusDot::new(status_dot_state);
                    render_widget(sec.pc, &mut dot, list_box_x + 10.0, draw_y + (item_h - 10.0) / 2.0, 10.0, 10.0, ctx);

                    let active_txt = [0.90, 0.90, 0.95, 1.0];

                    // ONE transport button, showing the action it will take:
                    // play on a stopped service, stop on a running one. The two
                    // dimmed half-buttons this replaced were never both live —
                    // exactly one of them did anything on any given row.
                    //
                    // `running` is NOT the `is_active` the status dot reads.
                    // `update` sets a transitional state the instant the button
                    // is clicked (activating/starting, deactivating/stopping),
                    // and counting those as the state they are heading for is
                    // what flips this icon under the pointer instead of leaving
                    // it stale until the next refresh lands. The dot keeps
                    // reporting the confirmed state: the button says what it
                    // will do, the dot says what is true.
                    let running = transport_running(&service.active_state, &service.sub_state);

                    // Fallback labels only — an icon face never draws them.
                    // Without the icons a narrow row is back to needing the
                    // one-glyph words it used before.
                    let (start_lbl, stop_lbl, restart_lbl) = if icons_ok || !is_small {
                        ("Start", "Stop", "Restart")
                    } else {
                        ("\u{25b6}", "\u{25a0}", "\u{27f3}")
                    };

                    // Start/Stop, collapsed
                    sec.pc.button_icon(
                        if running { "stop" } else { "play" },
                        if running { stop_lbl } else { start_lbl },
                        toggle_x,
                        btn_y,
                        btn_w,
                        btn_h,
                        if running { [0.55, 0.16, 0.16, 0.3] } else { [0.16, 0.35, 0.18, 0.4] },
                        if running { [0.70, 0.22, 0.22, 0.5] } else { [0.22, 0.45, 0.25, 0.6] },
                        active_txt,
                        1.0,
                        crate::app::AppAction::Services(if running {
                            ServicesMessage::Stop(service.name.clone(), service.is_system)
                        } else {
                            ServicesMessage::Start(service.name.clone(), service.is_system)
                        }),
                    );

                    // Restart button
                    sec.pc.button_icon(
                        "refresh",
                        restart_lbl,
                        restart_x,
                        btn_y,
                        r_btn_w,
                        btn_h,
                        [0.15, 0.28, 0.45, 0.3],
                        [0.20, 0.38, 0.58, 0.5],
                        active_txt,
                        1.0,
                        crate::app::AppAction::Services(ServicesMessage::Restart(service.name.clone(), service.is_system)),
                    );
                }
            }
            sec.pc.pop_clip_rect();

            if filtered_services.is_empty() {
                sec.pc.text("No services match the query", list_box_x + 16.0, list_box_y + 16.0, 12.0, TEXT_DIM);
            }

            // End the section so the well's bottom wall sits 12px below the
            // list (finish() places the wall at content_y + padding + 12).
            sec.content_y = list_box_y + list_box_h + 12.0 - (sec.padding() + 12.0);
        }
    });

    final_pc
}

/// The optimistic states [`update`] writes the instant a transport button is
/// clicked, before systemd has said anything back. Named because
/// [`transport_running`] has to agree with them: the whole point of writing
/// them is that the button's icon flips under the pointer instead of staying
/// stale until the next refresh lands.
const STARTING: (&str, &str) = ("activating", "starting");
const STOPPING: (&str, &str) = ("deactivating", "stopping");
const RESTARTING: (&str, &str) = ("activating", "restarting");

/// Whether the row's single transport button offers Stop (`true`) or Start
/// (`false`) — which is also which icon it wears.
///
/// A transitional state counts as the state it is heading FOR, not the one it
/// is leaving. That is what makes the click feel like a toggle: `update` marks
/// the unit `activating` the moment Start is pressed, and this reads that as
/// running, so the icon becomes Stop immediately.
///
/// This is deliberately NOT the `is_active` the status dot reads. The dot
/// reports what is confirmed true; the button reports what it will do.
pub fn transport_running(active_state: &str, sub_state: &str) -> bool {
    match active_state {
        "active" | "activating" | "reloading" => true,
        "deactivating" => false,
        _ => sub_state == "running",
    }
}

pub fn update(state: &mut ServicesState, msg: ServicesMessage) {
    match msg {
        ServicesMessage::Refreshed(new_services) => {
            state.loaded = true;
            state.services = new_services;
            state.items.clear();
        }
        ServicesMessage::SetTab(tab) => {
            state.active_tab = tab;
            state.list.set_scroll_y(0.0);
            state.items.clear();
        }
        ServicesMessage::Start(name, is_system) => {
            if let Some(srv) = state.services.iter_mut().find(|s| s.name == name && s.is_system == is_system) {
                srv.active_state = STARTING.0.to_string();
                srv.sub_state = STARTING.1.to_string();
            }
            service_action(&name, "start", is_system);
        }
        ServicesMessage::Stop(name, is_system) => {
            if let Some(srv) = state.services.iter_mut().find(|s| s.name == name && s.is_system == is_system) {
                srv.active_state = STOPPING.0.to_string();
                srv.sub_state = STOPPING.1.to_string();
            }
            service_action(&name, "stop", is_system);
        }
        ServicesMessage::Restart(name, is_system) => {
            if let Some(srv) = state.services.iter_mut().find(|s| s.name == name && s.is_system == is_system) {
                srv.active_state = RESTARTING.0.to_string();
                srv.sub_state = RESTARTING.1.to_string();
            }
            service_action(&name, "restart", is_system);
        }
    }
}

// ── Background Fetching ──

pub async fn fetch_services() -> Vec<ServiceInfo> {
    let mut services = Vec::new();

    // 1. Fetch system-level services
    if let Ok(output) = tokio::process::Command::new("systemctl")
        .args(["list-units", "--type=service", "--all", "--no-legend"])
        .output()
        .await
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if let Some(info) = parse_service_line(line, true) {
                services.push(info);
            }
        }
    }

    // 2. Fetch user-level services
    if let Ok(output) = tokio::process::Command::new("systemctl")
        .args(["--user", "list-units", "--type=service", "--all", "--no-legend"])
        .output()
        .await
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if let Some(info) = parse_service_line(line, false) {
                services.push(info);
            }
        }
    }

    // Sort alphabetically by name
    services.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    services
}

fn parse_service_line(line: &str, is_system: bool) -> Option<ServiceInfo> {
    let cleaned = line.trim_start_matches('●').trim();
    if cleaned.is_empty() {
        return None;
    }
    let parts: Vec<&str> = cleaned.split_whitespace().collect();
    if parts.len() >= 4 && parts[0].ends_with(".service") {
        let name = parts[0].to_string();
        let _load = parts[1];
        let active_state = parts[2].to_string();
        let sub_state = parts[3].to_string();
        let description = parts[4..].join(" ");
        Some(ServiceInfo {
            name,
            description,
            active_state,
            sub_state,
            is_system,
        })
    } else {
        None
    }
}

fn service_action(name: &str, action: &str, is_system: bool) {
    if is_system {
        // System service needs root privilege, spawn via pkexec
        let _ = tokio::process::Command::new("pkexec")
            .args(["systemctl", action, name])
            .spawn();
    } else {
        let _ = tokio::process::Command::new("systemctl")
            .args(["--user", action, name])
            .spawn();
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
        layout: &mut dyn LayoutStrategy,
        ctx: &mut cce_ui::context::UiContext,
    ) -> crate::app::PageContent {
        view(self, cx, cy, cw, ch, root_focused, sec_focused, layout, ctx)
    }

    fn propagate_widget_changes(&mut self, _actions: &mut Vec<crate::app::AppAction>) {}

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

    fn register_extra_dispatch_roots(&mut self, ctx: &mut cce_ui::context::UiContext) {
        let (list, items) = (&self.list, &mut self.items);
        for (idx, i) in items.iter_mut().enumerate() {
            if list.get_item_draw_y(idx, 4.0).is_none() {
                continue;
            }
            let (id, ptr) = (i.id(), i.as_ptr_mut());
            ctx.register_widget(id, ptr);
        }
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
}

#[cfg(test)]
mod tests {
    use super::{transport_running, RESTARTING, STARTING, STOPPING};

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
        // The exact states `update` writes optimistically. If these two ever
        // disagree, the button stops feeling like a toggle: the icon would sit
        // on the old action until systemd's next list lands, and a second
        // click would re-send the action already in flight.
        assert!(transport_running(STARTING.0, STARTING.1), "Start must show Stop immediately");
        assert!(!transport_running(STOPPING.0, STOPPING.1), "Stop must show Start immediately");
        // Restart keeps the unit running throughout, so the transport button
        // must not flicker to Start while it cycles.
        assert!(transport_running(RESTARTING.0, RESTARTING.1), "Restart must keep showing Stop");
    }
}
