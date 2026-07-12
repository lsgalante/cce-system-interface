use crate::app::{AppAction, PageContent, SectionContextExt};
use cce_ui::layout::{render_widget, PageLayoutBuilder, LayoutStrategy};
use cce_ui::widget::{Spinbox, Slider, WidgetHost};

#[derive(Debug, Clone)]
pub struct AudioSink {
    pub id: u32,
    pub name: String,
    pub volume: f32,
    pub muted: bool,
    pub active: bool,
}

#[derive(Debug, Clone)]
pub struct AudioSource {
    pub id: u32,
    pub name: String,
    pub volume: f32,
    pub muted: bool,
    pub active: bool,
}

#[derive(Debug, Clone, Default)]
pub struct AudioState {
    pub loaded: bool,
    pub sinks: Vec<AudioSink>,
    pub sources: Vec<AudioSource>,
    pub sink_spinboxes: Vec<Box<cce_ui::widget::Adapted<cce_ui::widget::Spinbox>>>,
    pub source_spinboxes: Vec<Box<cce_ui::widget::Adapted<cce_ui::widget::Spinbox>>>,
    pub sink_sliders: Vec<Box<cce_ui::widget::Adapted<cce_ui::widget::Slider>>>,
    pub source_sliders: Vec<Box<cce_ui::widget::Adapted<cce_ui::widget::Slider>>>,
    pub sink_dragging: Option<usize>,
    pub source_dragging: Option<usize>,
}

#[derive(Debug, Clone)]
pub enum AudioMessage {
    Refreshed(AudioState),
    SinkVolume(u32, f32),
    SinkMute(u32),
    SourceVolume(u32, f32),
    SourceMute(u32),
}

fn drm_connected_ports() -> Vec<String> {
    let mut connected = Vec::new();
    let Ok(entries) = std::fs::read_dir("/sys/class/drm") else {
        return connected;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let status_path = entry.path().join("status");
        if status_path.exists() {
            if let Ok(s) = std::fs::read_to_string(&status_path) {
                if s.trim() == "connected" {
                    connected.push(name);
                }
            }
        }
    }
    connected
}

fn is_hdmi_active(name: &str, connected_ports: &[String]) -> bool {
    if !name.contains("HDMI") {
        return true;
    }
    for port in connected_ports {
        if !port.contains("HDMI") {
            continue;
        }
        if let Some(num) = name.split("HDMI").nth(1).and_then(|s| s.chars().next()) {
            if port.contains(&format!("HDMI-A-{}", num)) {
                return true;
            }
        }
    }
    false
}

fn short_name(name: &str) -> String {
    if let Some(hifi_part) = name.split("HiFi__").nth(1) {
        let label = hifi_part.replace("__sink", "").replace("_", " ");
        return label;
    }
    if let Some(alsa) = name.strip_prefix("alsa_output.") {
        return alsa.split('.').next_back().unwrap_or(alsa).replace("_", " ");
    }
    if let Some(alsa) = name.strip_prefix("alsa_input.") {
        return alsa.split('.').next_back().unwrap_or(alsa).replace("_", " ");
    }
    name.to_string()
}

fn set_sink_volume(id: u32, vol: f32) {
    let pct = (vol * 100.0).round() as u32;
    let _ = tokio::process::Command::new("pactl")
        .args(["set-sink-volume", &id.to_string(), &format!("{}%", pct)])
        .spawn();
}

fn set_sink_mute(id: u32, mute: bool) {
    let _ = tokio::process::Command::new("pactl")
        .args(["set-sink-mute", &id.to_string(), if mute { "1" } else { "0" }])
        .spawn();
}

fn set_source_volume(id: u32, vol: f32) {
    let pct = (vol * 100.0).round() as u32;
    let _ = tokio::process::Command::new("pactl")
        .args(["set-source-volume", &id.to_string(), &format!("{}%", pct)])
        .spawn();
}

fn set_source_mute(id: u32, mute: bool) {
    let _ = tokio::process::Command::new("pactl")
        .args(["set-source-mute", &id.to_string(), if mute { "1" } else { "0" }])
        .spawn();
}

pub async fn fetch_audio_state() -> AudioState {
    let connected = drm_connected_ports();
    let sinks = fetch_sinks(&connected).await;
    let sources = fetch_sources(&connected).await;
    AudioState {
        loaded: true,
        sinks,
        sources,
        sink_spinboxes: Vec::new(),
        source_spinboxes: Vec::new(),
        sink_sliders: Vec::new(),
        source_sliders: Vec::new(),
        sink_dragging: None,
        source_dragging: None,
    }
}

async fn fetch_sinks(connected_ports: &[String]) -> Vec<AudioSink> {
    let short_out = match tokio::process::Command::new("pactl")
        .args(["list", "sinks", "short"]).output().await
    {
        Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(),
        Err(_) => return Vec::new(),
    };

    let mut sinks = Vec::new();
    for line in short_out.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 2 { continue; }
        let id = parts[0].parse::<u32>().unwrap_or(0);
        let name = parts[1].to_string();

        let vol = tokio::process::Command::new("pactl")
            .args(["get-sink-volume", &id.to_string()]).output().await.ok()
            .and_then(|o| {
                let s = String::from_utf8_lossy(&o.stdout);
                s.split('/').nth(1)
                    .and_then(|v| v.trim().trim_end_matches('%').parse::<f32>().ok())
            }).unwrap_or(50.0);

        let muted = tokio::process::Command::new("pactl")
            .args(["get-sink-mute", &id.to_string()]).output().await.ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains("yes"))
            .unwrap_or(false);

        sinks.push(AudioSink {
            id, name: short_name(&name), volume: vol / 100.0, muted,
            active: is_hdmi_active(&name, connected_ports),
        });
    }
    sinks.sort_by(|a, b| b.id.cmp(&a.id));
    sinks
}

async fn fetch_sources(connected_ports: &[String]) -> Vec<AudioSource> {
    let short_out = match tokio::process::Command::new("pactl")
        .args(["list", "sources", "short"]).output().await
    {
        Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(),
        Err(_) => return Vec::new(),
    };

    let mut sources = Vec::new();
    for line in short_out.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 2 { continue; }
        let id = parts[0].parse::<u32>().unwrap_or(0);
        let name = parts[1].to_string();

        if name.contains(".monitor") { continue; }

        let vol = tokio::process::Command::new("pactl")
            .args(["get-source-volume", &id.to_string()]).output().await.ok()
            .and_then(|o| {
                let s = String::from_utf8_lossy(&o.stdout);
                s.split('/').nth(1)
                    .and_then(|v| v.trim().trim_end_matches('%').parse::<f32>().ok())
            }).unwrap_or(50.0);

        let muted = tokio::process::Command::new("pactl")
            .args(["get-source-mute", &id.to_string()]).output().await.ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains("yes"))
            .unwrap_or(false);

        sources.push(AudioSource {
            id, name: short_name(&name), volume: vol / 100.0, muted,
            active: is_hdmi_active(&name, connected_ports),
        });
    }
    sources.sort_by(|a, b| b.id.cmp(&a.id));
    sources
}

#[allow(dead_code)]
const TEXT_FG: [f32; 4] = [0.83, 0.83, 0.83, 1.0];
const TEXT_DIM: [f32; 4] = [0.53, 0.53, 0.60, 1.0];
const MUTED_BG: [f32; 4] = [0.33, 0.20, 0.20, 1.0];
const BTN_INACTIVE: [f32; 4] = [0.13, 0.18, 0.14, 1.0];
const BTN_HOVER: [f32; 4] = [0.25, 0.30, 0.26, 1.0];
#[allow(dead_code)]
const BLANK_BAR: [f32; 4] = [0.15, 0.15, 0.24, 1.0];
#[allow(dead_code)]
const FILL_BAR: [f32; 4] = [0.30, 0.50, 0.32, 1.0];
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
#[allow(dead_code)]
const RED: [f32; 4] = [1.0, 0.33, 0.33, 1.0];

pub fn view(state: &mut AudioState, cx: f32, cy: f32, cw: f32, ch: f32, sec_focused: &[bool], layout: &mut dyn LayoutStrategy, ctx: &mut cce_ui::context::UiContext) -> PageContent {
    let mut final_pc = PageContent::new();
    let sec_w = 320.0f32;
    let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(2);

    // ── Output section ──
    builder.add_section(&mut final_pc, "Output", sec_focused.first().copied().unwrap_or(false), |sec| {
        if !state.loaded {
            sec.text("Loading output devices...", 12.0, 0.0, 12.0, TEXT_DIM);
        } else if state.sinks.is_empty() {
            sec.text("No output devices found", 12.0, 0.0, 12.0, TEXT_DIM);
        }

        if state.loaded {
            for (idx, sink) in state.sinks.iter().enumerate() {
                let sec_title = if sink.active {
                    sink.name.clone()
                } else {
                    format!("{} (inactive)", sink.name)
                };

                sec.add_section(&sec_title, false, |subsec| {
                    if !sink.active {
                        subsec.text("Device is inactive.", 12.0, 0.0, 12.0, TEXT_DIM);
                    } else {
                        let label = if sink.muted {
                            format!("Volume: {:.0}%  (muted)", sink.volume * 100.0)
                        } else {
                            format!("Volume: {:.0}%", sink.volume * 100.0)
                        };
                        state.sink_sliders[idx].set_label(&label);
                        state.sink_sliders[idx].set_value(sink.volume);

                        let mut stack = subsec.vstack(8.0);

                        let label_h = cce_ui::widget::label_offset(&*state.sink_sliders[idx]);
                        let slider_h = cce_ui::layout::slider_height() + label_h;
                        stack.add_widget(&mut *state.sink_sliders[idx], stack.context.cw - 28.0, slider_h, ctx);

                        let sb_h = cce_ui::layout::spinbox_height();
                        let gap = 8.0;

                        state.sink_spinboxes[idx].value = (sink.volume * 100.0).round() as i32;

                        let mute_label = if sink.muted { "Unmute" } else { "Mute" };
                        let mute_col = if sink.muted { MUTED_BG } else { BTN_INACTIVE };

                        stack.add_row(2, gap, sb_h, |sec_ctx, i, x, w| {
                            if i == 0 {
                                state.sink_spinboxes[idx].set_row_rect(x, w);
                                let y = sec_ctx.ay();
                                render_widget(sec_ctx.pc, &mut *state.sink_spinboxes[idx], x, y, w, sb_h, ctx);
                            } else {
                                sec_ctx.button(
                                    mute_label,
                                    x,
                                    sec_ctx.ay(),
                                    w,
                                    sb_h,
                                    mute_col,
                                    BTN_HOVER,
                                    WHITE,
                                    AppAction::Audio(AudioMessage::SinkMute(sink.id)),
                                );
                            }
                        });
                    }
                });
            }
        }
    });

    // ── Input section ──
    builder.add_section(&mut final_pc, "Input", sec_focused.get(1).copied().unwrap_or(false), |sec| {
        if !state.loaded {
            sec.text("Loading input devices...", 12.0, 0.0, 12.0, TEXT_DIM);
        } else if state.sources.is_empty() {
            sec.text("No input devices found", 12.0, 0.0, 12.0, TEXT_DIM);
        }

        if state.loaded {
            for (idx, src) in state.sources.iter().enumerate() {
                let sec_title = if src.active {
                    src.name.clone()
                } else {
                    format!("{} (inactive)", src.name)
                };

                sec.add_section(&sec_title, false, |subsec| {
                    if !src.active {
                        subsec.text("Device is inactive.", 12.0, 0.0, 12.0, TEXT_DIM);
                    } else {
                        let label = if src.muted {
                            format!("Volume: {:.0}%  (muted)", src.volume * 100.0)
                        } else {
                            format!("Volume: {:.0}%", src.volume * 100.0)
                        };
                        state.source_sliders[idx].set_label(&label);
                        state.source_sliders[idx].set_value(src.volume);

                        let mut stack = subsec.vstack(8.0);

                        let label_h = cce_ui::widget::label_offset(&*state.source_sliders[idx]);
                        let slider_h = cce_ui::layout::slider_height() + label_h;
                        stack.add_widget(&mut *state.source_sliders[idx], stack.context.cw - 28.0, slider_h, ctx);

                        let sb_h = cce_ui::layout::spinbox_height();
                        let gap = 8.0;

                        state.source_spinboxes[idx].value = (src.volume * 100.0).round() as i32;

                        let mute_label = if src.muted { "Unmute" } else { "Mute" };
                        let mute_col = if src.muted { MUTED_BG } else { BTN_INACTIVE };

                        stack.add_row(2, gap, sb_h, |sec_ctx, i, x, w| {
                            if i == 0 {
                                state.source_spinboxes[idx].set_row_rect(x, w);
                                let y = sec_ctx.ay();
                                render_widget(sec_ctx.pc, &mut *state.source_spinboxes[idx], x, y, w, sb_h, ctx);
                            } else {
                                sec_ctx.button(
                                    mute_label,
                                    x,
                                    sec_ctx.ay(),
                                    w,
                                    sb_h,
                                    mute_col,
                                    BTN_HOVER,
                                    WHITE,
                                    AppAction::Audio(AudioMessage::SourceMute(src.id)),
                                );
                            }
                        });
                    }
                });
            }
        }
    });

    final_pc
}


pub fn update(state: &mut AudioState, msg: AudioMessage) {
    match msg {
        AudioMessage::Refreshed(new) => {
            state.loaded = new.loaded;
            state.sinks = new.sinks;
            state.sources = new.sources;
            state.sink_spinboxes.resize_with(state.sinks.len(), || Box::new(Spinbox::new(50, 0, 100, 1)));
            state.source_spinboxes.resize_with(state.sources.len(), || Box::new(Spinbox::new(50, 0, 100, 1)));
            state.sink_sliders.resize_with(state.sinks.len(), || Box::new(Slider::new().with_range(0.0, 1.0).with_scroll(true)));
            state.source_sliders.resize_with(state.sources.len(), || Box::new(Slider::new().with_range(0.0, 1.0).with_scroll(true)));
        }
        AudioMessage::SinkVolume(id, vol) => {
            if let Some(sink) = state.sinks.iter_mut().find(|s| s.id == id) {
                sink.volume = vol;
                set_sink_volume(id, vol);
            }
        }
        AudioMessage::SinkMute(id) => {
            if let Some(sink) = state.sinks.iter_mut().find(|s| s.id == id) {
                sink.muted = !sink.muted;
                set_sink_mute(id, sink.muted);
            }
        }
        AudioMessage::SourceVolume(id, vol) => {
            if let Some(src) = state.sources.iter_mut().find(|s| s.id == id) {
                src.volume = vol;
                set_source_volume(id, vol);
            }
        }
        AudioMessage::SourceMute(id) => {
            if let Some(src) = state.sources.iter_mut().find(|s| s.id == id) {
                src.muted = !src.muted;
                set_source_mute(id, src.muted);
            }
        }
    }
}

impl crate::pages::AppPage for AudioState {
    // Sections: [Output, Input] — only active devices' controls, spinbox before slider
    // per device (the old link order).
    fn section_widgets(&mut self) -> Vec<Vec<*mut (dyn cce_ui::widget::WidgetHost + 'static)>> {
        use cce_ui::widget::WidgetHost;
        let mut output: Vec<*mut (dyn WidgetHost + 'static)> = Vec::new();
        for (i, sink) in self.sinks.iter().enumerate() {
            if sink.active {
                if let Some(sb) = self.sink_spinboxes.get_mut(i) {
                    output.push(sb.as_ptr_mut());
                }
                if let Some(sl) = self.sink_sliders.get_mut(i) {
                    output.push(sl.as_ptr_mut());
                }
            }
        }
        let mut input: Vec<*mut (dyn WidgetHost + 'static)> = Vec::new();
        for (i, src) in self.sources.iter().enumerate() {
            if src.active {
                if let Some(sb) = self.source_spinboxes.get_mut(i) {
                    input.push(sb.as_ptr_mut());
                }
                if let Some(sl) = self.source_sliders.get_mut(i) {
                    input.push(sl.as_ptr_mut());
                }
            }
        }
        vec![output, input]
    }

    fn view(
        &mut self,
        cx: f32,
        cy: f32,
        cw: f32,
        ch: f32,
        _root_focused: bool,
        sec_focused: &[bool],
        layout: &mut dyn cce_ui::layout::LayoutStrategy,
        ctx: &mut cce_ui::context::UiContext,
    ) -> crate::app::PageContent {
        view(self, cx, cy, cw, ch, sec_focused, layout, ctx)
    }

    fn propagate_widget_changes(&mut self, actions: &mut Vec<crate::app::AppAction>) {
        for (i, sb) in self.sink_spinboxes.iter_mut().enumerate() {
            if sb.take_change() {
                let id = self.sinks[i].id;
                actions.push(AppAction::Audio(AudioMessage::SinkVolume(id, sb.value as f32 / 100.0)));
            }
        }
        for (i, sb) in self.source_spinboxes.iter_mut().enumerate() {
            if sb.take_change() {
                let id = self.sources[i].id;
                actions.push(AppAction::Audio(AudioMessage::SourceVolume(id, sb.value as f32 / 100.0)));
            }
        }
        for (i, slider) in self.sink_sliders.iter_mut().enumerate() {
            if slider.take_change() {
                let id = self.sinks[i].id;
                actions.push(AppAction::Audio(AudioMessage::SinkVolume(id, slider.value() as f32 / 100.0)));
            }
        }
        for (i, slider) in self.source_sliders.iter_mut().enumerate() {
            if slider.take_change() {
                let id = self.sources[i].id;
                actions.push(AppAction::Audio(AudioMessage::SourceVolume(id, slider.value() as f32 / 100.0)));
            }
        }
    }

    fn handle_pointer_move(
        &mut self,
        lx: f32,
        ly: f32,
        actions: &mut Vec<crate::app::AppAction>,
        _ctx: &mut cce_ui::context::UiContext,
    ) -> bool {
        if let Some(idx) = self.sink_dragging {
            if let Some(slider) = self.sink_sliders.get_mut(idx) {
                if slider.drag_update(lx, ly) {
                    let id = self.sinks[idx].id;
                    let val = slider.inner().value();
                    if idx < self.sink_spinboxes.len() {
                        self.sink_spinboxes[idx].value = (val * 100.0).round() as i32;
                    }
                    actions.push(AppAction::Audio(AudioMessage::SinkVolume(id, val)));
                    return true;
                }
            }
        } else if let Some(idx) = self.source_dragging {
            if let Some(slider) = self.source_sliders.get_mut(idx) {
                if slider.drag_update(lx, ly) {
                    let id = self.sources[idx].id;
                    let val = slider.inner().value();
                    if idx < self.source_spinboxes.len() {
                        self.source_spinboxes[idx].value = (val * 100.0).round() as i32;
                    }
                    actions.push(AppAction::Audio(AudioMessage::SourceVolume(id, val)));
                    return true;
                }
            }
        }
        false
    }

    fn handle_pointer_down(&mut self, _lx: f32, _ly: f32, _ctx: &mut cce_ui::context::UiContext) -> bool {
        for (i, s) in self.sink_sliders.iter().enumerate() {
            if s.is_dragging() {
                self.sink_dragging = Some(i);
                return true;
            }
        }
        for (i, s) in self.source_sliders.iter().enumerate() {
            if s.is_dragging() {
                self.source_dragging = Some(i);
                return true;
            }
        }
        false
    }

    fn handle_pointer_up(&mut self, _ctx: &mut cce_ui::context::UiContext) -> bool {
        let mut any = false;
        if let Some(idx) = self.sink_dragging {
            self.sink_sliders[idx].drag_end();
            self.sink_dragging = None;
            any = true;
        }
        if let Some(idx) = self.source_dragging {
            self.source_sliders[idx].drag_end();
            self.source_dragging = None;
            any = true;
        }
        any
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cce_ui::layout::AdaptiveGrid;

    #[test]
    fn test_view_layout_grid() {
        use cce_ui::widget::{Spinbox, Slider};
        let mut state = AudioState {
            loaded: true,
            sinks: vec![
                AudioSink { id: 71, name: "Speaker".to_string(), volume: 0.57, muted: false, active: true },
                AudioSink { id: 70, name: "HDMI1".to_string(), volume: 0.5, muted: false, active: false },
                AudioSink { id: 69, name: "HDMI2".to_string(), volume: 0.5, muted: false, active: false },
                AudioSink { id: 68, name: "HDMI3".to_string(), volume: 0.5, muted: false, active: false },
            ],
            sources: vec![],
            sink_spinboxes: vec![
                Box::new(Spinbox::new(57, 0, 100, 1)),
                Box::new(Spinbox::new(50, 0, 100, 1)),
                Box::new(Spinbox::new(50, 0, 100, 1)),
                Box::new(Spinbox::new(50, 0, 100, 1)),
            ],
            source_spinboxes: vec![],
            sink_sliders: vec![
                Box::new(Slider::new()),
                Box::new(Slider::new()),
                Box::new(Slider::new()),
                Box::new(Slider::new()),
            ],
            source_sliders: vec![],
            sink_dragging: None,
            source_dragging: None,
        };
        let mut layout = AdaptiveGrid::new(260.0, 20.0);
        let pc = view(&mut state, 10.0, 20.0, 800.0, 600.0, &[false, false], &mut layout, &mut cce_ui::context::UiContext::new());
        for (i, (c, x, y, w, h, r, _)) in pc.rects.iter().enumerate() {
            println!("TEST_PC_RECT {}: color={:?}, x={}, y={}, w={}, h={}, r={}", i, c, x, y, w, h, r);
        }
        for (i, (t, sz, x, y, c, _, _)) in pc.texts.iter().enumerate() {
            println!("TEST_PC_TEXT {}: text='{}', size={}, x={}, y={}, color={:?}", i, t, sz, x, y, c);
        }
        assert!(!pc.rects.is_empty() || !pc.texts.is_empty());
    }

    #[test]
    #[allow(unused_assignments)]
    fn test_boxed_spinbox_right_click_crash() {
        use cce_ui::widget::{WidgetHost, Spinbox};
        let mut state = AudioState::default();
        state.sink_spinboxes.push(Box::new(Spinbox::new(50, 0, 100, 1)));
        let mut ctx = cce_ui::context::UiContext::new();
        let sb = &mut state.sink_spinboxes[0];
        sb.set_rect(0.0, 0.0, 100.0, 44.0);
        let res = sb.mouse_input(
            cce_ui::widget::MouseButton::Right,
            cce_ui::widget::ElementState::Pressed,
            50.0,
            20.0,
            &mut ctx,
        );
        assert!(res);
        assert!(cce_ui::widget::context_menu::is_visible());

        // Now replace the state simulating config reload/refresh
        let new_state = AudioState::default();
        state = new_state;

        // Assert that the context menu is hidden (cleared)
        assert!(!cce_ui::widget::context_menu::is_visible());
    }
}


