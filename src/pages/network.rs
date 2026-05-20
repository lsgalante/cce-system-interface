use crate::app::{AppAction, PageContent};

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
    pub wifi_enabled: bool,
    pub connected_ssid: String,
    pub signal_strength: u8,
    pub ip_address: String,
    pub device: String,
    pub available: Vec<WifiNetwork>,
    pub bt_enabled: bool,
    pub bt_devices: Vec<BluetoothDevice>,
    pub bt_scanning: bool,
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
    let (bt_enabled, bt_devices) = fetch_bluetooth_state().await;

    NetworkState {
        wifi_enabled, connected_ssid, signal_strength: signal,
        ip_address, device, available,
        bt_enabled, bt_devices, bt_scanning: false,
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

async fn fetch_bluetooth_state() -> (bool, Vec<BluetoothDevice>) {
    let bt_enabled = tokio::process::Command::new("bluetoothctl")
        .args(["show"]).output().await.ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).lines().any(|l| l.contains("Powered: yes")))
        .unwrap_or(false);

    let devices = if bt_enabled { fetch_bt_devices().await } else { Vec::new() };
    (bt_enabled, devices)
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
        let name = parts[1].to_string();

        let info = tokio::process::Command::new("bluetoothctl")
            .args(["info", &mac]).output().await.ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default();

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
const SECTION_BORDER: [f32; 4] = [0.18, 0.18, 0.27, 1.0];
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

pub fn view(state: &NetworkState, cx: f32, cy: f32, cw: f32, _ch: f32) -> PageContent {
    let mut pc = PageContent::new();
    let mut y = cy + 12.0;

    // ── WiFi ──
    pc.text("WiFi", cx + 12.0, y, 14.0, TEXT_FG);
    pc.button(if state.wifi_enabled { "ON" } else { "OFF" },
        cx + cw - 80.0, y, 60.0, 28.0,
        if state.wifi_enabled { TOGGLE_ON } else { TOGGLE_OFF }, BTN_HOVER, WHITE,
        AppAction::Radios(NetworkMessage::ToggleWifi));
    y += 34.0;

    if !state.connected_ssid.is_empty() {
        pc.text(&format!("Connected: {}", state.connected_ssid), cx + 14.0, y, 13.0, ACCENT);
        y += 18.0;
        pc.text(&format!("Signal: {}%  IP: {}", state.signal_strength, state.ip_address),
            cx + 14.0, y, 12.0, TEXT_DIM);
        y += 16.0;
    } else if state.wifi_enabled {
        pc.text("Not connected", cx + 14.0, y, 12.0, TEXT_DIM);
        y += 16.0;
    }

    if state.wifi_enabled && !state.available.is_empty() {
        for net in &state.available {
            let prefix = if net.in_use { ">" } else { " " };
            let label = format!("{}  {}  ({}%)", prefix, net.ssid, net.signal);
            let active = net.in_use;
            pc.button(&label, cx + 14.0, y, cw - 28.0, 26.0,
                if active { ACT_BTN } else { NET_BTN }, BTN_HOVER,
                if active { ACCENT } else { TEXT_FG },
                AppAction::Radios(NetworkMessage::ConnectWifi(net.ssid.clone())));
            y += 30.0;
        }
    }

    // ── Bluetooth ──
    y += 4.0;
    pc.rect(SECTION_BORDER, cx + 8.0, y, cw - 16.0, 1.0);
    y += 8.0;
    pc.text("Bluetooth", cx + 12.0, y, 14.0, TEXT_FG);

    // BT toggle
    pc.button(if state.bt_enabled { "ON" } else { "OFF" },
        cx + cw - 140.0, y, 60.0, 28.0,
        if state.bt_enabled { TOGGLE_ON } else { TOGGLE_OFF }, BTN_HOVER, WHITE,
        AppAction::Radios(NetworkMessage::ToggleBluetooth));

    // Scan button
    pc.button("Scan", cx + cw - 72.0, y, 52.0, 28.0,
        TOGGLE_OFF, BTN_HOVER, WHITE,
        AppAction::Radios(NetworkMessage::BtScan));
    y += 34.0;

    if state.bt_devices.is_empty() {
        if state.bt_enabled {
            pc.text("No paired devices found", cx + 14.0, y, 12.0, TEXT_DIM);
        }
    } else {
        for dev in &state.bt_devices {
            let status = if dev.connected { ">" } else { " " };
            let label = format!("{} {} ({})", status, dev.name, dev.mac);
            let action_label = if dev.connected { "Disconnect" } else { "Connect" };
            pc.text(&label, cx + 14.0, y, 12.0, if dev.connected { ACCENT } else { TEXT_FG });
            pc.button(action_label, cx + cw - 90.0, y - 2.0, 70.0, 22.0,
                if dev.connected { TOGGLE_OFF } else { TOGGLE_ON }, BTN_HOVER, WHITE,
                if dev.connected {
                    AppAction::Radios(NetworkMessage::BtDisconnect(dev.mac.clone()))
                } else {
                    AppAction::Radios(NetworkMessage::BtConnect(dev.mac.clone()))
                });
            y += 24.0;
        }
    }

    pc
}

pub fn update(state: &mut NetworkState, msg: NetworkMessage) {
    match msg {
        NetworkMessage::Refreshed(new) => { *state = new; }
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
    }
}
