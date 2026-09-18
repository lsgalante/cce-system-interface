use crate::app::{AppAction, PageContent};
use crate::power_meter::{self, Meter, Mode};
use cce_ui::widget::ScrollRegion;
use cce_ui::layout::{PageLayoutBuilder, LayoutStrategy, RenderTarget};
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct ProcessRow {
    pub pid: String,
    pub cpu: String,
    pub mem_pct: String,
    pub rss_kb: u64,
    pub command: String,
    /// Estimated draw from [`power_meter`]; None until the meter has two
    /// samples of this pid, or when nothing on the host reports watts.
    pub watts: Option<f32>,
    /// Context switches per second over the last interval — the honest
    /// signal for a process that burns power while looking idle.
    pub wakeups: Option<f32>,
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
}

impl Default for ProcessesState {
    fn default() -> Self {
        Self {
            loaded: false,
            processes: Vec::new(),
            cpu_list: ScrollRegion::new(24.0, 2.0).with_frame(false),
            killing: std::collections::HashSet::new(),
            sort: ProcSort::Cpu,
            power: PowerSummary::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ProcessesMessage {
    Refreshed(ProcessesState),
    /// The row's ✕ button: SIGTERM this pid.
    Kill(String),
    /// A column header click. Mem headers toggle (Mem ⇄ back to Cpu); the
    /// CPU % header always selects Cpu.
    SortBy(ProcSort),
    None,
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
        ProcSort::Mem => rows.sort_by(|a, b| b.rss_kb.cmp(&a.rss_kb)),
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

/// Process name from a `ps … cmd` field: basename of argv[0], so cce binaries
/// longer than the kernel's 15-char `comm` cap display whole (`comm` showed
/// "cce-system-inte"). Kernel threads (`[kworker/0:1]`) keep their brackets.
pub fn command_display(cmd: &str) -> String {
    let first = cmd.split_whitespace().next().unwrap_or(cmd);
    if first.starts_with('[') {
        return first.to_string();
    }
    std::path::Path::new(first)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(first)
        .to_string()
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

pub async fn fetch_processes_state() -> ProcessesState {
    let dgpu_w = dgpu_draw_w().await;
    let attribution = tokio::task::spawn_blocking(move || {
        let snap = power_meter::sample(dgpu_w);
        METER.lock().unwrap().get_or_insert_with(Meter::new).tick(snap)
    })
    .await
    .ok();
    let power = attribution
        .as_ref()
        .map(|a| PowerSummary { mode: a.mode, total_w: a.total_w, floor_w: a.floor_w, attributed_w: a.attributed_w })
        .unwrap_or_default();

    let processes = {
        let mut list = Vec::new();
        if let Some(o) = tokio::process::Command::new("ps")
            .args(["-eo", "pid,%cpu,%mem,rss,cmd", "--sort=-%cpu"])
            .output().await.ok()
        {
            let text = String::from_utf8_lossy(&o.stdout);
            for line in text.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 5 {
                    let pp = parts[0]
                        .parse::<u32>()
                        .ok()
                        .and_then(|pid| attribution.as_ref().and_then(|a| a.per_pid.get(&pid)).copied());
                    list.push(ProcessRow {
                        pid: parts[0].to_string(),
                        cpu: parts[1].to_string(),
                        mem_pct: parts[2].to_string(),
                        rss_kb: parts[3].parse().unwrap_or(0),
                        command: command_display(&parts[4..].join(" ")),
                        watts: pp.and_then(|p| p.watts),
                        wakeups: pp.map(|p| p.wakeups_per_s),
                    });
                }
            }
        }
        list
    };

    ProcessesState {
        loaded: true,
        processes,
        cpu_list: ScrollRegion::new(24.0, 2.0).with_frame(false),
        killing: std::collections::HashSet::new(),
        sort: ProcSort::Cpu,
        power,
    }
}

const TEXT_FG: [f32; 4] = [0.83, 0.83, 0.83, 1.0];
const TEXT_DIM: [f32; 4] = [0.53, 0.53, 0.60, 1.0];

pub fn view(state: &mut ProcessesState, cx: f32, cy: f32, cw: f32, ch: f32, root_focused: bool, sec_focused: &[bool], layout: &mut dyn LayoutStrategy, _ctx: &mut cce_ui::context::UiContext) -> PageContent {
    let mut final_pc = PageContent::new();
    let sec_w = 320.0f32;
    let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(1);

    // ── Processes Section (label-less well) ──
    builder.add_section_spanned(&mut final_pc, "", 1, root_focused || sec_focused.first().copied().unwrap_or(false), |sec| {
        let rx = sec.left;
        if !state.loaded {
            sec.text("Loading processes...", 12.0, 0.0, 12.0, TEXT_FG);
        } else {
            // Scrolling box configuration for process list: one even inset
            // between the list and the well's walls on all four sides.
            let inset = 12.0;
            let list_box_x = rx + inset;
            let list_box_w = sec.cw - 2.0 * inset;
            // Power summary line above the list; the list starts below it.
            let summary_h = 18.0;
            sec.pc.text(&summary_line(&state.power), list_box_x, sec.well_top() + inset + 2.0, 11.0, TEXT_DIM);
            let list_box_y = sec.well_top() + inset + summary_h;
            // Fill the page: the well's bottom wall lands at the page bottom,
            // the list keeps its even inset inside the well.
            let list_box_h = ((cy + ch) - inset - list_box_y).max(120.0);

            // Dissolved List (Phase 6v): scroll state + frame prims are app-owned. The
            // scrollable viewport starts below the header.
            //
            // Columns live in CONTENT space at fixed offsets; every draw
            // subtracts scroll_x. CONTENT_W > box width = the h-bar appears.
            const COL_PID: f32 = 12.0;
            const COL_COMMAND: f32 = 80.0;
            const COL_RSS: f32 = 400.0;
            const COL_MEM: f32 = 480.0;
            const COL_CPU: f32 = 545.0;
            const COL_WATTS: f32 = 605.0;
            const COL_WAKE: f32 = 655.0;
            const COL_KILL: f32 = 720.0;
            const CONTENT_W: f32 = 745.0;

            let header_h = 22.0;
            state.cpu_list.set_rect(list_box_x, list_box_y, list_box_w, list_box_h);
            state.cpu_list.set_content_w(CONTENT_W);
            // The bottom scrollbar needs its own band: rows must stop above
            // it or the last row draws under the pills.
            let bottom_reserve = if state.cpu_list.h_scroll_active() { 18.0 } else { 6.0 };
            state.cpu_list.update_bounds(state.processes.len(), list_box_y + header_h, list_box_h - header_h - bottom_reserve);
            state.cpu_list.push_prims(sec.pc);

            let ox = state.cpu_list.scroll_x;

            // Header row is background-less (the well shows through); only the
            // divider separates it from the rows.
            sec.pc.rect([0.18, 0.18, 0.24, 1.0], list_box_x + 1.0, list_box_y + header_h, list_box_w - 2.0, 1.0); // Divider

            // Header labels pan with the columns, clipped to the box. The
            // sortable ones (MEM, MEM %, CPU %) are buttons: the active key
            // shows brighter with a ▾. Both memory headers toggle the same
            // Mem sort — one bigger target, no distinction to learn.
            let active = |k: ProcSort| state.sort == k;
            let hdr = |on: bool| if on { [0.78, 0.78, 0.85, 1.0] } else { TEXT_DIM };
            let mark = |label: &str, on: bool| {
                if on { format!("{} \u{25bc}", label) } else { label.to_string() }
            };
            sec.pc.push_clip_rect(list_box_x, list_box_y, list_box_w, header_h);
            sec.pc.text("PID", list_box_x + COL_PID - ox, list_box_y + 5.0, 11.0, TEXT_DIM);
            sec.pc.text("COMMAND", list_box_x + COL_COMMAND - ox, list_box_y + 5.0, 11.0, TEXT_DIM);
            sec.pc.text(&mark("MEM", active(ProcSort::Mem)), list_box_x + COL_RSS - ox, list_box_y + 5.0, 11.0, hdr(active(ProcSort::Mem)));
            sec.pc.text("MEM %", list_box_x + COL_MEM - ox, list_box_y + 5.0, 11.0, hdr(active(ProcSort::Mem)));
            sec.pc.text(&mark("CPU %", active(ProcSort::Cpu)), list_box_x + COL_CPU - ox, list_box_y + 5.0, 11.0, hdr(active(ProcSort::Cpu)));
            sec.pc.text(&mark("W", active(ProcSort::Power)), list_box_x + COL_WATTS - ox, list_box_y + 5.0, 11.0, hdr(active(ProcSort::Power)));
            sec.pc.text(&mark("WAKE/s", active(ProcSort::Wakeups)), list_box_x + COL_WAKE - ox, list_box_y + 5.0, 11.0, hdr(active(ProcSort::Wakeups)));

            // Invisible header hit targets (transparent, subtle hover), inside
            // the header clip so they pan and cut with the labels. They share
            // no rect with the row buttons, so emission order is free here.
            sec.pc.button("", list_box_x + COL_RSS - ox - 4.0, list_box_y, 52.0, header_h - 2.0,
                [0.0; 4], [1.0, 1.0, 1.0, 0.05], [0.0; 4], AppAction::Processes(ProcessesMessage::SortBy(ProcSort::Mem)));
            sec.pc.button("", list_box_x + COL_MEM - ox - 4.0, list_box_y, 58.0, header_h - 2.0,
                [0.0; 4], [1.0, 1.0, 1.0, 0.05], [0.0; 4], AppAction::Processes(ProcessesMessage::SortBy(ProcSort::Mem)));
            sec.pc.button("", list_box_x + COL_CPU - ox - 4.0, list_box_y, 58.0, header_h - 2.0,
                [0.0; 4], [1.0, 1.0, 1.0, 0.05], [0.0; 4], AppAction::Processes(ProcessesMessage::SortBy(ProcSort::Cpu)));
            sec.pc.button("", list_box_x + COL_WATTS - ox - 4.0, list_box_y, 48.0, header_h - 2.0,
                [0.0; 4], [1.0, 1.0, 1.0, 0.05], [0.0; 4], AppAction::Processes(ProcessesMessage::SortBy(ProcSort::Power)));
            sec.pc.button("", list_box_x + COL_WAKE - ox - 4.0, list_box_y, 62.0, header_h - 2.0,
                [0.0; 4], [1.0, 1.0, 1.0, 0.05], [0.0; 4], AppAction::Processes(ProcessesMessage::SortBy(ProcSort::Wakeups)));
            sec.pc.pop_clip_rect();

            let row_h = 24.0;

            // Visible process rows rendering (virtualized/clipped)
            sec.pc.push_clip_rect(list_box_x, list_box_y + header_h, list_box_w, list_box_h - header_h);
            for (idx, p) in state.processes.iter().enumerate() {
                if let Some(draw_y) = state.cpu_list.get_item_draw_y(idx, 4.0) {
                    // Standard row action button (transparent background, highlights
                    // on hover). Viewport-fixed on purpose: the hover band spans the
                    // visible row whatever the horizontal pan.
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

                    // A pending kill dims the row until the next refresh
                    // settles it (gone, or alive again = the kill didn't take).
                    let dim = state.killing.contains(&p.pid);
                    let fg = if dim { [0.45, 0.45, 0.50, 1.0] } else { [0.80, 0.80, 0.85, 1.0] };
                    let mem_fg = if dim { [0.45, 0.45, 0.50, 1.0] } else { [0.62, 0.72, 0.88, 1.0] };
                    let cpu_fg = if dim { [0.45, 0.45, 0.50, 1.0] } else { [0.56, 0.83, 0.56, 1.0] };
                    let watt_fg = if dim { [0.45, 0.45, 0.50, 1.0] } else { [0.90, 0.75, 0.45, 1.0] };

                    sec.pc.text(&p.pid, list_box_x + COL_PID - ox, draw_y + 6.0, 12.0, fg);
                    sec.pc.text(&p.command, list_box_x + COL_COMMAND - ox, draw_y + 6.0, 12.0, fg);
                    sec.pc.text(&format_rss(p.rss_kb), list_box_x + COL_RSS - ox, draw_y + 6.0, 12.0, mem_fg);
                    sec.pc.text(&format!("{}%", p.mem_pct), list_box_x + COL_MEM - ox, draw_y + 6.0, 12.0, mem_fg);
                    sec.pc.text(&format!("{}%", p.cpu), list_box_x + COL_CPU - ox, draw_y + 6.0, 12.0, cpu_fg);
                    sec.pc.text(&format_watts(p.watts), list_box_x + COL_WATTS - ox, draw_y + 6.0, 12.0, watt_fg);
                    sec.pc.text(&format_wakeups(p.wakeups), list_box_x + COL_WAKE - ox, draw_y + 6.0, 12.0, fg);

                    // Kill button, in content space like the columns. Emitted
                    // AFTER the row button on purpose: overlapping page
                    // buttons all see the click and the LAST take_click wins
                    // the dispatched action (input_handler's collect loop), so
                    // ✕ beats the row's no-op exactly because it comes later.
                    if !dim {
                        sec.pc.button(
                            "\u{00d7}",
                            list_box_x + COL_KILL - ox,
                            draw_y + 3.0,
                            20.0,
                            row_h - 6.0,
                            [0.0, 0.0, 0.0, 0.0],
                            [0.75, 0.30, 0.30, 0.45],
                            [0.85, 0.55, 0.55, 1.0],
                            AppAction::Processes(ProcessesMessage::Kill(p.pid.clone())),
                        );
                    }
                }
            }
            sec.pc.pop_clip_rect();
            
            if state.processes.is_empty() {
                sec.pc.text("No active processes", list_box_x + 12.0, list_box_y + header_h + 16.0, 12.0, TEXT_DIM);
            }

            // End the section so the well's bottom wall sits `inset` below the
            // list (finish() places the wall at content_y + padding + 12).
            sec.content_y = list_box_y + list_box_h + inset - (sec.padding() + 12.0);
        }
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
        }
        ProcessesMessage::SortBy(key) => {
            // Clicking the already-active header (Mem, Power, Wakeups)
            // toggles back to the Cpu default; CPU % is always a plain select.
            state.sort = if state.sort == key { ProcSort::Cpu } else { key };
            sort_rows(&mut state.processes, state.sort);
        }
        ProcessesMessage::Kill(pid) => {
            // Plain SIGTERM, same privileges as the app. No confirm dialog:
            // the target is a small ✕ the pointer has to mean. Failure needs
            // no channel — a survivor un-dims on the next 3s refresh.
            // Parsed, not passed through: `kill 0` signals the whole process
            // group (this app included), and negative pids kill groups too.
            if pid.parse::<u32>().is_ok_and(|n| n > 0) {
                let _ = std::process::Command::new("kill").arg(&pid).spawn();
                state.killing.insert(pid);
            }
        }
        ProcessesMessage::None => {}
    }
}

impl crate::pages::AppPage for ProcessesState {
    // Sections: [Processes]
    fn section_widgets(&mut self) -> Vec<Vec<cce_ui::widget::WidgetId>> {
        vec![Vec::new()]
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_layout_grid() {
        let mut state = ProcessesState::default();
        let mut layout = cce_ui::layout::ColumnLayout::new(20.0);
        let sec_focused = vec![false];
        let mut ctx = cce_ui::context::UiContext::new();
        let pc = view(&mut state, 10.0, 20.0, 800.0, 600.0, false, &sec_focused, &mut layout, &mut ctx);
        assert!(!pc.rects.is_empty() || !pc.texts.is_empty());
    }

    #[test]
    fn command_display_prefers_basename_and_keeps_kernel_threads() {
        // Longer than the 15-char comm cap ps used to truncate at.
        assert_eq!(command_display("/home/x/.local/bin/cce-system-interface --flag"), "cce-system-interface");
        assert_eq!(command_display("bash"), "bash");
        assert_eq!(command_display("[kworker/0:1-events]"), "[kworker/0:1-events]");
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
            watts: None,
            wakeups: None,
        }
    }

    #[test]
    fn kill_buttons_emitted_after_row_buttons_and_skip_pending() {
        let mut state = ProcessesState { loaded: true, ..Default::default() };
        state.processes = (1..=3).map(|i| row(&i.to_string())).collect();
        state.killing.insert("2".to_string());
        let mut layout = cce_ui::layout::ColumnLayout::new(20.0);
        let sec_focused = vec![false];
        let mut ctx = cce_ui::context::UiContext::new();
        let pc = view(&mut state, 10.0, 20.0, 800.0, 600.0, false, &sec_focused, &mut layout, &mut ctx);

        let kills: Vec<usize> = pc
            .buttons
            .iter()
            .enumerate()
            .filter(|(_, (_, a, _))| matches!(a, AppAction::Processes(ProcessesMessage::Kill(_))))
            .map(|(i, _)| i)
            .collect();
        // One ✕ per row except the pending one (pid 2).
        assert_eq!(kills.len(), 2, "{:?}", pc.buttons.iter().map(|(_, a, _)| a).collect::<Vec<_>>());
        // Ordering invariant the dispatch relies on: each ✕ comes after its
        // row's hover button — last take_click wins, so ✕ must be later.
        let rows: Vec<usize> = pc
            .buttons
            .iter()
            .enumerate()
            .filter(|(_, (_, a, _))| matches!(a, AppAction::Processes(ProcessesMessage::None)))
            .map(|(i, _)| i)
            .collect();
        assert!(kills[0] > rows[0]);
    }

    #[test]
    fn kill_guards_and_dim_lifecycle() {
        let mut state = ProcessesState { loaded: true, ..Default::default() };
        // Group-signal and garbage pids are refused outright.
        update(&mut state, ProcessesMessage::Kill("0".to_string()));
        update(&mut state, ProcessesMessage::Kill("-1".to_string()));
        update(&mut state, ProcessesMessage::Kill("abc".to_string()));
        update(&mut state, ProcessesMessage::Kill(String::new()));
        assert!(state.killing.is_empty());

        // A real (nonexistent, > pid_max) pid marks the row...
        update(&mut state, ProcessesMessage::Kill("99999999".to_string()));
        assert!(state.killing.contains("99999999"));

        // ...and the next refresh clears every pending mark.
        update(
            &mut state,
            ProcessesMessage::Refreshed(ProcessesState { loaded: true, ..Default::default() }),
        );
        assert!(state.killing.is_empty());
    }

    fn sized_row(pid: &str, cpu: &str, rss: u64) -> ProcessRow {
        ProcessRow {
            pid: pid.to_string(),
            cpu: cpu.to_string(),
            mem_pct: "0.0".to_string(),
            rss_kb: rss,
            command: "p".to_string(),
            watts: None,
            wakeups: None,
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
        update(&mut state, ProcessesMessage::Refreshed(fresh));
        // The cpu-ordered fetch was re-sorted under the surviving Mem key.
        assert_eq!(state.sort, ProcSort::Mem);
        assert_eq!(state.processes[0].pid, "y");
    }

    #[test]
    fn header_sort_buttons_emitted() {
        let mut state = ProcessesState { loaded: true, ..Default::default() };
        state.processes = vec![sized_row("1", "1.0", 1)];
        let mut layout = cce_ui::layout::ColumnLayout::new(20.0);
        let sec_focused = vec![false];
        let mut ctx = cce_ui::context::UiContext::new();
        let pc = view(&mut state, 10.0, 20.0, 800.0, 600.0, false, &sec_focused, &mut layout, &mut ctx);
        let sorts: Vec<ProcSort> = pc
            .buttons
            .iter()
            .filter_map(|(_, a, _)| match a {
                AppAction::Processes(ProcessesMessage::SortBy(k)) => Some(*k),
                _ => None,
            })
            .collect();
        // MEM + MEM % both toggle Mem; CPU % selects Cpu; then W and WAKE/s.
        assert_eq!(sorts, [ProcSort::Mem, ProcSort::Mem, ProcSort::Cpu, ProcSort::Power, ProcSort::Wakeups]);
        // Active-sort indicator rides the CPU % header by default.
        assert!(pc.texts.iter().any(|t| t.0.starts_with("CPU %") && t.0.contains('\u{25bc}')));
    }

    #[test]
    fn columns_pan_with_horizontal_scroll() {
        let mut state = ProcessesState { loaded: true, ..Default::default() };
        state.processes = (0..3).map(|i| row(&i.to_string())).collect();
        let mut layout = cce_ui::layout::ColumnLayout::new(20.0);
        let sec_focused = vec![false];
        let mut ctx = cce_ui::context::UiContext::new();

        // Prefix match: the active sort column carries a " ▾" suffix.
        let header_x = |pc: &crate::app::PageContent, s: &str| {
            pc.texts.iter().find(|t| t.0.starts_with(s)).map(|t| t.2).unwrap()
        };
        let pc0 = view(&mut state, 10.0, 20.0, 500.0, 400.0, false, &sec_focused, &mut layout, &mut ctx);
        let x0 = header_x(&pc0, "MEM %");
        // Content (650) is wider than the ~500px page, so the list is
        // h-scrollable; pan and every column shifts left by exactly that.
        assert!(state.cpu_list.h_scroll_active());
        state.cpu_list.scroll_x = 40.0;
        let pc1 = view(&mut state, 10.0, 20.0, 500.0, 400.0, false, &sec_focused, &mut layout, &mut ctx);
        assert_eq!(header_x(&pc1, "MEM %"), x0 - 40.0);
        assert_eq!(header_x(&pc1, "CPU %"), header_x(&pc0, "CPU %") - 40.0);
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
        update(&mut state, ProcessesMessage::Refreshed(fresh));
        assert_eq!(state.power.mode, Mode::Battery);
        let mut layout = cce_ui::layout::ColumnLayout::new(20.0);
        let mut ctx = cce_ui::context::UiContext::new();
        let pc = view(&mut state, 10.0, 20.0, 900.0, 600.0, false, &[false], &mut layout, &mut ctx);
        let all: Vec<&str> = pc.texts.iter().map(|t| t.0.as_str()).collect();
        assert!(pc.texts.iter().any(|t| t.0 == "1.3"), "{all:?}");
        assert!(pc.texts.iter().any(|t| t.0 == "40"));
        assert!(pc.texts.iter().any(|t| t.0.starts_with("20.0 W total")));
    }
}
