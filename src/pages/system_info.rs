use crate::app::{AppAction, PageContent, SectionContextExt};
use cce_ui::layout::{render_widget, PageLayoutBuilder, LayoutStrategy};
use cce_ui::widget::{Label, Dropdown, InfoBox, Toggle, Spinbox};
use crate::config_manager::CONFIG_PATH;
use std::io::Write;
use std::fs;

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
    pub bell: bool,
    pub duration: i32,
}

#[derive(Debug, Clone)]
pub struct StatusData {
    pub font_size: u16,
    pub padding: u16,
    pub separators: bool,
    pub underline: bool,
    pub running: bool,
    pub bg_opacity: f32,
    pub bg_blur: f32,
}

#[derive(Debug, Clone)]
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
    pub cpu_label: Label,
    pub cpu_usage_label: Label,
    pub cpu_temp_label: Label,
    pub gpu_labels: Vec<Label>,

    // Power-related fields
    pub battery: BatteryInfo,
    pub on_ac: bool,
    pub cpu_powersave: bool,
    pub gpu_powersave: bool,
    pub cpu_gov_menu: Dropdown,
    pub gpu_gov_menu: Dropdown,

    // Notifications
    pub notifications_loaded: bool,
    pub notifications_enable: bool,
    pub notifications_enable_toggle: Toggle,
    pub notifications_bell: bool,
    pub notifications_bell_toggle: Toggle,
    pub notifications_duration: i32,
    pub notifications_duration_spinbox: Spinbox,

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
            cpu_label: Label::new("CPU Info"),
            cpu_usage_label: Label::new("CPU Usage"),
            cpu_temp_label: Label::new("CPU Temp"),
            gpu_labels: Vec::new(),

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

            notifications_loaded: false,
            notifications_enable: true,
            notifications_enable_toggle: Toggle::new().with_label("Enable Notifications").with_config(&get_config_path(), "enable"),
            notifications_bell: false,
            notifications_bell_toggle: Toggle::new().with_label("Play Bell Sound").with_config(&get_config_path(), "bell"),
            notifications_duration: 5,
            notifications_duration_spinbox: Spinbox::new(5, 1, 60, 1)
                .with_label("Notification Duration")
                .with_unit("s")
                .with_config(&get_config_path(), "duration"),

        }
    }
}

#[derive(Debug, Clone)]
pub enum SystemMessage {
    Refreshed(SystemState),
    Suspend,
    Hibernate,
    Reboot,
    PowerOff,

    // Moved variants
    SetCpuPerformance,
    SetCpuPowersave,
    SetGpuDefault,
    SetGpuPowersave,
    ToggleNotificationsEnable,
    ToggleNotificationsBell,
    SetNotificationsDuration(i32),
    SendTestNotification,
    NotificationsRefreshed(NotificationsConfig),

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
    cmd.arg(format!("/home/lsgalante/.local/share/cce-system-interface/helpers/{}", script));
    let _ = cce_ui::process::spawn_detached(cmd);
}

fn spawn_gpu_power(powersave: bool) {
    let script = if powersave { "gpu-powersave-on" } else { "gpu-powersave-off" };
    let mut cmd = std::process::Command::new("pkexec");
    cmd.arg(format!("/home/lsgalante/.local/share/cce-system-interface/helpers/{}", script));
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

    let cpu_temp = read_cpu_temp();
    let tp_gpu_temp = read_thinkpad_gpu_temp();
    let nv_gpu_temp = read_nvidia_gpu_temp().await;

    let cpu_label_text = format!("CPU  {}  ({} cores)", cpu_model, cpu_cores);
    let cpu_usage_text = format!("Usage  {:.0}%", cpu_usage);
    let cpu_temp_text = cpu_temp.map(|t| format!("Temp  {:.0}°C", t)).unwrap_or_else(|| "Temp  N/A".to_string());

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

    let (battery, on_ac) = fetch_upower().await;
    let cpu_powersave = current_cpu_governor() == "powersave";
    let gpu_powersave = current_gpu_power_cap().await;

    SystemState {
        hostname,
        kernel,
        uptime,
        loaded: true,

        cpu_model,
        cpu_usage,
        cpu_cores,
        gpus,
        cpu_label: Label::new(&cpu_label_text).with_font_size(12.0).with_color([212, 212, 212]),
        cpu_usage_label: Label::new(&cpu_usage_text).with_font_size(12.0).with_color([212, 212, 212]),
        cpu_temp_label: Label::new(&cpu_temp_text).with_font_size(12.0).with_color([212, 212, 212]),
        gpu_labels,

        battery,
        on_ac,
        cpu_powersave,
        gpu_powersave,
        cpu_gov_menu: Dropdown::new(
            vec!["Performance".to_string(), "Powersave".to_string()],
            if cpu_powersave { 1 } else { 0 },
        ).with_label("CPU Governor"),
        gpu_gov_menu: Dropdown::new(
            vec!["Default (80W)".to_string(), "Eco Cap (5W)".to_string()],
            if gpu_powersave { 1 } else { 0 },
        ).with_label("GPU Power Limit"),

        notifications_loaded: false,
        notifications_enable: true,
        notifications_enable_toggle: Toggle::new().with_label("Enable Notifications").with_config(&get_config_path(), "enable"),
        notifications_bell: false,
        notifications_bell_toggle: Toggle::new().with_label("Play Bell Sound").with_config(&get_config_path(), "bell"),
        notifications_duration: 5,
        notifications_duration_spinbox: Spinbox::new(5, 1, 60, 1)
            .with_label("Notification Duration")
            .with_unit("s")
            .with_config(&get_config_path(), "duration"),
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
    let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(9);

    // ── 1. System Section ──
    builder.add_section(&mut final_pc, "System", false, |sec| {
        if !state.loaded {
            sec.text("Loading system information...", 12.0, 0.0, 14.0, TEXT_FG);
            sec.spacing(10.0);
        } else {
            sec.text(&format!("{}  —  Linux {}", state.hostname, state.kernel), 12.0, 0.0, 14.0, TEXT_FG);
            sec.spacing(10.0);
            sec.text(&format!("Uptime: {}", state.uptime), 12.0, 0.0, 12.0, TEXT_DIM);
        }
    });

    // ── 2. System Actions Section ──
    builder.add_section(&mut final_pc, "System Actions", false, |sec| {
        let yt = sec.ay();
        let act_btn_h = 32.0;

        let cols = sec.row_layout(4, 8.0);
        for (i, &(x, w)) in cols.iter().enumerate() {
            match i {
                0 => {
                    sec.button("Suspend", x, yt, w, act_btn_h,
                        SAFE_BG, BTN_HOVER, WHITE, AppAction::SystemInfo(SystemMessage::Suspend));
                }
                1 => {
                    sec.button("Hibernate", x, yt, w, act_btn_h,
                        SAFE_BG, BTN_HOVER, WHITE, AppAction::SystemInfo(SystemMessage::Hibernate));
                }
                2 => {
                    sec.button("Reboot", x, yt, w, act_btn_h,
                        DANGER_BG, BTN_HOVER, WHITE, AppAction::SystemInfo(SystemMessage::Reboot));
                }
                3 => {
                    sec.button("Power Off", x, yt, w, act_btn_h,
                        DANGER_BG, BTN_HOVER, WHITE, AppAction::SystemInfo(SystemMessage::PowerOff));
                }
                _ => {}
            }
        }
        sec.spacing(12.0);
    });

    // ── 3. CPU Section ──
    builder.add_section(&mut final_pc, "CPU", false, |sec| {
        if !state.loaded {
            sec.text("Loading CPU model and utilization...", 12.0, 0.0, 12.0, TEXT_FG);
            sec.spacing(10.0);
        } else {
            // CPU Info Label
            sec.widget(&mut state.cpu_label, 12.0, sec.cw - 24.0, 26.0, ctx);
            sec.spacing(12.0);

            // CPU Usage Label
            sec.widget(&mut state.cpu_usage_label, 12.0, sec.cw - 24.0, 26.0, ctx);
            sec.spacing(12.0);

            // CPU Temp Label
            sec.widget(&mut state.cpu_temp_label, 12.0, sec.cw - 24.0, 26.0, ctx);
            sec.spacing(12.0);
        }
    });

    // ── 4. GPU Section ──
    builder.add_section(&mut final_pc, "GPU", false, |sec_gpu| {
        if !state.loaded {
            sec_gpu.text("Loading GPU models...", 12.0, 0.0, 12.0, TEXT_FG);
            sec_gpu.spacing(10.0);
        } else {
            for (i, gpu_lbl) in state.gpu_labels.iter_mut().enumerate() {
                if i > 0 { sec_gpu.spacing(12.0); }
                sec_gpu.widget(gpu_lbl, 12.0, sec_gpu.cw - 24.0, 26.0, ctx);
            }
        }
    });

    // ── 5. CPU Governor Section ──
    builder.add_section(&mut final_pc, "CPU Governor", sec_focused.get(4).copied().unwrap_or(false), |sec_gov| {
        let rx = sec_gov.left;
        if !state.loaded {
            sec_gov.text("Loading CPU governor...", 12.0, 0.0, 12.0, TEXT_DIM);
            sec_gov.spacing(18.0);
        } else {
            sec_gov.widget(&mut state.cpu_gov_menu, 12.0, sec_gov.cw - 24.0, 26.0, ctx);
            sec_gov.spacing(12.0);

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
            sec_gov.spacing(info_h + 12.0);
        }
    });

    // ── 6. GPU Power Section ──
    builder.add_section(&mut final_pc, "GPU Power", sec_focused.get(5).copied().unwrap_or(false), |sec_gpow| {
        let rx = sec_gpow.left;
        if !state.loaded {
            sec_gpow.text("Loading GPU power status...", 12.0, 0.0, 12.0, TEXT_DIM);
            sec_gpow.spacing(18.0);
        } else {
            sec_gpow.widget(&mut state.gpu_gov_menu, 12.0, sec_gpow.cw - 24.0, 26.0, ctx);
            sec_gpow.spacing(12.0);

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
            sec_gpow.spacing(info_h + 12.0);
        }
    });

    // ── 7. Battery Section ──
    builder.add_section(&mut final_pc, "Battery", false, |sec_bat| {
        if !state.loaded {
            sec_bat.text("Loading battery status...", 12.0, 0.0, 12.0, TEXT_DIM);
            sec_bat.spacing(18.0);
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
            sec_bat.spacing(30.0);

            let state_str = format!("{}  •  {:.1}W  •  {:.1}/{:.1} Wh",
                bat.state, bat.energy_rate, bat.energy, bat.energy_full);
            sec_bat.text(&state_str, 12.0, 0.0, 12.0, TEXT_DIM);
            sec_bat.spacing(18.0);

            let time_str = if bat.time_to_empty > 0 {
                format!("Time remaining: {}", format_duration(bat.time_to_empty))
            } else if bat.time_to_full > 0 {
                format!("Time to full: {}", format_duration(bat.time_to_full))
            } else { String::new() };
            if !time_str.is_empty() {
                sec_bat.text(&time_str, 12.0, 0.0, 12.0, TEXT_DIM);
                sec_bat.spacing(18.0);
            }

            let detail_str = format!("{}  {}", bat.vendor, bat.model);
            sec_bat.text(&detail_str, 12.0, 0.0, 11.0, TEXT_DIM);
            sec_bat.spacing(20.0);

            let ac_str = if state.on_ac { "On AC Power" } else { "On Battery" };
            sec_bat.text(ac_str, 12.0, 0.0, 14.0, TEXT_FG);
            sec_bat.spacing(20.0);
        }
    });

    // ── 8. System Notifications Section ──
    builder.add_section(&mut final_pc, "System Notifications", sec_focused.get(7).copied().unwrap_or(false), |sec2| {
        let sec_w = sec2.cw;
        state.notifications_enable_toggle.set_toggled(state.notifications_enable);
        sec2.widget_full(&mut state.notifications_enable_toggle, cce_ui::layout::toggle_height(), ctx);
        sec2.spacing(8.0);

        state.notifications_bell_toggle.set_toggled(state.notifications_bell);
        sec2.widget_full(&mut state.notifications_bell_toggle, cce_ui::layout::toggle_height(), ctx);
        sec2.spacing(16.0);

        state.notifications_duration_spinbox.value = state.notifications_duration;
        state.notifications_duration_spinbox.set_label("Notification Duration");
        sec2.widget(&mut state.notifications_duration_spinbox, 14.0, sec_w - 28.0, 44.0, ctx);
        sec2.spacing(16.0);

        let btn_h = 32.0;
        let btn_y = sec2.ay();
        let white_color = [1.0, 1.0, 1.0, 1.0];
        let btn_bg = [0.20, 0.40, 0.65, 1.0];
        let btn_hover = [0.28, 0.50, 0.78, 1.0];
        
        let cols = sec2.row_layout(1, 0.0);
        if let Some(&(x, w)) = cols.first() {
            sec2.button(
                "Send Test Notification",
                x,
                btn_y,
                w,
                btn_h,
                btn_bg,
                btn_hover,
                white_color,
                AppAction::SystemInfo(SystemMessage::SendTestNotification),
            );
        }
        sec2.spacing(12.0);
    });


    final_pc
}

pub fn update(state: &mut SystemState, msg: SystemMessage) {
    match msg {
        SystemMessage::Refreshed(new) => {
            state.hostname = new.hostname;
            state.kernel = new.kernel;
            state.uptime = new.uptime;
            state.loaded = new.loaded;

            state.cpu_model = new.cpu_model;
            state.cpu_usage = new.cpu_usage;
            state.cpu_cores = new.cpu_cores;
            state.gpus = new.gpus;
            state.cpu_label = new.cpu_label;
            state.cpu_usage_label = new.cpu_usage_label;
            state.cpu_temp_label = new.cpu_temp_label;
            state.gpu_labels = new.gpu_labels;

            state.battery = new.battery;
            state.on_ac = new.on_ac;
            state.cpu_powersave = new.cpu_powersave;
            state.gpu_powersave = new.gpu_powersave;
            state.cpu_gov_menu.selected = new.cpu_gov_menu.selected;
            state.gpu_gov_menu.selected = new.gpu_gov_menu.selected;
        }
        SystemMessage::Suspend => spawn_systemctl("suspend"),
        SystemMessage::Hibernate => spawn_systemctl("hibernate"),
        SystemMessage::Reboot => spawn_systemctl("reboot"),
        SystemMessage::PowerOff => spawn_systemctl("poweroff"),

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
        SystemMessage::ToggleNotificationsEnable => {
            state.notifications_enable = !state.notifications_enable;
            write_enable_notifications(state.notifications_enable);
        }
        SystemMessage::ToggleNotificationsBell => {
            state.notifications_bell = !state.notifications_bell;
            write_config_value("bell", &state.notifications_bell.to_string());
        }
        SystemMessage::SetNotificationsDuration(d) => {
            state.notifications_duration = d;
            write_config_value("duration", &state.notifications_duration.to_string());
        }
        SystemMessage::SendTestNotification => {
            tokio::spawn(async move {
                if let Ok(connection) = zbus::Connection::session().await {
                    let _ = connection.call_method(
                        Some("org.freedesktop.Notifications"),
                        "/org/freedesktop/Notifications",
                        Some("org.freedesktop.Notifications"),
                        "Notify",
                        &(
                            "cce-client",
                            0u32,
                            "",
                            "System notifications are working correctly!",
                            "",
                            Vec::<&str>::new(),
                            std::collections::HashMap::<&str, zbus::zvariant::Value>::new(),
                            -1i32,
                        )
                    ).await;
                }
            });
        }
        SystemMessage::NotificationsRefreshed(new) => {
            state.notifications_loaded = true;
            state.notifications_enable = new.enable;
            state.notifications_bell = new.bell;
            state.notifications_duration = new.duration;
        }

    }
}

// ── Notifications Configuration Reader & Writer ──

fn get_socket_path() -> String {
    match std::env::var("WAYLAND_DISPLAY") {
        Ok(display) => format!("/tmp/cce-{}.sock", display),
        Err(_) => "/tmp/cce.sock".to_string(),
    }
}

pub fn read_notifications_config() -> NotificationsConfig {
    let content = fs::read_to_string(CONFIG_PATH).unwrap_or_default();
    let enable = parse_notifications_enable(&content);
    let bell = parse_notifications_bell(&content);
    let duration = parse_notifications_duration(&content);
    NotificationsConfig {
        enable,
        bell,
        duration,
    }
}

fn parse_json(content: &str) -> serde_json::Value {
    serde_json::from_str(content).unwrap_or_default()
}

fn parse_notifications_enable(content: &str) -> bool {
    let val = parse_json(content);
    val["notifications"]["enable"].as_bool().unwrap_or(true)
}

fn parse_notifications_bell(content: &str) -> bool {
    let val = parse_json(content);
    val["notifications"]["bell"].as_bool().unwrap_or(false)
}

fn parse_notifications_duration(content: &str) -> i32 {
    let val = parse_json(content);
    val["notifications"]["duration"].as_i64().map(|v| v as i32).unwrap_or(5)
}

fn send_ipc_command(cmd: &str) {
    if let Ok(mut stream) = std::os::unix::net::UnixStream::connect(get_socket_path()) {
        let _ = stream.write_all(format!("{}\n", cmd).as_bytes());
    }
}

fn write_config_value(key: &str, value: &str) {
    cce_ui::config::write_config_value(&get_config_path(), key, value, "notifications");
}

fn write_enable_notifications(enabled: bool) {
    write_config_value("enable", &enabled.to_string());
    send_ipc_command("reload");
}

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
            "/home/lsgalante/.config/cce/config.json".to_string()
        })
    }
    #[cfg(not(test))]
    {
        "/home/lsgalante/.config/cce/config.json".to_string()
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_layout_grid() {
        let mut state = SystemState::default();
        let mut layout = cce_ui::layout::ColumnLayout::new(20.0);
        let sec_focused = vec![false, false, false, false, false, false, false, false];
        let mut ctx = cce_ui::context::UiContext::new();
        let pc = view(&mut state, 10.0, 20.0, 800.0, 600.0, false, &sec_focused, &mut layout, &mut ctx);
        assert!(!pc.rects.is_empty() || !pc.texts.is_empty());
    }

    #[test]
    fn test_parse_notifications_enable_default() {
        assert!(parse_notifications_enable(""));
        assert!(parse_notifications_enable("[layout]\ngap = 18\n"));
    }

    #[test]
    fn test_parse_notifications_enable_explicit() {
        let content = "{\"notifications\": {\"enable\": false}}";
        assert!(!parse_notifications_enable(content));

        let content = "{\"notifications\": {\"enable\": true}}";
        assert!(parse_notifications_enable(content));
    }

    #[test]
    fn test_parse_notifications_enable_other_sections() {
        let content = r#"{
            "layout": {"enable": false},
            "notifications": {"enable": true},
            "input": {"enable": false}
        }"#;
        assert!(parse_notifications_enable(content));

        let content = r#"{
            "layout": {"enable": true},
            "notifications": {"enable": false},
            "input": {"enable": true}
        }"#;
        assert!(!parse_notifications_enable(content));
    }

    #[test]
    fn test_parse_notifications_duration_default() {
        assert_eq!(parse_notifications_duration(""), 5);
        assert_eq!(parse_notifications_duration("{\"notifications\": {}}"), 5);
    }

    #[test]
    fn test_parse_notifications_duration_explicit() {
        let content = "{\"notifications\": {\"duration\": 10}}";
        assert_eq!(parse_notifications_duration(content), 10);
    }


}
