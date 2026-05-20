use crate::app::{AppAction, PageContent};

#[derive(Debug, Clone)]
pub struct DisplayOutput {
    pub name: String,
    pub resolution: String,
    pub refresh: String,
    pub scale: f32,
    pub connected: bool,
}

#[derive(Debug, Clone, Default)]
pub struct DisplayState {
    pub brightness: f32,
    pub max_brightness: f32,
    pub outputs: Vec<DisplayOutput>,
    pub night_light: bool,
}

#[derive(Debug, Clone)]
pub enum DisplayMessage {
    Refreshed(DisplayState),
    BrightnessDecrement,
    BrightnessIncrement,
}

pub async fn fetch_display_state() -> DisplayState {
    let (brightness, max_brightness) = fetch_brightness().await;
    let outputs = fetch_outputs().await;
    let night_light = is_night_light_on().await;
    DisplayState { brightness, max_brightness, outputs, night_light }
}

async fn fetch_brightness() -> (f32, f32) {
    let cur = tokio::process::Command::new("brightnessctl")
        .arg("get").output().await.ok()
        .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse::<f32>().ok())
        .unwrap_or(50.0);
    let max = tokio::process::Command::new("brightnessctl")
        .arg("max").output().await.ok()
        .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse::<f32>().ok())
        .unwrap_or(100.0);
    (cur, max)
}

async fn fetch_outputs() -> Vec<DisplayOutput> {
    let output = tokio::process::Command::new("wlr-randr")
        .output().await.ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    let mut displays = Vec::new();
    let mut current: Option<DisplayOutput> = None;

    for line in output.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with(' ') && trimmed.contains('"') {
            if let Some(prev) = current.take() { displays.push(prev); }
            let name = trimmed.split_whitespace().next().unwrap_or("").to_string();
            current = Some(DisplayOutput {
                name, resolution: String::new(), refresh: String::new(),
                scale: 1.0, connected: true,
            });
            continue;
        }
        if let Some(ref mut out) = current {
            if trimmed.contains("px,") && trimmed.contains("Hz") && trimmed.contains("current") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if let Some(mode) = parts.first() { out.resolution = mode.to_string(); }
                if let Some(hz_idx) = parts.iter().position(|p| *p == "Hz") {
                    if hz_idx > 0 { out.refresh = parts[hz_idx - 1].to_string(); }
                }
            } else if let Some(rest) = trimmed.strip_prefix("Scale:") {
                out.scale = rest.trim().parse::<f32>().unwrap_or(1.0);
            } else if trimmed.starts_with("Enabled:") && trimmed.contains("no") {
                out.connected = false;
            }
        }
    }
    if let Some(prev) = current.take() { displays.push(prev); }
    displays
}

async fn is_night_light_on() -> bool {
    tokio::process::Command::new("gdbus")
        .args(["call", "--session", "--dest", "org.gnome.SettingsDaemon.Color",
               "--object-path", "/org/gnome/SettingsDaemon/Color",
               "--method", "org.gnome.SettingsDaemon.Color.Get", "night-light-enabled"])
        .output().await.ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("true"))
        .unwrap_or(false)
}

fn spawn_brightness(pct: u32) {
    let _ = tokio::process::Command::new("brightnessctl")
        .args(["set", &format!("{}%", pct)]).spawn();
}

const TEXT_FG: [f32; 4] = [0.83, 0.83, 0.83, 1.0];
const TEXT_DIM: [f32; 4] = [0.53, 0.53, 0.60, 1.0];
const BTN_ACTIVE: [f32; 4] = [0.20, 0.40, 0.22, 1.0];
const BTN_INACTIVE: [f32; 4] = [0.13, 0.18, 0.14, 1.0];
const BTN_HOVER: [f32; 4] = [0.25, 0.30, 0.26, 1.0];
const SECTION_BORDER: [f32; 4] = [0.18, 0.18, 0.27, 1.0];
const BLANK_BAR: [f32; 4] = [0.15, 0.15, 0.24, 1.0];
const FILL_BAR: [f32; 4] = [0.30, 0.50, 0.32, 1.0];
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

pub fn view(state: &DisplayState, cx: f32, cy: f32, cw: f32, _ch: f32) -> PageContent {
    let mut pc = PageContent::new();
    let mut y = cy + 12.0;

    // ── Brightness ──
    pc.text("Brightness", cx + 12.0, y, 14.0, TEXT_FG);
    y += 22.0;

    let bright_pct = if state.max_brightness > 0.0 {
        (state.brightness / state.max_brightness * 100.0) as u32
    } else { 0 };

    let bar_w = cw - 100.0;
    pc.rect(BLANK_BAR, cx + 12.0, y, bar_w, 8.0);
    pc.rect(FILL_BAR, cx + 12.0, y, bar_w * bright_pct as f32 / 100.0, 8.0);
    pc.text(&format!("{}%", bright_pct), cx + 16.0 + bar_w, y - 2.0, 11.0, TEXT_DIM);
    y += 14.0;

    let btn_h = 28.0;
    let btn_y = y;
    pc.button("-10", cx + 12.0, btn_y, 36.0, btn_h,
        BTN_INACTIVE, BTN_HOVER, WHITE,
        AppAction::Display(DisplayMessage::BrightnessDecrement));
    pc.text("brightnessctl set", cx + 56.0, btn_y + 8.0, 10.0, TEXT_DIM);
    pc.button("+10", cx + 12.0 + bar_w - 36.0, btn_y, 36.0, btn_h,
        BTN_ACTIVE, BTN_HOVER, WHITE,
        AppAction::Display(DisplayMessage::BrightnessIncrement));

    y = btn_y + btn_h + 12.0;

    // ── Night Light ──
    pc.rect(SECTION_BORDER, cx + 8.0, y, cw - 16.0, 1.0);
    y += 8.0;
    let nl_label = if state.night_light { "Night Light: ON" } else { "Night Light: OFF" };
    pc.text(nl_label, cx + 12.0, y, 13.0, TEXT_FG);
    y += 24.0;

    // ── Outputs ──
    pc.rect(SECTION_BORDER, cx + 8.0, y, cw - 16.0, 1.0);
    y += 8.0;
    pc.text("Outputs", cx + 12.0, y, 14.0, TEXT_FG);
    y += 22.0;

    for out in &state.outputs {
        if out.connected {
            let scale_info = if out.scale > 1.0 {
                if let Some((w_str, h_str)) = out.resolution.rsplit_once('x') {
                    if let (Ok(w), Ok(h)) = (w_str.parse::<u32>(), h_str.parse::<u32>()) {
                        format!("  logical {:.0}x{:.0} | scale {:.0}x", w as f32 / out.scale, h as f32 / out.scale, out.scale)
                    } else { format!("  scale {:.0}x", out.scale) }
                } else { String::new() }
            } else { String::new() };
            pc.text(&format!("{}  {} @ {}Hz{}", out.name, out.resolution, out.refresh, scale_info),
                cx + 14.0, y, 12.0, TEXT_FG);
        } else {
            pc.text(&format!("{}  (disconnected)", out.name), cx + 14.0, y, 12.0, TEXT_DIM);
        }
        y += 18.0;
    }

    pc
}

pub fn update(state: &mut DisplayState, msg: DisplayMessage) {
    match msg {
        DisplayMessage::Refreshed(new) => { *state = new; }
        DisplayMessage::BrightnessDecrement => {
            let pct = if state.max_brightness > 0.0 {
                (state.brightness / state.max_brightness * 100.0) as u32
            } else { 0 };
            if pct > 0 {
                let new_pct = pct.saturating_sub(10).max(0);
                state.brightness = new_pct as f32 / 100.0 * state.max_brightness;
                spawn_brightness(new_pct);
            }
        }
        DisplayMessage::BrightnessIncrement => {
            let pct = if state.max_brightness > 0.0 {
                (state.brightness / state.max_brightness * 100.0) as u32
            } else { 0 };
            if pct < 100 {
                let new_pct = (pct + 10).min(100);
                state.brightness = new_pct as f32 / 100.0 * state.max_brightness;
                spawn_brightness(new_pct);
            }
        }
    }
}
