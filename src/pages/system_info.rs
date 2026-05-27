use crate::app::{AppAction, PageContent};
use clear_ui::layout::Section;

#[derive(Debug, Clone, Default)]
pub struct SystemState {
    pub hostname: String,
    pub kernel: String,
    pub uptime: String,
    pub loaded: bool,
}

#[derive(Debug, Clone)]
pub enum SystemMessage {
    Refreshed(SystemState),
    Suspend,
    Hibernate,
    Reboot,
    PowerOff,
}

pub async fn fetch_system_state() -> SystemState {
    let hostname = tokio::process::Command::new("hostname")
        .output().await.ok()
        .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().split('.').next().map(|s| s.to_string()))
        .unwrap_or_default();

    let kernel = tokio::process::Command::new("uname")
        .args(["-r"]).output().await.ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    let uptime = tokio::process::Command::new("uptime")
        .args(["-p"]).output().await.ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().trim_start_matches("up ").to_string())
        .unwrap_or_default();

    SystemState { hostname, kernel, uptime, loaded: true }
}

const TEXT_FG: [f32; 4] = [0.83, 0.83, 0.83, 1.0];
const TEXT_DIM: [f32; 4] = [0.53, 0.53, 0.60, 1.0];
const BTN_HOVER: [f32; 4] = [0.25, 0.30, 0.26, 1.0];
const DANGER_BG: [f32; 4] = [0.67, 0.20, 0.20, 1.0];
const SAFE_BG: [f32; 4] = [0.20, 0.33, 0.22, 1.0];
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

fn spawn_systemctl(action: &str) {
    let _ = tokio::process::Command::new("systemctl").arg(action).spawn();
}

pub fn view(state: &SystemState, cx: f32, cy: f32, cw: f32, _ch: f32) -> PageContent {
    let mut pc = PageContent::new();
    let mut y = cy + 12.0;

    let mut sec = Section::new(&mut pc, cx, y, cw, "System");
    if !state.loaded {
        sec.text(&mut pc, "Loading system information...", 12.0, 0.0, 14.0, TEXT_FG);
        sec.spacing(10.0);
    } else {
        sec.text(&mut pc, &format!("{}  —  Linux {}", state.hostname, state.kernel), 12.0, 0.0, 14.0, TEXT_FG);
        sec.spacing(10.0);
        sec.text(&mut pc, &format!("Uptime: {}", state.uptime), 12.0, 0.0, 12.0, TEXT_DIM);
    }
    y = sec.finish(&mut pc);

    // ── System Actions section ──
    let mut sec_act = Section::new(&mut pc, cx, y, cw, "System Actions");

    let yt = sec_act.ay();
    let act_btn_h = 32.0;

    sec_act.row(4, 8.0, act_btn_h, |i, x, w| {
        match i {
            0 => {
                pc.button("Suspend", x, yt, w, act_btn_h,
                    SAFE_BG, BTN_HOVER, WHITE, AppAction::SystemInfo(SystemMessage::Suspend));
            }
            1 => {
                pc.button("Hibernate", x, yt, w, act_btn_h,
                    SAFE_BG, BTN_HOVER, WHITE, AppAction::SystemInfo(SystemMessage::Hibernate));
            }
            2 => {
                pc.button("Reboot", x, yt, w, act_btn_h,
                    DANGER_BG, BTN_HOVER, WHITE, AppAction::SystemInfo(SystemMessage::Reboot));
            }
            3 => {
                pc.button("Power Off", x, yt, w, act_btn_h,
                    DANGER_BG, BTN_HOVER, WHITE, AppAction::SystemInfo(SystemMessage::PowerOff));
            }
            _ => {}
        }
    });
    sec_act.spacing(12.0);
    sec_act.finish(&mut pc);

    pc
}

pub fn update(_state: &mut SystemState, msg: SystemMessage) {
    match msg {
        SystemMessage::Refreshed(new) => { *_state = new; }
        SystemMessage::Suspend => spawn_systemctl("suspend"),
        SystemMessage::Hibernate => spawn_systemctl("hibernate"),
        SystemMessage::Reboot => spawn_systemctl("reboot"),
        SystemMessage::PowerOff => spawn_systemctl("poweroff"),
    }
}
