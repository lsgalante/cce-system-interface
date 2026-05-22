use crate::app::PageContent;
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

pub fn view(state: &SystemState, cx: f32, cy: f32, cw: f32, _ch: f32) -> PageContent {
    let mut pc = PageContent::new();
    let y = cy + 12.0;

    let mut sec = Section::new(&mut pc, cx, y, cw, "System");
    if !state.loaded {
        sec.text(&mut pc, "Loading system information...", 12.0, 0.0, 14.0, TEXT_FG);
        sec.spacing(10.0);
    } else {
        sec.text(&mut pc, &format!("{}  —  Linux {}", state.hostname, state.kernel), 12.0, 0.0, 14.0, TEXT_FG);
        sec.spacing(10.0);
        sec.text(&mut pc, &format!("Uptime: {}", state.uptime), 12.0, 0.0, 12.0, TEXT_DIM);
    }
    sec.finish(&mut pc);

    pc
}

pub fn update(state: &mut SystemState, msg: SystemMessage) {
    match msg {
        SystemMessage::Refreshed(new) => { *state = new; }
    }
}
