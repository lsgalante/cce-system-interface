use std::fs;
use std::io::Write;

use crate::app::{AppAction, PageContent};
use clear_ui::layout::Section;
use clear_ui::widget::{Toggle, Spinbox, Slider};

const CONFIG_PATH: &str = "/home/lsgalante/.config/ccec/config.toml";

fn get_socket_path() -> String {
    match std::env::var("WAYLAND_DISPLAY") {
        Ok(display) => format!("/tmp/ccec-{}.sock", display),
        Err(_) => "/tmp/ccec.sock".to_string(),
    }
}

#[derive(Debug, Clone)]
pub struct NotificationsState {
    pub enable: bool,
    pub enable_toggle: Toggle,
    pub bell: bool,
    pub bell_toggle: Toggle,
    pub duration: i32,
    pub duration_spinbox: Spinbox,
    pub opacity: f32,
    pub opacity_slider: Slider,
}

impl Default for NotificationsState {
    fn default() -> Self {
        Self {
            enable: true,
            enable_toggle: Toggle::new().with_label("Enable Notifications"),
            bell: false,
            bell_toggle: Toggle::new().with_label("Play Bell Sound"),
            duration: 5,
            duration_spinbox: Spinbox::new(5, 1, 60, 1)
                .with_label("Notification Duration")
                .with_unit("s"),
            opacity: 0.9,
            opacity_slider: Slider::new()
                .with_label("Transparency")
                .with_value(0.9),
        }
    }
}

#[derive(Debug, Clone)]
pub enum NotificationsMessage {
    ToggleEnable,
    ToggleBell,
    SetDuration(i32),
    SetOpacity(f32),
    SendTestNotification,
    Refreshed(NotificationsState),
}

pub fn read_notifications_config() -> NotificationsState {
    let content = fs::read_to_string(CONFIG_PATH).unwrap_or_default();
    let enable = parse_notifications_enable(&content);
    let bell = parse_notifications_bell(&content);
    let duration = parse_notifications_duration(&content);
    let opacity = parse_transparency_opacity(&content);
    NotificationsState {
        enable,
        enable_toggle: Toggle::new().with_label("Enable Notifications"),
        bell,
        bell_toggle: Toggle::new().with_label("Play Bell Sound"),
        duration,
        duration_spinbox: Spinbox::new(duration, 1, 60, 1)
            .with_label("Notification Duration")
            .with_unit("s"),
        opacity,
        opacity_slider: Slider::new()
            .with_label("Transparency")
            .with_value(opacity),
    }
}

fn parse_notifications_enable(content: &str) -> bool {
    let mut in_section = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[notifications]" {
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

fn parse_notifications_bell(content: &str) -> bool {
    let mut in_section = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[notifications]" {
            in_section = true;
            continue;
        }
        if trimmed.starts_with('[') && in_section {
            break;
        }
        if in_section && trimmed.starts_with("bell") {
            if let Some(val) = trimmed.split('=').nth(1) {
                return val.trim() == "true";
            }
        }
    }
    false // default to false
}

fn parse_notifications_duration(content: &str) -> i32 {
    let mut in_section = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[notifications]" {
            in_section = true;
            continue;
        }
        if trimmed.starts_with('[') && in_section {
            break;
        }
        if in_section && trimmed.starts_with("duration") {
            if let Some(val) = trimmed.split('=').nth(1) {
                if let Ok(d) = val.trim().parse::<i32>() {
                    return d;
                }
            }
        }
    }
    5 // default to 5 seconds
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
    let mut updated_lines = Vec::new();
    let mut in_section = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[notifications]" {
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
        let has_section = content.lines().any(|l| l.trim() == "[notifications]");
        if has_section {
            let mut in_section = false;
            let mut inserted = false;
            for line in updated.lines() {
                if line.trim() == "[notifications]" {
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
            updated.push_str("\n[notifications]\n");
            updated.push_str(&new_line);
            updated.push_str("\n");
        }
    }
    let _ = fs::write(CONFIG_PATH, updated);
}

fn write_enable_notifications(enabled: bool) {
    write_config_value("enable", &enabled.to_string());
    send_ipc_command("reload");
}

const BTN_BG: [f32; 4] = [0.20, 0.40, 0.65, 1.0];
const BTN_HOVER: [f32; 4] = [0.28, 0.50, 0.78, 1.0];
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

fn parse_transparency_opacity(content: &str) -> f32 {
    let mut in_section = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[transparency]" {
            in_section = true;
            continue;
        }
        if trimmed.starts_with('[') && in_section {
            break;
        }
        if in_section && trimmed.starts_with("opacity") {
            if let Some(val) = trimmed.split('=').nth(1) {
                if let Ok(o) = val.trim().parse::<f32>() {
                    return o.clamp(0.0, 1.0);
                }
            }
        }
    }
    0.9 // default to 0.9
}

fn write_transparency_config_value(key: &str, value: &str) {
    let content = fs::read_to_string(CONFIG_PATH).unwrap_or_default();
    let new_line = format!("{} = {}", key, value);

    let mut found = false;
    let mut updated_lines = Vec::new();
    let mut in_section = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[transparency]" {
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
        let has_section = content.lines().any(|l| l.trim() == "[transparency]");
        if has_section {
            let mut in_section = false;
            let mut inserted = false;
            for line in updated.lines() {
                if line.trim() == "[transparency]" {
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
            updated.push_str("\n[transparency]\n");
            updated.push_str(&new_line);
            updated.push_str("\n");
        }
    }
    let _ = fs::write(CONFIG_PATH, updated);
}

pub fn view(state: &mut NotificationsState, cx: f32, cy: f32, cw: f32, _ch: f32) -> PageContent {
    let mut pc = PageContent::new();
    let y = cy + 12.0;

    let mut sec = Section::new(&mut pc, cx, y, cw, "System Notifications");

    let toggle_w = 48.0;
    let toggle_h = 24.0;
    state.enable_toggle.set_toggled(state.enable);
    sec.widget(&mut pc, &mut state.enable_toggle, 14.0, toggle_w, toggle_h);
    sec.spacing(8.0);

    state.bell_toggle.set_toggled(state.bell);
    sec.widget(&mut pc, &mut state.bell_toggle, 14.0, toggle_w, toggle_h);
    sec.spacing(16.0);

    state.duration_spinbox.value = state.duration;
    state.duration_spinbox.set_label("Notification Duration");
    sec.widget(&mut pc, &mut state.duration_spinbox, 14.0, 200.0, 26.0);
    sec.spacing(16.0);

    let btn_w = 160.0;
    let btn_h = 32.0;
    let btn_y = sec.ay();
    sec.row(1, 0.0, btn_h, |_, x, _| {
        pc.button(
            "Send Test Notification",
            x,
            btn_y,
            btn_w,
            btn_h,
            BTN_BG,
            BTN_HOVER,
            WHITE,
            AppAction::Notifications(NotificationsMessage::SendTestNotification),
        );
    });
    sec.spacing(12.0);
    sec.finish(&mut pc);

    let mut sec2 = Section::new(&mut pc, cx, sec.ay() + 24.0, cw, "Transparency");
    state.opacity_slider.set_value(state.opacity);
    sec2.widget(&mut pc, &mut state.opacity_slider, 14.0, 300.0, 20.0);
    sec2.spacing(12.0);
    sec2.finish(&mut pc);

    pc
}

pub fn update(state: &mut NotificationsState, msg: NotificationsMessage) {
    match msg {
        NotificationsMessage::ToggleEnable => {
            state.enable = !state.enable;
            write_enable_notifications(state.enable);
        }
        NotificationsMessage::ToggleBell => {
            state.bell = !state.bell;
            write_config_value("bell", &state.bell.to_string());
        }
        NotificationsMessage::SetDuration(d) => {
            state.duration = d;
            write_config_value("duration", &state.duration.to_string());
        }
        NotificationsMessage::SetOpacity(o) => {
            state.opacity = o;
            write_transparency_config_value("opacity", &format!("{:.2}", o));
            send_ipc_command("reload");
        }
        NotificationsMessage::SendTestNotification => {
            send_ipc_command("notify \"ccec\" \"System notifications are working correctly!\"");
        }
        NotificationsMessage::Refreshed(new) => {
            *state = new;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_notifications_enable_default() {
        assert!(parse_notifications_enable(""));
        assert!(parse_notifications_enable("[layout]\ngap = 18\n"));
    }

    #[test]
    fn test_parse_notifications_enable_explicit() {
        let content = "\
[notifications]
enable = false
";
        assert!(!parse_notifications_enable(content));

        let content = "\
[notifications]
enable = true
";
        assert!(parse_notifications_enable(content));
    }

    #[test]
    fn test_parse_notifications_enable_other_sections() {
        let content = "\
[layout]
enable = false

[notifications]
enable = true

[input]
enable = false
";
        assert!(parse_notifications_enable(content));

        let content = "\
[layout]
enable = true

[notifications]
enable = false

[input]
enable = true
";
        assert!(!parse_notifications_enable(content));
    }

    #[test]
    fn test_parse_notifications_duration_default() {
        assert_eq!(parse_notifications_duration(""), 5);
        assert_eq!(parse_notifications_duration("[notifications]\n"), 5);
    }

    #[test]
    fn test_parse_notifications_duration_explicit() {
        let content = "\
[notifications]
duration = 10
";
        assert_eq!(parse_notifications_duration(content), 10);
    }
}
