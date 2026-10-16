//! Systemd timers — the system's "cron". View system/user timers with their
//! schedules and how their last run went, trigger the activated service
//! immediately, enable/disable timer units (system scope through pkexec),
//! and create, edit and delete user timers.

use crate::app::{button_need, form_button, AppAction, PageCanvas};
use cce_ui::context::UiContext;
use cce_ui::widget::Handle;
use cce_ui::widget::ScrollRegion;
use cce_ui::compose::{lay_row, Cell, PageLayoutBuilder, PageFlow};
use cce_ui::scene::layout::Rect;
use cce_ui::widget::{StatusDot, DotStatus, InteractiveListItem, TextBox};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerTab {
    System,
    User,
}

#[derive(Debug, Clone)]
pub struct TimerInfo {
    pub unit: String,
    pub activates: String,
    /// Absolute epoch microseconds; None when the timer has no scheduled run.
    pub next_usec: Option<u64>,
    pub last_usec: Option<u64>,
    pub active: bool,
    /// systemd UnitFileState: enabled / disabled / static / ...
    pub file_state: String,
    pub is_system: bool,
    /// The unit file lives in ~/.config/systemd/user — safe to edit in place.
    pub editable: bool,
    /// Listed by `list-timers`: systemd has it loaded, so `next`/`last` mean
    /// something. A disabled timer nothing pulled in is not (fstrim, paccache).
    pub loaded: bool,
    /// How the activated service's last run went when it did not succeed
    /// (`exit-code`, `signal`, `timeout`, … or `failed`). A timer whose job
    /// fails every time used to read "last 3h ago" all the same.
    pub last_failure: Option<String>,
}

/// What a click asks of a timer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerAction {
    /// Start the activated service now.
    RunNow,
    Enable,
    Disable,
    /// After Create: reload and `enable --now` the new timer.
    Activate,
    /// After an edit: reload and `try-restart` the timer.
    Apply,
    /// Disable and remove a user timer (and its service, when this page wrote it).
    Delete,
}

#[derive(Debug, Clone)]
pub struct TimersState {
    pub loaded: bool,
    pub timers: Vec<TimerInfo>,
    pub active_tab: TimerTab,
    pub list: ScrollRegion,
    pub items: Vec<Handle<cce_ui::widget::Adapted<cce_ui::widget::InteractiveListItem>>>,
    pub creating: bool,
    /// Base unit name (without .timer) being edited, form shared with create.
    pub editing: Option<String>,
    pub name_box: Handle<cce_ui::widget::Adapted<TextBox>>,
    pub command_box: Handle<cce_ui::widget::Adapted<TextBox>>,
    pub schedule_box: Handle<cce_ui::widget::Adapted<TextBox>>,
    /// The last action's outcome: Ok in green, Err in red.
    pub status_msg: Option<Result<String, String>>,
    /// The edit form's Delete is waiting on its confirmation.
    pub confirm_delete: bool,
    /// Timers with an action in flight (unit, system scope). Their optimistic
    /// state is put back over each refresh, and a second action waits.
    pub pending: Vec<(TimerAction, String, bool)>,
    /// How results come back from the spawned commands (set by the host).
    pub sender: Option<calloop::channel::Sender<AppAction>>,
}

impl Default for TimersState {
    fn default() -> Self {
        Self {
            loaded: false,
            timers: Vec::new(),
            active_tab: TimerTab::System,
            // Rows sized from the list font (two lines), not a figure it outgrows.
            list: ScrollRegion::new(cce_ui::widget::InteractiveListItem::row_height(true), 4.0).with_frame(false).with_sink_behind(true),
            items: Vec::new(),
            creating: false,
            editing: None,
            name_box: Handle::none(),
            command_box: Handle::none(),
            schedule_box: Handle::none(),
            status_msg: None,
            confirm_delete: false,
            pending: Vec::new(),
            sender: None,
        }
    }
}

impl TimersState {
    /// The page's state, its widgets inserted into `ctx`.
    pub fn new(ctx: &mut UiContext) -> Self {
        Self {
            name_box: ctx.insert(TextBox::new(String::new()).with_draw_bg_border(true).with_label("Name")
                .with_placeholder("backup")),
            command_box: ctx.insert(TextBox::new(String::new()).with_draw_bg_border(true).with_label("Command")
                .with_placeholder("/home/me/bin/backup.sh --fast")),
            schedule_box: ctx.insert(TextBox::new(String::new()).with_draw_bg_border(true).with_label("Schedule (OnCalendar)")
                .with_placeholder("daily \u{2022} Mon 09:00 \u{2022} *-*-* 03:00:00")),
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone)]
pub enum TimersMessage {
    Refreshed(Vec<TimerInfo>),
    SetTab(TimerTab),
    /// An action on a timer (by its `.timer` unit name).
    Request(TimerAction, String, bool),
    Finished(TimerAction, String, bool, Result<(), String>),
    /// The edit form's Delete, and its confirmation.
    DeleteAsk,
    DeleteConfirm,
    DeleteCancel,
    CreateStart,
    CreateCancel,
    CreateSave,
    /// Open the form pre-filled from the unit files (user timers only).
    EditStart(String),
}

const TEXT_DIM: [f32; 4] = [0.53, 0.53, 0.60, 1.0];
const OK_GREEN: [f32; 4] = [0.56, 0.83, 0.56, 1.0];
/// Error text: light enough for small type on the dark plate (7.6:1; the
/// old [0.85, 0.25, 0.25] was 4.1:1, under AA for text this size).
const RED: [f32; 4] = [0.95, 0.55, 0.55, 1.0];
/// The least room the list keeps on a short window.
const LIST_MIN_H: f32 = 120.0;
/// A row's status dot: a small LED, not a control.
const DOT: f32 = 10.0;

/// "46min" / "3h" / "5d 3h" — coarse two-unit humanization.
fn humanize(secs: u64) -> String {
    let (d, h, m) = (secs / 86_400, (secs % 86_400) / 3600, (secs % 3600) / 60);
    if d > 0 {
        if h > 0 { format!("{}d {}h", d, h) } else { format!("{}d", d) }
    } else if h > 0 {
        if m > 0 { format!("{}h {}min", h, m) } else { format!("{}h", h) }
    } else if m > 0 {
        format!("{}min", m)
    } else {
        format!("{}s", secs)
    }
}

fn now_usec() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_micros() as u64)
        .unwrap_or(0)
}

fn schedule_line(t: &TimerInfo, now: u64) -> String {
    // A failed last run leads, so the row's truncation never cuts it.
    let failed = t.last_failure.as_ref().map(|r| format!("last run failed ({r}) \u{2022} ")).unwrap_or_default();
    if !t.loaded {
        return format!("{failed}{} \u{2022} not scheduled", t.activates);
    }
    let next = match t.next_usec {
        Some(n) if n > now => format!("next in {}", humanize((n - now) / 1_000_000)),
        Some(_) => "next imminent".to_string(),
        None => "no run scheduled".to_string(),
    };
    let last = match t.last_usec {
        Some(l) if l > 0 && l <= now => format!("last {} ago", humanize((now - l) / 1_000_000)),
        _ => "never ran".to_string(),
    };
    format!("{failed}{} \u{2022} {} \u{2022} {}", t.activates, next, last)
}

// ── Background fetching ──

fn systemctl(user: bool) -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new("systemctl");
    if user {
        cmd.arg("--user");
    }
    cmd
}

async fn stdout_of(mut cmd: tokio::process::Command) -> String {
    match crate::output_within(&mut cmd, crate::FETCH_TIMEOUT).await {
        Ok(o) => String::from_utf8_lossy(&o.stdout).into_owned(),
        Err(_) => String::new(),
    }
}

/// `list-timers --all` (the LOADED timers, with their next and last runs)
/// joined with `list-unit-files --type=timer` (every installed one, and
/// whether it is enabled). The first alone never listed a disabled timer
/// nothing had pulled in — fstrim and paccache among them here — so the
/// row's Enable button had nothing to appear on. Templates and aliases are
/// left out, as on the Services page.
pub fn merge_timers(json: &str, files: &str, is_system: bool) -> Vec<TimerInfo> {
    let parsed: Vec<serde_json::Value> = serde_json::from_str(json).unwrap_or_default();
    let file_states: Vec<(&str, &str)> = files
        .lines()
        .filter_map(|l| {
            let mut p = l.split_whitespace();
            Some((p.next()?, p.next()?))
        })
        .collect();
    let state_of = |unit: &str| file_states.iter().find(|(u, _)| *u == unit).map(|(_, s)| s.to_string()).unwrap_or_default();
    let mut timers: Vec<TimerInfo> = parsed
        .iter()
        .filter_map(|v| {
            let unit = v.get("unit")?.as_str()?.to_string();
            Some(TimerInfo {
                file_state: state_of(&unit),
                unit,
                activates: v.get("activates").and_then(|a| a.as_str()).unwrap_or("").to_string(),
                // 0 is how systemd says "none".
                next_usec: v.get("next").and_then(|n| n.as_u64()).filter(|n| *n > 0),
                last_usec: v.get("last").and_then(|n| n.as_u64()),
                active: false,
                is_system,
                editable: false,
                loaded: true,
                last_failure: None,
            })
        })
        .collect();
    for (unit, state) in &file_states {
        if unit.ends_with("@.timer") || *state == "alias" || timers.iter().any(|t| t.unit == *unit) {
            continue;
        }
        timers.push(TimerInfo {
            unit: unit.to_string(),
            activates: String::new(),
            next_usec: None,
            last_usec: None,
            active: false,
            file_state: state.to_string(),
            is_system,
            editable: false,
            loaded: false,
            last_failure: None,
        });
    }
    timers
}

/// `systemctl show` output for several units: one `Key=value` map per unit.
pub fn show_blocks(text: &str) -> Vec<std::collections::HashMap<String, String>> {
    text.split("\n\n")
        .map(|block| {
            block
                .lines()
                .filter_map(|l| l.split_once('=').map(|(k, v)| (k.to_string(), v.to_string())))
                .collect::<std::collections::HashMap<_, _>>()
        })
        .filter(|m| m.contains_key("Id"))
        .collect()
}

/// The activated service's last outcome when it was not a success.
pub fn service_failure(props: &std::collections::HashMap<String, String>) -> Option<String> {
    let result = props.get("Result").map(String::as_str).unwrap_or("");
    if props.get("ActiveState").map(String::as_str) == Some("failed") {
        Some(if result.is_empty() || result == "success" { "failed".to_string() } else { result.to_string() })
    } else if !result.is_empty() && result != "success" {
        Some(result.to_string())
    } else {
        None
    }
}

async fn fetch_scope(user: bool) -> Vec<TimerInfo> {
    let mut list = systemctl(user);
    list.args(["list-timers", "--all", "--output=json"]);
    // Cached, as on Services (`crate::unit_files`).
    let (json, files) = tokio::join!(stdout_of(list), crate::unit_files::list(!user, "timer"));
    let mut timers = merge_timers(&json, &files, !user);
    if timers.is_empty() {
        return timers;
    }

    // One batched `show` for active/enabled state, and the unit an
    // unloaded timer would run (`show` loads it to answer).
    let mut show = systemctl(user);
    show.arg("show").args(timers.iter().map(|t| t.unit.clone())).args(["-p", "Id,ActiveState,UnitFileState,Unit"]);
    for props in show_blocks(&stdout_of(show).await) {
        if let Some(t) = timers.iter_mut().find(|t| Some(&t.unit) == props.get("Id")) {
            t.active = props.get("ActiveState").is_some_and(|v| v == "active");
            if let Some(fs) = props.get("UnitFileState").filter(|v| !v.is_empty()) {
                t.file_state = fs.clone();
            }
            if t.activates.is_empty() {
                t.activates = props.get("Unit").cloned().unwrap_or_default();
            }
        }
    }

    // And one for how each activated service's last run went.
    let services: Vec<String> = timers.iter().map(|t| t.activates.clone()).filter(|a| !a.is_empty()).collect();
    if !services.is_empty() {
        let mut show = systemctl(user);
        show.arg("show").args(&services).args(["-p", "Id,ActiveState,Result"]);
        for props in show_blocks(&stdout_of(show).await) {
            let failure = service_failure(&props);
            for t in timers.iter_mut().filter(|t| Some(&t.activates) == props.get("Id")) {
                t.last_failure = failure.clone();
            }
        }
    }
    if user {
        if let Some(dir) = user_unit_dir() {
            for t in timers.iter_mut() {
                t.editable = dir.join(&t.unit).exists();
            }
        }
    }
    timers.sort_by_key(|a| a.unit.to_lowercase());
    timers
}

fn user_unit_dir() -> Option<std::path::PathBuf> {
    std::env::var("HOME").ok().map(|h| std::path::PathBuf::from(h).join(".config/systemd/user"))
}

pub async fn fetch_timers() -> Vec<TimerInfo> {
    let (mut all, user) = tokio::join!(fetch_scope(false), fetch_scope(true));
    all.extend(user);
    all
}

/// A TextBox's live contents: the in-progress edit buffer while focused, the
/// committed text otherwise (the recurring TextBox landmine).
fn live_text(tb: &cce_ui::widget::Adapted<TextBox>) -> String {
    if tb.editing {
        tb.edit_buffer.trim().to_string()
    } else {
        tb.text.trim().to_string()
    }
}

/// Whether a user unit of this name exists anywhere systemd would find it
/// (`~/.config`, `/usr/lib`, a drop-in path): `show` says `not-found` only
/// when there is none.
fn user_unit_exists(unit: &str) -> bool {
    if user_unit_dir().is_some_and(|d| d.join(unit).exists()) {
        return true;
    }
    std::process::Command::new("systemctl")
        .args(["--user", "show", "-p", "LoadState", "--value", "--", unit])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() != "not-found")
        .unwrap_or(true)
}

/// Write a USER timer: `<name>.service` + `<name>.timer` under
/// ~/.config/systemd/user, schedule validated by `systemd-analyze calendar`.
/// Enabling it is [`TimerAction::Activate`], run after.
///
/// `exists` is asked about both names first. Only the `.timer` used to be
/// checked, so a name like `cce-desktop` overwrote the session's own
/// `cce-desktop.service` with a one-shot command, and `pipewire` shadowed the
/// packaged pipewire.service from ~/.config.
fn create_user_timer(name: &str, command: &str, schedule: &str, exists: impl Fn(&str) -> bool) -> Result<(), String> {
    if name.is_empty() || command.is_empty() || schedule.is_empty() {
        return Err("All fields are required".into());
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err("Name: letters, digits, - and _ only".into());
    }
    // A leading `-` reads as an option to systemctl.
    if !name.starts_with(|c: char| c.is_ascii_alphanumeric()) {
        return Err("Name must start with a letter or digit".into());
    }
    for unit in [format!("{name}.timer"), format!("{name}.service")] {
        if exists(&unit) {
            return Err(format!("{unit} already exists: pick another name"));
        }
    }
    let valid = std::process::Command::new("systemd-analyze")
        .args(["calendar", schedule])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !valid {
        return Err(format!("Invalid OnCalendar expression: {}", schedule));
    }

    let home = std::env::var("HOME").map_err(|_| "HOME not set".to_string())?;
    let dir = std::path::PathBuf::from(home).join(".config/systemd/user");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let timer_path = dir.join(format!("{}.timer", name));

    let service = format!(
        "[Unit]\nDescription={name} {CREATED_MARK}\n\n[Service]\nType=oneshot\nExecStart={exec}\n",
        name = name,
        exec = exec_start_for(command),
    );
    let timer = format!(
        "[Unit]\nDescription={name} schedule\n\n[Timer]\nOnCalendar={schedule}\nPersistent=true\n\n[Install]\nWantedBy=timers.target\n",
    );
    std::fs::write(dir.join(format!("{}.service", name)), service).map_err(|e| e.to_string())?;
    std::fs::write(&timer_path, timer).map_err(|e| e.to_string())?;
    Ok(())
}

/// The Description= `create_user_timer` writes: how Delete knows a service
/// file is the page's own to remove, rather than one the timer only runs.
const CREATED_MARK: &str = "(created by cce-system-interface)";

/// The `ExecStart=` value that runs `command` through `/bin/sh -c`.
///
/// systemd parses `ExecStart` itself before any shell sees it: `%` starts a
/// specifier, `$` an environment substitution, and inside the quotes `\` is
/// a C escape. Until 2026-10-02 the command was quoted for a SHELL
/// (`'` → `'\''`) and nothing else, so `date +%F > file` wrote a unit
/// systemd refused to load ("Failed to resolve unit specifiers"), a `$HOME`
/// was expanded by systemd from its own environment, and the page still
/// said "Created and enabled". Escaped here for systemd, the command reaches
/// `sh -c` exactly as typed.
fn exec_start_for(command: &str) -> String {
    let mut out = String::from("/bin/sh -c '");
    for c in command.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\'' => out.push_str("\\'"),
            '\n' => out.push_str("\\n"),
            '%' => out.push_str("%%"),
            '$' => out.push_str("$$"),
            c => out.push(c),
        }
    }
    out.push('\'');
    out
}

/// The command an `ExecStart=` value written by `exec_start_for` runs, or
/// None for any other value (a unit this page did not write, or one written
/// before 2026-10-02 with shell-style quoting), which the edit form then
/// shows and saves raw.
fn shell_command_of(exec_start: &str) -> Option<String> {
    let body = exec_start.strip_prefix("/bin/sh -c '")?.strip_suffix('\'')?;
    let mut out = String::new();
    let mut chars = body.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next()? {
                '\\' => out.push('\\'),
                '\'' => out.push('\''),
                'n' => out.push('\n'),
                _ => return None,
            },
            '%' if chars.peek() == Some(&'%') => {
                chars.next();
                out.push('%');
            }
            '$' if chars.peek() == Some(&'$') => {
                chars.next();
                out.push('$');
            }
            // An unescaped quote, `%` or `$` is not something exec_start_for
            // writes: a shell-quoted legacy unit, or a hand-written one.
            '\'' | '%' | '$' => return None,
            c => out.push(c),
        }
    }
    Some(out)
}

/// First `Key=value` in a unit file, or None.
fn read_unit_field(path: &std::path::Path, key: &str) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    content
        .lines()
        .find_map(|l| l.trim().strip_prefix(key).and_then(|r| r.strip_prefix('=')).map(|v| v.trim().to_string()))
}

/// Replace the first `key=` line's value in-place, preserving everything else;
/// errors when the file has no such line (an unusual unit we shouldn't rewrite).
fn replace_unit_field(path: &std::path::Path, key: &str, value: &str) -> Result<(), String> {
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut replaced = false;
    let out: Vec<String> = content
        .lines()
        .map(|l| {
            if !replaced && l.trim_start().starts_with(key) && l.trim_start()[key.len()..].starts_with('=') {
                replaced = true;
                format!("{}={}", key, value)
            } else {
                l.to_string()
            }
        })
        .collect();
    if !replaced {
        return Err(format!("{} has no {}= line", path.display(), key));
    }
    std::fs::write(path, out.join("\n") + "\n").map_err(|e| e.to_string())
}

/// Edit an existing USER timer in place: swap the .service ExecStart and the
/// .timer OnCalendar lines, keeping the rest of both files untouched.
/// Reloading it is [`TimerAction::Apply`], run after.
fn update_user_timer(base: &str, command: &str, schedule: &str) -> Result<(), String> {
    if command.is_empty() || schedule.is_empty() {
        return Err("Command and schedule are required".into());
    }
    let valid = std::process::Command::new("systemd-analyze")
        .args(["calendar", schedule])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !valid {
        return Err(format!("Invalid OnCalendar expression: {}", schedule));
    }
    let dir = user_unit_dir().ok_or("HOME not set")?;
    replace_unit_field(&dir.join(format!("{}.timer", base)), "OnCalendar", schedule)?;
    let service_path = dir.join(format!("{}.service", base));
    if service_path.exists() {
        // The form showed this unit's command decoded when it was one
        // `exec_start_for` wrote (see EditStart), so it goes back encoded;
        // any other ExecStart was shown raw and is written back raw.
        let ours = read_unit_field(&service_path, "ExecStart")
            .is_some_and(|v| shell_command_of(&v).is_some());
        let value = if ours { exec_start_for(command) } else { command.to_string() };
        replace_unit_field(&service_path, "ExecStart", &value)?;
    }
    Ok(())
}

/// One systemctl call, awaited; a system timer's through pkexec (126/127
/// are pkexec's own: not authorized, or the dialog dismissed).
async fn systemctl_run(args: &[&str], is_system: bool) -> Result<(), String> {
    let mut cmd = if is_system {
        let mut c = tokio::process::Command::new("pkexec");
        c.arg("systemctl");
        c
    } else {
        systemctl(true)
    };
    let out = cmd.args(args).output().await.map_err(|e| format!("could not run systemctl: {e}"))?;
    if out.status.success() {
        return Ok(());
    }
    let text = String::from_utf8_lossy(&out.stderr).trim().to_string();
    Err(match out.status.code() {
        Some(126) | Some(127) if is_system => "authorization was cancelled or denied".to_string(),
        _ if text.is_empty() => format!("systemctl exited with {}", out.status),
        _ => text,
    })
}

/// Carry out one action. `activates` is the service a timer runs.
async fn perform(action: TimerAction, unit: &str, activates: &str, is_system: bool) -> Result<(), String> {
    match action {
        TimerAction::RunNow => {
            if activates.is_empty() {
                return Err(format!("{unit} names no unit to run"));
            }
            systemctl_run(&["start", "--", activates], is_system).await
        }
        TimerAction::Enable => systemctl_run(&["enable", "--now", "--", unit], is_system).await,
        TimerAction::Disable => systemctl_run(&["disable", "--now", "--", unit], is_system).await,
        TimerAction::Activate => {
            systemctl_run(&["daemon-reload"], false).await?;
            systemctl_run(&["enable", "--now", "--", unit], false).await
        }
        TimerAction::Apply => {
            systemctl_run(&["daemon-reload"], false).await?;
            systemctl_run(&["try-restart", "--", unit], false).await
        }
        TimerAction::Delete => {
            let dir = user_unit_dir().ok_or("HOME not set")?;
            // Stopping it first; a timer already disabled is fine.
            let _ = systemctl_run(&["disable", "--now", "--", unit], false).await;
            std::fs::remove_file(dir.join(unit)).map_err(|e| format!("removing {unit}: {e}"))?;
            let service = format!("{}.service", unit.trim_end_matches(".timer"));
            if is_page_service(&dir.join(&service)) {
                std::fs::remove_file(dir.join(&service)).map_err(|e| format!("removing {service}: {e}"))?;
            }
            systemctl_run(&["daemon-reload"], false).await
        }
    }
}

/// A service file this page wrote ([`CREATED_MARK`]): Delete removes it
/// with its timer. Any other is left alone — something else may run it.
fn is_page_service(path: &std::path::Path) -> bool {
    read_unit_field(path, "Description").is_some_and(|d| d.ends_with(CREATED_MARK))
}

impl TimersState {
    fn find_mut(&mut self, unit: &str, is_system: bool) -> Option<&mut TimerInfo> {
        self.timers.iter_mut().find(|t| t.unit == unit && t.is_system == is_system)
    }

    fn apply_optimistic(&mut self, action: TimerAction, unit: &str, is_system: bool) {
        let state = match action {
            TimerAction::Enable => "enabled",
            TimerAction::Disable => "disabled",
            _ => return,
        };
        if let Some(t) = self.find_mut(unit, is_system) {
            t.file_state = state.to_string();
        }
    }
}

/// Send `action`, unless the timer already has one in flight (a second
/// click on a system timer used to queue a second password prompt).
fn run_action(state: &mut TimersState, action: TimerAction, unit: String, is_system: bool) {
    if state.pending.iter().any(|(_, u, s)| *u == unit && *s == is_system) {
        return;
    }
    let activates = state.timers.iter().find(|t| t.unit == unit && t.is_system == is_system).map(|t| t.activates.clone()).unwrap_or_default();
    state.apply_optimistic(action, &unit, is_system);
    state.pending.push((action, unit.clone(), is_system));
    let Some(tx) = state.sender.clone() else { return };
    tokio::spawn(async move {
        let res = perform(action, &unit, &activates, is_system).await;
        let _ = tx.send(AppAction::Timers(TimersMessage::Finished(action, unit, is_system, res)));
    });
}

/// A refresh now rather than at the next 5 s tick, after an action.
fn refresh(state: &TimersState) {
    // An action may have created, enabled, disabled or deleted a timer.
    crate::unit_files::invalidate();
    let Some(tx) = state.sender.clone() else { return };
    tokio::spawn(async move {
        let _ = tx.send(AppAction::Timers(TimersMessage::Refreshed(fetch_timers().await)));
    });
}

/// What the page says when an action finishes.
fn outcome_text(action: TimerAction, unit: &str, activates: &str, res: Result<(), String>) -> Result<String, String> {
    match (action, res) {
        (TimerAction::RunNow, Ok(())) => Ok(format!("Started {activates}")),
        (TimerAction::Enable, Ok(())) => Ok(format!("Enabled {unit}")),
        (TimerAction::Disable, Ok(())) => Ok(format!("Disabled {unit}")),
        (TimerAction::Activate, Ok(())) => Ok(format!("Created and enabled {unit}")),
        (TimerAction::Apply, Ok(())) => Ok(format!("Updated {unit}")),
        (TimerAction::Delete, Ok(())) => Ok(format!("Deleted {unit}")),
        (TimerAction::Activate, Err(e)) => Err(format!("Created {unit}, but enabling it failed: {e}")),
        (TimerAction::Apply, Err(e)) => Err(format!("Saved {unit}, but reloading it failed: {e}")),
        (TimerAction::RunNow, Err(e)) => Err(format!("Couldn't start {activates}: {e}")),
        (TimerAction::Enable, Err(e)) => Err(format!("Couldn't enable {unit}: {e}")),
        (TimerAction::Disable, Err(e)) => Err(format!("Couldn't disable {unit}: {e}")),
        (TimerAction::Delete, Err(e)) => Err(format!("Couldn't delete {unit}: {e}")),
    }
}

/// A timer row's button: the `icon` glyph (tinted `label_color`, so the
/// red Disable stays red) when `compact`, else the `word`. `word` is also
/// the fallback a glyph face shows when the icon set is missing.
#[allow(clippy::too_many_arguments)]
fn button_glyph(pc: &mut PageCanvas, compact: bool, icon: &str, word: &str, x: f32, y: f32, w: f32, h: f32,
                bg: [f32; 4], hover_bg: [f32; 4], label_color: [f32; 4], action: crate::app::AppAction) {
    if compact {
        pc.button_icon_tinted(icon, word, x, y, w, h, bg, hover_bg, label_color, action);
    } else {
        pc.button(word, x, y, w, h, bg, hover_bg, label_color, action);
    }
}

pub fn view(state: &mut TimersState, cx: f32, cy: f32, cw: f32, ch: f32, _root_focused: bool, sec_focused: &[bool], layout: &mut PageFlow, ctx: &mut cce_ui::context::UiContext) -> PageCanvas {
    let mut final_pc = PageCanvas::new();
    let sec_w = 320.0f32;
    let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(1);

    builder.add_section_spanned(&mut final_pc, "", 1, sec_focused.first().copied().unwrap_or(false), |sec| {
        let sec_w = sec.cw;
        let mut form = sec.form();
        if !state.loaded {
            form.column().text("Loading systemd timers...", 12.0, TEXT_DIM);
            sec.place(form, ctx);
            return;
        }
        // The list fills the page like the services list: the well's floor lands at the
        // page's bottom.
        form.fill_height((cy + ch) - form.top() - sec.bottom_inset());

        let active_bg = [0.20, 0.40, 0.65, 0.4];
        let tab_colors = |on: bool| {
            let face = if on { active_bg } else { [0.10, 0.10, 0.16, 0.3] };
            (face, [0.20, 0.20, 0.25, 0.15], [0.90, 0.90, 0.95, 1.0])
        };
        let (label1, label2) = if sec_w < 250.0 { ("System", "User") } else { ("System Timers", "User Timers") };
        let text_on = [0.90, 0.90, 0.95, 1.0];

        let now = now_usec();
        let filtered: Vec<&TimerInfo> = state.timers.iter()
            .filter(|t| t.is_system == (state.active_tab == TimerTab::System))
            .collect();
        if state.items.len() != filtered.len() {
            for h in state.items.drain(..) {
                ctx.remove(h);
            }
            for _ in 0..filtered.len() {
                state.items.push(ctx.insert(InteractiveListItem::new("")));
            }
        }

        let active_tab = state.active_tab;
        let list = &mut state.list;
        let items = &state.items;
        let mut col = form.column();
        col.row(|r| {
            form_button(r, label1, 0.0, tab_colors(active_tab == TimerTab::System),
                AppAction::Timers(TimersMessage::SetTab(TimerTab::System)));
            form_button(r, label2, 0.0, tab_colors(active_tab == TimerTab::User),
                AppAction::Timers(TimersMessage::SetTab(TimerTab::User)));
        });

        // New Timer (user scope) — a button as wide as its label; the form unfolds below.
        let new_bg = if state.creating { active_bg } else { [0.13, 0.18, 0.14, 1.0] };
        col.row(|r| {
            form_button(r, "New Timer", button_need("New Timer"), (new_bg, [0.25, 0.30, 0.26, 1.0], text_on),
                AppAction::Timers(TimersMessage::CreateStart));
        });

        if state.creating || state.editing.is_some() {
            let field_h = cce_ui::layout::spinbox_height();
            let note = match &state.editing {
                Some(base) => format!("Editing {}.timer \u{2014} command and schedule rewrite in place.", base),
                None => "New user timer \u{2014} runs the command on the schedule.".to_string(),
            };
            col.text(note, 11.0, TEXT_DIM);
            if state.creating {
                col.widget_h(ctx, state.name_box, field_h);
            }
            col.widget_h(ctx, state.command_box, field_h);
            col.widget_h(ctx, state.schedule_box, field_h);
            let save_label = if state.editing.is_some() { "Save" } else { "Create" };
            let danger = ([0.30, 0.14, 0.14, 1.0], [0.45, 0.20, 0.20, 1.0], [0.98, 0.65, 0.65, 1.0]);
            let editing = state.editing.is_some();
            col.row(|r| {
                form_button(r, save_label, button_need(save_label), ([0.13, 0.18, 0.14, 1.0], [0.25, 0.30, 0.26, 1.0], text_on),
                    AppAction::Timers(TimersMessage::CreateSave));
                form_button(r, "Cancel", button_need("Cancel"), ([0.15, 0.15, 0.20, 1.0], [0.22, 0.22, 0.28, 1.0], text_on),
                    AppAction::Timers(TimersMessage::CreateCancel));
                r.space(0.0, true);
                if editing {
                    form_button(r, "Delete", button_need("Delete"), danger, AppAction::Timers(TimersMessage::DeleteAsk));
                }
            });
            if let (true, Some(base)) = (state.confirm_delete, state.editing.clone()) {
                let ours = user_unit_dir().is_some_and(|d| is_page_service(&d.join(format!("{base}.service"))));
                let note = if ours {
                    format!("Delete {base}.timer and the {base}.service this page made for it?")
                } else {
                    format!("Delete {base}.timer? Its service file is left in place.")
                };
                col.text(note, 12.0, [0.95, 0.75, 0.55, 1.0]);
                col.row(|r| {
                    form_button(r, "Delete", button_need("Delete"), danger, AppAction::Timers(TimersMessage::DeleteConfirm));
                    form_button(r, "Keep", button_need("Keep"), ([0.15, 0.15, 0.20, 1.0], [0.22, 0.22, 0.28, 1.0], text_on),
                        AppAction::Timers(TimersMessage::DeleteCancel));
                    r.space(0.0, true);
                });
            }
        }

        if let Some(ref msg) = state.status_msg {
            let (text, color) = match msg {
                Ok(m) => (m.clone(), OK_GREEN),
                Err(e) => (e.clone(), RED),
            };
            let wrap_w = col.form_width();
            let lines: Vec<String> = text.lines().flat_map(|l| crate::app::wrap_to_width(l, wrap_w, 12.0)).take(6).collect();
            col.lines(lines, 12.0, color);
        }

        col.fill(LIST_MIN_H, move |pc, rect, ctx| {
            let (list_box_x, list_box_y, list_box_w, list_box_h) = (rect.x, rect.y, rect.width, rect.height);
            list.set_rect(list_box_x, list_box_y, list_box_w, list_box_h);
            list.update_bounds(filtered.len(), list_box_y, list_box_h);
            list.push_prims(pc.as_mut());
            let item_h = list.item_height;

            pc.push_clip(crate::app::rect(list_box_x, list_box_y, list_box_w, list_box_h));
                for (idx, timer) in filtered.iter().enumerate() {
                    if let Some(draw_y) = list.get_item_draw_y(idx, 4.0) {
                        let is_small = sec_w < 350.0;
                        // A narrow row's buttons are cce-icons glyphs (pencil,
                        // play, stop, circle) — but only when the icon set is
                        // THERE: without it they fall back to words, and a word
                        // needs the wide button. `upload_icon` caches per
                        // (name, px), so asking every row is one hash lookup.
                        let compact = is_small && cce_ui::upload_icon("play", 32).is_some();
                        // Glyph buttons are square at the control height.
                        let btn_h = cce_ui::layout::button_height();
                        // Word buttons are as wide as their words.
                        let run_label = if is_small { "Run" } else { "Run Now" };
                        let word_w = |words: &[&str]| words.iter().map(|w| crate::app::button_need(w)).fold(0.0, f32::max);
                        let run_w = if compact { btn_h } else { word_w(&[run_label]) };
                        let en_w = if compact { btn_h } else { word_w(&["Disable", "Enable", "static"]) };
                        let edit_w = if compact { btn_h } else { word_w(&["Edit"]) };

                        // The status dot, the name and schedule, then the buttons at the
                        // row's end (Edit only for an editable unit).
                        let row = Rect { x: list_box_x, y: draw_y, width: list_box_w, height: item_h };
                        let mut layout = vec![Cell::fixed(DOT, DOT), Cell::grow(item_h)];
                        if timer.editable {
                            layout.push(Cell::fixed(edit_w, btn_h));
                        }
                        layout.push(Cell::fixed(run_w, btn_h));
                        layout.push(Cell::fixed(en_w, btn_h));
                        let cells = lay_row(row, &layout);
                        let (dot_rect, item_rect) = (cells[0], cells[1]);
                        let en_x = cells[cells.len() - 1].x;
                        let run_x = cells[cells.len() - 2].x;
                        let edit_x = if timer.editable { cells[2].x } else { run_x };
                        let btn_y = cells[cells.len() - 1].y;

                        // Title + schedule subtitle, truncated to the item's text room.
                        let text_max_w = item_rect.width - 2.0 * cce_ui::layout::CONTROL_TEXT_INSET;
                        let max_chars = ((text_max_w / 6.0) as usize).max(10);
                        // Char-safe, counted in characters (the "•" is three bytes).
                        let subtitle = cce_ui::widget::display::truncate_tail(&schedule_line(timer, now), max_chars);

                        let item = items[idx];
                        let item_btn = &mut ctx[item];
                        item_btn.title = timer.unit.clone();
                        item_btn.subtitle = Some(subtitle);
                        // Cut at its cell, so a long schedule stops short of the buttons.
                        pc.push_clip(item_rect);
                        crate::app::place_and_paint(pc, item, item_rect, ctx);
                        pc.pop_clip();

                        let dot_state = if timer.last_failure.is_some() {
                            DotStatus::Error
                        } else if timer.active {
                            DotStatus::Active
                        } else {
                            DotStatus::Inactive
                        };
                        // Drawn this frame only: out of the context once it is placed.
                        let dot = ctx.insert(StatusDot::new(dot_state));
                        crate::app::place_and_paint(pc, dot, dot_rect, ctx);
                        ctx.remove(dot);

                        let active_txt = [0.90, 0.90, 0.95, 1.0];

                        // Edit: only for user units living in ~/.config/systemd/user.
                        if timer.editable {
                            // `button_glyph` below: the glyph when compact,
                            // else (or with no icon set) the word.
                            button_glyph(
                                pc,
                                compact,
                                "pencil",
                                "Edit",
                                edit_x,
                                btn_y,
                                edit_w,
                                btn_h,
                                [0.15, 0.15, 0.20, 1.0],
                                [0.22, 0.22, 0.28, 1.0],
                                active_txt,
                                crate::app::AppAction::Timers(TimersMessage::EditStart(timer.unit.clone())),
                            );
                        }

                        // Run Now: start the activated service immediately.
                        button_glyph(
                            pc,
                            compact,
                            "play",
                            if is_small { "Run" } else { "Run Now" },
                            run_x,
                            btn_y,
                            run_w,
                            btn_h,
                            [0.16, 0.35, 0.18, 0.4],
                            [0.22, 0.45, 0.25, 0.6],
                            active_txt,
                            crate::app::AppAction::Timers(TimersMessage::Request(TimerAction::RunNow, timer.unit.clone(), timer.is_system)),
                        );

                        // Enable/Disable the timer unit; static units have no toggle.
                        match timer.file_state.as_str() {
                            "enabled" | "enabled-runtime" => {
                                button_glyph(
                                    pc,
                                    compact,
                                    "stop",
                                    "Disable",
                                    en_x,
                                    btn_y,
                                    en_w,
                                    btn_h,
                                    [0.25, 0.14, 0.14, 1.0],
                                    [0.40, 0.20, 0.20, 1.0],
                                    [0.95, 0.55, 0.55, 1.0],
                                    crate::app::AppAction::Timers(TimersMessage::Request(TimerAction::Disable, timer.unit.clone(), timer.is_system)),
                                );
                            }
                            "disabled" => {
                                // `circle`, the filled dot the narrow row
                                // always used: `play` is Run Now's beside it.
                                button_glyph(
                                    pc,
                                    compact,
                                    "circle",
                                    "Enable",
                                    en_x,
                                    btn_y,
                                    en_w,
                                    btn_h,
                                    [0.15, 0.15, 0.20, 1.0],
                                    [0.22, 0.22, 0.28, 1.0],
                                    active_txt,
                                    crate::app::AppAction::Timers(TimersMessage::Request(TimerAction::Enable, timer.unit.clone(), timer.is_system)),
                                );
                            }
                            // No toggle: say what it is instead (static, masked,
                            // generated, …) rather than calling them all static.
                            other => {
                                pc.text(other, en_x + cce_ui::layout::CONTROL_TEXT_INSET, crate::app::label_y_in(btn_y, btn_h, 11.0, None), 11.0, crate::app::rgb(TEXT_DIM));
                            }
                        }
                    }
                }
            pc.pop_clip();
            // The scrollbar's fore copy, over the rows at the raise's fade.
            list.push_scrollbar_fore(pc.as_mut());

            if filtered.is_empty() {
                let inset = cce_ui::layout::plate_padding();
                pc.text("No timers in this scope", list_box_x + inset, list_box_y + inset, 12.0, crate::app::rgb(TEXT_DIM));
            }
        });
        sec.place(form, ctx);
    });

    final_pc
}

pub fn update(state: &mut TimersState, msg: TimersMessage, ctx: &mut UiContext) {
    match msg {
        TimersMessage::Refreshed(timers) => {
            state.loaded = true;
            state.timers = timers;
            // An action still in flight keeps its optimistic state: systemd's
            // answer, not the refresh, settles it.
            for (action, unit, is_system) in state.pending.clone() {
                state.apply_optimistic(action, &unit, is_system);
            }
            for h in state.items.drain(..) {
                ctx.remove(h);
            }
        }
        TimersMessage::SetTab(tab) => {
            state.active_tab = tab;
            state.list.set_scroll_y(0.0);
            for h in state.items.drain(..) {
                ctx.remove(h);
            }
        }
        TimersMessage::Request(action, unit, is_system) => {
            state.status_msg = None;
            run_action(state, action, unit, is_system);
        }
        TimersMessage::Finished(action, unit, is_system, res) => {
            state.pending.retain(|(a, u, s)| !(*a == action && *u == unit && *s == is_system));
            let activates = state.timers.iter().find(|t| t.unit == unit && t.is_system == is_system).map(|t| t.activates.clone()).unwrap_or_default();
            state.status_msg = Some(outcome_text(action, &unit, &activates, res));
            refresh(state);
        }
        TimersMessage::DeleteAsk => state.confirm_delete = true,
        TimersMessage::DeleteCancel => state.confirm_delete = false,
        TimersMessage::DeleteConfirm => {
            if let Some(base) = state.editing.take() {
                state.confirm_delete = false;
                state.creating = false;
                run_action(state, TimerAction::Delete, format!("{base}.timer"), false);
            }
        }
        TimersMessage::CreateStart => {
            state.creating = true;
            state.editing = None;
            state.confirm_delete = false;
            state.status_msg = None;
            for tb in [state.name_box, state.command_box, state.schedule_box] {
                let tb = &mut ctx[tb];
                tb.text = String::new();
                tb.edit_buffer = String::new();
            }
        }
        TimersMessage::CreateCancel => {
            state.creating = false;
            state.editing = None;
            state.confirm_delete = false;
            state.status_msg = None;
        }
        TimersMessage::EditStart(unit) => {
            let base = unit.trim_end_matches(".timer").to_string();
            let dir = user_unit_dir();
            let schedule = dir.as_ref()
                .and_then(|d| read_unit_field(&d.join(format!("{}.timer", base)), "OnCalendar"))
                .unwrap_or_default();
            let command = dir.as_ref()
                .and_then(|d| read_unit_field(&d.join(format!("{}.service", base)), "ExecStart"))
                .map(|raw| shell_command_of(&raw).unwrap_or(raw))
                .unwrap_or_default();
            state.creating = false;
            state.editing = Some(base.clone());
            state.confirm_delete = false;
            state.status_msg = None;
            ctx[state.name_box].text = base.clone();
            ctx[state.name_box].edit_buffer = base;
            ctx[state.command_box].text = command.clone();
            ctx[state.command_box].edit_buffer = command;
            ctx[state.schedule_box].text = schedule.clone();
            ctx[state.schedule_box].edit_buffer = schedule;
        }
        TimersMessage::CreateSave => {
            let command = live_text(&ctx[state.command_box]);
            let schedule = live_text(&ctx[state.schedule_box]);
            let (written, unit, action) = if let Some(base) = state.editing.clone() {
                (update_user_timer(&base, &command, &schedule), format!("{base}.timer"), TimerAction::Apply)
            } else {
                let name = live_text(&ctx[state.name_box]);
                (create_user_timer(&name, &command, &schedule, user_unit_exists), format!("{name}.timer"), TimerAction::Activate)
            };
            match written {
                Ok(()) => {
                    state.creating = false;
                    state.editing = None;
                    state.confirm_delete = false;
                    // Said once systemd has answered (`Finished`).
                    state.status_msg = None;
                    // Show the unit where it (re)appears on the next refresh.
                    state.active_tab = TimerTab::User;
                    for h in state.items.drain(..) {
                        ctx.remove(h);
                    }
                    run_action(state, action, unit, false);
                }
                Err(e) => state.status_msg = Some(Err(e)),
            }
        }
    }
}

impl crate::pages::AppPage for TimersState {
    // Sections: [Timers] — the form boxes join the group while it is open
    // (name box only on create; edits keep the unit name fixed).
    fn section_widgets(&mut self) -> Vec<Vec<cce_ui::widget::WidgetId>> {
        if self.creating {
            vec![vec![
                self.name_box.id(),
                self.command_box.id(),
                self.schedule_box.id(),
            ]]
        } else if self.editing.is_some() {
            vec![vec![
                self.command_box.id(),
                self.schedule_box.id(),
            ]]
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
        _ctx: &mut UiContext,
    ) {
    }

    // Filtered by `get_item_draw_y`, the same predicate the view's paint loop virtualizes
    // on — a scrolled-out row keeps its last-drawn rect and would otherwise win the
    // hit-test against the row actually on screen. See the note in packages.rs.
    fn extra_dispatch_roots(&mut self) -> Vec<cce_ui::widget::WidgetId> {
        let (list, items) = (&self.list, &self.items);
        items
            .iter()
            .enumerate()
            .filter(|(idx, _)| list.get_item_draw_y(*idx, 4.0).is_some())
            .map(|(_, i)| i.id())
            .collect()
    }


    fn handle_pointer_move(
        &mut self,
        lx: f32,
        ly: f32,
        _actions: &mut Vec<crate::app::AppAction>,
        _ctx: &mut cce_ui::context::UiContext,
    ) -> bool {
        self.loaded && self.list.cursor_moved(lx, ly)
    }

    fn handle_pointer_down(&mut self, lx: f32, ly: f32, _ctx: &mut cce_ui::context::UiContext) -> bool {
        self.loaded && self.list.press(lx, ly)
    }

    fn handle_pointer_up(&mut self, _ctx: &mut cce_ui::context::UiContext) -> bool {
        self.list.release()
    }

    fn handle_mouse_wheel(&mut self, delta: &cce_ui::widget::MouseScrollDelta, lx: f32, ly: f32) -> bool {
        self.loaded && self.list.wheel(delta, lx, ly)
    }

    fn handle_key_input(&mut self, event: &cce_ui::widget::KeyEvent) -> bool {
        self.loaded && self.list.keyboard(event)
    }

    fn tick(&mut self, dt: f32) -> bool {
        self.list.tick(dt)
    }

    fn frameless_lists(&self) -> Vec<&ScrollRegion> {
        vec![&self.list]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AWKWARD: [&str; 6] = [
        "date +%F > /tmp/x",
        "echo it's $HOME",
        r"printf 'a\tb\n' | tr '\t' ,",
        "rsync -a ~/a/ /mnt/b/ && notify-send 'done 100%'",
        "echo $$ ${USER} %h %%",
        "plain-command --flag",
    ];

    #[test]
    fn a_timer_command_round_trips_through_its_exec_start() {
        for cmd in AWKWARD {
            let exec = exec_start_for(cmd);
            assert_eq!(shell_command_of(&exec).as_deref(), Some(cmd), "{exec}");
            // No bare specifier or substitution survives for systemd to act on.
            let body = exec.trim_start_matches("/bin/sh -c '");
            assert!(!body.replace("%%", "").contains('%'), "{exec}");
            assert!(!body.replace("$$", "").contains('$'), "{exec}");
        }
    }

    #[test]
    fn a_unit_this_page_did_not_write_is_edited_raw() {
        // The old shell-style quoting, and hand-written lines.
        for raw in [r"/bin/sh -c 'echo it'\''s'", "/usr/bin/backup --all", "/bin/sh -c 'echo 100%'"] {
            assert_eq!(shell_command_of(raw), None, "{raw}");
        }
    }

    /// Asks systemd itself: `cargo test -p cce-system-interface --lib
    /// timers -- --ignored`. Writes units to a temp dir only and loads none.
    #[test]
    #[ignore]
    fn systemd_accepts_every_written_unit() {
        let dir = std::env::temp_dir().join(format!("cce-timer-units-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for (i, cmd) in AWKWARD.iter().enumerate() {
            let path = dir.join(format!("t{i}.service"));
            std::fs::write(&path, format!("[Service]\nType=oneshot\nExecStart={}\n", exec_start_for(cmd))).unwrap();
            let out = std::process::Command::new("systemd-analyze")
                .args(["--user", "verify"])
                .arg(&path)
                .output()
                .unwrap();
            let err = String::from_utf8_lossy(&out.stderr);
            assert!(!err.contains("Failed to resolve") && !err.contains("fatal"), "{cmd:?}: {err}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn humanize_ranges() {
        assert_eq!(humanize(30), "30s");
        assert_eq!(humanize(46 * 60), "46min");
        assert_eq!(humanize(3 * 3600), "3h");
        assert_eq!(humanize(3 * 3600 + 20 * 60), "3h 20min");
        assert_eq!(humanize(5 * 86_400 + 3 * 3600), "5d 3h");
    }

    #[test]
    fn unit_field_read_and_replace() {
        let dir = std::env::temp_dir().join("cce-timer-edit-test");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("t.timer");
        std::fs::write(&p, "[Unit]\nDescription=x\n\n[Timer]\nOnCalendar=daily\nPersistent=true\n").unwrap();

        assert_eq!(read_unit_field(&p, "OnCalendar").as_deref(), Some("daily"));
        replace_unit_field(&p, "OnCalendar", "Mon 09:00").unwrap();
        let content = std::fs::read_to_string(&p).unwrap();
        assert!(content.contains("OnCalendar=Mon 09:00"), "{content}");
        assert!(content.contains("Persistent=true"), "rest preserved: {content}");
        assert!(replace_unit_field(&p, "Nonexistent", "x").is_err());
    }

    #[test]
    fn create_timer_validation_rejects_before_side_effects() {
        let none = |_: &str| false;
        assert!(create_user_timer("", "echo hi", "daily", none).is_err());
        assert!(create_user_timer("backup", "", "daily", none).is_err());
        assert!(create_user_timer("backup", "echo hi", "", none).is_err());
        assert!(create_user_timer("bad name!", "echo hi", "daily", none).is_err());
        assert!(create_user_timer("-backup", "echo hi", "daily", none).is_err(), "a leading - reads as an option");
    }

    /// The name is refused when EITHER unit exists, wherever systemd would
    /// find it — `cce-desktop` used to overwrite the session's own service.
    #[test]
    fn create_refuses_a_name_that_is_taken() {
        let taken = |unit: &str| unit == "cce-desktop.service" || unit == "pipewire.service";
        let e = create_user_timer("cce-desktop", "echo hi", "daily", taken).unwrap_err();
        assert!(e.contains("cce-desktop.service already exists"), "{e}");
        assert!(create_user_timer("pipewire", "echo hi", "daily", taken).is_err());
        let e = create_user_timer("x", "echo", "daily", |u: &str| u == "x.timer").unwrap_err();
        assert!(e.contains("x.timer"), "{e}");
    }

    /// Disabled timers nothing loaded are listed too; templates and aliases
    /// are not; systemd's 0 means no next run.
    #[test]
    fn unit_files_join_the_loaded_timers() {
        let json = r#"[{"next":1792124612735344,"last":1792122806936818,"unit":"a.timer","activates":"a.service"},
                       {"next":0,"last":0,"unit":"b.timer","activates":"b.service"}]"#;
        let files = "a.timer enabled enabled\nfstrim.timer disabled disabled\nsysupdate.timer alias -\nt@.timer disabled -\n";
        let list = merge_timers(json, files, true);
        let names: Vec<&str> = list.iter().map(|t| t.unit.as_str()).collect();
        assert_eq!(names, ["a.timer", "b.timer", "fstrim.timer"]);
        assert_eq!(list[0].file_state, "enabled");
        assert_eq!(list[1].next_usec, None);
        assert!(!list[2].loaded && list[2].file_state == "disabled");
    }

    #[test]
    fn a_failed_last_run_leads_the_row() {
        let props = |a: &str, r: &str| {
            [("Id", "x.service"), ("ActiveState", a), ("Result", r)].iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
        };
        assert_eq!(service_failure(&props("inactive", "success")), None);
        assert_eq!(service_failure(&props("inactive", "exit-code")).as_deref(), Some("exit-code"));
        assert_eq!(service_failure(&props("failed", "success")).as_deref(), Some("failed"));
        let blocks = show_blocks("Id=a.service\nResult=success\n\nId=b.service\nResult=signal\n");
        assert_eq!(blocks.len(), 2);
        assert_eq!(service_failure(&blocks[1]).as_deref(), Some("signal"));

        let now = 1_000_000_000_000_000u64;
        let t = TimerInfo {
            unit: "x.timer".into(),
            activates: "x.service".into(),
            next_usec: Some(now + 60_000_000),
            last_usec: Some(now - 3_600_000_000),
            active: true,
            file_state: "enabled".into(),
            is_system: false,
            editable: false,
            loaded: true,
            last_failure: Some("exit-code".into()),
        };
        assert!(schedule_line(&t, now).starts_with("last run failed (exit-code)"), "{}", schedule_line(&t, now));
        let unloaded = TimerInfo { loaded: false, last_failure: None, ..t };
        assert_eq!(schedule_line(&unloaded, now), "x.service \u{2022} not scheduled");
    }

    /// Errors are shown in red, results only once systemd answers, and an
    /// action in flight survives refreshes and blocks a second.
    #[test]
    fn actions_report_and_survive_refreshes() {
        let mut ui = UiContext::new();
        let mut st = TimersState::new(&mut ui);
        let t = |state: &str| TimerInfo {
            unit: "f.timer".into(),
            activates: "f.service".into(),
            next_usec: None,
            last_usec: None,
            active: false,
            file_state: state.into(),
            is_system: true,
            editable: false,
            loaded: false,
            last_failure: None,
        };
        update(&mut st, TimersMessage::Refreshed(vec![t("disabled")]), &mut ui);
        update(&mut st, TimersMessage::Request(TimerAction::Enable, "f.timer".into(), true), &mut ui);
        update(&mut st, TimersMessage::Refreshed(vec![t("disabled")]), &mut ui);
        assert_eq!(st.timers[0].file_state, "enabled", "the refresh does not undo the click");
        update(&mut st, TimersMessage::Request(TimerAction::Disable, "f.timer".into(), true), &mut ui);
        assert_eq!(st.pending.len(), 1, "a second action waits");
        update(&mut st, TimersMessage::Finished(TimerAction::Enable, "f.timer".into(), true, Err("authorization was cancelled or denied".into())), &mut ui);
        assert!(st.pending.is_empty());
        assert!(matches!(&st.status_msg, Some(Err(e)) if e.contains("Couldn't enable f.timer")));

        let mut layout = cce_ui::compose::PageFlow::new();
        let pc = view(&mut st, 10.0, 20.0, 800.0, 600.0, false, &[false], &mut layout, &mut ui).finish();
        let red = pc.texts().into_iter().find(|t| t.text.contains("Couldn't enable")).expect("the error is drawn");
        assert_eq!(red.color, crate::app::rgb(RED));
    }

    #[test]
    fn create_reports_only_after_enabling() {
        assert_eq!(outcome_text(TimerAction::Activate, "b.timer", "b.service", Ok(())), Ok("Created and enabled b.timer".into()));
        let e = outcome_text(TimerAction::Activate, "b.timer", "b.service", Err("Unit b.timer has a bad unit file setting.".into()));
        assert!(matches!(e, Err(m) if m.starts_with("Created b.timer, but enabling it failed")));
    }

    /// Delete takes the service only when this page wrote it.
    #[test]
    fn delete_removes_only_the_pages_own_service() {
        let dir = std::env::temp_dir().join(format!("cce-timer-delete-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let ours = dir.join("a.service");
        std::fs::write(&ours, format!("[Unit]\nDescription=a {CREATED_MARK}\n\n[Service]\nExecStart=/bin/true\n")).unwrap();
        let theirs = dir.join("b.service");
        std::fs::write(&theirs, "[Unit]\nDescription=Restic backup\n").unwrap();
        assert!(is_page_service(&ours));
        assert!(!is_page_service(&theirs));
        assert!(!is_page_service(&dir.join("missing.service")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn schedule_line_composes() {
        let now = 1_000_000_000_000_000u64;
        let t = TimerInfo {
            unit: "x.timer".into(),
            activates: "x.service".into(),
            next_usec: Some(now + 46 * 60 * 1_000_000),
            last_usec: Some(now - 16 * 3600 * 1_000_000),
            active: true,
            file_state: "enabled".into(),
            is_system: true,
            editable: false,
            loaded: true,
            last_failure: None,
        };
        let line = schedule_line(&t, now);
        assert!(line.contains("x.service"), "{line}");
        assert!(line.contains("next in 46min"), "{line}");
        assert!(line.contains("last 16h ago"), "{line}");

        let t2 = TimerInfo { next_usec: None, last_usec: None, ..t };
        let line2 = schedule_line(&t2, now);
        assert!(line2.contains("no run scheduled"), "{line2}");
        assert!(line2.contains("never ran"), "{line2}");
    }
}
