use crate::app::{AppAction, PageContent};
use crate::scroll_region::ScrollRegion;
use cce_ui::layout::{PageLayoutBuilder, LayoutStrategy, RenderTarget};
use cce_ui::widget::{Adapted, Toggle};

#[derive(Debug, Clone)]
pub struct WifiNetwork {
    pub ssid: String,
    pub signal: u8,
    pub secured: bool,
    pub in_use: bool,
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
    pub wifi_list: ScrollRegion,
    pub wifi_toggle: Adapted<Toggle>,
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
            wifi_list: ScrollRegion::new(26.0, 4.0),
            wifi_toggle: Toggle::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum NetworkMessage {
    Refreshed(NetworkState),
    ToggleWifi,
    ConnectWifi(String),
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

    NetworkState {
        loaded: true,
        wifi_enabled, connected_ssid, signal_strength: signal,
        ip_address, device, available,
        wifi_list: ScrollRegion::new(26.0, 4.0),
        wifi_toggle: Toggle::new(),
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

fn wifi_connect(ssid: &str) {
    let _ = tokio::process::Command::new("nmcli")
        .args(["dev", "wifi", "connect", ssid]).spawn();
}

fn wifi_toggle(enable: bool) {
    let _ = tokio::process::Command::new("nmcli")
        .args(["radio", "wifi", if enable { "on" } else { "off" }]).spawn();
}

const TEXT_FG: [f32; 4] = [0.83, 0.83, 0.83, 1.0];
const TEXT_DIM: [f32; 4] = [0.53, 0.53, 0.60, 1.0];
const ACCENT: [f32; 4] = [0.36, 0.56, 0.38, 1.0];
const BTN_HOVER: [f32; 4] = [0.25, 0.30, 0.26, 1.0];
const NET_BTN: [f32; 4] = [0.13, 0.20, 0.27, 1.0];
const ACT_BTN: [f32; 4] = [0.16, 0.29, 0.18, 1.0];

pub fn view(state: &mut NetworkState, cx: f32, cy: f32, cw: f32, ch: f32, root_focused: bool, layout: &mut dyn LayoutStrategy, ctx: &mut cce_ui::context::UiContext) -> PageContent {
    let mut final_pc = PageContent::new();
    let sec_w = 320.0f32;
    let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(1);

    // ── WiFi (label-less well) ──
    builder.add_section_spanned(&mut final_pc, "", 1, root_focused, |sec| {
        let sec_w = sec.cw;
        let rx = sec.left;
        let padding = sec.padding();
        let row_gap = cce_ui::layout::label_margin();
        let margin = padding.max(12.0);

        if !state.loaded {
            sec.text("Loading WiFi interfaces...", margin, 0.0, 12.0, TEXT_DIM);
        } else {
            let wifi_btn_w = if sec_w < 200.0 { 40.0 } else { 60.0 };

            state.wifi_toggle.set_toggled(state.wifi_enabled);
            state.wifi_toggle.set_label(if state.wifi_enabled { "ON" } else { "OFF" });
            // Hand-placed at its real width — sec.widget grid-places at full
            // column width (the old wide "ON" plate).
            let yt = sec.ay();
            let tx = sec.ax(margin);
            cce_ui::layout::render_widget(sec.pc, &mut state.wifi_toggle, tx, yt, wifi_btn_w, 28.0, ctx);
            sec.content_y = yt + 28.0 + row_gap;

            if state.wifi_enabled {
                // Flowing text rows — sec.text advances content_y itself.
                if !state.connected_ssid.is_empty() {
                    let ssid_max_chars = ((sec_w - 2.0 * margin) / 7.0) as usize;
                    let ssid_truncated = if state.connected_ssid.len() > ssid_max_chars {
                        format!("{}...", &state.connected_ssid[..ssid_max_chars.saturating_sub(3)])
                    } else {
                        state.connected_ssid.clone()
                    };
                    sec.text(&format!("Connected: {}", ssid_truncated), margin, 0.0, 13.0, ACCENT);

                    if sec_w < 220.0 {
                        sec.text(&format!("Signal: {}%", state.signal_strength), margin, 0.0, 12.0, TEXT_DIM);
                    } else {
                        sec.text(&format!("Signal: {}%  IP: {}", state.signal_strength, state.ip_address),
                            margin, 0.0, 12.0, TEXT_DIM);
                    }
                } else {
                    sec.text("Not connected", margin, 0.0, 12.0, TEXT_DIM);
                }
                sec.content_y += row_gap;
            }

            if state.wifi_enabled && !state.available.is_empty() {
                let list_box_x = rx + margin;
                let list_box_y = sec.ay();
                let list_box_w = sec_w - 2.0 * margin;
                let list_box_h = 160.0;

                // Dissolved List (Phase 6v): scroll state + frame prims are app-owned.
                state.wifi_list.set_rect(list_box_x, list_box_y, list_box_w, list_box_h);
                state.wifi_list.update_bounds(state.available.len(), list_box_y, list_box_h);
                state.wifi_list.push_prims(sec.pc);

                let btn_w = list_box_w - 2.0 * margin;
                let max_chars = ((btn_w / 6.5) as usize).saturating_sub(10).max(5);

                sec.pc.push_clip_rect(list_box_x, list_box_y, list_box_w, list_box_h);
                for (idx, net) in state.available.iter().enumerate() {
                    if let Some(draw_y) = state.wifi_list.get_item_draw_y(idx, 4.0) {
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
                            AppAction::Network(NetworkMessage::ConnectWifi(net.ssid.clone())));
                    }
                }
                sec.pc.pop_clip_rect();
                sec.content_y += list_box_h + row_gap;
            }
        }
    });

    final_pc
}

pub fn update(state: &mut NetworkState, msg: NetworkMessage) {
    match msg {
        NetworkMessage::Refreshed(new) => {
            state.loaded = new.loaded;
            state.wifi_enabled = new.wifi_enabled;
            state.connected_ssid = new.connected_ssid;
            state.signal_strength = new.signal_strength;
            state.ip_address = new.ip_address;
            state.device = new.device;
            state.available = new.available;
        }
        NetworkMessage::ToggleWifi => {
            state.wifi_enabled = !state.wifi_enabled;
            wifi_toggle(state.wifi_enabled);
        }
        NetworkMessage::ConnectWifi(ssid) => { wifi_connect(&ssid); }
    }
}

impl NetworkState {
    /// The wifi list is only laid out (and its rect refreshed) when this holds — gate the
    /// dissolved region's input on it so a stale rect can't eat events.
    fn wifi_list_visible(&self) -> bool {
        self.loaded && self.wifi_enabled && !self.available.is_empty()
    }
}

impl crate::pages::AppPage for NetworkState {
    // Sections: [WiFi]
    // Mirrors the view's load gate (d13a901): `wifi_toggle` is painted only in
    // the `else` of `if !state.loaded`, so reporting it while the page still
    // reads "Loading WiFi interfaces..." is a dead root.
    fn section_widgets(&mut self) -> Vec<Vec<cce_ui::widget::WidgetId>> {
        if !self.loaded {
            return vec![Vec::new()];
        }
        vec![vec![self.wifi_toggle.id()]]
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
        // Page root dissolved (6u): the ctrl-nav entry focuses section 0 now, which used to
        // be expressed as root focus here.
        let focused = root_focused || sec_focused.first().copied().unwrap_or(false);
        view(self, cx, cy, cw, ch, focused, layout, ctx)
    }

    fn propagate_widget_changes(&mut self, actions: &mut Vec<crate::app::AppAction>) {
        if self.wifi_toggle.take_change() {
            actions.push(crate::app::AppAction::Network(NetworkMessage::ToggleWifi));
        }
    }

    fn handle_pointer_move(
        &mut self,
        lx: f32,
        ly: f32,
        _actions: &mut Vec<crate::app::AppAction>,
        _ctx: &mut cce_ui::context::UiContext,
    ) -> bool {
        self.wifi_list_visible() && self.wifi_list.cursor_moved(lx, ly)
    }

    fn handle_pointer_down(&mut self, lx: f32, ly: f32, _ctx: &mut cce_ui::context::UiContext) -> bool {
        self.wifi_list_visible() && self.wifi_list.press(lx, ly)
    }

    fn handle_pointer_up(&mut self, _ctx: &mut cce_ui::context::UiContext) -> bool {
        self.wifi_list.release()
    }

    fn handle_mouse_wheel(&mut self, delta: &cce_ui::widget::MouseScrollDelta, lx: f32, ly: f32) -> bool {
        self.wifi_list_visible() && self.wifi_list.wheel(delta, lx, ly)
    }

    fn handle_key_input(&mut self, event: &cce_ui::widget::KeyEvent) -> bool {
        self.wifi_list_visible() && self.wifi_list.keyboard(event)
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

    #[test]
    fn section_widgets_mirror_load_gate() {
        use crate::pages::AppPage;
        let mut st = NetworkState::default();
        // Not loaded: the view paints only "Loading WiFi interfaces...", so
        // reporting the toggle would be a root nothing registered this frame.
        assert_eq!(st.section_widgets(), vec![Vec::new()]);
        st.loaded = true;
        assert_eq!(st.section_widgets()[0].len(), 1);
    }
}
