use crate::app::{AppAction, PageContent};
use crate::scroll_region::ScrollRegion;
use cce_ui::layout::{PageLayoutBuilder, LayoutStrategy, RenderTarget};

#[derive(Debug, Clone)]
pub struct ProcessRow {
    pub pid: String,
    pub cpu: String,
    pub mem_pct: String,
    pub rss_kb: u64,
    pub command: String,
}

#[derive(Debug, Clone)]
pub struct ProcessesState {
    pub loaded: bool,
    pub processes: Vec<ProcessRow>,
    pub cpu_list: ScrollRegion,
}

impl Default for ProcessesState {
    fn default() -> Self {
        Self {
            loaded: false,
            processes: Vec::new(),
            cpu_list: ScrollRegion::new(24.0, 2.0).with_frame(false),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ProcessesMessage {
    Refreshed(ProcessesState),
    None,
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
                    list.push(ProcessRow {
                        pid: parts[0].to_string(),
                        cpu: parts[1].to_string(),
                        mem_pct: parts[2].to_string(),
                        rss_kb: parts[3].parse().unwrap_or(0),
                        command: command_display(&parts[4..].join(" ")),
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
            let list_box_y = sec.well_top() + inset;
            let list_box_w = sec.cw - 2.0 * inset;
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
            const COL_RSS: f32 = 420.0;
            const COL_MEM: f32 = 510.0;
            const COL_CPU: f32 = 580.0;
            const CONTENT_W: f32 = 650.0;

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

            // Header labels pan with the columns, clipped to the box.
            sec.pc.push_clip_rect(list_box_x, list_box_y, list_box_w, header_h);
            sec.pc.text("PID", list_box_x + COL_PID - ox, list_box_y + 5.0, 11.0, TEXT_DIM);
            sec.pc.text("COMMAND", list_box_x + COL_COMMAND - ox, list_box_y + 5.0, 11.0, TEXT_DIM);
            sec.pc.text("MEM", list_box_x + COL_RSS - ox, list_box_y + 5.0, 11.0, TEXT_DIM);
            sec.pc.text("MEM %", list_box_x + COL_MEM - ox, list_box_y + 5.0, 11.0, TEXT_DIM);
            sec.pc.text("CPU %", list_box_x + COL_CPU - ox, list_box_y + 5.0, 11.0, TEXT_DIM);
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

                    sec.pc.text(&p.pid, list_box_x + COL_PID - ox, draw_y + 6.0, 12.0, [0.80, 0.80, 0.85, 1.0]);
                    sec.pc.text(&p.command, list_box_x + COL_COMMAND - ox, draw_y + 6.0, 12.0, [0.80, 0.80, 0.85, 1.0]);
                    sec.pc.text(&format_rss(p.rss_kb), list_box_x + COL_RSS - ox, draw_y + 6.0, 12.0, [0.62, 0.72, 0.88, 1.0]);
                    sec.pc.text(&format!("{}%", p.mem_pct), list_box_x + COL_MEM - ox, draw_y + 6.0, 12.0, [0.62, 0.72, 0.88, 1.0]);
                    sec.pc.text(&format!("{}%", p.cpu), list_box_x + COL_CPU - ox, draw_y + 6.0, 12.0, [0.56, 0.83, 0.56, 1.0]);
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
        }
    }

    #[test]
    fn columns_pan_with_horizontal_scroll() {
        let mut state = ProcessesState { loaded: true, ..Default::default() };
        state.processes = (0..3).map(|i| row(&i.to_string())).collect();
        let mut layout = cce_ui::layout::ColumnLayout::new(20.0);
        let sec_focused = vec![false];
        let mut ctx = cce_ui::context::UiContext::new();

        let header_x = |pc: &crate::app::PageContent, s: &str| {
            pc.texts.iter().find(|t| t.0 == s).map(|t| t.2).unwrap()
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
}
