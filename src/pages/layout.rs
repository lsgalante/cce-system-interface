use std::fs;
use std::io::Write;

use crate::app::{AppAction, PageContent};

const CONFIG_PATH: &str = "/home/lsgalante/.config/clearwm/config.toml";
const CLEARWM_SOCK: &str = "/tmp/clearwm.sock";

#[derive(Debug, Clone)]
pub struct LayoutState {
    pub background_color: [u8; 3],
    pub border_color: [u8; 3],
    pub fullscreen_border_width: u16,
    pub cascade_border_width: u16,
    pub grid_border_width: u16,
    pub vsplit_border_width: u16,
    pub hsplit_border_width: u16,
    pub floating_border_width: u16,
    pub color_options: Vec<(&'static str, [u8; 3])>,
}

impl Default for LayoutState {
    fn default() -> Self {
        Self {
            background_color: [0x0a, 0x1a, 0x0e],
            border_color: [0x3e, 0x3e, 0x3e],
            fullscreen_border_width: 0,
            cascade_border_width: 6,
            grid_border_width: 6,
            vsplit_border_width: 6,
            hsplit_border_width: 6,
            floating_border_width: 6,
            color_options: preset_colors(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum LayoutMessage {
    SetBackground([u8; 3]),
    SetBorderColor([u8; 3]),
    FullscreenDown, FullscreenUp,
    CascadeDown, CascadeUp,
    GridDown, GridUp,
    VsplitDown, VsplitUp,
    HsplitDown, HsplitUp,
    FloatingDown, FloatingUp,
    Refreshed(LayoutState),
}

fn preset_colors() -> Vec<(&'static str, [u8; 3])> {
    vec![
        ("Black",       [0x00, 0x00, 0x00]),
        ("Dark Gray",   [0x1a, 0x1a, 0x2e]),
        ("Slate",       [0x2d, 0x2d, 0x3d]),
        ("Dark Forest", [0x0a, 0x1a, 0x0e]),
        ("Forest",      [0x1a, 0x2a, 0x1c]),
        ("Dark Teal",   [0x0a, 0x1a, 0x1e]),
        ("Navy",        [0x0a, 0x0f, 0x2e]),
        ("Dark Wine",   [0x1e, 0x0a, 0x14]),
        ("Dark Brown",  [0x1e, 0x16, 0x0e]),
        ("Charcoal",    [0x22, 0x22, 0x22]),
        ("Midnight",    [0x10, 0x10, 0x20]),
        ("Deep Sea",    [0x06, 0x14, 0x1e]),
    ]
}

pub fn read_layout_config() -> LayoutState {
    let content = fs::read_to_string(CONFIG_PATH).unwrap_or_default();
    LayoutState {
        background_color: parse_color_from_key(&content, "background_color", [0x0a, 0x1a, 0x0e]),
        border_color: parse_color_from_key(&content, "border_color", [0x3e, 0x3e, 0x3e]),
        fullscreen_border_width: parse_u16_from(&content, "fullscreen_border_width", 0),
        cascade_border_width: parse_u16_from(&content, "cascade_border_width", 6),
        grid_border_width: parse_u16_from(&content, "grid_border_width", 6),
        vsplit_border_width: parse_u16_from(&content, "vsplit_border_width", 6),
        hsplit_border_width: parse_u16_from(&content, "hsplit_border_width", 6),
        floating_border_width: parse_u16_from(&content, "floating_border_width", 6),
        color_options: preset_colors(),
    }
}

fn parse_color_from_key(content: &str, key: &str, default: [u8; 3]) -> [u8; 3] {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(key) {
            let rest = rest.trim_start_matches(|c: char| c == ' ' || c == '=' || c == '"');
            let hex = rest.trim_end_matches('"').trim();
            return parse_hex(hex);
        }
    }
    default
}

fn parse_u16_from(content: &str, key: &str, default: u16) -> u16 {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(key) {
            let rest = rest.trim_start_matches(|c: char| c == ' ' || c == '=' || c == '"');
            return rest.trim_end_matches('"').trim().parse::<u16>().unwrap_or(default);
        }
    }
    default
}

fn parse_hex(s: &str) -> [u8; 3] {
    let s = s.trim_start_matches('#');
    if s.len() >= 6 {
        let r = u8::from_str_radix(&s[0..2], 16).unwrap_or(0x0a);
        let g = u8::from_str_radix(&s[2..4], 16).unwrap_or(0x1a);
        let b = u8::from_str_radix(&s[4..6], 16).unwrap_or(0x0e);
        [r, g, b]
    } else { [0x0a, 0x1a, 0x0e] }
}

fn write_config_value(key: &str, value: &str) -> bool {
    let content = fs::read_to_string(CONFIG_PATH).unwrap_or_default();
    let new_line = format!("{} = {}", key, value);
    let mut found = false;
    let updated: String = content.lines()
        .map(|line| {
            if line.trim().starts_with(key) { found = true; new_line.clone() }
            else { line.to_string() }
        }).collect::<Vec<_>>().join("\n");
    if !found {
        let mut result = String::new();
        let mut in_layout = false;
        let mut inserted = false;
        for line in updated.lines() {
            if line.trim() == "[layout]" { in_layout = true; }
            else if line.trim().starts_with('[') && in_layout {
                if !inserted { result.push_str(&new_line); result.push('\n'); inserted = true; }
                in_layout = false;
            }
            result.push_str(line); result.push('\n');
        }
        if in_layout && !inserted { result.push_str(&new_line); result.push('\n'); }
        fs::write(CONFIG_PATH, result).is_ok()
    } else { fs::write(CONFIG_PATH, updated).is_ok() }
}

fn send_ipc_command(cmd: &str) {
    if let Ok(mut stream) = std::os::unix::net::UnixStream::connect(CLEARWM_SOCK) {
        let _ = stream.write_all(format!("{}\n", cmd).as_bytes());
    }
}

fn apply_background(rgb: [u8; 3]) {
    let _ = std::process::Command::new("pkill").args(["-x", "swaybg"]).status();
    std::thread::sleep(std::time::Duration::from_millis(100));
    let hex = format!("{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2]);
    let _ = std::process::Command::new("swaybg").arg("-c").arg(&hex).spawn();
    write_config_value("background_color", &format!("\"#{}\"", hex));
    send_ipc_command(&format!("layout background_color #{}", hex));
}

fn apply_border_color(rgb: [u8; 3]) {
    let hex = format!("\"#{:02x}{:02x}{:02x}\"", rgb[0], rgb[1], rgb[2]);
    write_config_value("border_color", &hex);
    send_ipc_command(&format!("layout border_color #{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2]));
}

fn apply_all_widths(s: &LayoutState) {
    let w = |k: &str, v: u16| { write_config_value(k, &v.to_string()); send_ipc_command(&format!("layout {} {}", k, v)); };
    w("fullscreen_border_width", s.fullscreen_border_width);
    w("cascade_border_width", s.cascade_border_width);
    w("grid_border_width", s.grid_border_width);
    w("vsplit_border_width", s.vsplit_border_width);
    w("hsplit_border_width", s.hsplit_border_width);
    w("floating_border_width", s.floating_border_width);
}

fn rgb_float(c: [u8; 3]) -> [f32; 4] {
    [c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0, 1.0]
}

const TEXT_FG: [f32; 4] = [0.83, 0.83, 0.83, 1.0];
const TEXT_DIM: [f32; 4] = [0.53, 0.53, 0.60, 1.0];
const ACCENT: [f32; 4] = [0.36, 0.56, 0.38, 1.0];
const BTN_ACTIVE: [f32; 4] = [0.20, 0.40, 0.22, 1.0];
const BTN_INACTIVE: [f32; 4] = [0.13, 0.18, 0.14, 1.0];
const BTN_HOVER: [f32; 4] = [0.25, 0.30, 0.26, 1.0];
const SECTION_BORDER: [f32; 4] = [0.18, 0.18, 0.27, 1.0];
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

pub fn view(state: &LayoutState, cx: f32, cy: f32, cw: f32, _ch: f32) -> PageContent {
    let mut pc = PageContent::new();
    let mut y = cy + 12.0;

    // ── Background Color ──
    pc.text("Desktop Background", cx + 12.0, y, 14.0, TEXT_FG);
    y += 22.0;

    let hex = format!("#{:02x}{:02x}{:02x}", state.background_color[0], state.background_color[1], state.background_color[2]);
    pc.text(&format!("Current: {}", hex), cx + 14.0, y, 12.0, ACCENT);
    y += 18.0;

    // Color swatch grid
    let swatch_w = 48.0;
    let swatch_h = 24.0;
    let gap = 6.0;
    let cols = 4usize;
    let total_w = cols as f32 * (swatch_w + gap);
    let start_x = cx + (cw - total_w) / 2.0;

    for (i, (name, rgb)) in state.color_options.iter().enumerate() {
        let col = i % cols;
        let row = i / cols;
        let sx = start_x + col as f32 * (swatch_w + gap);
        let sy = y + row as f32 * (swatch_h + 16.0);
        let is_selected = *rgb == state.background_color;
        let border = if is_selected { ACCENT } else { [0.24, 0.24, 0.24, 1.0] };

        pc.rect(rgb_float(*rgb), sx, sy, swatch_w, swatch_h);
        if is_selected {
            pc.rect(border, sx - 1.0, sy - 1.0, swatch_w + 2.0, swatch_h + 2.0);
        }
        pc.text(name, sx + swatch_w / 2.0 - name.len() as f32 * 2.5, sy + swatch_h + 2.0, 8.0, TEXT_DIM);
        // Clickable over the swatch
        let action = AppAction::Layout(LayoutMessage::SetBackground(*rgb));
        pc.button("", sx, sy, swatch_w, swatch_h,
            [0.0, 0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 0.0], action);
    }
    let swatch_rows = (state.color_options.len() + cols - 1) / cols;
    y += swatch_rows as f32 * (swatch_h + 16.0) + 8.0;

    // ── Border Color ──
    pc.rect(SECTION_BORDER, cx + 8.0, y, cw - 16.0, 1.0);
    y += 8.0;
    pc.text("Border Color", cx + 12.0, y, 14.0, TEXT_FG);
    y += 22.0;

    for (i, (name, rgb)) in state.color_options.iter().enumerate() {
        let col = i % cols;
        let row = i / cols;
        let sx = start_x + col as f32 * (swatch_w + gap);
        let sy = y + row as f32 * (swatch_h + 16.0);
        let is_selected = *rgb == state.border_color;
        let border = if is_selected { ACCENT } else { [0.24, 0.24, 0.24, 1.0] };

        pc.rect(rgb_float(*rgb), sx, sy, swatch_w, swatch_h);
        if is_selected {
            pc.rect(border, sx - 1.0, sy - 1.0, swatch_w + 2.0, swatch_h + 2.0);
        }
        pc.text(name, sx + swatch_w / 2.0 - name.len() as f32 * 2.5, sy + swatch_h + 2.0, 8.0, TEXT_DIM);
        let action = AppAction::Layout(LayoutMessage::SetBorderColor(*rgb));
        pc.button("", sx, sy, swatch_w, swatch_h,
            [0.0, 0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 0.0], action);
    }
    y += swatch_rows as f32 * (swatch_h + 16.0) + 8.0;

    // ── Border Widths ──
    pc.rect(SECTION_BORDER, cx + 8.0, y, cw - 16.0, 1.0);
    y += 8.0;
    pc.text("Border Width", cx + 12.0, y, 14.0, TEXT_FG);
    y += 22.0;

    let widths: [(&str, u16, AppAction, AppAction); 6] = [
        ("Fullscreen", state.fullscreen_border_width,
         AppAction::Layout(LayoutMessage::FullscreenDown), AppAction::Layout(LayoutMessage::FullscreenUp)),
        ("Cascade", state.cascade_border_width,
         AppAction::Layout(LayoutMessage::CascadeDown), AppAction::Layout(LayoutMessage::CascadeUp)),
        ("Grid", state.grid_border_width,
         AppAction::Layout(LayoutMessage::GridDown), AppAction::Layout(LayoutMessage::GridUp)),
        ("Vsplit", state.vsplit_border_width,
         AppAction::Layout(LayoutMessage::VsplitDown), AppAction::Layout(LayoutMessage::VsplitUp)),
        ("Hsplit", state.hsplit_border_width,
         AppAction::Layout(LayoutMessage::HsplitDown), AppAction::Layout(LayoutMessage::HsplitUp)),
        ("Floating", state.floating_border_width,
         AppAction::Layout(LayoutMessage::FloatingDown), AppAction::Layout(LayoutMessage::FloatingUp)),
    ];

    for (name, val, down, up) in &widths {
        pc.text(name, cx + 14.0, y + 6.0, 12.0, TEXT_DIM);
        pc.button("-1", cx + cw - 100.0, y, 30.0, 26.0,
            BTN_INACTIVE, BTN_HOVER, WHITE, down.clone());
        pc.text(&format!(" {}px ", val), cx + cw - 66.0, y + 6.0, 13.0, TEXT_FG);
        pc.button("+1", cx + cw - 38.0, y, 30.0, 26.0,
            BTN_ACTIVE, BTN_HOVER, WHITE, up.clone());
        y += 30.0;
    }

    pc
}

pub fn update(state: &mut LayoutState, msg: LayoutMessage) {
    let dec = |v: &mut u16| { if *v > 0 { *v -= 1; } };
    let inc = |v: &mut u16| { if *v < 100 { *v += 1; } };
    match msg {
        LayoutMessage::SetBackground(rgb) => {
            state.background_color = rgb;
            apply_background(rgb);
        }
        LayoutMessage::SetBorderColor(rgb) => {
            state.border_color = rgb;
            apply_border_color(rgb);
        }
        LayoutMessage::FullscreenDown => { dec(&mut state.fullscreen_border_width); apply_all_widths(state); }
        LayoutMessage::FullscreenUp => { inc(&mut state.fullscreen_border_width); apply_all_widths(state); }
        LayoutMessage::CascadeDown => { dec(&mut state.cascade_border_width); apply_all_widths(state); }
        LayoutMessage::CascadeUp => { inc(&mut state.cascade_border_width); apply_all_widths(state); }
        LayoutMessage::GridDown => { dec(&mut state.grid_border_width); apply_all_widths(state); }
        LayoutMessage::GridUp => { inc(&mut state.grid_border_width); apply_all_widths(state); }
        LayoutMessage::VsplitDown => { dec(&mut state.vsplit_border_width); apply_all_widths(state); }
        LayoutMessage::VsplitUp => { inc(&mut state.vsplit_border_width); apply_all_widths(state); }
        LayoutMessage::HsplitDown => { dec(&mut state.hsplit_border_width); apply_all_widths(state); }
        LayoutMessage::HsplitUp => { inc(&mut state.hsplit_border_width); apply_all_widths(state); }
        LayoutMessage::FloatingDown => { dec(&mut state.floating_border_width); apply_all_widths(state); }
        LayoutMessage::FloatingUp => { inc(&mut state.floating_border_width); apply_all_widths(state); }
        LayoutMessage::Refreshed(new) => { *state = new; }
    }
}
