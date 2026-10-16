use crate::app::{button_need, form_button, wrap_to_width, AppAction, PageCanvas};
use crate::power_meter::{self, Meter, Mode};
use cce_ui::context::UiContext;
use cce_ui::widget::{Handle, ScrollRegion, TextBox};
use cce_ui::compose::{lay_row, Cell, PageLayoutBuilder, PageFlow};
use cce_ui::scene::layout::Rect;
use std::sync::Mutex;

#[derive(Debug, Clone, Default)]
pub struct ProcessRow {
    pub pid: String,
    /// CPU % over the last refresh interval (top's figure, from the power
    /// meter's samples). `ps`'s %cpu — the process's LIFETIME average — is
    /// only the fallback for a pid the meter has not seen twice yet.
    pub cpu: String,
    pub mem_pct: String,
    pub rss_kb: u64,
    /// What the row calls it ([`command_display`]).
    pub command: String,
    /// Estimated draw from [`power_meter`]; None until the meter has two
    /// samples of this pid, or when nothing on the host reports watts.
    pub watts: Option<f32>,
    /// Context switches per second over the last interval — the honest
    /// signal for a process that burns power while looking idle.
    pub wakeups: Option<f32>,
    pub user: String,
    /// Owned by this user and not a kernel thread: what a plain `kill` can
    /// signal, so the only rows that offer Kill.
    pub mine: bool,
    /// A kernel thread (kthreadd's child): hidden unless asked for.
    pub kernel: bool,
    /// The full command line.
    pub args: String,
    pub ppid: u32,
    pub elapsed_s: u64,
    pub threads: u32,
    /// ps's STAT (R, S, D, Z, …).
    pub state: String,
    /// `/proc/<pid>/stat` starttime: the process's identity. A signal is
    /// sent only while the pid still has it — the list is up to 3 s old,
    /// and a pid freed in between can belong to another process.
    pub start_ticks: Option<u64>,
}

/// The whole-machine side of the estimate, for the line above the list.
#[derive(Debug, Clone, PartialEq)]
pub struct PowerSummary {
    pub mode: Mode,
    pub total_w: Option<f32>,
    pub floor_w: Option<f32>,
    pub attributed_w: f32,
}

impl Default for PowerSummary {
    fn default() -> Self {
        Self { mode: Mode::Warming, total_w: None, floor_w: None, attributed_w: 0.0 }
    }
}

/// One meter for the page's lifetime: attribution is a delta between
/// consecutive fetches, so the previous snapshot has to outlive the fetch.
static METER: Mutex<Option<Meter>> = Mutex::new(None);

/// Which column orders the list. Cpu is the default (ps sorts the fetch);
/// Mem is the toggle — clicking a memory header switches to it, clicking
/// again switches back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProcSort {
    #[default]
    Cpu,
    Mem,
    Power,
    Wakeups,
}

/// The signal a kill sends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    /// Ask it to exit (SIGTERM).
    Term,
    /// Force it (SIGKILL), for a process that ignored Term.
    Kill,
}

impl Signal {
    fn name(self) -> &'static str {
        match self {
            Signal::Term => "TERM",
            Signal::Kill => "KILL",
        }
    }
}

/// A process a kill is aimed at: its pid and identity.
#[derive(Debug, Clone, PartialEq)]
pub struct Target {
    pub pid: String,
    pub command: String,
    pub start_ticks: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct ProcessesState {
    pub loaded: bool,
    pub processes: Vec<ProcessRow>,
    pub cpu_list: ScrollRegion,
    /// Pids a kill was requested for, dimmed until the next refresh. The
    /// refresh clears it: a killed process is gone from the new list, and a
    /// survivor (EPERM, ignored TERM) un-dims — an honest "didn't die".
    pub killing: std::collections::HashSet<String>,
    /// Active sort column. Survives refreshes: Refreshed re-sorts the fresh
    /// list under this key rather than resetting to the fetch order.
    pub sort: ProcSort,
    pub power: PowerSummary,
    pub search_box: Handle<cce_ui::widget::Adapted<TextBox>>,
    pub show_kernel: bool,
    /// The row whose details pane is open.
    pub selected: Option<String>,
    /// A kill waiting on its confirmation.
    pub confirm: Option<Target>,
    /// TERMs sent, and when: a process still there a while later ignored
    /// it, and is offered Force Kill.
    pub termed: Vec<(Target, std::time::Instant)>,
    pub stubborn: Option<Target>,
    /// The last kill's outcome: Ok in green, Err in red.
    pub notice: Option<Result<String, String>>,
    /// How results come back from the spawned kills (set by the host).
    pub sender: Option<calloop::channel::Sender<AppAction>>,
}

impl Default for ProcessesState {
    fn default() -> Self {
        Self {
            loaded: false,
            processes: Vec::new(),
            cpu_list: ScrollRegion::new(24.0, 2.0).with_frame(false).with_sink_behind(true),
            killing: std::collections::HashSet::new(),
            sort: ProcSort::Cpu,
            power: PowerSummary::default(),
            search_box: Handle::none(),
            show_kernel: false,
            selected: None,
            confirm: None,
            termed: Vec::new(),
            stubborn: None,
            notice: None,
            sender: None,
        }
    }
}

impl ProcessesState {
    /// The page's state, its search box inserted into `ctx`.
    pub fn new(ctx: &mut UiContext) -> Self {
        Self {
            search_box: ctx.insert(TextBox::new(String::new()).with_placeholder("Filter Processes...")),
            ..Self::default()
        }
    }

    /// Whether the search box was made (`new`); a bare `default()` state —
    /// a fetch's snapshot, a test's — has none to read.
    fn has_search(&self) -> bool {
        self.search_box.id() != Handle::<cce_ui::widget::Adapted<TextBox>>::none().id()
    }

    fn query(&self, ctx: &UiContext) -> String {
        if !self.has_search() {
            return String::new();
        }
        let tb = &ctx[self.search_box];
        if tb.editing { tb.edit_buffer.to_lowercase() } else { tb.text.to_lowercase() }
    }

    /// Indices of the rows the table shows: kernel threads only when asked
    /// for, then the search over pid, user, name and command line.
    pub fn visible(&self, ctx: &UiContext) -> Vec<usize> {
        let q = self.query(ctx);
        self.processes
            .iter()
            .enumerate()
            .filter(|(_, p)| self.show_kernel || !p.kernel)
            .filter(|(_, p)| {
                q.is_empty()
                    || p.pid == q
                    || p.user.to_lowercase().contains(&q)
                    || p.command.to_lowercase().contains(&q)
                    || p.args.to_lowercase().contains(&q)
            })
            .map(|(i, _)| i)
            .collect()
    }

    fn row(&self, pid: &str) -> Option<&ProcessRow> {
        self.processes.iter().find(|p| p.pid == pid)
    }
}

#[derive(Debug, Clone)]
pub enum ProcessesMessage {
    Refreshed(Box<ProcessesState>),
    /// The row's Kill button: confirm, then SIGTERM.
    Kill(String),
    ConfirmKill,
    CancelKill,
    /// SIGKILL a process that ignored its SIGTERM.
    ForceKill,
    Signalled(Target, Signal, Result<(), String>),
    DismissNotice,
    /// A column header click. Mem headers toggle (Mem ⇄ back to Cpu); the
    /// CPU % header always selects Cpu.
    SortBy(ProcSort),
    ToggleKernel,
    /// Open (or, with None, close) a row's details.
    Select(Option<String>),
    None,
}

/// What ending this process would end, said in the kill confirmation.
pub fn kill_warning(row: &ProcessRow, own_pid: u32) -> Option<&'static str> {
    if row.pid.parse::<u32>().ok() == Some(own_pid) {
        return Some("This is the system interface itself.");
    }
    if !row.mine {
        return None;
    }
    match row.command.as_str() {
        "cce" | "cce-fx" => Some("This is the compositor: your session ends with it."),
        "systemd" => Some("This is your user service manager: your session ends with it."),
        "dbus-broker" | "dbus-broker-launch" | "dbus-daemon" => Some("This is your session's message bus: your session ends with it."),
        _ => None,
    }
}

/// Order rows under the active key, descending. Stable, so ties keep their
/// relative fetch order. Cpu re-parses the ps figure — after a Mem spell the
/// fetch order is long gone from the Vec.
fn sort_rows(rows: &mut [ProcessRow], sort: ProcSort) {
    match sort {
        ProcSort::Cpu => rows.sort_by(|a, b| {
            let (av, bv) = (a.cpu.parse::<f32>().unwrap_or(0.0), b.cpu.parse::<f32>().unwrap_or(0.0));
            bv.partial_cmp(&av).unwrap_or(std::cmp::Ordering::Equal)
        }),
        ProcSort::Mem => rows.sort_by_key(|a| std::cmp::Reverse(a.rss_kb)),
        // Unknowns sink below every known figure, however small.
        ProcSort::Power => rows.sort_by(|a, b| {
            b.watts.unwrap_or(-1.0).partial_cmp(&a.watts.unwrap_or(-1.0)).unwrap_or(std::cmp::Ordering::Equal)
        }),
        ProcSort::Wakeups => rows.sort_by(|a, b| {
            b.wakeups.unwrap_or(-1.0).partial_cmp(&a.wakeups.unwrap_or(-1.0)).unwrap_or(std::cmp::Ordering::Equal)
        }),
    }
}

/// Watts to one decimal; a dash below 0.05 W, so hundreds of idle rows do
/// not read as fake precision, and for rows the meter has no figure for.
pub fn format_watts(w: Option<f32>) -> String {
    match w {
        Some(w) if w >= 0.05 => format!("{:.1}", w),
        _ => "\u{2014}".to_string(),
    }
}

pub fn format_wakeups(w: Option<f32>) -> String {
    match w {
        Some(w) if w >= 0.5 => format!("{:.0}", w),
        _ => "\u{2014}".to_string(),
    }
}

/// "3h 12min" / "4d 2h" / "45s".
fn format_elapsed(secs: u64) -> String {
    let (d, h, m) = (secs / 86_400, (secs % 86_400) / 3600, (secs % 3600) / 60);
    match (d, h, m) {
        (0, 0, 0) => format!("{secs}s"),
        (0, 0, m) => format!("{m}min"),
        (0, h, m) => format!("{h}h {m}min"),
        (d, h, _) => format!("{d}d {h}h"),
    }
}

/// The line above the list: what was measured, what is baseline, what the
/// column adds up to — and, when the column is dashes, why.
pub fn summary_line(p: &PowerSummary) -> String {
    let how = match p.mode {
        Mode::Warming => return "Measuring power\u{2026}".to_string(),
        Mode::Unavailable => {
            return "Per-process power needs the battery discharging or readable RAPL counters  \u{00b7}  wakeups only"
                .to_string()
        }
        Mode::Battery => "estimated from battery draw",
        Mode::Rapl => "RAPL",
    };
    let mut parts = Vec::new();
    if let Some(t) = p.total_w {
        parts.push(format!("{:.1} W total", t));
    }
    if let Some(f) = p.floor_w {
        parts.push(format!("{:.1} W baseline", f));
    }
    parts.push(format!("{:.1} W attributed to processes ({})", p.attributed_w, how));
    parts.join("  \u{00b7}  ")
}

/// nvidia-smi's draw figure, asked for only while the card is awake: the
/// query itself would wake a suspended card, costing watts to report a zero.
async fn dgpu_draw_w() -> Option<f64> {
    if !power_meter::dgpu_awake() {
        return None;
    }
    let out = tokio::process::Command::new("nvidia-smi")
        .args(["--query-gpu=power.draw", "--format=csv,noheader,nounits"])
        .output()
        .await
        .ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8_lossy(&out.stdout).lines().next()?.trim().parse().ok()
}

/// Programs that run something else: a row naming only them says nothing
/// (every Python script read "python3"), so the script is shown after them.
fn is_interpreter(name: &str) -> bool {
    const NAMES: &[&str] = &[
        "python", "python2", "python3", "node", "nodejs", "deno", "bun", "bash", "sh", "zsh", "dash", "fish",
        "perl", "ruby", "lua", "luajit", "php", "java", "uv", "uvx", "env",
    ];
    NAMES.contains(&name) || name.starts_with("python2.") || name.starts_with("python3.")
}

/// Process name from a full command line: basename of argv[0], so cce
/// binaries longer than the kernel's 15-char `comm` cap display whole
/// (`comm` showed "cce-system-inte"). Kernel threads (`[kworker/0:1]`) keep
/// their brackets. An interpreter shows what it runs: `python3 backup.py`,
/// `python3 -m http.server`, `sh -c exec`.
pub fn command_display(cmd: &str) -> String {
    let mut words = cmd.split_whitespace();
    let first = words.next().unwrap_or(cmd);
    if first.starts_with('[') {
        return first.to_string();
    }
    let base = |w: &str| std::path::Path::new(w).file_name().and_then(|n| n.to_str()).unwrap_or(w).to_string();
    let name = base(first);
    if !is_interpreter(&name) {
        return name;
    }
    while let Some(w) = words.next() {
        match w {
            "-m" | "-c" | "-e" | "-jar" => {
                return match words.next() {
                    Some(next) if w == "-m" => format!("{name} -m {next}"),
                    Some(next) => format!("{name} {w} {}", base(next)),
                    None => format!("{name} {w}"),
                };
            }
            // `env FOO=1 prog`: the assignments are not the program.
            _ if w.starts_with('-') || (name == "env" && w.contains('=')) => continue,
            _ => return format!("{name} {}", base(w)),
        }
    }
    name
}

/// Humanized RSS from ps's KiB figure.
pub fn format_rss(kb: u64) -> String {
    if kb >= 1_048_576 {
        format!("{:.1} GB", kb as f64 / 1_048_576.0)
    } else if kb >= 1024 {
        format!("{} MB", kb / 1024)
    } else {
        format!("{} KB", kb)
    }
}

/// starttime from a `/proc/<pid>/stat` line (field 22; the comm field may
/// hold spaces and parentheses, so count from the last `)`).
pub fn parse_start_ticks(stat: &str) -> Option<u64> {
    stat[stat.rfind(')')? + 2..].split_whitespace().nth(19)?.parse().ok()
}

fn start_ticks(pid: &str) -> Option<u64> {
    parse_start_ticks(&std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?)
}

/// One `ps -o pid=,ppid=,uid=,user:32=,%cpu=,%mem=,rss=,etimes=,nlwp=,stat=,args=` line.
pub fn parse_ps_line(line: &str, my_uid: u32) -> Option<ProcessRow> {
    let mut f = line.split_whitespace();
    let pid = f.next()?.to_string();
    let ppid: u32 = f.next()?.parse().ok()?;
    let uid: u32 = f.next()?.parse().ok()?;
    let user = f.next()?.to_string();
    let cpu = f.next()?.to_string();
    let mem_pct = f.next()?.to_string();
    let rss_kb = f.next()?.parse().unwrap_or(0);
    let elapsed_s = f.next()?.parse().unwrap_or(0);
    let threads = f.next()?.parse().unwrap_or(1);
    let state = f.next()?.to_string();
    let args = f.collect::<Vec<_>>().join(" ");
    let kernel = ppid == 2 || pid == "2";
    Some(ProcessRow {
        command: command_display(&args),
        mine: uid == my_uid && !kernel,
        kernel,
        pid,
        cpu,
        mem_pct,
        rss_kb,
        user,
        args,
        ppid,
        elapsed_s,
        threads,
        state,
        ..Default::default()
    })
}

fn my_uid() -> u32 {
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata("/proc/self").map(|m| m.uid()).unwrap_or(u32::MAX)
}

pub async fn fetch_processes_state() -> ProcessesState {
    let dgpu_w = dgpu_draw_w().await;
    let attribution = tokio::task::spawn_blocking(move || {
        let mut meter = METER.lock().unwrap();
        let meter = meter.get_or_insert_with(Meter::new);
        // The first fetch has nothing to diff against, and CPU % is a diff:
        // take a baseline half a second earlier rather than show lifetime
        // averages for the first three seconds.
        if !meter.has_baseline() {
            meter.tick(power_meter::sample(dgpu_w));
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
        meter.tick(power_meter::sample(dgpu_w))
    })
    .await
    .ok();
    let power = attribution
        .as_ref()
        .map(|a| PowerSummary { mode: a.mode, total_w: a.total_w, floor_w: a.floor_w, attributed_w: a.attributed_w })
        .unwrap_or_default();

    let uid = my_uid();
    let mut processes = Vec::new();
    if let Ok(o) = tokio::process::Command::new("ps")
        .args(["-eo", "pid=,ppid=,uid=,user:32=,%cpu=,%mem=,rss=,etimes=,nlwp=,stat=,args="])
        .output()
        .await
    {
        for line in String::from_utf8_lossy(&o.stdout).lines() {
            let Some(mut row) = parse_ps_line(line, uid) else { continue };
            let pp = row.pid.parse::<u32>().ok().and_then(|pid| attribution.as_ref().and_then(|a| a.per_pid.get(&pid)).copied());
            if let Some(pp) = pp {
                row.cpu = format!("{:.1}", pp.cpu_pct);
                row.watts = pp.watts;
                row.wakeups = Some(pp.wakeups_per_s);
            }
            // Identity for the rows a kill can reach.
            if row.mine {
                row.start_ticks = start_ticks(&row.pid);
            }
            processes.push(row);
        }
    }
    sort_rows(&mut processes, ProcSort::Cpu);

    ProcessesState { loaded: true, processes, power, ..Default::default() }
}

/// Send `signal` to `target`, if the pid is still the process the list
/// showed. `kill` 0 or a negative pid signals a whole group, so only a
/// positive pid is ever passed on.
async fn send_signal(target: &Target, signal: Signal) -> Result<(), String> {
    if !target.pid.parse::<u32>().is_ok_and(|n| n > 0) {
        return Err("not a process id".to_string());
    }
    match (start_ticks(&target.pid), target.start_ticks) {
        (None, _) => return Err("it had already exited".to_string()),
        (Some(now), Some(then)) if now != then => {
            return Err("it had already exited (its PID now belongs to another process)".to_string());
        }
        _ => {}
    }
    let out = tokio::process::Command::new("kill")
        .args(["-s", signal.name(), "--", &target.pid])
        .output()
        .await
        .map_err(|e| format!("could not run kill: {e}"))?;
    if out.status.success() {
        return Ok(());
    }
    let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
    Err(err.trim_start_matches("kill: ").to_string())
}

fn spawn_signal(state: &mut ProcessesState, target: Target, signal: Signal) {
    state.killing.insert(target.pid.clone());
    state.notice = None;
    let Some(tx) = state.sender.clone() else { return };
    tokio::spawn(async move {
        let res = send_signal(&target, signal).await;
        let _ = tx.send(AppAction::Processes(ProcessesMessage::Signalled(target, signal, res)));
    });
}

const TEXT_FG: [f32; 4] = [0.83, 0.83, 0.83, 1.0];
const TEXT_DIM: [f32; 4] = [0.53, 0.53, 0.60, 1.0];
const OK_GREEN: [f32; 4] = [0.56, 0.83, 0.56, 1.0];
const RED: [f32; 4] = [0.85, 0.25, 0.25, 1.0];
const WARN: [f32; 4] = [0.95, 0.75, 0.55, 1.0];
const NEUTRAL: ([f32; 4], [f32; 4], [f32; 4]) = ([0.16, 0.16, 0.24, 1.0], [0.25, 0.30, 0.26, 1.0], TEXT_FG);
const DANGER: ([f32; 4], [f32; 4], [f32; 4]) = ([0.30, 0.14, 0.14, 1.0], [0.45, 0.20, 0.20, 1.0], [0.98, 0.65, 0.65, 1.0]);
/// The least room the table keeps on a short window.
const LIST_MIN_H: f32 = 120.0;
/// The table's header and row text sizes, and the sort chevron's.
const HDR_SIZE: f32 = 11.0;
const ROW_SIZE: f32 = 12.0;
const CHEVRON: f32 = 9.0;
/// The least room COMMAND keeps, in characters, before the table scrolls sideways.
const COMMAND_MIN_CHARS: usize = 24;
/// How long a TERMed process gets to exit before it counts as ignoring it.
const TERM_GRACE: std::time::Duration = std::time::Duration::from_secs(2);

/// The table's columns in content space — PID, USER, COMMAND, MEM, MEM %, CPU %, W, WAKE/s
/// and Kill — and the content width they need. Each column is as wide as its header (and
/// the sort chevron) or its widest likely value, whichever is wider; COMMAND takes the slack
/// of a box wider than that, so a wide window shows the whole table and a narrow one scrolls
/// it. They stand `list_gap` apart and in from the edges (`lay_row`).
fn table_columns(box_w: f32, row_h: f32, kill_w: f32) -> ([Rect; 9], f32) {
    let gap = cce_ui::layout::list_gap();
    let w = |s: &str| crate::app::text_width(s, ROW_SIZE, None);
    let head = |s: &str, sortable: bool| {
        crate::app::text_width(s, HDR_SIZE, None) + if sortable { gap / 2.0 + CHEVRON } else { 0.0 }
    };
    let widths = [
        w("4194304").max(head("PID", false)),
        w("lsgalante").max(head("USER", false)),
        w(&"x".repeat(COMMAND_MIN_CHARS)),
        w("999.9 GB").max(head("MEM", true)),
        w("100.0%").max(head("MEM %", false)),
        w("1000.0%").max(head("CPU %", true)),
        w("99.9").max(head("W", true)),
        w("99999").max(head("WAKE/s", true)),
        kill_w,
    ];
    let needed = widths.iter().sum::<f32>() + gap * (widths.len() + 1) as f32;
    let content_w = needed.max(box_w);
    let cells: Vec<Cell> = widths
        .iter()
        .enumerate()
        .map(|(i, &wd)| Cell { width: wd, height: row_h, grow: i == 2 })
        .collect();
    let rects = lay_row(Rect { x: 0.0, y: 0.0, width: content_w, height: row_h }, &cells);
    (std::array::from_fn(|i| rects[i]), content_w)
}

pub fn view(state: &mut ProcessesState, cx: f32, cy: f32, cw: f32, ch: f32, root_focused: bool, sec_focused: &[bool], layout: &mut PageFlow, ctx: &mut cce_ui::context::UiContext) -> PageCanvas {
    let mut final_pc = PageCanvas::new();
    let sec_w = 320.0f32;
    let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(1);

    // ── Processes Section (label-less well) ──
    builder.add_section_spanned(&mut final_pc, "", 1, root_focused || sec_focused.first().copied().unwrap_or(false), |sec| {
        let mut form = sec.form();
        if !state.loaded {
            form.column().text("Loading processes...", 12.0, TEXT_FG);
            sec.place(form, ctx);
            return;
        }
        // The table fills the page: the well's floor lands at the page's bottom.
        form.fill_height((cy + ch) - form.top() - sec.bottom_inset());
        let wrap_w = form.width();
        let visible = state.visible(ctx);
        let n_kernel = state.processes.iter().filter(|p| p.kernel).count();
        let mut col = form.column();
        // Power summary line above the table.
        col.text(summary_line(&state.power), 11.0, TEXT_DIM);

        // ── Search, and the kernel-thread switch ──
        if state.has_search() {
            col.widget_h(ctx, state.search_box, cce_ui::layout::textbox_height());
        }
        let kernel_label = if state.show_kernel { format!("Hide Kernel Threads ({n_kernel})") } else { format!("Show Kernel Threads ({n_kernel})") };
        col.row(|r| {
            form_button(r, kernel_label.clone(), button_need(&kernel_label), NEUTRAL, AppAction::Processes(ProcessesMessage::ToggleKernel));
            r.space(0.0, true);
        });

        // ── A kill waiting on its confirmation ──
        if let Some(t) = state.confirm.clone() {
            let warning = state.row(&t.pid).and_then(|r| kill_warning(r, std::process::id()));
            col.block(|b| {
                b.lines(wrap_to_width(&format!("Kill {} ({})?", t.command, t.pid), wrap_w, 12.0), 12.0, WARN);
                if let Some(w) = warning {
                    b.lines(wrap_to_width(w, wrap_w, 11.0), 11.0, RED);
                }
            });
            col.row(|r| {
                form_button(r, "Kill", button_need("Kill"), DANGER, AppAction::Processes(ProcessesMessage::ConfirmKill));
                form_button(r, "Cancel", button_need("Cancel"), NEUTRAL, AppAction::Processes(ProcessesMessage::CancelKill));
                r.space(0.0, true);
            });
        }

        // ── A process that ignored its TERM ──
        if let Some(t) = state.stubborn.clone() {
            col.lines(wrap_to_width(&format!("{} ({}) is still running after being asked to exit.", t.command, t.pid), wrap_w, 12.0), 12.0, WARN);
            col.row(|r| {
                form_button(r, "Force Kill", button_need("Force Kill"), DANGER, AppAction::Processes(ProcessesMessage::ForceKill));
                form_button(r, "Leave It", button_need("Leave It"), NEUTRAL, AppAction::Processes(ProcessesMessage::DismissNotice));
                r.space(0.0, true);
            });
        }

        // ── The last kill's outcome ──
        if let Some(n) = state.notice.clone() {
            let (text, color) = match n {
                Ok(m) => (m, OK_GREEN),
                Err(e) => (e, RED),
            };
            col.lines(wrap_to_width(&text, wrap_w, 12.0).into_iter().take(4).collect(), 12.0, color);
        }

        // ── Details of the clicked row ──
        if let Some(p) = state.selected.as_deref().and_then(|pid| state.row(pid)).cloned() {
            let parent = state.row(&p.ppid.to_string()).map(|r| format!("{} ({})", r.command, p.ppid)).unwrap_or_else(|| p.ppid.to_string());
            col.row(|r| {
                r.text_fill(format!("{} ({})", p.command, p.pid), 14.0, [0.35, 0.65, 0.90, 1.0]);
                if p.mine {
                    form_button(r, "Kill", button_need("Kill"), DANGER, AppAction::Processes(ProcessesMessage::Kill(p.pid.clone())));
                }
                form_button(r, "Close", button_need("Close"), NEUTRAL, AppAction::Processes(ProcessesMessage::Select(None)));
            });
            let facts = format!(
                "User {}  \u{00b7}  Parent {parent}  \u{00b7}  Started {} ago  \u{00b7}  {} thread{}  \u{00b7}  State {}",
                p.user,
                format_elapsed(p.elapsed_s),
                p.threads,
                if p.threads == 1 { "" } else { "s" },
                p.state,
            );
            col.lines(wrap_to_width(&facts, wrap_w, 11.0), 11.0, TEXT_DIM);
            col.lines(wrap_to_width(&p.args, wrap_w, 11.0).into_iter().take(6).collect(), 11.0, TEXT_FG);
        }

        let cpu_list = &mut state.cpu_list;
        let rows: Vec<ProcessRow> = visible.iter().map(|&i| state.processes[i].clone()).collect();
        let killing = &state.killing;
        let sort = state.sort;
        let selected = state.selected.clone();
        col.fill(LIST_MIN_H, move |pc, r, _| {
            let (list_box_x, list_box_y, list_box_w, list_box_h) = (r.x, r.y, r.width, r.height);
            let gap = cce_ui::layout::list_gap();
            // A row, and the header, are as tall as the region virtualizes rows at.
            let row_h = cpu_list.item_height;
            let header_h = row_h;
            let kill_h = cce_ui::layout::button_height();

            // Dissolved List (Phase 6v): scroll state + frame prims are app-owned. The
            // scrollable viewport starts below the header. Columns live in CONTENT space;
            // every draw subtracts scroll_x, and a table wider than the box scrolls.
            let (cols, content_w) = table_columns(list_box_w, row_h, kill_h);
            let [c_pid, c_user, c_cmd, c_rss, c_mem, c_cpu, c_watts, c_wake, c_kill] = cols;
            cpu_list.set_rect(list_box_x, list_box_y, list_box_w, list_box_h);
            cpu_list.set_content_w(content_w);
            // No band for the horizontal scrollbar: it rides the rows'
            // centre line, crossing the vertical one, behind the plate until
            // a scroll raises it.
            let rows_h = list_box_h - header_h - gap;
            cpu_list.update_bounds(rows.len(), list_box_y + header_h, rows_h);
            cpu_list.push_prims(pc.as_mut());

            let ox = cpu_list.scroll_x;
            let at = |c: Rect| list_box_x + c.x - ox;

            // Header row is background-less (the well shows through); only the
            // divider separates it from the rows.
            pc.quad(crate::app::rect(list_box_x, list_box_y + header_h, list_box_w, 1.0), [0.18, 0.18, 0.24, 1.0]);

            // Header labels pan with the columns, clipped to the box. The
            // sortable ones (MEM, MEM %, CPU %, W, WAKE/s) are hit targets the
            // width of their column: the active key shows brighter with a
            // `chevron-down` glyph after it (the sort is always descending).
            // Both memory headers toggle the same Mem sort — one bigger
            // target, no distinction to learn.
            let hdr = |on: bool| if on { [0.78, 0.78, 0.85, 1.0] } else { TEXT_DIM };
            let hdr_y = crate::app::label_y_in(list_box_y, header_h, HDR_SIZE, None);
            pc.push_clip(crate::app::rect(list_box_x, list_box_y, list_box_w, header_h));
            let header = |pc: &mut PageCanvas, label: &str, c: Rect, on: bool, chevron: bool| {
                let x = at(c);
                pc.text(label, x, hdr_y, HDR_SIZE, crate::app::rgb(hdr(on)));
                if on && chevron {
                    let gx = x + crate::app::text_width(label, HDR_SIZE, None) + gap / 2.0;
                    let gy = list_box_y + (header_h - CHEVRON) / 2.0;
                    pc.icon("chevron-down", crate::app::rect(gx, gy, CHEVRON, CHEVRON), hdr(on));
                }
            };
            header(pc, "PID", c_pid, false, false);
            header(pc, "USER", c_user, false, false);
            header(pc, "COMMAND", c_cmd, false, false);
            header(pc, "MEM", c_rss, sort == ProcSort::Mem, true);
            header(pc, "MEM %", c_mem, sort == ProcSort::Mem, false);
            header(pc, "CPU %", c_cpu, sort == ProcSort::Cpu, true);
            header(pc, "W", c_watts, sort == ProcSort::Power, true);
            header(pc, "WAKE/s", c_wake, sort == ProcSort::Wakeups, true);
            // The hit targets, list-row style like the rows (plateless, washed on
            // hover), inside the header clip so they pan and cut with the labels.
            // They share no rect with the row buttons, so emission order is free.
            for (c, key) in [(c_rss, ProcSort::Mem), (c_mem, ProcSort::Mem), (c_cpu, ProcSort::Cpu), (c_watts, ProcSort::Power), (c_wake, ProcSort::Wakeups)] {
                pc.list_row(at(c), list_box_y, c.width, header_h, AppAction::Processes(ProcessesMessage::SortBy(key)));
            }
            pc.pop_clip();

            // Visible process rows, clipped to the rows' viewport — not on
            // into the reserve band below it, where the scroll region still
            // hands back a row starting there and its Kill button stood alone
            // under the list.
            pc.push_clip(crate::app::rect(list_box_x, list_box_y + header_h, list_box_w, rows_h));
            for (idx, p) in rows.iter().enumerate() {
                if let Some(draw_y) = cpu_list.get_item_draw_y(idx, 4.0) {
                    // The row itself, in the DE's list style: plateless,
                    // washed on hover; a click opens its details. Viewport-fixed
                    // on purpose: the hover band spans the visible row whatever
                    // the horizontal pan.
                    let wash = lay_row(Rect { x: list_box_x, y: draw_y, width: list_box_w, height: row_h }, &[Cell::grow(row_h)])[0];
                    if selected.as_deref() == Some(p.pid.as_str()) {
                        pc.quad(crate::app::rect(wash.x, wash.y, wash.width, wash.height), [0.20, 0.40, 0.65, 0.25]);
                    }
                    pc.list_row(wash.x, wash.y, wash.width, wash.height, AppAction::Processes(ProcessesMessage::Select(Some(p.pid.clone()))));

                    // A pending kill dims the row until the next refresh
                    // settles it (gone, or alive again = the kill didn't take).
                    let dim = killing.contains(&p.pid);
                    let fg = if dim { [0.45, 0.45, 0.50, 1.0] } else { [0.80, 0.80, 0.85, 1.0] };
                    let mem_fg = if dim { [0.45, 0.45, 0.50, 1.0] } else { [0.62, 0.72, 0.88, 1.0] };
                    let cpu_fg = if dim { [0.45, 0.45, 0.50, 1.0] } else { [0.56, 0.83, 0.56, 1.0] };
                    let watt_fg = if dim { [0.45, 0.45, 0.50, 1.0] } else { [0.90, 0.75, 0.45, 1.0] };

                    let ty = crate::app::label_y_in(draw_y, row_h, ROW_SIZE, None);
                    let cell = |pc: &mut PageCanvas, text: &str, c: Rect, color: [f32; 4]| {
                        let x = at(c);
                        pc.text_with(text, x, ty, ROW_SIZE, crate::app::rgb(color), None, Some([x, ty - ROW_SIZE, x + c.width, ty + 2.0 * ROW_SIZE]));
                    };
                    cell(pc, &p.pid, c_pid, fg);
                    cell(pc, &p.user, c_user, if p.mine { fg } else { TEXT_DIM });
                    cell(pc, &p.command, c_cmd, fg);
                    cell(pc, &format_rss(p.rss_kb), c_rss, mem_fg);
                    cell(pc, &format!("{}%", p.mem_pct), c_mem, mem_fg);
                    cell(pc, &format!("{}%", p.cpu), c_cpu, cpu_fg);
                    cell(pc, &format_watts(p.watts), c_watts, watt_fg);
                    cell(pc, &format_wakeups(p.wakeups), c_wake, fg);

                    // Kill button, in content space like the columns, and only
                    // on a process a plain `kill` can signal: another user's or
                    // a kernel thread's always failed, silently. Emitted AFTER
                    // the row button on purpose: overlapping page buttons all
                    // see the click and the LAST one clicked wins the dispatched
                    // action (input_handler's sweep), so Kill beats the row's
                    // Select exactly because it comes later.
                    if p.mine && !dim {
                        // The `x` glyph in the danger tint; "Kill" only if
                        // the icon set is missing.
                        pc.button_icon_tinted(
                            "x",
                            "Kill",
                            at(c_kill),
                            draw_y + (row_h - kill_h) / 2.0,
                            c_kill.width,
                            kill_h,
                            [0.0, 0.0, 0.0, 0.0],
                            [0.75, 0.30, 0.30, 0.45],
                            [0.85, 0.55, 0.55, 1.0],
                            AppAction::Processes(ProcessesMessage::Kill(p.pid.clone())),
                        );
                    }
                }
            }
            pc.pop_clip();
            // The scrollbar's fore copy, over the rows at the raise's fade.
            cpu_list.push_scrollbar_fore(pc.as_mut());

            if rows.is_empty() {
                pc.text("No processes match", list_box_x + gap, list_box_y + header_h + gap, 12.0, crate::app::rgb(TEXT_DIM));
            }
        });
        sec.place(form, ctx);
    });

    final_pc
}

pub fn update(state: &mut ProcessesState, msg: ProcessesMessage) {
    match msg {
        ProcessesMessage::Refreshed(new) => {
            state.loaded = new.loaded;
            state.processes = new.processes;
            state.power = new.power;
            // The fetch arrives cpu-ordered; a non-default sort re-applies so
            // a refresh never silently flips the list back.
            if state.sort != ProcSort::Cpu {
                sort_rows(&mut state.processes, state.sort);
            }
            // Fresh list = every pending kill has resolved one way or the
            // other; rows that survived un-dim (the kill didn't take).
            state.killing.clear();
            // A TERMed process still there, as itself, after its grace ignored it.
            let rows = &state.processes;
            let alive = |t: &Target| rows.iter().any(|p| p.pid == t.pid && p.start_ticks == t.start_ticks);
            let (due, waiting): (Vec<_>, Vec<_>) = state.termed.drain(..).partition(|(_, at)| at.elapsed() >= TERM_GRACE);
            state.termed = waiting.into_iter().filter(|(t, _)| alive(t)).collect();
            if let Some((t, _)) = due.into_iter().find(|(t, _)| alive(t)) {
                state.stubborn = Some(t);
            }
            if state.stubborn.as_ref().is_some_and(|t| !alive(t)) {
                state.stubborn = None;
            }
            if let Some(pid) = state.selected.clone() {
                if state.row(&pid).is_none() {
                    state.selected = None;
                }
            }
        }
        ProcessesMessage::SortBy(key) => {
            // Clicking the already-active header (Mem, Power, Wakeups)
            // toggles back to the Cpu default; CPU % is always a plain select.
            state.sort = if state.sort == key { ProcSort::Cpu } else { key };
            sort_rows(&mut state.processes, state.sort);
        }
        ProcessesMessage::ToggleKernel => {
            state.show_kernel = !state.show_kernel;
            state.cpu_list.set_scroll_y(0.0);
        }
        ProcessesMessage::Select(pid) => state.selected = pid,
        ProcessesMessage::Kill(pid) => {
            // Only a row this user owns; the confirmation names it, and warns
            // when the session would go with it.
            if let Some(row) = state.row(&pid).filter(|r| r.mine) {
                state.confirm = Some(Target { pid: row.pid.clone(), command: row.command.clone(), start_ticks: row.start_ticks });
                state.notice = None;
            }
        }
        ProcessesMessage::CancelKill => state.confirm = None,
        ProcessesMessage::ConfirmKill => {
            if let Some(t) = state.confirm.take() {
                spawn_signal(state, t, Signal::Term);
            }
        }
        ProcessesMessage::ForceKill => {
            if let Some(t) = state.stubborn.take() {
                spawn_signal(state, t, Signal::Kill);
            }
        }
        ProcessesMessage::Signalled(target, signal, res) => {
            state.notice = Some(match res {
                Ok(()) => {
                    if signal == Signal::Term {
                        state.termed.push((target.clone(), std::time::Instant::now()));
                        Ok(format!("Asked {} ({}) to exit", target.command, target.pid))
                    } else {
                        Ok(format!("Killed {} ({})", target.command, target.pid))
                    }
                }
                Err(e) => {
                    state.killing.remove(&target.pid);
                    Err(format!("Couldn't kill {} ({}): {e}", target.command, target.pid))
                }
            });
        }
        ProcessesMessage::DismissNotice => {
            state.notice = None;
            state.stubborn = None;
        }
        ProcessesMessage::None => {}
    }
}

impl crate::pages::AppPage for ProcessesState {
    // Sections: [Processes]
    fn section_widgets(&mut self) -> Vec<Vec<cce_ui::widget::WidgetId>> {
        // The search box is painted (and so registered) once the list has landed.
        if self.loaded && self.has_search() {
            vec![vec![self.search_box.id()]]
        } else {
            vec![Vec::new()]
        }
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
    ) -> crate::app::PageCanvas {
        view(self, cx, cy, cw, ch, root_focused, sec_focused, layout, ctx)
    }

    fn propagate_widget_changes(
        &mut self,
        _events: &[(cce_ui::widget::WidgetId, cce_ui::widget::WidgetEvent)],
        _actions: &mut Vec<crate::app::AppAction>,
        _ctx: &mut cce_ui::context::UiContext,
    ) {
    }

    fn handle_pointer_move(
        &mut self,
        lx: f32,
        ly: f32,
        _actions: &mut Vec<crate::app::AppAction>,
        _ctx: &mut cce_ui::context::UiContext,
    ) -> bool {
        self.loaded && self.cpu_list.cursor_moved(lx, ly)
    }

    fn handle_pointer_down(&mut self, lx: f32, ly: f32, _ctx: &mut cce_ui::context::UiContext) -> bool {
        self.loaded && self.cpu_list.press(lx, ly)
    }

    fn handle_pointer_up(&mut self, _ctx: &mut cce_ui::context::UiContext) -> bool {
        self.cpu_list.release()
    }

    fn handle_mouse_wheel(&mut self, delta: &cce_ui::widget::MouseScrollDelta, lx: f32, ly: f32) -> bool {
        self.loaded && self.cpu_list.wheel(delta, lx, ly)
    }

    fn handle_key_input(&mut self, event: &cce_ui::widget::KeyEvent) -> bool {
        self.loaded && self.cpu_list.keyboard(event)
    }

    fn tick(&mut self, dt: f32) -> bool {
        self.cpu_list.tick(dt)
    }

    fn frameless_lists(&self) -> Vec<&ScrollRegion> {
        vec![&self.cpu_list]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_layout_grid() {
        let mut state = ProcessesState::default();
        let mut layout = cce_ui::compose::PageFlow::new();
        let sec_focused = vec![false];
        let mut ctx = cce_ui::context::UiContext::new();
        let pc = view(&mut state, 10.0, 20.0, 800.0, 600.0, false, &sec_focused, &mut layout, &mut ctx).finish();
        assert!(!pc.fills().is_empty() || !pc.texts().is_empty());
    }

    #[test]
    fn command_display_prefers_basename_and_keeps_kernel_threads() {
        // Longer than the 15-char comm cap ps used to truncate at.
        assert_eq!(command_display("/home/x/.local/bin/cce-system-interface --flag"), "cce-system-interface");
        assert_eq!(command_display("bash"), "bash");
        assert_eq!(command_display("[kworker/0:1-events]"), "[kworker/0:1-events]");
    }

    /// An interpreter shows what it runs.
    #[test]
    fn command_display_names_the_script() {
        assert_eq!(command_display("/usr/bin/python3 -u /home/x/bin/backup.py --fast"), "python3 backup.py");
        assert_eq!(command_display("python3 -m http.server 8000"), "python3 -m http.server");
        assert_eq!(command_display("/usr/bin/node /opt/app/server.js"), "node server.js");
        assert_eq!(command_display("/bin/sh -c exec foo"), "sh -c exec");
        assert_eq!(command_display("python3.13 tool.py"), "python3.13 tool.py");
        assert_eq!(command_display("/usr/bin/env FOO=1 python3 x.py"), "env python3");
        assert_eq!(command_display("bash"), "bash");
    }

    #[test]
    fn ps_lines_parse_with_owner_and_kernel_threads() {
        let me = 1000;
        let r = parse_ps_line("  4242  1  1000 lsgalante  3.5  1.2 20480  3600  12 Sl /usr/bin/python3 /x/y.py --a", me).unwrap();
        assert_eq!((r.pid.as_str(), r.ppid, r.user.as_str(), r.threads, r.state.as_str()), ("4242", 1, "lsgalante", 12, "Sl"));
        assert_eq!(r.command, "python3 y.py");
        assert_eq!(r.args, "/usr/bin/python3 /x/y.py --a");
        assert!(r.mine && !r.kernel);
        let k = parse_ps_line("  57  2  0 root  0.0  0.0  0  9000  1 I [kworker/0:1-events]", me).unwrap();
        assert!(k.kernel && !k.mine);
        let root = parse_ps_line("  800  1  0 root  0.1  0.3  5000  9000  3 Ss /usr/lib/systemd/systemd-logind", me).unwrap();
        assert!(!root.mine && !root.kernel);
    }

    #[test]
    fn start_ticks_survive_odd_comms() {
        let stat = "4242 (a b) c) S 1 4242 4242 0 -1 4194560 100 0 0 0 5 3 0 0 20 0 1 0 987654 1000 200";
        assert_eq!(parse_start_ticks(stat), Some(987654));
    }

    #[test]
    fn format_rss_humanizes() {
        assert_eq!(format_rss(512), "512 KB");
        assert_eq!(format_rss(4096), "4 MB");
        assert_eq!(format_rss(2_200_000), "2.1 GB");
    }

    fn row(pid: &str) -> ProcessRow {
        ProcessRow {
            pid: pid.to_string(),
            cpu: "1.0".to_string(),
            mem_pct: "2.0".to_string(),
            rss_kb: 1024,
            command: "proc".to_string(),
            user: "me".to_string(),
            mine: true,
            ..Default::default()
        }
    }

    #[test]
    fn kill_buttons_emitted_after_row_buttons_and_skip_pending_and_foreign() {
        let mut state = ProcessesState { loaded: true, ..Default::default() };
        state.processes = (1..=4).map(|i| row(&i.to_string())).collect();
        state.killing.insert("2".to_string());
        state.processes[3].mine = false;
        let mut layout = cce_ui::compose::PageFlow::new();
        let sec_focused = vec![false];
        let mut ctx = cce_ui::context::UiContext::new();
        let pc = view(&mut state, 10.0, 20.0, 800.0, 600.0, false, &sec_focused, &mut layout, &mut ctx);

        let kills: Vec<usize> = pc
            .buttons
            .iter()
            .enumerate()
            .filter(|(_, b)| matches!(b.action, AppAction::Processes(ProcessesMessage::Kill(_))))
            .map(|(i, _)| i)
            .collect();
        // One Kill per row except the pending one (pid 2) and another user's (pid 4).
        assert_eq!(kills.len(), 2, "{:?}", pc.buttons.iter().map(|b| &b.action).collect::<Vec<_>>());
        // Ordering invariant the dispatch relies on: each Kill comes after
        // its row's button — the last one clicked wins, so Kill must be later.
        let rows: Vec<usize> = pc
            .buttons
            .iter()
            .enumerate()
            .filter(|(_, b)| matches!(b.action, AppAction::Processes(ProcessesMessage::Select(_))))
            .map(|(i, _)| i)
            .collect();
        assert!(kills[0] > rows[0]);
    }

    /// A kill waits on its confirmation, names what dies with it, and
    /// only ever targets this user's processes.
    #[test]
    fn kills_confirm_and_warn() {
        let mut state = ProcessesState { loaded: true, ..Default::default() };
        let mut comp = row("100");
        comp.command = "cce".into();
        let mut foreign = row("200");
        foreign.mine = false;
        state.processes = vec![comp, foreign, row("300")];
        update(&mut state, ProcessesMessage::Kill("200".into()));
        assert!(state.confirm.is_none(), "another user's process is never a target");
        update(&mut state, ProcessesMessage::Kill("100".into()));
        assert!(state.killing.is_empty(), "nothing is sent before the confirmation");
        let mut layout = cce_ui::compose::PageFlow::new();
        let mut ctx = cce_ui::context::UiContext::new();
        let pc = view(&mut state, 10.0, 20.0, 800.0, 600.0, false, &[false], &mut layout, &mut ctx).finish();
        let texts: Vec<String> = pc.texts().into_iter().map(|t| t.text).collect();
        assert!(texts.iter().any(|t| t.contains("Kill cce (100)?")), "{texts:?}");
        assert!(texts.iter().any(|t| t.contains("your session ends")), "{texts:?}");
        update(&mut state, ProcessesMessage::CancelKill);
        assert!(state.confirm.is_none());
        update(&mut state, ProcessesMessage::Kill("300".into()));
        update(&mut state, ProcessesMessage::ConfirmKill);
        assert!(state.killing.contains("300"));
        assert_eq!(kill_warning(&row("300"), 300), Some("This is the system interface itself."));
    }

    /// A failure is said, and a process that outlives its TERM's grace is
    /// offered Force Kill.
    #[test]
    fn signal_outcomes_and_force() {
        let mut state = ProcessesState { loaded: true, ..Default::default() };
        let mut p = row("300");
        p.start_ticks = Some(42);
        state.processes = vec![p.clone()];
        let t = Target { pid: "300".into(), command: "proc".into(), start_ticks: Some(42) };
        update(&mut state, ProcessesMessage::Signalled(t.clone(), Signal::Term, Err("(300): Operation not permitted".into())));
        assert!(matches!(&state.notice, Some(Err(e)) if e.contains("Operation not permitted")));
        update(&mut state, ProcessesMessage::Signalled(t.clone(), Signal::Term, Ok(())));
        assert_eq!(state.termed.len(), 1);
        // Within the grace a survivor is just slow to exit.
        update(&mut state, ProcessesMessage::Refreshed(Box::new(ProcessesState { loaded: true, processes: vec![p.clone()], ..Default::default() })));
        assert!(state.stubborn.is_none());
        state.termed[0].1 -= TERM_GRACE;
        update(&mut state, ProcessesMessage::Refreshed(Box::new(ProcessesState { loaded: true, processes: vec![p.clone()], ..Default::default() })));
        assert_eq!(state.stubborn.as_ref().map(|t| t.pid.as_str()), Some("300"));
        update(&mut state, ProcessesMessage::ForceKill);
        assert!(state.stubborn.is_none() && state.killing.contains("300"));
    }

    #[test]
    fn kernel_threads_hide_and_search_narrows() {
        let mut ctx = cce_ui::context::UiContext::new();
        let mut state = ProcessesState::new(&mut ctx);
        state.loaded = true;
        let mut k = row("2");
        k.kernel = true;
        k.mine = false;
        k.command = "[kthreadd]".into();
        let mut py = row("10");
        py.command = "python3 backup.py".into();
        py.args = "/usr/bin/python3 /home/x/backup.py".into();
        state.processes = vec![k, py, row("11")];
        assert_eq!(state.visible(&ctx), vec![1, 2], "kernel threads are hidden by default");
        update(&mut state, ProcessesMessage::ToggleKernel);
        assert_eq!(state.visible(&ctx).len(), 3);
        ctx[state.search_box].text = "backup".into();
        assert_eq!(state.visible(&ctx), vec![1]);
        ctx[state.search_box].text = "11".into();
        assert_eq!(state.visible(&ctx), vec![2], "an exact pid matches");
    }

    #[test]
    fn a_row_click_opens_its_details() {
        let mut state = ProcessesState { loaded: true, ..Default::default() };
        let mut p = row("10");
        p.args = "/usr/bin/python3 /home/x/backup.py --fast".into();
        p.ppid = 1;
        p.elapsed_s = 3 * 3600 + 12 * 60;
        state.processes = vec![p];
        update(&mut state, ProcessesMessage::Select(Some("10".into())));
        let mut layout = cce_ui::compose::PageFlow::new();
        let mut ctx = cce_ui::context::UiContext::new();
        let pc = view(&mut state, 10.0, 20.0, 800.0, 600.0, false, &[false], &mut layout, &mut ctx).finish();
        let texts: Vec<String> = pc.texts().into_iter().map(|t| t.text).collect();
        assert!(texts.iter().any(|t| t.contains("/home/x/backup.py --fast")), "{texts:?}");
        assert!(texts.iter().any(|t| t.contains("Started 3h 12min ago")), "{texts:?}");
        update(&mut state, ProcessesMessage::Refreshed(Box::new(ProcessesState { loaded: true, ..Default::default() })));
        assert!(state.selected.is_none(), "a gone process closes its details");
    }

    fn sized_row(pid: &str, cpu: &str, rss: u64) -> ProcessRow {
        ProcessRow {
            pid: pid.to_string(),
            cpu: cpu.to_string(),
            mem_pct: "0.0".to_string(),
            rss_kb: rss,
            command: "p".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn sort_toggles_between_mem_and_cpu() {
        let mut state = ProcessesState { loaded: true, ..Default::default() };
        // Fetch order = cpu descending; memory order differs deliberately.
        state.processes = vec![
            sized_row("a", "9.0", 100),
            sized_row("b", "5.0", 900),
            sized_row("c", "1.0", 500),
        ];
        let order = |s: &ProcessesState| s.processes.iter().map(|p| p.pid.clone()).collect::<Vec<_>>();

        // Mem header: sort by RSS descending.
        update(&mut state, ProcessesMessage::SortBy(ProcSort::Mem));
        assert_eq!(state.sort, ProcSort::Mem);
        assert_eq!(order(&state), ["b", "c", "a"]);

        // Same header again: back to the cpu default, re-sorted (the fetch
        // order is gone from the Vec, so this must actually parse and sort).
        update(&mut state, ProcessesMessage::SortBy(ProcSort::Mem));
        assert_eq!(state.sort, ProcSort::Cpu);
        assert_eq!(order(&state), ["a", "b", "c"]);

        // CPU % header while already Cpu: stays Cpu.
        update(&mut state, ProcessesMessage::SortBy(ProcSort::Cpu));
        assert_eq!(state.sort, ProcSort::Cpu);
    }

    #[test]
    fn refresh_preserves_active_mem_sort() {
        let mut state = ProcessesState { loaded: true, ..Default::default() };
        update(&mut state, ProcessesMessage::SortBy(ProcSort::Mem));

        let fresh = ProcessesState {
            loaded: true,
            processes: vec![sized_row("x", "9.0", 10), sized_row("y", "1.0", 999)],
            ..Default::default()
        };
        update(&mut state, ProcessesMessage::Refreshed(Box::new(fresh)));
        // The cpu-ordered fetch was re-sorted under the surviving Mem key.
        assert_eq!(state.sort, ProcSort::Mem);
        assert_eq!(state.processes[0].pid, "y");
    }

    #[test]
    fn header_sort_buttons_emitted() {
        let mut state = ProcessesState { loaded: true, ..Default::default() };
        state.processes = vec![sized_row("1", "1.0", 1)];
        let mut layout = cce_ui::compose::PageFlow::new();
        let sec_focused = vec![false];
        let mut ctx = cce_ui::context::UiContext::new();
        let pc = view(&mut state, 10.0, 20.0, 800.0, 600.0, false, &sec_focused, &mut layout, &mut ctx).finish();
        let sorts: Vec<ProcSort> = pc
            .buttons
            .iter()
            .filter_map(|b| match &b.action {
                AppAction::Processes(ProcessesMessage::SortBy(k)) => Some(*k),
                _ => None,
            })
            .collect();
        // MEM + MEM % both toggle Mem; CPU % selects Cpu; then W and WAKE/s.
        assert_eq!(sorts, [ProcSort::Mem, ProcSort::Mem, ProcSort::Cpu, ProcSort::Power, ProcSort::Wakeups]);
        // Active-sort indicator rides the CPU % header by default — as a
        // `chevron-down` glyph after the header text, never a character in it.
        let (texts, icons) = (pc.texts(), pc.icons());
        assert!(texts.iter().any(|t| t.text == "CPU %"));
        assert!(texts.iter().all(|t| !t.text.contains('\u{25bc}')));
        if cce_ui::upload_icon("chevron-down", 18).is_some() {
            let cpu = texts.iter().find(|t| t.text == "CPU %").unwrap();
            let chevrons: Vec<_> = icons.iter().filter(|i| i.y < cpu.y + 20.0 && i.y > cpu.y - 20.0).collect();
            assert_eq!(chevrons.len(), 1, "one sort chevron");
            assert!(chevrons[0].x > cpu.x, "the chevron follows the header text");
        }
    }

    #[test]
    fn columns_pan_with_horizontal_scroll() {
        let mut state = ProcessesState { loaded: true, ..Default::default() };
        state.processes = (0..3).map(|i| row(&i.to_string())).collect();
        let mut layout = cce_ui::compose::PageFlow::new();
        let sec_focused = vec![false];
        let mut ctx = cce_ui::context::UiContext::new();

        let header_x = |pc: &crate::app::PageLayers, s: &str| {
            pc.texts().iter().find(|t| t.text.starts_with(s)).map(|t| t.x).unwrap()
        };
        let pc0 = view(&mut state, 10.0, 20.0, 500.0, 400.0, false, &sec_focused, &mut layout, &mut ctx).finish();
        let x0 = header_x(&pc0, "MEM %");
        // The columns are wider than the ~500px page, so the list is
        // h-scrollable; pan within its range (the columns are as wide as
        // their content, so the range is the table's, not a fixed figure)
        // and every column shifts left by exactly that.
        assert!(state.cpu_list.h_scroll_active());
        let pan = state.cpu_list.max_scroll_x() / 2.0;
        assert!(pan > 0.0);
        state.cpu_list.scroll_x = pan;
        let pc1 = view(&mut state, 10.0, 20.0, 500.0, 400.0, false, &sec_focused, &mut layout, &mut ctx).finish();
        assert_eq!(header_x(&pc1, "MEM %"), x0 - pan);
        assert_eq!(header_x(&pc1, "CPU %"), header_x(&pc0, "CPU %") - pan);
    }

    #[test]
    fn power_sort_puts_unknowns_last_and_toggles_back() {
        let mut state = ProcessesState { loaded: true, ..Default::default() };
        let mut a = sized_row("a", "9.0", 1);
        let mut b = sized_row("b", "5.0", 1);
        let c = sized_row("c", "1.0", 1);
        a.watts = Some(0.2);
        b.watts = Some(1.5);
        state.processes = vec![a, b, c];
        let order = |s: &ProcessesState| s.processes.iter().map(|p| p.pid.clone()).collect::<Vec<_>>();
        update(&mut state, ProcessesMessage::SortBy(ProcSort::Power));
        assert_eq!(order(&state), ["b", "a", "c"]);
        update(&mut state, ProcessesMessage::SortBy(ProcSort::Power));
        assert_eq!(state.sort, ProcSort::Cpu);
        assert_eq!(order(&state), ["a", "b", "c"]);
    }

    #[test]
    fn list_lies_inside_the_section_clip() {
        // The list used to sit at `left + 12`, 2 * padding outside the
        // content box the section clips to: the summary lost its first
        // letter and every row's hover wash its left edge.
        let mut state = ProcessesState { loaded: true, ..Default::default() };
        state.processes = (0..40).map(|i| row(&i.to_string())).collect();
        let mut layout = cce_ui::compose::PageFlow::new();
        let mut ctx = cce_ui::context::UiContext::new();
        let pc = view(&mut state, 10.0, 20.0, 820.0, 640.0, false, &[false], &mut layout, &mut ctx).finish();
        for t in &pc.texts() {
            if let Some(b) = t.bounds {
                assert!(t.x >= b[0], "{:?} starts at {} left of its clip {}", t.text, t.x, b[0]);
            }
        }
        for b in &pc.buttons {
            if let Some(c) = b.clip {
                assert!(cce_ui::widget::WidgetHost::base(&b.button).x >= c.x, "{:?} starts left of its clip", b.action);
            }
        }
    }

    #[test]
    fn rows_clip_at_their_viewport_not_the_band_below() {
        // The scroll region hands back a row that starts just past its
        // viewport; clipped only at the list's bottom, that row's Kill button
        // stood alone in the reserve band under the last row.
        let mut state = ProcessesState { loaded: true, ..Default::default() };
        state.processes = (0..40).map(|i| row(&i.to_string())).collect();
        let mut layout = cce_ui::compose::PageFlow::new();
        let mut ctx = cce_ui::context::UiContext::new();
        let pc = view(&mut state, 10.0, 20.0, 820.0, 640.0, false, &[false], &mut layout, &mut ctx);
        let rows_bottom = state.cpu_list.viewport_y + state.cpu_list.viewport_h;
        for b in &pc.buttons {
            if matches!(b.action, AppAction::Processes(ProcessesMessage::Kill(_))) {
                let c = b.clip.expect("rows are clipped");
                assert_eq!(c.y + c.height, rows_bottom);
            }
        }
    }

    #[test]
    fn watts_and_wakeups_format_with_dashes_for_noise() {
        assert_eq!(format_watts(None), "\u{2014}");
        assert_eq!(format_watts(Some(0.04)), "\u{2014}");
        assert_eq!(format_watts(Some(0.05)), "0.1");
        assert_eq!(format_watts(Some(2.345)), "2.3");
        assert_eq!(format_wakeups(None), "\u{2014}");
        assert_eq!(format_wakeups(Some(0.2)), "\u{2014}");
        assert_eq!(format_wakeups(Some(12.6)), "13");
    }

    #[test]
    fn summary_line_states_mode_and_reason() {
        let p = PowerSummary { mode: Mode::Battery, total_w: Some(16.0), floor_w: Some(10.0), attributed_w: 6.0 };
        assert_eq!(
            summary_line(&p),
            "16.0 W total  \u{00b7}  10.0 W baseline  \u{00b7}  6.0 W attributed to processes (estimated from battery draw)"
        );
        assert!(summary_line(&PowerSummary::default()).starts_with("Measuring"));
        let u = PowerSummary { mode: Mode::Unavailable, ..Default::default() };
        assert!(summary_line(&u).contains("wakeups only"));
    }

    #[test]
    fn refresh_carries_the_power_summary_and_rows_show_watts() {
        let mut state = ProcessesState { loaded: true, ..Default::default() };
        let mut r = sized_row("7", "1.0", 1);
        r.watts = Some(1.26);
        r.wakeups = Some(40.0);
        let fresh = ProcessesState {
            loaded: true,
            processes: vec![r],
            power: PowerSummary { mode: Mode::Battery, total_w: Some(20.0), floor_w: Some(15.0), attributed_w: 1.26 },
            ..Default::default()
        };
        update(&mut state, ProcessesMessage::Refreshed(Box::new(fresh)));
        assert_eq!(state.power.mode, Mode::Battery);
        let mut layout = cce_ui::compose::PageFlow::new();
        let mut ctx = cce_ui::context::UiContext::new();
        let pc = view(&mut state, 10.0, 20.0, 900.0, 600.0, false, &[false], &mut layout, &mut ctx).finish();
        let texts = pc.texts();
        let all: Vec<&str> = texts.iter().map(|t| t.text.as_str()).collect();
        assert!(texts.iter().any(|t| t.text == "1.3"), "{all:?}");
        assert!(texts.iter().any(|t| t.text == "40"));
        assert!(texts.iter().any(|t| t.text.starts_with("20.0 W total")));
    }
}
