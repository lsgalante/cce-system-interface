//! Systemd timers — the system's "cron". View system/user timers with their
//! schedules, trigger the activated service immediately, and enable/disable
//! timer units (system scope through pkexec).

use crate::app::{button_need, form_button, AppAction, PageContent};
use cce_ui::context::UiContext;
use cce_ui::widget::Handle;
use cce_ui::widget::ScrollRegion;
use cce_ui::layout::{lay_row, render_widget_h, Cell, PageLayoutBuilder, PageFlow, RenderTarget};
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
    pub status_msg: Option<String>,
}

impl Default for TimersState {
    fn default() -> Self {
        Self {
            loaded: false,
            timers: Vec::new(),
            active_tab: TimerTab::System,
            list: ScrollRegion::new(36.0, 6.0).with_frame(false).with_sink_behind(true),
            items: Vec::new(),
            creating: false,
            editing: None,
            name_box: Handle::none(),
            command_box: Handle::none(),
            schedule_box: Handle::none(),
            status_msg: None,
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
    /// Start the timer's activated service right now.
    RunNow(String, bool),
    Enable(String, bool),
    Disable(String, bool),
    CreateStart,
    CreateCancel,
    CreateSave,
    /// Open the form pre-filled from the unit files (user timers only).
    EditStart(String),
}

const TEXT_DIM: [f32; 4] = [0.53, 0.53, 0.60, 1.0];
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
    let next = match t.next_usec {
        Some(n) if n > now => format!("next in {}", humanize((n - now) / 1_000_000)),
        Some(_) => "next imminent".to_string(),
        None => "no run scheduled".to_string(),
    };
    let last = match t.last_usec {
        Some(l) if l > 0 && l <= now => format!("last {} ago", humanize((now - l) / 1_000_000)),
        _ => "never ran".to_string(),
    };
    format!("{} \u{2022} {} \u{2022} {}", t.activates, next, last)
}

// ── Background fetching ──

async fn fetch_scope(user: bool) -> Vec<TimerInfo> {
    let mut args: Vec<&str> = Vec::new();
    if user {
        args.push("--user");
    }
    args.extend(["list-timers", "--all", "--output=json"]);
    let out = match tokio::process::Command::new("systemctl").args(&args).output().await {
        Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(),
        Err(_) => return Vec::new(),
    };
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&out).unwrap_or_default();
    let mut timers: Vec<TimerInfo> = parsed
        .iter()
        .filter_map(|v| {
            Some(TimerInfo {
                unit: v.get("unit")?.as_str()?.to_string(),
                activates: v.get("activates").and_then(|a| a.as_str()).unwrap_or("").to_string(),
                next_usec: v.get("next").and_then(|n| n.as_u64()),
                last_usec: v.get("last").and_then(|n| n.as_u64()),
                active: false,
                file_state: String::new(),
                is_system: !user,
                editable: false,
            })
        })
        .collect();
    if timers.is_empty() {
        return timers;
    }

    // One batched `show` for active/enabled state, blocks split by blank lines.
    let mut show_args: Vec<String> = Vec::new();
    if user {
        show_args.push("--user".into());
    }
    show_args.push("show".into());
    show_args.extend(timers.iter().map(|t| t.unit.clone()));
    show_args.extend(["-p".into(), "Id,ActiveState,UnitFileState".into()]);
    if let Ok(o) = tokio::process::Command::new("systemctl").args(&show_args).output().await {
        let text = String::from_utf8_lossy(&o.stdout).to_string();
        for block in text.split("\n\n") {
            let mut id = None;
            let mut active = false;
            let mut file_state = String::new();
            for line in block.lines() {
                if let Some((k, v)) = line.split_once('=') {
                    match k {
                        "Id" => id = Some(v.to_string()),
                        "ActiveState" => active = v == "active",
                        "UnitFileState" => file_state = v.to_string(),
                        _ => {}
                    }
                }
            }
            if let Some(id) = id {
                if let Some(t) = timers.iter_mut().find(|t| t.unit == id) {
                    t.active = active;
                    t.file_state = file_state;
                }
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
    timers.sort_by(|a, b| a.unit.to_lowercase().cmp(&b.unit.to_lowercase()));
    timers
}

fn user_unit_dir() -> Option<std::path::PathBuf> {
    std::env::var("HOME").ok().map(|h| std::path::PathBuf::from(h).join(".config/systemd/user"))
}

pub async fn fetch_timers() -> Vec<TimerInfo> {
    let mut all = fetch_scope(false).await;
    all.extend(fetch_scope(true).await);
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

/// Create and enable a USER timer: `<name>.service` + `<name>.timer` under
/// ~/.config/systemd/user, schedule validated by `systemd-analyze calendar`.
fn create_user_timer(name: &str, command: &str, schedule: &str) -> Result<String, String> {
    if name.is_empty() || command.is_empty() || schedule.is_empty() {
        return Err("All fields are required".into());
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err("Name: letters, digits, - and _ only".into());
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
    if timer_path.exists() {
        return Err(format!("{}.timer already exists", name));
    }

    let service = format!(
        "[Unit]\nDescription={name} (created by cce-system-interface)\n\n[Service]\nType=oneshot\nExecStart={exec}\n",
        name = name,
        exec = exec_start_for(command),
    );
    let timer = format!(
        "[Unit]\nDescription={name} schedule\n\n[Timer]\nOnCalendar={schedule}\nPersistent=true\n\n[Install]\nWantedBy=timers.target\n",
    );
    std::fs::write(dir.join(format!("{}.service", name)), service).map_err(|e| e.to_string())?;
    std::fs::write(&timer_path, timer).map_err(|e| e.to_string())?;

    let _ = tokio::process::Command::new("sh")
        .args(["-c", &format!("systemctl --user daemon-reload && systemctl --user enable --now {}.timer", name)])
        .spawn();
    Ok(format!("Created and enabled {}.timer", name))
}

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
fn update_user_timer(base: &str, command: &str, schedule: &str) -> Result<String, String> {
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
    let _ = tokio::process::Command::new("sh")
        .args(["-c", &format!("systemctl --user daemon-reload && systemctl --user try-restart {}.timer", base)])
        .spawn();
    Ok(format!("Updated {}.timer", base))
}

fn systemctl_action(args: &[&str], is_system: bool) {
    if is_system {
        let mut full = vec!["systemctl"];
        full.extend(args);
        let _ = tokio::process::Command::new("pkexec").args(&full).spawn();
    } else {
        let mut full = vec!["--user"];
        full.extend(args);
        let _ = tokio::process::Command::new("systemctl").args(&full).spawn();
    }
}

/// A timer row's button: the `icon` glyph (tinted `label_color`, so the
/// red Disable stays red) when `compact`, else the `word`. `word` is also
/// the fallback a glyph face shows when the icon set is missing.
#[allow(clippy::too_many_arguments)]
fn button_glyph(pc: &mut PageContent, compact: bool, icon: &str, word: &str, x: f32, y: f32, w: f32, h: f32,
                bg: [f32; 4], hover_bg: [f32; 4], label_color: [f32; 4], action: crate::app::AppAction) {
    if compact {
        pc.button_icon_tinted(icon, word, x, y, w, h, bg, hover_bg, label_color, action);
    } else {
        pc.button(word, x, y, w, h, bg, hover_bg, label_color, action);
    }
}

pub fn view(state: &mut TimersState, cx: f32, cy: f32, cw: f32, ch: f32, _root_focused: bool, sec_focused: &[bool], layout: &mut PageFlow, ctx: &mut cce_ui::context::UiContext) -> PageContent {
    let mut final_pc = PageContent::new();
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
            col.row(|r| {
                form_button(r, save_label, button_need(save_label), ([0.13, 0.18, 0.14, 1.0], [0.25, 0.30, 0.26, 1.0], text_on),
                    AppAction::Timers(TimersMessage::CreateSave));
                form_button(r, "Cancel", button_need("Cancel"), ([0.15, 0.15, 0.20, 1.0], [0.22, 0.22, 0.28, 1.0], text_on),
                    AppAction::Timers(TimersMessage::CreateCancel));
            });
        }

        if let Some(ref msg) = state.status_msg {
            col.text(msg.clone(), 12.0, [0.56, 0.83, 0.56, 1.0]);
        }

        col.fill(LIST_MIN_H, move |pc, rect, ctx| {
            let (list_box_x, list_box_y, list_box_w, list_box_h) = (rect.x, rect.y, rect.width, rect.height);
            list.set_rect(list_box_x, list_box_y, list_box_w, list_box_h);
            list.update_bounds(filtered.len(), list_box_y, list_box_h);
            list.push_prims(pc);
            let item_h = list.item_height;

            pc.push_clip_rect(list_box_x, list_box_y, list_box_w, list_box_h);
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
                        let subtitle_full = schedule_line(timer, now);
                        let subtitle = if subtitle_full.len() > max_chars {
                            format!("{}...", &subtitle_full[..subtitle_full.char_indices().take(max_chars.saturating_sub(3)).last().map(|(i, c)| i + c.len_utf8()).unwrap_or(0)])
                        } else {
                            subtitle_full
                        };

                        let item = items[idx];
                        let item_btn = &mut ctx[item];
                        item_btn.title = timer.unit.clone();
                        item_btn.subtitle = Some(subtitle);
                        // Cut at its cell, so a long schedule stops short of the buttons.
                        pc.push_clip_rect(item_rect.x, item_rect.y, item_rect.width, item_rect.height);
                        render_widget_h(pc, item, item_rect.x, item_rect.y, item_rect.width, item_rect.height, ctx);
                        pc.pop_clip_rect();

                        let dot_state = if timer.active { DotStatus::Active } else { DotStatus::Inactive };
                        // Drawn this frame only: out of the context once it is placed.
                        let dot = ctx.insert(StatusDot::new(dot_state));
                        render_widget_h(pc, dot, dot_rect.x, dot_rect.y, dot_rect.width, dot_rect.height, ctx);
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
                            crate::app::AppAction::Timers(TimersMessage::RunNow(timer.activates.clone(), timer.is_system)),
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
                                    crate::app::AppAction::Timers(TimersMessage::Disable(timer.unit.clone(), timer.is_system)),
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
                                    crate::app::AppAction::Timers(TimersMessage::Enable(timer.unit.clone(), timer.is_system)),
                                );
                            }
                            _ => {
                                pc.text("static", en_x + cce_ui::layout::CONTROL_TEXT_INSET, crate::app::label_y_in(btn_y, btn_h, 11.0, None), 11.0, TEXT_DIM);
                            }
                        }
                    }
                }
            pc.pop_clip_rect();
            // The scrollbar's fore copy, over the rows at the raise's fade.
            list.push_scrollbar_fore(pc);

            if filtered.is_empty() {
                let inset = cce_ui::layout::plate_padding();
                pc.text("No timers in this scope", list_box_x + inset, list_box_y + inset, 12.0, TEXT_DIM);
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
        TimersMessage::RunNow(service, is_system) => {
            if !service.is_empty() {
                systemctl_action(&["start", &service], is_system);
            }
        }
        TimersMessage::Enable(unit, is_system) => {
            if let Some(t) = state.timers.iter_mut().find(|t| t.unit == unit && t.is_system == is_system) {
                t.file_state = "enabled".to_string();
            }
            systemctl_action(&["enable", "--now", &unit], is_system);
        }
        TimersMessage::Disable(unit, is_system) => {
            if let Some(t) = state.timers.iter_mut().find(|t| t.unit == unit && t.is_system == is_system) {
                t.file_state = "disabled".to_string();
            }
            systemctl_action(&["disable", "--now", &unit], is_system);
        }
        TimersMessage::CreateStart => {
            state.creating = true;
            state.editing = None;
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
            let result = if let Some(base) = state.editing.clone() {
                update_user_timer(&base, &command, &schedule)
            } else {
                create_user_timer(&live_text(&ctx[state.name_box]), &command, &schedule)
            };
            match result {
                Ok(msg) => {
                    state.creating = false;
                    state.editing = None;
                    state.status_msg = Some(msg);
                    // Show the unit where it (re)appears on the next refresh.
                    state.active_tab = TimerTab::User;
                    for h in state.items.drain(..) {
                        ctx.remove(h);
                    }
                }
                Err(e) => {
                    state.status_msg = Some(e);
                }
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
    ) -> crate::app::PageContent {
        view(self, cx, cy, cw, ch, root_focused, sec_focused, layout, ctx)
    }

    fn propagate_widget_changes(&mut self, _actions: &mut Vec<crate::app::AppAction>, _ctx: &mut UiContext) {}

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
        assert!(create_user_timer("", "echo hi", "daily").is_err());
        assert!(create_user_timer("backup", "", "daily").is_err());
        assert!(create_user_timer("backup", "echo hi", "").is_err());
        assert!(create_user_timer("bad name!", "echo hi", "daily").is_err());
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
