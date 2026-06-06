use crate::app::{AppAction, PageContent};
use clear_ui::layout::{Section, PageLayoutBuilder, LayoutStrategy};
use clear_ui::widget::{Label, Toggle, Widget, Spinbox};

use crate::pages::typeface::parse_u16_from;

#[derive(Debug, Clone)]
pub struct StatusState {
    pub font_size: u16,
    pub padding: u16,
    pub separators: bool,
    pub underline: bool,
    pub running: bool,
    pub loaded: bool,
    pub status_label: Label,
    pub size_label: Label,
    pub separators_toggle: Toggle,
    pub underline_toggle: Toggle,
    pub padding_spinbox: Spinbox,
}

impl Default for StatusState {
    fn default() -> Self {
        Self {
            font_size: 11,
            padding: 8,
            separators: true,
            underline: true,
            running: false,
            loaded: false,
            status_label: Label::new("Status Interface: Stopped").with_font_size(14.0).with_color([170, 51, 51]),
            size_label: Label::new("Font size: 11px").with_font_size(13.0).with_color([212, 212, 212]),
            separators_toggle: Toggle::new().with_label("Show Separators"),
            underline_toggle: Toggle::new().with_label("Show Underline"),
            padding_spinbox: Spinbox::new(8, 0, 32, 1).with_label("Side Padding").with_unit("px"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum StatusMessage {
    Refreshed(StatusState),
    FontSizeUp,
    FontSizeDown,
    ToggleSeparators,
    ToggleUnderline,
    ReloadStatus,
    SetPadding(u16),
}

pub async fn fetch_status_state() -> StatusState {
    let running = tokio::process::Command::new("pgrep")
        .args(["-f", "clear-status-interface"]).output().await.ok()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false);

    let font_size = read_status_font_size().unwrap_or(11);
    let padding = read_status_padding().unwrap_or(8);
    let separators = read_status_separators().unwrap_or(true);
    let underline = read_status_underline().unwrap_or(true);
    let status_color = if running { [92, 143, 97] } else { [170, 51, 51] };
    StatusState {
        font_size,
        padding,
        separators,
        underline,
        running,
        loaded: true,
        status_label: Label::new(&format!("Status Interface: {}", if running { "Running" } else { "Stopped" }))
            .with_font_size(14.0)
            .with_color(status_color),
        size_label: Label::new(&format!("Font size: {}px", font_size))
            .with_font_size(13.0)
            .with_color([212, 212, 212]),
        separators_toggle: Toggle::new().with_label("Show Separators"),
        underline_toggle: Toggle::new().with_label("Show Underline"),
        padding_spinbox: Spinbox::new(padding as i32, 0, 32, 1).with_label("Side Padding").with_unit("px"),
    }
}

#[cfg(test)]
thread_local! {
    static TEST_CONFIG_PATH: std::cell::RefCell<Option<String>> = std::cell::RefCell::new(None);
}

fn get_config_path() -> String {
    #[cfg(test)]
    {
        TEST_CONFIG_PATH.with(|p| {
            if let Some(path) = p.borrow().as_ref() {
                return path.clone();
            }
            "/home/lsgalante/.config/ccec/config.toml".to_string()
        })
    }
    #[cfg(not(test))]
    {
        "/home/lsgalante/.config/ccec/config.toml".to_string()
    }
}

fn write_status_value(key: &str, value: &str) {
    crate::pages::typeface::write_config_value_path(&get_config_path(), key, value);
}

fn read_status_font_size() -> Option<u16> {
    let content = std::fs::read_to_string(&get_config_path()).ok()?;
    Some(parse_u16_from(&content, "status_font_size", 11))
}

fn write_status_font_size(size: u16) {
    write_status_value("status_font_size", &size.to_string());
}

fn read_status_padding() -> Option<u16> {
    let content = std::fs::read_to_string(&get_config_path()).ok()?;
    Some(parse_u16_from(&content, "status_padding", 8))
}

fn write_status_padding(padding: u16) {
    write_status_value("status_padding", &padding.to_string());
}

fn read_status_separators() -> Option<bool> {
    let content = std::fs::read_to_string(&get_config_path()).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("status_separators") {
            let rest = rest.trim_start_matches(|c: char| c == ' ' || c == '=' || c == '"');
            if let Ok(val) = rest.trim_end_matches('"').trim().parse::<bool>() {
                return Some(val);
            }
        }
    }
    Some(true)
}

fn write_status_separators(val: bool) {
    write_status_value("status_separators", &val.to_string());
}

fn read_status_underline() -> Option<bool> {
    let content = std::fs::read_to_string(&get_config_path()).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("status_underline") {
            let rest = rest.trim_start_matches(|c: char| c == ' ' || c == '=' || c == '"');
            if let Ok(val) = rest.trim_end_matches('"').trim().parse::<bool>() {
                return Some(val);
            }
        }
    }
    Some(true)
}

fn write_status_underline(val: bool) {
    write_status_value("status_underline", &val.to_string());
}

fn get_socket_path() -> String {
    match std::env::var("WAYLAND_DISPLAY") {
        Ok(display) => format!("/tmp/ccec-{}.sock", display),
        Err(_) => "/tmp/ccec.sock".to_string(),
    }
}

fn send_ipc_command(cmd: &str) {
    if let Ok(mut stream) = std::os::unix::net::UnixStream::connect(get_socket_path()) {
        use std::io::Write;
        let _ = stream.write_all(format!("{}\n", cmd).as_bytes());
    }
}

fn status_interface_reload() {
    let _ = std::process::Command::new("pkill")
        .args(["-f", "clear-status-interface"])
        .status();
    std::thread::sleep(std::time::Duration::from_millis(150));
    send_ipc_command("spawn clear-status-interface");
}

const TEXT_FG: [f32; 4] = [0.83, 0.83, 0.83, 1.0];
const BTN_ACTIVE: [f32; 4] = [0.20, 0.40, 0.22, 1.0];
const BTN_INACTIVE: [f32; 4] = [0.13, 0.18, 0.14, 1.0];
const BTN_HOVER: [f32; 4] = [0.25, 0.30, 0.26, 1.0];
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

pub fn view(state: &mut StatusState, cx: f32, cy: f32, cw: f32, ch: f32, layout: &mut dyn LayoutStrategy) -> PageContent {
    let mut final_pc = PageContent::new();
    let sec_w = 320.0f32;
    let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(1);

    builder.add_section(&mut final_pc, |pc, rx, ry| {
        let mut sec = Section::new(pc, rx, ry, sec_w, "Status Interface");
        if !state.loaded {
            sec.text(pc, "Loading Status Interface status...", 12.0, 0.0, 12.0, TEXT_FG);
            sec.spacing(18.0);
        } else {
            // Status
            let status_text = if state.running { "Status Interface: Running" } else { "Status Interface: Stopped" };
            state.status_label.set_text(status_text);
            sec.widget(pc, &mut state.status_label, 12.0, sec_w - 24.0, 20.0);
            sec.spacing(12.0);

            // Font size
            state.size_label.set_text(&format!("Font size: {}px", state.font_size));
            sec.widget(pc, &mut state.size_label, 12.0, sec_w - 24.0, 20.0);
            sec.spacing(12.0);

            let btn_h = 28.0;
            let yt = sec.ay();
            pc.button("-1", sec.ax(12.0), yt, 36.0, btn_h,
                BTN_INACTIVE, BTN_HOVER, WHITE,
                AppAction::Status(StatusMessage::FontSizeDown));
            pc.text(&format!(" {}px ", state.font_size), sec.ax(56.0), yt + 7.0, 13.0, TEXT_FG);
            pc.button("+1", sec.ax(12.0 + 36.0 + 8.0), yt, 36.0, btn_h,
                BTN_ACTIVE, BTN_HOVER, WHITE,
                AppAction::Status(StatusMessage::FontSizeUp));
            sec.content_y += btn_h + 16.0;

            // Separators toggle
            state.separators_toggle.set_toggled(state.separators);
            sec.widget(pc, &mut state.separators_toggle, 12.0, 48.0, 24.0);
            sec.spacing(16.0);

            // Underline toggle
            state.underline_toggle.set_toggled(state.underline);
            sec.widget(pc, &mut state.underline_toggle, 12.0, 48.0, 24.0);
            sec.spacing(16.0);

            // Padding spinbox
            state.padding_spinbox.value = state.padding as i32;
            sec.widget(pc, &mut state.padding_spinbox, 12.0, 200.0, 26.0);
            sec.spacing(16.0);

            // Reload button
            let yt = sec.ay();
            let btn_w = (sec_w - 24.0).min(200.0);
            pc.button("Reload Status Interface", rx + sec_w / 2.0 - btn_w / 2.0, yt, btn_w, 32.0,
                BTN_INACTIVE, BTN_HOVER, WHITE,
                AppAction::Status(StatusMessage::ReloadStatus));
        }
        sec.finish(pc)
    });

    final_pc
}

pub fn update(state: &mut StatusState, msg: StatusMessage) {
    match msg {
        StatusMessage::Refreshed(new) => {
            let was_status_hovered = state.status_label.hovered();
            let was_size_hovered = state.size_label.hovered();
            let was_separators_hovered = state.separators_toggle.hovered();
            let was_underline_hovered = state.underline_toggle.hovered();
            *state = new;
            state.status_label.set_hovered(was_status_hovered);
            state.size_label.set_hovered(was_size_hovered);
            state.separators_toggle.set_hovered(was_separators_hovered);
            state.underline_toggle.set_hovered(was_underline_hovered);
        }
        StatusMessage::FontSizeUp => {
            if state.font_size < 28 {
                state.font_size += 1;
                write_status_font_size(state.font_size);
                status_interface_reload();
            }
        }
        StatusMessage::FontSizeDown => {
            if state.font_size > 8 {
                state.font_size -= 1;
                write_status_font_size(state.font_size);
                status_interface_reload();
            }
        }
        StatusMessage::ToggleSeparators => {
            state.separators = !state.separators;
            write_status_separators(state.separators);
            status_interface_reload();
        }
        StatusMessage::ToggleUnderline => {
            state.underline = !state.underline;
            write_status_underline(state.underline);
            status_interface_reload();
        }
        StatusMessage::SetPadding(val) => {
            state.padding = val;
            write_status_padding(val);
            status_interface_reload();
        }
        StatusMessage::ReloadStatus => {
            status_interface_reload();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_read_write_separators() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_status_separators.toml");
        let path_str = path.to_str().unwrap().to_string();

        let _ = fs::write(&path_str, "[layout]\nstatus_separators = true\nstatus_padding = 8\n");
        TEST_CONFIG_PATH.with(|p| *p.borrow_mut() = Some(path_str));

        let original = read_status_separators().unwrap_or(true);
        write_status_separators(!original);
        assert_eq!(read_status_separators(), Some(!original));
        write_status_separators(original);
        assert_eq!(read_status_separators(), Some(original));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_read_write_padding() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_status_padding.toml");
        let path_str = path.to_str().unwrap().to_string();

        let _ = fs::write(&path_str, "[layout]\nstatus_separators = true\nstatus_padding = 8\n");
        TEST_CONFIG_PATH.with(|p| *p.borrow_mut() = Some(path_str));

        let original = read_status_padding().unwrap_or(8);
        write_status_padding(12);
        assert_eq!(read_status_padding(), Some(12));
        write_status_padding(original);
        assert_eq!(read_status_padding(), Some(original));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_read_write_underline() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_status_underline.toml");
        let path_str = path.to_str().unwrap().to_string();

        let _ = fs::write(&path_str, "[layout]\nstatus_underline = true\nstatus_padding = 8\n");
        TEST_CONFIG_PATH.with(|p| *p.borrow_mut() = Some(path_str));

        let original = read_status_underline().unwrap_or(true);
        write_status_underline(!original);
        assert_eq!(read_status_underline(), Some(!original));
        write_status_underline(original);
        assert_eq!(read_status_underline(), Some(original));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_view_layout_grid() {
        let mut state = StatusState::default();
        let mut layout = clear_ui::layout::ColumnLayout::new(20.0);
        let pc = view(&mut state, 10.0, 20.0, 800.0, 600.0, &mut layout);
        assert!(!pc.rects.is_empty() || !pc.texts.is_empty());
    }
}
