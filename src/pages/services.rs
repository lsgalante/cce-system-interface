use crate::app::PageContent;
use clear_ui::layout::Section;
use clear_ui::widget::{Widget, TextLabel, ScrollBox};
use clear_ui::widget::{ElementState, KeyEvent, MouseButton, Key, NamedKey};

// ── TextBox Widget ──

#[derive(Debug, Clone)]
pub struct TextBox {
    x: f32, y: f32, w: f32, h: f32,
    pub text: String,
    pub editing: bool,
    pub edit_buffer: String,
    hovered: bool,
    just_changed: bool,
    label: Option<String>,
    row_x: f32,
    row_w: f32,
}

impl TextBox {
    pub fn new(text: String) -> Self {
        Self {
            x: 0.0, y: 0.0, w: 0.0, h: 0.0,
            text,
            editing: false,
            edit_buffer: String::new(),
            hovered: false,
            just_changed: false,
            label: None,
            row_x: 0.0,
            row_w: 0.0,
        }
    }

    pub fn with_label(mut self, label: &str) -> Self {
        self.label = Some(label.to_string());
        self
    }

    pub fn set_label(&mut self, label: &str) {
        self.label = Some(label.to_string());
    }

    pub fn take_change(&mut self) -> bool {
        let changed = self.just_changed;
        self.just_changed = false;
        changed
    }
}

impl Default for TextBox {
    fn default() -> Self {
        Self::new(String::new())
    }
}

impl Widget for TextBox {
    fn rect(&self) -> (f32, f32, f32, f32) { (self.x, self.y, self.w, self.h) }
    fn set_rect(&mut self, x: f32, y: f32, w: f32, h: f32) { self.x = x; self.y = y; self.w = w; self.h = h; }
    fn set_row_rect(&mut self, x: f32, w: f32) { self.row_x = x; self.row_w = w; }
    fn set_hovered(&mut self, v: bool) { self.hovered = v; }
    fn hovered(&self) -> bool { self.hovered }

    fn color(&self) -> [f32; 4] {
        [0.10, 0.10, 0.16, 1.0]
    }

    fn hit_test(&self, px: f32, py: f32) -> bool {
        let (x, y, w, h) = self.rect();
        let hx = if self.row_w > 0.0 { self.row_x } else { x };
        let hw = if self.row_w > 0.0 { self.row_w } else { w };
        let (hy, hh) = if self.label.is_some() {
            (y - 18.0, h + 18.0)
        } else {
            (y, h)
        };
        px >= hx && px <= hx + hw && py >= hy && py <= hy + hh
    }

    fn top_room(&self) -> f32 { if self.label.is_some() { 18.0 } else { 0.0 } }

    fn cursor_moved(&mut self, px: f32, py: f32) -> bool {
        let was = self.hovered;
        self.hovered = self.hit_test(px, py);
        was != self.hovered
    }

    fn mouse_input(&mut self, button: MouseButton, state: ElementState, px: f32, py: f32) -> bool {
        if button != MouseButton::Left { return false; }
        if state != ElementState::Pressed { return false; }
        if !self.hit_test(px, py) { return false; }
        self.focus();
        true
    }

    fn focus(&mut self) {
        if !self.editing {
            self.editing = true;
            self.edit_buffer = self.text.clone();
        }
    }

    fn unfocus(&mut self) {
        if self.editing {
            self.editing = false;
            if self.text != self.edit_buffer {
                self.text = self.edit_buffer.clone();
                self.just_changed = true;
            }
        }
    }

    fn keyboard_input(&mut self, event: &KeyEvent) -> bool {
        if !self.editing { return false; }
        if event.state != ElementState::Pressed { return false; }
        match &event.logical_key {
            Key::Named(NamedKey::Backspace) => {
                self.edit_buffer.pop();
                true
            }
            Key::Named(NamedKey::Enter) => {
                self.text = self.edit_buffer.clone();
                self.editing = false;
                self.just_changed = true;
                true
            }
            Key::Named(NamedKey::Escape) => {
                self.editing = false;
                true
            }
            _ => {
                if let Some(text) = &event.text {
                    if !event.repeat {
                        for ch in text.chars() {
                            if ch.is_alphanumeric() || ch == ' ' || ch == '-' || ch == '_' || ch == '*' || ch == '.' || ch == '@' {
                                self.edit_buffer.push(ch);
                            }
                        }
                    }
                }
                true
            }
        }
    }

    fn hover_highlight(&self) -> Option<[f32; 4]> {
        None
    }

    fn extra_quads(&self) -> Vec<(f32, f32, f32, f32, [f32; 4])> {
        let mut quads = Vec::new();
        if self.hovered {
            let (hy, hh) = if self.label.is_some() {
                (self.y - 18.0, self.h + 18.0)
            } else {
                (self.y, self.h)
            };
            let hx = if self.row_w > 0.0 { self.row_x } else { self.x };
            let hw = if self.row_w > 0.0 { self.row_w } else { self.w };
            quads.push((hx, hy, hw, hh, [1.0, 1.0, 1.0, 0.06]));
        }
        let bg_color = if self.editing {
            [0.12, 0.12, 0.18, 1.0]
        } else {
            [0.08, 0.08, 0.12, 1.0]
        };
        let border_color = if self.editing {
            [0.30, 0.50, 0.32, 1.0]
        } else if self.hovered {
            [0.25, 0.25, 0.35, 1.0]
        } else {
            [0.18, 0.18, 0.24, 1.0]
        };
        quads.push((self.x, self.y, self.w, self.h, border_color));
        quads.push((self.x + 1.0, self.y + 1.0, self.w - 2.0, self.h - 2.0, bg_color));
        quads
    }

    fn text_labels(&self) -> Vec<TextLabel> {
        let mut labels = Vec::new();
        if let Some(ref label) = self.label {
            labels.push(TextLabel {
                text: label.clone(),
                x: self.x + 4.0,
                y: self.y - 14.0,
                font_size: 12.0,
                color: [0x83, 0x83, 0x8a],
            });
        }
        let val_text = if self.editing {
            format!("{}|", self.edit_buffer)
        } else {
            self.text.clone()
        };
        labels.push(TextLabel {
            text: val_text,
            x: self.x + 8.0,
            y: self.y + (self.h - 12.0) / 2.0,
            font_size: 13.0,
            color: if self.editing { [0xee, 0xee, 0xf5] } else { [0xcc, 0xcc, 0xd4] },
        });
        labels
    }
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
    pub list_box: ScrollBox,
}

impl Default for ServicesState {
    fn default() -> Self {
        Self {
            loaded: false,
            services: Vec::new(),
            active_tab: ServiceTab::System,
            search_box: TextBox::new(String::new()).with_label("Filter Services"),
            list_box: ScrollBox::new(),
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

pub fn view(state: &mut ServicesState, cx: f32, cy: f32, cw: f32, _ch: f32) -> PageContent {
    let mut pc = PageContent::new();
    let y = cy + 12.0;

    let mut sec = Section::new(&mut pc, cx, y, cw, "Services");

    if !state.loaded {
        sec.text(&mut pc, "Loading systemd services...", 12.0, 0.0, 12.0, TEXT_DIM);
        sec.spacing(18.0);
    } else {
        // Tab header buttons: System Services, User Services
        let tab_w = 140.0;
        let tab_h = 28.0;
        let tab_y = sec.ay();
        let active_bg = [0.20, 0.40, 0.65, 0.4];
        let inactive_bg = [0.10, 0.10, 0.16, 0.3];
        let hover_bg = [0.20, 0.20, 0.25, 0.15];

        pc.button(
            "System Services",
            cx + 12.0,
            tab_y,
            tab_w,
            tab_h,
            if state.active_tab == ServiceTab::System { active_bg } else { inactive_bg },
            hover_bg,
            [0.90, 0.90, 0.95, 1.0],
            crate::app::AppAction::Services(ServicesMessage::SetTab(ServiceTab::System)),
        );

        pc.button(
            "User Services",
            cx + 12.0 + tab_w + 8.0,
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
        let search_w = cw - 24.0;
        let search_h = 28.0;
        
        state.search_box.set_row_rect(cx + 12.0, search_w);
        clear_ui::layout::render_widget(
            &mut pc,
            &mut state.search_box,
            cx + 12.0,
            search_y,
            search_w,
            search_h,
        );
        sec.content_y += search_h + state.search_box.top_room() + 16.0;

        // Scroll box list
        let list_box_x = cx + 12.0;
        let list_box_y = sec.ay();
        let list_box_w = cw - 24.0;
        let list_box_h = 360.0;
        
        clear_ui::layout::render_widget(&mut pc, &mut state.list_box, list_box_x, list_box_y, list_box_w, list_box_h);

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

        let item_h = 36.0;
        let item_gap = 6.0;
        let item_height_full = item_h + item_gap;
        let content_h = filtered_services.len() as f32 * item_height_full;

        state.list_box.update_bounds(content_h, list_box_y, list_box_h);

        for (idx, service) in filtered_services.iter().enumerate() {
            let virtual_y = idx as f32 * item_height_full + 4.0;
            if let Some(draw_y) = state.list_box.get_item_draw_y(virtual_y, item_h) {
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

                // Service Name
                pc.text(&service.name, list_box_x + 32.0, draw_y + 4.0, 13.0, [0.90, 0.90, 0.95, 1.0]);

                // Service Description
                let desc = if service.description.is_empty() { "No description" } else { &service.description };
                let desc_truncated = if desc.len() > 65 { format!("{}...", &desc[..62]) } else { desc.to_string() };
                pc.text(&desc_truncated, list_box_x + 32.0, draw_y + 19.0, 11.0, [0.55, 0.55, 0.60, 1.0]);

                // Control buttons: Start, Stop, Restart on the right
                let btn_w = 46.0;
                let r_btn_w = 54.0;
                let btn_gap = 6.0;
                let right_edge = list_box_x + list_box_w - 24.0 - 8.0;

                let restart_x = right_edge - r_btn_w;
                let stop_x = restart_x - btn_gap - btn_w;
                let start_x = stop_x - btn_gap - btn_w;

                let btn_y = draw_y + (item_h - 22.0) / 2.0;
                let btn_h = 22.0;

                let active_txt = [0.90, 0.90, 0.95, 1.0];
                let disabled_txt = [0.40, 0.40, 0.45, 1.0];

                // Start button
                pc.button(
                    "Start",
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
                    "Stop",
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
                    "Restart",
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

    sec.finish(&mut pc);
    pc
}

pub fn update(state: &mut ServicesState, msg: ServicesMessage) {
    match msg {
        ServicesMessage::Refreshed(new_services) => {
            state.loaded = true;
            state.services = new_services;
        }
        ServicesMessage::SetTab(tab) => {
            state.active_tab = tab;
            state.list_box.scroll_y = 0.0;
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
