use crate::app::{form_button, AppAction, PageContent};
use cce_ui::context::UiContext;
use cce_ui::widget::Handle;
use cce_ui::layout::{PageLayoutBuilder, PageFlow, RenderTarget};
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
    pub toggle: Handle<Adapted<Toggle>>,
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
            toggle: Handle::none(),
        }
    }
}

impl BluetoothState {
    /// The page's state, its widgets inserted into `ctx`.
    pub fn new(ctx: &mut UiContext) -> Self {
        Self { toggle: ctx.insert(Toggle::new()), ..Self::default() }
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
    BluetoothState { loaded: true, installed, service_active, enabled, devices, scanning: false, toggle: Handle::none() }
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
    let mut cmd = tokio::process::Command::new("bluetoothctl");
    cmd.args(["power", if enable { "on" } else { "off" }]);
    let _ = crate::spawn_awaited(cmd);
}

fn bt_connect(mac: &str) {
    let mut cmd = tokio::process::Command::new("bluetoothctl");
    cmd.args(["connect", mac]);
    let _ = crate::spawn_awaited(cmd);
}

fn bt_disconnect(mac: &str) {
    let mut cmd = tokio::process::Command::new("bluetoothctl");
    cmd.args(["disconnect", mac]);
    let _ = crate::spawn_awaited(cmd);
}

fn bt_scan() {
    let mut cmd = tokio::process::Command::new("bluetoothctl");
    cmd.args(["scan", "on"]);
    let _ = crate::spawn_awaited(cmd);
    let mut cmd = tokio::process::Command::new("sh");
    cmd.args(["-c", "sleep 5 && bluetoothctl scan off"]);
    let _ = crate::spawn_awaited(cmd);
}

pub fn view(state: &mut BluetoothState, cx: f32, cy: f32, cw: f32, ch: f32, sec_focused: &[bool], layout: &mut PageFlow, ctx: &mut cce_ui::context::UiContext) -> PageContent {
    let mut final_pc = PageContent::new();
    let sec_w = 320.0f32;
    let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(1);

    builder.add_section_spanned(&mut final_pc, "", 1, sec_focused.first().copied().unwrap_or(false), |sec| {
        let bt_sec_w = sec.cw;
        let font_size = 12.0;
        let mut form = sec.form();
        let mut col = form.column();

        if !state.loaded {
            col.text("Loading Bluetooth status...", font_size, TEXT_DIM);
        } else if !state.installed || !state.service_active {
            let (note, label, msg) = if !state.installed {
                ("Bluetooth tools (bluez) not installed", "Install Tools", BluetoothMessage::InstallTools)
            } else {
                ("Bluetooth service is stopped", "Start Service", BluetoothMessage::StartService)
            };
            col.text(note, font_size, TEXT_DIM);
            let btn_w = if bt_sec_w < 200.0 { 100.0 } else { 120.0 };
            col.row(|r| form_button(r, label, btn_w, (TOGGLE_ON, BTN_HOVER, WHITE), AppAction::Bluetooth(msg)));
        } else {
            let bt_btn_w = if bt_sec_w < 200.0 { 40.0 } else { 60.0 };
            let scan_btn_w = if bt_sec_w < 200.0 { 40.0 } else { 52.0 };
            ctx[state.toggle].set_toggled(state.enabled);
            ctx[state.toggle].set_label(if state.enabled { "ON" } else { "OFF" });
            col.row(|r| {
                r.widget_w_h(ctx, state.toggle, bt_btn_w, cce_ui::layout::toggle_height());
                form_button(r, "Scan", scan_btn_w, (TOGGLE_OFF, BTN_HOVER, WHITE), AppAction::Bluetooth(BluetoothMessage::Scan));
            });

            if state.devices.is_empty() {
                if state.enabled {
                    let no_devices_msg = if bt_sec_w < 200.0 { "No paired devices" } else { "No paired devices found" };
                    col.text(no_devices_msg, font_size, TEXT_DIM);
                }
            } else {
                for dev in &state.devices {
                    let btn_w = if bt_sec_w < 250.0 { 42.0 } else { 70.0 };
                    let action_label = match (dev.connected, bt_sec_w < 250.0) {
                        (true, true) => "Disc",
                        (true, false) => "Disconnect",
                        (false, true) => "Conn",
                        (false, false) => "Connect",
                    };
                    let is_unknown = dev.name.replace('-', ":").eq_ignore_ascii_case(&dev.mac);
                    let label = match (is_unknown, bt_sec_w < 350.0) {
                        (true, true) => dev.mac.clone(),
                        (true, false) => format!("Unknown Device ({})", dev.mac),
                        (false, true) => dev.name.clone(),
                        (false, false) => format!("{} ({})", dev.name, dev.mac),
                    };
                    let (face, action) = if dev.connected {
                        (TOGGLE_OFF, AppAction::Bluetooth(BluetoothMessage::Disconnect(dev.mac.clone())))
                    } else {
                        (TOGGLE_ON, AppAction::Bluetooth(BluetoothMessage::Connect(dev.mac.clone())))
                    };
                    let connected = dev.connected;
                    col.row(|r| {
                        form_button(r, action_label, btn_w, (face, BTN_HOVER, WHITE), action);
                        // A connected device wears a `check` glyph ahead of its name (it was
                        // a ">" in the label); every row keeps the glyph's room so the names
                        // line up either way. The name is cut where the row ends.
                        let line_h = cce_ui::layout::form_line_height(font_size);
                        r.draw(0.0, line_h, true, move |pc, cell, _| {
                            let g = font_size;
                            if connected {
                                pc.icon("check", cell.x, cell.y + (line_h - g) / 2.0, g, g, ACCENT);
                            }
                            let tx = cell.x + g + cce_ui::layout::CONTROL_TEXT_INSET;
                            let color = if connected { ACCENT } else { TEXT_FG };
                            pc.text_with_bounds(&label, tx, cell.y, font_size, color,
                                Some([tx, cell.y - font_size, cell.x + cell.width, cell.y + 2.0 * font_size]));
                        });
                    });
                }
            }
        }
        sec.place(form, ctx);
    });

    final_pc
}

pub fn update(state: &mut BluetoothState, msg: BluetoothMessage, _ctx: &mut UiContext) {
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
            let mut cmd = tokio::process::Command::new("pkexec");
            cmd.args(["sh", "-c", "pacman -S --noconfirm bluez bluez-utils && systemctl enable --now bluetooth"]);
            let _ = crate::spawn_awaited(cmd);
        }
        BluetoothMessage::StartService => {
            let mut cmd = tokio::process::Command::new("pkexec");
            cmd.args(["systemctl", "enable", "--now", "bluetooth"]);
            let _ = crate::spawn_awaited(cmd);
        }
    }
}

impl crate::pages::AppPage for BluetoothState {
    // Sections: [Bluetooth]
    // The gate mirrors view()'s branch chain exactly (the d13a901 lesson):
    // `toggle` is painted only in the innermost `else`, so reporting it from any
    // earlier branch is a dead root. Unlike a load gate this one does not close
    // on its own — `!installed` and `!service_active` are steady states, so on a
    // host without bluez the page's only ctrl-nav target stays dead for the life
    // of the process and every pointer move over the page drops an event.
    fn section_widgets(&mut self) -> Vec<Vec<cce_ui::widget::WidgetId>> {
        if !self.loaded || !self.installed || !self.service_active {
            return vec![Vec::new()];
        }
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
        layout: &mut PageFlow,
        ctx: &mut cce_ui::context::UiContext,
    ) -> crate::app::PageContent {
        view(self, cx, cy, cw, ch, sec_focused, layout, ctx)
    }

    fn propagate_widget_changes(&mut self, actions: &mut Vec<crate::app::AppAction>, ctx: &mut UiContext) {
        if ctx[self.toggle].take_change() {
            actions.push(crate::app::AppAction::Bluetooth(BluetoothMessage::Toggle));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pages::AppPage;

    #[test]
    fn section_widgets_mirror_branch_chain() {
        let mut ui = cce_ui::context::UiContext::new();
        let mut st = BluetoothState::new(&mut ui);
        // Each of the three early branches paints a message (and maybe a plain
        // PageContent button) but never `toggle` — the widget lives only in the
        // innermost `else`. Unlike a load gate, the middle two are steady
        // states: a host without bluez sits in one of them forever.
        assert_eq!(st.section_widgets(), vec![Vec::new()], "not loaded");
        st.loaded = true;
        assert_eq!(st.section_widgets(), vec![Vec::new()], "bluez absent");
        st.installed = true;
        assert_eq!(st.section_widgets(), vec![Vec::new()], "service stopped");
        st.service_active = true;
        assert_eq!(st.section_widgets()[0].len(), 1, "toggle painted");
    }
}
