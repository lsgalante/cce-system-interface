use crate::app::{AppAction, PageContent, SectionContextExt, section_divider};
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
#[allow(dead_code)]
const BLANK_BAR: [f32; 4] = [0.15, 0.15, 0.24, 1.0];
#[allow(dead_code)]
const FILL_BAR: [f32; 4] = [0.30, 0.50, 0.32, 1.0];
#[allow(dead_code)]
const RED: [f32; 4] = [1.0, 0.33, 0.33, 1.0];

const HEADING: [f32; 4] = [0.35, 0.65, 0.90, 1.0];
const BTN_NEUTRAL: ([f32; 4], [f32; 4]) = ([0.15, 0.15, 0.20, 1.0], [0.22, 0.22, 0.28, 1.0]);
const BTN_DANGER: ([f32; 4], [f32; 4]) = ([0.25, 0.14, 0.14, 1.0], [0.40, 0.20, 0.20, 1.0]);
const TEXT_BTN: [f32; 4] = [0.90, 0.90, 0.95, 1.0];
const TEXT_DANGER: [f32; 4] = [0.95, 0.55, 0.55, 1.0];

/// One device on one line: name, volume slider, spinbox, mute — or a dim
/// "inactive" note. The widgets stay index-aligned with the device vecs.
#[allow(clippy::too_many_arguments)]
fn device_row(
    stack: &mut cce_ui::layout::VStack<'_, '_, PageContent>,
    name: &str,
    active: bool,
    muted: bool,
    volume: f32,
    slider: &mut cce_ui::widget::Adapted<Slider>,
    spin: &mut cce_ui::widget::Adapted<Spinbox>,
    mute_action: AppAction,
    ctx: &mut cce_ui::context::UiContext,
) {
    if !active {
        let sc = &mut *stack.context;
        let mut y = sc.content_y;
        if y > sc.content_start_y {
            y += sc.row_gap;
        }
        let lx = sc.ax(12.0);
        sc.pc.text(name, lx, y, 12.0, TEXT_DIM);
        sc.pc.text("inactive", lx + 150.0, y, 12.0, TEXT_DIM);
        sc.content_y = y + 18.0;
        for h in &mut sc.grid.col_heights {
            *h = sc.content_y;
        }
        return;
    }

    slider.set_value(volume);
    spin.value = (volume * 100.0).round() as i32;

    let sb_h = cce_ui::layout::spinbox_height();
    let sl_h = cce_ui::layout::slider_height();
    let row_h = sb_h.max(sl_h).max(22.0);
    let (mute_label, colors, mute_text) = if muted {
        ("Unmute", BTN_DANGER, TEXT_DANGER)
    } else {
        ("Mute", BTN_NEUTRAL, TEXT_BTN)
    };

    stack.add_row(1, 0.0, row_h, |c, _, x, w| {
        let name_w = 150.0;
        let spin_w = 90.0;
        let mute_w = 80.0;
        let gap = 8.0;
        let slider_w = (w - name_w - spin_w - mute_w - 2.0 * gap).max(60.0);
        let y = c.ay();

        c.pc.text(name, x, y + (row_h - 14.0) / 2.0, 12.0, TEXT_FG);
        render_widget(c.pc, slider, x + name_w, y + (row_h - sl_h) / 2.0, slider_w, sl_h, ctx);
        let spin_x = x + name_w + slider_w + gap;
        spin.set_row_rect(spin_x, spin_w);
        render_widget(c.pc, spin, spin_x, y + (row_h - sb_h) / 2.0, spin_w, sb_h, ctx);
        c.button(
            mute_label,
            spin_x + spin_w + gap,
            y + (row_h - 22.0) / 2.0,
            mute_w,
            22.0,
            colors.0,
            colors.1,
            mute_text,
            mute_action.clone(),
        );
    });
}

pub fn view(state: &mut AudioState, cx: f32, cy: f32, cw: f32, ch: f32, sec_focused: &[bool], layout: &mut dyn LayoutStrategy, ctx: &mut cce_ui::context::UiContext) -> PageContent {
    let mut final_pc = PageContent::new();
    let sec_w = 320.0f32;
    let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(1);

    builder.add_section_spanned(&mut final_pc, "", 1, sec_focused.first().copied().unwrap_or(false), |sec| {
        if !state.loaded {
            sec.text("Loading audio devices...", 12.0, 0.0, 12.0, TEXT_DIM);
            return;
        }
        let mut stack = sec.vstack(cce_ui::layout::plate_gap());

        stack.context.text("Output", 12.0, 0.0, 14.0, HEADING);
        if state.sinks.is_empty() {
            stack.context.text("No output devices found", 12.0, 0.0, 12.0, TEXT_DIM);
        }
        let sinks = state.sinks.clone();
        for (idx, sink) in sinks.iter().enumerate() {
            device_row(
                &mut stack,
                &sink.name,
                sink.active,
                sink.muted,
                sink.volume,
                &mut state.sink_sliders[idx],
                &mut state.sink_spinboxes[idx],
                AppAction::Audio(AudioMessage::SinkMute(sink.id)),
                ctx,
            );
        }

        section_divider(stack.context);

        stack.context.text("Input", 12.0, 0.0, 14.0, HEADING);
        if state.sources.is_empty() {
            stack.context.text("No input devices found", 12.0, 0.0, 12.0, TEXT_DIM);
        }
        let sources = state.sources.clone();
        for (idx, src) in sources.iter().enumerate() {
            device_row(
                &mut stack,
                &src.name,
                src.active,
                src.muted,
                src.volume,
                &mut state.source_sliders[idx],
                &mut state.source_spinboxes[idx],
                AppAction::Audio(AudioMessage::SourceMute(src.id)),
                ctx,
            );
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
    fn section_widgets(&mut self) -> Vec<Vec<cce_ui::widget::WidgetId>> {
        let mut output: Vec<cce_ui::widget::WidgetId> = Vec::new();
        for (i, sink) in self.sinks.iter().enumerate() {
            if sink.active {
                if let Some(sb) = self.sink_spinboxes.get_mut(i) {
                    output.push(sb.id());
                }
                if let Some(sl) = self.sink_sliders.get_mut(i) {
                    output.push(sl.id());
                }
            }
        }
        let mut input: Vec<cce_ui::widget::WidgetId> = Vec::new();
        for (i, src) in self.sources.iter().enumerate() {
            if src.active {
                if let Some(sb) = self.source_spinboxes.get_mut(i) {
                    input.push(sb.id());
                }
                if let Some(sl) = self.source_sliders.get_mut(i) {
                    input.push(sl.id());
                }
            }
        }
        output.extend(input);
        vec![output]
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
                // Keep the paired spinbox display in step, as the old drag path did.
                if let Some(sb) = self.sink_spinboxes.get_mut(i) {
                    sb.value = slider.value();
                }
                actions.push(AppAction::Audio(AudioMessage::SinkVolume(id, slider.value() as f32 / 100.0)));
            }
        }
        for (i, slider) in self.source_sliders.iter_mut().enumerate() {
            if slider.take_change() {
                let id = self.sources[i].id;
                if let Some(sb) = self.source_spinboxes.get_mut(i) {
                    sb.value = slider.value();
                }
                actions.push(AppAction::Audio(AudioMessage::SourceVolume(id, slider.value() as f32 / 100.0)));
            }
        }
    }

    // The pointer drag hooks are GONE (6bd routed events): slider drags ride the
    // router's drag-target machinery — presses were already routed through the section
    // roots, and DragUpdate/DragEnd now reach the sliders the same way. Value changes
    // surface through the take_change drain above.
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
        };
        let mut layout = AdaptiveGrid::new(260.0, 20.0);
        // One flag: section_widgets() returns a single group (the output ids
        // with input appended), so the view draws one section and reads [0].
        let pc = view(&mut state, 10.0, 20.0, 800.0, 600.0, &[false], &mut layout, &mut cce_ui::context::UiContext::new());
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


