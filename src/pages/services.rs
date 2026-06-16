use std::fs;
use std::io::Write;
use crate::app::{AppAction, PageContent, SectionContextExt};
use clear_ui::layout::{PageLayoutBuilder, LayoutStrategy};
use clear_ui::widget::{Element, ScrollingList, TextBox, StatusDot, DotStatus, InteractiveListItem, Toggle, Spinbox, Slider, Label};
use crate::pages::interface::parse_u16_from;

// ── Notifications Data and Settings Configuration ──

#[derive(Debug, Clone)]
pub struct NotificationsConfig {
    pub enable: bool,
    pub bell: bool,
    pub duration: i32,
}

// ── Status Interface Data ──

#[derive(Debug, Clone)]
pub struct StatusData {
    pub font_size: u16,
    pub padding: u16,
    pub separators: bool,
    pub underline: bool,
    pub running: bool,
}

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
    pub service_items: Vec<InteractiveListItem>,
    pub notifications_loaded: bool,
    pub notifications_enable: bool,
    pub notifications_enable_toggle: Toggle,
    pub notifications_bell: bool,
    pub notifications_bell_toggle: Toggle,
    pub notifications_duration: i32,
    pub notifications_duration_spinbox: Spinbox,

    // Status Interface fields
    pub status_loaded: bool,
    pub status_font_size: u16,
    pub status_padding: u16,
    pub status_separators: bool,
    pub status_underline: bool,
    pub status_running: bool,
    pub status_label: Label,
    pub status_separators_toggle: Toggle,
    pub status_underline_toggle: Toggle,
    pub status_padding_spinbox: Spinbox,
}

impl Default for ServicesState {
    fn default() -> Self {
        Self {
            loaded: false,
            services: Vec::new(),
            active_tab: ServiceTab::System,
            search_box: TextBox::new(String::new()).with_label("Filter Services"),
            list_box: ScrollingList::new(36.0, 6.0),
            service_items: Vec::new(),
            notifications_loaded: false,
            notifications_enable: true,
            notifications_enable_toggle: Toggle::new().with_label("Enable Notifications"),
            notifications_bell: false,
            notifications_bell_toggle: Toggle::new().with_label("Play Bell Sound"),
            notifications_duration: 5,
            notifications_duration_spinbox: Spinbox::new(5, 1, 60, 1)
                .with_label("Notification Duration")
                .with_unit("s"),

            // Status Interface default initialization
            status_loaded: false,
            status_font_size: 11,
            status_padding: 8,
            status_separators: true,
            status_underline: true,
            status_running: false,
            status_label: Label::new("Status Interface: Stopped").with_font_size(14.0).with_color([170, 51, 51]),
            status_separators_toggle: Toggle::new().with_label("Show Separators"),
            status_underline_toggle: Toggle::new().with_label("Show Underline"),
            status_padding_spinbox: Spinbox::new(8, 0, 32, 1).with_label("Side Padding").with_unit("px"),
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
    ToggleNotificationsEnable,
    ToggleNotificationsBell,
    SetNotificationsDuration(i32),
    SendTestNotification,
    NotificationsRefreshed(NotificationsConfig),

    // Status Interface variants
    StatusRefreshed(StatusData),
    StatusToggleSeparators,
    StatusToggleUnderline,
    StatusReload,
    StatusSetPadding(u16),
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

pub fn view(state: &mut ServicesState, cx: f32, cy: f32, cw: f32, ch: f32, sec_focused: &[bool], layout: &mut dyn LayoutStrategy, ctx: &mut clear_ui::context::UiContext) -> PageContent {
    let mut final_pc = PageContent::new();
    let sec_w = 320.0f32;
    let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(3);

    builder.add_section(&mut final_pc, "Services", sec_focused.first().copied().unwrap_or(false), |sec| {
        let sec_w = sec.cw;
        if !state.loaded {
            sec.text("Loading systemd services...", 12.0, 0.0, 12.0, TEXT_DIM);
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

            let tab_x1 = sec.left + 12.0;
            let tab_x2 = sec.left + 12.0 + tab_w + 8.0;

            sec.pc.button(
                label1,
                tab_x1,
                tab_y,
                tab_w,
                tab_h,
                if state.active_tab == ServiceTab::System { active_bg } else { inactive_bg },
                hover_bg,
                [0.90, 0.90, 0.95, 1.0],
                crate::app::AppAction::Services(ServicesMessage::SetTab(ServiceTab::System)),
            );

            sec.pc.button(
                label2,
                tab_x2,
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
            let search_y = sec.ay();
            let search_w = sec_w - 24.0;
            let search_h = 46.0;
            
            state.search_box.set_row_rect(sec.left + 12.0, search_w);
            clear_ui::layout::render_widget(
                sec.pc,
                &mut state.search_box,
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
            
            clear_ui::layout::render_widget(sec.pc, &mut state.list_box, list_box_x, list_box_y, list_box_w, list_box_h, ctx);

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

            if state.service_items.len() != filtered_services.len() {
                state.service_items.clear();
                for _ in 0..filtered_services.len() {
                    state.service_items.push(InteractiveListItem::new(""));
                }
            }

            for (idx, service) in filtered_services.iter().enumerate() {
                if let Some(draw_y) = state.list_box.get_item_draw_y(idx, 4.0) {
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
                    clear_ui::layout::render_widget(sec.pc, item_btn, list_box_x + 24.0, draw_y, list_box_w - 44.0, item_h, ctx);

                    // Render StatusDot
                    let status_dot_state = if service.active_state == "failed" {
                        DotStatus::Error
                    } else if is_active {
                        DotStatus::Active
                    } else {
                        DotStatus::Inactive
                    };
                    let mut dot = StatusDot::new(status_dot_state);
                    clear_ui::layout::render_widget(sec.pc, &mut dot, list_box_x + 10.0, draw_y + (item_h - 10.0) / 2.0, 10.0, 10.0, ctx);

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

            if filtered_services.is_empty() {
                sec.pc.text("No services match the query", list_box_x + 16.0, list_box_y + 16.0, 12.0, TEXT_DIM);
            }

            sec.content_y += list_box_h;
        }
    });

    // ── System Notifications ──
    builder.add_section(&mut final_pc, "System Notifications", sec_focused.get(1).copied().unwrap_or(false), |sec2| {
        let sec_w = sec2.cw;
        state.notifications_enable_toggle.set_toggled(state.notifications_enable);
        sec2.widget_full(&mut state.notifications_enable_toggle, clear_ui::layout::toggle_height(), ctx);
        sec2.spacing(8.0);

        state.notifications_bell_toggle.set_toggled(state.notifications_bell);
        sec2.widget_full(&mut state.notifications_bell_toggle, clear_ui::layout::toggle_height(), ctx);
        sec2.spacing(16.0);

        state.notifications_duration_spinbox.value = state.notifications_duration;
        state.notifications_duration_spinbox.set_label("Notification Duration");
        sec2.widget(&mut state.notifications_duration_spinbox, 14.0, sec_w - 28.0, 44.0, ctx);
        sec2.spacing(16.0);

        let btn_h = 32.0;
        let btn_y = sec2.ay();
        let white_color = [1.0, 1.0, 1.0, 1.0];
        let btn_bg = [0.20, 0.40, 0.65, 1.0];
        let btn_hover = [0.28, 0.50, 0.78, 1.0];
        
        let cols = sec2.row_layout(1, 0.0);
        if let Some(&(x, w)) = cols.first() {
            sec2.button(
                "Send Test Notification",
                x,
                btn_y,
                w,
                btn_h,
                btn_bg,
                btn_hover,
                white_color,
                AppAction::Services(ServicesMessage::SendTestNotification),
            );
        }
        sec2.spacing(12.0);
    });

    // ── Status Interface ──
    builder.add_section(&mut final_pc, "Status Interface", sec_focused.get(2).copied().unwrap_or(false), |sec3| {
        let sec_w = sec3.cw;
        if !state.status_loaded {
            sec3.text("Loading Status Interface status...", 12.0, 0.0, 12.0, TEXT_DIM);
            sec3.spacing(18.0);
        } else {
            // Status
            let status_text = if state.status_running { "Status Interface: Running" } else { "Status Interface: Stopped" };
            let status_color = if state.status_running { [92, 143, 97] } else { [170, 51, 51] };
            state.status_label.set_text(status_text);
            state.status_label.set_color(status_color);
            sec3.widget(&mut state.status_label, 12.0, sec_w - 24.0, 20.0, ctx);
            sec3.spacing(12.0);

            sec3.spacing(4.0);

            // Separators toggle
            state.status_separators_toggle.set_toggled(state.status_separators);
            sec3.widget_full(&mut state.status_separators_toggle, clear_ui::layout::toggle_height(), ctx);
            sec3.spacing(16.0);

            // Underline toggle
            state.status_underline_toggle.set_toggled(state.status_underline);
            sec3.widget_full(&mut state.status_underline_toggle, clear_ui::layout::toggle_height(), ctx);
            sec3.spacing(16.0);

            // Padding spinbox
            state.status_padding_spinbox.value = state.status_padding as i32;
            sec3.widget(&mut state.status_padding_spinbox, 12.0, sec_w - 24.0, 44.0, ctx);
            sec3.spacing(16.0);

            // Reload button
            let yt_reload = sec3.ay();
            let btn_w = sec_w - 24.0;
            let button_x = sec3.left + 12.0;
            sec3.button(
                "Reload Status Interface",
                button_x,
                yt_reload,
                btn_w,
                32.0,
                [0.13, 0.18, 0.14, 1.0],
                [0.25, 0.30, 0.26, 1.0],
                [1.0, 1.0, 1.0, 1.0],
                AppAction::Services(ServicesMessage::StatusReload),
            );
            sec3.spacing(12.0);
        }
    });

    final_pc
}

pub fn update(state: &mut ServicesState, msg: ServicesMessage) {
    match msg {
        ServicesMessage::Refreshed(new_services) => {
            state.loaded = true;
            state.services = new_services;
            state.service_items.clear();
        }
        ServicesMessage::SetTab(tab) => {
            state.active_tab = tab;
            state.list_box.set_scroll_y(0.0);
            state.service_items.clear();
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
        ServicesMessage::ToggleNotificationsEnable => {
            state.notifications_enable = !state.notifications_enable;
            write_enable_notifications(state.notifications_enable);
        }
        ServicesMessage::ToggleNotificationsBell => {
            state.notifications_bell = !state.notifications_bell;
            write_config_value("bell", &state.notifications_bell.to_string());
        }
        ServicesMessage::SetNotificationsDuration(d) => {
            state.notifications_duration = d;
            write_config_value("duration", &state.notifications_duration.to_string());
        }
        ServicesMessage::SendTestNotification => {
            send_ipc_command("notify \"cce-client\" \"System notifications are working correctly!\"");
        }
        ServicesMessage::NotificationsRefreshed(new) => {
            state.notifications_loaded = true;
            state.notifications_enable = new.enable;
            state.notifications_bell = new.bell;
            state.notifications_duration = new.duration;
        }
        ServicesMessage::StatusRefreshed(new) => {
            let was_status_hovered = state.status_label.hovered();
            let was_separators_hovered = state.status_separators_toggle.hovered();
            let was_underline_hovered = state.status_underline_toggle.hovered();

            state.status_loaded = true;
            state.status_font_size = new.font_size;
            state.status_padding = new.padding;
            state.status_separators = new.separators;
            state.status_underline = new.underline;
            state.status_running = new.running;

            state.status_label.set_hovered(was_status_hovered);
            state.status_separators_toggle.set_hovered(was_separators_hovered);
            state.status_underline_toggle.set_hovered(was_underline_hovered);
        }

        ServicesMessage::StatusToggleSeparators => {
            state.status_separators = !state.status_separators;
            write_status_separators(state.status_separators);
            status_interface_reload();
        }
        ServicesMessage::StatusToggleUnderline => {
            state.status_underline = !state.status_underline;
            write_status_underline(state.status_underline);
            status_interface_reload();
        }
        ServicesMessage::StatusSetPadding(val) => {
            state.status_padding = val;
            write_status_padding(val);
            status_interface_reload();
        }
        ServicesMessage::StatusReload => {
            status_interface_reload();
        }
    }
}

// ── Notifications Configuration Reader & Writer ──

const CONFIG_PATH: &str = "/home/lsgalante/.config/cce/config.toml";

fn get_socket_path() -> String {
    match std::env::var("WAYLAND_DISPLAY") {
        Ok(display) => format!("/tmp/cce-client-{}.sock", display),
        Err(_) => "/tmp/cce-client.sock".to_string(),
    }
}

pub fn read_notifications_config() -> NotificationsConfig {
    let content = fs::read_to_string(CONFIG_PATH).unwrap_or_default();
    let enable = parse_notifications_enable(&content);
    let bell = parse_notifications_bell(&content);
    let duration = parse_notifications_duration(&content);
    NotificationsConfig {
        enable,
        bell,
        duration,
    }
}

fn parse_notifications_enable(content: &str) -> bool {
    let mut in_section = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[notifications]" {
            in_section = true;
            continue;
        }
        if trimmed.starts_with('[') && in_section {
            break;
        }
        if in_section && trimmed.starts_with("enable") {
            if let Some(val) = trimmed.split('=').nth(1) {
                return val.trim() == "true";
            }
        }
    }
    true // default to true
}

fn parse_notifications_bell(content: &str) -> bool {
    let mut in_section = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[notifications]" {
            in_section = true;
            continue;
        }
        if trimmed.starts_with('[') && in_section {
            break;
        }
        if in_section && trimmed.starts_with("bell") {
            if let Some(val) = trimmed.split('=').nth(1) {
                return val.trim() == "true";
            }
        }
    }
    false // default to false
}

fn parse_notifications_duration(content: &str) -> i32 {
    let mut in_section = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[notifications]" {
            in_section = true;
            continue;
        }
        if trimmed.starts_with('[') && in_section {
            break;
        }
        if in_section && trimmed.starts_with("duration") {
            if let Some(val) = trimmed.split('=').nth(1) {
                if let Ok(d) = val.trim().parse::<i32>() {
                    return d;
                }
            }
        }
    }
    5 // default to 5 seconds
}

fn send_ipc_command(cmd: &str) {
    if let Ok(mut stream) = std::os::unix::net::UnixStream::connect(get_socket_path()) {
        let _ = stream.write_all(format!("{}\n", cmd).as_bytes());
    }
}

fn write_config_value(key: &str, value: &str) {
    let content = fs::read_to_string(CONFIG_PATH).unwrap_or_default();
    let new_line = format!("{} = {}", key, value);

    let mut found = false;
    let mut updated_lines = Vec::new();
    let mut in_section = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[notifications]" {
            in_section = true;
            updated_lines.push(line.to_string());
            continue;
        }
        if trimmed.starts_with('[') && in_section {
            in_section = false;
        }
        if in_section && trimmed.starts_with(key) {
            found = true;
            updated_lines.push(new_line.clone());
        } else {
            updated_lines.push(line.to_string());
        }
    }

    let mut updated = updated_lines.join("\n");

    if !found {
        let mut result = String::new();
        let has_section = content.lines().any(|l| l.trim() == "[notifications]");
        if has_section {
            let mut in_section = false;
            let mut inserted = false;
            for line in updated.lines() {
                if line.trim() == "[notifications]" {
                    in_section = true;
                    result.push_str(line);
                    result.push('\n');
                    continue;
                }
                if line.trim().starts_with('[') && in_section {
                    if !inserted {
                        result.push_str(&new_line);
                        result.push('\n');
                        inserted = true;
                    }
                    in_section = false;
                }
                result.push_str(line);
                result.push('\n');
            }
            if !inserted {
                result.push_str(&new_line);
                result.push('\n');
            }
            updated = result;
        } else {
            updated.push_str("\n[notifications]\n");
            updated.push_str(&new_line);
            updated.push_str("\n");
        }
    }
    let _ = fs::write(CONFIG_PATH, updated);
}

fn write_enable_notifications(enabled: bool) {
    write_config_value("enable", &enabled.to_string());
    send_ipc_command("reload");
}


thread_local! {
    static TEST_CONFIG_PATH: std::cell::RefCell<Option<String>> = std::cell::RefCell::new(None);
}

fn get_config_path() -> String {
    #[cfg(test)]
    {
        TEST_CONFIG_PATH.with(|p| {
            if let Some(path) = p.borrow().as_ref() {
                return path.clone();
            }
            "/home/lsgalante/.config/cce/config.toml".to_string()
        })
    }
    #[cfg(not(test))]
    {
        "/home/lsgalante/.config/cce/config.toml".to_string()
    }
}

fn write_status_value(key: &str, value: &str) {
    crate::pages::interface::write_config_value_path(&get_config_path(), key, value);
}

fn read_status_font_size() -> Option<u16> {
    let content = std::fs::read_to_string(&get_config_path()).ok()?;
    Some(parse_u16_from(&content, "status_font_size", 11))
}


fn read_status_padding() -> Option<u16> {
    let content = std::fs::read_to_string(&get_config_path()).ok()?;
    Some(parse_u16_from(&content, "status_padding", 8))
}

fn write_status_padding(padding: u16) {
    write_status_value("status_padding", &padding.to_string());
}

fn read_status_separators() -> Option<bool> {
    let content = std::fs::read_to_string(&get_config_path()).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("status_separators") {
            let rest = rest.trim_start_matches(|c: char| c == ' ' || c == '=' || c == '"');
            if let Ok(val) = rest.trim_end_matches('"').trim().parse::<bool>() {
                return Some(val);
            }
        }
    }
    Some(true)
}

fn write_status_separators(val: bool) {
    write_status_value("status_separators", &val.to_string());
}

fn read_status_underline() -> Option<bool> {
    let content = std::fs::read_to_string(&get_config_path()).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("status_underline") {
            let rest = rest.trim_start_matches(|c: char| c == ' ' || c == '=' || c == '"');
            if let Ok(val) = rest.trim_end_matches('"').trim().parse::<bool>() {
                return Some(val);
            }
        }
    }
    Some(true)
}

fn write_status_underline(val: bool) {
    write_status_value("status_underline", &val.to_string());
}

fn status_interface_reload() {
    let _ = std::process::Command::new("pkill")
        .args(["-f", "cce-status-interface"])
        .status();
    std::thread::sleep(std::time::Duration::from_millis(150));
    send_ipc_command("spawn cce-status-interface");
}

pub async fn fetch_status_state() -> StatusData {
    let running = tokio::process::Command::new("pgrep")
        .args(["-f", "cce-status-interface"]).output().await.ok()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false);

    let font_size = read_status_font_size().unwrap_or(11);
    let padding = read_status_padding().unwrap_or(8);
    let separators = read_status_separators().unwrap_or(true);
    let underline = read_status_underline().unwrap_or(true);

    StatusData {
        font_size,
        padding,
        separators,
        underline,
        running,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_layout_grid() {
        let mut state = ServicesState::default();
        let mut layout = clear_ui::layout::ColumnLayout::new(20.0);
        let sec_focused = vec![false, false];
        let mut ctx = clear_ui::context::UiContext::new();
        let pc = view(&mut state, 10.0, 20.0, 800.0, 600.0, &sec_focused, &mut layout, &mut ctx);
        assert!(!pc.rects.is_empty() || !pc.texts.is_empty());
    }

    #[test]
    fn test_parse_notifications_enable_default() {
        assert!(parse_notifications_enable(""));
        assert!(parse_notifications_enable("[layout]\ngap = 18\n"));
    }

    #[test]
    fn test_parse_notifications_enable_explicit() {
        let content = "\
[notifications]
enable = false
";
        assert!(!parse_notifications_enable(content));

        let content = "\
[notifications]
enable = true
";
        assert!(parse_notifications_enable(content));
    }

    #[test]
    fn test_parse_notifications_enable_other_sections() {
        let content = "\
[layout]
enable = false

[notifications]
enable = true

[input]
enable = false
";
        assert!(parse_notifications_enable(content));

        let content = "\
[layout]
enable = true

[notifications]
enable = false

[input]
enable = true
";
        assert!(!parse_notifications_enable(content));
    }

    #[test]
    fn test_parse_notifications_duration_default() {
        assert_eq!(parse_notifications_duration(""), 5);
        assert_eq!(parse_notifications_duration("[notifications]\n"), 5);
    }

    #[test]
    fn test_parse_notifications_duration_explicit() {
        let content = "\
[notifications]
duration = 10
";
        assert_eq!(parse_notifications_duration(content), 10);
    }

    #[test]
    fn test_read_write_separators() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_status_separators.toml");
        let path_str = path.to_str().unwrap().to_string();

        let _ = fs::write(&path_str, "[layout]\nstatus_separators = true\nstatus_padding = 8\n");
        TEST_CONFIG_PATH.with(|p| *p.borrow_mut() = Some(path_str));

        let original = read_status_separators().unwrap_or(true);
        write_status_separators(!original);
        assert_eq!(read_status_separators(), Some(!original));
        write_status_separators(original);
        assert_eq!(read_status_separators(), Some(original));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_read_write_padding() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_status_padding.toml");
        let path_str = path.to_str().unwrap().to_string();

        let _ = fs::write(&path_str, "[layout]\nstatus_separators = true\nstatus_padding = 8\n");
        TEST_CONFIG_PATH.with(|p| *p.borrow_mut() = Some(path_str));

        let original = read_status_padding().unwrap_or(8);
        write_status_padding(12);
        assert_eq!(read_status_padding(), Some(12));
        write_status_padding(original);
        assert_eq!(read_status_padding(), Some(original));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_read_write_underline() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_status_underline.toml");
        let path_str = path.to_str().unwrap().to_string();

        let _ = fs::write(&path_str, "[layout]\nstatus_underline = true\nstatus_padding = 8\n");
        TEST_CONFIG_PATH.with(|p| *p.borrow_mut() = Some(path_str));

        let original = read_status_underline().unwrap_or(true);
        write_status_underline(!original);
        assert_eq!(read_status_underline(), Some(!original));
        write_status_underline(original);
        assert_eq!(read_status_underline(), Some(original));

        let _ = fs::remove_file(path);
    }
}

