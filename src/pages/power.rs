use crate::app::{AppAction, PageContent};
use clear_ui::layout::Section;

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

#[derive(Debug, Clone, Default)]
pub struct PowerState {
    pub loaded: bool,
    pub battery: BatteryInfo,
    pub on_ac: bool,
    pub cpu_powersave: bool,
    pub gpu_powersave: bool,
}

#[derive(Debug, Clone)]
pub enum PowerMessage {
    Refreshed(PowerState),
    SetCpuPerformance,
    SetCpuPowersave,
    SetGpuDefault,
    SetGpuPowersave,
    Suspend,
    Hibernate,
    Reboot,
    PowerOff,
    Tick,
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

fn spawn_systemctl(action: &str) {
    let _ = tokio::process::Command::new("systemctl").arg(action).spawn();
}

fn spawn_cpu_power(powersave: bool) {
    let script = if powersave { "cpu-powersave-on" } else { "cpu-powersave-off" };
    let _ = tokio::process::Command::new("pkexec")
        .arg(format!("/home/lsgalante/.local/share/clear-system-interface/helpers/{}", script))
        .spawn();
}

fn spawn_gpu_power(powersave: bool) {
    let script = if powersave { "gpu-powersave-on" } else { "gpu-powersave-off" };
    let _ = tokio::process::Command::new("pkexec")
        .arg(format!("/home/lsgalante/.local/share/clear-system-interface/helpers/{}", script))
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

pub async fn fetch_power_state() -> PowerState {
    let (battery, on_ac) = fetch_upower().await;
    let cpu_powersave = current_cpu_governor() == "powersave";
    let gpu_powersave = current_gpu_power_cap().await;
    PowerState { loaded: true, battery, on_ac, cpu_powersave, gpu_powersave }
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

const TEXT_FG: [f32; 4] = [0.83, 0.83, 0.83, 1.0];
const TEXT_DIM: [f32; 4] = [0.53, 0.53, 0.60, 1.0];
const ACCENT: [f32; 4] = [0.36, 0.56, 0.38, 1.0];
const BTN_ACTIVE: [f32; 4] = [0.20, 0.40, 0.22, 1.0];
const BTN_INACTIVE: [f32; 4] = [0.13, 0.18, 0.14, 1.0];
const BTN_HOVER: [f32; 4] = [0.25, 0.30, 0.26, 1.0];
const DANGER_BG: [f32; 4] = [0.67, 0.20, 0.20, 1.0];
const SAFE_BG: [f32; 4] = [0.20, 0.33, 0.22, 1.0];
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const RED: [f32; 4] = [1.0, 0.33, 0.33, 1.0];
const ORANGE: [f32; 4] = [1.0, 0.73, 0.20, 1.0];

pub fn view(state: &PowerState, cx: f32, cy: f32, cw: f32, _ch: f32) -> PageContent {
    let mut pc = PageContent::new();
    let mut y = cy + 12.0;

    // ── Battery section ──
    let mut sec = Section::new(&mut pc, cx, y, cw, "Battery");

    if !state.loaded {
        sec.text(&mut pc, "Loading battery status...", 12.0, 0.0, 12.0, TEXT_DIM);
        sec.spacing(18.0);
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
        sec.text(&mut pc, &pct_str, 12.0, 0.0, 24.0, pct_color);
        sec.spacing(30.0);

        let state_str = format!("{}  •  {:.1}W  •  {:.1}/{:.1} Wh",
            bat.state, bat.energy_rate, bat.energy, bat.energy_full);
        sec.text(&mut pc, &state_str, 12.0, 0.0, 12.0, TEXT_DIM);
        sec.spacing(18.0);

        let time_str = if bat.time_to_empty > 0 {
            format!("Time remaining: {}", format_duration(bat.time_to_empty))
        } else if bat.time_to_full > 0 {
            format!("Time to full: {}", format_duration(bat.time_to_full))
        } else { String::new() };
        if !time_str.is_empty() {
            sec.text(&mut pc, &time_str, 12.0, 0.0, 12.0, TEXT_DIM);
            sec.spacing(18.0);
        }

        let detail_str = format!("{}  {}", bat.vendor, bat.model);
        sec.text(&mut pc, &detail_str, 12.0, 0.0, 11.0, TEXT_DIM);
        sec.spacing(20.0);

        let ac_str = if state.on_ac { "On AC Power" } else { "On Battery" };
        sec.text(&mut pc, ac_str, 12.0, 0.0, 14.0, TEXT_FG);
    }

    y = sec.finish(&mut pc);

    // ── CPU Governor section ──
    let mut sec = Section::new(&mut pc, cx, y, cw, "CPU Governor");

    if !state.loaded {
        sec.text(&mut pc, "Loading CPU governor...", 12.0, 0.0, 12.0, TEXT_DIM);
        sec.spacing(18.0);
    } else {
        let btn_w = (cw - 40.0) / 2.0;
        let btn_h = 44.0;
        let yt = sec.ay();

        let perf_active = !state.cpu_powersave;
        let (perf_bg, perf_desc, perf_desc_color) = if perf_active {
            (BTN_ACTIVE, "Governor set to performance", ACCENT)
        } else {
            (BTN_INACTIVE, "Switch to performance governor", TEXT_DIM)
        };

        pc.button("Performance", sec.ax(12.0), yt, btn_w, btn_h,
            perf_bg, BTN_HOVER, WHITE,
            AppAction::Power(PowerMessage::SetCpuPerformance));
        sec.text(&mut pc, perf_desc, 16.0, 26.0, 10.0, perf_desc_color);

        let (save_bg, save_desc, save_desc_color) = if state.cpu_powersave {
            (BTN_ACTIVE, "Governor set to powersave — lower power, slower burst", ACCENT)
        } else {
            (BTN_INACTIVE, "Switch to powersave governor (requires auth)", TEXT_DIM)
        };

        let save_x = 16.0 + btn_w;
        pc.button("Powersave", sec.ax(save_x), yt, btn_w, btn_h,
            save_bg, BTN_HOVER, WHITE,
            AppAction::Power(PowerMessage::SetCpuPowersave));
        sec.text(&mut pc, save_desc, save_x + 4.0, 26.0, 10.0, save_desc_color);
        sec.content_y += btn_h + 12.0;
    }
    y = sec.finish(&mut pc);

    // ── GPU Power section ──
    let mut sec = Section::new(&mut pc, cx, y, cw, "GPU Power");

    if !state.loaded {
        sec.text(&mut pc, "Loading GPU power status...", 12.0, 0.0, 12.0, TEXT_DIM);
        sec.spacing(18.0);
    } else {
        let btn_w = (cw - 40.0) / 2.0;
        let btn_h = 44.0;
        let yt = sec.ay();
        let save_x = 16.0 + btn_w;

        let gpu_def_active = !state.gpu_powersave;
        let (gpu_def_bg, gpu_def_desc, gpu_def_desc_c) = if gpu_def_active {
            (BTN_ACTIVE, "NVIDIA running at default power limit", ACCENT)
        } else {
            (BTN_INACTIVE, "Restore default power limit (requires auth)", TEXT_DIM)
        };

        pc.button("80W Default", sec.ax(12.0), yt, btn_w, btn_h,
            gpu_def_bg, BTN_HOVER, WHITE,
            AppAction::Power(PowerMessage::SetGpuDefault));
        sec.text(&mut pc, gpu_def_desc, 16.0, 26.0, 10.0, gpu_def_desc_c);

        let (gpu_cap_bg, gpu_cap_desc, gpu_cap_desc_c) = if state.gpu_powersave {
            (BTN_ACTIVE, "NVIDIA power limit capped at 5W — minimal draw", ACCENT)
        } else {
            (BTN_INACTIVE, "Cap NVIDIA to 5W power limit (requires auth)", TEXT_DIM)
        };

        pc.button("5W Cap", sec.ax(save_x), yt, btn_w, btn_h,
            gpu_cap_bg, BTN_HOVER, WHITE,
            AppAction::Power(PowerMessage::SetGpuPowersave));
        sec.text(&mut pc, gpu_cap_desc, save_x + 4.0, 26.0, 10.0, gpu_cap_desc_c);
        sec.content_y += btn_h + 12.0;
    }
    y = sec.finish(&mut pc);

    // ── System Actions section ──
    let mut sec = Section::new(&mut pc, cx, y, cw, "System Actions");

    let yt = sec.ay();
    let act_btn_w = (cw - 48.0) / 4.0;
    let act_btn_h = 32.0;

    pc.button("Suspend", sec.ax(12.0), yt, act_btn_w, act_btn_h,
        SAFE_BG, BTN_HOVER, WHITE, AppAction::Power(PowerMessage::Suspend));
    pc.button("Hibernate", sec.ax(16.0 + act_btn_w), yt, act_btn_w, act_btn_h,
        SAFE_BG, BTN_HOVER, WHITE, AppAction::Power(PowerMessage::Hibernate));
    pc.button("Reboot", sec.ax(20.0 + 2.0 * act_btn_w), yt, act_btn_w, act_btn_h,
        DANGER_BG, BTN_HOVER, WHITE, AppAction::Power(PowerMessage::Reboot));
    pc.button("Power Off", sec.ax(24.0 + 3.0 * act_btn_w), yt, act_btn_w, act_btn_h,
        DANGER_BG, BTN_HOVER, WHITE, AppAction::Power(PowerMessage::PowerOff));
    sec.finish(&mut pc);

    pc
}

pub fn update(state: &mut PowerState, msg: PowerMessage) {
    match msg {
        PowerMessage::Refreshed(new) => { *state = new; }
        PowerMessage::SetCpuPerformance => {
            state.cpu_powersave = false;
            spawn_cpu_power(false);
        }
        PowerMessage::SetCpuPowersave => {
            state.cpu_powersave = true;
            spawn_cpu_power(true);
        }
        PowerMessage::SetGpuDefault => {
            state.gpu_powersave = false;
            spawn_gpu_power(false);
        }
        PowerMessage::SetGpuPowersave => {
            state.gpu_powersave = true;
            spawn_gpu_power(true);
        }
        PowerMessage::Suspend => spawn_systemctl("suspend"),
        PowerMessage::Hibernate => spawn_systemctl("hibernate"),
        PowerMessage::Reboot => spawn_systemctl("reboot"),
        PowerMessage::PowerOff => spawn_systemctl("poweroff"),
        PowerMessage::Tick => {} // handled externally
    }
}
