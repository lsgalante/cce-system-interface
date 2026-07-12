use crate::app::{AppAction, PageContent, SectionContextExt};
use cce_ui::layout::{render_widget, PageLayoutBuilder, LayoutStrategy};
use cce_ui::widget::{Label, Dropdown, InfoBox, Element, Button, Container};

#[derive(Debug, Clone, Default)]
pub struct BatteryInfo {
    pub percentage: f32,
    pub state: String,
    pub energy: f64,
    pub energy_full: f64,
    pub energy_rate: f64,
    pub time_to_empty: i64,
    pub time_to_full: i64,
    pub vendor: String,
    pub model: String,
}

#[derive(Debug, Clone)]
pub struct NotificationsConfig {
    pub enable: bool,
    pub bell: String,
    pub duration: i32,
}

#[derive(Clone)]
pub struct SystemState {
    pub hostname: String,
    pub kernel: String,
    pub uptime: String,
    pub loaded: bool,

    // Hardware status
    pub cpu_model: String,
    pub cpu_usage: f32,
    pub cpu_cores: u32,
    pub gpus: Vec<String>,
    pub gpu_strings: Vec<String>,
    pub cpu_label: cce_ui::widget::Adapted<cce_ui::widget::Label>,
    pub cpu_usage_label: cce_ui::widget::Adapted<cce_ui::widget::Label>,
    pub cpu_temp_label: cce_ui::widget::Adapted<cce_ui::widget::Label>,

    // Power-related fields
    pub battery: BatteryInfo,
    pub on_ac: bool,
    pub cpu_powersave: bool,
    pub gpu_powersave: bool,
    pub cpu_gov_menu: cce_ui::widget::Adapted<Dropdown>,
    pub gpu_gov_menu: cce_ui::widget::Adapted<Dropdown>,

    // Native layout tracking and widgets
    pub initialized: bool,
    pub sender: Option<calloop::channel::Sender<AppAction>>,
    pub hostname_label: cce_ui::widget::Adapted<cce_ui::widget::Label>,
    pub uptime_label: cce_ui::widget::Adapted<cce_ui::widget::Label>,

    pub battery_label_pct: cce_ui::widget::Adapted<cce_ui::widget::Label>,
    pub battery_label_state: cce_ui::widget::Adapted<cce_ui::widget::Label>,
    pub battery_label_time: cce_ui::widget::Adapted<cce_ui::widget::Label>,
    pub battery_label_details: cce_ui::widget::Adapted<cce_ui::widget::Label>,
    pub battery_label_ac: cce_ui::widget::Adapted<cce_ui::widget::Label>,

    pub cpu_info_box: cce_ui::widget::Adapted<cce_ui::widget::InfoBox>,
    pub gpu_info_box: cce_ui::widget::Adapted<cce_ui::widget::InfoBox>,

    pub suspend_btn: cce_ui::widget::Adapted<cce_ui::widget::Button>,
    pub hibernate_btn: cce_ui::widget::Adapted<cce_ui::widget::Button>,
    pub reboot_btn: cce_ui::widget::Adapted<cce_ui::widget::Button>,
    pub poweroff_btn: cce_ui::widget::Adapted<cce_ui::widget::Button>,
    pub force_shutdown_btn: cce_ui::widget::Adapted<cce_ui::widget::Button>,


    pub actions_row: Container,
}

impl std::fmt::Debug for SystemState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SystemState")
            .field("hostname", &self.hostname)
            .field("kernel", &self.kernel)
            .field("uptime", &self.uptime)
            .field("loaded", &self.loaded)
            .finish()
    }
}

impl Default for SystemState {
    fn default() -> Self {
        Self {
            hostname: String::new(),
            kernel: String::new(),
            uptime: String::new(),
            loaded: false,

            cpu_model: String::new(),
            cpu_usage: 0.0,
            cpu_cores: 0,
            gpus: Vec::new(),
            gpu_strings: Vec::new(),
            cpu_label: Label::new("CPU Info"),
            cpu_usage_label: Label::new("CPU Usage"),
            cpu_temp_label: Label::new("CPU Temp"),

            battery: BatteryInfo::default(),
            on_ac: true,
            cpu_powersave: false,
            gpu_powersave: false,
            cpu_gov_menu: Dropdown::new(
                vec!["Performance".to_string(), "Powersave".to_string()],
                0,
            ).with_label("CPU Governor"),
            gpu_gov_menu: Dropdown::new(
                vec!["Default (80W)".to_string(), "Eco Cap (5W)".to_string()],
                0,
            ).with_label("GPU Power Limit"),

            initialized: false,
            sender: None,
            hostname_label: Label::new(""),
            uptime_label: Label::new(""),

            battery_label_pct: Label::new("").with_font_size(24.0).with_color([92, 143, 97]),
            battery_label_state: Label::new("").with_font_size(12.0).with_color([135, 135, 153]),
            battery_label_time: Label::new("").with_font_size(12.0).with_color([135, 135, 153]),
            battery_label_details: Label::new("").with_font_size(11.0).with_color([135, 135, 153]),
            battery_label_ac: Label::new("").with_font_size(14.0).with_color([212, 212, 212]),

            cpu_info_box: InfoBox::new("CPU Governor", vec![]),
            gpu_info_box: InfoBox::new("GPU Power Limit", vec![]),

            suspend_btn: Button::new(0.0, 0.0, 0.0, 32.0)
                .with_label("Suspend")
                .with_bg([0.20, 0.33, 0.22, 1.0])
                .with_hover_bg([0.25, 0.30, 0.26, 1.0])
                .with_label_color([1.0, 1.0, 1.0, 1.0]),
            hibernate_btn: Button::new(0.0, 0.0, 0.0, 32.0)
                .with_label("Hibernate")
                .with_bg([0.20, 0.33, 0.22, 1.0])
                .with_hover_bg([0.25, 0.30, 0.26, 1.0])
                .with_label_color([1.0, 1.0, 1.0, 1.0]),
            reboot_btn: Button::new(0.0, 0.0, 0.0, 32.0)
                .with_label("Reboot")
                .with_bg([0.67, 0.20, 0.20, 1.0])
                .with_hover_bg([0.25, 0.30, 0.26, 1.0])
                .with_label_color([1.0, 1.0, 1.0, 1.0]),
            poweroff_btn: Button::new(0.0, 0.0, 0.0, 32.0)
                .with_label("Power Off")
                .with_bg([0.67, 0.20, 0.20, 1.0])
                .with_hover_bg([0.25, 0.30, 0.26, 1.0])
                .with_label_color([1.0, 1.0, 1.0, 1.0]),
            force_shutdown_btn: Button::new(0.0, 0.0, 0.0, 32.0)
                .with_label("Force Shutdown")
                .with_bg([0.67, 0.20, 0.20, 1.0])
                .with_hover_bg([0.25, 0.30, 0.26, 1.0])
                .with_label_color([1.0, 1.0, 1.0, 1.0]),

            actions_row: Container::new().with_layout(cce_ui::widget::ColumnsLayout {
                padding_x: 0.0,
                padding_y: 0.0,
                spacing: 8.0,
            }),
        }
    }
}

#[derive(Debug, Clone)]
pub enum SystemMessage {
    Refreshed(SystemInfo),
    Suspend,
    Hibernate,
    Reboot,
    PowerOff,
    ForceShutdown,

    // Moved variants
    SetCpuPerformance,
    SetCpuPowersave,
    SetGpuDefault,
    SetGpuPowersave,
}

// ── zbus proxies ────────────────────────────────────────────────────

#[zbus::proxy(
    interface = "org.freedesktop.UPower.Device",
    default_service = "org.freedesktop.UPower",
    default_path = "/org/freedesktop/UPower/devices/battery_BAT0"
)]
trait UpowerBattery {
    #[zbus(property)]
    fn percentage(&self) -> zbus::Result<f64>;
    #[zbus(property)]
    fn state(&self) -> zbus::Result<u32>;
    #[zbus(property)]
    fn energy(&self) -> zbus::Result<f64>;
    #[zbus(property)]
    fn energy_full(&self) -> zbus::Result<f64>;
    #[zbus(property)]
    fn energy_rate(&self) -> zbus::Result<f64>;
    #[zbus(property)]
    fn time_to_empty(&self) -> zbus::Result<i64>;
    #[zbus(property)]
    fn time_to_full(&self) -> zbus::Result<i64>;
    #[zbus(property)]
    fn vendor(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn model(&self) -> zbus::Result<String>;
}

#[zbus::proxy(
    interface = "org.freedesktop.UPower",
    default_service = "org.freedesktop.UPower",
    default_path = "/org/freedesktop/UPower"
)]
trait UpowerDaemon {
    #[zbus(property, name = "OnBattery")]
    fn on_battery(&self) -> zbus::Result<bool>;
}

// ── Helpers ─────────────────────────────────────────────────────────

fn format_duration(secs: i64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    if h > 0 { format!("{}h {}m", h, m) } else { format!("{}m", m) }
}

fn spawn_cpu_power(powersave: bool) {
    let script = if powersave { "cpu-powersave-on" } else { "cpu-powersave-off" };
    let mut cmd = std::process::Command::new("pkexec");
    cmd.arg(cce_ui::config::data_home().join("cce-settings").join("helpers").join(script));
    let _ = cce_ui::process::spawn_detached(cmd);
}

fn spawn_gpu_power(powersave: bool) {
    let script = if powersave { "gpu-powersave-on" } else { "gpu-powersave-off" };
    let mut cmd = std::process::Command::new("pkexec");
    cmd.arg(cce_ui::config::data_home().join("cce-settings").join("helpers").join(script));
    let _ = cce_ui::process::spawn_detached(cmd);
}

fn current_cpu_governor() -> String {
    std::fs::read_to_string("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor")
        .unwrap_or_default().trim().to_string()
}

async fn current_gpu_power_cap() -> bool {
    tokio::process::Command::new("nvidia-smi")
        .args(["--query-gpu=power.limit", "--format=csv,noheader,nounits"])
        .output().await.ok()
        .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse::<f32>().ok())
        .map(|w| w <= 10.0).unwrap_or(false)
}

async fn fetch_upower() -> (BatteryInfo, bool) {
    let conn = match zbus::Connection::system().await {
        Ok(c) => c,
        Err(_) => return (BatteryInfo::default(), true),
    };

    let battery = match UpowerBatteryProxy::new(&conn).await {
        Ok(proxy) => BatteryInfo {
            percentage: proxy.percentage().await.unwrap_or(0.0) as f32,
            state: {
                let s = proxy.state().await.unwrap_or(0);
                match s { 1 => "charging", 2 => "discharging", 4 => "fully-charged", _ => "unknown" }.into()
            },
            energy: proxy.energy().await.unwrap_or(0.0),
            energy_full: proxy.energy_full().await.unwrap_or(0.0),
            energy_rate: proxy.energy_rate().await.unwrap_or(0.0),
            time_to_empty: proxy.time_to_empty().await.unwrap_or(0),
            time_to_full: proxy.time_to_full().await.unwrap_or(0),
            vendor: proxy.vendor().await.unwrap_or_default(),
            model: proxy.model().await.unwrap_or_default(),
        },
        Err(_) => BatteryInfo::default(),
    };

    let on_ac = match UpowerDaemonProxy::new(&conn).await {
        Ok(proxy) => !proxy.on_battery().await.unwrap_or(false),
        Err(_) => true,
    };

    (battery, on_ac)
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

fn spawn_systemctl(action: &str) {
    let mut cmd = std::process::Command::new("systemctl");
    cmd.arg(action);
    let _ = cce_ui::process::spawn_detached(cmd);
}

fn spawn_systemctl_force(action: &str) {
    let mut cmd = std::process::Command::new("systemctl");
    cmd.arg(action);
    cmd.arg("-f");
    cmd.arg("-f");
    let _ = cce_ui::process::spawn_detached(cmd);
}

#[derive(Debug, Clone, Default)]
pub struct SystemInfo {
    pub hostname: String,
    pub kernel: String,
    pub uptime: String,
    pub cpu_model: String,
    pub cpu_cores: u32,
    pub cpu_usage: f32,
    pub gpus: Vec<String>,
    pub gpu_strings: Vec<String>,
    pub battery: BatteryInfo,
    pub on_ac: bool,
    pub cpu_powersave: bool,
    pub gpu_powersave: bool,
}

pub async fn fetch_system_state() -> SystemInfo {
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

    static CPU_INFO: std::sync::OnceLock<(String, u32)> = std::sync::OnceLock::new();
    let (cpu_model, cpu_cores) = CPU_INFO.get_or_init(|| {
        let output = std::process::Command::new("lscpu")
            .output().ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default();
        let model = output.lines()
            .find(|l| l.contains("Model name"))
            .and_then(|l| l.split(':').nth(1))
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        let cores = output.lines()
            .find(|l| l.contains("CPU(s)"))
            .and_then(|l| {
                let rest = l.split(':').nth(1).unwrap_or("").trim();
                rest.split_whitespace().next().and_then(|n| n.parse::<u32>().ok())
            })
            .unwrap_or(0);
        (model, cores)
    }).clone();

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

    static GPUS_INFO: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    let gpus = GPUS_INFO.get_or_init(|| {
        let mut list = Vec::new();
        if let Some(o) = std::process::Command::new("lspci").output().ok() {
            for line in String::from_utf8_lossy(&o.stdout).lines() {
                if line.contains("VGA") || line.contains("3D") {
                    if let Some(name) = line.split(':').nth(2) {
                        let trimmed = name.trim().to_string();
                        if !trimmed.is_empty() {
                            list.push(trimmed);
                        }
                    }
                }
            }
        }
        list
    }).clone();


    let tp_gpu_temp = read_thinkpad_gpu_temp();
    let nv_gpu_temp = read_nvidia_gpu_temp().await;

    let gpu_strings = gpus.iter().map(|gpu_name| {
        let temp = if gpu_name.to_lowercase().contains("nvidia") {
            nv_gpu_temp.or(tp_gpu_temp)
        } else {
            tp_gpu_temp
        };
        let temp_str = temp.map(|t| format!("  —  {:.0}°C", t)).unwrap_or_default();
        format!("GPU  {}{}", gpu_name, temp_str)
    }).collect();

    let (battery, on_ac) = fetch_upower().await;
    let cpu_powersave = current_cpu_governor() == "powersave";
    let gpu_powersave = current_gpu_power_cap().await;

    SystemInfo {
        hostname,
        kernel,
        uptime,
        cpu_model,
        cpu_usage,
        cpu_cores,
        gpus,
        gpu_strings,
        battery,
        on_ac,
        cpu_powersave,
        gpu_powersave,
    }
}

const TEXT_FG: [f32; 4] = [0.83, 0.83, 0.83, 1.0];
const TEXT_DIM: [f32; 4] = [0.53, 0.53, 0.60, 1.0];
const ACCENT: [f32; 4] = [0.36, 0.56, 0.38, 1.0];
const RED: [f32; 4] = [1.0, 0.33, 0.33, 1.0];
const ORANGE: [f32; 4] = [1.0, 0.73, 0.20, 1.0];
const BTN_HOVER: [f32; 4] = [0.25, 0.30, 0.26, 1.0];
const DANGER_BG: [f32; 4] = [0.67, 0.20, 0.20, 1.0];
const SAFE_BG: [f32; 4] = [0.20, 0.33, 0.22, 1.0];
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

pub fn view(state: &mut SystemState, cx: f32, cy: f32, cw: f32, ch: f32, _root_focused: bool, sec_focused: &[bool], layout: &mut dyn LayoutStrategy, ctx: &mut cce_ui::context::UiContext) -> PageContent {
    let mut final_pc = PageContent::new();
    let sec_w = 320.0f32;
    let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(8);

    // ── 1. System Section ──
    builder.add_section(&mut final_pc, "System", false, |sec| {
        if !state.loaded {
            sec.text("Loading system information...", 12.0, 0.0, 14.0, TEXT_FG);
        } else {
            sec.text(&format!("{}  —  Linux {}", state.hostname, state.kernel), 12.0, 0.0, 14.0, TEXT_FG);
            sec.text(&format!("Uptime: {}", state.uptime), 12.0, 0.0, 12.0, TEXT_DIM);
        }
    });

    // ── 2. System Actions Section ──
    builder.add_section(&mut final_pc, "System Actions", false, |sec| {
        let mut stack = sec.vstack(8.0);
        let act_btn_h = 32.0;

        stack.add_row(4, 8.0, act_btn_h, |ctx, i, x, w| {
            match i {
                0 => {
                    ctx.button("Suspend", x, ctx.ay(), w, act_btn_h,
                        SAFE_BG, BTN_HOVER, WHITE, AppAction::SystemInfo(SystemMessage::Suspend));
                }
                1 => {
                    ctx.button("Hibernate", x, ctx.ay(), w, act_btn_h,
                        SAFE_BG, BTN_HOVER, WHITE, AppAction::SystemInfo(SystemMessage::Hibernate));
                }
                2 => {
                    ctx.button("Reboot", x, ctx.ay(), w, act_btn_h,
                        DANGER_BG, BTN_HOVER, WHITE, AppAction::SystemInfo(SystemMessage::Reboot));
                }
                3 => {
                    ctx.button("Power Off", x, ctx.ay(), w, act_btn_h,
                        DANGER_BG, BTN_HOVER, WHITE, AppAction::SystemInfo(SystemMessage::PowerOff));
                }
                _ => {}
            }
        });

        stack.add_row(1, 0.0, act_btn_h, |ctx, _, x, w| {
            ctx.button("Force Shutdown", x, ctx.ay(), w, act_btn_h,
                DANGER_BG, BTN_HOVER, WHITE, AppAction::SystemInfo(SystemMessage::ForceShutdown));
        });
    });

    // ── 3. CPU Section ──
    builder.add_section(&mut final_pc, "CPU", false, |sec| {
        if !state.loaded {
            sec.text("Loading CPU model and utilization...", 12.0, 0.0, 12.0, TEXT_FG);
        } else {
            // Same strings the old Label widgets carried, stacked vertically (the
            // grid-column widget placement overlapped them at narrow widths).
            sec.text(&format!("CPU  {}  ({} cores)", state.cpu_model, state.cpu_cores), 12.0, 0.0, 12.0, TEXT_FG);
            sec.text(&format!("Usage  {:.0}%", state.cpu_usage), 12.0, 0.0, 12.0, TEXT_FG);
            let cpu_temp_text = read_cpu_temp().map(|t| format!("Temp  {:.0}°C", t)).unwrap_or_else(|| "Temp  N/A".to_string());
            sec.text(&cpu_temp_text, 12.0, 0.0, 12.0, TEXT_FG);
        }
    });

    // ── 4. GPU Section ──
    builder.add_section(&mut final_pc, "GPU", false, |sec_gpu| {
        if !state.loaded {
            sec_gpu.text("Loading GPU models...", 12.0, 0.0, 12.0, TEXT_FG);
        } else {
            for gpu_text in state.gpu_strings.iter() {
                sec_gpu.text(gpu_text, 12.0, 0.0, 12.0, TEXT_FG);
            }
        }
    });

    // ── 5. CPU Governor Section ──
    builder.add_section(&mut final_pc, "CPU Governor", sec_focused.get(4).copied().unwrap_or(false), |sec_gov| {
        let rx = sec_gov.left;
        if !state.loaded {
            sec_gov.text("Loading CPU governor...", 12.0, 0.0, 12.0, TEXT_DIM);
        } else {
            sec_gov.widget(&mut state.cpu_gov_menu, 12.0, sec_gov.cw - 24.0, 26.0, ctx);

            let (info_title, info_lines) = if state.cpu_powersave {
                (
                    "CPU Governor: Powersave",
                    vec![
                        "• Active: powersave".to_string(),
                        "• Governor set to powersave — lower power, slower burst".to_string(),
                    ],
                )
            } else {
                (
                    "CPU Governor: Performance",
                    vec![
                        "• Active: performance".to_string(),
                        "• Governor set to performance".to_string(),
                    ],
                )
            };

            let mut info_box = InfoBox::new(info_title, info_lines);
            let info_h = 80.0;
            let info_y = sec_gov.ay();
            render_widget(sec_gov.pc, &mut info_box, rx + 12.0, info_y, sec_gov.cw - 24.0, info_h, ctx);
            // Advance the section cursor past the hand-placed box.
            sec_gov.content_y = sec_gov.content_y.max(info_y + info_h);
            for h in &mut sec_gov.grid.col_heights {
                *h = h.max(sec_gov.content_y);
            }
        }
    });

    // ── 6. GPU Power Section ──
    builder.add_section(&mut final_pc, "GPU Power", sec_focused.get(5).copied().unwrap_or(false), |sec_gpow| {
        let rx = sec_gpow.left;
        if !state.loaded {
            sec_gpow.text("Loading GPU power status...", 12.0, 0.0, 12.0, TEXT_DIM);
        } else {
            sec_gpow.widget(&mut state.gpu_gov_menu, 12.0, sec_gpow.cw - 24.0, 26.0, ctx);

            let (info_title, info_lines) = if state.gpu_powersave {
                (
                    "GPU Power Limit: Eco Cap",
                    vec![
                        "• Mode: 5W Cap".to_string(),
                        "• NVIDIA power limit capped at 5W — minimal draw".to_string(),
                    ],
                )
            } else {
                (
                    "GPU Power Limit: Default",
                    vec![
                        "• Mode: 80W Default".to_string(),
                        "• NVIDIA running at default power limit".to_string(),
                    ],
                )
            };

            let mut info_box = InfoBox::new(info_title, info_lines);
            let info_h = 80.0;
            let info_y = sec_gpow.ay();
            render_widget(sec_gpow.pc, &mut info_box, rx + 12.0, info_y, sec_gpow.cw - 24.0, info_h, ctx);
            // Advance the section cursor past the hand-placed box.
            sec_gpow.content_y = sec_gpow.content_y.max(info_y + info_h);
            for h in &mut sec_gpow.grid.col_heights {
                *h = h.max(sec_gpow.content_y);
            }
        }
    });

    // ── 7. Battery Section ──
    builder.add_section(&mut final_pc, "Battery", false, |sec_bat| {
        if !state.loaded {
            sec_bat.text("Loading battery status...", 12.0, 0.0, 12.0, TEXT_DIM);
        } else {
            let bat = &state.battery;
            let bat_icon = match bat.state.as_str() {
                "charging" => "+",
                "fully-charged" => "=",
                _ => "",
            };

            let pct_color = if bat.percentage < 20.0 { RED }
                else if bat.percentage < 50.0 { ORANGE }
                else { ACCENT };

            let pct_str = format!("{} {:.0}%", bat_icon, bat.percentage);
            sec_bat.text(&pct_str, 12.0, 0.0, 24.0, pct_color);

            let state_str = format!("{}  •  {:.1}W  •  {:.1}/{:.1} Wh",
                bat.state, bat.energy_rate, bat.energy, bat.energy_full);
            sec_bat.text(&state_str, 12.0, 0.0, 12.0, TEXT_DIM);

            let time_str = if bat.time_to_empty > 0 {
                format!("Time remaining: {}", format_duration(bat.time_to_empty))
            } else if bat.time_to_full > 0 {
                format!("Time to full: {}", format_duration(bat.time_to_full))
            } else { String::new() };
            if !time_str.is_empty() {
                sec_bat.text(&time_str, 12.0, 0.0, 12.0, TEXT_DIM);
            }

            let detail_str = format!("{}  {}", bat.vendor, bat.model);
            sec_bat.text(&detail_str, 12.0, 0.0, 11.0, TEXT_DIM);

            let ac_str = if state.on_ac { "On AC Power" } else { "On Battery" };
            sec_bat.text(ac_str, 12.0, 0.0, 14.0, TEXT_FG);
        }
    });



    final_pc
}

pub fn update(state: &mut SystemState, msg: SystemMessage, ctx: &mut cce_ui::context::UiContext) {
    match msg {
        SystemMessage::Refreshed(new) => {
            state.hostname = new.hostname;
            state.kernel = new.kernel;
            state.uptime = new.uptime;
            state.loaded = true;

            state.cpu_model = new.cpu_model;
            state.cpu_usage = new.cpu_usage;
            state.cpu_cores = new.cpu_cores;
            state.gpus = new.gpus;
            state.gpu_strings = new.gpu_strings.clone();

            state.battery = new.battery;
            state.on_ac = new.on_ac;
            state.cpu_powersave = new.cpu_powersave;
            state.gpu_powersave = new.gpu_powersave;
            state.cpu_gov_menu.selected = if new.cpu_powersave { 1 } else { 0 };
            state.gpu_gov_menu.selected = if new.gpu_powersave { 1 } else { 0 };

            if state.loaded {
                state.hostname_label.set_text(&format!("{}  —  Linux {}", state.hostname, state.kernel));
                state.uptime_label.set_text(&format!("Uptime: {}", state.uptime));

                let cpu_label_text = format!("CPU  {}  ({} cores)", state.cpu_model, state.cpu_cores);
                let cpu_usage_text = format!("Usage  {:.0}%", state.cpu_usage);
                let cpu_temp_text = read_cpu_temp().map(|t| format!("Temp  {:.0}°C", t)).unwrap_or_else(|| "Temp  N/A".to_string());

                state.cpu_label.set_text(&cpu_label_text);
                state.cpu_usage_label.set_text(&cpu_usage_text);
                state.cpu_temp_label.set_text(&cpu_temp_text);

                // Update info boxes
                let (cpu_title, cpu_lines) = if state.cpu_powersave {
                    (
                        "CPU Governor: Powersave",
                        vec![
                            "• Active: powersave".to_string(),
                            "• Governor set to powersave — lower power, slower burst".to_string(),
                        ],
                    )
                } else {
                    (
                        "CPU Governor: Performance",
                        vec![
                            "• Active: performance".to_string(),
                            "• Governor set to performance".to_string(),
                        ],
                    )
                };
                state.cpu_info_box.title = cpu_title.to_string();
                state.cpu_info_box.lines = cpu_lines;

                let (gpu_title, gpu_lines) = if state.gpu_powersave {
                    (
                        "GPU Power Limit: Eco Cap",
                        vec![
                            "• Mode: 5W Cap".to_string(),
                            "• NVIDIA power limit capped at 5W — minimal draw".to_string(),
                        ],
                    )
                } else {
                    (
                        "GPU Power Limit: Default",
                        vec![
                            "• Mode: 80W Default".to_string(),
                            "• NVIDIA running at default power limit".to_string(),
                        ],
                    )
                };
                state.gpu_info_box.title = gpu_title.to_string();
                state.gpu_info_box.lines = gpu_lines;

                // Update battery labels
                let bat = &state.battery;
                let bat_icon = match bat.state.as_str() {
                    "charging" => "+",
                    "fully-charged" => "=",
                    _ => "",
                };

                let pct_color = if bat.percentage < 20.0 { [255, 84, 84] }
                    else if bat.percentage < 50.0 { [255, 186, 51] }
                    else { [92, 143, 97] };

                let pct_str = format!("{} {:.0}%", bat_icon, bat.percentage);
                state.battery_label_pct.set_text(&pct_str);
                state.battery_label_pct.set_color(pct_color);

                let state_str = format!("{}  •  {:.1}W  •  {:.1}/{:.1} Wh",
                    bat.state, bat.energy_rate, bat.energy, bat.energy_full);
                state.battery_label_state.set_text(&state_str);

                let time_str = if bat.time_to_empty > 0 {
                    format!("Time remaining: {}", format_duration(bat.time_to_empty))
                } else if bat.time_to_full > 0 {
                    format!("Time to full: {}", format_duration(bat.time_to_full))
                } else { String::new() };
                state.battery_label_time.set_text(&time_str);

                let detail_str = format!("{}  {}", bat.vendor, bat.model);
                state.battery_label_details.set_text(&detail_str);

                let ac_str = if state.on_ac { "On AC Power" } else { "On Battery" };
                state.battery_label_ac.set_text(ac_str);

                state.hostname_label.mark_dirty(ctx);
            }
        }
        SystemMessage::Suspend => spawn_systemctl("suspend"),
        SystemMessage::Hibernate => spawn_systemctl("hibernate"),
        SystemMessage::Reboot => spawn_systemctl("reboot"),
        SystemMessage::PowerOff => spawn_systemctl("poweroff"),
        SystemMessage::ForceShutdown => spawn_systemctl_force("poweroff"),

        SystemMessage::SetCpuPerformance => {
            state.cpu_powersave = false;
            state.cpu_gov_menu.selected = 0;
            spawn_cpu_power(false);
        }
        SystemMessage::SetCpuPowersave => {
            state.cpu_powersave = true;
            state.cpu_gov_menu.selected = 1;
            spawn_cpu_power(true);
        }
        SystemMessage::SetGpuDefault => {
            state.gpu_powersave = false;
            state.gpu_gov_menu.selected = 0;
            spawn_gpu_power(false);
        }
        SystemMessage::SetGpuPowersave => {
            state.gpu_powersave = true;
            state.gpu_gov_menu.selected = 1;
            spawn_gpu_power(true);
        }
    }
}



impl crate::pages::AppPage for SystemState {
    // Sections: [System, System Actions, CPU, GPU, CPU Governor, GPU Power, Battery]
    fn section_widgets(&mut self) -> Vec<Vec<*mut (dyn cce_ui::widget::Element + 'static)>> {
        use cce_ui::widget::Element;
        vec![
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            vec![self.cpu_gov_menu.as_ptr_mut()],
            vec![self.gpu_gov_menu.as_ptr_mut()],
            Vec::new(),
        ]
    }

    fn view(
        &mut self,
        cx: f32,
        cy: f32,
        cw: f32,
        ch: f32,
        root_focused: bool,
        sec_focused: &[bool],
        layout: &mut dyn LayoutStrategy,
        ctx: &mut cce_ui::context::UiContext,
    ) -> crate::app::PageContent {
        // Phase 6u: System used to render through the WIDGET TREE (the only page that
        // did) — its content reached the frame via the root aggregate walking
        // Switcher → Page(AdaptiveGridLayout) → sections → widgets. With that chain
        // dissolved, the page renders through the same immediate-mode view as every
        // other page (this free `view` predates the flip; it was never wired up).
        view(self, cx, cy, cw, ch, root_focused, sec_focused, layout, ctx)
    }

    fn propagate_widget_changes(&mut self, actions: &mut Vec<crate::app::AppAction>) {
        if self.cpu_gov_menu.take_change() {
            if self.cpu_gov_menu.selected == 0 {
                actions.push(crate::app::AppAction::SystemInfo(SystemMessage::SetCpuPerformance));
            } else {
                actions.push(crate::app::AppAction::SystemInfo(SystemMessage::SetCpuPowersave));
            }
        }
        if self.gpu_gov_menu.take_change() {
            if self.gpu_gov_menu.selected == 0 {
                actions.push(crate::app::AppAction::SystemInfo(SystemMessage::SetGpuDefault));
            } else {
                actions.push(crate::app::AppAction::SystemInfo(SystemMessage::SetGpuPowersave));
            }
        }
    }
}





#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_layout_grid() {
        let mut state = SystemState::default();
        let mut layout = cce_ui::layout::ColumnLayout::new(20.0);
        let sec_focused = vec![false, false, false, false, false, false, false];
        let mut ctx = cce_ui::context::UiContext::new();
        let pc = view(&mut state, 10.0, 20.0, 800.0, 600.0, false, &sec_focused, &mut layout, &mut ctx);
        assert!(!pc.rects.is_empty() || !pc.texts.is_empty());
    }
}
