use crate::app::{AppAction, PageContent, SectionContextExt};
use cce_ui::layout::{render_widget, PageLayoutBuilder, LayoutStrategy, RenderTarget};
use cce_ui::widget::{List, Toggle, Element};

#[derive(Debug, Clone)]
pub struct WifiNetwork {
    pub ssid: String,
    pub signal: u8,
    pub secured: bool,
    pub in_use: bool,
}

#[derive(Debug, Clone)]
pub struct BluetoothDevice {
    pub mac: String,
    pub name: String,
    pub icon: String,
    pub connected: bool,
}

#[derive(Debug, Clone)]
pub struct NetworkState {
    pub loaded: bool,
    pub wifi_enabled: bool,
    pub connected_ssid: String,
    pub signal_strength: u8,
    pub ip_address: String,
    pub device: String,
    pub available: Vec<WifiNetwork>,
    pub bt_installed: bool,
    pub bt_service_active: bool,
    pub bt_enabled: bool,
    pub bt_devices: Vec<BluetoothDevice>,
    pub bt_scanning: bool,
    pub wifi_list_box: List,
    pub wifi_toggle: Toggle,
    pub bt_toggle: Toggle,
}

impl Default for NetworkState {
    fn default() -> Self {
        Self {
            loaded: false,
            wifi_enabled: false,
            connected_ssid: String::new(),
            signal_strength: 0,
            ip_address: String::new(),
            device: String::new(),
            available: Vec::new(),
            bt_installed: false,
            bt_service_active: false,
            bt_enabled: false,
            bt_devices: Vec::new(),
            bt_scanning: false,
            wifi_list_box: List::new(26.0, 4.0),
            wifi_toggle: Toggle::new(),
            bt_toggle: Toggle::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum NetworkMessage {
    Refreshed(NetworkState),
    ToggleWifi,
    ConnectWifi(String),
    ToggleBluetooth,
    BtConnect(String),
    BtDisconnect(String),
    BtScan,
    InstallBtTools,
    StartBtService,
}

pub async fn fetch_network_state() -> NetworkState {
    let wifi_enabled = tokio::process::Command::new("nmcli")
        .args(["radio", "wifi"]).output().await.ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().starts_with("enabled"))
        .unwrap_or(false);

    let active = tokio::process::Command::new("nmcli")
        .args(["-t", "-f", "NAME,DEVICE,TYPE", "con", "show", "--active"])
        .output().await.ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    let (connected_ssid, device) = active.lines()
        .filter_map(|l| {
            let parts: Vec<&str> = l.split(':').collect();
            if parts.len() >= 3 && parts[parts.len() - 1] == "802-11-wireless" {
                let ssid = parts[..parts.len() - 2].join(":");
                let ssid_unescaped = ssid.replace("\\:", ":");
                let device = parts[parts.len() - 2].to_string();
                Some((ssid_unescaped, device))
            } else { None }
        }).next().unwrap_or_default();

    let signal = tokio::process::Command::new("nmcli")
        .args(["-t", "-f", "ACTIVE,SIGNAL", "dev", "wifi", "list"])
        .output().await.ok()
        .and_then(|o| {
            String::from_utf8_lossy(&o.stdout).lines()
                .find(|l| l.starts_with("yes:"))
                .and_then(|l| l.split(':').nth(1))
                .and_then(|v| v.parse::<u8>().ok())
        }).unwrap_or(0);

    let ip_address = tokio::process::Command::new("nmcli")
        .args(["-t", "-f", "IP4.ADDRESS", "dev", "show", &device])
        .output().await.ok()
        .and_then(|o| {
            String::from_utf8_lossy(&o.stdout).lines()
                .find(|l| !l.is_empty())
                .map(|l| {
                    let val = l.split(':').nth(1).unwrap_or(l);
                    val.split('/').next().unwrap_or(val).to_string()
                })
        }).unwrap_or_default();

    let available = if wifi_enabled { fetch_wifi_list().await } else { Vec::new() };
    let (bt_installed, bt_service_active, bt_enabled, bt_devices) = fetch_bluetooth_state().await;

    NetworkState {
        loaded: true,
        wifi_enabled, connected_ssid, signal_strength: signal,
        ip_address, device, available,
        bt_installed, bt_service_active,
        bt_enabled, bt_devices, bt_scanning: false,
        wifi_list_box: List::new(26.0, 4.0),
        wifi_toggle: Toggle::new(),
        bt_toggle: Toggle::new(),
    }
}

async fn fetch_wifi_list() -> Vec<WifiNetwork> {
    let out = match tokio::process::Command::new("nmcli")
        .args(["-t", "-f", "SSID,SIGNAL,SECURITY,IN-USE", "dev", "wifi", "list"])
        .output().await
    {
        Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(),
        Err(_) => return Vec::new(),
    };

    let mut networks = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for line in out.lines() {
        let parts: Vec<&str> = line.splitn(4, ':').collect();
        if parts.len() >= 3 {
            let ssid = parts[0].to_string();
            if ssid.is_empty() || ssid == "--" || seen.contains(&ssid) { continue; }
            seen.insert(ssid.clone());
            networks.push(WifiNetwork {
                ssid, signal: parts[1].parse::<u8>().unwrap_or(0),
                secured: !parts[2].is_empty(),
                in_use: parts.len() > 3 && parts[3] == "*",
            });
        }
    }
    networks.sort_by(|a, b| b.signal.cmp(&a.signal));
    networks
}

async fn fetch_bluetooth_state() -> (bool, bool, bool, Vec<BluetoothDevice>) {
    let bt_installed = tokio::process::Command::new("bluetoothctl")
        .arg("--version")
        .output()
        .await
        .is_ok();

    if !bt_installed {
        return (false, false, false, Vec::new());
    }

    let bt_service_active = tokio::process::Command::new("systemctl")
        .args(["is-active", "bluetooth"])
        .output()
        .await
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "active")
        .unwrap_or(false);

    if !bt_service_active {
        return (true, false, false, Vec::new());
    }

    let bt_enabled = tokio::process::Command::new("bluetoothctl")
        .args(["show"]).output().await.ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).lines().any(|l| l.contains("Powered: yes")))
        .unwrap_or(false);

    let devices = if bt_enabled { fetch_bt_devices().await } else { Vec::new() };
    (true, true, bt_enabled, devices)
}

async fn fetch_bt_devices() -> Vec<BluetoothDevice> {
    let out = match tokio::process::Command::new("bluetoothctl")
        .args(["devices"]).output().await
    {
        Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(),
        Err(_) => return Vec::new(),
    };

    let mut devices = Vec::new();
    for line in out.lines() {
        let rest = line.strip_prefix("Device ").unwrap_or("");
        let parts: Vec<&str> = rest.splitn(2, ' ').collect();
        if parts.len() < 2 || parts[0].is_empty() || parts[1].is_empty() { continue; }
        let mac = parts[0].to_string();
        let default_name = parts[1].to_string();

        let info = tokio::process::Command::new("bluetoothctl")
            .args(["info", &mac]).output().await.ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default();

        let info_name = info.lines()
            .find(|l| l.contains("Name:"))
            .and_then(|l| l.splitn(2, ':').nth(1).map(|s| s.trim().to_string()));

        let info_alias = info.lines()
            .find(|l| l.contains("Alias:"))
            .and_then(|l| l.splitn(2, ':').nth(1).map(|s| s.trim().to_string()));

        let name = info_name.or(info_alias).unwrap_or(default_name);

        let connected = info.lines().any(|l| l.contains("Connected: yes"));
        let icon = info.lines()
            .find(|l| l.contains("Icon:"))
            .and_then(|l| l.split(':').nth(1).map(|s| s.trim().to_string()))
            .unwrap_or_else(|| "audio-card".into());

        devices.push(BluetoothDevice { mac, name, icon, connected });
    }
    devices
}

fn wifi_connect(ssid: &str) {
    let _ = tokio::process::Command::new("nmcli")
        .args(["dev", "wifi", "connect", ssid]).spawn();
}

fn wifi_toggle(enable: bool) {
    let _ = tokio::process::Command::new("nmcli")
        .args(["radio", "wifi", if enable { "on" } else { "off" }]).spawn();
}

fn bt_toggle(enable: bool) {
    let _ = tokio::process::Command::new("bluetoothctl")
        .args([if enable { "power" } else { "power" }, if enable { "on" } else { "off" }]).spawn();
}

fn bt_connect(mac: &str) {
    let _ = tokio::process::Command::new("bluetoothctl")
        .args(["connect", mac]).spawn();
}

fn bt_disconnect(mac: &str) {
    let _ = tokio::process::Command::new("bluetoothctl")
        .args(["disconnect", mac]).spawn();
}

fn bt_scan() {
    let _ = tokio::process::Command::new("bluetoothctl")
        .args(["scan", "on"]).spawn();
    let _ = tokio::process::Command::new("sh")
        .args(["-c", "sleep 5 && bluetoothctl scan off"]).spawn();
}

const TEXT_FG: [f32; 4] = [0.83, 0.83, 0.83, 1.0];
const TEXT_DIM: [f32; 4] = [0.53, 0.53, 0.60, 1.0];
const ACCENT: [f32; 4] = [0.36, 0.56, 0.38, 1.0];
const TOGGLE_ON: [f32; 4] = [0.16, 0.41, 0.18, 1.0];
const TOGGLE_OFF: [f32; 4] = [0.16, 0.16, 0.24, 1.0];
const BTN_HOVER: [f32; 4] = [0.25, 0.30, 0.26, 1.0];
const NET_BTN: [f32; 4] = [0.13, 0.20, 0.27, 1.0];
const ACT_BTN: [f32; 4] = [0.16, 0.29, 0.18, 1.0];
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

pub fn view(state: &mut NetworkState, cx: f32, cy: f32, cw: f32, ch: f32, root_focused: bool, layout: &mut dyn LayoutStrategy, ctx: &mut cce_ui::context::UiContext) -> PageContent {
    let mut final_pc = PageContent::new();
    let sec_w = 320.0f32;
    let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(2);

    // ── WiFi ──
    builder.add_section(&mut final_pc, "WiFi", root_focused, |sec| {
        let sec_w = sec.cw;
        let rx = sec.left;
        let padding = sec.padding();
        let row_gap = cce_ui::layout::label_margin();
        let margin = padding.max(12.0);

        if !state.loaded {
            sec.text("Loading WiFi interfaces...", margin, 0.0, 12.0, TEXT_DIM);
        } else {
            let wifi_btn_w = if sec_w < 200.0 { 40.0 } else { 60.0 };
            let wifi_btn_x = margin;

            state.wifi_toggle.set_toggled(state.wifi_enabled);
            state.wifi_toggle.set_label(if state.wifi_enabled { "ON" } else { "OFF" });
            sec.widget(&mut state.wifi_toggle, wifi_btn_x, wifi_btn_w, 28.0, ctx);

            if state.wifi_enabled {
                let status_y = sec.ay();
                let font_size_1 = 13.0;
                let font_size_2 = 12.0;
                let status_area_h;

                if !state.connected_ssid.is_empty() {
                    let y1 = 0.0;
                    let y2 = font_size_1 + row_gap;
                    status_area_h = y2 + font_size_2 + row_gap;

                    let ssid_max_chars = ((sec_w - 2.0 * margin) / 7.0) as usize;
                    let ssid_truncated = if state.connected_ssid.len() > ssid_max_chars {
                        format!("{}...", &state.connected_ssid[..ssid_max_chars.saturating_sub(3)])
                    } else {
                        state.connected_ssid.clone()
                    };
                    sec.text(&format!("Connected: {}", ssid_truncated), margin, y1, font_size_1, ACCENT);

                    if sec_w < 220.0 {
                        sec.text(&format!("Signal: {}%", state.signal_strength), margin, y2, font_size_2, TEXT_DIM);
                    } else {
                        sec.text(&format!("Signal: {}%  IP: {}", state.signal_strength, state.ip_address),
                            margin, y2, font_size_2, TEXT_DIM);
                    }
                } else {
                    status_area_h = font_size_2 + row_gap;
                    sec.text("Not connected", margin, 0.0, font_size_2, TEXT_DIM);
                }
                sec.content_y = status_y + status_area_h;
            }

            if state.wifi_enabled && !state.available.is_empty() {
                let list_box_x = rx + margin;
                let list_box_y = sec.ay();
                let list_box_w = sec_w - 2.0 * margin;
                let list_box_h = 160.0;

                render_widget(sec.pc, &mut state.wifi_list_box, list_box_x, list_box_y, list_box_w, list_box_h, ctx);

                state.wifi_list_box.update_bounds(state.available.len(), list_box_y, list_box_h);

                let btn_w = list_box_w - 2.0 * margin;
                let max_chars = ((btn_w / 6.5) as usize).saturating_sub(10).max(5);

                sec.pc.push_clip_rect(list_box_x, list_box_y, list_box_w, list_box_h);
                for (idx, net) in state.available.iter().enumerate() {
                    if let Some(draw_y) = state.wifi_list_box.get_item_draw_y(idx, 4.0) {
                        let prefix = if net.in_use { ">" } else { " " };
                        let ssid_truncated = if net.ssid.len() > max_chars {
                            format!("{}...", &net.ssid[..max_chars.saturating_sub(3)])
                        } else {
                            net.ssid.clone()
                        };
                        let label = format!("{}  {}  ({}%)", prefix, ssid_truncated, net.signal);
                        let active = net.in_use;
                        sec.pc.button(&label, list_box_x + margin, draw_y, btn_w, 26.0,
                            if active { ACT_BTN } else { NET_BTN }, BTN_HOVER,
                            if active { ACCENT } else { TEXT_FG },
                            AppAction::Radios(NetworkMessage::ConnectWifi(net.ssid.clone())));
                    }
                }
                sec.pc.pop_clip_rect();
                sec.content_y += list_box_h + row_gap;
            }
        }
    });

    // ── Bluetooth ──
    builder.add_section(&mut final_pc, "Bluetooth", false, |sec| {
        let bt_sec_w = sec.cw;
        let padding = sec.padding();
        let row_gap = cce_ui::layout::label_margin();
        let margin = padding.max(12.0);
        let font_size = 12.0;
        let btn_h = 28.0;

        if !state.loaded {
            sec.text("Loading Bluetooth status...", margin, 0.0, font_size, TEXT_DIM);
        } else if !state.bt_installed {
            sec.text("Bluetooth tools (bluez) not installed", margin, 0.0, font_size, TEXT_DIM);
            let btn_w = if bt_sec_w < 200.0 { 100.0 } else { 120.0 };
            let yt = sec.ay();
            sec.button("Install Tools", sec.ax(margin), yt, btn_w, btn_h,
                TOGGLE_ON, BTN_HOVER, WHITE,
                AppAction::Radios(NetworkMessage::InstallBtTools));
            sec.content_y = yt + btn_h + row_gap;
        } else if !state.bt_service_active {
            sec.text("Bluetooth service is stopped", margin, 0.0, font_size, TEXT_DIM);
            let btn_w = if bt_sec_w < 200.0 { 100.0 } else { 120.0 };
            let yt = sec.ay();
            sec.button("Start Service", sec.ax(margin), yt, btn_w, btn_h,
                TOGGLE_ON, BTN_HOVER, WHITE,
                AppAction::Radios(NetworkMessage::StartBtService));
            sec.content_y = yt + btn_h + row_gap;
        } else {
            let yt = sec.ay();
            let bt_btn_w = if bt_sec_w < 200.0 { 40.0 } else { 60.0 };
            let scan_btn_w = if bt_sec_w < 200.0 { 40.0 } else { 52.0 };
            let bt_btn_x = margin;
            let scan_btn_x = margin + bt_btn_w + row_gap;
 
            state.bt_toggle.set_toggled(state.bt_enabled);
            state.bt_toggle.set_label(if state.bt_enabled { "ON" } else { "OFF" });
            sec.widget(&mut state.bt_toggle, bt_btn_x, bt_btn_w, btn_h, ctx);
            sec.button("Scan", sec.ax(scan_btn_x), yt, scan_btn_w, btn_h,
                TOGGLE_OFF, BTN_HOVER, WHITE,
                AppAction::Radios(NetworkMessage::BtScan));
            sec.content_y = yt + btn_h + row_gap;
 
            if state.bt_devices.is_empty() {
                if state.bt_enabled {
                    let no_devices_msg = if bt_sec_w < 200.0 { "No paired devices" } else { "No paired devices found" };
                    sec.text(no_devices_msg, margin, 0.0, font_size, TEXT_DIM);
                }
            } else {
                let item_h = 22.0;
                for dev in &state.bt_devices {
                    let status = if dev.connected { ">" } else { " " };
                    let btn_w = if bt_sec_w < 250.0 { 42.0 } else { 70.0 };
                    let action_label = if dev.connected {
                        if bt_sec_w < 250.0 { "Disc" } else { "Disconnect" }
                    } else {
                        if bt_sec_w < 250.0 { "Conn" } else { "Connect" }
                    };
 
                    let label_max_w = (bt_sec_w - btn_w - 2.0 * padding - margin - row_gap).max(20.0);
                    let label_max_chars = ((label_max_w / 6.0) as usize).max(5);
 
                    let is_unknown = dev.name.replace('-', ":").eq_ignore_ascii_case(&dev.mac);
                    let label = if is_unknown {
                        if bt_sec_w < 350.0 {
                            format!("{} {}", status, dev.mac)
                        } else {
                            format!("{} Unknown Device ({})", status, dev.mac)
                        }
                    } else {
                        if bt_sec_w < 350.0 {
                            let name_truncated = if dev.name.len() > label_max_chars {
                                format!("{}...", &dev.name[..label_max_chars.saturating_sub(3)])
                            } else {
                                dev.name.clone()
                            };
                            format!("{} {}", status, name_truncated)
                        } else {
                            let full_label = format!("{} {} ({})", status, dev.name, dev.mac);
                            if full_label.len() > label_max_chars {
                                format!("{}...", &full_label[..label_max_chars.saturating_sub(3)])
                            } else {
                                full_label
                            }
                        }
                    };
 
                    let yt = sec.ay();
                    let btn_x = margin;
                    let text_x = margin + btn_w + row_gap;
                    let text_y_offset = (item_h - font_size) / 2.0;
                    sec.button(action_label, sec.ax(btn_x), yt, btn_w, item_h,
                        if dev.connected { TOGGLE_OFF } else { TOGGLE_ON }, BTN_HOVER, WHITE,
                        if dev.connected {
                            AppAction::Radios(NetworkMessage::BtDisconnect(dev.mac.clone()))
                        } else {
                            AppAction::Radios(NetworkMessage::BtConnect(dev.mac.clone()))
                        });
                    sec.text(&label, text_x, text_y_offset, font_size, if dev.connected { ACCENT } else { TEXT_FG });
                    sec.content_y = yt + item_h + row_gap;
                }
            }
        }
    });

    final_pc
}

pub fn update(state: &mut NetworkState, msg: NetworkMessage) {
    match msg {
        NetworkMessage::Refreshed(new) => {
            let old_scroll = state.wifi_list_box.scroll_y();
            let was_wifi_hovered = state.wifi_toggle.hovered();
            let was_bt_hovered = state.bt_toggle.hovered();
            *state = new;
            state.wifi_list_box.set_scroll_y(old_scroll);
            state.wifi_toggle.set_hovered(was_wifi_hovered);
            state.bt_toggle.set_hovered(was_bt_hovered);
        }
        NetworkMessage::ToggleWifi => {
            state.wifi_enabled = !state.wifi_enabled;
            wifi_toggle(state.wifi_enabled);
        }
        NetworkMessage::ConnectWifi(ssid) => { wifi_connect(&ssid); }
        NetworkMessage::ToggleBluetooth => {
            state.bt_enabled = !state.bt_enabled;
            bt_toggle(state.bt_enabled);
        }
        NetworkMessage::BtConnect(mac) => { bt_connect(&mac); }
        NetworkMessage::BtDisconnect(mac) => { bt_disconnect(&mac); }
        NetworkMessage::BtScan => { bt_scan(); }
        NetworkMessage::InstallBtTools => {
            let _ = tokio::process::Command::new("pkexec")
                .args(["sh", "-c", "pacman -S --noconfirm bluez bluez-utils && systemctl enable --now bluetooth"])
                .spawn();
        }
        NetworkMessage::StartBtService => {
            let _ = tokio::process::Command::new("pkexec")
                .args(["systemctl", "enable", "--now", "bluetooth"])
                .spawn();
        }
    }
}

impl crate::pages::AppPage for NetworkState {
    fn clear_children(&mut self, ctx: &mut cce_ui::context::UiContext) {
        self.wifi_list_box.scroll_box.clear_children(ctx);
        self.wifi_list_box.scroll_box.set_parent(None, ctx);
        self.wifi_toggle.clear_children(ctx);
        self.wifi_toggle.set_parent(None, ctx);
        self.bt_toggle.clear_children(ctx);
        self.bt_toggle.set_parent(None, ctx);
    }

    fn get_section_containers(&self) -> Vec<cce_ui::widget::SectionContainer> {
        vec![
            cce_ui::widget::SectionContainer::new("WiFi").with_layout(cce_ui::widget::AdaptiveGridLayout {
                min_col_width: 140.0,
                gap: 8.0,
                padding_x: 0.0,
                padding_y: 0.0,
                grid: None,
            }),
            cce_ui::widget::SectionContainer::new("Bluetooth").with_layout(cce_ui::widget::AdaptiveGridLayout {
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

        cce_ui::widget::link_parent_child(&mut sec_containers[0], &mut self.wifi_toggle, ctx);
        if self.wifi_enabled && !self.available.is_empty() {
            cce_ui::widget::link_parent_child(&mut sec_containers[0], &mut self.wifi_list_box.scroll_box, ctx);
        }
        cce_ui::widget::link_parent_child(&mut sec_containers[1], &mut self.bt_toggle, ctx);
    }

    fn view(
        &mut self,
        cx: f32,
        cy: f32,
        cw: f32,
        ch: f32,
        root_focused: bool,
        _sec_focused: &[bool],
        layout: &mut dyn LayoutStrategy,
        ctx: &mut cce_ui::context::UiContext,
    ) -> crate::app::PageContent {
        view(self, cx, cy, cw, ch, root_focused, layout, ctx)
    }

    fn propagate_widget_changes(&mut self, actions: &mut Vec<crate::app::AppAction>) {
        if self.wifi_toggle.take_change() {
            actions.push(crate::app::AppAction::Radios(NetworkMessage::ToggleWifi));
        }
        if self.bt_toggle.take_change() {
            actions.push(crate::app::AppAction::Radios(NetworkMessage::ToggleBluetooth));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_layout_grid() {
        let mut state = NetworkState::default();
        state.loaded = true;
        state.wifi_enabled = true;
        let mut layout = cce_ui::layout::ColumnLayout::new(20.0);
        let pc = view(&mut state, 10.0, 20.0, 800.0, 600.0, false, &mut layout, &mut cce_ui::context::UiContext::new());
        assert!(!pc.rects.is_empty() || !pc.texts.is_empty() || !pc.buttons.is_empty());
    }

    #[test]
    fn test_view_layout_connected() {
        let mut state = NetworkState::default();
        state.loaded = true;
        state.wifi_enabled = true;
        state.connected_ssid = "MyHomeWiFi".to_string();
        state.signal_strength = 80;
        state.ip_address = "192.168.1.50".to_string();
        let mut layout = cce_ui::layout::ColumnLayout::new(20.0);
        let pc = view(&mut state, 10.0, 20.0, 800.0, 600.0, false, &mut layout, &mut cce_ui::context::UiContext::new());
        assert!(!pc.rects.is_empty() || !pc.texts.is_empty() || !pc.buttons.is_empty());
    }
}
