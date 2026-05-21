use std::fs;
use std::io::Write;

use crate::app::PageContent;
use clear_ui::layout::Section;
use clear_ui::widget::{ColorSelector, Spinbox};

const CONFIG_PATH: &str = "/home/lsgalante/.config/clearwm/config.toml";
const CLEARWM_SOCK: &str = "/tmp/clearwm.sock";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidthParam {
    Fullscreen, Cascade, Grid, Vsplit, Hsplit, Floating,
}

impl WidthParam {
    pub const ALL: [WidthParam; 6] = [
        WidthParam::Fullscreen, WidthParam::Cascade, WidthParam::Grid,
        WidthParam::Vsplit, WidthParam::Hsplit, WidthParam::Floating,
    ];
    pub fn key(self) -> &'static str {
        match self {
            WidthParam::Fullscreen => "fullscreen_border_width",
            WidthParam::Cascade => "cascade_border_width",
            WidthParam::Grid => "grid_border_width",
            WidthParam::Vsplit => "vsplit_border_width",
            WidthParam::Hsplit => "hsplit_border_width",
            WidthParam::Floating => "floating_border_width",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            WidthParam::Fullscreen => "Fullscreen",
            WidthParam::Cascade => "Cascade",
            WidthParam::Grid => "Grid",
            WidthParam::Vsplit => "Vsplit",
            WidthParam::Hsplit => "Hsplit",
            WidthParam::Floating => "Floating",
        }
    }
}

fn make_spinboxes(fs: u16, ca: u16, g: u16, v: u16, h: u16, fl: u16) -> Vec<Spinbox> {
    vec![
        Spinbox::new(fs as i32, 0, 100, 1),
        Spinbox::new(ca as i32, 0, 100, 1),
        Spinbox::new(g as i32, 0, 100, 1),
        Spinbox::new(v as i32, 0, 100, 1),
        Spinbox::new(h as i32, 0, 100, 1),
        Spinbox::new(fl as i32, 0, 100, 1),
    ]
}

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
    pub spinboxes: Vec<Spinbox>,
    pub color_selectors: Vec<ColorSelector>,
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
            spinboxes: make_spinboxes(0, 6, 6, 6, 6, 6),
            color_selectors: vec![
                ColorSelector::new([0x0a, 0x1a, 0x0e]).with_label("Desktop Background"),
                ColorSelector::new([0x3e, 0x3e, 0x3e]).with_label("Border Color"),
            ],
        }
    }
}

#[derive(Debug, Clone)]
pub enum LayoutMessage {
    SetBackground([u8; 3]),
    SetBorderColor([u8; 3]),
    PickBackgroundColor,
    PickBorderColor,
    SetWidth(WidthParam, u16),
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
    let fs = parse_u16_from(&content, "fullscreen_border_width", 0);
    let ca = parse_u16_from(&content, "cascade_border_width", 6);
    let g = parse_u16_from(&content, "grid_border_width", 6);
    let v = parse_u16_from(&content, "vsplit_border_width", 6);
    let h = parse_u16_from(&content, "hsplit_border_width", 6);
    let fl = parse_u16_from(&content, "floating_border_width", 6);
    LayoutState {
        background_color: parse_color_from_key(&content, "background_color", [0x0a, 0x1a, 0x0e]),
        border_color: parse_color_from_key(&content, "border_color", [0x3e, 0x3e, 0x3e]),
        fullscreen_border_width: fs,
        cascade_border_width: ca,
        grid_border_width: g,
        vsplit_border_width: v,
        hsplit_border_width: h,
        floating_border_width: fl,
        color_options: preset_colors(),
        spinboxes: make_spinboxes(fs, ca, g, v, h, fl),
        color_selectors: vec![
            ColorSelector::new(parse_color_from_key(&content, "background_color", [0x0a, 0x1a, 0x0e]))
                .with_label("Desktop Background"),
            ColorSelector::new(parse_color_from_key(&content, "border_color", [0x3e, 0x3e, 0x3e]))
                .with_label("Border Color"),
        ],
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

pub fn view(state: &mut LayoutState, cx: f32, cy: f32, cw: f32, _ch: f32) -> PageContent {
    let mut pc = PageContent::new();
    let mut y = cy + 12.0;

    let mut sec = Section::new(&mut pc, cx, y, cw, "Desktop Background");
    state.color_selectors[0].color = state.background_color;
    sec.widget(&mut pc, &mut state.color_selectors[0], 12.0, 220.0, 22.0);
    y = sec.finish(&mut pc);

    let mut sec = Section::new(&mut pc, cx, y, cw, "Border Color");
    state.color_selectors[1].color = state.border_color;
    sec.widget(&mut pc, &mut state.color_selectors[1], 12.0, 220.0, 22.0);
    y = sec.finish(&mut pc);

    let mut sec = Section::new(&mut pc, cx, y, cw, "Border Width");
    sec.spacing(8.0);
    for (i, param) in WidthParam::ALL.iter().enumerate() {
        state.spinboxes[i].set_label(param.label());
        sec.widget(&mut pc, &mut state.spinboxes[i], 14.0, 200.0, 26.0);
        sec.spacing(8.0);
    }
    sec.finish(&mut pc);

    pc
}

fn set_width(state: &mut LayoutState, param: WidthParam, val: u16) {
    let val = val.min(100);
    match param {
        WidthParam::Fullscreen => state.fullscreen_border_width = val,
        WidthParam::Cascade => state.cascade_border_width = val,
        WidthParam::Grid => state.grid_border_width = val,
        WidthParam::Vsplit => state.vsplit_border_width = val,
        WidthParam::Hsplit => state.hsplit_border_width = val,
        WidthParam::Floating => state.floating_border_width = val,
    }
    state.spinboxes[param_idx(param)].value = val as i32;
    apply_all_widths(state);
}

fn param_idx(p: WidthParam) -> usize {
    match p {
        WidthParam::Fullscreen => 0,
        WidthParam::Cascade => 1,
        WidthParam::Grid => 2,
        WidthParam::Vsplit => 3,
        WidthParam::Hsplit => 4,
        WidthParam::Floating => 5,
    }
}

pub fn update(state: &mut LayoutState, msg: LayoutMessage) {
    match msg {
        LayoutMessage::SetBackground(rgb) => {
            state.background_color = rgb;
            apply_background(rgb);
        }
        LayoutMessage::SetBorderColor(rgb) => {
            state.border_color = rgb;
            apply_border_color(rgb);
        }
        LayoutMessage::PickBackgroundColor | LayoutMessage::PickBorderColor => {}
        LayoutMessage::SetWidth(p, v) => set_width(state, p, v),
        LayoutMessage::Refreshed(new) => { *state = new; }
    }
}
