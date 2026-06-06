use crate::app::{AppAction, PageContent};
use clear_ui::layout::{Section, PageLayoutBuilder, LayoutStrategy};

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

pub fn view(state: &SystemState, cx: f32, cy: f32, cw: f32, ch: f32, layout: &mut dyn LayoutStrategy) -> PageContent {
    let mut final_pc = PageContent::new();
    let sec_w = 320.0f32;
    let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(2);

    // 1. System Section
    builder.add_section(&mut final_pc, |pc, rx, ry| {
        let mut sec = Section::new(pc, rx, ry, sec_w, "System");
        if !state.loaded {
            sec.text(pc, "Loading system information...", 12.0, 0.0, 14.0, TEXT_FG);
            sec.spacing(10.0);
        } else {
            sec.text(pc, &format!("{}  —  Linux {}", state.hostname, state.kernel), 12.0, 0.0, 14.0, TEXT_FG);
            sec.spacing(10.0);
            sec.text(pc, &format!("Uptime: {}", state.uptime), 12.0, 0.0, 12.0, TEXT_DIM);
        }
        sec.finish(pc)
    });

    // 2. System Actions Section
    builder.add_section(&mut final_pc, |pc, rx, ry| {
        let mut sec_act = Section::new(pc, rx, ry, sec_w, "System Actions");
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
        sec_act.finish(pc)
    });

    final_pc
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
