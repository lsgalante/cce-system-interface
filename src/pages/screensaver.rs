use std::fs;
use crate::app::{AppAction, PageContent};
use clear_ui::layout::Section;
use clear_ui::widget::{Toggle, Spinbox, Dropdown};

const CONFIG_PATH: &str = "/home/lsgalante/.config/ccec/config.toml";

#[derive(Debug, Clone)]
pub struct ScreensaverState {
    pub enable: bool,
    pub enable_toggle: Toggle,
    pub timeout: i32,
    pub timeout_spinbox: Spinbox,
    pub lock_screen: bool,
    pub lock_screen_toggle: Toggle,
    pub style: String,
    pub style_menu: Dropdown,
}

impl Default for ScreensaverState {
    fn default() -> Self {
        Self {
            enable: true,
            enable_toggle: Toggle::new().with_label("Enable Screensaver"),
            timeout: 10,
            timeout_spinbox: Spinbox::new(10, 1, 120, 1)
                .with_label("Screensaver Timeout")
                .with_unit("m"),
            lock_screen: true,
            lock_screen_toggle: Toggle::new().with_label("Lock Screen on Activation"),
            style: "starfield".to_string(),
            style_menu: Dropdown::new(
                vec!["Blank".to_string(), "Starfield".to_string(), "Matrix Rain".to_string()],
                1,
            ).with_label("Screensaver Style"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ScreensaverMessage {
    ToggleEnable,
    ToggleLockScreen,
    SetTimeout(i32),
    SetStyle(usize),
    StartPreview,
    Refreshed(ScreensaverState),
}

pub fn read_screensaver_config() -> ScreensaverState {
    let content = fs::read_to_string(CONFIG_PATH).unwrap_or_default();
    let enable = parse_screensaver_enable(&content);
    let timeout = parse_screensaver_timeout(&content);
    let lock_screen = parse_screensaver_lock_screen(&content);
    let style = parse_screensaver_style(&content);

    let style_idx = match style.to_lowercase().as_str() {
        "blank" => 0,
        "starfield" => 1,
        "matrix" => 2,
        _ => 1, // default to Starfield
    };

    ScreensaverState {
        enable,
        enable_toggle: Toggle::new().with_label("Enable Screensaver"),
        timeout,
        timeout_spinbox: Spinbox::new(timeout, 1, 120, 1)
            .with_label("Screensaver Timeout")
            .with_unit("m"),
        lock_screen,
        lock_screen_toggle: Toggle::new().with_label("Lock Screen on Activation"),
        style: style.clone(),
        style_menu: Dropdown::new(
            vec!["Blank".to_string(), "Starfield".to_string(), "Matrix Rain".to_string()],
            style_idx,
        ).with_label("Screensaver Style"),
    }
}

fn parse_screensaver_enable(content: &str) -> bool {
    let mut in_section = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[screensaver]" {
            in_section = true;
            continue;
        }
        if trimmed.starts_with('[') && in_section {
            break;
        }
        if in_section && trimmed.starts_with("enable") {
            if let Some(val) = trimmed.split('=').nth(1) {
                return val.trim() == "true";
            }
        }
    }
    true // default to true
}

fn parse_screensaver_lock_screen(content: &str) -> bool {
    let mut in_section = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[screensaver]" {
            in_section = true;
            continue;
        }
        if trimmed.starts_with('[') && in_section {
            break;
        }
        if in_section && trimmed.starts_with("lock_screen") {
            if let Some(val) = trimmed.split('=').nth(1) {
                return val.trim() == "true";
            }
        }
    }
    true // default to true
}

fn parse_screensaver_timeout(content: &str) -> i32 {
    let mut in_section = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[screensaver]" {
            in_section = true;
            continue;
        }
        if trimmed.starts_with('[') && in_section {
            break;
        }
        if in_section && trimmed.starts_with("timeout") {
            if let Some(val) = trimmed.split('=').nth(1) {
                if let Ok(t) = val.trim().parse::<i32>() {
                    return t;
                }
            }
        }
    }
    10 // default to 10 minutes
}

fn parse_screensaver_style(content: &str) -> String {
    let mut in_section = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[screensaver]" {
            in_section = true;
            continue;
        }
        if trimmed.starts_with('[') && in_section {
            break;
        }
        if in_section && trimmed.starts_with("style") {
            if let Some(val) = trimmed.split('=').nth(1) {
                return val.trim().trim_matches('"').to_string();
            }
        }
    }
    "starfield".to_string() // default to starfield
}

fn write_config_value(key: &str, value: &str) {
    let content = fs::read_to_string(CONFIG_PATH).unwrap_or_default();
    let new_line = format!("{} = {}", key, value);

    let mut found = false;
    let mut updated_lines = Vec::new();
    let mut in_section = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[screensaver]" {
            in_section = true;
            updated_lines.push(line.to_string());
            continue;
        }
        if trimmed.starts_with('[') && in_section {
            in_section = false;
        }
        if in_section && trimmed.starts_with(key) {
            found = true;
            updated_lines.push(new_line.clone());
        } else {
            updated_lines.push(line.to_string());
        }
    }

    let mut updated = updated_lines.join("\n");

    if !found {
        let mut result = String::new();
        let has_section = content.lines().any(|l| l.trim() == "[screensaver]");
        if has_section {
            let mut in_section = false;
            let mut inserted = false;
            for line in updated.lines() {
                if line.trim() == "[screensaver]" {
                    in_section = true;
                    result.push_str(line);
                    result.push('\n');
                    continue;
                }
                if line.trim().starts_with('[') && in_section {
                    if !inserted {
                        result.push_str(&new_line);
                        result.push('\n');
                        inserted = true;
                    }
                    in_section = false;
                }
                result.push_str(line);
                result.push('\n');
            }
            if !inserted {
                result.push_str(&new_line);
                result.push('\n');
            }
            updated = result;
        } else {
            updated.push_str("\n[screensaver]\n");
            updated.push_str(&new_line);
            updated.push_str("\n");
        }
    }
    let _ = fs::write(CONFIG_PATH, updated);
}

const BTN_BG: [f32; 4] = [0.20, 0.40, 0.65, 1.0];
const BTN_HOVER: [f32; 4] = [0.28, 0.50, 0.78, 1.0];
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

pub fn view(state: &mut ScreensaverState, cx: f32, cy: f32, cw: f32, _ch: f32) -> PageContent {
    let mut pc = PageContent::new();
    let y = cy + 12.0;

    let mut sec = Section::new(&mut pc, cx, y, cw, "Screensaver Settings");

    let toggle_w = 48.0;
    let toggle_h = 24.0;
    
    state.enable_toggle.set_toggled(state.enable);
    sec.widget(&mut pc, &mut state.enable_toggle, 14.0, toggle_w, toggle_h);
    sec.spacing(8.0);

    state.lock_screen_toggle.set_toggled(state.lock_screen);
    sec.widget(&mut pc, &mut state.lock_screen_toggle, 14.0, toggle_w, toggle_h);
    sec.spacing(16.0);

    state.timeout_spinbox.value = state.timeout;
    sec.widget(&mut pc, &mut state.timeout_spinbox, 14.0, 200.0, 26.0);
    sec.spacing(16.0);

    sec.widget(&mut pc, &mut state.style_menu, 14.0, 200.0, 26.0);
    sec.spacing(24.0);

    let btn_w = 160.0;
    let btn_h = 32.0;
    let btn_y = sec.ay();
    sec.row(1, 0.0, btn_h, |_, x, _| {
        pc.button(
            "Preview Screensaver",
            x,
            btn_y,
            btn_w,
            btn_h,
            BTN_BG,
            BTN_HOVER,
            WHITE,
            AppAction::Screensaver(ScreensaverMessage::StartPreview),
        );
    });
    sec.spacing(12.0);
    sec.finish(&mut pc);

    pc
}

pub fn update(state: &mut ScreensaverState, msg: ScreensaverMessage) {
    match msg {
        ScreensaverMessage::ToggleEnable => {
            state.enable = !state.enable;
            write_config_value("enable", &state.enable.to_string());
        }
        ScreensaverMessage::ToggleLockScreen => {
            state.lock_screen = !state.lock_screen;
            write_config_value("lock_screen", &state.lock_screen.to_string());
        }
        ScreensaverMessage::SetTimeout(t) => {
            state.timeout = t;
            write_config_value("timeout", &state.timeout.to_string());
        }
        ScreensaverMessage::SetStyle(idx) => {
            state.style_menu.selected = idx;
            let val = match idx {
                0 => "blank",
                1 => "starfield",
                2 => "matrix",
                _ => "starfield",
            };
            state.style = val.to_string();
            write_config_value("style", &format!("\"{}\"", val));
        }
        ScreensaverMessage::StartPreview => {
            let style_flag = match state.style_menu.selected {
                0 => "blank",
                1 => "starfield",
                2 => "matrix",
                _ => "starfield",
            };
            
            // Spawn screensaver tool from PATH or local directory
            std::process::Command::new("/home/lsgalante/Dropbox/Clear/cce-screenaver/target/debug/cce-screenaver")
                .arg(style_flag)
                .spawn()
                .or_else(|_| {
                    std::process::Command::new("cce-screenaver")
                        .arg(style_flag)
                        .spawn()
                })
                .ok();
        }
        ScreensaverMessage::Refreshed(new) => {
            *state = new;
        }
    }
}
