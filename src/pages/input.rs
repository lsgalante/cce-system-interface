use std::fs;
use std::io::Write;

use crate::app::PageContent;
use clear_ui::layout::Section;
use clear_ui::widget::{Dropdown, Spinbox, Toggle, Widget, Finger, Trackpad};

const CONFIG_PATH: &str = "/home/lsgalante/.config/ccec/config.toml";

fn get_socket_path() -> String {
    match std::env::var("WAYLAND_DISPLAY") {
        Ok(display) => format!("/tmp/ccec-{}.sock", display),
        Err(_) => "/tmp/ccec.sock".to_string(),
    }
}

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
    pub fingers: Vec<Finger>,
    pub trackpad: Trackpad,

    // Inertial settings
    pub inertial_scroll: bool,
    pub scroll_friction: u16,
    pub inertial_pointer: bool,
    pub pointer_friction: u16,
    pub inertial_trackpad: bool,
    pub trackpad_friction: u16,

    pub scroll_toggle: Toggle,
    pub scroll_friction_spinbox: Spinbox,
    pub pointer_toggle: Toggle,
    pub pointer_friction_spinbox: Spinbox,
    pub trackpad_toggle: Toggle,
    pub trackpad_friction_spinbox: Spinbox,

    // Trackpoint settings
    pub dwtp: bool,
    pub trackpoint_accel_speed: f32,
    pub trackpoint_accel_profile: String,

    pub dwtp_toggle: Toggle,
    pub trackpoint_accel_speed_spinbox: Spinbox,
    pub trackpoint_accel_profile_menu: Dropdown,

    // Cursor settings
    pub cursor_theme: String,
    pub cursor_size: u32,
    pub cursor_theme_menu: Dropdown,
    pub cursor_size_spinbox: Spinbox,
    pub cursor_themes: Vec<String>,
}

fn scan_cursor_themes() -> Vec<String> {
    let mut themes = vec!["default".to_string()];
    let paths = [
        "/usr/share/icons",
        "/home/lsgalante/.icons",
        "/home/lsgalante/.local/share/icons",
    ];
    for base_path in &paths {
        if let Ok(entries) = std::fs::read_dir(base_path) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_dir() {
                        let path = entry.path();
                        if path.join("cursors").is_dir() {
                            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                                let name_str = name.to_string();
                                if !themes.contains(&name_str) {
                                    themes.push(name_str);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    themes.sort();
    themes
}

impl Default for InputState {
    fn default() -> Self {
        let cursor_themes = vec!["default".to_string()];
        Self {
            tap_to_click: false,
            repeat_rate: 50,
            repeat_delay: 300,
            rate_spinbox: Spinbox::new(50, 1, 100, 1).with_label("Repeat Rate").with_unit("ms"),
            delay_spinbox: Spinbox::new(300, 100, 2000, 10).with_label("Repeat Delay").with_unit("ms"),
            tap_toggle: Toggle::new().with_label("Tap to Click"),
            keybinds: Vec::new(),
            fingers: Vec::new(),
            trackpad: Trackpad::new(),

            inertial_scroll: true,
            scroll_friction: 90,
            inertial_pointer: false,
            pointer_friction: 95,
            inertial_trackpad: false,
            trackpad_friction: 95,

            scroll_toggle: Toggle::new().with_label("Inertial Scroll"),
            scroll_friction_spinbox: Spinbox::new(90, 50, 99, 1).with_label("Scroll Friction").with_unit("%"),
            pointer_toggle: Toggle::new().with_label("Inertial Pointer (Trackpoint)"),
            pointer_friction_spinbox: Spinbox::new(95, 50, 99, 1).with_label("Pointer Friction").with_unit("%"),
            trackpad_toggle: Toggle::new().with_label("Inertial Pointer (Trackpad)"),
            trackpad_friction_spinbox: Spinbox::new(95, 50, 99, 1).with_label("Trackpad Friction").with_unit("%"),

            dwtp: true,
            trackpoint_accel_speed: 0.5,
            trackpoint_accel_profile: "flat".to_string(),

            dwtp_toggle: Toggle::new().with_label("Disable While Trackpointing"),
            trackpoint_accel_speed_spinbox: Spinbox::new(5, -10, 10, 1).with_label("Acceleration Speed").with_decimals(1),
            trackpoint_accel_profile_menu: Dropdown::new(vec!["flat".to_string(), "adaptive".to_string()], 0).with_label("Acceleration Profile"),

            // Cursor settings defaults
            cursor_theme: "default".to_string(),
            cursor_size: 24,
            cursor_theme_menu: Dropdown::new(cursor_themes.clone(), 0).with_label("Cursor Theme"),
            cursor_size_spinbox: Spinbox::new(24, 16, 64, 4).with_label("Cursor Size").with_unit("px"),
            cursor_themes,
        }
    }
}

impl InputState {
    pub fn is_over_trackpad(&self, lx: f32, ly: f32) -> bool {
        self.trackpad.hit_test(lx, ly)
    }
}

#[derive(Debug, Clone)]
pub enum InputMessage {
    ToggleTapToClick,
    ApplyRepeat,
    Refreshed(InputState),
    UpdateFingers(Vec<Finger>),

    ToggleInertialScroll,
    ApplyScrollFriction,
    ToggleInertialPointer,
    ApplyPointerFriction,
    ToggleInertialTrackpad,
    ApplyTrackpadFriction,

    ToggleDwtp,
    ApplyTrackpointAccelSpeed,
    ApplyTrackpointAccelProfile(usize),

    ApplyCursorTheme(usize),
    ApplyCursorSize,
}

pub fn read_input_config() -> InputState {
    let content = fs::read_to_string(CONFIG_PATH).unwrap_or_default();
    let rate = parse_u16_key(&content, "rate", 50);
    let delay = parse_u16_key(&content, "delay", 300);
    let tap = parse_bool_from(&content, "tap_to_click");

    let inertial_scroll = parse_bool_from_default(&content, "inertial_scroll", true);
    let scroll_friction = parse_u16_key(&content, "scroll_friction", 90);
    let inertial_pointer = parse_bool_from_default(&content, "inertial_pointer", false);
    let pointer_friction = parse_u16_key(&content, "pointer_friction", 95);
    let inertial_trackpad = parse_bool_from_default(&content, "inertial_trackpad", false);
    let trackpad_friction = parse_u16_key(&content, "trackpad_friction", 95);

    let dwtp = parse_bool_from_default(&content, "dwtp", true);
    let trackpoint_accel_speed = parse_f32_key(&content, "trackpoint_accel_speed", 0.5);
    let trackpoint_accel_profile = parse_string_key(&content, "trackpoint_accel_profile", "flat");

    let speed_val = (trackpoint_accel_speed * 10.0).round() as i32;
    let profile_idx = if trackpoint_accel_profile == "adaptive" { 1 } else { 0 };

    let cursor_theme = parse_string_key(&content, "cursor_theme", "default");
    let cursor_size = parse_u16_key(&content, "cursor_size", 24);
    let cursor_themes = scan_cursor_themes();
    let theme_idx = cursor_themes.iter().position(|t| t == &cursor_theme).unwrap_or(0);

    InputState {
        tap_to_click: tap,
        repeat_rate: rate,
        repeat_delay: delay,
        rate_spinbox: Spinbox::new(rate as i32, 1, 100, 1).with_label("Repeat Rate").with_unit("ms"),
        delay_spinbox: Spinbox::new(delay as i32, 100, 2000, 10).with_label("Repeat Delay").with_unit("ms"),
        tap_toggle: Toggle::new().with_label("Tap to Click"),
        keybinds: parse_keybinds(&content),
        fingers: Vec::new(),
        trackpad: Trackpad::new(),

        inertial_scroll,
        scroll_friction,
        inertial_pointer,
        pointer_friction,
        inertial_trackpad,
        trackpad_friction,

        scroll_toggle: Toggle::new().with_label("Inertial Scroll"),
        scroll_friction_spinbox: Spinbox::new(scroll_friction as i32, 50, 99, 1).with_label("Scroll Friction").with_unit("%"),
        pointer_toggle: Toggle::new().with_label("Inertial Pointer (Trackpoint)"),
        pointer_friction_spinbox: Spinbox::new(pointer_friction as i32, 50, 99, 1).with_label("Pointer Friction").with_unit("%"),
        trackpad_toggle: Toggle::new().with_label("Inertial Pointer (Trackpad)"),
        trackpad_friction_spinbox: Spinbox::new(trackpad_friction as i32, 50, 99, 1).with_label("Trackpad Friction").with_unit("%"),

        dwtp,
        trackpoint_accel_speed,
        trackpoint_accel_profile,

        dwtp_toggle: Toggle::new().with_label("Disable While Trackpointing"),
        trackpoint_accel_speed_spinbox: Spinbox::new(speed_val, -10, 10, 1).with_label("Acceleration Speed").with_decimals(1),
        trackpoint_accel_profile_menu: Dropdown::new(vec!["flat".to_string(), "adaptive".to_string()], profile_idx).with_label("Acceleration Profile"),

        // Cursor settings
        cursor_theme: cursor_theme.clone(),
        cursor_size: cursor_size as u32,
        cursor_theme_menu: Dropdown::new(cursor_themes.clone(), theme_idx).with_label("Cursor Theme"),
        cursor_size_spinbox: Spinbox::new(cursor_size as i32, 16, 64, 4).with_label("Cursor Size").with_unit("px"),
        cursor_themes,
    }
}

fn parse_bool_from(content: &str, key: &str) -> bool {
    content.lines().find(|l| l.trim().starts_with(key))
        .and_then(|l| l.split('=').nth(1))
        .map(|v| v.trim() == "true")
        .unwrap_or(false)
}

fn parse_bool_from_default(content: &str, key: &str, default: bool) -> bool {
    content.lines().find(|l| l.trim().starts_with(key))
        .and_then(|l| l.split('=').nth(1))
        .map(|v| v.trim() == "true")
        .unwrap_or(default)
}

fn parse_u16_key(content: &str, key: &str, default: u16) -> u16 {
    content.lines().find(|l| l.trim().starts_with(key))
        .and_then(|l| l.split('=').nth(1))
        .and_then(|v| v.trim().parse::<u16>().ok())
        .unwrap_or(default)
}

fn parse_f32_key(content: &str, key: &str, default: f32) -> f32 {
    content.lines().find(|l| l.trim().starts_with(key))
        .and_then(|l| l.split('=').nth(1))
        .and_then(|v| v.trim().parse::<f32>().ok())
        .unwrap_or(default)
}

fn parse_string_key(content: &str, key: &str, default: &str) -> String {
    content.lines().find(|l| l.trim().starts_with(key))
        .and_then(|l| l.split('=').nth(1))
        .map(|v| v.trim().trim_matches('"').to_string())
        .unwrap_or_else(|| default.to_string())
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
    if let Ok(mut stream) = std::os::unix::net::UnixStream::connect(get_socket_path()) {
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
        let section = if key == "tap_to_click" || key == "dwtp"
                || key == "trackpoint_accel_speed" || key == "trackpoint_accel_profile"
                || key == "cursor_theme" || key == "cursor_size" {
            "[input]"
        } else if key == "inertial_scroll" || key == "scroll_friction"
               || key == "inertial_pointer" || key == "pointer_friction"
               || key == "inertial_trackpad" || key == "trackpad_friction" {
            "[inertial]"
        } else {
            "[repeat]"
        };

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
}

fn apply_repeat_config(rate: u16, delay: u16) {
    write_config_value("rate", &rate.to_string());
    write_config_value("delay", &delay.to_string());
    send_ipc_command(&format!("repeat rate {}", rate));
    send_ipc_command(&format!("repeat delay {}", delay));
}

const TEXT_FG: [f32; 4] = [0.83, 0.83, 0.83, 1.0];
const TEXT_DIM: [f32; 4] = [0.53, 0.53, 0.60, 1.0];

pub fn view(state: &mut InputState, cx: f32, cy: f32, cw: f32, _ch: f32, sec_focused: &[bool]) -> PageContent {
    let mut pc = PageContent::new();
    let mut y = cy + 12.0;

    // ── Touchpad ──
    let mut sec = Section::new(&mut pc, cx, y, cw, "Touchpad");

    let toggle_w = 48.0;
    let toggle_h = 24.0;
    state.tap_toggle.set_toggled(state.tap_to_click);
    sec.widget(&mut pc, &mut state.tap_toggle, 14.0, toggle_w, toggle_h);
    sec.spacing(8.0);

    // Built-in trackpad visualizer widget
    let pad_w = 280.0;
    let pad_h = 140.0;
    state.trackpad.set_fingers(state.fingers.clone());
    sec.widget(&mut pc, &mut state.trackpad, 14.0, pad_w, pad_h);
    sec.spacing(12.0);
    y = sec.finish(&mut pc);

    // ── Trackpoint ──
    let mut sec = Section::new(&mut pc, cx, y, cw, "Trackpoint");

    state.dwtp_toggle.set_toggled(state.dwtp);
    sec.widget(&mut pc, &mut state.dwtp_toggle, 14.0, toggle_w, toggle_h);
    sec.spacing(12.0);

    sec.widget(&mut pc, &mut state.trackpoint_accel_speed_spinbox, 14.0, 200.0, 26.0);
    sec.spacing(12.0);

    sec.widget(&mut pc, &mut state.trackpoint_accel_profile_menu, 14.0, 200.0, 26.0);
    sec.spacing(8.0);
    y = sec.finish_focused(&mut pc, sec_focused.get(0).copied().unwrap_or(false));

    // ── Keyboard ──
    let mut sec = Section::new(&mut pc, cx, y, cw, "Keyboard");

    sec.widget(&mut pc, &mut state.rate_spinbox, 14.0, 200.0, 26.0);
    sec.spacing(8.0);

    sec.widget(&mut pc, &mut state.delay_spinbox, 14.0, 200.0, 26.0);
    sec.spacing(8.0);
    y = sec.finish_focused(&mut pc, sec_focused.get(1).copied().unwrap_or(false));

    // ── Cursor ──
    let mut sec = Section::new(&mut pc, cx, y, cw, "Cursor");

    sec.widget(&mut pc, &mut state.cursor_theme_menu, 14.0, 200.0, 26.0);
    sec.spacing(12.0);

    sec.widget(&mut pc, &mut state.cursor_size_spinbox, 14.0, 200.0, 26.0);
    sec.spacing(8.0);

    y = sec.finish_focused(&mut pc, sec_focused.get(2).copied().unwrap_or(false));

    // ── Inertial Input ──
    let mut sec = Section::new(&mut pc, cx, y, cw, "Inertial Input");

    state.scroll_toggle.set_toggled(state.inertial_scroll);
    sec.widget(&mut pc, &mut state.scroll_toggle, 14.0, toggle_w, toggle_h);
    sec.spacing(12.0);

    sec.widget(&mut pc, &mut state.scroll_friction_spinbox, 14.0, 200.0, 26.0);
    sec.spacing(16.0);

    state.pointer_toggle.set_toggled(state.inertial_pointer);
    sec.widget(&mut pc, &mut state.pointer_toggle, 14.0, toggle_w, toggle_h);
    sec.spacing(12.0);

    sec.widget(&mut pc, &mut state.pointer_friction_spinbox, 14.0, 200.0, 26.0);
    sec.spacing(16.0);

    state.trackpad_toggle.set_toggled(state.inertial_trackpad);
    sec.widget(&mut pc, &mut state.trackpad_toggle, 14.0, toggle_w, toggle_h);
    sec.spacing(12.0);

    sec.widget(&mut pc, &mut state.trackpad_friction_spinbox, 14.0, 200.0, 26.0);
    sec.spacing(8.0);

    y = sec.finish_focused(&mut pc, sec_focused.get(3).copied().unwrap_or(false));

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

    state.trackpoint_accel_profile_menu.render_popover(&mut pc);
    state.cursor_theme_menu.render_popover(&mut pc);

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
        InputMessage::ToggleInertialScroll => {
            state.inertial_scroll = !state.inertial_scroll;
            write_config_value("inertial_scroll", &state.inertial_scroll.to_string());
        }
        InputMessage::ApplyScrollFriction => {
            let friction = state.scroll_friction_spinbox.value.max(50).min(99) as u16;
            state.scroll_friction = friction;
            write_config_value("scroll_friction", &friction.to_string());
        }
        InputMessage::ToggleInertialPointer => {
            state.inertial_pointer = !state.inertial_pointer;
            write_config_value("inertial_pointer", &state.inertial_pointer.to_string());
        }
        InputMessage::ApplyPointerFriction => {
            let friction = state.pointer_friction_spinbox.value.max(50).min(99) as u16;
            state.pointer_friction = friction;
            write_config_value("pointer_friction", &friction.to_string());
        }
        InputMessage::ToggleInertialTrackpad => {
            state.inertial_trackpad = !state.inertial_trackpad;
            write_config_value("inertial_trackpad", &state.inertial_trackpad.to_string());
        }
        InputMessage::ApplyTrackpadFriction => {
            let friction = state.trackpad_friction_spinbox.value.max(50).min(99) as u16;
            state.trackpad_friction = friction;
            write_config_value("trackpad_friction", &friction.to_string());
        }
        InputMessage::ToggleDwtp => {
            state.dwtp = !state.dwtp;
            write_config_value("dwtp", &state.dwtp.to_string());
            send_ipc_command(&format!("input dwtp {}", state.dwtp));
        }
        InputMessage::ApplyTrackpointAccelSpeed => {
            let val = state.trackpoint_accel_speed_spinbox.value as f32 / 10.0;
            state.trackpoint_accel_speed = val;
            write_config_value("trackpoint_accel_speed", &val.to_string());
            send_ipc_command(&format!("input trackpoint-accel-speed {}", val));
        }
        InputMessage::ApplyTrackpointAccelProfile(idx) => {
            let profile = if idx == 1 { "adaptive" } else { "flat" };
            state.trackpoint_accel_profile = profile.to_string();
            state.trackpoint_accel_profile_menu.selected = idx;
            write_config_value("trackpoint_accel_profile", &format!("\"{}\"", profile));
            send_ipc_command(&format!("input trackpoint-accel-profile {}", profile));
        }
        InputMessage::ApplyCursorTheme(idx) => {
            if idx < state.cursor_themes.len() {
                let theme = state.cursor_themes[idx].clone();
                state.cursor_theme = theme.clone();
                state.cursor_theme_menu.selected = idx;
                write_config_value("cursor_theme", &format!("\"{}\"", theme));
                send_ipc_command(&format!("input cursor-theme {}", theme));
            }
        }
        InputMessage::ApplyCursorSize => {
            let size = state.cursor_size_spinbox.value.max(16).min(64) as u32;
            state.cursor_size = size;
            write_config_value("cursor_size", &size.to_string());
            send_ipc_command(&format!("input cursor-size {}", size));
        }
        InputMessage::Refreshed(new) => {
            let fingers = state.fingers.clone();
            *state = new;
            state.fingers = fingers;
        }
        InputMessage::UpdateFingers(fingers) => {
            state.fingers = fingers;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_over_trackpad() {
        let mut state = InputState::default();
        state.trackpad.set_rect(100.0, 200.0, 300.0, 150.0);

        // Inside
        assert!(state.is_over_trackpad(150.0, 250.0));
        assert!(state.is_over_trackpad(100.0, 200.0));
        assert!(state.is_over_trackpad(400.0, 350.0));

        // Outside X
        assert!(!state.is_over_trackpad(99.0, 250.0));
        assert!(!state.is_over_trackpad(401.0, 250.0));

        // Outside Y
        assert!(!state.is_over_trackpad(150.0, 199.0));
        assert!(!state.is_over_trackpad(150.0, 351.0));
    }
}

