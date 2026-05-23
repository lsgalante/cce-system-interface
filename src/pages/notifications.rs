use std::fs;
use std::io::Write;

use crate::app::{AppAction, PageContent};
use clear_ui::layout::{render_widget, Section};
use clear_ui::widget::{Toggle, Widget};

const CONFIG_PATH: &str = "/home/lsgalante/.config/clearwm/config.toml";
const CLEARWM_SOCK: &str = "/tmp/clearwm.sock";

#[derive(Debug, Clone)]
pub struct NotificationsState {
    pub enable: bool,
    pub enable_toggle: Toggle,
}

impl Default for NotificationsState {
    fn default() -> Self {
        Self {
            enable: true,
            enable_toggle: Toggle::new().with_label("Enable Notifications"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum NotificationsMessage {
    ToggleEnable,
    SendTestNotification,
    Refreshed(NotificationsState),
}

pub fn read_notifications_config() -> NotificationsState {
    let content = fs::read_to_string(CONFIG_PATH).unwrap_or_default();
    let enable = parse_notifications_enable(&content);
    NotificationsState {
        enable,
        enable_toggle: Toggle::new().with_label("Enable Notifications"),
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

fn send_ipc_command(cmd: &str) {
    if let Ok(mut stream) = std::os::unix::net::UnixStream::connect(CLEARWM_SOCK) {
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

const TEXT_FG: [f32; 4] = [0.83, 0.83, 0.83, 1.0];
const ACCENT: [f32; 4] = [0.36, 0.56, 0.38, 1.0];
const BTN_HOVER: [f32; 4] = [0.25, 0.30, 0.26, 1.0];

pub fn view(state: &mut NotificationsState, cx: f32, cy: f32, cw: f32, _ch: f32) -> PageContent {
    let mut pc = PageContent::new();
    let y = cy + 12.0;

    let mut sec = Section::new(&mut pc, cx, y, cw, "System Notifications");

    let yt = sec.ay();
    let toggle_w = 48.0;
    let toggle_h = 24.0;
    state.enable_toggle.set_toggled(state.enable);
    state.enable_toggle.set_row_rect(sec.ax(8.0), cw - 16.0);
    render_widget(&mut pc, &mut state.enable_toggle, sec.ax(100.0), yt, toggle_w, toggle_h);
    sec.content_y += toggle_h + 24.0;

    let btn_w = 160.0;
    let btn_h = 32.0;
    let btn_x = sec.ax(0.0);
    let btn_y = sec.ay();
    pc.button(
        "Send Test Notification",
        btn_x,
        btn_y,
        btn_w,
        btn_h,
        ACCENT,
        BTN_HOVER,
        TEXT_FG,
        AppAction::Notifications(NotificationsMessage::SendTestNotification),
    );
    sec.content_y += btn_h + 12.0;

    sec.finish(&mut pc);
    pc
}

pub fn update(state: &mut NotificationsState, msg: NotificationsMessage) {
    match msg {
        NotificationsMessage::ToggleEnable => {
            state.enable = !state.enable;
            write_enable_notifications(state.enable);
        }
        NotificationsMessage::SendTestNotification => {
            send_ipc_command("notify \"clearwm\" \"System notifications are working correctly!\"");
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
}
