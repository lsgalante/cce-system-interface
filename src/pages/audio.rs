use crate::app::{AppAction, PageContent};

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
    pub sinks: Vec<AudioSink>,
    pub sources: Vec<AudioSource>,
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
    AudioState { sinks, sources }
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
const BTN_ACTIVE: [f32; 4] = [0.20, 0.40, 0.22, 1.0];
const BTN_INACTIVE: [f32; 4] = [0.13, 0.18, 0.14, 1.0];
const BTN_HOVER: [f32; 4] = [0.25, 0.30, 0.26, 1.0];
const BLANK_BAR: [f32; 4] = [0.15, 0.15, 0.24, 1.0];
const FILL_BAR: [f32; 4] = [0.30, 0.50, 0.32, 1.0];
const SECTION_BORDER: [f32; 4] = [0.18, 0.18, 0.27, 1.0];
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const RED: [f32; 4] = [1.0, 0.33, 0.33, 1.0];

pub fn view(state: &AudioState, cx: f32, cy: f32, cw: f32, _ch: f32) -> PageContent {
    let mut pc = PageContent::new();
    let mut y = cy + 12.0;

    // ── Output section ──
    pc.text("Output", cx + 12.0, y, 14.0, TEXT_FG);
    y += 22.0;

    if state.sinks.is_empty() {
        pc.text("No output devices found", cx + 12.0, y, 12.0, TEXT_DIM);
        y += 18.0;
    }

    for sink in &state.sinks {
        let label = if !sink.active {
            format!("{}  (inactive)", sink.name)
        } else if sink.muted {
            format!("{}  {:.0}%  (muted)", sink.name, sink.volume * 100.0)
        } else {
            format!("{}  {:.0}%", sink.name, sink.volume * 100.0)
        };
        let lc = if sink.muted { RED } else { TEXT_FG };
        pc.text(&label, cx + 14.0, y, 13.0, lc);
        y += 18.0;

        if sink.active {
            // Volume bar (visual only — we use +/- buttons)
            let bar_w = cw - 100.0;
            let bar_x = cx + 14.0;
            pc.rect(BLANK_BAR, bar_x, y, bar_w, 8.0);
            pc.rect(FILL_BAR, bar_x, y, bar_w * sink.volume, 8.0);
            pc.text(&format!("{:.0}%", sink.volume * 100.0), bar_x + bar_w + 8.0, y - 2.0, 11.0, TEXT_DIM);

            // Buttons
            let btn_h = 28.0;
            let btn_y = y + 12.0;
            let mut bx = cx + 14.0;

            pc.button("-10", bx, btn_y, 36.0, btn_h,
                BTN_INACTIVE, BTN_HOVER, WHITE,
                AppAction::Audio(AudioMessage::SinkVolume(sink.id, (sink.volume - 0.10).max(0.0))));
            bx += 40.0;

            let mute_label = if sink.muted { "Unmute" } else { "Mute" };
            let mute_bg = if sink.muted { MUTED_BG } else { BTN_INACTIVE };
            pc.button(mute_label, bx, btn_y, 52.0, btn_h,
                mute_bg, BTN_HOVER, WHITE,
                AppAction::Audio(AudioMessage::SinkMute(sink.id)));
            bx += 56.0;

            pc.button("+10", bx, btn_y, 36.0, btn_h,
                BTN_ACTIVE, BTN_HOVER, WHITE,
                AppAction::Audio(AudioMessage::SinkVolume(sink.id, (sink.volume + 0.10).min(1.0))));

            y = btn_y + btn_h + 6.0;
        } else {
            y += 6.0;
        }
    }

    // ── Input section ──
    y += 8.0;
    pc.rect(SECTION_BORDER, cx + 8.0, y, cw - 16.0, 1.0);
    y += 8.0;
    pc.text("Input", cx + 12.0, y, 14.0, TEXT_FG);
    y += 22.0;

    if state.sources.is_empty() {
        pc.text("No input devices found", cx + 12.0, y, 12.0, TEXT_DIM);
        y += 18.0;
    }

    for src in &state.sources {
        let label = if !src.active {
            format!("{}  (inactive)", src.name)
        } else if src.muted {
            format!("{}  {:.0}%  (muted)", src.name, src.volume * 100.0)
        } else {
            format!("{}  {:.0}%", src.name, src.volume * 100.0)
        };
        let lc = if src.muted { RED } else { TEXT_FG };
        pc.text(&label, cx + 14.0, y, 13.0, lc);
        y += 18.0;

        if src.active {
            let bar_w = cw - 100.0;
            let bar_x = cx + 14.0;
            pc.rect(BLANK_BAR, bar_x, y, bar_w, 8.0);
            pc.rect(FILL_BAR, bar_x, y, bar_w * src.volume, 8.0);
            pc.text(&format!("{:.0}%", src.volume * 100.0), bar_x + bar_w + 8.0, y - 2.0, 11.0, TEXT_DIM);

            let btn_h = 28.0;
            let btn_y = y + 12.0;
            let mut bx = cx + 14.0;

            pc.button("-10", bx, btn_y, 36.0, btn_h,
                BTN_INACTIVE, BTN_HOVER, WHITE,
                AppAction::Audio(AudioMessage::SourceVolume(src.id, (src.volume - 0.10).max(0.0))));
            bx += 40.0;

            let mute_label = if src.muted { "Unmute" } else { "Mute" };
            let mute_bg = if src.muted { MUTED_BG } else { BTN_INACTIVE };
            pc.button(mute_label, bx, btn_y, 52.0, btn_h,
                mute_bg, BTN_HOVER, WHITE,
                AppAction::Audio(AudioMessage::SourceMute(src.id)));
            bx += 56.0;

            pc.button("+10", bx, btn_y, 36.0, btn_h,
                BTN_ACTIVE, BTN_HOVER, WHITE,
                AppAction::Audio(AudioMessage::SourceVolume(src.id, (src.volume + 0.10).min(1.0))));

            y = btn_y + btn_h + 6.0;
        } else {
            y += 6.0;
        }
    }

    pc
}

pub fn update(state: &mut AudioState, msg: AudioMessage) {
    match msg {
        AudioMessage::Refreshed(new) => { *state = new; }
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
