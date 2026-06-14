use crate::app::{AppAction, PageContent, SectionContextExt};
use clear_ui::layout::{render_widget, PageLayoutBuilder, LayoutStrategy};
use clear_ui::widget::{Spinbox, Slider, Element};

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
    pub sink_spinboxes: Vec<Spinbox>,
    pub source_spinboxes: Vec<Spinbox>,
    pub sink_sliders: Vec<Slider>,
    pub source_sliders: Vec<Slider>,
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

const TEXT_FG: [f32; 4] = [0.83, 0.83, 0.83, 1.0];
const TEXT_DIM: [f32; 4] = [0.53, 0.53, 0.60, 1.0];
const MUTED_BG: [f32; 4] = [0.33, 0.20, 0.20, 1.0];
const BTN_INACTIVE: [f32; 4] = [0.13, 0.18, 0.14, 1.0];
const BTN_HOVER: [f32; 4] = [0.25, 0.30, 0.26, 1.0];
const BLANK_BAR: [f32; 4] = [0.15, 0.15, 0.24, 1.0];
const FILL_BAR: [f32; 4] = [0.30, 0.50, 0.32, 1.0];
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const RED: [f32; 4] = [1.0, 0.33, 0.33, 1.0];

pub fn view(state: &mut AudioState, cx: f32, cy: f32, cw: f32, ch: f32, sec_focused: &[bool], layout: &mut dyn LayoutStrategy, ctx: &mut clear_ui::context::UiContext) -> PageContent {
    let mut final_pc = PageContent::new();
    let sec_w = 320.0f32;
    let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(2);

    // ── Output section ──
    builder.add_section(&mut final_pc, "Output", sec_focused.first().copied().unwrap_or(false), |sec| {
        if !state.loaded {
            sec.text("Loading output devices...", 12.0, 0.0, 12.0, TEXT_DIM);
            sec.spacing(18.0);
        } else if state.sinks.is_empty() {
            sec.text("No output devices found", 12.0, 0.0, 12.0, TEXT_DIM);
            sec.spacing(18.0);
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
                        subsec.spacing(8.0);
                        subsec.text("Device is inactive.", 12.0, 0.0, 12.0, TEXT_DIM);
                        subsec.spacing(8.0);
                    } else {
                        subsec.spacing(8.0);
                        let label = if sink.muted {
                            format!("Volume: {:.0}%  (muted)", sink.volume * 100.0)
                        } else {
                            format!("Volume: {:.0}%", sink.volume * 100.0)
                        };
                        state.sink_sliders[idx].set_label(&label);

                        let bar_w = subsec.cw - 2.0 * (subsec.padding() + 14.0);
                        let bar_x = 14.0;
                        let yt = subsec.ay();
                        state.sink_sliders[idx].set_value(sink.volume);
                        let slider_x = subsec.ax(bar_x);
                        
                        let label_h = clear_ui::widget::label_offset(&state.sink_sliders[idx]);
                        let slider_h = clear_ui::layout::slider_height() + label_h;
                        render_widget(subsec.pc, &mut state.sink_sliders[idx], slider_x, yt, bar_w, slider_h, ctx);

                        let row_y = subsec.ay() + slider_h + 8.0;
                        let sb_w = 100.0;
                        let sb_h = clear_ui::layout::spinbox_height();
                        let mute_w = 60.0;
                        let gap = 8.0;

                        let row_rect_x = subsec.ax(8.0);
                        state.sink_spinboxes[idx].value = (sink.volume * 100.0).round() as i32;
                        state.sink_spinboxes[idx].set_row_rect(row_rect_x, subsec.cw - 16.0);
                        let sb_x = subsec.ax(bar_x);
                        render_widget(subsec.pc, &mut state.sink_spinboxes[idx], sb_x, row_y, sb_w, sb_h, ctx);

                        let mute_label = if sink.muted { "Unmute" } else { "Mute" };
                        let mute_col = if sink.muted { MUTED_BG } else { BTN_INACTIVE };
                        let mute_btn_x = sb_x + sb_w + gap;
                        subsec.button(mute_label, mute_btn_x, row_y, mute_w, sb_h,
                            mute_col, BTN_HOVER, WHITE,
                            AppAction::Audio(AudioMessage::SinkMute(sink.id)));

                        subsec.content_y = row_y + sb_h - 4.0;
                    }
                });
            }
        }
    });

    // ── Input section ──
    builder.add_section(&mut final_pc, "Input", sec_focused.get(1).copied().unwrap_or(false), |sec| {
        if !state.loaded {
            sec.text("Loading input devices...", 12.0, 0.0, 12.0, TEXT_DIM);
            sec.spacing(18.0);
        } else if state.sources.is_empty() {
            sec.text("No input devices found", 12.0, 0.0, 12.0, TEXT_DIM);
            sec.spacing(18.0);
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
                        subsec.spacing(8.0);
                        subsec.text("Device is inactive.", 12.0, 0.0, 12.0, TEXT_DIM);
                        subsec.spacing(8.0);
                    } else {
                        subsec.spacing(8.0);
                        let label = if src.muted {
                            format!("Volume: {:.0}%  (muted)", src.volume * 100.0)
                        } else {
                            format!("Volume: {:.0}%", src.volume * 100.0)
                        };
                        state.source_sliders[idx].set_label(&label);

                        let bar_w = subsec.cw - 2.0 * (subsec.padding() + 14.0);
                        let bar_x = 14.0;
                        let yt = subsec.ay();
                        state.source_sliders[idx].set_value(src.volume);
                        let slider_x = subsec.ax(bar_x);
                        
                        let label_h = clear_ui::widget::label_offset(&state.source_sliders[idx]);
                        let slider_h = clear_ui::layout::slider_height() + label_h;
                        render_widget(subsec.pc, &mut state.source_sliders[idx], slider_x, yt, bar_w, slider_h, ctx);

                        let row_y = subsec.ay() + slider_h + 8.0;
                        let sb_w = 100.0;
                        let sb_h = clear_ui::layout::spinbox_height();
                        let mute_w = 60.0;
                        let gap = 8.0;

                        let row_rect_x = subsec.ax(8.0);
                        state.source_spinboxes[idx].value = (src.volume * 100.0).round() as i32;
                        state.source_spinboxes[idx].set_row_rect(row_rect_x, subsec.cw - 16.0);
                        let sb_x = subsec.ax(bar_x);
                        render_widget(subsec.pc, &mut state.source_spinboxes[idx], sb_x, row_y, sb_w, sb_h, ctx);

                        let mute_label = if src.muted { "Unmute" } else { "Mute" };
                        let mute_col = if src.muted { MUTED_BG } else { BTN_INACTIVE };
                        let mute_btn_x = sb_x + sb_w + gap;
                        subsec.button(mute_label, mute_btn_x, row_y, mute_w, sb_h,
                            mute_col, BTN_HOVER, WHITE,
                            AppAction::Audio(AudioMessage::SourceMute(src.id)));

                        subsec.content_y = row_y + sb_h - 4.0;
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
            *state = new;
            state.sink_spinboxes.resize_with(state.sinks.len(), || Spinbox::new(50, 0, 100, 1));
            state.source_spinboxes.resize_with(state.sources.len(), || Spinbox::new(50, 0, 100, 1));
            state.sink_sliders.resize_with(state.sinks.len(), || Slider::new().with_range(0.0, 1.0).with_scroll(true));
            state.source_sliders.resize_with(state.sources.len(), || Slider::new().with_range(0.0, 1.0).with_scroll(true));
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

#[cfg(test)]
mod tests {
    use super::*;
    use clear_ui::layout::GridLayout;

    #[test]
    fn test_view_layout_grid() {
        let mut state = AudioState::default();
        let mut layout = GridLayout::new(260.0, 20.0);
        let pc = view(&mut state, 10.0, 20.0, 800.0, 600.0, &[false, false], &mut layout, &mut clear_ui::context::UiContext::new());
        assert!(!pc.rects.is_empty() || !pc.texts.is_empty());
    }
}

