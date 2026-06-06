use crate::app::PageContent;
use clear_ui::layout::{Section, PageLayoutBuilder, LayoutStrategy};
use clear_ui::widget::{Widget, TextLabel, ScrollBox, ScrollingList, TextBox};
use clear_ui::widget::{ElementState, KeyEvent, MouseButton, Key, NamedKey};



// ── Service Types and Page State ──

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
    pub search_box: TextBox,
    pub list_box: ScrollingList,
}

impl Default for ServicesState {
    fn default() -> Self {
        Self {
            loaded: false,
            services: Vec::new(),
            active_tab: ServiceTab::System,
            search_box: TextBox::new(String::new()).with_label("Filter Services"),
            list_box: ScrollingList::new(36.0, 6.0),
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
        // User service does not need root
        let _ = tokio::process::Command::new("systemctl")
            .args(["--user", action, name])
            .spawn();
    }
}

// ── View & Update ──

const TEXT_DIM: [f32; 4] = [0.53, 0.53, 0.60, 1.0];

pub fn view(state: &mut ServicesState, cx: f32, cy: f32, cw: f32, ch: f32, root_focused: bool, layout: &mut dyn LayoutStrategy) -> PageContent {
    let mut final_pc = PageContent::new();
    let sec_w = 320.0f32;
    let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(1);

    builder.add_section(&mut final_pc, |pc, rx, ry| {
        let mut sec = Section::new(pc, rx, ry, sec_w, "Services");

        if !state.loaded {
            sec.text(pc, "Loading systemd services...", 12.0, 0.0, 12.0, TEXT_DIM);
            sec.spacing(18.0);
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

            pc.button(
                label1,
                rx + 12.0,
                tab_y,
                tab_w,
                tab_h,
                if state.active_tab == ServiceTab::System { active_bg } else { inactive_bg },
                hover_bg,
                [0.90, 0.90, 0.95, 1.0],
                crate::app::AppAction::Services(ServicesMessage::SetTab(ServiceTab::System)),
            );

            pc.button(
                label2,
                rx + 12.0 + tab_w + 8.0,
                tab_y,
                tab_w,
                tab_h,
                if state.active_tab == ServiceTab::User { active_bg } else { inactive_bg },
                hover_bg,
                [0.90, 0.90, 0.95, 1.0],
                crate::app::AppAction::Services(ServicesMessage::SetTab(ServiceTab::User)),
            );
            sec.content_y += tab_h + 12.0;

            // Search textbox
            let search_y = sec.ay() + state.search_box.top_room();
            let search_w = sec_w - 24.0;
            let search_h = 28.0;
            
            state.search_box.set_row_rect(rx + 12.0, search_w);
            clear_ui::layout::render_widget(
                pc,
                &mut state.search_box,
                rx + 12.0,
                search_y,
                search_w,
                search_h,
            );
            sec.content_y += search_h + state.search_box.top_room() + 16.0;

            // Scroll box list
            let list_box_x = rx + 12.0;
            let list_box_y = sec.ay();
            let list_box_w = sec_w - 24.0;
            let list_box_h = 360.0;
            
            clear_ui::layout::render_widget(pc, &mut state.list_box, list_box_x, list_box_y, list_box_w, list_box_h);

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

            // Update ScrollingList bounds
            state.list_box.update_bounds(filtered_services.len(), list_box_y, list_box_h);

            let item_h = state.list_box.item_height;

            for (idx, service) in filtered_services.iter().enumerate() {
                if let Some(draw_y) = state.list_box.get_item_draw_y(idx, 4.0) {
                    // Item background
                    let bg_color = [0.08, 0.08, 0.12, 0.2];
                    pc.rect(bg_color, list_box_x + 4.0, draw_y, list_box_w - 24.0, item_h);

                    // Status indicator color
                    let is_active = service.active_state == "active" || service.sub_state == "running";
                    let status_color = if service.active_state == "failed" {
                        [0.85, 0.25, 0.25, 1.0] // failed = red
                    } else if is_active {
                        [0.25, 0.75, 0.35, 1.0] // active = green
                    } else {
                        [0.55, 0.55, 0.60, 1.0] // inactive/dead = gray
                    };

                    // Render status dot (small square)
                    pc.rect(status_color, list_box_x + 14.0, draw_y + (item_h - 10.0) / 2.0, 10.0, 10.0);

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

                    // Service Name
                    pc.text(&service.name, list_box_x + 32.0, draw_y + 4.0, 13.0, [0.90, 0.90, 0.95, 1.0]);

                    // Service Description (Truncate dynamically based on remaining space before Start button)
                    let text_max_w = (start_x - 8.0) - (list_box_x + 32.0);
                    let max_chars = ((text_max_w / 6.0) as usize).max(10);
                    let desc = if service.description.is_empty() { "No description" } else { &service.description };
                    let desc_truncated = if desc.len() > max_chars {
                        format!("{}...", &desc[..max_chars.saturating_sub(3)])
                    } else {
                        desc.to_string()
                    };
                    pc.text(&desc_truncated, list_box_x + 32.0, draw_y + 19.0, 11.0, [0.55, 0.55, 0.60, 1.0]);

                    let active_txt = [0.90, 0.90, 0.95, 1.0];
                    let disabled_txt = [0.40, 0.40, 0.45, 1.0];

                    let start_lbl = if is_small { "▶" } else { "Start" };
                    let stop_lbl = if is_small { "■" } else { "Stop" };
                    let restart_lbl = if is_small { "⟳" } else { "Restart" };

                    // Start button
                    pc.button(
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
                    pc.button(
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
                    pc.button(
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

            if filtered_services.is_empty() {
                pc.text("No services match the query", list_box_x + 16.0, list_box_y + 16.0, 12.0, TEXT_DIM);
            }

            sec.content_y += list_box_h;
        }
        sec.finish_focused(pc, root_focused)
    });

    final_pc
}

pub fn update(state: &mut ServicesState, msg: ServicesMessage) {
    match msg {
        ServicesMessage::Refreshed(new_services) => {
            state.loaded = true;
            state.services = new_services;
        }
        ServicesMessage::SetTab(tab) => {
            state.active_tab = tab;
            state.list_box.set_scroll_y(0.0);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_layout_grid() {
        let mut state = ServicesState::default();
        let mut layout = clear_ui::layout::ColumnLayout::new(20.0);
        let pc = view(&mut state, 10.0, 20.0, 800.0, 600.0, false, &mut layout);
        assert!(!pc.rects.is_empty() || !pc.texts.is_empty());
    }
}

