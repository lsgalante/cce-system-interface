use crate::app::PageContent;

#[derive(Debug, Clone, Default)]
pub struct SystemState {
    pub hostname: String,
    pub kernel: String,
    pub cpu_model: String,
    pub cpu_usage: f32,
    pub cpu_cores: u32,
    pub uptime: String,
    pub gpu: String,
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

    let (cpu_model, cpu_cores) = {
        let lscpu = tokio::process::Command::new("lscpu")
            .output().await.ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default();
        let model = lscpu.lines()
            .find(|l| l.contains("Model name"))
            .and_then(|l| l.split(':').nth(1))
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        let cores = lscpu.lines()
            .find(|l| l.contains("CPU(s)"))
            .and_then(|l| {
                let rest = l.split(':').nth(1).unwrap_or("").trim();
                rest.split_whitespace().next().and_then(|n| n.parse::<u32>().ok())
            })
            .unwrap_or(0);
        (model, cores)
    };

    let cpu_usage = {
        let read_stat = || -> Option<(u64, u64)> {
            let stat = std::fs::read_to_string("/proc/stat").ok()?;
            let first = stat.lines().next()?;
            let vals: Vec<u64> = first.split_whitespace().skip(1).filter_map(|v| v.parse().ok()).collect();
            if vals.len() < 3 { return None; }
            let total: u64 = vals.iter().sum();
            let idle = vals.get(3).copied().unwrap_or(0);
            Some((idle, total))
        };
        let (idle1, total1) = read_stat().unwrap_or((0, 1));
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let (idle2, total2) = read_stat().unwrap_or((0, 1));
        let d_idle = idle2.saturating_sub(idle1);
        let d_total = total2.saturating_sub(total1);
        if d_total > 0 {
            (1.0 - d_idle as f64 / d_total as f64) * 100.0
        } else { 0.0 }
    } as f32;

    let uptime = tokio::process::Command::new("uptime")
        .args(["-p"]).output().await.ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().trim_start_matches("up ").to_string())
        .unwrap_or_default();

    let gpu = tokio::process::Command::new("lspci")
        .output().await.ok()
        .and_then(|o| {
            String::from_utf8_lossy(&o.stdout).lines()
                .find(|l| l.contains("VGA") || l.contains("3D"))
                .and_then(|l| l.split(':').nth(2))
                .map(|s| s.trim().to_string())
        })
        .unwrap_or_default();

    SystemState { hostname, kernel, cpu_model, cpu_usage, cpu_cores, uptime, gpu }
}

const TEXT_FG: [f32; 4] = [0.83, 0.83, 0.83, 1.0];
const TEXT_DIM: [f32; 4] = [0.53, 0.53, 0.60, 1.0];

pub fn view(state: &SystemState, cx: f32, cy: f32, _cw: f32, _ch: f32) -> PageContent {
    let mut pc = PageContent::new();
    let mut y = cy + 12.0;

    // Hostname / kernel
    pc.text(&format!("{}  —  Linux {}", state.hostname, state.kernel), cx + 12.0, y, 14.0, TEXT_FG);
    y += 22.0;

    // Uptime
    pc.text(&format!("Uptime: {}", state.uptime), cx + 12.0, y, 12.0, TEXT_DIM);
    y += 18.0;

    // CPU
    pc.text(&format!("CPU  {}  ({} cores)  —  {:.0}%", state.cpu_model, state.cpu_cores, state.cpu_usage),
        cx + 12.0, y, 12.0, TEXT_FG);
    y += 18.0;

    // GPU
    pc.text(&format!("GPU  {}", state.gpu), cx + 12.0, y, 12.0, TEXT_FG);

    pc
}

pub fn update(_state: &mut SystemState, _msg: SystemMessage) {}
