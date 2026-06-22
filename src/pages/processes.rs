use crate::app::{AppAction, PageContent, SectionContextExt};
use cce_ui::layout::{render_widget, PageLayoutBuilder, LayoutStrategy, RenderTarget};
use cce_ui::widget::{Label, ScrollingList, Dropdown, InfoBox, TextBox, StatusDot, DotStatus, InteractiveListItem, Toggle, Spinbox, Element};
use crate::pages::interface::parse_u16_from;
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
}

#[derive(Debug, Clone)]
pub struct ServiceInfo {
    pub name: String,
    pub description: String,
    pub active_state: String,
    pub sub_state: String,
    pub is_system: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceTab {
    System,
    User,
}

impl Default for ServiceTab {
    fn default() -> Self {
        ServiceTab::System
    }
}

#[derive(Debug, Clone)]
pub struct ProcessesState {
    pub cpu_model: String,
    pub cpu_usage: f32,
    pub cpu_cores: u32,
    pub gpus: Vec<String>,
    pub loaded: bool,
    pub cpu_label: Label,
    pub cpu_usage_label: Label,
    pub cpu_temp_label: Label,
    pub gpu_labels: Vec<Label>,
    pub processes: Vec<(String, String, String)>, // (pid, cpu, comm)
    pub cpu_list_box: ScrollingList,

    // Power-related fields
    pub battery: BatteryInfo,
    pub on_ac: bool,
    pub cpu_powersave: bool,
    pub gpu_powersave: bool,
    pub cpu_gov_menu: Dropdown,
    pub gpu_gov_menu: Dropdown,

    // Services-related fields
    pub services_loaded: bool,
    pub services: Vec<ServiceInfo>,
    pub services_active_tab: ServiceTab,
    pub services_search_box: TextBox,
    pub services_list_box: ScrollingList,
    pub service_items: Vec<InteractiveListItem>,
    pub notifications_loaded: bool,
    pub notifications_enable: bool,
    pub notifications_enable_toggle: Toggle,
    pub notifications_bell: bool,
    pub notifications_bell_toggle: Toggle,
    pub notifications_duration: i32,
    pub notifications_duration_spinbox: Spinbox,

    // Status Interface fields
    pub status_loaded: bool,
    pub status_font_size: u16,
    pub status_padding: u16,
    pub status_separators: bool,
    pub status_underline: bool,
    pub status_running: bool,
    pub status_label: Label,
    pub status_separators_toggle: Toggle,
    pub status_underline_toggle: Toggle,
    pub status_padding_spinbox: Spinbox,
}

impl Default for ProcessesState {
    fn default() -> Self {
        Self {
            cpu_model: String::new(),
            cpu_usage: 0.0,
            cpu_cores: 0,
            gpus: Vec::new(),
            loaded: false,
            cpu_label: Label::new("CPU Info"),
            cpu_usage_label: Label::new("CPU Usage"),
            cpu_temp_label: Label::new("CPU Temp"),
            gpu_labels: Vec::new(),
            processes: Vec::new(),
            cpu_list_box: ScrollingList::new(24.0, 2.0),

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

            services_loaded: false,
            services: Vec::new(),
            services_active_tab: ServiceTab::System,
            services_search_box: TextBox::new(String::new()).with_label("Filter Services"),
            services_list_box: ScrollingList::new(36.0, 6.0),
            service_items: Vec::new(),
            notifications_loaded: false,
            notifications_enable: true,
            notifications_enable_toggle: Toggle::new().with_label("Enable Notifications"),
            notifications_bell: false,
            notifications_bell_toggle: Toggle::new().with_label("Play Bell Sound"),
            notifications_duration: 5,
            notifications_duration_spinbox: Spinbox::new(5, 1, 60, 1)
                .with_label("Notification Duration")
                .with_unit("s"),

            status_loaded: false,
            status_font_size: 11,
            status_padding: 8,
            status_separators: true,
            status_underline: true,
            status_running: false,
            status_label: Label::new("Status Interface: Stopped").with_font_size(14.0).with_color([170, 51, 51]),
            status_separators_toggle: Toggle::new().with_label("Show Separators"),
            status_underline_toggle: Toggle::new().with_label("Show Underline"),
            status_padding_spinbox: Spinbox::new(8, 0, 32, 1).with_label("Side Padding").with_unit("px"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ProcessesMessage {
    Refreshed(ProcessesState),
    SetCpuPerformance,
    SetCpuPowersave,
    SetGpuDefault,
    SetGpuPowersave,
    None,

    // Services-related variants
    ServicesRefreshed(Vec<ServiceInfo>),
    ServicesSetTab(ServiceTab),
    ServicesStart(String, bool),
    ServicesStop(String, bool),
    ServicesRestart(String, bool),
    ToggleNotificationsEnable,
    ToggleNotificationsBell,
    SetNotificationsDuration(i32),
    SendTestNotification,
    NotificationsRefreshed(NotificationsConfig),

    // Status Interface variants
    StatusRefreshed(StatusData),
    StatusToggleSeparators,
    StatusToggleUnderline,
    StatusReload,
    StatusSetPadding(u16),
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
    let _ = tokio::process::Command::new("pkexec")
        .arg(format!("/home/lsgalante/.local/share/cce-system-interface/helpers/{}", script))
        .spawn();
}

fn spawn_gpu_power(powersave: bool) {
    let script = if powersave { "gpu-powersave-on" } else { "gpu-powersave-off" };
    let _ = tokio::process::Command::new("pkexec")
        .arg(format!("/home/lsgalante/.local/share/cce-system-interface/helpers/{}", script))
        .spawn();
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

pub async fn fetch_processes_state() -> ProcessesState {
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

    ProcessesState {
        cpu_model,
        cpu_usage,
        cpu_cores,
        gpus,
        loaded: true,
        cpu_label: Label::new(&cpu_label_text).with_font_size(12.0).with_color([212, 212, 212]),
        cpu_usage_label: Label::new(&cpu_usage_text).with_font_size(12.0).with_color([212, 212, 212]),
        cpu_temp_label: Label::new(&cpu_temp_text).with_font_size(12.0).with_color([212, 212, 212]),
        gpu_labels,
        processes,
        cpu_list_box: ScrollingList::new(24.0, 2.0),
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

        services_loaded: false,
        services: Vec::new(),
        services_active_tab: ServiceTab::System,
        services_search_box: TextBox::new(String::new()).with_label("Filter Services"),
        services_list_box: ScrollingList::new(36.0, 6.0),
        service_items: Vec::new(),
        notifications_loaded: false,
        notifications_enable: true,
        notifications_enable_toggle: Toggle::new().with_label("Enable Notifications"),
        notifications_bell: false,
        notifications_bell_toggle: Toggle::new().with_label("Play Bell Sound"),
        notifications_duration: 5,
        notifications_duration_spinbox: Spinbox::new(5, 1, 60, 1)
            .with_label("Notification Duration")
            .with_unit("s"),

        status_loaded: false,
        status_font_size: 11,
        status_padding: 8,
        status_separators: true,
        status_underline: true,
        status_running: false,
        status_label: Label::new("Status Interface: Stopped").with_font_size(14.0).with_color([170, 51, 51]),
        status_separators_toggle: Toggle::new().with_label("Show Separators"),
        status_underline_toggle: Toggle::new().with_label("Show Underline"),
        status_padding_spinbox: Spinbox::new(8, 0, 32, 1).with_label("Side Padding").with_unit("px"),
    }
}

const TEXT_FG: [f32; 4] = [0.83, 0.83, 0.83, 1.0];
const TEXT_DIM: [f32; 4] = [0.53, 0.53, 0.60, 1.0];
const ACCENT: [f32; 4] = [0.36, 0.56, 0.38, 1.0];
const RED: [f32; 4] = [1.0, 0.33, 0.33, 1.0];
const ORANGE: [f32; 4] = [1.0, 0.73, 0.20, 1.0];

pub fn view(state: &mut ProcessesState, cx: f32, cy: f32, cw: f32, ch: f32, root_focused: bool, sec_focused: &[bool], layout: &mut dyn LayoutStrategy, ctx: &mut cce_ui::context::UiContext) -> PageContent {
    let mut final_pc = PageContent::new();
    let sec_w = 320.0f32;
    let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(8);

    // ── CPU Section ──
    builder.add_section(&mut final_pc, "CPU", root_focused, |sec| {
        let rx = sec.left;
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

            // Scrolling box configuration for process list
            let list_box_x = rx + 12.0;
            let list_box_y = sec.ay();
            let list_box_w = sec.cw - 24.0;
            let list_box_h = 220.0;
            
            // Render the standardized ScrollBox widget
            render_widget(sec.pc, &mut state.cpu_list_box, list_box_x, list_box_y, list_box_w, list_box_h, ctx);

            // Header for process list columns (drawn static on top of the ScrollBox background)
            let header_h = 22.0;
            sec.pc.rect([0.12, 0.12, 0.16, 0.5], list_box_x + 1.0, list_box_y + 1.0, list_box_w - 2.0, header_h);
            sec.pc.rect([0.18, 0.18, 0.24, 1.0], list_box_x + 1.0, list_box_y + header_h, list_box_w - 2.0, 1.0); // Divider
            
            sec.pc.text("PID", list_box_x + 12.0, list_box_y + 5.0, 11.0, [0.53, 0.53, 0.60, 1.0]);
            sec.pc.text("COMMAND", list_box_x + 80.0, list_box_y + 5.0, 11.0, [0.53, 0.53, 0.60, 1.0]);
            sec.pc.text("CPU %", list_box_x + list_box_w - 60.0, list_box_y + 5.0, 11.0, [0.53, 0.53, 0.60, 1.0]);

            let row_h = 24.0;
            // Update ScrollingList bounds for the scrollable viewport (which starts below the header)
            state.cpu_list_box.update_bounds(state.processes.len(), list_box_y + header_h, list_box_h - header_h - 6.0);

            // Visible process rows rendering (virtualized/clipped)
            sec.pc.push_clip_rect(list_box_x, list_box_y + header_h, list_box_w, list_box_h - header_h);
            for (idx, (pid, cpu, comm)) in state.processes.iter().enumerate() {
                if let Some(draw_y) = state.cpu_list_box.get_item_draw_y(idx, 4.0) {
                    // Standard row action button (transparent background, highlights on hover)
                    sec.pc.button(
                        "",
                        list_box_x + 2.0,
                        draw_y,
                        list_box_w - 16.0,
                        row_h,
                        [0.0, 0.0, 0.0, 0.0],
                        [1.0, 1.0, 1.0, 0.06],
                        [0.0, 0.0, 0.0, 0.0],
                        AppAction::Processes(ProcessesMessage::None),
                    );
                    
                    sec.pc.text(pid, list_box_x + 12.0, draw_y + 6.0, 12.0, [0.80, 0.80, 0.85, 1.0]);
                    sec.pc.text(comm, list_box_x + 80.0, draw_y + 6.0, 12.0, [0.80, 0.80, 0.85, 1.0]);
                    sec.pc.text(&format!("{}%", cpu), list_box_x + list_box_w - 60.0, draw_y + 6.0, 12.0, [0.56, 0.83, 0.56, 1.0]);
                }
            }
            sec.pc.pop_clip_rect();
            
            if state.processes.is_empty() {
                sec.pc.text("No active processes", list_box_x + 12.0, list_box_y + header_h + 16.0, 12.0, TEXT_DIM);
            }

            sec.content_y += list_box_h;
        }
    });

    // ── GPU Section ──
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

    // ── Battery Section ──
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

    // ── CPU Governor section ──
    builder.add_section(&mut final_pc, "CPU Governor", false, |sec_gov| {
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

    // ── GPU Power section ──
    builder.add_section(&mut final_pc, "GPU Power", false, |sec_gpow| {
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

    // ── Services Section ──
    builder.add_section_spanned(&mut final_pc, "Services", 2, sec_focused.get(5).copied().unwrap_or(false), |sec| {
        let sec_w = sec.cw;
        if !state.services_loaded {
            sec.text("Loading systemd services...", 12.0, 0.0, 12.0, TEXT_DIM);
            sec.spacing(18.0);
        } else {
            // Tab header buttons: System Services, User Services
            let tab_w = (sec_w - 24.0 - 8.0) / 2.0;
            let tab_h = 28.0;
            let tab_y = sec.ay();
            let active_bg = [0.20, 0.40, 0.65, 0.4];
            let inactive_bg = [0.10, 0.10, 0.16, 0.3];
            let hover_bg = [0.20, 0.20, 0.25, 0.15];

            let label1 = if tab_w < 110.0 { "System" } else { "System Services" };
            let label2 = if tab_w < 110.0 { "User" } else { "User Services" };

            let tab_x1 = sec.left + 12.0;
            let tab_x2 = sec.left + 12.0 + tab_w + 8.0;

            sec.pc.button(
                label1,
                tab_x1,
                tab_y,
                tab_w,
                tab_h,
                if state.services_active_tab == ServiceTab::System { active_bg } else { inactive_bg },
                hover_bg,
                [0.90, 0.90, 0.95, 1.0],
                crate::app::AppAction::Processes(ProcessesMessage::ServicesSetTab(ServiceTab::System)),
            );

            sec.pc.button(
                label2,
                tab_x2,
                tab_y,
                tab_w,
                tab_h,
                if state.services_active_tab == ServiceTab::User { active_bg } else { inactive_bg },
                hover_bg,
                [0.90, 0.90, 0.95, 1.0],
                crate::app::AppAction::Processes(ProcessesMessage::ServicesSetTab(ServiceTab::User)),
            );
            sec.content_y += tab_h + 12.0;

            // Search textbox
            let search_y = sec.ay();
            let search_w = sec_w - 24.0;
            let search_h = 46.0;
            
            state.services_search_box.set_row_rect(sec.left + 12.0, search_w);
            cce_ui::layout::render_widget(
                sec.pc,
                &mut state.services_search_box,
                sec.left + 12.0,
                search_y,
                search_w,
                search_h,
                ctx,
            );
            sec.content_y += search_h + 16.0;

            // Scroll box list
            let list_box_x = sec.left + 12.0;
            let list_box_y = sec.ay();
            let list_box_w = sec_w - 24.0;
            let list_box_h = 360.0;
            
            cce_ui::layout::render_widget(sec.pc, &mut state.services_list_box, list_box_x, list_box_y, list_box_w, list_box_h, ctx);

            // Filter services
            let query = if state.services_search_box.editing {
                state.services_search_box.edit_buffer.to_lowercase()
            } else {
                state.services_search_box.text.to_lowercase()
            };
            let filtered_services: Vec<&ServiceInfo> = state.services.iter()
                .filter(|s| s.is_system == (state.services_active_tab == ServiceTab::System))
                .filter(|s| s.name.to_lowercase().contains(&query) || s.description.to_lowercase().contains(&query))
                .collect();

            // Update ScrollingList bounds
            state.services_list_box.update_bounds(filtered_services.len(), list_box_y, list_box_h);

            let item_h = state.services_list_box.item_height;

            if state.service_items.len() != filtered_services.len() {
                state.service_items.clear();
                for _ in 0..filtered_services.len() {
                    state.service_items.push(InteractiveListItem::new(""));
                }
            }

            sec.pc.push_clip_rect(list_box_x, list_box_y, list_box_w, list_box_h);
            for (idx, service) in filtered_services.iter().enumerate() {
                if let Some(draw_y) = state.services_list_box.get_item_draw_y(idx, 4.0) {
                    let is_active = service.active_state == "active" || service.sub_state == "running";

                    // Control buttons: Start, Stop, Restart on the right
                    let is_small = sec_w < 350.0;
                    let btn_w = if is_small { 24.0 } else { 46.0 };
                    let r_btn_w = if is_small { 24.0 } else { 54.0 };
                    let btn_gap = if is_small { 4.0 } else { 6.0 };
                    let right_edge = list_box_x + list_box_w - 24.0 - 8.0;

                    let restart_x = right_edge - r_btn_w;
                    let stop_x = restart_x - btn_gap - btn_w;
                    let start_x = stop_x - btn_gap - btn_w;

                    let btn_y = draw_y + (item_h - 22.0) / 2.0;
                    let btn_h = 22.0;

                    // Service Description (Truncate dynamically based on remaining space before Start button)
                    let text_max_w = (start_x - 8.0) - (list_box_x + 32.0);
                    let max_chars = ((text_max_w / 6.0) as usize).max(10);
                    let desc = if service.description.is_empty() { "No description" } else { &service.description };
                    let desc_truncated = if desc.len() > max_chars {
                        format!("{}...", &desc[..max_chars.saturating_sub(3)])
                    } else {
                        desc.to_string()
                    };

                    // Render InteractiveListItem background and text labels
                    let item_btn = &mut state.service_items[idx];
                    item_btn.title = service.name.clone();
                    item_btn.subtitle = Some(desc_truncated);
                    cce_ui::layout::render_widget(sec.pc, item_btn, list_box_x + 24.0, draw_y, list_box_w - 44.0, item_h, ctx);

                    // Render StatusDot
                    let status_dot_state = if service.active_state == "failed" {
                        DotStatus::Error
                    } else if is_active {
                        DotStatus::Active
                    } else {
                        DotStatus::Inactive
                    };
                    let mut dot = StatusDot::new(status_dot_state);
                    cce_ui::layout::render_widget(sec.pc, &mut dot, list_box_x + 10.0, draw_y + (item_h - 10.0) / 2.0, 10.0, 10.0, ctx);

                    let active_txt = [0.90, 0.90, 0.95, 1.0];
                    let disabled_txt = [0.40, 0.40, 0.45, 1.0];

                    let start_lbl = if is_small { "▶" } else { "Start" };
                    let stop_lbl = if is_small { "■" } else { "Stop" };
                    let restart_lbl = if is_small { "⟳" } else { "Restart" };

                    // Start button
                    sec.pc.button(
                        start_lbl,
                        start_x,
                        btn_y,
                        btn_w,
                        btn_h,
                        if !is_active { [0.16, 0.35, 0.18, 0.4] } else { [0.12, 0.12, 0.16, 0.1] },
                        [0.22, 0.45, 0.25, 0.6],
                        if !is_active { active_txt } else { disabled_txt },
                        crate::app::AppAction::Processes(ProcessesMessage::ServicesStart(service.name.clone(), service.is_system)),
                    );

                    // Stop button
                    sec.pc.button(
                        stop_lbl,
                        stop_x,
                        btn_y,
                        btn_w,
                        btn_h,
                        if is_active { [0.55, 0.16, 0.16, 0.3] } else { [0.12, 0.12, 0.16, 0.1] },
                        [0.70, 0.22, 0.22, 0.5],
                        if is_active { active_txt } else { disabled_txt },
                        crate::app::AppAction::Processes(ProcessesMessage::ServicesStop(service.name.clone(), service.is_system)),
                    );

                    // Restart button
                    sec.pc.button(
                        restart_lbl,
                        restart_x,
                        btn_y,
                        r_btn_w,
                        btn_h,
                        [0.15, 0.28, 0.45, 0.3],
                        [0.20, 0.38, 0.58, 0.5],
                        active_txt,
                        crate::app::AppAction::Processes(ProcessesMessage::ServicesRestart(service.name.clone(), service.is_system)),
                    );
                }
            }
            sec.pc.pop_clip_rect();

            if filtered_services.is_empty() {
                sec.pc.text("No services match the query", list_box_x + 16.0, list_box_y + 16.0, 12.0, TEXT_DIM);
            }

            sec.content_y += list_box_h;
        }
    });

    // ── System Notifications ──
    builder.add_section(&mut final_pc, "System Notifications", sec_focused.get(6).copied().unwrap_or(false), |sec2| {
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
                AppAction::Processes(ProcessesMessage::SendTestNotification),
            );
        }
        sec2.spacing(12.0);
    });

    // ── Status Interface ──
    builder.add_section(&mut final_pc, "Status Interface", sec_focused.get(7).copied().unwrap_or(false), |sec3| {
        let sec_w = sec3.cw;
        if !state.status_loaded {
            sec3.text("Loading Status Interface status...", 12.0, 0.0, 12.0, TEXT_DIM);
            sec3.spacing(18.0);
        } else {
            // Status
            let status_text = if state.status_running { "Status Interface: Running" } else { "Status Interface: Stopped" };
            let status_color = if state.status_running { [92, 143, 97] } else { [170, 51, 51] };
            state.status_label.set_text(status_text);
            state.status_label.set_color(status_color);
            sec3.widget(&mut state.status_label, 12.0, sec_w - 24.0, 20.0, ctx);
            sec3.spacing(12.0);

            sec3.spacing(4.0);

            // Separators toggle
            state.status_separators_toggle.set_toggled(state.status_separators);
            sec3.widget_full(&mut state.status_separators_toggle, cce_ui::layout::toggle_height(), ctx);
            sec3.spacing(16.0);

            // Underline toggle
            state.status_underline_toggle.set_toggled(state.status_underline);
            sec3.widget_full(&mut state.status_underline_toggle, cce_ui::layout::toggle_height(), ctx);
            sec3.spacing(16.0);

            // Padding spinbox
            state.status_padding_spinbox.value = state.status_padding as i32;
            sec3.widget(&mut state.status_padding_spinbox, 12.0, sec_w - 24.0, 44.0, ctx);
            sec3.spacing(16.0);

            // Reload button
            let yt_reload = sec3.ay();
            let btn_w = sec_w - 24.0;
            let button_x = sec3.left + 12.0;
            sec3.button(
                "Reload Status Interface",
                button_x,
                yt_reload,
                btn_w,
                32.0,
                [0.13, 0.18, 0.14, 1.0],
                [0.25, 0.30, 0.26, 1.0],
                [1.0, 1.0, 1.0, 1.0],
                AppAction::Processes(ProcessesMessage::StatusReload),
            );
            sec3.spacing(12.0);
        }
    });

    final_pc
}

pub fn update(state: &mut ProcessesState, msg: ProcessesMessage) {
    match msg {
        ProcessesMessage::Refreshed(new) => {
            state.loaded = new.loaded;
            state.cpu_model = new.cpu_model;
            state.cpu_usage = new.cpu_usage;
            state.cpu_cores = new.cpu_cores;
            state.gpus = new.gpus;
            state.cpu_label = new.cpu_label;
            state.cpu_usage_label = new.cpu_usage_label;
            state.cpu_temp_label = new.cpu_temp_label;
            state.gpu_labels = new.gpu_labels;
            state.processes = new.processes;
            let old_scroll = state.cpu_list_box.scroll_y();
            state.cpu_list_box = new.cpu_list_box;
            state.cpu_list_box.set_scroll_y(old_scroll);

            state.battery = new.battery;
            state.on_ac = new.on_ac;
            state.cpu_powersave = new.cpu_powersave;
            state.gpu_powersave = new.gpu_powersave;
            state.cpu_gov_menu.selected = new.cpu_gov_menu.selected;
            state.gpu_gov_menu.selected = new.gpu_gov_menu.selected;
        }
        ProcessesMessage::SetCpuPerformance => {
            state.cpu_powersave = false;
            state.cpu_gov_menu.selected = 0;
            spawn_cpu_power(false);
        }
        ProcessesMessage::SetCpuPowersave => {
            state.cpu_powersave = true;
            state.cpu_gov_menu.selected = 1;
            spawn_cpu_power(true);
        }
        ProcessesMessage::SetGpuDefault => {
            state.gpu_powersave = false;
            state.gpu_gov_menu.selected = 0;
            spawn_gpu_power(false);
        }
        ProcessesMessage::SetGpuPowersave => {
            state.gpu_powersave = true;
            state.gpu_gov_menu.selected = 1;
            spawn_gpu_power(true);
        }
        ProcessesMessage::ServicesRefreshed(new_services) => {
            state.services_loaded = true;
            state.services = new_services;
            state.service_items.clear();
        }
        ProcessesMessage::ServicesSetTab(tab) => {
            state.services_active_tab = tab;
            state.services_list_box.set_scroll_y(0.0);
            state.service_items.clear();
        }
        ProcessesMessage::ServicesStart(name, is_system) => {
            if let Some(srv) = state.services.iter_mut().find(|s| s.name == name && s.is_system == is_system) {
                srv.active_state = "activating".to_string();
                srv.sub_state = "starting".to_string();
            }
            service_action(&name, "start", is_system);
        }
        ProcessesMessage::ServicesStop(name, is_system) => {
            if let Some(srv) = state.services.iter_mut().find(|s| s.name == name && s.is_system == is_system) {
                srv.active_state = "deactivating".to_string();
                srv.sub_state = "stopping".to_string();
            }
            service_action(&name, "stop", is_system);
        }
        ProcessesMessage::ServicesRestart(name, is_system) => {
            if let Some(srv) = state.services.iter_mut().find(|s| s.name == name && s.is_system == is_system) {
                srv.active_state = "activating".to_string();
                srv.sub_state = "restarting".to_string();
            }
            service_action(&name, "restart", is_system);
        }
        ProcessesMessage::ToggleNotificationsEnable => {
            state.notifications_enable = !state.notifications_enable;
            write_enable_notifications(state.notifications_enable);
        }
        ProcessesMessage::ToggleNotificationsBell => {
            state.notifications_bell = !state.notifications_bell;
            write_config_value("bell", &state.notifications_bell.to_string());
        }
        ProcessesMessage::SetNotificationsDuration(d) => {
            state.notifications_duration = d;
            write_config_value("duration", &state.notifications_duration.to_string());
        }
        ProcessesMessage::SendTestNotification => {
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
        ProcessesMessage::NotificationsRefreshed(new) => {
            state.notifications_loaded = true;
            state.notifications_enable = new.enable;
            state.notifications_bell = new.bell;
            state.notifications_duration = new.duration;
        }
        ProcessesMessage::StatusRefreshed(new) => {
            let was_status_hovered = state.status_label.hovered();
            let was_separators_hovered = state.status_separators_toggle.hovered();
            let was_underline_hovered = state.status_underline_toggle.hovered();

            state.status_loaded = true;
            state.status_font_size = new.font_size;
            state.status_padding = new.padding;
            state.status_separators = new.separators;
            state.status_underline = new.underline;
            state.status_running = new.running;

            state.status_label.set_hovered(was_status_hovered);
            state.status_separators_toggle.set_hovered(was_separators_hovered);
            state.status_underline_toggle.set_hovered(was_underline_hovered);
        }
        ProcessesMessage::StatusToggleSeparators => {
            state.status_separators = !state.status_separators;
            write_status_separators(state.status_separators);
            status_interface_reload();
        }
        ProcessesMessage::StatusToggleUnderline => {
            state.status_underline = !state.status_underline;
            write_status_underline(state.status_underline);
            status_interface_reload();
        }
        ProcessesMessage::StatusSetPadding(val) => {
            state.status_padding = val;
            write_status_padding(val);
            status_interface_reload();
        }
        ProcessesMessage::StatusReload => {
            status_interface_reload();
        }
        ProcessesMessage::None => {}
    }
}

// ── Background Fetching ──

pub async fn fetch_services() -> Vec<ServiceInfo> {
    let mut services = Vec::new();

    // 1. Fetch system-level services
    if let Ok(output) = tokio::process::Command::new("systemctl")
        .args(["list-units", "--type=service", "--all", "--no-legend"])
        .output()
        .await
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if let Some(info) = parse_service_line(line, true) {
                services.push(info);
            }
        }
    }

    // 2. Fetch user-level services
    if let Ok(output) = tokio::process::Command::new("systemctl")
        .args(["--user", "list-units", "--type=service", "--all", "--no-legend"])
        .output()
        .await
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if let Some(info) = parse_service_line(line, false) {
                services.push(info);
            }
        }
    }

    // Sort alphabetically by name
    services.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    services
}

fn parse_service_line(line: &str, is_system: bool) -> Option<ServiceInfo> {
    let cleaned = line.trim_start_matches('●').trim();
    if cleaned.is_empty() {
        return None;
    }
    let parts: Vec<&str> = cleaned.split_whitespace().collect();
    if parts.len() >= 4 && parts[0].ends_with(".service") {
        let name = parts[0].to_string();
        let _load = parts[1];
        let active_state = parts[2].to_string();
        let sub_state = parts[3].to_string();
        let description = parts[4..].join(" ");
        Some(ServiceInfo {
            name,
            description,
            active_state,
            sub_state,
            is_system,
        })
    } else {
        None
    }
}

fn service_action(name: &str, action: &str, is_system: bool) {
    if is_system {
        // System service needs root privilege, spawn via pkexec
        let _ = tokio::process::Command::new("pkexec")
            .args(["systemctl", action, name])
            .spawn();
    } else {
        let _ = tokio::process::Command::new("systemctl")
            .args(["--user", action, name])
            .spawn();
    }
}

// ── Notifications Configuration Reader & Writer ──

const CONFIG_PATH: &str = "/home/lsgalante/.config/cce/config.json";

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
    let content = fs::read_to_string(CONFIG_PATH).unwrap_or_default();
    let mut val = parse_json(&content);
    let j_val = if let Ok(b) = value.parse::<bool>() {
        serde_json::json!(b)
    } else if let Ok(n) = value.parse::<i64>() {
        serde_json::json!(n)
    } else if let Ok(f) = value.parse::<f64>() {
        serde_json::json!(f)
    } else {
        serde_json::json!(value)
    };
    if let Some(notifications) = val.get_mut("notifications").and_then(|n| n.as_object_mut()) {
        notifications.insert(key.to_string(), j_val);
    } else {
        let mut map = serde_json::Map::new();
        map.insert(key.to_string(), j_val);
        if let Some(obj) = val.as_object_mut() {
            obj.insert("notifications".to_string(), serde_json::Value::Object(map));
        }
    }
    if let Ok(updated_str) = serde_json::to_string_pretty(&val) {
        let _ = fs::write(CONFIG_PATH, updated_str);
    }
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

fn write_status_value(key: &str, value: &str) {
    crate::pages::interface::write_config_value_path(&get_config_path(), key, value);
}

fn read_status_font_size() -> Option<u16> {
    let content = std::fs::read_to_string(&get_config_path()).ok()?;
    Some(parse_u16_from(&content, "status_font_size", 11))
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
    Some(crate::pages::interface::parse_bool_from(&content, "status_separators", true))
}

fn write_status_separators(val: bool) {
    write_status_value("status_separators", &val.to_string());
}

fn read_status_underline() -> Option<bool> {
    let content = std::fs::read_to_string(&get_config_path()).ok()?;
    Some(crate::pages::interface::parse_bool_from(&content, "status_underline", true))
}

fn write_status_underline(val: bool) {
    write_status_value("status_underline", &val.to_string());
}

fn status_interface_reload() {
    let _ = std::process::Command::new("pkill")
        .args(["-f", "cce-status-interface"])
        .status();
    std::thread::sleep(std::time::Duration::from_millis(150));
    send_ipc_command("spawn cce-status-interface");
}

pub async fn fetch_status_state() -> StatusData {
    let running = tokio::process::Command::new("pgrep")
        .args(["-f", "cce-status-interface"]).output().await.ok()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false);

    let font_size = read_status_font_size().unwrap_or(11);
    let padding = read_status_padding().unwrap_or(8);
    let separators = read_status_separators().unwrap_or(true);
    let underline = read_status_underline().unwrap_or(true);

    StatusData {
        font_size,
        padding,
        separators,
        underline,
        running,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_layout_grid() {
        let mut state = ProcessesState::default();
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

    #[test]
    fn test_read_write_separators() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_status_separators.json");
        let path_str = path.to_str().unwrap().to_string();

        let _ = fs::write(&path_str, "{\"layout\": {\"status_separators\": true, \"status_padding\": 8}}");
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
        let path = dir.join("test_status_padding.json");
        let path_str = path.to_str().unwrap().to_string();

        let _ = fs::write(&path_str, "{\"layout\": {\"status_separators\": true, \"status_padding\": 8}}");
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
        let path = dir.join("test_status_underline.json");
        let path_str = path.to_str().unwrap().to_string();

        let _ = fs::write(&path_str, "{\"layout\": {\"status_underline\": true, \"status_padding\": 8}}");
        TEST_CONFIG_PATH.with(|p| *p.borrow_mut() = Some(path_str));

        let original = read_status_underline().unwrap_or(true);
        write_status_underline(!original);
        assert_eq!(read_status_underline(), Some(!original));
        write_status_underline(original);
        assert_eq!(read_status_underline(), Some(original));

        let _ = fs::remove_file(path);
    }
}
