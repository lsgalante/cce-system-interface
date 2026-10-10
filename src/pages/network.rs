use crate::app::{AppAction, PageContent};
use cce_ui::context::UiContext;
use cce_ui::widget::Handle;
use cce_ui::widget::ScrollRegion;
use cce_ui::compose::{lay_row, Cell, PageLayoutBuilder, PageFlow};
use cce_ui::scene::paint::{RenderTarget};
use cce_ui::scene::layout::Rect;
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
    pub wifi_toggle: Handle<Adapted<Toggle>>,
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
            wifi_list: ScrollRegion::new(26.0, 4.0).with_sink_behind(true),
            wifi_toggle: Handle::none(),
        }
    }
}

impl NetworkState {
    /// The page's state, its widgets inserted into `ctx`.
    pub fn new(ctx: &mut UiContext) -> Self {
        Self { wifi_toggle: ctx.insert(Toggle::new()), ..Self::default() }
    }
}

#[derive(Debug, Clone)]
pub enum NetworkMessage {
    Refreshed(Box<NetworkState>),
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
        wifi_list: ScrollRegion::new(26.0, 4.0).with_sink_behind(true),
        wifi_toggle: Handle::none(),
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
            if ssid.is_empty() || ssid == "--" { continue; }
            let in_use = parts.len() > 3 && parts[3] == "*";
            if seen.contains(&ssid) {
                // One SSID, several access points (a mesh, 2.4 + 5 GHz):
                // the row is in use if ANY of them is. Keeping the first
                // line's flag lost the mark whenever nmcli listed an idle
                // AP of the network ahead of the connected one.
                if in_use {
                    if let Some(n) = networks.iter_mut().find(|n: &&mut WifiNetwork| n.ssid == ssid) {
                        n.in_use = true;
                    }
                }
                continue;
            }
            seen.insert(ssid.clone());
            networks.push(WifiNetwork {
                ssid, signal: parts[1].parse::<u8>().unwrap_or(0),
                secured: !parts[2].is_empty(),
                in_use,
            });
        }
    }
    networks.sort_by_key(|a| std::cmp::Reverse(a.signal));
    networks
}

fn wifi_connect(ssid: &str) {
    let mut cmd = tokio::process::Command::new("nmcli");
    cmd.args(["dev", "wifi", "connect", ssid]);
    let _ = crate::spawn_awaited(cmd);
}

fn wifi_toggle(enable: bool) {
    let mut cmd = tokio::process::Command::new("nmcli");
    cmd.args(["radio", "wifi", if enable { "on" } else { "off" }]);
    let _ = crate::spawn_awaited(cmd);
}

const TEXT_FG: [f32; 4] = [0.83, 0.83, 0.83, 1.0];
const TEXT_DIM: [f32; 4] = [0.53, 0.53, 0.60, 1.0];
/// The Wi-Fi list's own height: a window of rows that scrolls.
const WIFI_LIST_H: f32 = 160.0;
const ACCENT: [f32; 4] = [0.36, 0.56, 0.38, 1.0];
const BTN_HOVER: [f32; 4] = [0.25, 0.30, 0.26, 1.0];
const NET_BTN: [f32; 4] = [0.13, 0.20, 0.27, 1.0];
const ACT_BTN: [f32; 4] = [0.16, 0.29, 0.18, 1.0];

pub fn view(state: &mut NetworkState, cx: f32, cy: f32, cw: f32, ch: f32, root_focused: bool, layout: &mut PageFlow, ctx: &mut cce_ui::context::UiContext) -> PageContent {
    let mut final_pc = PageContent::new();
    let sec_w = 320.0f32;
    let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(1);

    // ── WiFi (label-less well) ──
    builder.add_section_spanned(&mut final_pc, "", 1, root_focused, |sec| {
        let sec_w = sec.cw;
        let mut form = sec.form();
        let mut col = form.column();

        if !state.loaded {
            col.text("Loading WiFi interfaces...", 12.0, TEXT_DIM);
        } else {
            let wifi_btn_w = if sec_w < 200.0 { 40.0 } else { 60.0 };
            ctx[state.wifi_toggle].set_toggled(state.wifi_enabled);
            ctx[state.wifi_toggle].set_label(if state.wifi_enabled { "ON" } else { "OFF" });
            // At its own width, not the row's: the old wide "ON" plate.
            col.row(|r| {
                r.widget_w_h(ctx, state.wifi_toggle, wifi_btn_w, cce_ui::layout::toggle_height());
            });

            if state.wifi_enabled {
                let text_w = col.form_width();
                col.block(|b| {
                    if state.connected_ssid.is_empty() {
                        b.text("Not connected", 12.0, TEXT_DIM);
                        return;
                    }
                    let ssid_max_chars = (text_w / 7.0) as usize;
                    let ssid_truncated = if state.connected_ssid.len() > ssid_max_chars {
                        format!("{}...", &state.connected_ssid[..ssid_max_chars.saturating_sub(3)])
                    } else {
                        state.connected_ssid.clone()
                    };
                    b.text(format!("Connected: {}", ssid_truncated), 13.0, ACCENT);
                    if sec_w < 220.0 {
                        b.text(format!("Signal: {}%", state.signal_strength), 12.0, TEXT_DIM);
                    } else {
                        b.text(format!("Signal: {}%  IP: {}", state.signal_strength, state.ip_address), 12.0, TEXT_DIM);
                    }
                });
            }

            if state.wifi_enabled && !state.available.is_empty() {
                let list = &mut state.wifi_list;
                let available = &state.available;
                col.draw(0.0, WIFI_LIST_H, false, move |pc, r, _| {
                    // Dissolved List (Phase 6v): scroll state + frame prims are app-owned.
                    list.set_rect(r.x, r.y, r.width, r.height);
                    list.update_bounds(available.len(), r.y, r.height);
                    list.push_prims(pc);

                    let row_h = list.item_height;
                    let btn_w = r.width - 2.0 * cce_ui::layout::list_gap();
                    let max_chars = ((btn_w / 6.5) as usize).saturating_sub(10).max(5);

                    pc.push_clip_rect(r.x, r.y, r.width, r.height);
                    for (idx, net) in available.iter().enumerate() {
                        if let Some(draw_y) = list.get_item_draw_y(idx, 4.0) {
                            let ssid_truncated = if net.ssid.len() > max_chars {
                                format!("{}...", &net.ssid[..max_chars.saturating_sub(3)])
                            } else {
                                net.ssid.clone()
                            };
                            let label = format!("{}  ({}%)", ssid_truncated, net.signal);
                            let active = net.in_use;
                            let cell = lay_row(Rect { x: r.x, y: draw_y, width: r.width, height: row_h }, &[Cell::grow(row_h)])[0];
                            let row_x = cell.x;
                            pc.button(&label, row_x, draw_y, btn_w, row_h,
                                if active { ACT_BTN } else { NET_BTN }, BTN_HOVER,
                                if active { ACCENT } else { TEXT_FG },
                                AppAction::Network(NetworkMessage::ConnectWifi(net.ssid.clone())));
                            // The network in use wears a `check` glyph at the
                            // row's left (it was a ">" in the label), in the
                            // plain text colour: ACCENT is the row's own green
                            // and a glyph in it all but vanishes there.
                            if active {
                                let g = 12.0;
                                pc.icon("check", row_x + cce_ui::layout::CONTROL_TEXT_INSET,
                                    draw_y + (row_h - g) / 2.0, g, g, TEXT_FG);
                            }
                        }
                    }
                    pc.pop_clip_rect();
                    // The scrollbar's fore copy, over the rows at the raise's fade.
                    list.push_scrollbar_fore(pc);
                });
            }
        }
        sec.place(form, ctx);
    });

    final_pc
}

pub fn update(state: &mut NetworkState, msg: NetworkMessage, _ctx: &mut UiContext) {
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
        layout: &mut PageFlow,
        ctx: &mut cce_ui::context::UiContext,
    ) -> crate::app::PageContent {
        // Page root dissolved (6u): the ctrl-nav entry focuses section 0 now, which used to
        // be expressed as root focus here.
        let focused = root_focused || sec_focused.first().copied().unwrap_or(false);
        view(self, cx, cy, cw, ch, focused, layout, ctx)
    }

    fn propagate_widget_changes(&mut self, actions: &mut Vec<crate::app::AppAction>, ctx: &mut UiContext) {
        if ctx[self.wifi_toggle].take_change() {
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

    fn tick(&mut self, dt: f32) -> bool {
        self.wifi_list.tick(dt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_layout_grid() {
        let mut ui = cce_ui::context::UiContext::new();
        let mut state = NetworkState::new(&mut ui);
        state.loaded = true;
        state.wifi_enabled = true;
        let mut layout = cce_ui::compose::PageFlow::new();
        let pc = view(&mut state, 10.0, 20.0, 800.0, 600.0, false, &mut layout, &mut ui);
        assert!(!pc.rects.is_empty() || !pc.texts.is_empty() || !pc.buttons.is_empty());
    }

    #[test]
    fn test_view_layout_connected() {
        let mut ui = cce_ui::context::UiContext::new();
        let mut state = NetworkState::new(&mut ui);
        state.loaded = true;
        state.wifi_enabled = true;
        state.connected_ssid = "MyHomeWiFi".to_string();
        state.signal_strength = 80;
        state.ip_address = "192.168.1.50".to_string();
        let mut layout = cce_ui::compose::PageFlow::new();
        let pc = view(&mut state, 10.0, 20.0, 800.0, 600.0, false, &mut layout, &mut ui);
        assert!(!pc.rects.is_empty() || !pc.texts.is_empty() || !pc.buttons.is_empty());
    }

    #[test]
    fn section_widgets_mirror_load_gate() {
        let mut ui = cce_ui::context::UiContext::new();
        use crate::pages::AppPage;
        let mut st = NetworkState::new(&mut ui);
        // Not loaded: the view paints only "Loading WiFi interfaces...", so
        // reporting the toggle would be a root nothing registered this frame.
        assert_eq!(st.section_widgets(), vec![Vec::new()]);
        st.loaded = true;
        assert_eq!(st.section_widgets()[0].len(), 1);
    }
}
