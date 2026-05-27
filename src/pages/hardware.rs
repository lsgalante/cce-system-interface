use crate::app::PageContent;
use clear_ui::layout::Section;
use clear_ui::widget::{Label, ScrollingList};

#[derive(Debug, Clone)]
pub struct HardwareState {
    pub cpu_model: String,
    pub cpu_usage: f32,
    pub cpu_cores: u32,
    pub gpus: Vec<String>,
    pub loaded: bool,
    pub cpu_label: Label,
    pub gpu_labels: Vec<Label>,
    pub processes: Vec<(String, String, String)>, // (pid, cpu, comm)
    pub cpu_list_box: ScrollingList,
}

impl Default for HardwareState {
    fn default() -> Self {
        Self {
            cpu_model: String::new(),
            cpu_usage: 0.0,
            cpu_cores: 0,
            gpus: Vec::new(),
            loaded: false,
            cpu_label: Label::new("CPU Info"),
            gpu_labels: Vec::new(),
            processes: Vec::new(),
            cpu_list_box: ScrollingList::new(24.0, 2.0),
        }
    }
}

#[derive(Debug, Clone)]
pub enum HardwareMessage {
    Refreshed(HardwareState),
    None,
}

fn read_cpu_temp() -> Option<f32> {
    if let Ok(entries) = std::fs::read_dir("/sys/class/hwmon") {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if let Ok(name) = std::fs::read_to_string(path.join("name")) {
                let name = name.trim();
                if name == "thinkpad" {
                    if let Ok(val) = std::fs::read_to_string(path.join("temp1_input")) {
                        if let Ok(temp_milli) = val.trim().parse::<f32>() {
                            return Some(temp_milli / 1000.0);
                        }
                    }
                } else if name == "coretemp" {
                    if let Ok(val) = std::fs::read_to_string(path.join("temp1_input")) {
                        if let Ok(temp_milli) = val.trim().parse::<f32>() {
                            return Some(temp_milli / 1000.0);
                        }
                    }
                }
            }
        }
    }
    None
}

fn read_thinkpad_gpu_temp() -> Option<f32> {
    if let Ok(entries) = std::fs::read_dir("/sys/class/hwmon") {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if let Ok(name) = std::fs::read_to_string(path.join("name")) {
                if name.trim() == "thinkpad" {
                    for i in 1..=8 {
                        let label_path = path.join(format!("temp{}_label", i));
                        if let Ok(lbl) = std::fs::read_to_string(&label_path) {
                            if lbl.trim() == "GPU" {
                                if let Ok(val) = std::fs::read_to_string(path.join(format!("temp{}_input", i))) {
                                    if let Ok(temp_milli) = val.trim().parse::<f32>() {
                                        return Some(temp_milli / 1000.0);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

async fn read_nvidia_gpu_temp() -> Option<f32> {
    let out = tokio::process::Command::new("nvidia-smi")
        .args(["--query-gpu=temperature.gpu", "--format=csv,noheader,nounits"])
        .output().await.ok()?;
    let val_str = String::from_utf8_lossy(&out.stdout);
    val_str.trim().parse::<f32>().ok()
}

pub async fn fetch_hardware_state() -> HardwareState {
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

    let mut gpus = Vec::new();
    if let Some(o) = tokio::process::Command::new("lspci").output().await.ok() {
        for line in String::from_utf8_lossy(&o.stdout).lines() {
            if line.contains("VGA") || line.contains("3D") {
                if let Some(name) = line.split(':').nth(2) {
                    let trimmed = name.trim().to_string();
                    if !trimmed.is_empty() {
                        gpus.push(trimmed);
                    }
                }
            }
        }
    }

    let processes = {
        let mut list = Vec::new();
        if let Some(o) = tokio::process::Command::new("ps")
            .args(["-eo", "pid,%cpu,comm", "--sort=-%cpu"])
            .output().await.ok()
        {
            let text = String::from_utf8_lossy(&o.stdout);
            for line in text.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    let pid = parts[0].to_string();
                    let cpu = parts[1].to_string();
                    let comm = parts[2..].join(" ");
                    list.push((pid, cpu, comm));
                }
            }
        }
        list
    };

    let cpu_temp = read_cpu_temp();
    let tp_gpu_temp = read_thinkpad_gpu_temp();
    let nv_gpu_temp = read_nvidia_gpu_temp().await;

    let cpu_temp_str = cpu_temp.map(|t| format!("  —  {:.0}°C", t)).unwrap_or_default();
    let cpu_label_text = format!("CPU  {}  ({} cores)  —  {:.0}%{}", cpu_model, cpu_cores, cpu_usage, cpu_temp_str);

    let gpu_labels = gpus.iter().map(|gpu_name| {
        let temp = if gpu_name.to_lowercase().contains("nvidia") {
            nv_gpu_temp.or(tp_gpu_temp)
        } else {
            tp_gpu_temp
        };
        let temp_str = temp.map(|t| format!("  —  {:.0}°C", t)).unwrap_or_default();
        let text = format!("GPU  {}{}", gpu_name, temp_str);
        Label::new(&text).with_font_size(12.0).with_color([212, 212, 212])
    }).collect();

    HardwareState {
        cpu_model,
        cpu_usage,
        cpu_cores,
        gpus,
        loaded: true,
        cpu_label: Label::new(&cpu_label_text).with_font_size(12.0).with_color([212, 212, 212]),
        gpu_labels,
        processes,
        cpu_list_box: ScrollingList::new(24.0, 2.0),
    }
}

const TEXT_FG: [f32; 4] = [0.83, 0.83, 0.83, 1.0];
const TEXT_DIM: [f32; 4] = [0.53, 0.53, 0.60, 1.0];

pub fn view(state: &mut HardwareState, cx: f32, cy: f32, cw: f32, _ch: f32, root_focused: bool) -> PageContent {
    let mut pc = PageContent::new();
    let mut y = cy + 12.0;

    // ── CPU Section ──
    let mut sec = Section::new(&mut pc, cx, y, cw, "CPU");
    if !state.loaded {
        sec.text(&mut pc, "Loading CPU model and utilization...", 12.0, 0.0, 12.0, TEXT_FG);
        sec.spacing(10.0);
    } else {
        // CPU Info Label
        sec.widget(&mut pc, &mut state.cpu_label, 12.0, cw - 24.0, 26.0);
        sec.spacing(12.0);

        // Scrolling box configuration for process list
        let list_box_x = cx + 12.0;
        let list_box_y = sec.ay();
        let list_box_w = cw - 24.0;
        let list_box_h = 220.0;
        
        // Render the standardized ScrollBox widget
        clear_ui::layout::render_widget(&mut pc, &mut state.cpu_list_box, list_box_x, list_box_y, list_box_w, list_box_h);

        // Header for process list columns (drawn static on top of the ScrollBox background)
        let header_h = 22.0;
        pc.rect([0.12, 0.12, 0.16, 0.5], list_box_x + 1.0, list_box_y + 1.0, list_box_w - 2.0, header_h);
        pc.rect([0.18, 0.18, 0.24, 1.0], list_box_x + 1.0, list_box_y + header_h, list_box_w - 2.0, 1.0); // Divider
        
        pc.text("PID", list_box_x + 12.0, list_box_y + 5.0, 11.0, [0.53, 0.53, 0.60, 1.0]);
        pc.text("COMMAND", list_box_x + 80.0, list_box_y + 5.0, 11.0, [0.53, 0.53, 0.60, 1.0]);
        pc.text("CPU %", list_box_x + list_box_w - 60.0, list_box_y + 5.0, 11.0, [0.53, 0.53, 0.60, 1.0]);

        let row_h = 24.0;
        let row_gap = 2.0;
        // Update ScrollingList bounds for the scrollable viewport (which starts below the header)
        state.cpu_list_box.update_bounds(state.processes.len(), list_box_y + header_h, list_box_h - header_h - 6.0);

        // Visible process rows rendering (virtualized/clipped)
        
        for (idx, (pid, cpu, comm)) in state.processes.iter().enumerate() {
            if let Some(draw_y) = state.cpu_list_box.get_item_draw_y(idx, 4.0) {
                // Standard row action button (transparent background, highlights on hover)
                pc.button(
                    "",
                    list_box_x + 2.0,
                    draw_y,
                    list_box_w - 16.0,
                    row_h,
                    [0.0, 0.0, 0.0, 0.0],
                    [1.0, 1.0, 1.0, 0.06],
                    [0.0, 0.0, 0.0, 0.0],
                    crate::app::AppAction::Hardware(HardwareMessage::None),
                );
                
                pc.text(pid, list_box_x + 12.0, draw_y + 6.0, 12.0, [0.80, 0.80, 0.85, 1.0]);
                pc.text(comm, list_box_x + 80.0, draw_y + 6.0, 12.0, [0.80, 0.80, 0.85, 1.0]);
                pc.text(&format!("{}%", cpu), list_box_x + list_box_w - 60.0, draw_y + 6.0, 12.0, [0.56, 0.83, 0.56, 1.0]);
            }
        }
        
        if state.processes.is_empty() {
            pc.text("No active processes", list_box_x + 12.0, list_box_y + header_h + 16.0, 12.0, TEXT_DIM);
        }

        sec.content_y += list_box_h;
    }
    y = sec.finish_focused(&mut pc, root_focused);

    // ── GPU Section ──
    let mut sec_gpu = Section::new(&mut pc, cx, y, cw, "GPU");
    if !state.loaded {
        sec_gpu.text(&mut pc, "Loading GPU models...", 12.0, 0.0, 12.0, TEXT_FG);
        sec_gpu.spacing(10.0);
    } else {
        for (i, gpu_lbl) in state.gpu_labels.iter_mut().enumerate() {
            if i > 0 { sec_gpu.spacing(12.0); }
            sec_gpu.widget(&mut pc, gpu_lbl, 12.0, cw - 24.0, 26.0);
        }
    }
    sec_gpu.finish(&mut pc);

    pc
}

pub fn update(state: &mut HardwareState, msg: HardwareMessage) {
    match msg {
        HardwareMessage::Refreshed(new) => {
            state.loaded = new.loaded;
            state.cpu_model = new.cpu_model;
            state.cpu_usage = new.cpu_usage;
            state.cpu_cores = new.cpu_cores;
            state.gpus = new.gpus;
            state.cpu_label = new.cpu_label;
            state.gpu_labels = new.gpu_labels;
            state.processes = new.processes;
            let old_scroll = state.cpu_list_box.scroll_y();
            state.cpu_list_box = new.cpu_list_box;
            state.cpu_list_box.set_scroll_y(old_scroll);
        }
        HardwareMessage::None => {}
    }
}
