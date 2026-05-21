use std::fs;
use std::io::Write;

use crate::app::{AppAction, PageContent};
use clear_ui::layout::{render_widget, Section};
use clear_ui::widget::{Spinbox, Toggle};

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
    pub rate_spinbox: Spinbox,
    pub delay_spinbox: Spinbox,
    pub tap_toggle: Toggle,
    pub keybinds: Vec<Keybind>,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            tap_to_click: false,
            repeat_rate: 50,
            repeat_delay: 300,
            rate_spinbox: Spinbox::new(50, 1, 100, 1).with_label("Repeat Rate"),
            delay_spinbox: Spinbox::new(300, 100, 2000, 10).with_label("Repeat Delay"),
        tap_toggle: Toggle::new().with_label("Tap to Click"),
            keybinds: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum InputMessage {
    ToggleTapToClick,
    ApplyRepeat,
    Refreshed(InputState),
}

pub fn read_input_config() -> InputState {
    let content = fs::read_to_string(CONFIG_PATH).unwrap_or_default();
    let rate = parse_u16_key(&content, "rate", 50);
    let delay = parse_u16_key(&content, "delay", 300);
    let tap = parse_bool_from(&content, "tap_to_click");
    InputState {
        tap_to_click: tap,
        repeat_rate: rate,
        repeat_delay: delay,
        rate_spinbox: Spinbox::new(rate as i32, 1, 100, 1).with_label("Repeat Rate"),
        delay_spinbox: Spinbox::new(delay as i32, 100, 2000, 10).with_label("Repeat Delay"),
        tap_toggle: Toggle::new().with_label("Tap to Click"),
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
const BTN_HOVER: [f32; 4] = [0.25, 0.30, 0.26, 1.0];
const TOGGLE_ON: [f32; 4] = [0.16, 0.41, 0.18, 1.0];
const TOGGLE_OFF: [f32; 4] = [0.16, 0.16, 0.24, 1.0];
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

pub fn view(state: &mut InputState, cx: f32, cy: f32, cw: f32, _ch: f32) -> PageContent {
    let mut pc = PageContent::new();
    let mut y = cy + 12.0;

    // ── Touchpad ──
    let mut sec = Section::new(&mut pc, cx, y, cw, "Touchpad");

    let yt = sec.ay();
    let toggle_w = 48.0;
    let toggle_h = 24.0;
    state.tap_toggle.set_toggled(state.tap_to_click);
    render_widget(&mut pc, &mut state.tap_toggle, sec.ax(100.0), yt, toggle_w, toggle_h);
    sec.content_y += toggle_h + 12.0;
    y = sec.finish(&mut pc);

    // ── Keyboard ──
    let mut sec = Section::new(&mut pc, cx, y, cw, "Keyboard");

    sec.widget(&mut pc, &mut state.rate_spinbox, 14.0, 200.0, 26.0);
    sec.spacing(8.0);

    sec.widget(&mut pc, &mut state.delay_spinbox, 14.0, 200.0, 26.0);
    sec.spacing(8.0);
    y = sec.finish(&mut pc);

    // ── Keybindings ──
    let mut sec = Section::new(&mut pc, cx, y, cw, "Keyboard Bindings");

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
        sec.text(&mut pc, &binding, 14.0, 0.0, 12.0, TEXT_FG);
        let label_w = cw - 200.0;
        sec.text(&mut pc, &action_label, 14.0 + label_w.min(180.0), 0.0, 12.0, TEXT_DIM);
        sec.spacing(18.0);
    }
    sec.finish(&mut pc);

    pc
}

pub fn update(state: &mut InputState, msg: InputMessage) {
    match msg {
        InputMessage::ToggleTapToClick => {
            state.tap_to_click = !state.tap_to_click;
            write_tap_to_click(state.tap_to_click);
        }
        InputMessage::ApplyRepeat => {
            let rate = state.rate_spinbox.value.max(1).min(100) as u16;
            let delay = state.delay_spinbox.value.max(100).min(2000) as u16;
            state.repeat_rate = rate;
            state.repeat_delay = delay;
            apply_repeat_config(rate, delay);
        }
        InputMessage::Refreshed(new) => { *state = new; }
    }
}
