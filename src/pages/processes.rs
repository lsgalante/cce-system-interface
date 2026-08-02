use crate::app::{AppAction, PageContent};
use crate::scroll_region::ScrollRegion;
use cce_ui::layout::{PageLayoutBuilder, LayoutStrategy, RenderTarget};

#[derive(Debug, Clone)]
pub struct ProcessesState {
    pub loaded: bool,
    pub processes: Vec<(String, String, String)>, // (pid, cpu, comm)
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

pub async fn fetch_processes_state() -> ProcessesState {
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
            let header_h = 22.0;
            state.cpu_list.set_rect(list_box_x, list_box_y, list_box_w, list_box_h);
            state.cpu_list.update_bounds(state.processes.len(), list_box_y + header_h, list_box_h - header_h - 6.0);
            state.cpu_list.push_prims(sec.pc);

            // Header row is background-less (the well shows through); only the
            // divider separates it from the rows.
            sec.pc.rect([0.18, 0.18, 0.24, 1.0], list_box_x + 1.0, list_box_y + header_h, list_box_w - 2.0, 1.0); // Divider

            sec.pc.text("PID", list_box_x + 12.0, list_box_y + 5.0, 11.0, [0.53, 0.53, 0.60, 1.0]);
            sec.pc.text("COMMAND", list_box_x + 80.0, list_box_y + 5.0, 11.0, [0.53, 0.53, 0.60, 1.0]);
            sec.pc.text("CPU %", list_box_x + list_box_w - 60.0, list_box_y + 5.0, 11.0, [0.53, 0.53, 0.60, 1.0]);

            let row_h = 24.0;

            // Visible process rows rendering (virtualized/clipped)
            sec.pc.push_clip_rect(list_box_x, list_box_y + header_h, list_box_w, list_box_h - header_h);
            for (idx, (pid, cpu, comm)) in state.processes.iter().enumerate() {
                if let Some(draw_y) = state.cpu_list.get_item_draw_y(idx, 4.0) {
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
}
