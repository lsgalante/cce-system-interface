//! Systemd timers — the system's "cron". View system/user timers with their
//! schedules, trigger the activated service immediately, and enable/disable
//! timer units (system scope through pkexec).

use crate::app::{PageContent, SectionContextExt};
use crate::scroll_region::ScrollRegion;
use cce_ui::layout::{render_widget, PageLayoutBuilder, LayoutStrategy, RenderTarget};
use cce_ui::widget::{StatusDot, DotStatus, InteractiveListItem, TextBox, WidgetHost};

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
    pub items: Vec<cce_ui::widget::Adapted<cce_ui::widget::InteractiveListItem>>,
    pub creating: bool,
    /// Base unit name (without .timer) being edited, form shared with create.
    pub editing: Option<String>,
    pub name_box: cce_ui::widget::Adapted<TextBox>,
    pub command_box: cce_ui::widget::Adapted<TextBox>,
    pub schedule_box: cce_ui::widget::Adapted<TextBox>,
    pub status_msg: Option<String>,
}

impl Default for TimersState {
    fn default() -> Self {
        Self {
            loaded: false,
            timers: Vec::new(),
            active_tab: TimerTab::System,
            list: ScrollRegion::new(36.0, 6.0).with_frame(false),
            items: Vec::new(),
            creating: false,
            editing: None,
            name_box: TextBox::new(String::new()).with_draw_bg_border(true).with_label("Name")
                .with_placeholder("backup"),
            command_box: TextBox::new(String::new()).with_draw_bg_border(true).with_label("Command")
                .with_placeholder("/home/me/bin/backup.sh --fast"),
            schedule_box: TextBox::new(String::new()).with_draw_bg_border(true).with_label("Schedule (OnCalendar)")
                .with_placeholder("daily \u{2022} Mon 09:00 \u{2022} *-*-* 03:00:00"),
            status_msg: None,
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
        "[Unit]\nDescription={name} (created by cce-system-interface)\n\n[Service]\nType=oneshot\nExecStart=/bin/sh -c '{command}'\n",
        name = name,
        command = command.replace('\'', "'\\''"),
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
        replace_unit_field(&service_path, "ExecStart", command)?;
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

pub fn view(state: &mut TimersState, cx: f32, cy: f32, cw: f32, ch: f32, _root_focused: bool, sec_focused: &[bool], layout: &mut dyn LayoutStrategy, ctx: &mut cce_ui::context::UiContext) -> PageContent {
    let mut final_pc = PageContent::new();
    let sec_w = 320.0f32;
    let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(1);

    builder.add_section_spanned(&mut final_pc, "", 1, sec_focused.first().copied().unwrap_or(false), |sec| {
        let sec_w = sec.cw;
        if !state.loaded {
            sec.text("Loading systemd timers...", 12.0, 0.0, 12.0, TEXT_DIM);
        } else {
            // Tab header buttons: System Timers, User Timers
            let mut stack = sec.vstack(8.0);
            let tab_h = 28.0;
            let active_bg = [0.20, 0.40, 0.65, 0.4];
            let inactive_bg = [0.10, 0.10, 0.16, 0.3];
            let hover_bg = [0.20, 0.20, 0.25, 0.15];

            let label1 = if stack.context.cw < 250.0 { "System" } else { "System Timers" };
            let label2 = if stack.context.cw < 250.0 { "User" } else { "User Timers" };

            stack.add_row(2, 8.0, tab_h, |ctx, i, x, w| {
                if i == 0 {
                    ctx.button(
                        label1,
                        x,
                        ctx.ay(),
                        w,
                        tab_h,
                        if state.active_tab == TimerTab::System { active_bg } else { inactive_bg },
                        hover_bg,
                        [0.90, 0.90, 0.95, 1.0],
                        crate::app::AppAction::Timers(TimersMessage::SetTab(TimerTab::System)),
                    );
                } else {
                    ctx.button(
                        label2,
                        x,
                        ctx.ay(),
                        w,
                        tab_h,
                        if state.active_tab == TimerTab::User { active_bg } else { inactive_bg },
                        hover_bg,
                        [0.90, 0.90, 0.95, 1.0],
                        crate::app::AppAction::Timers(TimersMessage::SetTab(TimerTab::User)),
                    );
                }
            });

            stack.context.spacing(4.0);

            // New Timer (user scope) — compact button; the form unfolds below.
            let new_bg = if state.creating { active_bg } else { [0.13, 0.18, 0.14, 1.0] };
            stack.add_row(3, 8.0, 26.0, |c, i, x, w| {
                if i == 0 {
                    c.button("New Timer", x, c.ay(), w, 26.0,
                        new_bg, [0.25, 0.30, 0.26, 1.0], [0.90, 0.90, 0.95, 1.0],
                        crate::app::AppAction::Timers(TimersMessage::CreateStart));
                }
            });

            if state.creating || state.editing.is_some() {
                let item_w = sec_w - 2.0 * (stack.context.padding() + 12.0);
                let rx = stack.context.left;
                let widget_h = cce_ui::layout::spinbox_height();

                if let Some(base) = &state.editing {
                    stack.context.text(&format!("Editing {}.timer \u{2014} command and schedule rewrite in place.", base), 12.0, 0.0, 11.0, TEXT_DIM);
                } else {
                    stack.context.text("New user timer \u{2014} runs the command on the schedule.", 12.0, 0.0, 11.0, TEXT_DIM);
                }

                if state.creating {
                    state.name_box.set_row_rect(rx + 12.0, item_w);
                    stack.add_widget(&mut state.name_box, item_w, widget_h, ctx);
                }
                state.command_box.set_row_rect(rx + 12.0, item_w);
                stack.add_widget(&mut state.command_box, item_w, widget_h, ctx);
                state.schedule_box.set_row_rect(rx + 12.0, item_w);
                stack.add_widget(&mut state.schedule_box, item_w, widget_h, ctx);

                stack.context.spacing(4.0);
                let save_label = if state.editing.is_some() { "Save" } else { "Create" };
                stack.add_row(3, 8.0, 26.0, |c, i, x, w| {
                    match i {
                        0 => c.button(save_label, x, c.ay(), w, 26.0,
                            [0.13, 0.18, 0.14, 1.0], [0.25, 0.30, 0.26, 1.0], [0.90, 0.90, 0.95, 1.0],
                            crate::app::AppAction::Timers(TimersMessage::CreateSave)),
                        1 => c.button("Cancel", x, c.ay(), w, 26.0,
                            [0.15, 0.15, 0.20, 1.0], [0.22, 0.22, 0.28, 1.0], [0.90, 0.90, 0.95, 1.0],
                            crate::app::AppAction::Timers(TimersMessage::CreateCancel)),
                        _ => {}
                    }
                });
            }

            if let Some(ref msg) = state.status_msg {
                stack.context.text(msg, 12.0, 0.0, 12.0, [0.56, 0.83, 0.56, 1.0]);
            }

            stack.context.spacing(8.0);

            // Scroll box list, filling the page like the services list.
            let list_box_x = sec.left + 12.0;
            let list_box_y = sec.ay();
            let list_box_w = sec_w - 24.0;
            let list_box_h = ((cy + ch) - 12.0 - list_box_y).max(120.0);

            let now = now_usec();
            let filtered: Vec<&TimerInfo> = state.timers.iter()
                .filter(|t| t.is_system == (state.active_tab == TimerTab::System))
                .collect();

            state.list.set_rect(list_box_x, list_box_y, list_box_w, list_box_h);
            state.list.update_bounds(filtered.len(), list_box_y, list_box_h);
            state.list.push_prims(sec.pc);

            let item_h = state.list.item_height;

            if state.items.len() != filtered.len() {
                state.items.clear();
                for _ in 0..filtered.len() {
                    state.items.push(InteractiveListItem::new(""));
                }
            }

            sec.pc.push_clip_rect(list_box_x, list_box_y, list_box_w, list_box_h);
            for (idx, timer) in filtered.iter().enumerate() {
                if let Some(draw_y) = state.list.get_item_draw_y(idx, 4.0) {
                    let is_small = sec_w < 350.0;
                    let run_w = if is_small { 40.0 } else { 76.0 };
                    let en_w = if is_small { 40.0 } else { 66.0 };
                    let edit_w = if is_small { 36.0 } else { 50.0 };
                    let btn_gap = if is_small { 4.0 } else { 6.0 };
                    let right_edge = list_box_x + list_box_w - 24.0 - 8.0;

                    let en_x = right_edge - en_w;
                    let run_x = en_x - btn_gap - run_w;
                    let edit_x = run_x - btn_gap - edit_w;

                    let btn_y = draw_y + (item_h - 22.0) / 2.0;
                    let btn_h = 22.0;

                    // Title + schedule subtitle (truncated to the space before the buttons).
                    let text_left_edge = if timer.editable { edit_x } else { run_x };
                    let text_max_w = (text_left_edge - 8.0) - (list_box_x + 32.0);
                    let max_chars = ((text_max_w / 6.0) as usize).max(10);
                    let subtitle_full = schedule_line(timer, now);
                    let subtitle = if subtitle_full.len() > max_chars {
                        format!("{}...", &subtitle_full[..subtitle_full.char_indices().take(max_chars.saturating_sub(3)).last().map(|(i, c)| i + c.len_utf8()).unwrap_or(0)])
                    } else {
                        subtitle_full
                    };

                    let item_btn = &mut state.items[idx];
                    item_btn.title = timer.unit.clone();
                    item_btn.subtitle = Some(subtitle);
                    render_widget(sec.pc, item_btn, list_box_x + 24.0, draw_y, list_box_w - 44.0, item_h, ctx);

                    let dot_state = if timer.active { DotStatus::Active } else { DotStatus::Inactive };
                    let mut dot = StatusDot::new(dot_state);
                    render_widget(sec.pc, &mut dot, list_box_x + 10.0, draw_y + (item_h - 10.0) / 2.0, 10.0, 10.0, ctx);

                    let active_txt = [0.90, 0.90, 0.95, 1.0];

                    // Edit: only for user units living in ~/.config/systemd/user.
                    if timer.editable {
                        let lbl = if is_small { "\u{270e}" } else { "Edit" };
                        sec.pc.button(
                            lbl,
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
                    let run_lbl = if is_small { "\u{25b6}" } else { "Run Now" };
                    sec.pc.button(
                        run_lbl,
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
                            let lbl = if is_small { "\u{25a0}" } else { "Disable" };
                            sec.pc.button(
                                lbl,
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
                            let lbl = if is_small { "\u{25cf}" } else { "Enable" };
                            sec.pc.button(
                                lbl,
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
                            sec.pc.text("static", en_x + 8.0, btn_y + 5.0, 11.0, TEXT_DIM);
                        }
                    }
                }
            }
            sec.pc.pop_clip_rect();

            if filtered.is_empty() {
                sec.pc.text("No timers in this scope", list_box_x + 16.0, list_box_y + 16.0, 12.0, TEXT_DIM);
            }

            // End the section so the well's bottom wall sits 12px below the list.
            sec.content_y = list_box_y + list_box_h + 12.0 - (sec.padding() + 12.0);
        }
    });

    final_pc
}

pub fn update(state: &mut TimersState, msg: TimersMessage) {
    match msg {
        TimersMessage::Refreshed(timers) => {
            state.loaded = true;
            state.timers = timers;
            state.items.clear();
        }
        TimersMessage::SetTab(tab) => {
            state.active_tab = tab;
            state.list.set_scroll_y(0.0);
            state.items.clear();
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
            for tb in [&mut state.name_box, &mut state.command_box, &mut state.schedule_box] {
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
                .unwrap_or_default();
            state.creating = false;
            state.editing = Some(base.clone());
            state.status_msg = None;
            state.name_box.text = base.clone();
            state.name_box.edit_buffer = base;
            state.command_box.text = command.clone();
            state.command_box.edit_buffer = command;
            state.schedule_box.text = schedule.clone();
            state.schedule_box.edit_buffer = schedule;
        }
        TimersMessage::CreateSave => {
            let command = live_text(&state.command_box);
            let schedule = live_text(&state.schedule_box);
            let result = if let Some(base) = state.editing.clone() {
                update_user_timer(&base, &command, &schedule)
            } else {
                create_user_timer(&live_text(&state.name_box), &command, &schedule)
            };
            match result {
                Ok(msg) => {
                    state.creating = false;
                    state.editing = None;
                    state.status_msg = Some(msg);
                    // Show the unit where it (re)appears on the next refresh.
                    state.active_tab = TimerTab::User;
                    state.items.clear();
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
        layout: &mut dyn LayoutStrategy,
        ctx: &mut cce_ui::context::UiContext,
    ) -> crate::app::PageContent {
        view(self, cx, cy, cw, ch, root_focused, sec_focused, layout, ctx)
    }

    fn propagate_widget_changes(&mut self, _actions: &mut Vec<crate::app::AppAction>) {}

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

    fn register_extra_dispatch_roots(&mut self, ctx: &mut cce_ui::context::UiContext) {
        let (list, items) = (&self.list, &mut self.items);
        for (idx, i) in items.iter_mut().enumerate() {
            if list.get_item_draw_y(idx, 4.0).is_none() {
                continue;
            }
            let (id, ptr) = (i.id(), i.as_ptr_mut());
            ctx.register_widget(id, ptr);
        }
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
