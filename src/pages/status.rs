use crate::app::{AppAction, PageContent};
use clear_ui::layout::Section;
use clear_ui::widget::{Label, Widget};
use crate::pages::typeface::{parse_u16_from, write_config_value};

#[derive(Debug, Clone)]
pub struct StatusState {
    pub font_size: u16,
    pub running: bool,
    pub loaded: bool,
    pub status_label: Label,
    pub size_label: Label,
}

impl Default for StatusState {
    fn default() -> Self {
        Self {
            font_size: 11,
            running: false,
            loaded: false,
            status_label: Label::new("Status Interface: Stopped").with_font_size(14.0).with_color([170, 51, 51]),
            size_label: Label::new("Font size: 11px").with_font_size(13.0).with_color([212, 212, 212]),
        }
    }
}

#[derive(Debug, Clone)]
pub enum StatusMessage {
    Refreshed(StatusState),
    FontSizeUp,
    FontSizeDown,
    ReloadStatus,
}

pub async fn fetch_status_state() -> StatusState {
    let running = tokio::process::Command::new("pgrep")
        .args(["-f", "clear-status-interface"]).output().await.ok()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false);

    let font_size = read_status_font_size().unwrap_or(11);
    let status_color = if running { [92, 143, 97] } else { [170, 51, 51] };
    StatusState {
        font_size,
        running,
        loaded: true,
        status_label: Label::new(&format!("Status Interface: {}", if running { "Running" } else { "Stopped" }))
            .with_font_size(14.0)
            .with_color(status_color),
        size_label: Label::new(&format!("Font size: {}px", font_size))
            .with_font_size(13.0)
            .with_color([212, 212, 212]),
    }
}

fn read_status_font_size() -> Option<u16> {
    let content = std::fs::read_to_string("/home/lsgalante/.config/clearwm/config.toml").ok()?;
    Some(parse_u16_from(&content, "status_font_size", 11))
}

fn write_status_font_size(size: u16) {
    write_config_value("status_font_size", &size.to_string());
}

fn status_interface_reload() {
    let _ = std::process::Command::new("pkill")
        .args(["-f", "clear-status-interface"])
        .spawn();
}

const TEXT_FG: [f32; 4] = [0.83, 0.83, 0.83, 1.0];
const BTN_ACTIVE: [f32; 4] = [0.20, 0.40, 0.22, 1.0];
const BTN_INACTIVE: [f32; 4] = [0.13, 0.18, 0.14, 1.0];
const BTN_HOVER: [f32; 4] = [0.25, 0.30, 0.26, 1.0];
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

pub fn view(state: &mut StatusState, cx: f32, cy: f32, cw: f32, _ch: f32) -> PageContent {
    let mut pc = PageContent::new();
    let y = cy + 12.0;

    let mut sec = Section::new(&mut pc, cx, y, cw, "Status Interface");

    if !state.loaded {
        sec.text(&mut pc, "Loading Status Interface status...", 12.0, 0.0, 12.0, TEXT_FG);
        sec.spacing(18.0);
    } else {
        // Status
        let status_text = if state.running { "Status Interface: Running" } else { "Status Interface: Stopped" };
        state.status_label.set_text(status_text);
        sec.widget(&mut pc, &mut state.status_label, 12.0, cw - 24.0, 20.0);
        sec.spacing(12.0);

        // Font size
        state.size_label.set_text(&format!("Font size: {}px", state.font_size));
        sec.widget(&mut pc, &mut state.size_label, 12.0, cw - 24.0, 20.0);
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
        sec.content_y += btn_h + 12.0;

        // Reload button
        let yt = sec.ay();
        let btn_w = (cw - 24.0).min(200.0);
        pc.button("Reload Status Interface", cx + cw / 2.0 - btn_w / 2.0, yt, btn_w, 32.0,
            BTN_INACTIVE, BTN_HOVER, WHITE,
            AppAction::Status(StatusMessage::ReloadStatus));
    }
    sec.finish(&mut pc);

    pc
}

pub fn update(state: &mut StatusState, msg: StatusMessage) {
    match msg {
        StatusMessage::Refreshed(new) => {
            let was_status_hovered = state.status_label.hovered();
            let was_size_hovered = state.size_label.hovered();
            *state = new;
            state.status_label.set_hovered(was_status_hovered);
            state.size_label.set_hovered(was_size_hovered);
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
        StatusMessage::ReloadStatus => {
            status_interface_reload();
        }
    }
}
