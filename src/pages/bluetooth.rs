use crate::app::{AppAction, PageContent, SectionContextExt};
use cce_ui::layout::{render_widget, PageLayoutBuilder, LayoutStrategy};
use cce_ui::widget::{Adapted, Toggle};

#[derive(Debug, Clone)]
pub struct BluetoothDevice {
    pub mac: String,
    pub name: String,
    pub icon: String,
    pub connected: bool,
}

#[derive(Debug, Clone)]
pub struct BluetoothState {
    pub loaded: bool,
    pub installed: bool,
    pub service_active: bool,
    pub enabled: bool,
    pub devices: Vec<BluetoothDevice>,
    pub scanning: bool,
    pub toggle: Adapted<Toggle>,
}

impl Default for BluetoothState {
    fn default() -> Self {
        Self {
            loaded: false,
            installed: false,
            service_active: false,
            enabled: false,
            devices: Vec::new(),
            scanning: false,
            toggle: Toggle::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum BluetoothMessage {
    Refreshed(BluetoothState),
    Toggle,
    Connect(String),
    Disconnect(String),
    Scan,
    InstallTools,
    StartService,
}

const TEXT_FG: [f32; 4] = [0.83, 0.83, 0.83, 1.0];
const TEXT_DIM: [f32; 4] = [0.53, 0.53, 0.60, 1.0];
const ACCENT: [f32; 4] = [0.35, 0.65, 0.90, 1.0];
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const TOGGLE_ON: [f32; 4] = [0.13, 0.18, 0.14, 1.0];
const TOGGLE_OFF: [f32; 4] = [0.15, 0.15, 0.20, 1.0];
const BTN_HOVER: [f32; 4] = [0.25, 0.30, 0.26, 1.0];

pub async fn fetch_bluetooth_page_state() -> BluetoothState {
    let installed = tokio::process::Command::new("bluetoothctl")
        .arg("--version")
        .output()
        .await
        .is_ok();
    if !installed {
        return BluetoothState { loaded: true, ..Default::default() };
    }

    let service_active = tokio::process::Command::new("systemctl")
        .args(["is-active", "bluetooth"])
        .output()
        .await
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "active")
        .unwrap_or(false);
    if !service_active {
        return BluetoothState { loaded: true, installed, ..Default::default() };
    }

    let enabled = tokio::process::Command::new("bluetoothctl")
        .args(["show"]).output().await.ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).lines().any(|l| l.contains("Powered: yes")))
        .unwrap_or(false);

    let devices = if enabled { fetch_devices().await } else { Vec::new() };
    BluetoothState { loaded: true, installed, service_active, enabled, devices, scanning: false, toggle: Toggle::new() }
}

async fn fetch_devices() -> Vec<BluetoothDevice> {
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

fn bt_toggle(enable: bool) {
    let _ = tokio::process::Command::new("bluetoothctl")
        .args(["power", if enable { "on" } else { "off" }])
        .spawn();
}

fn bt_connect(mac: &str) {
    let _ = tokio::process::Command::new("bluetoothctl")
        .args(["connect", mac])
        .spawn();
}

fn bt_disconnect(mac: &str) {
    let _ = tokio::process::Command::new("bluetoothctl")
        .args(["disconnect", mac])
        .spawn();
}

fn bt_scan() {
    let _ = tokio::process::Command::new("bluetoothctl")
        .args(["scan", "on"])
        .spawn();
    let _ = tokio::process::Command::new("sh")
        .args(["-c", "sleep 5 && bluetoothctl scan off"])
        .spawn();
}

pub fn view(state: &mut BluetoothState, cx: f32, cy: f32, cw: f32, ch: f32, sec_focused: &[bool], layout: &mut dyn LayoutStrategy, ctx: &mut cce_ui::context::UiContext) -> PageContent {
    let mut final_pc = PageContent::new();
    let sec_w = 320.0f32;
    let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(1);

    builder.add_section_spanned(&mut final_pc, "", 1, sec_focused.first().copied().unwrap_or(false), |sec| {
        let bt_sec_w = sec.cw;
        let padding = sec.padding();
        let row_gap = cce_ui::layout::label_margin();
        let margin = padding.max(12.0);
        let font_size = 12.0;
        let btn_h = 28.0;

        if !state.loaded {
            sec.text("Loading Bluetooth status...", margin, 0.0, font_size, TEXT_DIM);
        } else if !state.installed {
            sec.text("Bluetooth tools (bluez) not installed", margin, 0.0, font_size, TEXT_DIM);
            let btn_w = if bt_sec_w < 200.0 { 100.0 } else { 120.0 };
            let yt = sec.ay();
            sec.button("Install Tools", sec.ax(margin), yt, btn_w, btn_h,
                TOGGLE_ON, BTN_HOVER, WHITE,
                AppAction::Bluetooth(BluetoothMessage::InstallTools));
            sec.content_y = yt + btn_h + row_gap;
        } else if !state.service_active {
            sec.text("Bluetooth service is stopped", margin, 0.0, font_size, TEXT_DIM);
            let btn_w = if bt_sec_w < 200.0 { 100.0 } else { 120.0 };
            let yt = sec.ay();
            sec.button("Start Service", sec.ax(margin), yt, btn_w, btn_h,
                TOGGLE_ON, BTN_HOVER, WHITE,
                AppAction::Bluetooth(BluetoothMessage::StartService));
            sec.content_y = yt + btn_h + row_gap;
        } else {
            let yt = sec.ay();
            let bt_btn_w = if bt_sec_w < 200.0 { 40.0 } else { 60.0 };
            let scan_btn_w = if bt_sec_w < 200.0 { 40.0 } else { 52.0 };
            let scan_btn_x = margin + bt_btn_w + row_gap;

            state.toggle.set_toggled(state.enabled);
            state.toggle.set_label(if state.enabled { "ON" } else { "OFF" });
            // Hand-placed: sec.widget grid-places at full column width, which
            // would sit the toggle under the Scan button.
            let tx = sec.ax(margin);
            render_widget(sec.pc, &mut state.toggle, tx, yt, bt_btn_w, btn_h, ctx);
            sec.button("Scan", sec.ax(scan_btn_x), yt, scan_btn_w, btn_h,
                TOGGLE_OFF, BTN_HOVER, WHITE,
                AppAction::Bluetooth(BluetoothMessage::Scan));
            sec.content_y = yt + btn_h + row_gap;

            if state.devices.is_empty() {
                if state.enabled {
                    let no_devices_msg = if bt_sec_w < 200.0 { "No paired devices" } else { "No paired devices found" };
                    sec.text(no_devices_msg, margin, 0.0, font_size, TEXT_DIM);
                }
            } else {
                let item_h = 22.0;
                for dev in &state.devices {
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
                            AppAction::Bluetooth(BluetoothMessage::Disconnect(dev.mac.clone()))
                        } else {
                            AppAction::Bluetooth(BluetoothMessage::Connect(dev.mac.clone()))
                        });
                    sec.text(&label, text_x, text_y_offset, font_size, if dev.connected { ACCENT } else { TEXT_FG });
                    sec.content_y = yt + item_h + row_gap;
                }
            }
        }
    });

    final_pc
}

pub fn update(state: &mut BluetoothState, msg: BluetoothMessage) {
    match msg {
        BluetoothMessage::Refreshed(new) => {
            state.loaded = new.loaded;
            state.installed = new.installed;
            state.service_active = new.service_active;
            state.enabled = new.enabled;
            state.devices = new.devices;
            state.scanning = new.scanning;
        }
        BluetoothMessage::Toggle => {
            state.enabled = !state.enabled;
            bt_toggle(state.enabled);
        }
        BluetoothMessage::Connect(mac) => { bt_connect(&mac); }
        BluetoothMessage::Disconnect(mac) => { bt_disconnect(&mac); }
        BluetoothMessage::Scan => { bt_scan(); }
        BluetoothMessage::InstallTools => {
            let _ = tokio::process::Command::new("pkexec")
                .args(["sh", "-c", "pacman -S --noconfirm bluez bluez-utils && systemctl enable --now bluetooth"])
                .spawn();
        }
        BluetoothMessage::StartService => {
            let _ = tokio::process::Command::new("pkexec")
                .args(["systemctl", "enable", "--now", "bluetooth"])
                .spawn();
        }
    }
}

impl crate::pages::AppPage for BluetoothState {
    // Sections: [Bluetooth]
    fn section_widgets(&mut self) -> Vec<Vec<cce_ui::widget::WidgetId>> {
        vec![vec![self.toggle.id()]]
    }

    fn view(
        &mut self,
        cx: f32,
        cy: f32,
        cw: f32,
        ch: f32,
        _root_focused: bool,
        sec_focused: &[bool],
        layout: &mut dyn LayoutStrategy,
        ctx: &mut cce_ui::context::UiContext,
    ) -> crate::app::PageContent {
        view(self, cx, cy, cw, ch, sec_focused, layout, ctx)
    }

    fn propagate_widget_changes(&mut self, actions: &mut Vec<crate::app::AppAction>) {
        if self.toggle.take_change() {
            actions.push(crate::app::AppAction::Bluetooth(BluetoothMessage::Toggle));
        }
    }
}
