use std::fs;
use std::io::Write;

use crate::app::{AppAction, PageContent};

const CONFIG_PATH: &str = "/home/lsgalante/.config/clearwm/config.toml";
const CLEARWM_SOCK: &str = "/tmp/clearwm.sock";

#[derive(Debug, Clone)]
pub struct Keybind {
    pub mods: String,
    pub key: String,
    pub action: String,
    pub command: String,
}

#[derive(Debug, Clone)]
pub struct InputState {
    pub tap_to_click: bool,
    pub repeat_rate: u16,
    pub repeat_delay: u16,
    pub keybinds: Vec<Keybind>,
}

impl Default for InputState {
    fn default() -> Self {
        Self { tap_to_click: false, repeat_rate: 50, repeat_delay: 300, keybinds: Vec::new() }
    }
}

#[derive(Debug, Clone)]
pub enum InputMessage {
    ToggleTapToClick,
    RepeatRateDown,
    RepeatRateUp,
    RepeatDelayDown,
    RepeatDelayUp,
    Refreshed(InputState),
}

pub fn read_input_config() -> InputState {
    let content = fs::read_to_string(CONFIG_PATH).unwrap_or_default();
    InputState {
        tap_to_click: parse_bool_from(&content, "tap_to_click"),
        repeat_rate: parse_u16_key(&content, "rate", 50),
        repeat_delay: parse_u16_key(&content, "delay", 300),
        keybinds: parse_keybinds(&content),
    }
}

fn parse_bool_from(content: &str, key: &str) -> bool {
    content.lines().find(|l| l.trim().starts_with(key))
        .and_then(|l| l.split('=').nth(1))
        .map(|v| v.trim() == "true")
        .unwrap_or(false)
}

fn parse_u16_key(content: &str, key: &str, default: u16) -> u16 {
    content.lines().find(|l| l.trim().starts_with(key))
        .and_then(|l| l.split('=').nth(1))
        .and_then(|v| v.trim().parse::<u16>().ok())
        .unwrap_or(default)
}

fn parse_keybinds(content: &str) -> Vec<Keybind> {
    let mut keybinds = Vec::new();
    let mut current: Option<Keybind> = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[[keybind]]" {
            if let Some(kb) = current.take() { keybinds.push(kb); }
            current = Some(Keybind {
                mods: String::new(), key: String::new(),
                action: String::new(), command: String::new(),
            });
            continue;
        }
        if let Some(ref mut kb) = current {
            let set = |rest: &str| rest.trim_start_matches(|c: char| c == ' ' || c == '=').trim_matches('"').to_string();
            if let Some(rest) = trimmed.strip_prefix("mods") { kb.mods = set(rest); }
            else if let Some(rest) = trimmed.strip_prefix("key") { kb.key = set(rest); }
            else if let Some(rest) = trimmed.strip_prefix("action") { kb.action = set(rest); }
            else if let Some(rest) = trimmed.strip_prefix("command") { kb.command = set(rest); }
        }
    }
    if let Some(kb) = current.take() { keybinds.push(kb); }
    keybinds
}

fn send_ipc_command(cmd: &str) {
    if let Ok(mut stream) = std::os::unix::net::UnixStream::connect(CLEARWM_SOCK) {
        let _ = stream.write_all(format!("{}\n", cmd).as_bytes());
    }
}

fn write_config_value(key: &str, value: &str) {
    let content = fs::read_to_string(CONFIG_PATH).unwrap_or_default();
    let new_line = format!("{} = {}", key, value);

    let mut found = false;
    let updated: String = content.lines()
        .map(|line| {
            if line.trim().starts_with(key) { found = true; new_line.clone() }
            else { line.to_string() }
        })
        .collect::<Vec<_>>()
        .join("\n");

    if !found {
        let section = if key == "tap_to_click" { "[input]" } else { "[repeat]" };
        let mut result = String::new();
        let mut in_section = false;
        let mut inserted = false;
        for line in updated.lines() {
            if line.trim() == section { in_section = true; }
            else if line.trim().starts_with('[') && in_section {
                if !inserted { result.push_str(&new_line); result.push('\n'); inserted = true; }
                in_section = false;
            }
            result.push_str(line);
            result.push('\n');
        }
        if !inserted {
            if !in_section { result.push('\n'); result.push_str(section); result.push('\n'); }
            result.push_str(&new_line);
            result.push('\n');
        }
        let _ = fs::write(CONFIG_PATH, result);
    } else {
        let _ = fs::write(CONFIG_PATH, updated);
    }
}

fn write_tap_to_click(enabled: bool) {
    write_config_value("tap_to_click", &enabled.to_string());
    send_ipc_command(&format!("input tap-to-click {}", enabled));
}

fn apply_repeat_config(rate: u16, delay: u16) {
    write_config_value("rate", &rate.to_string());
    write_config_value("delay", &delay.to_string());
    send_ipc_command(&format!("repeat rate {}", rate));
    send_ipc_command(&format!("repeat delay {}", delay));
}

const TEXT_FG: [f32; 4] = [0.83, 0.83, 0.83, 1.0];
const TEXT_DIM: [f32; 4] = [0.53, 0.53, 0.60, 1.0];
const ACCENT: [f32; 4] = [0.36, 0.56, 0.38, 1.0];
const BTN_ACTIVE: [f32; 4] = [0.20, 0.40, 0.22, 1.0];
const BTN_INACTIVE: [f32; 4] = [0.13, 0.18, 0.14, 1.0];
const BTN_HOVER: [f32; 4] = [0.25, 0.30, 0.26, 1.0];
const TOGGLE_ON: [f32; 4] = [0.16, 0.41, 0.18, 1.0];
const TOGGLE_OFF: [f32; 4] = [0.16, 0.16, 0.24, 1.0];
const SECTION_BORDER: [f32; 4] = [0.18, 0.18, 0.27, 1.0];
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

pub fn view(state: &InputState, cx: f32, cy: f32, cw: f32, _ch: f32) -> PageContent {
    let mut pc = PageContent::new();
    let mut y = cy + 12.0;

    // ── Touchpad ──
    pc.text("Touchpad", cx + 12.0, y, 14.0, TEXT_FG);
    y += 22.0;

    let tap_bg = if state.tap_to_click { TOGGLE_ON } else { TOGGLE_OFF };
    let tap_label = if state.tap_to_click { "Tap to Click: ON" } else { "Tap to Click: OFF" };
    pc.text(tap_label, cx + 14.0, y + 6.0, 13.0, if state.tap_to_click { ACCENT } else { TEXT_DIM });
    let btn_w = (cw - 32.0).min(100.0);
    pc.button(if state.tap_to_click { "ON" } else { "OFF" }, cx + cw - btn_w - 14.0, y, btn_w, 28.0,
        tap_bg, BTN_HOVER, WHITE,
        AppAction::Input(InputMessage::ToggleTapToClick));
    y += 36.0;

    // ── Keyboard ──
    pc.rect(SECTION_BORDER, cx + 8.0, y, cw - 16.0, 1.0);
    y += 8.0;
    pc.text("Keyboard", cx + 12.0, y, 14.0, TEXT_FG);
    y += 22.0;

    // Repeat Rate
    pc.text(&format!("Repeat Rate: {} /sec", state.repeat_rate), cx + 14.0, y, 12.0, TEXT_DIM);
    y += 18.0;
    pc.button("-1", cx + 14.0, y, 36.0, 28.0, BTN_INACTIVE, BTN_HOVER, WHITE,
        AppAction::Input(InputMessage::RepeatRateDown));
    pc.text(&format!(" {} ", state.repeat_rate), cx + 58.0, y + 7.0, 13.0, TEXT_FG);
    pc.button("+1", cx + 14.0 + 36.0 + 8.0, y, 36.0, 28.0, BTN_ACTIVE, BTN_HOVER, WHITE,
        AppAction::Input(InputMessage::RepeatRateUp));
    y += 34.0;

    // Repeat Delay
    pc.text(&format!("Repeat Delay: {}ms", state.repeat_delay), cx + 14.0, y, 12.0, TEXT_DIM);
    y += 18.0;
    pc.button("-10", cx + 14.0, y, 36.0, 28.0, BTN_INACTIVE, BTN_HOVER, WHITE,
        AppAction::Input(InputMessage::RepeatDelayDown));
    pc.text(&format!(" {}ms ", state.repeat_delay), cx + 58.0, y + 7.0, 13.0, TEXT_FG);
    pc.button("+10", cx + 14.0 + 36.0 + 8.0, y, 36.0, 28.0, BTN_ACTIVE, BTN_HOVER, WHITE,
        AppAction::Input(InputMessage::RepeatDelayUp));
    y += 40.0;

    // ── Keybindings ──
    pc.rect(SECTION_BORDER, cx + 8.0, y, cw - 16.0, 1.0);
    y += 8.0;
    pc.text("Keyboard Bindings", cx + 12.0, y, 14.0, TEXT_FG);
    y += 22.0;

    for kb in &state.keybinds {
        let binding = if kb.mods.is_empty() {
            kb.key.clone()
        } else {
            format!("{}+{}", kb.mods, kb.key)
        };
        let action_label = if kb.command.is_empty() {
            kb.action.clone()
        } else {
            format!("{}: {}", kb.action, kb.command)
        };
        pc.text(&binding, cx + 14.0, y, 12.0, TEXT_FG);
        let label_w = cw - 200.0;
        pc.text(&action_label, cx + 14.0 + label_w.min(180.0), y, 12.0, TEXT_DIM);
        y += 18.0;
    }

    pc
}

pub fn update(state: &mut InputState, msg: InputMessage) {
    match msg {
        InputMessage::ToggleTapToClick => {
            state.tap_to_click = !state.tap_to_click;
            write_tap_to_click(state.tap_to_click);
        }
        InputMessage::RepeatRateDown => {
            if state.repeat_rate > 1 { state.repeat_rate -= 1; }
            apply_repeat_config(state.repeat_rate, state.repeat_delay);
        }
        InputMessage::RepeatRateUp => {
            if state.repeat_rate < 100 { state.repeat_rate += 1; }
            apply_repeat_config(state.repeat_rate, state.repeat_delay);
        }
        InputMessage::RepeatDelayDown => {
            if state.repeat_delay > 100 { state.repeat_delay -= 10; }
            apply_repeat_config(state.repeat_rate, state.repeat_delay);
        }
        InputMessage::RepeatDelayUp => {
            if state.repeat_delay < 2000 { state.repeat_delay += 10; }
            apply_repeat_config(state.repeat_rate, state.repeat_delay);
        }
        InputMessage::Refreshed(new) => { *state = new; }
    }
}
