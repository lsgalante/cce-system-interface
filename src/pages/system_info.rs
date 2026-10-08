use crate::app::{AppAction, PageContent, SectionContextExt};
use cce_ui::widget::Owned;
use cce_ui::layout::{PageLayoutBuilder, LayoutStrategy};
use cce_ui::widget::{Label, WidgetHost, Button};


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
    pub cpu_label: Owned<cce_ui::widget::Adapted<cce_ui::widget::Label>>,
    pub cpu_usage_label: Owned<cce_ui::widget::Adapted<cce_ui::widget::Label>>,
    pub cpu_temp_label: Owned<cce_ui::widget::Adapted<cce_ui::widget::Label>>,

    // Power-related fields

    // Native layout tracking and widgets
    pub initialized: bool,
    pub sender: Option<calloop::channel::Sender<AppAction>>,
    pub sysfiles: SysFiles,
    pub hostname_label: Owned<cce_ui::widget::Adapted<cce_ui::widget::Label>>,
    pub uptime_label: Owned<cce_ui::widget::Adapted<cce_ui::widget::Label>>,
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
            cpu_label: Owned::new(Label::new("CPU Info")),
            cpu_usage_label: Owned::new(Label::new("CPU Usage")),
            cpu_temp_label: Owned::new(Label::new("CPU Temp")),


            initialized: false,
            sender: None,
            sysfiles: SysFiles::default(),
            hostname_label: Owned::new(Label::new("")),
            uptime_label: Owned::new(Label::new("")),
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

/// Width a label needs on a button plate. Feeds `add_row_for`, so a row of
/// buttons is divided by what is written on them rather than into equal
/// slices — "Reboot" and "Hibernate" are not the same size and a row that
/// pretends otherwise clips one and pads the other.
///
/// Asks the Button itself (`intrinsic_size`: its label shaped in the button
/// font, plus its 8px inset each side) rather than measuring here. This used
/// `measure_text_width` in the control-label font, an inked extent that runs
/// ~20% short of the shaped run under Berkeley Mono, so the row handed every
/// button less than its label and "Hibernate" / "Power Off" were cut off.
fn button_need(label: &str) -> f32 {
    Button::new(0.0, 0.0, 0.0, 0.0)
        .with_label(label)
        .intrinsic_size()
        .map_or(0.0, |s| s.width)
}

/// `text` broken into lines no wider than `width` at `size`, measured as the
/// renderer shapes it (the default UI face, as `PageContent::text` draws).
/// Breaks after a space or before a `/`, so a path splits on its separators;
/// a run with neither is cut wherever it overflows. A section's text is
/// clipped to its content box, so anything that does not fit has to wrap or
/// it is simply lost — a GPU name, the CPU model, a pending install path.
fn wrap_to_width(text: &str, width: f32, size: f32) -> Vec<String> {
    let offsets = cce_ui::geometry_font_system()
        .lock()
        .map(|mut fs| cce_ui::backend::text::shaped_cluster_offsets(&mut fs, text, size, None))
        .unwrap_or_default();
    if offsets.last().map_or(true, |&(_, total)| total <= width) {
        return vec![text.to_string()];
    }
    let mut lines = Vec::new();
    let (mut start, mut start_x) = (0usize, 0.0f32);
    // Where the next line would begin if this one broke at the last chance.
    let mut chance: Option<(usize, f32)> = None;
    for pair in offsets.windows(2) {
        let ((b, x), (next_b, next_x)) = (pair[0], pair[1]);
        if next_x - start_x > width && b > start {
            let (cut, cut_x) = chance.filter(|&(c, _)| c > start).unwrap_or((b, x));
            lines.push(text[start..cut].trim_end().to_string());
            (start, start_x, chance) = (cut, cut_x, None);
        }
        match &text[b..next_b] {
            " " => chance = Some((next_b, next_x)),
            "/" if b > start => chance = Some((b, x)),
            _ => {}
        }
    }
    lines.push(text[start..].trim_end().to_string());
    lines.retain(|l| !l.is_empty());
    lines
}

/// [`SectionContext::text`](cce_ui::layout::SectionContext::text), wrapped
/// to the content box instead of clipped by it. Continuation lines follow at
/// the line pitch `text` itself advances by, without the row gap, so one
/// wrapped item still reads as one item.
fn wrapped_text(sec: &mut cce_ui::layout::SectionContext<'_, PageContent>, text: &str, size: f32, color: [f32; 4]) {
    use cce_ui::layout::RenderTarget;
    let x = sec.content_left();
    let right = x + sec.content_width();
    let mut y = sec.content_y;
    if y > sec.content_start_y {
        y += sec.row_gap;
    }
    for line in wrap_to_width(text, sec.content_width(), size) {
        sec.pc.text_with_bounds(&line, x, y, size, color, Some([x, y - size, right, y + 2.0 * size]));
        y += size + 4.0;
    }
    sec.content_y = y;
    for h in &mut sec.grid.col_heights {
        *h = y;
    }
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

pub fn view(state: &mut SystemState, cx: f32, cy: f32, cw: f32, ch: f32, _root_focused: bool, sec_focused: &[bool], layout: &mut dyn LayoutStrategy, _ctx: &mut cce_ui::context::UiContext) -> PageContent {
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
        if !state.loaded {
            sec.text("Loading system information...", 12.0, 0.0, 14.0, TEXT_FG);
        } else {
            wrapped_text(sec, &format!("{}  —  Linux {}", state.hostname, state.kernel), 14.0, TEXT_FG);
            wrapped_text(sec, &format!("Uptime: {}", state.uptime), 12.0, TEXT_DIM);
        }
    });

    // ── 2. System Actions Section ──
    builder.add_section(&mut final_pc, "System Actions", sec_focused.get(1).copied().unwrap_or(false), |sec| {
        let gap = cce_ui::layout::plate_gap();
        let mut stack = sec.vstack(gap);
        // The toolkit's button height, like every other cce-ui button — this
        // row was a hand-set 32px, a size no other control in the DE wears.
        let btn_h = cce_ui::layout::button_height();

        const ACTIONS: [&str; 4] = ["Suspend", "Hibernate", "Reboot", "Power Off"];
        let needs: Vec<f32> = ACTIONS.iter().map(|l| button_need(l)).collect();
        // All four on one row when their labels fit; otherwise the safe pair
        // over the destructive pair, rather than squeezing every plate below
        // its label (`row_layout_for` scales an overfull row down, which is
        // what clipped the labels).
        let one_row = needs.iter().sum::<f32>() + gap * (needs.len() - 1) as f32
            <= stack.context.content_width();
        let rows: &[std::ops::Range<usize>] = if one_row { &[0..4] } else { &[0..2, 2..4] };
        for r in rows {
            let first = r.start;
            stack.add_row_for(&needs[r.clone()], gap, btn_h, |ctx, i, x, w| {
                let (label, bg, msg) = match first + i {
                    0 => ("Suspend", SAFE_BG, SystemMessage::Suspend),
                    1 => ("Hibernate", SAFE_BG, SystemMessage::Hibernate),
                    2 => ("Reboot", DANGER_BG, SystemMessage::Reboot),
                    _ => ("Power Off", DANGER_BG, SystemMessage::PowerOff),
                };
                ctx.button(label, x, ctx.ay(), w, btn_h, bg, BTN_HOVER, WHITE, AppAction::SystemInfo(msg));
            });
        }

        // No trailing gap after the last row: the well's own floor inset is
        // the space below it, as it is beside and above.
        stack.spacing = 0.0;
        stack.add_row(1, 0.0, btn_h, |ctx, _, x, w| {
            ctx.button("Force Shutdown", x, ctx.ay(), w, btn_h,
                DANGER_BG, BTN_HOVER, WHITE, AppAction::SystemInfo(SystemMessage::ForceShutdown));
        });
    });

    // ── 3. CPU Section ──
    builder.add_section(&mut final_pc, "CPU", sec_focused.get(2).copied().unwrap_or(false), |sec| {
        if !state.loaded {
            sec.text("Loading CPU model and utilization...", 12.0, 0.0, 12.0, TEXT_FG);
        } else {
            // Same strings the old Label widgets carried, stacked vertically (the
            // grid-column widget placement overlapped them at narrow widths).
            wrapped_text(sec, &format!("CPU  {}  ({} cores)", state.cpu_model, state.cpu_cores), 12.0, TEXT_FG);
            wrapped_text(sec, &format!("Usage  {:.0}%", state.cpu_usage), 12.0, TEXT_FG);
            let cpu_temp_text = read_cpu_temp().map(|t| format!("Temp  {:.0}°C", t)).unwrap_or_else(|| "Temp  N/A".to_string());
            wrapped_text(sec, &cpu_temp_text, 12.0, TEXT_FG);
        }
    });

    // ── 4. GPU Section ──
    builder.add_section(&mut final_pc, "GPU", sec_focused.get(3).copied().unwrap_or(false), |sec_gpu| {
        if !state.loaded {
            sec_gpu.text("Loading GPU models...", 12.0, 0.0, 12.0, TEXT_FG);
        } else {
            for gpu_text in state.gpu_strings.iter() {
                wrapped_text(sec_gpu, gpu_text, 12.0, TEXT_FG);
            }
        }
    });

    // ── 5. System Files Section ──
    // The root-owned half of the install, which `ccebuild install` cannot
    // touch. It lives here rather than on the desktop menu so the pending
    // changes can be READ before they are authorized: this batch can rewrite
    // /etc/pam.d, and a flat menu button would be one misclick from replacing
    // the greeter out of a half-built tree.
    builder.add_section(&mut final_pc, "System Files", sec_focused.get(4).copied().unwrap_or(false), |sec| {
        let sf = &state.sysfiles;
        let mut stack = sec.vstack(6.0);

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
        stack.add_row(1, 0.0, 16.0, |c, _, x, _| {
            let y = c.ay();
            c.pc.text(&status, x, y + 4.0, 12.0, color);
        });

        // Name the files. "2 files differ" is not enough to authorize a root
        // install on — which one is the greeter matters.
        // A path pair is wider than the well, and the well clips: wrapped,
        // continuation lines indented past the first, so each file still
        // reads as one entry.
        let wrap_w = stack.context.content_width();
        for line in sf.pending.iter().take(8) {
            let lines = wrap_to_width(line, wrap_w - 8.0, 11.0);
            let (first, rest) = lines.split_first().map_or((String::new(), &[][..]), |(f, r)| (f.clone(), r));
            let rest: Vec<String> = rest.iter().flat_map(|l| wrap_to_width(l, wrap_w - 20.0, 11.0)).collect();
            stack.add_row(1, 0.0, 14.0 * (1 + rest.len()) as f32, move |c, _, x, _| {
                let y = c.ay();
                c.pc.text(&first, x + 8.0, y + 3.0, 11.0, TEXT_DIM);
                for (k, l) in rest.iter().enumerate() {
                    c.pc.text(l, x + 20.0, y + 3.0 + 14.0 * (k + 1) as f32, 11.0, TEXT_DIM);
                }
            });
        }

        if let Some(ref res) = sf.result {
            let (msg, col) = match res {
                Ok(()) => ("Installed — takes effect at next login".to_string(), TEXT_DIM),
                Err(e) => (format!("Failed: {}", e), DANGER_BG),
            };
            let lines = wrap_to_width(&msg, stack.context.content_width(), 11.0);
            stack.add_row(1, 0.0, 16.0 + 14.0 * (lines.len().max(1) - 1) as f32, move |c, _, x, _| {
                let y = c.ay();
                for (k, l) in lines.iter().enumerate() {
                    c.pc.text(l, x, y + 4.0 + 14.0 * k as f32, 11.0, col);
                }
            });
        }

        let btn_h = cce_ui::layout::button_height();
        // The buttons close the well: no trailing gap below them.
        stack.spacing = 0.0;
        let has_work = sf.scanned && !sf.pending.is_empty();
        let busy = sf.busy;
        let needs = [button_need("Check"), button_need("Install (root)")];
        stack.add_row_for(&needs, cce_ui::layout::plate_gap(), btn_h, move |c, i, x, w| {
            match i {
                0 => {
                    c.button("Check", x, c.ay(), w, btn_h,
                        SAFE_BG, BTN_HOVER, WHITE,
                        AppAction::SystemInfo(SystemMessage::ScanSystemFiles));
                }
                1 => {
                    // Only offered when there is something to install. The
                    // apply re-plans anyway, so a stale-enabled button would
                    // be a no-op rather than a hazard — but it would also put
                    // up a root prompt for nothing.
                    if has_work && !busy {
                        c.button("Install (root)", x, c.ay(), w, btn_h,
                            DANGER_BG, BTN_HOVER, WHITE,
                            AppAction::SystemInfo(SystemMessage::InstallSystemFiles));
                    }
                }
                _ => {}
            }
        });
    });

    // ── 6. Builds Section ──
    // How long each installed cce binary took to build, slowest first.
    builder.add_section(&mut final_pc, "Builds", sec_focused.get(5).copied().unwrap_or(false), |sec| {
        if !state.loaded {
            sec.text("Loading installed builds...", 12.0, 0.0, 12.0, TEXT_FG);
            return;
        }
        if state.builds.is_empty() {
            sec.text("No cce binaries in ~/.local/bin", 12.0, 0.0, 12.0, TEXT_DIM);
            return;
        }
        let mut stack = sec.vstack(0.0);
        for b in &state.builds {
            // "—" is a binary no ccebuild build has timed (a bare `cargo
            // build`, or one built before the timing was recorded).
            let took = b.seconds.map(format_build_duration).unwrap_or_else(|| "—".to_string());
            let name = b.name.clone();
            stack.add_row(1, 0.0, 16.0, move |c, _, x, _| {
                let y = c.ay();
                c.pc.text(&name, x, y + 3.0, 11.0, TEXT_FG);
                c.pc.text(&took, x + 140.0, y + 3.0, 11.0, TEXT_DIM);
            });
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
            state.builds = new.builds;

            if state.loaded {
                state.hostname_label.set_text(&format!("{}  —  Linux {}", state.hostname, state.kernel));
                state.uptime_label.set_text(&format!("Uptime: {}", state.uptime));

                let cpu_label_text = format!("CPU  {}  ({} cores)", state.cpu_model, state.cpu_cores);
                let cpu_usage_text = format!("Usage  {:.0}%", state.cpu_usage);
                let cpu_temp_text = read_cpu_temp().map(|t| format!("Temp  {:.0}°C", t)).unwrap_or_else(|| "Temp  N/A".to_string());

                state.cpu_label.set_text(&cpu_label_text);
                state.cpu_usage_label.set_text(&cpu_usage_text);
                state.cpu_temp_label.set_text(&cpu_temp_text);



                state.hostname_label.mark_dirty(ctx);
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

    fn propagate_widget_changes(&mut self, _actions: &mut Vec<crate::app::AppAction>) {
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
        let mut state = SystemState::default();
        let mut layout = cce_ui::layout::ColumnLayout::new(20.0);
        let sec_focused = vec![false, false, false, false, false, false, false];
        let mut ctx = cce_ui::context::UiContext::new();
        let pc = view(&mut state, 10.0, 20.0, 800.0, 600.0, false, &sec_focused, &mut layout, &mut ctx);
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
