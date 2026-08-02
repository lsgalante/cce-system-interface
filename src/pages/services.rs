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
            list: ScrollRegion::new(36.0, 6.0),
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

                    // Control buttons: Start, Stop, Restart on the right
                    let is_small = sec_w < 350.0;
                    let btn_w = if is_small { 24.0 } else { 46.0 };
                    let r_btn_w = if is_small { 24.0 } else { 54.0 };
                    let btn_gap = if is_small { 4.0 } else { 6.0 };
                    let right_edge = list_box_x + list_box_w - 24.0 - 8.0;

                    let restart_x = right_edge - r_btn_w;
                    let stop_x = restart_x - btn_gap - btn_w;
                    let start_x = stop_x - btn_gap - btn_w;

                    let btn_y = draw_y + (item_h - 22.0) / 2.0;
                    let btn_h = 22.0;

                    // Service Description (Truncate dynamically based on remaining space before Start button)
                    let text_max_w = (start_x - 8.0) - (list_box_x + 32.0);
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
                    let disabled_txt = [0.40, 0.40, 0.45, 1.0];

                    let start_lbl = if is_small { "▶" } else { "Start" };
                    let stop_lbl = if is_small { "■" } else { "Stop" };
                    let restart_lbl = if is_small { "⟳" } else { "Restart" };

                    // Start button
                    sec.pc.button(
                        start_lbl,
                        start_x,
                        btn_y,
                        btn_w,
                        btn_h,
                        if !is_active { [0.16, 0.35, 0.18, 0.4] } else { [0.12, 0.12, 0.16, 0.1] },
                        [0.22, 0.45, 0.25, 0.6],
                        if !is_active { active_txt } else { disabled_txt },
                        crate::app::AppAction::Services(ServicesMessage::Start(service.name.clone(), service.is_system)),
                    );

                    // Stop button
                    sec.pc.button(
                        stop_lbl,
                        stop_x,
                        btn_y,
                        btn_w,
                        btn_h,
                        if is_active { [0.55, 0.16, 0.16, 0.3] } else { [0.12, 0.12, 0.16, 0.1] },
                        [0.70, 0.22, 0.22, 0.5],
                        if is_active { active_txt } else { disabled_txt },
                        crate::app::AppAction::Services(ServicesMessage::Stop(service.name.clone(), service.is_system)),
                    );

                    // Restart button
                    sec.pc.button(
                        restart_lbl,
                        restart_x,
                        btn_y,
                        r_btn_w,
                        btn_h,
                        [0.15, 0.28, 0.45, 0.3],
                        [0.20, 0.38, 0.58, 0.5],
                        active_txt,
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
                srv.active_state = "activating".to_string();
                srv.sub_state = "starting".to_string();
            }
            service_action(&name, "start", is_system);
        }
        ServicesMessage::Stop(name, is_system) => {
            if let Some(srv) = state.services.iter_mut().find(|s| s.name == name && s.is_system == is_system) {
                srv.active_state = "deactivating".to_string();
                srv.sub_state = "stopping".to_string();
            }
            service_action(&name, "stop", is_system);
        }
        ServicesMessage::Restart(name, is_system) => {
            if let Some(srv) = state.services.iter_mut().find(|s| s.name == name && s.is_system == is_system) {
                srv.active_state = "activating".to_string();
                srv.sub_state = "restarting".to_string();
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
        vec![vec![self.search_box.id()]]
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

    fn extra_dispatch_roots(&mut self) -> Vec<cce_ui::widget::WidgetId> {
        self.items.iter().map(|i| i.id()).collect()
    }

    fn register_extra_dispatch_roots(&mut self, ctx: &mut cce_ui::context::UiContext) {
        for i in self.items.iter_mut() {
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
