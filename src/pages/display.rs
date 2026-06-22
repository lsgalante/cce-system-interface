use crate::app::{PageContent, SectionContextExt};
use cce_ui::layout::{render_widget, PageLayoutBuilder, LayoutStrategy};
use cce_ui::widget::{Spinbox, Label, Element, Toggle, Dropdown, Slider};

const CONFIG_PATH: &str = "/home/lsgalante/.config/cce/config.json";

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
    pub brightness_slider: Slider,
    pub brightness_spinbox: Spinbox,
    pub night_light_label: Label,
    // screensaver fields:
    pub screensaver_enable: bool,
    pub screensaver_enable_toggle: Toggle,
    pub screensaver_timeout: i32,
    pub screensaver_timeout_spinbox: Spinbox,
    pub screensaver_lock_screen: bool,
    pub screensaver_lock_screen_toggle: Toggle,
    pub screensaver_style: String,
    pub screensaver_style_menu: Dropdown,
}

impl Default for DisplayState {
    fn default() -> Self {
        Self {
            loaded: false,
            brightness: 0.0,
            max_brightness: 0.0,
            outputs: Vec::new(),
            night_light: false,
            brightness_slider: Slider::new().with_range(0.0, 100.0).with_scroll(true),
            brightness_spinbox: Spinbox::new(50, 0, 100, 5).with_unit("%"),
            night_light_label: Label::new("Night Light: OFF").with_font_size(13.0).with_color([0xd4, 0xd4, 0xd4]),
            screensaver_enable: true,
            screensaver_enable_toggle: Toggle::new().with_label("Enable Screensaver").with_config(CONFIG_PATH, "enable"),
            screensaver_timeout: 10,
            screensaver_timeout_spinbox: Spinbox::new(10, 1, 120, 1)
                .with_label("Screensaver Timeout")
                .with_unit("m")
                .with_config(CONFIG_PATH, "timeout"),
            screensaver_lock_screen: true,
            screensaver_lock_screen_toggle: Toggle::new().with_label("Lock Screen on Activation").with_config(CONFIG_PATH, "lock_screen"),
            screensaver_style: "starfield".to_string(),
            screensaver_style_menu: Dropdown::new(
                vec!["Blank".to_string(), "Starfield".to_string(), "Matrix Rain".to_string()],
                1,
            ).with_label("Screensaver Style")
            .with_config(CONFIG_PATH, "style"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum DisplayMessage {
    Refreshed(DisplayState),
    BrightnessSet(u32),
    ToggleScreensaverEnable,
    ToggleScreensaverLockScreen,
    SetScreensaverTimeout(i32),
    SetScreensaverStyle(usize),
    StartScreensaverPreview,
}

fn parse_json(content: &str) -> serde_json::Value {
    serde_json::from_str(content).unwrap_or_default()
}

fn parse_screensaver_enable(content: &str) -> bool {
    let val = parse_json(content);
    val["screensaver"]["enable"].as_bool().unwrap_or(true)
}

fn parse_screensaver_lock_screen(content: &str) -> bool {
    let val = parse_json(content);
    val["screensaver"]["lock_screen"].as_bool().unwrap_or(true)
}

fn parse_screensaver_timeout(content: &str) -> i32 {
    let val = parse_json(content);
    val["screensaver"]["timeout"].as_i64().map(|v| v as i32).unwrap_or(10)
}

fn parse_screensaver_style(content: &str) -> String {
    let val = parse_json(content);
    val["screensaver"]["style"].as_str().unwrap_or("starfield").to_string()
}

pub fn write_config_value(key: &str, value: &str) {
    cce_ui::config::write_config_value(CONFIG_PATH, key, value, "screensaver");
}

pub async fn fetch_display_state() -> DisplayState {
    let (brightness, max_brightness) = fetch_brightness().await;
    let outputs = fetch_outputs().await;
    let night_light = is_night_light_on().await;
    let pct = if max_brightness > 0.0 {
        (brightness / max_brightness * 100.0).round() as i32
    } else { 50 };

    let content = std::fs::read_to_string(CONFIG_PATH).unwrap_or_default();
    let screensaver_enable = parse_screensaver_enable(&content);
    let screensaver_timeout = parse_screensaver_timeout(&content);
    let screensaver_lock_screen = parse_screensaver_lock_screen(&content);
    let screensaver_style = parse_screensaver_style(&content);

    let style_idx = match screensaver_style.to_lowercase().as_str() {
        "blank" => 0,
        "starfield" => 1,
        "matrix" => 2,
        _ => 1, // default to Starfield
    };

    DisplayState {
        loaded: true,
        brightness, max_brightness, outputs, night_light,
        brightness_slider: Slider::new().with_range(0.0, 100.0).with_scroll(true).with_value(pct.max(1) as f32 / 100.0),
        brightness_spinbox: Spinbox::new(pct.max(1), 1, 100, 5).with_unit("%"),
        night_light_label: Label::new(if night_light { "Night Light: ON" } else { "Night Light: OFF" })
            .with_font_size(13.0)
            .with_color([0xd4, 0xd4, 0xd4]),
        screensaver_enable,
        screensaver_enable_toggle: Toggle::new().with_label("Enable Screensaver").with_config(CONFIG_PATH, "enable"),
        screensaver_timeout,
        screensaver_timeout_spinbox: Spinbox::new(screensaver_timeout, 1, 120, 1)
            .with_label("Screensaver Timeout")
            .with_unit("m")
            .with_config(CONFIG_PATH, "timeout"),
        screensaver_lock_screen,
        screensaver_lock_screen_toggle: Toggle::new().with_label("Lock Screen on Activation").with_config(CONFIG_PATH, "lock_screen"),
        screensaver_style: screensaver_style.clone(),
        screensaver_style_menu: Dropdown::new(
            vec!["Blank".to_string(), "Starfield".to_string(), "Matrix Rain".to_string()],
            style_idx,
        ).with_label("Screensaver Style")
        .with_config(CONFIG_PATH, "style"),
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
        .args(["set", &format!("{}%", pct), "-n"]).spawn();
}

const TEXT_DIM: [f32; 4] = [0.53, 0.53, 0.60, 1.0];
#[allow(dead_code)]
const BLANK_BAR: [f32; 4] = [0.15, 0.15, 0.24, 1.0];
#[allow(dead_code)]
const FILL_BAR: [f32; 4] = [0.30, 0.50, 0.32, 1.0];

const BTN_BG: [f32; 4] = [0.20, 0.40, 0.65, 1.0];
const BTN_HOVER: [f32; 4] = [0.28, 0.50, 0.78, 1.0];
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

pub fn view(state: &mut DisplayState, cx: f32, cy: f32, cw: f32, ch: f32, layout: &mut dyn LayoutStrategy, ctx: &mut cce_ui::context::UiContext) -> PageContent {
    let mut final_pc = PageContent::new();
    let sec_w = 320.0f32;
    let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(4);

    // ── Brightness ──
    builder.add_section(&mut final_pc, "Brightness", false, |sec| {
        if !state.loaded {
            sec.text("Loading display settings...", 12.0, 0.0, 12.0, TEXT_DIM);
            sec.spacing(18.0);
        } else {
            let bright_pct = if state.max_brightness > 0.0 {
                (state.brightness / state.max_brightness * 100.0).round() as i32
            } else { 0 };

            let pad = sec.padding();
            let bar_w = sec.cw - 2.0 * pad - 12.0 - 45.0;
            let yt = sec.ay();
            state.brightness_slider.set_value(bright_pct as f32 / 100.0);
                        let slider_x = sec.ax(12.0);
            render_widget(sec.pc, &mut state.brightness_slider, slider_x, yt, bar_w, cce_ui::layout::slider_height(), ctx);
            sec.text(&format!("{}%", bright_pct), 12.0 + bar_w + 8.0, 7.0, 11.0, TEXT_DIM);
            sec.spacing(34.0);

            state.brightness_spinbox.value = bright_pct;
            sec.widget_full(&mut state.brightness_spinbox, cce_ui::layout::spinbox_height(), ctx);
            sec.spacing(12.0);
        }
    });

    // ── Night Light ──
    builder.add_section(&mut final_pc, "Night Light", false, |sec| {
        if !state.loaded {
            sec.text("Loading...", 12.0, 0.0, 12.0, TEXT_DIM);
            sec.spacing(18.0);
        } else {
            let nl_label = if state.night_light { "Night Light: ON" } else { "Night Light: OFF" };
            state.night_light_label.set_text(nl_label);
            sec.widget(&mut state.night_light_label, 12.0, sec_w - 24.0, 20.0, ctx);
            sec.spacing(8.0);
        }
    });

    // ── Outputs ──
    builder.add_section(&mut final_pc, "Outputs", false, |sec| {
        if !state.loaded {
            sec.text("Loading outputs...", 12.0, 0.0, 12.0, TEXT_DIM);
            sec.spacing(18.0);
        } else {
            for out in &mut state.outputs {
                sec.add_section(&out.name, false, |subsec| {
                    subsec.widget(&mut out.resolution_label, 12.0, 240.0, 20.0, ctx);
                    if let Some(ref mut scale_lbl) = out.scale_label {
                        subsec.widget(scale_lbl, 12.0, 240.0, 20.0, ctx);
                    }
                });
            }
        }
    });

    // ── Screensaver Settings ──
    builder.add_section(&mut final_pc, "Screensaver Settings", false, |sec| {
        state.screensaver_enable_toggle.set_toggled(state.screensaver_enable);
        sec.widget_full(&mut state.screensaver_enable_toggle, cce_ui::layout::toggle_height(), ctx);
        sec.spacing(8.0);

        state.screensaver_lock_screen_toggle.set_toggled(state.screensaver_lock_screen);
        sec.widget_full(&mut state.screensaver_lock_screen_toggle, cce_ui::layout::toggle_height(), ctx);
        sec.spacing(16.0);

                state.screensaver_timeout_spinbox.value = state.screensaver_timeout;
        sec.widget_full(&mut state.screensaver_timeout_spinbox, cce_ui::layout::spinbox_height(), ctx);
        sec.spacing(16.0);

        sec.widget_full(&mut state.screensaver_style_menu, cce_ui::layout::dropdown_height(), ctx);
        sec.spacing(24.0);

        let btn_h = 32.0;
        let btn_y = sec.ay();
        let cols = sec.row_layout(1, 0.0);
        if let Some(&(x, w)) = cols.first() {
            sec.button(
                "Preview Screensaver",
                x,
                btn_y,
                w,
                btn_h,
                BTN_BG,
                BTN_HOVER,
                WHITE,
                crate::app::AppAction::Display(DisplayMessage::StartScreensaverPreview),
            );
        }
        sec.spacing(12.0);
    });

    final_pc
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

            let enable_hover = state.screensaver_enable_toggle.hovered();
            let lock_hover = state.screensaver_lock_screen_toggle.hovered();
            let timeout_hover = state.screensaver_timeout_spinbox.hovered();
            let style_hover = state.screensaver_style_menu.hovered();
            let brightness_slider_hover = state.brightness_slider.hovered();
            
            *state = new;
            state.night_light_label.set_hovered(was_nl_hovered);
            state.brightness_slider.set_hovered(brightness_slider_hover);
            
            state.screensaver_enable_toggle.set_hovered(enable_hover);
            state.screensaver_lock_screen_toggle.set_hovered(lock_hover);
            state.screensaver_timeout_spinbox.set_hovered(timeout_hover);
            state.screensaver_style_menu.set_hovered(style_hover);

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
            state.brightness_slider.set_value(pct as f32 / 100.0);
        }
        DisplayMessage::ToggleScreensaverEnable => {
            state.screensaver_enable = !state.screensaver_enable;
            write_config_value("enable", &state.screensaver_enable.to_string());
        }
        DisplayMessage::ToggleScreensaverLockScreen => {
            state.screensaver_lock_screen = !state.screensaver_lock_screen;
            write_config_value("lock_screen", &state.screensaver_lock_screen.to_string());
        }
        DisplayMessage::SetScreensaverTimeout(t) => {
            state.screensaver_timeout = t;
            write_config_value("timeout", &state.screensaver_timeout.to_string());
        }
        DisplayMessage::SetScreensaverStyle(idx) => {
            state.screensaver_style_menu.selected = idx;
            let val = match idx {
                0 => "blank",
                1 => "starfield",
                2 => "matrix",
                _ => "starfield",
            };
            state.screensaver_style = val.to_string();
            write_config_value("style", &format!("\"{}\"", val));
        }
        DisplayMessage::StartScreensaverPreview => {
            let style_flag = match state.screensaver_style_menu.selected {
                0 => "blank",
                1 => "starfield",
                2 => "matrix",
                _ => "starfield",
            };
            
            // Spawn screensaver tool from PATH or local directory
            let mut cmd = std::process::Command::new("/home/lsgalante/Dropbox/Clear/cce-screenaver/target/debug/cce-screenaver");
            cmd.arg(style_flag);
            if cce_ui::process::spawn_detached(cmd).is_err() {
                let mut cmd_fallback = std::process::Command::new("cce-screenaver");
                cmd_fallback.arg(style_flag);
                let _ = cce_ui::process::spawn_detached(cmd_fallback);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_layout_grid() {
        let mut state = DisplayState::default();
        let mut layout = cce_ui::layout::AdaptiveGrid::new(260.0, 20.0);
        let pc = view(&mut state, 10.0, 20.0, 800.0, 600.0, &mut layout, &mut cce_ui::context::UiContext::new());
        assert!(!pc.rects.is_empty() || !pc.texts.is_empty());
    }
}

