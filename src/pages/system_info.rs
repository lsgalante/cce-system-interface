use crate::app::{button_need, form_button, form_button_fit, wrap_to_width, AppAction, PageContent};
use cce_ui::context::UiContext;
use cce_ui::widget::Handle;
use cce_ui::compose::{PageLayoutBuilder, PageFlow};
use cce_ui::widget::{Label, WidgetHostExt};


#[derive(Debug, Clone)]
pub struct NotificationsConfig {
    pub enable: bool,
    pub bell: String,
    pub duration: i32,
}

/// The root-owned half of the desktop's install: `/usr/bin` binaries, system
/// units and `/etc/pam.d` stacks. `ccebuild install` never touches these —
/// they need root — so they drift silently, and have: the greeter fix for the
/// suspend/resume hang sat built but undeployed for weeks because deploying it
/// meant remembering to run one command in a terminal.
///
/// The plan is read with `install-system --dry-run`, which compares as the
/// normal user (every target is world-readable) and prints one `name -> dest`
/// line per pending change. Applying re-plans and runs the whole batch under a
/// single pkexec, authenticated by the session's polkit agent
/// (`cce-authenticator`) like every other privileged action in this desktop.
#[derive(Clone, Default)]
pub struct SysFiles {
    /// Whether a scan has completed; until then the section says so rather
    /// than claiming everything is up to date.
    pub scanned: bool,
    /// One `name -> dest` line per pending change. Empty after a scan means
    /// the system artifacts match the build.
    pub pending: Vec<String>,
    /// A scan or an install is in flight. Also gates the button, so a second
    /// click cannot start a second pkexec.
    pub busy: bool,
    /// Outcome of the last install attempt, shown until the next one.
    pub result: Option<Result<(), String>>,
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
    pub builds: Vec<InstalledBuild>,
    pub cpu_label: Handle<cce_ui::widget::Adapted<cce_ui::widget::Label>>,
    pub cpu_usage_label: Handle<cce_ui::widget::Adapted<cce_ui::widget::Label>>,
    pub cpu_temp_label: Handle<cce_ui::widget::Adapted<cce_ui::widget::Label>>,

    // Power-related fields

    // Native layout tracking and widgets
    pub initialized: bool,
    pub sender: Option<calloop::channel::Sender<AppAction>>,
    pub sysfiles: SysFiles,
    pub hostname_label: Handle<cce_ui::widget::Adapted<cce_ui::widget::Label>>,
    pub uptime_label: Handle<cce_ui::widget::Adapted<cce_ui::widget::Label>>,
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
            builds: Vec::new(),
            cpu_label: Handle::none(),
            cpu_usage_label: Handle::none(),
            cpu_temp_label: Handle::none(),


            initialized: false,
            sender: None,
            sysfiles: SysFiles::default(),
            hostname_label: Handle::none(),
            uptime_label: Handle::none(),
        }
    }
}

impl SystemState {
    /// The page's state, its widgets inserted into `ctx`.
    pub fn new(ctx: &mut UiContext) -> Self {
        Self {
            cpu_label: ctx.insert(Label::new("CPU Info")),
            cpu_usage_label: ctx.insert(Label::new("CPU Usage")),
            cpu_temp_label: ctx.insert(Label::new("CPU Temp")),
            hostname_label: ctx.insert(Label::new("")),
            uptime_label: ctx.insert(Label::new("")),
            ..Self::default()
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

    /// Read the pending root-owned changes (`install-system --dry-run`).
    ScanSystemFiles,
    SystemFilesScanned(Vec<String>),
    /// Apply them, authenticating through the polkit agent.
    InstallSystemFiles,
    SystemFilesInstalled(Result<(), String>),
}

// ── Helpers ─────────────────────────────────────────────────────────



fn read_cpu_temp() -> Option<f32> {
    if let Ok(entries) = std::fs::read_dir("/sys/class/hwmon") {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if let Ok(name) = std::fs::read_to_string(path.join("name")) {
                let name = name.trim();
                if name == "thinkpad" || name == "coretemp" {
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

/// `uptime -p`'s wording ("1 day, 8 hours, 13 minutes"), from seconds.
fn pretty_uptime(secs: u64) -> String {
    let units = [("year", 365 * 86400), ("week", 7 * 86400), ("day", 86400), ("hour", 3600), ("minute", 60)];
    let mut rest = secs;
    let mut parts = Vec::new();
    for (name, size) in units {
        let n = rest / size;
        rest %= size;
        if n > 0 {
            parts.push(format!("{n} {name}{}", if n == 1 { "" } else { "s" }));
        }
    }
    if parts.is_empty() {
        "0 minutes".to_string()
    } else {
        parts.join(", ")
    }
}

/// nvidia-smi's temperature, asked for only while the card is awake — the
/// query wakes a runtime-suspended card (processes.rs makes the same check
/// for its draw figure). Until 2026-10-05 the System page asked every five
/// seconds regardless, which kept the dGPU out of D3cold while it was open.
async fn read_nvidia_gpu_temp() -> Option<f32> {
    if !crate::power_meter::dgpu_awake() {
        return None;
    }
    let out = tokio::process::Command::new("nvidia-smi")
        .args(["--query-gpu=temperature.gpu", "--format=csv,noheader,nounits"])
        .output().await.ok()?;
    let val_str = String::from_utf8_lossy(&out.stdout);
    val_str.trim().parse::<f32>().ok()
}

/// The installed `ccebuild`, by absolute path. An app launched from the menu
/// gets systemd's environment rather than the session's, so `~/.local/bin` is
/// not reliably on PATH — the same trap the desktop-menu script avoids by
/// spelling out `$HOME/.local/bin/ccectl`. Falls back to the bare name so a
/// PATH that does have it still works.
fn ccebuild_path() -> std::path::PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        let p = std::path::Path::new(&home).join(".local/bin/ccebuild");
        if p.is_file() {
            return p;
        }
    }
    std::path::PathBuf::from("ccebuild")
}

/// The `  name -> dest` lines of a dry run: one per root-owned file that
/// differs from the build. Anything else the script prints (the `==>` banners,
/// the queued root commands) is not a pending change and is dropped.
fn parse_pending(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .filter(|l| l.starts_with("  ") && l.contains(" -> "))
        .map(|l| l.trim().to_string())
        .collect()
}

/// Run one `ccebuild install-system` variant off the UI thread and post the
/// result back through the page's sender. Both calls shell out rather than
/// reimplementing the compare, so the plan has exactly one author — the same
/// reason the apply re-plans instead of trusting what the scan printed.
fn spawn_sysfiles<F>(sender: Option<calloop::channel::Sender<AppAction>>, args: &'static [&'static str], done: F)
where
    F: FnOnce(std::io::Result<std::process::Output>) -> SystemMessage + Send + 'static,
{
    let Some(tx) = sender else { return };
    std::thread::spawn(move || {
        let out = std::process::Command::new(ccebuild_path()).args(args).output();
        let _ = tx.send(AppAction::SystemInfo(done(out)));
    });
}

/// A line of text wrapped to the form's width rather than cut at it: its lines one under the
/// next, so one wrapped item still reads as one item.
fn wrapped_text(g: &mut cce_ui::compose::FormGroup<'_, '_, PageContent>, text: &str, size: f32, color: [f32; 4]) {
    let lines = wrap_to_width(text, g.form_width(), size);
    g.lines(lines, size, color);
}

fn spawn_systemctl(action: &str) {
    let mut cmd = std::process::Command::new("systemctl");
    cmd.arg(action);
    let _ = crate::spawn_detached(cmd);
}

fn spawn_systemctl_force(action: &str) {
    let mut cmd = std::process::Command::new("systemctl");
    cmd.arg(action);
    cmd.arg("-f");
    cmd.arg("-f");
    let _ = crate::spawn_detached(cmd);
}

/// One compiled cce binary in `~/.local/bin` and how long it took to build.
#[derive(Debug, Clone, PartialEq)]
pub struct InstalledBuild {
    pub name: String,
    /// Seconds the last recorded build of this binary took: its own bin unit
    /// plus its crate's lib and build script, not its dependencies. `None`
    /// when no build of it went through `ccebuild`, the only thing that
    /// records them.
    pub seconds: Option<f32>,
}

/// `ccebuild`'s build-time record, `bin<TAB>seconds<TAB>epoch` per line
/// (`BUILD_TIMES` in the script, filled from cargo's `--timings` reports).
/// Cargo keeps no build durations of its own, so a binary built by a bare
/// `cargo build` has no line here.
fn parse_build_times(text: &str) -> std::collections::HashMap<String, f32> {
    text.lines()
        .filter_map(|l| {
            let mut f = l.split('\t');
            let bin = f.next()?;
            let secs = f.next()?.parse::<f32>().ok()?;
            Some((bin.to_string(), secs))
        })
        .collect()
}

/// "8.4s" / "1min 32s" — a build time, which unlike an age wants its tenths
/// under a minute.
fn format_build_duration(secs: f32) -> String {
    // Rounded first, so 59.96 reads "1min 00s" rather than "60.0s".
    if (secs * 10.0).round() < 600.0 {
        format!("{:.1}s", secs)
    } else {
        let s = secs.round() as u64;
        format!("{}min {:02}s", s / 60, s % 60)
    }
}

/// Every compiled `cce*` binary in `~/.local/bin` with its recorded build
/// time, slowest first and unrecorded ones last. Symlinks (`cce`,
/// `cce-settings`) are aliases, and shell scripts (`ccebuild`, `cce-shadow`)
/// have no build, so only ELF files count.
fn scan_installed_builds() -> Vec<InstalledBuild> {
    use std::io::Read;
    let Some(home) = std::env::var_os("HOME") else { return Vec::new() };
    let home = std::path::Path::new(&home);
    let bindir = home.join(".local/bin");
    let state = std::env::var_os("XDG_STATE_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| home.join(".local/state"));
    let times = std::fs::read_to_string(state.join("cce/build-times"))
        .map(|t| parse_build_times(&t))
        .unwrap_or_default();
    let Ok(entries) = std::fs::read_dir(&bindir) else { return Vec::new() };

    let mut builds: Vec<InstalledBuild> = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            if !name.starts_with("cce") || !e.file_type().ok()?.is_file() {
                return None;
            }
            let mut magic = [0u8; 4];
            std::fs::File::open(e.path()).ok()?.read_exact(&mut magic).ok()?;
            if &magic != b"\x7fELF" {
                return None;
            }
            let seconds = times.get(&name).copied();
            Some(InstalledBuild { name, seconds })
        })
        .collect();
    builds.sort_by(|a, b| match (a.seconds, b.seconds) {
        (Some(x), Some(y)) => y.total_cmp(&x).then_with(|| a.name.cmp(&b.name)),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.name.cmp(&b.name),
    });
    builds
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
    pub builds: Vec<InstalledBuild>,
}

pub async fn fetch_system_state() -> SystemInfo {
    // The kernel's own copy, not the `hostname` binary: inetutils is not
    // part of a base Arch install, and without it the System well opened on
    // a bare "—  Linux …" with nothing in front of the dash.
    let hostname = std::fs::read_to_string("/proc/sys/kernel/hostname")
        .ok()
        .and_then(|h| h.trim().split('.').next().map(|s| s.to_string()))
        .unwrap_or_default();

    // Both from /proc rather than spawning `uname -r` and `uptime -p` on
    // every refresh.
    let kernel = std::fs::read_to_string("/proc/sys/kernel/osrelease")
        .map(|k| k.trim().to_string())
        .unwrap_or_default();

    let uptime = std::fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|u| u.split_whitespace().next()?.parse::<f64>().ok())
        .map(|secs| pretty_uptime(secs as u64))
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
        // Busy share since the previous refresh; the first one samples a
        // 100 ms window, as every refresh used to.
        static LAST: std::sync::Mutex<Option<(u64, u64)>> = std::sync::Mutex::new(None);
        let prev = *LAST.lock().unwrap();
        let (idle1, total1) = match prev {
            Some(p) => p,
            None => {
                let first = read_stat().unwrap_or((0, 1));
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                first
            }
        };
        let (idle2, total2) = read_stat().unwrap_or((0, 1));
        *LAST.lock().unwrap() = Some((idle2, total2));
        let d_idle = idle2.saturating_sub(idle1);
        let d_total = total2.saturating_sub(total1);
        if d_total > 0 {
            (1.0 - d_idle as f64 / d_total as f64) * 100.0
        } else { 0.0 }
    } as f32;

    static GPUS_INFO: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    let gpus = GPUS_INFO.get_or_init(|| {
        let mut list = Vec::new();
        if let Ok(o) = std::process::Command::new("lspci").output() {
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


    SystemInfo {
        hostname,
        kernel,
        uptime,
        cpu_model,
        cpu_usage,
        cpu_cores,
        gpus,
        gpu_strings,
        builds: scan_installed_builds(),
    }
}

const TEXT_FG: [f32; 4] = [0.83, 0.83, 0.83, 1.0];
const TEXT_DIM: [f32; 4] = [0.53, 0.53, 0.60, 1.0];
const BTN_HOVER: [f32; 4] = [0.25, 0.30, 0.26, 1.0];
const DANGER_BG: [f32; 4] = [0.67, 0.20, 0.20, 1.0];
const SAFE_BG: [f32; 4] = [0.20, 0.33, 0.22, 1.0];
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

pub fn view(state: &mut SystemState, cx: f32, cy: f32, cw: f32, ch: f32, _root_focused: bool, sec_focused: &[bool], layout: &mut PageFlow, ctx: &mut cce_ui::context::UiContext) -> PageContent {
    let mut final_pc = PageContent::new();
    let sec_w = 320.0f32;
    // Six, matching the add_section calls below and the six groups
    // section_widgets reports. The count caps the grid's column count
    // (`n.min(cols)`), so a stale count only bites once the window is wide
    // enough for that many columns — harmless, but it reads as a missing
    // section.
    let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(6);

    // ── 1. System Section ──
    builder.add_section(&mut final_pc, "System", sec_focused.first().copied().unwrap_or(false), |sec| {
        let mut form = sec.form();
        form.column().block(|b| {
            if !state.loaded {
                b.text("Loading system information...", 14.0, TEXT_FG);
            } else {
                wrapped_text(b, &format!("{}  —  Linux {}", state.hostname, state.kernel), 14.0, TEXT_FG);
                wrapped_text(b, &format!("Uptime: {}", state.uptime), 12.0, TEXT_DIM);
            }
        });
        sec.place(form, ctx);
    });

    // ── 2. System Actions Section ──
    builder.add_section(&mut final_pc, "System Actions", sec_focused.get(1).copied().unwrap_or(false), |sec| {
        const ACTIONS: [(&str, [f32; 4], SystemMessage); 4] = [
            ("Suspend", SAFE_BG, SystemMessage::Suspend),
            ("Hibernate", SAFE_BG, SystemMessage::Hibernate),
            ("Reboot", DANGER_BG, SystemMessage::Reboot),
            ("Power Off", DANGER_BG, SystemMessage::PowerOff),
        ];
        let needs: Vec<f32> = ACTIONS.iter().map(|a| button_need(a.0)).collect();
        let mut form = sec.form();
        // All four on one row when their labels fit; otherwise the safe pair over the
        // destructive pair, rather than squeezing every plate below its label.
        let gap = cce_ui::layout::control_gap();
        let one_row = needs.iter().sum::<f32>() + gap * (needs.len() - 1) as f32 <= form.width();
        // A slice of row ranges: `[0..4]` is one row, not the indices 0 to 3.
        #[allow(clippy::single_range_in_vec_init)]
        let rows: &[std::ops::Range<usize>] = if one_row { &[0..4] } else { &[0..2, 2..4] };
        let mut col = form.column();
        for range in rows {
            col.row(|r| {
                for i in range.clone() {
                    let (label, bg, msg) = ACTIONS[i].clone();
                    form_button_fit(r, label, needs[i], (bg, BTN_HOVER, WHITE), AppAction::SystemInfo(msg));
                }
            });
        }
        form_button(&mut col, "Force Shutdown", 0.0, (DANGER_BG, BTN_HOVER, WHITE), AppAction::SystemInfo(SystemMessage::ForceShutdown));
        sec.place(form, ctx);
    });

    // ── 3. CPU Section ──
    builder.add_section(&mut final_pc, "CPU", sec_focused.get(2).copied().unwrap_or(false), |sec| {
        let mut form = sec.form();
        form.column().block(|b| {
            if !state.loaded {
                b.text("Loading CPU model and utilization...", 12.0, TEXT_FG);
            } else {
                wrapped_text(b, &format!("CPU  {}  ({} cores)", state.cpu_model, state.cpu_cores), 12.0, TEXT_FG);
                wrapped_text(b, &format!("Usage  {:.0}%", state.cpu_usage), 12.0, TEXT_FG);
                let cpu_temp_text = read_cpu_temp().map(|t| format!("Temp  {:.0}°C", t)).unwrap_or_else(|| "Temp  N/A".to_string());
                wrapped_text(b, &cpu_temp_text, 12.0, TEXT_FG);
            }
        });
        sec.place(form, ctx);
    });

    // ── 4. GPU Section ──
    builder.add_section(&mut final_pc, "GPU", sec_focused.get(3).copied().unwrap_or(false), |sec| {
        let mut form = sec.form();
        form.column().block(|b| {
            if !state.loaded {
                b.text("Loading GPU models...", 12.0, TEXT_FG);
            } else {
                for gpu_text in state.gpu_strings.iter() {
                    wrapped_text(b, gpu_text, 12.0, TEXT_FG);
                }
            }
        });
        sec.place(form, ctx);
    });

    // ── 5. System Files Section ──
    // The root-owned half of the install, which `ccebuild install` cannot
    // touch. It lives here rather than on the desktop menu so the pending
    // changes can be READ before they are authorized: this batch can rewrite
    // /etc/pam.d, and a flat menu button would be one misclick from replacing
    // the greeter out of a half-built tree.
    builder.add_section(&mut final_pc, "System Files", sec_focused.get(4).copied().unwrap_or(false), |sec| {
        let sf = &state.sysfiles;
        let (status, color) = if sf.busy {
            ("Checking…".to_string(), TEXT_DIM)
        } else if !sf.scanned {
            ("Not checked yet".to_string(), TEXT_DIM)
        } else if sf.pending.is_empty() {
            ("Up to date with the build".to_string(), TEXT_DIM)
        } else {
            (
                format!(
                    "{} file{} differ{} from the build",
                    sf.pending.len(),
                    if sf.pending.len() == 1 { "" } else { "s" },
                    if sf.pending.len() == 1 { "s" } else { "" },
                ),
                TEXT_FG,
            )
        };
        let mut form = sec.form();
        let wrap_w = form.width();
        let mut col = form.column();
        col.block(|b| {
            b.text(status, 12.0, color);
            // Name the files. "2 files differ" is not enough to authorize a root
            // install on — which one is the greeter matters. A path pair is wider
            // than the well, and the well clips: wrapped, continuation lines
            // indented past the first, so each file still reads as one entry.
            let indent = cce_ui::layout::CONTROL_TEXT_INSET;
            for line in sf.pending.iter().take(8) {
                let lines = wrap_to_width(line, wrap_w, 11.0);
                let (first, rest) = lines.split_first().map_or((String::new(), &[][..]), |(f, r)| (f.clone(), r));
                let rest: Vec<String> = rest.iter().flat_map(|l| wrap_to_width(l, wrap_w - indent, 11.0)).collect();
                b.text(first, 11.0, TEXT_DIM);
                if !rest.is_empty() {
                    b.row(|r| {
                        r.space(indent, false);
                        r.lines(rest, 11.0, TEXT_DIM);
                    });
                }
            }
        });

        if let Some(ref res) = sf.result {
            let (msg, color) = match res {
                Ok(()) => ("Installed — takes effect at next login".to_string(), TEXT_DIM),
                Err(e) => (format!("Failed: {}", e), DANGER_BG),
            };
            wrapped_text(&mut col, &msg, 11.0, color);
        }

        let has_work = sf.scanned && !sf.pending.is_empty();
        let busy = sf.busy;
        let (check_need, install_need) = (button_need("Check"), button_need("Install (root)"));
        col.row(|r| {
            form_button_fit(r, "Check", check_need, (SAFE_BG, BTN_HOVER, WHITE), AppAction::SystemInfo(SystemMessage::ScanSystemFiles));
            // Only offered when there is something to install. The apply re-plans anyway,
            // so a stale-enabled button would be a no-op rather than a hazard — but it
            // would also put up a root prompt for nothing. Its room is kept either way, so
            // Check does not change width.
            if has_work && !busy {
                form_button_fit(r, "Install (root)", install_need, (DANGER_BG, BTN_HOVER, WHITE), AppAction::SystemInfo(SystemMessage::InstallSystemFiles));
            } else {
                r.space(install_need, true);
            }
        });
        sec.place(form, ctx);
    });

    // ── 6. Builds Section ──
    // How long each installed cce binary took to build, slowest first.
    builder.add_section(&mut final_pc, "Builds", sec_focused.get(5).copied().unwrap_or(false), |sec| {
        let mut form = sec.form();
        let mut col = form.column();
        if !state.loaded {
            col.text("Loading installed builds...", 12.0, TEXT_FG);
        } else if state.builds.is_empty() {
            col.text("No cce binaries in ~/.local/bin", 12.0, TEXT_DIM);
        } else {
            // "—" is a binary no ccebuild build has timed (a bare `cargo
            // build`, or one built before the timing was recorded).
            let pairs = state
                .builds
                .iter()
                .map(|b| (b.name.clone(), TEXT_FG, b.seconds.map(format_build_duration).unwrap_or_else(|| "—".to_string()), TEXT_DIM))
                .collect();
            crate::app::form_pairs(&mut col, 11.0, pairs);
        }
        sec.place(form, ctx);
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
            state.builds = new.builds;

            if state.loaded {
                ctx[state.hostname_label].set_text(&format!("{}  —  Linux {}", state.hostname, state.kernel));
                ctx[state.uptime_label].set_text(&format!("Uptime: {}", state.uptime));

                let cpu_label_text = format!("CPU  {}  ({} cores)", state.cpu_model, state.cpu_cores);
                let cpu_usage_text = format!("Usage  {:.0}%", state.cpu_usage);
                let cpu_temp_text = read_cpu_temp().map(|t| format!("Temp  {:.0}°C", t)).unwrap_or_else(|| "Temp  N/A".to_string());

                ctx[state.cpu_label].set_text(&cpu_label_text);
                ctx[state.cpu_usage_label].set_text(&cpu_usage_text);
                ctx[state.cpu_temp_label].set_text(&cpu_temp_text);



                ctx.lend_h(state.hostname_label, |w, ctx| w.mark_dirty(ctx));
            }

            // First refresh doubles as the first system-files scan, so the
            // section has an answer without the user pressing Check. Guarded
            // on both flags because Refreshed repeats on a timer and each scan
            // is a process spawn.
            if !state.sysfiles.scanned && !state.sysfiles.busy {
                update(state, SystemMessage::ScanSystemFiles, ctx);
            }
        }
        SystemMessage::Suspend => spawn_systemctl("suspend"),
        SystemMessage::Hibernate => spawn_systemctl("hibernate"),
        SystemMessage::Reboot => spawn_systemctl("reboot"),
        SystemMessage::PowerOff => spawn_systemctl("poweroff"),
        SystemMessage::ForceShutdown => spawn_systemctl_force("poweroff"),

        SystemMessage::ScanSystemFiles => {
            state.sysfiles.busy = true;
            spawn_sysfiles(state.sender.clone(), &["install-system", "--dry-run"], |out| {
                let pending = match out {
                    Ok(o) => parse_pending(&String::from_utf8_lossy(&o.stdout)),
                    // A scan that could not run reports nothing pending, and
                    // `scanned` still flips — the section then says "up to
                    // date", which is wrong but harmless, where a spinner that
                    // never resolves would be a hang. The install button is the
                    // real check: it re-plans and would find the work.
                    Err(_) => Vec::new(),
                };
                SystemMessage::SystemFilesScanned(pending)
            });
        }
        SystemMessage::SystemFilesScanned(pending) => {
            state.sysfiles.pending = pending;
            state.sysfiles.scanned = true;
            state.sysfiles.busy = false;
        }
        SystemMessage::InstallSystemFiles => {
            state.sysfiles.busy = true;
            state.sysfiles.result = None;
            // --pkexec explicitly rather than letting the auto path decide:
            // this process has no TTY, so auto would pick pkexec anyway, but
            // saying so keeps the GUI's behaviour independent of how the
            // script guesses.
            spawn_sysfiles(state.sender.clone(), &["install-system", "--pkexec"], |out| {
                let res = match out {
                    Ok(o) if o.status.success() => Ok(()),
                    // A cancelled or failed polkit prompt exits non-zero with
                    // its reason on stderr; surface that rather than a code.
                    Ok(o) => {
                        let err = String::from_utf8_lossy(&o.stderr).trim().to_string();
                        Err(if err.is_empty() { format!("exited {}", o.status) } else { err })
                    }
                    Err(e) => Err(e.to_string()),
                };
                SystemMessage::SystemFilesInstalled(res)
            });
        }
        SystemMessage::SystemFilesInstalled(res) => {
            let ok = res.is_ok();
            state.sysfiles.result = Some(res);
            state.sysfiles.busy = false;
            if ok {
                // Re-scan rather than assuming the list is now empty: the
                // apply re-plans, so what it actually did is only knowable by
                // asking again.
                state.sysfiles.scanned = false;
                update(state, SystemMessage::ScanSystemFiles, ctx);
            }
        }
    }
}



impl crate::pages::AppPage for SystemState {
    // Sections: [System, System Actions, CPU, GPU, System Files, Builds]
    fn section_widgets(&mut self) -> Vec<Vec<cce_ui::widget::WidgetId>> {
        vec![
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
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
        layout: &mut PageFlow,
        ctx: &mut cce_ui::context::UiContext,
    ) -> crate::app::PageContent {
        // Phase 6u: System used to render through the WIDGET TREE (the only page that
        // did) — its content reached the frame via the root aggregate walking
        // Switcher → Page(AdaptiveGridLayout) → sections → widgets. With that chain
        // dissolved, the page renders through the same immediate-mode view as every
        // other page (this free `view` predates the flip; it was never wired up).
        view(self, cx, cy, cw, ch, root_focused, sec_focused, layout, ctx)
    }

    fn propagate_widget_changes(&mut self, _actions: &mut Vec<crate::app::AppAction>, _ctx: &mut UiContext) {
    }
}





#[cfg(test)]
mod tests {
    use super::*;

    /// The dry run prints pending changes, banners and the queued root
    /// commands down one stream; only the indented `name -> dest` lines are
    /// changes. Getting this wrong in the lenient direction would list
    /// `install -m 644 ... -> ...` as a pending file, and in the strict
    /// direction would hide a pending PAM stack — which is the one thing the
    /// section exists to show before a root install is authorized.
    #[test]
    fn parse_pending_takes_only_the_change_lines() {
        let out = "  cce-display-manager -> /usr/bin/cce-display-manager\n                   \x20 cce-lock -> /etc/pam.d/cce-lock\n                   ==> dry run; would run as root:\n                   \n                   cp -a /usr/bin/cce-display-manager /usr/bin/cce-display-manager.bak-2026-09-19 \n                   install -m 644 /src/cce-lock /etc/pam.d/cce-lock \n";
        assert_eq!(
            parse_pending(out),
            vec![
                "cce-display-manager -> /usr/bin/cce-display-manager".to_string(),
                "cce-lock -> /etc/pam.d/cce-lock".to_string(),
            ]
        );
    }

    #[test]
    fn parse_pending_is_empty_when_nothing_differs() {
        assert!(parse_pending("==> system artifacts already up to date\n").is_empty());
    }

    /// The record is ccebuild's TSV; a malformed line (a hand edit, a
    /// half-written file) drops out rather than failing the whole section.
    #[test]
    fn parse_build_times_reads_ccebuild_lines() {
        let t = parse_build_times("cce-mail\t83.27\t1791251542\ncce-ui\t4.1\t1791251542\nbroken line\ncce-x\tnan?\t1\n");
        assert_eq!(t.len(), 2);
        assert_eq!(t.get("cce-mail"), Some(&83.27));
        assert_eq!(t.get("cce-ui"), Some(&4.1));
    }

    #[test]
    fn format_build_duration_keeps_tenths_under_a_minute() {
        assert_eq!(format_build_duration(8.44), "8.4s");
        assert_eq!(format_build_duration(92.0), "1min 32s");
        assert_eq!(format_build_duration(59.96), "1min 00s");
    }

    #[test]
    fn test_view_layout_grid() {
        let mut ui = cce_ui::context::UiContext::new();
        let mut state = SystemState::new(&mut ui);
        let mut layout = cce_ui::compose::PageFlow::new();
        let sec_focused = vec![false, false, false, false, false, false, false];
        let pc = view(&mut state, 10.0, 20.0, 800.0, 600.0, false, &sec_focused, &mut layout, &mut ui);
        assert!(!pc.rects.is_empty() || !pc.texts.is_empty());
    }
}

#[cfg(test)]
mod uptime_tests {
    use super::pretty_uptime;

    #[test]
    fn reads_like_uptime_p() {
        assert_eq!(pretty_uptime(30), "0 minutes");
        assert_eq!(pretty_uptime(60), "1 minute");
        assert_eq!(pretty_uptime(86400 + 8 * 3600 + 13 * 60 + 5), "1 day, 8 hours, 13 minutes");
        assert_eq!(pretty_uptime(15 * 86400), "2 weeks, 1 day");
    }
}
