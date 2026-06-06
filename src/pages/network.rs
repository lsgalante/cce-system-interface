use crate::app::{AppAction, PageContent, SectionContextExt};
use clear_ui::layout::{render_widget, PageLayoutBuilder, LayoutStrategy};
use clear_ui::widget::ScrollingList;

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

#[derive(Debug, Clone, Default)]
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
    pub wifi_list_box: ScrollingList,
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
        .args(["-t", "-f", "NAME,DEVICE,SIGNAL", "con", "show", "--active"])
        .output().await.ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    let (connected_ssid, device) = active.lines()
        .filter(|l| !l.contains(":lo:"))
        .filter_map(|l| {
            let parts: Vec<&str> = l.splitn(3, ':').collect();
            if parts.len() >= 2 && !parts[0].is_empty() {
                Some((parts[0].to_string(), parts[1].to_string()))
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
                .map(|l| l.split('/').next().unwrap_or(l).to_string())
        }).unwrap_or_default();

    let available = if wifi_enabled { fetch_wifi_list().await } else { Vec::new() };
    let (bt_installed, bt_service_active, bt_enabled, bt_devices) = fetch_bluetooth_state().await;

    NetworkState {
        loaded: true,
        wifi_enabled, connected_ssid, signal_strength: signal,
        ip_address, device, available,
        bt_installed, bt_service_active,
        bt_enabled, bt_devices, bt_scanning: false,
        wifi_list_box: ScrollingList::new(26.0, 4.0),
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

pub fn view(state: &mut NetworkState, cx: f32, cy: f32, cw: f32, ch: f32, root_focused: bool, layout: &mut dyn LayoutStrategy) -> PageContent {
    let mut final_pc = PageContent::new();
    let sec_w = 320.0f32;
    let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(2);

    // ── WiFi ──
    builder.add_section(&mut final_pc, "WiFi", root_focused, |sec| {
        let rx = sec.left;

        if !state.loaded {
            sec.text("Loading WiFi interfaces...", 12.0, 0.0, 12.0, TEXT_DIM);
            sec.spacing(18.0);
        } else {
            let yt = sec.ay();
            let wifi_btn_w = if sec_w < 200.0 { 40.0 } else { 60.0 };
            let wifi_btn_x = sec_w - wifi_btn_w - 12.0;

            sec.button(if state.wifi_enabled { "ON" } else { "OFF" },
                sec.ax(wifi_btn_x), yt, wifi_btn_w, 28.0,
                if state.wifi_enabled { TOGGLE_ON } else { TOGGLE_OFF }, BTN_HOVER, WHITE,
                AppAction::Radios(NetworkMessage::ToggleWifi));
            sec.content_y += 34.0;

            if !state.connected_ssid.is_empty() {
                let ssid_max_chars = ((sec_w - 24.0) / 7.0) as usize;
                let ssid_truncated = if state.connected_ssid.len() > ssid_max_chars {
                    format!("{}...", &state.connected_ssid[..ssid_max_chars.saturating_sub(3)])
                } else {
                    state.connected_ssid.clone()
                };
                sec.text(&format!("Connected: {}", ssid_truncated), 14.0, 0.0, 13.0, ACCENT);
                sec.spacing(18.0);

                if sec_w < 220.0 {
                    sec.text(&format!("Signal: {}%", state.signal_strength), 14.0, 0.0, 12.0, TEXT_DIM);
                    sec.spacing(16.0);
                    if !state.ip_address.is_empty() {
                        sec.text(&format!("IP: {}", state.ip_address), 14.0, 0.0, 12.0, TEXT_DIM);
                        sec.spacing(16.0);
                    }
                } else {
                    sec.text(&format!("Signal: {}%  IP: {}", state.signal_strength, state.ip_address),
                        14.0, 0.0, 12.0, TEXT_DIM);
                    sec.spacing(16.0);
                }
            } else if state.wifi_enabled {
                sec.text("Not connected", 14.0, 0.0, 12.0, TEXT_DIM);
                sec.spacing(16.0);
            }

            if state.wifi_enabled && !state.available.is_empty() {
                let list_box_x = rx + 12.0;
                let list_box_y = sec.ay();
                let list_box_w = sec_w - 24.0;
                let list_box_h = 160.0;

                render_widget(sec.pc, &mut state.wifi_list_box, list_box_x, list_box_y, list_box_w, list_box_h);

                state.wifi_list_box.update_bounds(state.available.len(), list_box_y, list_box_h);

                let btn_w = list_box_w - 24.0;
                let max_chars = ((btn_w / 6.5) as usize).saturating_sub(10).max(5);

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
                        sec.button(&label, list_box_x + 4.0, draw_y, btn_w, 26.0,
                            if active { ACT_BTN } else { NET_BTN }, BTN_HOVER,
                            if active { ACCENT } else { TEXT_FG },
                            AppAction::Radios(NetworkMessage::ConnectWifi(net.ssid.clone())));
                    }
                }
                sec.content_y += list_box_h + 8.0;
            }
        }
    });

    // ── Bluetooth ──
    builder.add_section_with_width(&mut final_pc, sec_w * 2.0, "Bluetooth", false, |sec| {
        let bt_sec_w = sec_w * 2.0;
        let rx = sec.left;

        if !state.loaded {
            sec.text("Loading Bluetooth status...", 12.0, 0.0, 12.0, TEXT_DIM);
            sec.spacing(18.0);
        } else if !state.bt_installed {
            sec.text("Bluetooth tools (bluez) not installed", 14.0, 0.0, 12.0, TEXT_DIM);
            sec.spacing(18.0);
            let btn_w = if bt_sec_w < 200.0 { 100.0 } else { 120.0 };
            let yt = sec.ay();
            sec.button("Install Tools", rx + 12.0, yt, btn_w, 28.0,
                TOGGLE_ON, BTN_HOVER, WHITE,
                AppAction::Radios(NetworkMessage::InstallBtTools));
            sec.content_y += 34.0;
        } else if !state.bt_service_active {
            sec.text("Bluetooth service is stopped", 14.0, 0.0, 12.0, TEXT_DIM);
            sec.spacing(18.0);
            let btn_w = if bt_sec_w < 200.0 { 100.0 } else { 120.0 };
            let yt = sec.ay();
            sec.button("Start Service", rx + 12.0, yt, btn_w, 28.0,
                TOGGLE_ON, BTN_HOVER, WHITE,
                AppAction::Radios(NetworkMessage::StartBtService));
            sec.content_y += 34.0;
        } else {
            let yt = sec.ay();
            let bt_btn_w = if bt_sec_w < 200.0 { 40.0 } else { 60.0 };
            let scan_btn_w = if bt_sec_w < 200.0 { 40.0 } else { 52.0 };
            let bt_btn_x = bt_sec_w - bt_btn_w - scan_btn_w - 20.0;
            let scan_btn_x = bt_sec_w - scan_btn_w - 12.0;

            sec.button(if state.bt_enabled { "ON" } else { "OFF" },
                sec.ax(bt_btn_x), yt, bt_btn_w, 28.0,
                if state.bt_enabled { TOGGLE_ON } else { TOGGLE_OFF }, BTN_HOVER, WHITE,
                AppAction::Radios(NetworkMessage::ToggleBluetooth));
            sec.button("Scan", sec.ax(scan_btn_x), yt, scan_btn_w, 28.0,
                TOGGLE_OFF, BTN_HOVER, WHITE,
                AppAction::Radios(NetworkMessage::BtScan));
            sec.content_y += 34.0;

            if state.bt_devices.is_empty() {
                if state.bt_enabled {
                    let no_devices_msg = if bt_sec_w < 200.0 { "No paired devices" } else { "No paired devices found" };
                    sec.text(no_devices_msg, 14.0, 0.0, 12.0, TEXT_DIM);
                }
            } else {
                for dev in &state.bt_devices {
                    let status = if dev.connected { ">" } else { " " };
                    let btn_w = if bt_sec_w < 250.0 { 42.0 } else { 70.0 };
                    let action_label = if dev.connected {
                        if bt_sec_w < 250.0 { "Disc" } else { "Disconnect" }
                    } else {
                        if bt_sec_w < 250.0 { "Conn" } else { "Connect" }
                    };

                    let label_max_w = (bt_sec_w - btn_w - 20.0 - 14.0 - 8.0).max(20.0);
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
                    sec.text(&label, 14.0, 0.0, 12.0, if dev.connected { ACCENT } else { TEXT_FG });
                    sec.button(action_label, sec.ax(bt_sec_w - btn_w - 20.0), yt - 2.0, btn_w, 22.0,
                        if dev.connected { TOGGLE_OFF } else { TOGGLE_ON }, BTN_HOVER, WHITE,
                        if dev.connected {
                            AppAction::Radios(NetworkMessage::BtDisconnect(dev.mac.clone()))
                        } else {
                            AppAction::Radios(NetworkMessage::BtConnect(dev.mac.clone()))
                        });
                    sec.content_y += 24.0;
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
            *state = new;
            state.wifi_list_box.set_scroll_y(old_scroll);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_layout_grid() {
        let mut state = NetworkState::default();
        let mut layout = clear_ui::layout::ColumnLayout::new(20.0);
        let pc = view(&mut state, 10.0, 20.0, 800.0, 600.0, false, &mut layout);
        assert!(!pc.rects.is_empty() || !pc.texts.is_empty() || !pc.buttons.is_empty());
    }
}
