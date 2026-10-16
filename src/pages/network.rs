//! Network: Wi-Fi through NetworkManager (`nmcli`), plus the other
//! connections it has up (VPNs, wired, tunnels).
//!
//! A row click only selects a network: connecting waits on the Connect
//! button under the list, which names the network and says whether it is
//! open. Until 2026-10-16 a click connected at once, and the list re-sorted
//! by signal every refresh, so a click just after one could land on
//! whichever network had moved under the pointer. The list now also keeps
//! its order while the pointer is over it.
//!
//! The password goes to nmcli on its stdin (`--ask`), never in argv, where
//! every process on the machine could read it from /proc. Nothing in the DE
//! answers NetworkManager's secret requests, so a secured network that has
//! never been joined needs it typed here.

use std::process::Stdio;

use crate::app::{button_need, form_button, form_divider, form_pairs, wrap_to_width, AppAction, PageCanvas};
use cce_ui::compose::{lay_row, Cell, PageFlow, PageLayoutBuilder};
use cce_ui::context::UiContext;
use cce_ui::scene::layout::Rect;
use cce_ui::widget::input::TextBox;
use cce_ui::widget::ScrollRegion;
use cce_ui::widget::{Adapted, Handle, Toggle};

/// One network in range, its access points folded into one row.
#[derive(Debug, Clone, PartialEq)]
pub struct WifiNetwork {
    pub ssid: String,
    /// The strongest of its access points.
    pub signal: u8,
    /// nmcli's SECURITY column: "" for an open network, else e.g. "WPA2",
    /// "WPA2 802.1X", "WPA3".
    pub security: String,
    pub in_use: bool,
}

impl WifiNetwork {
    pub fn is_open(&self) -> bool {
        self.security.is_empty() || self.security == "--"
    }

    /// WPA-Enterprise: needs an identity and certificates `dev wifi connect`
    /// cannot take.
    pub fn is_enterprise(&self) -> bool {
        self.security.contains("802.1X")
    }
}

/// An active connection that is not Wi-Fi or loopback.
#[derive(Debug, Clone, PartialEq)]
pub struct OtherConnection {
    pub name: String,
    pub kind: String,
}

/// What the page worker reads, every 5 s while the page is open.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NetworkSnapshot {
    /// nmcli answered and NetworkManager is running.
    pub nm_running: bool,
    /// Not blocked by a hardware switch or airplane-mode key.
    pub wifi_hw_enabled: bool,
    pub wifi_enabled: bool,
    /// The Wi-Fi device ("wlp0s20f3"), connected or not.
    pub device: String,
    /// The SSID of the access point in use — not the connection profile's
    /// name, which can differ ("Home 1", or renamed).
    pub connected_ssid: String,
    pub signal_strength: u8,
    pub ip_address: String,
    pub available: Vec<WifiNetwork>,
    pub others: Vec<OtherConnection>,
}

/// An nmcli command in flight.
#[derive(Debug, Clone, PartialEq)]
pub enum Busy {
    Connecting(String),
    Disconnecting,
    /// Turning the radio on (true) or off.
    Radio(bool),
    Scanning,
}

pub struct NetworkState {
    pub loaded: bool,
    pub snap: NetworkSnapshot,
    pub wifi_list: ScrollRegion,
    pub wifi_toggle: Handle<Adapted<Toggle>>,
    pub password_box: Handle<Adapted<TextBox>>,
    /// The network whose Connect / Disconnect panel is open.
    pub selected: Option<String>,
    pub busy: Option<Busy>,
    /// The last failed action, shown until dismissed or the next one.
    pub error: Option<String>,
    /// How results come back from the spawned commands (set by the host).
    pub sender: Option<calloop::channel::Sender<AppAction>>,
}

impl Default for NetworkState {
    fn default() -> Self {
        Self {
            loaded: false,
            snap: NetworkSnapshot::default(),
            // Frameless on the section's well, the DE's list: no grey slab
            // behind the rows. 0 takes the toolkit's row height for the
            // list font.
            wifi_list: ScrollRegion::new(0.0, LIST_GAP).with_frame(false).with_sink_behind(true),
            wifi_toggle: Handle::none(),
            password_box: Handle::none(),
            selected: None,
            busy: None,
            error: None,
            sender: None,
        }
    }
}

impl NetworkState {
    /// The page's state, its widgets inserted into `ctx`.
    pub fn new(ctx: &mut UiContext) -> Self {
        let mut toggle = Toggle::new();
        // It reads ON / OFF; this is what it switches.
        toggle.set_accessible_name(Some("Wi-Fi"));
        let password = TextBox::new(String::new())
            .with_multiline(false)
            .with_draw_bg_border(true)
            .with_password(true)
            .with_label("Password");
        Self { wifi_toggle: ctx.insert(toggle), password_box: ctx.insert(password), ..Self::default() }
    }

    fn selected_network(&self) -> Option<&WifiNetwork> {
        let ssid = self.selected.as_deref()?;
        self.snap.available.iter().find(|n| n.ssid == ssid)
    }

    /// The password box is laid out: a secured, non-enterprise network not
    /// in use is selected and no connect is running.
    fn password_visible(&self) -> bool {
        self.wifi_list_visible()
            && !matches!(self.busy, Some(Busy::Connecting(_)))
            && self.selected_network().is_some_and(|n| !n.in_use && !n.is_open() && !n.is_enterprise())
    }

    /// The wifi list is only laid out (and its rect refreshed) when this holds — gate the
    /// dissolved region's input on it so a stale rect can't eat events.
    fn wifi_list_visible(&self) -> bool {
        self.loaded && self.snap.nm_running && self.snap.wifi_enabled && !self.snap.available.is_empty()
    }
}

#[derive(Debug, Clone)]
pub enum NetworkMessage {
    Refreshed(Box<NetworkSnapshot>),
    /// The switch was flipped.
    ToggleWifi,
    /// A row was clicked: open its panel. Connects nothing.
    Select(String),
    CancelSelect,
    /// Connect to the selected network, with the password box's text.
    Connect,
    Disconnect,
    Scan,
    Finished(Busy, Result<(), String>),
    DismissError,
}

// ── nmcli ──

/// One `nmcli -t` line's fields. Terse output separates fields with `:` and
/// escapes a `:` or `\` inside a value with a backslash, so an SSID holding
/// a colon must not be split on it.
pub fn split_terse(line: &str) -> Vec<String> {
    let mut fields = vec![String::new()];
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                if let Some(next) = chars.next() {
                    fields.last_mut().unwrap().push(next);
                }
            }
            ':' => fields.push(String::new()),
            c => fields.last_mut().unwrap().push(c),
        }
    }
    fields
}

/// `nmcli -t -f IN-USE,SSID,SIGNAL,SECURITY dev wifi list`: one row per SSID
/// (in use if any of its access points is, at its strongest signal), the
/// network in use first and the rest by signal; and the SSID and signal of
/// the access point in use. Hidden networks (no SSID) are left out.
pub fn parse_wifi_list(out: &str) -> (Vec<WifiNetwork>, Option<(String, u8)>) {
    let mut networks: Vec<WifiNetwork> = Vec::new();
    let mut connected = None;
    for line in out.lines() {
        let f = split_terse(line);
        if f.len() < 4 || f[1].is_empty() {
            continue;
        }
        let (in_use, ssid, security) = (f[0].trim() == "*", f[1].clone(), f[3].clone());
        let signal = f[2].trim().parse::<u8>().unwrap_or(0);
        if in_use {
            connected = Some((ssid.clone(), signal));
        }
        match networks.iter_mut().find(|n| n.ssid == ssid) {
            Some(n) => {
                n.in_use |= in_use;
                n.signal = n.signal.max(signal);
            }
            None => networks.push(WifiNetwork { ssid, signal, security, in_use }),
        }
    }
    networks.sort_by(|a, b| b.in_use.cmp(&a.in_use).then(b.signal.cmp(&a.signal)).then(a.ssid.cmp(&b.ssid)));
    (networks, connected)
}

/// `nmcli -t -f NAME,TYPE con show --active`, less Wi-Fi and loopback.
pub fn parse_other_connections(out: &str) -> Vec<OtherConnection> {
    out.lines()
        .map(split_terse)
        .filter(|f| f.len() >= 2 && f[1] != "802-11-wireless" && f[1] != "loopback")
        .map(|f| OtherConnection { name: f[0].clone(), kind: connection_kind(&f[1]) })
        .collect()
}

fn connection_kind(t: &str) -> String {
    match t {
        "802-3-ethernet" | "ethernet" => "Wired",
        "vpn" => "VPN",
        "wireguard" => "WireGuard",
        "tun" => "Tunnel",
        "bridge" => "Bridge",
        "bluetooth" => "Bluetooth",
        "gsm" | "cdma" => "Mobile broadband",
        other => other,
    }
    .to_string()
}

/// The first `IP4.ADDRESS[n]` of `nmcli -t -f IP4.ADDRESS dev show`, without
/// its prefix length.
fn parse_ip(out: &str) -> String {
    out.lines()
        .map(split_terse)
        .find(|f| f.len() >= 2 && !f[1].is_empty())
        .map(|f| f[1].split('/').next().unwrap_or_default().to_string())
        .unwrap_or_default()
}

async fn nmcli_stdout(args: &[&str]) -> Option<String> {
    let out = crate::output_within(tokio::process::Command::new("nmcli").args(args), crate::FETCH_TIMEOUT).await.ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

pub async fn fetch_network_state() -> NetworkSnapshot {
    let running = nmcli_stdout(&["-t", "-f", "RUNNING", "general"]).await;
    if running.as_deref().map(str::trim) != Some("running") {
        return NetworkSnapshot::default();
    }
    let radio = nmcli_stdout(&["-t", "-f", "WIFI-HW,WIFI", "radio", "all"]).await.unwrap_or_default();
    let radio = split_terse(radio.trim());
    let wifi_hw_enabled = radio.first().map(String::as_str) != Some("disabled");
    let wifi_enabled = radio.get(1).map(String::as_str) == Some("enabled");

    let devices = nmcli_stdout(&["-t", "-f", "DEVICE,TYPE", "dev"]).await.unwrap_or_default();
    let device = devices
        .lines()
        .map(split_terse)
        .find(|f| f.len() >= 2 && f[1] == "wifi")
        .map(|f| f[0].clone())
        .unwrap_or_default();

    let active = nmcli_stdout(&["-t", "-f", "NAME,TYPE", "con", "show", "--active"]).await.unwrap_or_default();
    let others = parse_other_connections(&active);

    let (available, connected) = if wifi_enabled {
        // `--rescan no`: the list NetworkManager already has. Its default
        // (auto) made the card scan every 30 s while this page was open,
        // which costs a connected card latency; Scan asks for one.
        let mut args = vec!["-t", "-f", "IN-USE,SSID,SIGNAL,SECURITY", "dev", "wifi", "list", "--rescan", "no"];
        if !device.is_empty() {
            args.extend(["ifname", device.as_str()]);
        }
        parse_wifi_list(&nmcli_stdout(&args).await.unwrap_or_default())
    } else {
        (Vec::new(), None)
    };
    let (connected_ssid, signal_strength) = connected.unwrap_or_default();

    let ip_address = if connected_ssid.is_empty() || device.is_empty() {
        String::new()
    } else {
        parse_ip(&nmcli_stdout(&["-t", "-f", "IP4.ADDRESS", "dev", "show", &device]).await.unwrap_or_default())
    };

    NetworkSnapshot {
        nm_running: true,
        wifi_hw_enabled,
        wifi_enabled,
        device,
        connected_ssid,
        signal_strength,
        ip_address,
        available,
        others,
    }
}

/// Run nmcli to completion: its failure in its own words, less the "Error: ".
async fn nmcli_run(args: Vec<String>) -> Result<(), String> {
    let out = tokio::process::Command::new("nmcli")
        .args(&args)
        .stdin(Stdio::null())
        .output()
        .await
        .map_err(|e| format!("Could not run nmcli: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(clean_error(&String::from_utf8_lossy(&out.stderr)))
    }
}

/// `nmcli --ask dev wifi connect`, the password (if any) on its stdin: nmcli
/// asks for one up front for a new secured network, and again if a saved one
/// is refused; with nothing there it gets end-of-file and fails instead of
/// waiting. `-w 45` bounds the activation.
async fn wifi_connect(ssid: String, device: String, password: Option<String>) -> Result<(), String> {
    use tokio::io::AsyncWriteExt;
    let mut cmd = tokio::process::Command::new("nmcli");
    cmd.args(["--ask", "-w", "45", "dev", "wifi", "connect", &ssid]);
    if !device.is_empty() {
        cmd.args(["ifname", &device]);
    }
    cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| format!("Could not run nmcli: {e}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        if let Some(pw) = &password {
            let _ = stdin.write_all(format!("{pw}\n").as_bytes()).await;
        }
        // Dropped: end-of-file for any further prompt.
    }
    let out = child.wait_with_output().await.map_err(|e| format!("nmcli failed: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(explain_connect_error(&String::from_utf8_lossy(&out.stderr), &ssid, password.is_some()))
    }
}

fn clean_error(stderr: &str) -> String {
    let text = stderr.trim();
    let text = text.strip_prefix("Error: ").unwrap_or(text);
    if text.is_empty() { "nmcli failed without saying why.".to_string() } else { text.to_string() }
}

/// A connect failure as a person can act on it. NetworkManager reports a
/// wrong password and a missing one alike ("Secrets were required, but not
/// provided"); which it was is whether one was typed.
pub fn explain_connect_error(stderr: &str, ssid: &str, had_password: bool) -> String {
    let secrets = stderr.contains("Secrets were required")
        || stderr.contains("802-11-wireless-security.psk")
        || stderr.contains("802-11-wireless-security.key-mgmt");
    if secrets {
        return if had_password {
            format!("{ssid} did not accept the password.")
        } else {
            format!("{ssid} needs its password.")
        };
    }
    if stderr.contains("No network with SSID") {
        return format!("{ssid} is no longer in range.");
    }
    if stderr.contains("Timeout") {
        return format!("{ssid} did not answer in time.");
    }
    clean_error(stderr)
}

// ── Actions ──

fn spawn_action(state: &mut NetworkState, busy: Busy, job: impl std::future::Future<Output = Result<(), String>> + Send + 'static) {
    state.error = None;
    state.busy = Some(busy.clone());
    let Some(tx) = state.sender.clone() else { return };
    tokio::spawn(async move {
        let res = job.await;
        let _ = tx.send(AppAction::Network(NetworkMessage::Finished(busy, res)));
    });
}

/// A refresh now rather than at the next 5 s tick, after an action.
fn refresh(state: &NetworkState) {
    let Some(tx) = state.sender.clone() else { return };
    tokio::spawn(async move {
        let snap = fetch_network_state().await;
        let _ = tx.send(AppAction::Network(NetworkMessage::Refreshed(Box::new(snap))));
    });
}

/// `new` in `old`'s order: the rows already shown keep their places and
/// networks that appeared go after them, so the row under the pointer stays
/// the row under the pointer.
pub fn keep_order(old: &[WifiNetwork], mut new: Vec<WifiNetwork>) -> Vec<WifiNetwork> {
    let mut out = Vec::with_capacity(new.len());
    for o in old {
        if let Some(i) = new.iter().position(|n| n.ssid == o.ssid) {
            out.push(new.remove(i));
        }
    }
    out.extend(new);
    out
}

pub fn update(state: &mut NetworkState, msg: NetworkMessage, ctx: &mut UiContext) {
    match msg {
        NetworkMessage::Refreshed(new) => {
            let mut new = *new;
            // A radio change in flight shows the state asked for until it answers.
            if let Some(Busy::Radio(on)) = state.busy {
                new.wifi_enabled = on;
            }
            if state.wifi_list.hovered {
                new.available = keep_order(&state.snap.available, new.available);
            }
            state.snap = new;
            state.loaded = true;
            // A network gone out of range takes its panel with it.
            if state.selected_network().is_none() && !matches!(state.busy, Some(Busy::Connecting(_))) {
                state.selected = None;
            }
        }
        NetworkMessage::ToggleWifi => {
            if matches!(state.busy, Some(Busy::Radio(_))) {
                return;
            }
            let on = !state.snap.wifi_enabled;
            state.snap.wifi_enabled = on;
            let args = ["radio", "wifi", if on { "on" } else { "off" }].map(String::from).to_vec();
            spawn_action(state, Busy::Radio(on), nmcli_run(args));
        }
        NetworkMessage::Select(ssid) => {
            if state.selected.as_deref() != Some(ssid.as_str()) {
                ctx[state.password_box].text.clear();
                ctx[state.password_box].edit_buffer.clear();
            }
            state.selected = Some(ssid);
        }
        NetworkMessage::CancelSelect => {
            state.selected = None;
            ctx[state.password_box].text.clear();
            ctx[state.password_box].edit_buffer.clear();
        }
        NetworkMessage::Connect => {
            if state.busy.is_some() {
                return;
            }
            let Some(net) = state.selected_network().cloned() else { return };
            if net.in_use || net.is_enterprise() {
                return;
            }
            let tb = &mut ctx[state.password_box];
            let typed = if tb.editing { tb.edit_buffer.clone() } else { tb.text.clone() };
            // Not kept past the attempt: a typo is retyped either way.
            tb.text.clear();
            tb.edit_buffer.clear();
            let password = (!net.is_open() && !typed.is_empty()).then_some(typed);
            let device = state.snap.device.clone();
            spawn_action(state, Busy::Connecting(net.ssid.clone()), wifi_connect(net.ssid, device, password));
        }
        NetworkMessage::Disconnect => {
            if state.busy.is_some() || state.snap.device.is_empty() {
                return;
            }
            let args = ["dev", "disconnect", state.snap.device.as_str()].map(String::from).to_vec();
            spawn_action(state, Busy::Disconnecting, nmcli_run(args));
        }
        NetworkMessage::Scan => {
            if state.busy.is_some() {
                return;
            }
            let mut args = vec!["dev".to_string(), "wifi".to_string(), "rescan".to_string()];
            if !state.snap.device.is_empty() {
                args.extend(["ifname".to_string(), state.snap.device.clone()]);
            }
            spawn_action(state, Busy::Scanning, nmcli_run(args));
        }
        NetworkMessage::Finished(busy, res) => {
            if state.busy.as_ref() == Some(&busy) {
                state.busy = None;
            }
            match res {
                Ok(()) => {
                    if matches!(busy, Busy::Connecting(_) | Busy::Disconnecting) {
                        state.selected = None;
                    }
                }
                Err(e) => {
                    state.error = Some(match busy {
                        Busy::Radio(on) => format!("Could not turn Wi-Fi {}: {e}", if on { "on" } else { "off" }),
                        Busy::Disconnecting => format!("Could not disconnect: {e}"),
                        Busy::Scanning => format!("Could not scan: {e}"),
                        Busy::Connecting(_) => e,
                    });
                }
            }
            refresh(state);
        }
        NetworkMessage::DismissError => state.error = None,
    }
}

// ── View ──

const TEXT_FG: [f32; 4] = [0.83, 0.83, 0.83, 1.0];
const TEXT_DIM: [f32; 4] = [0.53, 0.53, 0.60, 1.0];
/// Error text: light enough for small type on the dark plate (7.6:1; the
/// old [0.85, 0.25, 0.25] was 4.1:1, under AA for text this size).
const RED: [f32; 4] = [0.95, 0.55, 0.55, 1.0];
const WARN: [f32; 4] = [0.95, 0.75, 0.55, 1.0];
const ACCENT: [f32; 4] = [0.36, 0.56, 0.38, 1.0];
/// The network in use, in the list: ACCENT lifted to read on a row's
/// selected wash as well as on the plate.
const ACCENT_TEXT: [f32; 4] = [0.55, 0.82, 0.58, 1.0];
/// Rows the Wi-Fi list shows before it scrolls.
const WIFI_LIST_ROWS: usize = 5;
const LIST_GAP: f32 = 4.0;
/// The first row's offset into the list (`get_item_draw_y`).
const LIST_TOP: f32 = 4.0;
const NEUTRAL: ([f32; 4], [f32; 4], [f32; 4]) = ([0.16, 0.16, 0.24, 1.0], [0.25, 0.30, 0.26, 1.0], TEXT_FG);
const GO: ([f32; 4], [f32; 4], [f32; 4]) = ([0.16, 0.29, 0.18, 1.0], [0.25, 0.30, 0.26, 1.0], [1.0, 1.0, 1.0, 1.0]);
const DANGER: ([f32; 4], [f32; 4], [f32; 4]) = ([0.30, 0.14, 0.14, 1.0], [0.45, 0.20, 0.20, 1.0], [0.98, 0.65, 0.65, 1.0]);

/// `text` cut by whole characters, with an ellipsis, to fit `max_w` at
/// `size`. Until 2026-10-16 SSIDs were cut by a byte count, which panicked
/// on a character wider than a byte (é, an emoji) in a narrow window.
pub fn elide(text: &str, max_w: f32, size: f32, font: Option<&str>) -> String {
    let w = |s: &str| crate::app::text_width(s, size, font);
    if w(text) <= max_w {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    (0..chars.len())
        .rev()
        .map(|n| chars[..n].iter().collect::<String>() + "…")
        .find(|s| w(s) <= max_w)
        .unwrap_or_else(|| "…".to_string())
}

fn wrapped(col: &mut cce_ui::compose::FormGroup<'_, '_, PageCanvas>, text: &str, size: f32, color: [f32; 4]) {
    let lines = wrap_to_width(text, col.form_width(), size);
    col.lines(lines, size, color);
}

pub fn view(state: &mut NetworkState, cx: f32, cy: f32, cw: f32, ch: f32, focused: &[bool], layout: &mut PageFlow, ctx: &mut UiContext) -> PageCanvas {
    let mut final_pc = PageCanvas::new();
    let sec_w = 320.0f32;
    let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(2);
    let focus = |i: usize| focused.get(i).copied().unwrap_or(false);

    builder.add_section(&mut final_pc, "Wi-Fi", focus(0), |sec| {
        let mut form = sec.form();
        let mut col = form.column();
        // Read before the list's draw closure borrows `wifi_list` for the
        // rest of the form.
        let list_visible = state.wifi_list_visible();
        let panel = state.selected_network().cloned().filter(|_| list_visible);
        let busy = state.busy.clone();
        let error = state.error.clone();
        let snap = &state.snap;

        if !state.loaded {
            col.text("Reading NetworkManager...", crate::app::text_body(), TEXT_DIM);
        } else if !snap.nm_running {
            wrapped(&mut col, "NetworkManager is not running, so Wi-Fi cannot be managed here.", crate::app::text_body(), WARN);
        } else {
            ctx[state.wifi_toggle].set_toggled(snap.wifi_enabled);
            ctx[state.wifi_toggle].set_label(if snap.wifi_enabled { "ON" } else { "OFF" });
            let toggle_w = if col.form_width() < 200.0 { 40.0 } else { 60.0 };
            col.row(|r| {
                r.widget_w_h(ctx, state.wifi_toggle, toggle_w, cce_ui::layout::toggle_height());
            });
            if !snap.wifi_hw_enabled {
                wrapped(&mut col, "Wi-Fi is blocked by a hardware switch or the airplane-mode key.", crate::app::text_small(), WARN);
            }
            if let Some(Busy::Radio(on)) = busy {
                col.text(if on { "Turning Wi-Fi on..." } else { "Turning Wi-Fi off..." }, crate::app::text_small(), TEXT_DIM);
            }

            if snap.wifi_enabled {
                let text_w = col.form_width();
                if snap.connected_ssid.is_empty() {
                    col.text("Not connected", crate::app::text_body(), TEXT_DIM);
                } else {
                    let name = elide(&snap.connected_ssid, text_w - crate::app::text_width("Connected: ", crate::app::text_body(), None), crate::app::text_body(), None);
                    col.text(format!("Connected: {name}"), crate::app::text_body(), ACCENT);
                    let ip = if snap.ip_address.is_empty() { String::new() } else { format!("  IP: {}", snap.ip_address) };
                    wrapped(&mut col, &format!("Signal: {}%{ip}", snap.signal_strength), crate::app::text_body(), TEXT_DIM);
                }
                if busy == Some(Busy::Disconnecting) {
                    col.text("Disconnecting...", crate::app::text_small(), TEXT_DIM);
                }
            }

            if list_visible {
                // A whole number of rows, so the last one is never cut by
                // the list's bottom edge.
                let pitch = state.wifi_list.item_height + state.wifi_list.item_gap;
                let rows = state.snap.available.len().clamp(1, WIFI_LIST_ROWS) as f32;
                let list_h = LIST_TOP + rows * pitch - state.wifi_list.item_gap + LIST_TOP;
                let list = &mut state.wifi_list;
                let available = &state.snap.available;
                let selected = state.selected.clone();
                col.draw(0.0, list_h, false, move |pc, r, _| {
                    // Dissolved List (Phase 6v): scroll state is app-owned; a
                    // frameless list draws no frame, and the host lays its
                    // idle scrollbar under the plate (`frameless_lists`).
                    list.set_rect(r.x, r.y, r.width, r.height);
                    list.update_bounds(available.len(), r.y, r.height);
                    list.push_prims(pc.as_mut());

                    // The list font and colour, as the toolkit's own rows wear.
                    let (font, size) = cce_ui::layout::list_font_parsed();
                    let font = Some(font.as_str());
                    let fg = cce_ui::color::list_font_color();
                    let row_h = list.item_height;
                    let glyph = (size * 0.85).round();
                    let gap = cce_ui::layout::list_gap();
                    let security_slot = crate::app::text_width("Open", size, font).max(glyph);
                    pc.push_clip(r);
                    for (idx, net) in available.iter().enumerate() {
                        let Some(draw_y) = list.get_item_draw_y(idx, LIST_TOP) else { continue };
                        // `lay_row` on the list's own width: one list gap in
                        // from each end, the same on both sides.
                        let row = lay_row(Rect { x: r.x, y: draw_y, width: r.width, height: row_h }, &[Cell::grow(row_h)])[0];
                        let is_selected = selected.as_deref() == Some(net.ssid.as_str());
                        // Selects; Connect is a separate, named button.
                        pc.list_row_selected(row.x, row.y, row.width, row.height, is_selected,
                            AppAction::Network(NetworkMessage::Select(net.ssid.clone())));

                        let ty = crate::app::label_y_in(draw_y, row_h, size, font);
                        let gy = draw_y + (row_h - glyph) / 2.0;
                        let text = |pc: &mut PageCanvas, t: &str, x: f32, color: [f32; 4]| {
                            pc.text_with(t, x, ty, size, crate::app::rgb(color), font.map(str::to_string), None);
                        };
                        // Right: a lock on a secured network or "Open" on one
                        // that is not, in one slot so the percentages line up.
                        let slot_x = row.x + row.width - gap - security_slot;
                        if net.is_open() {
                            text(pc, "Open", slot_x, WARN);
                        } else {
                            pc.icon("lock", crate::app::rect(slot_x + (security_slot - glyph) / 2.0, gy, glyph, glyph), fg);
                        }
                        let pct = format!("{}%", net.signal);
                        let pct_x = slot_x - gap - crate::app::text_width(&pct, size, font);
                        text(pc, &pct, pct_x, TEXT_DIM);

                        // Left: the check on the network in use, then its name.
                        let check_x = row.x + gap;
                        if net.in_use {
                            pc.icon("check", crate::app::rect(check_x, gy, glyph, glyph), ACCENT_TEXT);
                        }
                        let name_x = check_x + glyph + gap;
                        let name = elide(&net.ssid, (pct_x - gap - name_x).max(0.0), size, font);
                        text(pc, &name, name_x, if net.in_use { ACCENT_TEXT } else { fg });
                    }
                    pc.pop_clip();
                    // The scrollbar's fore copy, over the rows at the raise's fade.
                    list.push_scrollbar_fore(pc.as_mut());
                });
            } else if snap.wifi_enabled {
                col.text("No networks found.", crate::app::text_small(), TEXT_DIM);
            }

            if snap.wifi_enabled {
                let (label, action) = if busy == Some(Busy::Scanning) {
                    ("Scanning...", AppAction::Wake)
                } else {
                    ("Scan", AppAction::Network(NetworkMessage::Scan))
                };
                col.row(|r| {
                    form_button(r, label, button_need("Scanning..."), NEUTRAL, action);
                    r.space(0.0, true);
                });
            }

            // ── The selected network: Connect, or Disconnect if in use ──
            if let Some(net) = panel {
                form_divider(&mut col);
                let connecting = busy == Some(Busy::Connecting(net.ssid.clone()));
                if net.in_use {
                    wrapped(&mut col, &format!("Connected to {}.", net.ssid), crate::app::text_body(), TEXT_FG);
                    // `nmcli dev disconnect` also stops the device joining
                    // another network on its own, as GNOME's Disconnect does.
                    wrapped(&mut col, "After Disconnect, Wi-Fi joins no network until you pick one.", crate::app::text_small(), TEXT_DIM);
                    col.row(|r| {
                        form_button(r, "Disconnect", button_need("Disconnect"), DANGER, AppAction::Network(NetworkMessage::Disconnect));
                        form_button(r, "Close", button_need("Close"), NEUTRAL, AppAction::Network(NetworkMessage::CancelSelect));
                        r.space(0.0, true);
                    });
                } else if connecting {
                    wrapped(&mut col, &format!("Connecting to {}...", net.ssid), crate::app::text_body(), TEXT_FG);
                } else {
                    wrapped(&mut col, &format!("Connect to {}?", net.ssid), crate::app::text_body(), TEXT_FG);
                    if net.is_enterprise() {
                        wrapped(
                            &mut col,
                            &format!("{}: an enterprise network needs an identity and certificate this page cannot set up.", net.security),
                            crate::app::text_small(),
                            WARN,
                        );
                    } else if net.is_open() {
                        wrapped(&mut col, "Open network: what you send is not encrypted, and anyone can run one by this name.", crate::app::text_small(), WARN);
                    } else {
                        wrapped(&mut col, &format!("Secured ({}).", net.security), crate::app::text_small(), TEXT_DIM);
                        col.widget_h(ctx, state.password_box, cce_ui::layout::textbox_height());
                        wrapped(&mut col, "Leave the password empty for a network you have joined before.", crate::app::text_small(), TEXT_DIM);
                    }
                    col.row(|r| {
                        if !net.is_enterprise() {
                            form_button(r, "Connect", button_need("Connect"), GO, AppAction::Network(NetworkMessage::Connect));
                        }
                        form_button(r, "Cancel", button_need("Cancel"), NEUTRAL, AppAction::Network(NetworkMessage::CancelSelect));
                        r.space(0.0, true);
                    });
                }
            }

            // ── The last failed action ──
            if let Some(err) = error {
                form_divider(&mut col);
                let lines: Vec<String> = err.lines().flat_map(|l| wrap_to_width(l, col.form_width(), crate::app::text_small())).take(6).collect();
                col.lines(lines, crate::app::text_small(), RED);
                col.row(|r| {
                    form_button(r, "Dismiss", button_need("Dismiss"), NEUTRAL, AppAction::Network(NetworkMessage::DismissError));
                    r.space(0.0, true);
                });
            }
        }
        sec.place(form, ctx);
    });

    builder.add_section(&mut final_pc, "Other Connections", focus(1), |sec| {
        let mut form = sec.form();
        let mut col = form.column();
        if state.loaded && state.snap.nm_running {
            if state.snap.others.is_empty() {
                col.text("None active.", crate::app::text_small(), TEXT_DIM);
            } else {
                let pairs = state
                    .snap
                    .others
                    .iter()
                    .map(|o| (o.name.clone(), TEXT_FG, o.kind.clone(), TEXT_DIM))
                    .collect();
                form_pairs(&mut col, crate::app::text_small(), pairs);
            }
        }
        sec.place(form, ctx);
    });

    final_pc
}

impl crate::pages::AppPage for NetworkState {
    // Sections: [Wi-Fi, Other Connections]
    // Mirrors the view's gates: the toggle is painted only once
    // NetworkManager has answered, the password box only on a secured
    // network's panel; reporting either while not painted is a dead root.
    fn section_widgets(&mut self) -> Vec<Vec<cce_ui::widget::WidgetId>> {
        let mut wifi = Vec::new();
        if self.loaded && self.snap.nm_running {
            wifi.push(self.wifi_toggle.id());
        }
        if self.password_visible() {
            wifi.push(self.password_box.id());
        }
        vec![wifi, Vec::new()]
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
        ctx: &mut UiContext,
    ) -> PageCanvas {
        // Page root dissolved (6u): the ctrl-nav entry focuses section 0 now, which used to
        // be expressed as root focus here.
        let first = root_focused || sec_focused.first().copied().unwrap_or(false);
        let focused = [first, sec_focused.get(1).copied().unwrap_or(false)];
        view(self, cx, cy, cw, ch, &focused, layout, ctx)
    }

    fn propagate_widget_changes(
        &mut self,
        events: &[(cce_ui::widget::WidgetId, cce_ui::widget::WidgetEvent)],
        actions: &mut Vec<AppAction>,
        _ctx: &mut UiContext,
    ) {
        for &(id, ref ev) in events {
            if *ev == cce_ui::widget::WidgetEvent::Changed && id == self.wifi_toggle.id() {
                actions.push(AppAction::Network(NetworkMessage::ToggleWifi));
            }
        }
    }

    fn handle_pointer_move(
        &mut self,
        lx: f32,
        ly: f32,
        _actions: &mut Vec<AppAction>,
        _ctx: &mut UiContext,
    ) -> bool {
        if !self.wifi_list_visible() {
            self.wifi_list.hovered = false;
            return false;
        }
        self.wifi_list.cursor_moved(lx, ly)
    }

    fn handle_pointer_down(&mut self, lx: f32, ly: f32, _ctx: &mut UiContext) -> bool {
        self.wifi_list_visible() && self.wifi_list.press(lx, ly)
    }

    fn handle_pointer_up(&mut self, _ctx: &mut UiContext) -> bool {
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

    fn frameless_lists(&self) -> Vec<&ScrollRegion> {
        vec![&self.wifi_list]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pages::AppPage;

    fn net(ssid: &str, signal: u8, security: &str, in_use: bool) -> WifiNetwork {
        WifiNetwork { ssid: ssid.into(), signal, security: security.into(), in_use }
    }

    fn loaded(ui: &mut UiContext) -> NetworkState {
        let mut st = NetworkState::new(ui);
        st.loaded = true;
        st.snap = NetworkSnapshot {
            nm_running: true,
            wifi_hw_enabled: true,
            wifi_enabled: true,
            device: "wlan0".into(),
            connected_ssid: "Home".into(),
            signal_strength: 70,
            ip_address: "192.168.1.5".into(),
            available: vec![net("Home", 70, "WPA2", true), net("Cafe", 60, "", false), net("Office", 50, "WPA2", false)],
            others: vec![OtherConnection { name: "tailscale0".into(), kind: "Tunnel".into() }],
        };
        st
    }

    fn render(st: &mut NetworkState, ui: &mut UiContext, cw: f32) -> crate::app::PageLayers {
        let mut layout = PageFlow::new();
        view(st, 0.0, 0.0, cw, 600.0, &[false, false], &mut layout, ui).finish()
    }

    #[test]
    fn terse_fields_unescape_colons_and_backslashes() {
        assert_eq!(split_terse(r"*:My\:Net:70:WPA2"), ["*", "My:Net", "70", "WPA2"]);
        assert_eq!(split_terse(r" :back\\slash:5:"), [" ", r"back\slash", "5", ""]);
    }

    /// Real `nmcli -t -f IN-USE,SSID,SIGNAL,SECURITY dev wifi list` output
    /// (2026-10-16, nmcli 1.58), plus a colon in a name.
    #[test]
    fn the_wifi_list_folds_access_points_and_finds_the_one_in_use() {
        let out = "\
 :RaNet:84:WPA2
 :MetMuseumFreeWiFi:77:
*:MetMuseumFreeWiFi:70:
 :RaNet:80:WPA2
 ::77:WPA2 802.1X
 :Lab\\:5G:40:WPA3
";
        let (nets, connected) = parse_wifi_list(out);
        assert_eq!(connected, Some(("MetMuseumFreeWiFi".to_string(), 70)), "the signal of the AP in use, not the strongest");
        let names: Vec<&str> = nets.iter().map(|n| n.ssid.as_str()).collect();
        assert_eq!(names, ["MetMuseumFreeWiFi", "RaNet", "Lab:5G"], "in use first, then by signal; hidden left out");
        assert!(nets[0].in_use && nets[0].is_open() && nets[0].signal == 77);
        assert!(!nets[1].in_use && !nets[1].is_open() && nets[1].signal == 84);
        assert_eq!(nets[2].signal, 40);
    }

    #[test]
    fn other_connections_leave_out_wifi_and_loopback() {
        let out = "MetMuseumFreeWiFi:802-11-wireless\ntailscale0:tun\nlo:loopback\nWork VPN:vpn\n";
        let others = parse_other_connections(out);
        assert_eq!(others, [
            OtherConnection { name: "tailscale0".into(), kind: "Tunnel".into() },
            OtherConnection { name: "Work VPN".into(), kind: "VPN".into() },
        ]);
        assert_eq!(parse_ip("IP4.ADDRESS[1]:192.168.1.5/24\n"), "192.168.1.5");
    }

    /// The row under the pointer stays the row under the pointer.
    #[test]
    fn a_hovered_list_keeps_its_order() {
        let old = vec![net("A", 50, "", false), net("B", 40, "", false), net("C", 30, "", false)];
        let new = vec![net("C", 90, "", false), net("D", 80, "", false), net("A", 20, "", false)];
        let kept = keep_order(&old, new);
        let names: Vec<&str> = kept.iter().map(|n| n.ssid.as_str()).collect();
        assert_eq!(names, ["A", "C", "D"]);
        assert_eq!(kept[1].signal, 90, "values update in place");

        let mut ui = UiContext::new();
        let mut st = loaded(&mut ui);
        st.wifi_list.hovered = true;
        let mut snap = st.snap.clone();
        snap.available.reverse();
        update(&mut st, NetworkMessage::Refreshed(Box::new(snap.clone())), &mut ui);
        assert_eq!(st.snap.available[0].ssid, "Home");
        st.wifi_list.hovered = false;
        update(&mut st, NetworkMessage::Refreshed(Box::new(snap)), &mut ui);
        assert_eq!(st.snap.available[0].ssid, "Office", "re-sorted once the pointer is away");
    }

    /// A click selects; nothing connects until Connect.
    #[test]
    fn a_row_click_selects_and_connect_waits_for_the_button() {
        let mut ui = UiContext::new();
        let mut st = loaded(&mut ui);
        update(&mut st, NetworkMessage::Select("Office".into()), &mut ui);
        assert_eq!(st.selected.as_deref(), Some("Office"));
        assert!(st.busy.is_none(), "selecting runs nothing");
        assert!(st.password_visible(), "a secured network asks for its password");
        assert_eq!(st.section_widgets()[0].len(), 2);

        update(&mut st, NetworkMessage::Select("Cafe".into()), &mut ui);
        assert!(!st.password_visible(), "an open network has none");

        // The network in use offers Disconnect, not a reconnect.
        update(&mut st, NetworkMessage::Select("Home".into()), &mut ui);
        update(&mut st, NetworkMessage::Connect, &mut ui);
        assert!(st.busy.is_none());

        update(&mut st, NetworkMessage::Select("Office".into()), &mut ui);
        ui[st.password_box].text = "hunter22".into();
        update(&mut st, NetworkMessage::Connect, &mut ui);
        assert_eq!(st.busy, Some(Busy::Connecting("Office".into())));
        assert!(ui[st.password_box].text.is_empty(), "the password is not kept");
    }

    #[test]
    fn a_failed_connect_is_reported() {
        let mut ui = UiContext::new();
        let mut st = loaded(&mut ui);
        st.selected = Some("Office".into());
        st.busy = Some(Busy::Connecting("Office".into()));
        let err = explain_connect_error(
            "Error: Connection activation failed: (7) Secrets were required, but not provided.\n",
            "Office",
            true,
        );
        update(&mut st, NetworkMessage::Finished(Busy::Connecting("Office".into()), Err(err)), &mut ui);
        assert!(st.busy.is_none());
        assert_eq!(st.error.as_deref(), Some("Office did not accept the password."));
        assert_eq!(st.selected.as_deref(), Some("Office"), "the panel stays for another try");

        assert_eq!(explain_connect_error("Error: ... Secrets were required ...", "X", false), "X needs its password.");
        assert_eq!(explain_connect_error("Error: No network with SSID 'X' found.", "X", false), "X is no longer in range.");
        assert_eq!(explain_connect_error("Error: something else\n", "X", false), "something else");
    }

    #[test]
    fn a_failed_radio_switch_is_reported_and_put_back() {
        let mut ui = UiContext::new();
        let mut st = loaded(&mut ui);
        update(&mut st, NetworkMessage::ToggleWifi, &mut ui);
        assert!(!st.snap.wifi_enabled);
        assert_eq!(st.busy, Some(Busy::Radio(false)));
        // A refresh while it runs shows what was asked for.
        let snap = st.snap.clone();
        update(&mut st, NetworkMessage::Refreshed(Box::new(NetworkSnapshot { wifi_enabled: true, ..snap })), &mut ui);
        assert!(!st.snap.wifi_enabled);
        update(&mut st, NetworkMessage::Finished(Busy::Radio(false), Err("Not authorized".into())), &mut ui);
        assert!(st.error.as_deref().unwrap().contains("Could not turn Wi-Fi off"));
        assert!(st.busy.is_none());
    }

    #[test]
    fn a_network_out_of_range_closes_its_panel() {
        let mut ui = UiContext::new();
        let mut st = loaded(&mut ui);
        update(&mut st, NetworkMessage::Select("Office".into()), &mut ui);
        let mut snap = st.snap.clone();
        snap.available.retain(|n| n.ssid != "Office");
        update(&mut st, NetworkMessage::Refreshed(Box::new(snap)), &mut ui);
        assert!(st.selected.is_none());
    }

    /// The 2026-10-16 crash: a long SSID with multi-byte characters, in a
    /// narrow window, was cut by bytes mid-character.
    #[test]
    fn multibyte_names_never_panic_at_any_width() {
        let name = "Café Café Café Café Café Caf"; // 33 bytes
        for cw in [800.0, 300.0, 250.0, 120.0] {
            let mut ui = UiContext::new();
            let mut st = loaded(&mut ui);
            st.snap.connected_ssid = name.into();
            st.snap.available = vec![net(name, 50, "WPA2", true), net("🏠🏠🏠🏠🏠🏠🏠🏠", 40, "", false)];
            st.selected = Some(name.into());
            let _ = render(&mut st, &mut ui, cw);
        }
        let e = elide("Café Café Café", 30.0, 12.0, None);
        assert!(e.ends_with('…') || e == "Café Café Café");
    }

    #[test]
    fn section_widgets_mirror_the_view() {
        let mut ui = UiContext::new();
        let mut st = NetworkState::new(&mut ui);
        assert_eq!(st.section_widgets(), vec![Vec::new(), Vec::new()]);
        st.loaded = true;
        assert_eq!(st.section_widgets(), vec![Vec::new(), Vec::new()], "NetworkManager not running: no toggle drawn");
        let mut st = loaded(&mut ui);
        assert_eq!(st.section_widgets()[0], vec![st.wifi_toggle.id()]);
    }

    /// Every state the page can be in renders.
    #[test]
    fn every_state_renders() {
        let mut ui = UiContext::new();
        let mut st = NetworkState::new(&mut ui);
        let _ = render(&mut st, &mut ui, 800.0);
        st.loaded = true;
        let pc = render(&mut st, &mut ui, 800.0);
        assert!(pc.texts().iter().any(|t| t.text.contains("NetworkManager is not running")));
        let mut st = loaded(&mut ui);
        st.error = Some("Could not scan: busy".into());
        st.selected = Some("Cafe".into());
        let pc = render(&mut st, &mut ui, 800.0);
        let texts: Vec<String> = pc.texts().into_iter().map(|t| t.text).collect();
        assert!(texts.iter().any(|t| t.starts_with("Connect to Cafe")), "{texts:?}");
        assert!(texts.iter().any(|t| t.contains("not encrypted")));
        assert!(texts.iter().any(|t| t == "tailscale0"));
        assert!(texts.iter().any(|t| t.contains("Could not scan")));
    }
}
