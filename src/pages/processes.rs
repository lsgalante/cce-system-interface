use crate::app::{AppAction, PageContent};
use cce_ui::layout::{render_widget, PageLayoutBuilder, LayoutStrategy, RenderTarget};
use cce_ui::widget::{List, TextBox, StatusDot, DotStatus, InteractiveListItem, Element};

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
pub struct ProcessesState {
    pub loaded: bool,
    pub processes: Vec<(String, String, String)>, // (pid, cpu, comm)
    pub cpu_list_box: List,

    // Services-related fields
    pub services_loaded: bool,
    pub services: Vec<ServiceInfo>,
    pub services_active_tab: ServiceTab,
    pub services_search_box: TextBox,
    pub services_list_box: List,
    pub service_items: Vec<InteractiveListItem>,
}

impl Default for ProcessesState {
    fn default() -> Self {
        Self {
            loaded: false,
            processes: Vec::new(),
            cpu_list_box: List::new(24.0, 2.0),

            services_loaded: false,
            services: Vec::new(),
            services_active_tab: ServiceTab::System,
            services_search_box: TextBox::new(String::new()).with_label("Filter Services"),
            services_list_box: List::new(36.0, 6.0),
            service_items: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ProcessesMessage {
    Refreshed(ProcessesState),
    None,

    // Services-related variants
    ServicesRefreshed(Vec<ServiceInfo>),
    ServicesSetTab(ServiceTab),
    ServicesStart(String, bool),
    ServicesStop(String, bool),
    ServicesRestart(String, bool),
}

pub async fn fetch_processes_state() -> ProcessesState {
    let processes = {
        let mut list = Vec::new();
        if let Some(o) = tokio::process::Command::new("ps")
            .args(["-eo", "pid,%cpu,comm", "--sort=-%cpu"])
            .output().await.ok()
        {
            let text = String::from_utf8_lossy(&o.stdout);
            for line in text.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    let pid = parts[0].to_string();
                    let cpu = parts[1].to_string();
                    let comm = parts[2..].join(" ");
                    list.push((pid, cpu, comm));
                }
            }
        }
        list
    };

    ProcessesState {
        loaded: true,
        processes,
        cpu_list_box: List::new(24.0, 2.0),
        services_loaded: false,
        services: Vec::new(),
        services_active_tab: ServiceTab::System,
        services_search_box: TextBox::new(String::new()).with_label("Filter Services"),
        services_list_box: List::new(36.0, 6.0),
        service_items: Vec::new(),
    }
}

const TEXT_FG: [f32; 4] = [0.83, 0.83, 0.83, 1.0];
const TEXT_DIM: [f32; 4] = [0.53, 0.53, 0.60, 1.0];

pub fn view(state: &mut ProcessesState, cx: f32, cy: f32, cw: f32, ch: f32, root_focused: bool, sec_focused: &[bool], layout: &mut dyn LayoutStrategy, ctx: &mut cce_ui::context::UiContext) -> PageContent {
    let mut final_pc = PageContent::new();
    let sec_w = 320.0f32;
    let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(2);

    // ── Processes Section ──
    builder.add_section_spanned(&mut final_pc, "Processes", 2, root_focused || sec_focused.first().copied().unwrap_or(false), |sec| {
        let rx = sec.left;
        if !state.loaded {
            sec.text("Loading processes...", 12.0, 0.0, 12.0, TEXT_FG);
        } else {
            // Scrolling box configuration for process list
            let list_box_x = rx + 12.0;
            let list_box_y = sec.ay();
            let list_box_w = sec.cw - 24.0;
            let list_box_h = 220.0;
            
            // Render the standardized ScrollBox widget
            render_widget(sec.pc, &mut state.cpu_list_box, list_box_x, list_box_y, list_box_w, list_box_h, ctx);

            // Header for process list columns (drawn static on top of the ScrollBox background)
            let header_h = 22.0;
            sec.pc.rect([0.12, 0.12, 0.16, 0.5], list_box_x + 1.0, list_box_y + 1.0, list_box_w - 2.0, header_h);
            sec.pc.rect([0.18, 0.18, 0.24, 1.0], list_box_x + 1.0, list_box_y + header_h, list_box_w - 2.0, 1.0); // Divider
            
            sec.pc.text("PID", list_box_x + 12.0, list_box_y + 5.0, 11.0, [0.53, 0.53, 0.60, 1.0]);
            sec.pc.text("COMMAND", list_box_x + 80.0, list_box_y + 5.0, 11.0, [0.53, 0.53, 0.60, 1.0]);
            sec.pc.text("CPU %", list_box_x + list_box_w - 60.0, list_box_y + 5.0, 11.0, [0.53, 0.53, 0.60, 1.0]);

            let row_h = 24.0;
            // Update List bounds for the scrollable viewport (which starts below the header)
            state.cpu_list_box.update_bounds(state.processes.len(), list_box_y + header_h, list_box_h - header_h - 6.0);

            // Visible process rows rendering (virtualized/clipped)
            sec.pc.push_clip_rect(list_box_x, list_box_y + header_h, list_box_w, list_box_h - header_h);
            for (idx, (pid, cpu, comm)) in state.processes.iter().enumerate() {
                if let Some(draw_y) = state.cpu_list_box.get_item_draw_y(idx, 4.0) {
                    // Standard row action button (transparent background, highlights on hover)
                    sec.pc.button(
                        "",
                        list_box_x + 2.0,
                        draw_y,
                        list_box_w - 16.0,
                        row_h,
                        [0.0, 0.0, 0.0, 0.0],
                        [1.0, 1.0, 1.0, 0.06],
                        [0.0, 0.0, 0.0, 0.0],
                        AppAction::Processes(ProcessesMessage::None),
                    );
                    
                    sec.pc.text(pid, list_box_x + 12.0, draw_y + 6.0, 12.0, [0.80, 0.80, 0.85, 1.0]);
                    sec.pc.text(comm, list_box_x + 80.0, draw_y + 6.0, 12.0, [0.80, 0.80, 0.85, 1.0]);
                    sec.pc.text(&format!("{}%", cpu), list_box_x + list_box_w - 60.0, draw_y + 6.0, 12.0, [0.56, 0.83, 0.56, 1.0]);
                }
            }
            sec.pc.pop_clip_rect();
            
            if state.processes.is_empty() {
                sec.pc.text("No active processes", list_box_x + 12.0, list_box_y + header_h + 16.0, 12.0, TEXT_DIM);
            }

            sec.content_y += list_box_h;
        }
    });

    // ── Services Section ──
    builder.add_section_spanned(&mut final_pc, "Services", 2, sec_focused.get(1).copied().unwrap_or(false), |sec| {
        let sec_w = sec.cw;
        if !state.services_loaded {
            sec.text("Loading systemd services...", 12.0, 0.0, 12.0, TEXT_DIM);
        } else {
            // Tab header buttons: System Services, User Services
            let tab_w = (sec_w - 24.0 - 8.0) / 2.0;
            let tab_h = 28.0;
            let tab_y = sec.ay();
            let active_bg = [0.20, 0.40, 0.65, 0.4];
            let inactive_bg = [0.10, 0.10, 0.16, 0.3];
            let hover_bg = [0.20, 0.20, 0.25, 0.15];

            let label1 = if tab_w < 110.0 { "System" } else { "System Services" };
            let label2 = if tab_w < 110.0 { "User" } else { "User Services" };

            let tab_x1 = sec.left + 12.0;
            let tab_x2 = sec.left + 12.0 + tab_w + 8.0;

            sec.pc.button(
                label1,
                tab_x1,
                tab_y,
                tab_w,
                tab_h,
                if state.services_active_tab == ServiceTab::System { active_bg } else { inactive_bg },
                hover_bg,
                [0.90, 0.90, 0.95, 1.0],
                crate::app::AppAction::Processes(ProcessesMessage::ServicesSetTab(ServiceTab::System)),
            );

            sec.pc.button(
                label2,
                tab_x2,
                tab_y,
                tab_w,
                tab_h,
                if state.services_active_tab == ServiceTab::User { active_bg } else { inactive_bg },
                hover_bg,
                [0.90, 0.90, 0.95, 1.0],
                crate::app::AppAction::Processes(ProcessesMessage::ServicesSetTab(ServiceTab::User)),
            );
            sec.content_y += tab_h + 12.0;

            // Search textbox
            let search_y = sec.ay();
            let search_w = sec_w - 24.0;
            let search_h = 46.0;
            
            state.services_search_box.set_row_rect(sec.left + 12.0, search_w);
            render_widget(
                sec.pc,
                &mut state.services_search_box,
                sec.left + 12.0,
                search_y,
                search_w,
                search_h,
                ctx,
            );
            sec.content_y += search_h + 16.0;

            // Scroll box list
            let list_box_x = sec.left + 12.0;
            let list_box_y = sec.ay();
            let list_box_w = sec_w - 24.0;
            let list_box_h = 360.0;
            
            render_widget(sec.pc, &mut state.services_list_box, list_box_x, list_box_y, list_box_w, list_box_h, ctx);

            // Filter services
            let query = if state.services_search_box.editing {
                state.services_search_box.edit_buffer.to_lowercase()
            } else {
                state.services_search_box.text.to_lowercase()
            };
            let filtered_services: Vec<&ServiceInfo> = state.services.iter()
                .filter(|s| s.is_system == (state.services_active_tab == ServiceTab::System))
                .filter(|s| s.name.to_lowercase().contains(&query) || s.description.to_lowercase().contains(&query))
                .collect();

            // Update List bounds
            state.services_list_box.update_bounds(filtered_services.len(), list_box_y, list_box_h);

            let item_h = state.services_list_box.item_height;

            if state.service_items.len() != filtered_services.len() {
                state.service_items.clear();
                for _ in 0..filtered_services.len() {
                    state.service_items.push(InteractiveListItem::new(""));
                }
            }

            sec.pc.push_clip_rect(list_box_x, list_box_y, list_box_w, list_box_h);
            for (idx, service) in filtered_services.iter().enumerate() {
                if let Some(draw_y) = state.services_list_box.get_item_draw_y(idx, 4.0) {
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
                    let item_btn = &mut state.service_items[idx];
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
                        crate::app::AppAction::Processes(ProcessesMessage::ServicesStart(service.name.clone(), service.is_system)),
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
                        crate::app::AppAction::Processes(ProcessesMessage::ServicesStop(service.name.clone(), service.is_system)),
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
                        crate::app::AppAction::Processes(ProcessesMessage::ServicesRestart(service.name.clone(), service.is_system)),
                    );
                }
            }
            sec.pc.pop_clip_rect();

            if filtered_services.is_empty() {
                sec.pc.text("No services match the query", list_box_x + 16.0, list_box_y + 16.0, 12.0, TEXT_DIM);
            }

            sec.content_y += list_box_h;
        }
    });

    final_pc
}

pub fn update(state: &mut ProcessesState, msg: ProcessesMessage) {
    match msg {
        ProcessesMessage::Refreshed(new) => {
            state.loaded = new.loaded;
            state.processes = new.processes;
            let old_scroll = state.cpu_list_box.scroll_y();
            state.cpu_list_box = new.cpu_list_box;
            state.cpu_list_box.set_scroll_y(old_scroll);
        }
        ProcessesMessage::ServicesRefreshed(new_services) => {
            state.services_loaded = true;
            state.services = new_services;
            state.service_items.clear();
        }
        ProcessesMessage::ServicesSetTab(tab) => {
            state.services_active_tab = tab;
            state.services_list_box.set_scroll_y(0.0);
            state.service_items.clear();
        }
        ProcessesMessage::ServicesStart(name, is_system) => {
            if let Some(srv) = state.services.iter_mut().find(|s| s.name == name && s.is_system == is_system) {
                srv.active_state = "activating".to_string();
                srv.sub_state = "starting".to_string();
            }
            service_action(&name, "start", is_system);
        }
        ProcessesMessage::ServicesStop(name, is_system) => {
            if let Some(srv) = state.services.iter_mut().find(|s| s.name == name && s.is_system == is_system) {
                srv.active_state = "deactivating".to_string();
                srv.sub_state = "stopping".to_string();
            }
            service_action(&name, "stop", is_system);
        }
        ProcessesMessage::ServicesRestart(name, is_system) => {
            if let Some(srv) = state.services.iter_mut().find(|s| s.name == name && s.is_system == is_system) {
                srv.active_state = "activating".to_string();
                srv.sub_state = "restarting".to_string();
            }
            service_action(&name, "restart", is_system);
        }
        ProcessesMessage::None => {}
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

impl crate::pages::AppPage for ProcessesState {
    fn clear_children(&mut self, ctx: &mut cce_ui::context::UiContext) {
        self.cpu_list_box.scroll_box.clear_children(ctx);
        self.cpu_list_box.scroll_box.set_parent(None, ctx);
        self.services_search_box.clear_children(ctx);
        self.services_search_box.set_parent(None, ctx);
        self.services_list_box.scroll_box.clear_children(ctx);
        self.services_list_box.scroll_box.set_parent(None, ctx);
    }

    fn get_section_containers(&self) -> Vec<cce_ui::widget::SectionContainer> {
        vec![
            cce_ui::widget::SectionContainer::new("Processes").with_layout(cce_ui::widget::AdaptiveGridLayout {
                min_col_width: 140.0,
                gap: 8.0,
                padding_x: 0.0,
                padding_y: 0.0,
                grid: None,
            }),
            cce_ui::widget::SectionContainer::new("Services").with_layout(cce_ui::widget::AdaptiveGridLayout {
                min_col_width: 140.0,
                gap: 8.0,
                padding_x: 0.0,
                padding_y: 0.0,
                grid: None,
            }),
        ]
    }

    fn link_children(
        &mut self,
        page_root: &mut dyn cce_ui::widget::Element,
        sec_containers: &mut [cce_ui::widget::SectionContainer],
        ctx: &mut cce_ui::context::UiContext,
    ) {
        cce_ui::widget::link_parent_child(page_root, &mut sec_containers[0], ctx);
        cce_ui::widget::link_parent_child(page_root, &mut sec_containers[1], ctx);

        cce_ui::widget::link_parent_child(&mut sec_containers[0], &mut self.cpu_list_box.scroll_box, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[1], &mut self.services_search_box, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[1], &mut self.services_list_box.scroll_box, ctx);
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_layout_grid() {
        let mut state = ProcessesState::default();
        let mut layout = cce_ui::layout::ColumnLayout::new(20.0);
        let sec_focused = vec![false, false];
        let mut ctx = cce_ui::context::UiContext::new();
        let pc = view(&mut state, 10.0, 20.0, 800.0, 600.0, false, &sec_focused, &mut layout, &mut ctx);
        assert!(!pc.rects.is_empty() || !pc.texts.is_empty());
    }
}
