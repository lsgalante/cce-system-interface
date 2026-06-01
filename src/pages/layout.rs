use std::fs;
use std::io::Write;

use crate::app::PageContent;
use clear_ui::layout::Section;
use clear_ui::widget::{Dropdown, Spinbox};

const CONFIG_PATH: &str = "/home/lsgalante/.config/ccec/config.toml";

fn get_socket_path() -> String {
    match std::env::var("WAYLAND_DISPLAY") {
        Ok(display) => format!("/tmp/ccec-{}.sock", display),
        Err(_) => "/tmp/ccec.sock".to_string(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidthParam {
    Fullscreen, Cascade, Grid, Floating,
}

impl WidthParam {
    pub const ALL: [WidthParam; 4] = [
        WidthParam::Fullscreen, WidthParam::Cascade, WidthParam::Grid,
        WidthParam::Floating,
    ];
    pub fn key(self) -> &'static str {
        match self {
            WidthParam::Fullscreen => "fullscreen_border_width",
            WidthParam::Cascade => "cascade_border_width",
            WidthParam::Grid => "grid_border_width",
            WidthParam::Floating => "floating_border_width",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            WidthParam::Fullscreen => "Fullscreen",
            WidthParam::Cascade => "Cascade",
            WidthParam::Grid => "Grid",
            WidthParam::Floating => "Floating",
        }
    }
}

fn make_spinboxes(fs: u16, ca: u16, g: u16, fl: u16) -> Vec<Spinbox> {
    vec![
        Spinbox::new(fs as i32, 0, 100, 1),
        Spinbox::new(ca as i32, 0, 100, 1),
        Spinbox::new(g as i32, 0, 100, 1),
        Spinbox::new(fl as i32, 0, 100, 1),
    ]
}

#[derive(Debug, Clone)]
pub struct LayoutState {
    pub fullscreen_border_width: u16,
    pub cascade_border_width: u16,
    pub grid_border_width: u16,
    pub floating_border_width: u16,
    pub cascade_offset: u16,
    pub edge_gap: u16,
    pub top_gap: u16,
    pub transition_duration: u16,
    pub spinboxes: Vec<Spinbox>,
    pub cascade_offset_spinbox: Spinbox,
    pub edge_gap_spinbox: Spinbox,
    pub top_gap_spinbox: Spinbox,
    pub transition_duration_spinbox: Spinbox,
    pub tag_layout_menus: Vec<Dropdown>,
}

impl Default for LayoutState {
    fn default() -> Self {
        Self {
            fullscreen_border_width: 0,
            cascade_border_width: 6,
            grid_border_width: 6,
            floating_border_width: 6,
            cascade_offset: 20,
            edge_gap: 48,
            top_gap: 48,
            transition_duration: 300,
            spinboxes: make_spinboxes(0, 6, 6, 6),
            cascade_offset_spinbox: Spinbox::new(20, 0, 200, 1),
            edge_gap_spinbox: Spinbox::new(48, 0, 200, 1),
            top_gap_spinbox: Spinbox::new(48, 0, 200, 1),
            transition_duration_spinbox: Spinbox::new(300, 0, 2000, 50),
            tag_layout_menus: (1..=4).map(|i| {
                Dropdown::new(
                    vec![
                        "Cascade".to_string(),
                        "Grid".to_string(),
                        "Fullscreen".to_string(),
                        "Floating".to_string(),
                        "Popup".to_string(),
                    ],
                    0
                ).with_label(&format!("Tag {}", i))
            }).collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum LayoutMessage {
    SetWidth(WidthParam, u16),
    SetCascadeOffset(u16),
    SetEdgeGap(u16),
    SetTopGap(u16),
    SetTransitionDuration(u16),
    SetTagLayout(usize, usize),
    Refreshed(LayoutState),
}

pub fn read_layout_config() -> LayoutState {
    let content = fs::read_to_string(CONFIG_PATH).unwrap_or_default();
    let fs = parse_u16_from(&content, "fullscreen_border_width", 0);
    let ca = parse_u16_from(&content, "cascade_border_width", 6);
    let g = parse_u16_from(&content, "grid_border_width", 6);
    let fl = parse_u16_from(&content, "floating_border_width", 6);
    let co = parse_u16_from(&content, "cascade_offset", 20);
    let gl = parse_u16_from(&content, "gap_left", 48);
    let gt = parse_u16_from(&content, "gap_top", 48);
    let td = parse_u16_from(&content, "transition_duration", 300);

    let tag_modes = parse_tag_layouts_from_config(&content);
    let dropdown_options = vec![
        "Cascade".to_string(),
        "Grid".to_string(),
        "Fullscreen".to_string(),
        "Floating".to_string(),
        "Popup".to_string(),
    ];
    let tag_layout_menus = (1..=4).map(|i| {
        let mode_str = &tag_modes[i - 1];
        let idx = dropdown_options.iter().position(|opt| opt.to_lowercase() == mode_str.to_lowercase()).unwrap_or(0);
        Dropdown::new(dropdown_options.clone(), idx).with_label(&format!("Tag {}", i))
    }).collect();

    LayoutState {
        fullscreen_border_width: fs,
        cascade_border_width: ca,
        grid_border_width: g,
        floating_border_width: fl,
        cascade_offset: co,
        edge_gap: gl,
        top_gap: gt,
        transition_duration: td,
        spinboxes: make_spinboxes(fs, ca, g, fl),
        cascade_offset_spinbox: Spinbox::new(co as i32, 0, 200, 1),
        edge_gap_spinbox: Spinbox::new(gl as i32, 0, 200, 1),
        top_gap_spinbox: Spinbox::new(gt as i32, 0, 200, 1),
        transition_duration_spinbox: Spinbox::new(td as i32, 0, 2000, 50),
        tag_layout_menus,
    }
}

fn parse_tag_layouts_from_config(content: &str) -> Vec<String> {
    let mut modes = vec!["cascade".to_string(); 4];
    let mut current_tag = None;
    let mut current_mode = None;
    
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[[tag_layout]]" {
            if let (Some(tag), Some(mode)) = (current_tag, current_mode.take()) {
                if tag >= 1 && tag <= 4 {
                    modes[tag - 1] = mode;
                }
            }
            current_tag = None;
            continue;
        }
        if trimmed.starts_with('[') && !trimmed.starts_with("[[") {
            if let (Some(tag), Some(mode)) = (current_tag, current_mode.take()) {
                if tag >= 1 && tag <= 4 {
                    modes[tag - 1] = mode;
                }
            }
            current_tag = None;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("tag") {
            let rest = rest.trim_start_matches(|c: char| c == ' ' || c == '=').trim();
            if let Ok(tag) = rest.parse::<usize>() {
                current_tag = Some(tag);
            }
        } else if let Some(rest) = trimmed.strip_prefix("mode") {
            let rest = rest.trim_start_matches(|c: char| c == ' ' || c == '=').trim().trim_matches('"').to_string();
            current_mode = Some(rest);
        }
    }
    if let (Some(tag), Some(mode)) = (current_tag, current_mode.take()) {
        if tag >= 1 && tag <= 4 {
            modes[tag - 1] = mode;
        }
    }
    modes
}

fn write_tag_layout(tag_num: usize, mode_str: &str) -> bool {
    let content = fs::read_to_string(CONFIG_PATH).unwrap_or_default();
    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
    
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim();
        if line == "[[tag_layout]]" {
            let mut tag_val = None;
            let mut mode_line_idx = None;
            
            let mut j = i + 1;
            while j < lines.len() {
                let next_line = lines[j].trim();
                if next_line.starts_with("[[") || (next_line.starts_with('[') && !next_line.starts_with("[[")) {
                    break;
                }
                if next_line.starts_with("tag") {
                    if let Some(val_str) = next_line.split('=').nth(1) {
                        if let Ok(v) = val_str.trim().parse::<usize>() {
                            tag_val = Some(v);
                        }
                    }
                } else if next_line.starts_with("mode") {
                    mode_line_idx = Some(j);
                }
                j += 1;
            }
            
            if tag_val == Some(tag_num) {
                if let Some(idx) = mode_line_idx {
                    lines[idx] = format!("mode = \"{}\"", mode_str.to_lowercase());
                    let result = lines.join("\n") + "\n";
                    return fs::write(CONFIG_PATH, result).is_ok();
                }
            }
            i = j;
        } else {
            i += 1;
        }
    }
    
    let mut result = lines.join("\n");
    if !result.ends_with('\n') {
        result.push('\n');
    }
    result.push_str(&format!("\n[[tag_layout]]\ntag = {}\nmode = \"{}\"\n", tag_num, mode_str.to_lowercase()));
    fs::write(CONFIG_PATH, result).is_ok()
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
    if let Ok(mut stream) = std::os::unix::net::UnixStream::connect(get_socket_path()) {
        let _ = stream.write_all(format!("{}\n", cmd).as_bytes());
    }
}

fn apply_all_widths(s: &LayoutState) {
    let w = |k: &str, v: u16| { write_config_value(k, &v.to_string()); send_ipc_command(&format!("layout {} {}", k, v)); };
    w("fullscreen_border_width", s.fullscreen_border_width);
    w("cascade_border_width", s.cascade_border_width);
    w("grid_border_width", s.grid_border_width);
    w("floating_border_width", s.floating_border_width);
    w("cascade_offset", s.cascade_offset);
    w("gap_left", s.edge_gap);
    w("gap_right", s.edge_gap);
    w("gap_bottom", s.edge_gap);
    w("gap_top", s.top_gap);
    w("transition_duration", s.transition_duration);
}

pub fn view(state: &mut LayoutState, cx: f32, cy: f32, cw: f32, _ch: f32, sec_focused: &[bool]) -> PageContent {
    let mut pc = PageContent::new();
    let mut y = cy + 12.0;

    // 1. Border Width Section
    let mut sec_bw = Section::new(&mut pc, cx, y, cw, "Border Width");
    sec_bw.spacing(8.0);
    for (i, param) in WidthParam::ALL.iter().enumerate() {
        state.spinboxes[i].set_label(param.label());
        sec_bw.widget(&mut pc, &mut state.spinboxes[i], 14.0, 200.0, 26.0);
        sec_bw.spacing(8.0);
    }
    y = sec_bw.finish_focused(&mut pc, sec_focused.get(0).copied().unwrap_or(false));

    // 2. Cascade Section
    let mut sec_cascade = Section::new(&mut pc, cx, y, cw, "Cascade");
    sec_cascade.spacing(8.0);
    state.cascade_offset_spinbox.set_label("Offset");
    sec_cascade.widget(&mut pc, &mut state.cascade_offset_spinbox, 14.0, 200.0, 26.0);
    sec_cascade.spacing(8.0);
    state.edge_gap_spinbox.set_label("Edge Gap");
    sec_cascade.widget(&mut pc, &mut state.edge_gap_spinbox, 14.0, 200.0, 26.0);
    sec_cascade.spacing(8.0);
    state.top_gap_spinbox.set_label("Top Gap");
    sec_cascade.widget(&mut pc, &mut state.top_gap_spinbox, 14.0, 200.0, 26.0);
    sec_cascade.spacing(8.0);
    y = sec_cascade.finish_focused(&mut pc, sec_focused.get(1).copied().unwrap_or(false));

    // 3. Movement Section
    let mut movement_sec = Section::new(&mut pc, cx, y, cw, "Movement");
    movement_sec.spacing(8.0);
    state.transition_duration_spinbox.set_label("Duration (ms)");
    movement_sec.widget(&mut pc, &mut state.transition_duration_spinbox, 14.0, 200.0, 26.0);
    movement_sec.spacing(8.0);
    y = movement_sec.finish_focused(&mut pc, sec_focused.get(2).copied().unwrap_or(false));

    // 4. Default Layouts Section
    let mut default_layouts_sec = Section::new(&mut pc, cx, y, cw, "Default Layouts");
    default_layouts_sec.spacing(8.0);
    for i in 0..4 {
        default_layouts_sec.widget(&mut pc, &mut state.tag_layout_menus[i], 14.0, 200.0, 26.0);
        default_layouts_sec.spacing(8.0);
    }
    default_layouts_sec.finish_focused(&mut pc, sec_focused.get(3).copied().unwrap_or(false));

    for menu in &mut state.tag_layout_menus {
        menu.render_popover(&mut pc);
    }

    pc
}

fn set_width(state: &mut LayoutState, param: WidthParam, val: u16) {
    let val = val.min(100);
    match param {
        WidthParam::Fullscreen => state.fullscreen_border_width = val,
        WidthParam::Cascade => state.cascade_border_width = val,
        WidthParam::Grid => state.grid_border_width = val,
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
        WidthParam::Floating => 3,
    }
}

pub fn update(state: &mut LayoutState, msg: LayoutMessage) {
    match msg {
        LayoutMessage::SetWidth(p, v) => set_width(state, p, v),
        LayoutMessage::SetCascadeOffset(v) => {
            let val = v.min(200);
            state.cascade_offset = val;
            state.cascade_offset_spinbox.value = val as i32;
            apply_all_widths(state);
        }
        LayoutMessage::SetEdgeGap(v) => {
            let val = v.min(200);
            state.edge_gap = val;
            state.edge_gap_spinbox.value = val as i32;
            apply_all_widths(state);
        }
        LayoutMessage::SetTopGap(v) => {
            let val = v.min(200);
            state.top_gap = val;
            state.top_gap_spinbox.value = val as i32;
            apply_all_widths(state);
        }
        LayoutMessage::SetTransitionDuration(v) => {
            let val = v.min(2000);
            state.transition_duration = val;
            state.transition_duration_spinbox.value = val as i32;
            apply_all_widths(state);
        }
        LayoutMessage::SetTagLayout(tag, idx) => {
            if tag >= 1 && tag <= 4 && idx < 5 {
                state.tag_layout_menus[tag - 1].selected = idx;
                let modes = vec!["cascade", "grid", "fullscreen", "floating", "popup"];
                let mode_str = modes[idx];
                write_tag_layout(tag, mode_str);
                send_ipc_command(&format!("tag-layout {} {}", tag, mode_str));
            }
        }
        LayoutMessage::Refreshed(new) => { *state = new; }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tag_layouts_empty() {
        let content = "";
        let modes = parse_tag_layouts_from_config(content);
        assert_eq!(modes, vec!["cascade", "cascade", "cascade", "cascade"]);
    }

    #[test]
    fn test_parse_tag_layouts_single() {
        let content = r#"
[layout]
gap = 10

[[tag_layout]]
tag = 2
mode = "grid"
"#;
        let modes = parse_tag_layouts_from_config(content);
        assert_eq!(modes, vec!["cascade", "grid", "cascade", "cascade"]);
    }

    #[test]
    fn test_parse_tag_layouts_multiple() {
        let content = r#"
[[tag_layout]]
tag = 1
mode = "fullscreen"

[[tag_layout]]
tag = 4
mode = "floating"
"#;
        let modes = parse_tag_layouts_from_config(content);
        assert_eq!(modes, vec!["fullscreen", "cascade", "cascade", "floating"]);
    }

    #[test]
    fn test_parse_tag_layouts_out_of_bounds() {
        let content = r#"
[[tag_layout]]
tag = 5
mode = "grid"

[[tag_layout]]
tag = 0
mode = "popup"
"#;
        let modes = parse_tag_layouts_from_config(content);
        assert_eq!(modes, vec!["cascade", "cascade", "cascade", "cascade"]);
    }
}
