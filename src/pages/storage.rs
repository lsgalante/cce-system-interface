use crate::app::PageContent;
use clear_ui::layout::{Section, PageLayoutBuilder, LayoutStrategy};

#[derive(Debug, Clone, Default)]
pub struct StorageState {
    pub disk_total: f64,
    pub disk_used: f64,
    pub ram_total: f64,
    pub ram_used: f64,
    pub loaded: bool,
}

#[derive(Debug, Clone)]
pub enum StorageMessage {
    Refreshed(StorageState),
}

pub async fn fetch_storage_state() -> StorageState {
    let disk_output = tokio::process::Command::new("df")
        .args(["-BG", "/"])
        .output().await.ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();
    let (disk_total, disk_used) = parse_disk(&disk_output);

    let mem_output = tokio::process::Command::new("free")
        .args(["-b"])
        .output().await.ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();
    let (ram_total, ram_used) = parse_mem(&mem_output);

    StorageState { disk_total, disk_used, ram_total, ram_used, loaded: true }
}

fn parse_disk(info: &str) -> (f64, f64) {
    for line in info.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 4 {
            let total = parts[1].trim_end_matches('G').parse::<f64>().unwrap_or(0.0);
            let used = parts[2].trim_end_matches('G').parse::<f64>().unwrap_or(0.0);
            return (total, used);
        }
    }
    (0.0, 0.0)
}

fn parse_mem(info: &str) -> (f64, f64) {
    for line in info.lines() {
        if line.starts_with("Mem:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let total = parts[1].parse::<f64>().unwrap_or(0.0) / 1_073_741_824.0;
                let used = parts[2].parse::<f64>().unwrap_or(0.0) / 1_073_741_824.0;
                return (total, used);
            }
        }
    }
    (0.0, 0.0)
}

const LABEL_FG: [f32; 4] = [0.56, 0.83, 0.56, 1.0];
const TEXT_FG: [f32; 4] = [0.83, 0.83, 0.83, 1.0];

pub fn view(state: &StorageState, cx: f32, cy: f32, cw: f32, ch: f32, layout: &mut dyn LayoutStrategy) -> PageContent {
    let mut final_pc = PageContent::new();
    let sec_w = 320.0f32;
    let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(1);

    builder.add_section(&mut final_pc, |pc, rx, ry| {
        let mut sec = Section::new(pc, rx, ry, sec_w, "Local Storage");
        if !state.loaded {
            sec.text(pc, "Loading storage and memory usage...", 12.0, 0.0, 12.0, TEXT_FG);
            sec.spacing(18.0);
        } else {
            let disk_pct = if state.disk_total > 0.0 {
                state.disk_used / state.disk_total * 100.0
            } else {
                0.0
            };

            sec.text(pc, "Disk", 12.0, 0.0, 12.0, LABEL_FG);
            sec.text(pc,
                &format!("{:.0} / {:.0} GiB  ({:.0}%)", state.disk_used, state.disk_total, disk_pct),
                100.0, 0.0, 12.0, TEXT_FG,
            );
            sec.spacing(18.0);

            let bar_w = sec_w - 24.0;
            let yt = sec.ay();
            pc.rect([0.15, 0.15, 0.25, 1.0], sec.ax(12.0), yt, bar_w, 8.0);
            if disk_pct > 0.0 {
                pc.rect([0.36, 0.60, 0.36, 1.0], sec.ax(12.0), yt, bar_w * (disk_pct as f32 / 100.0).min(1.0), 8.0);
            }
            sec.content_y += 20.0;

            let ram_pct = if state.ram_total > 0.0 {
                state.ram_used / state.ram_total * 100.0
            } else {
                0.0
            };

            sec.text(pc, "RAM", 12.0, 0.0, 12.0, LABEL_FG);
            sec.text(pc,
                &format!("{:.1} / {:.1} GiB  ({:.0}%)", state.ram_used, state.ram_total, ram_pct),
                100.0, 0.0, 12.0, TEXT_FG,
            );
            sec.spacing(18.0);

            let yt = sec.ay();
            pc.rect([0.15, 0.15, 0.25, 1.0], sec.ax(12.0), yt, bar_w, 8.0);
            if ram_pct > 0.0 {
                pc.rect([0.50, 0.50, 0.65, 1.0], sec.ax(12.0), yt, bar_w * (ram_pct as f32 / 100.0).min(1.0), 8.0);
            }
        }
        sec.finish(pc)
    });

    final_pc
}

pub fn update(state: &mut StorageState, msg: StorageMessage) {
    match msg {
        StorageMessage::Refreshed(new) => { *state = new; }
    }
}
