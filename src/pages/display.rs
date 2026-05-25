use crate::app::PageContent;
use clear_ui::layout::{render_widget, Section};
use clear_ui::widget::{Spinbox, Label, Widget};

#[derive(Debug, Clone)]
pub struct DisplayOutput {
    pub name: String,
    pub resolution: String,
    pub refresh: String,
    pub scale: f32,
    pub connected: bool,
    pub name_label: Label,
    pub resolution_label: Label,
    pub scale_label: Option<Label>,
}

impl DisplayOutput {
    pub fn update_labels(&mut self) {
        if self.connected {
            self.name_label.set_text(&self.name);
            self.resolution_label.set_text(&format!("{} @ {}Hz", self.resolution, self.refresh));
            if self.scale > 1.0 {
                let scale_str = if let Some((w_str, h_str)) = self.resolution.rsplit_once('x') {
                    if let (Ok(w), Ok(h)) = (w_str.parse::<u32>(), h_str.parse::<u32>()) {
                        format!("logical {:.0}x{:.0} | scale {:.0}x", w as f32 / self.scale, h as f32 / self.scale, self.scale)
                    } else { format!("scale {:.0}x", self.scale) }
                } else { format!("scale {:.0}x", self.scale) };
                
                self.scale_label = Some(Label::new(&scale_str)
                    .with_font_size(11.0)
                    .with_color([135, 135, 150]));
            } else {
                self.scale_label = None;
            }
        } else {
            self.name_label.set_text(&self.name);
            self.name_label.set_color([135, 135, 150]);
            self.resolution_label.set_text("(disconnected)");
            self.resolution_label.set_color([135, 135, 150]);
            self.scale_label = None;
        }
    }
}

#[derive(Debug, Clone)]
pub struct DisplayState {
    pub loaded: bool,
    pub brightness: f32,
    pub max_brightness: f32,
    pub outputs: Vec<DisplayOutput>,
    pub night_light: bool,
    pub brightness_spinbox: Spinbox,
    pub night_light_label: Label,
}

impl Default for DisplayState {
    fn default() -> Self {
        Self {
            loaded: false,
            brightness: 0.0,
            max_brightness: 0.0,
            outputs: Vec::new(),
            night_light: false,
            brightness_spinbox: Spinbox::new(50, 0, 100, 5).with_unit("%"),
            night_light_label: Label::new("Night Light: OFF").with_font_size(13.0).with_color([0xd4, 0xd4, 0xd4]),
        }
    }
}

#[derive(Debug, Clone)]
pub enum DisplayMessage {
    Refreshed(DisplayState),
    BrightnessSet(u32),
}

pub async fn fetch_display_state() -> DisplayState {
    let (brightness, max_brightness) = fetch_brightness().await;
    let outputs = fetch_outputs().await;
    let night_light = is_night_light_on().await;
    let pct = if max_brightness > 0.0 {
        (brightness / max_brightness * 100.0).round() as i32
    } else { 50 };
    DisplayState {
        loaded: true,
        brightness, max_brightness, outputs, night_light,
        brightness_spinbox: Spinbox::new(pct, 0, 100, 5).with_unit("%"),
        night_light_label: Label::new(if night_light { "Night Light: ON" } else { "Night Light: OFF" })
            .with_font_size(13.0)
            .with_color([0xd4, 0xd4, 0xd4]),
    }
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
            if let Some(mut prev) = current.take() {
                prev.update_labels();
                displays.push(prev);
            }
            let name = trimmed.split_whitespace().next().unwrap_or("").to_string();
            let name_label = Label::new(&name).with_font_size(12.0).with_color([212, 212, 212]);
            let resolution_label = Label::new("").with_font_size(12.0).with_color([212, 212, 212]);
            current = Some(DisplayOutput {
                name, resolution: String::new(), refresh: String::new(),
                scale: 1.0, connected: true,
                name_label,
                resolution_label,
                scale_label: None,
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
    if let Some(mut prev) = current.take() {
        prev.update_labels();
        displays.push(prev);
    }
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
const BLANK_BAR: [f32; 4] = [0.15, 0.15, 0.24, 1.0];
const FILL_BAR: [f32; 4] = [0.30, 0.50, 0.32, 1.0];

pub fn view(state: &mut DisplayState, cx: f32, cy: f32, cw: f32, _ch: f32) -> PageContent {
    let mut pc = PageContent::new();
    let mut y = cy + 12.0;

    // ── Brightness ──
    let mut sec = Section::new(&mut pc, cx, y, cw, "Brightness");

    if !state.loaded {
        sec.text(&mut pc, "Loading display settings...", 12.0, 0.0, 12.0, TEXT_DIM);
        sec.spacing(18.0);
    } else {
        let bright_pct = if state.max_brightness > 0.0 {
            (state.brightness / state.max_brightness * 100.0).round() as i32
        } else { 0 };

        let bar_w = cw - 100.0;
        let yt = sec.ay();
        pc.rect(BLANK_BAR, sec.ax(12.0), yt, bar_w, 8.0);
        pc.rect(FILL_BAR, sec.ax(12.0), yt, bar_w * bright_pct as f32 / 100.0, 8.0);
        pc.text(&format!("{}%", bright_pct), sec.ax(16.0 + bar_w), yt - 2.0, 11.0, TEXT_DIM);
        sec.content_y += 14.0;

        let yt = sec.ay();
        let sb_w = 100.0;
        let sb_h = 26.0;
        state.brightness_spinbox.value = bright_pct;
        state.brightness_spinbox.set_row_rect(sec.ax(8.0), cw - 16.0);
        render_widget(&mut pc, &mut state.brightness_spinbox, sec.ax(12.0), yt, sb_w, sb_h);
        sec.content_y += sb_h + 12.0;
    }
    y = sec.finish(&mut pc);

    // ── Night Light ──
    let mut sec = Section::new(&mut pc, cx, y, cw, "Night Light");
    if !state.loaded {
        sec.text(&mut pc, "Loading...", 12.0, 0.0, 12.0, TEXT_DIM);
        sec.spacing(18.0);
    } else {
        let nl_label = if state.night_light { "Night Light: ON" } else { "Night Light: OFF" };
        state.night_light_label.set_text(nl_label);
        sec.widget(&mut pc, &mut state.night_light_label, 12.0, cw - 24.0, 20.0);
        sec.spacing(8.0);
    }
    y = sec.finish(&mut pc);

    // ── Outputs ──
    let mut sec = Section::new(&mut pc, cx, y, cw, "Outputs");

    if !state.loaded {
        sec.text(&mut pc, "Loading outputs...", 12.0, 0.0, 12.0, TEXT_DIM);
        sec.spacing(18.0);
    } else {
        for out in &mut state.outputs {
            let yt = sec.ay();
            
            // Name label at x = 14.0
            render_widget(&mut pc, &mut out.name_label, sec.ax(14.0), yt, 100.0, 20.0);
            
            // Resolution label at x = 120.0
            render_widget(&mut pc, &mut out.resolution_label, sec.ax(120.0), yt, 160.0, 20.0);
            
            // Scale label at x = 290.0 if present
            if let Some(ref mut scale_lbl) = out.scale_label {
                render_widget(&mut pc, scale_lbl, sec.ax(290.0), yt, cw - 304.0, 20.0);
            }
            
            sec.content_y += 20.0;
            sec.spacing(12.0);
        }
    }
    sec.finish(&mut pc);

    pc
}

pub fn update(state: &mut DisplayState, msg: DisplayMessage) {
    match msg {
        DisplayMessage::Refreshed(new) => {
            let was_nl_hovered = state.night_light_label.hovered();
            
            let mut hovers = std::collections::HashMap::new();
            for out in &state.outputs {
                hovers.insert(out.name.clone(), (
                    out.name_label.hovered(),
                    out.resolution_label.hovered(),
                    out.scale_label.as_ref().map(|l| l.hovered()).unwrap_or(false)
                ));
            }
            
            *state = new;
            state.night_light_label.set_hovered(was_nl_hovered);
            
            for out in &mut state.outputs {
                if let Some(&(name_h, res_h, scale_h)) = hovers.get(&out.name) {
                    out.name_label.set_hovered(name_h);
                    out.resolution_label.set_hovered(res_h);
                    if let Some(ref mut scale_lbl) = out.scale_label {
                        scale_lbl.set_hovered(scale_h);
                    }
                }
            }
        }
        DisplayMessage::BrightnessSet(pct) => {
            let pct = pct.clamp(0, 100);
            state.brightness = pct as f32 / 100.0 * state.max_brightness;
            spawn_brightness(pct);
            state.brightness_spinbox.value = pct as i32;
        }
    }
}
