use std::fs;
use std::io::Write;

use crate::app::PageContent;
use clear_ui::layout::{render_widget, Section, PageLayoutBuilder, LayoutStrategy};
use clear_ui::widget::{Spinbox, Dropdown};

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
    pub grid_gap: u16,
    pub status_height: u16,
    pub transition_duration: u16,
    pub spinboxes: Vec<Spinbox>,
    pub cascade_offset_spinbox: Spinbox,
    pub edge_gap_spinbox: Spinbox,
    pub top_gap_spinbox: Spinbox,
    pub grid_gap_spinbox: Spinbox,
    pub status_height_spinbox: Spinbox,
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
            grid_gap: 6,
            status_height: 24,
            transition_duration: 300,
            spinboxes: make_spinboxes(0, 6, 6, 6),
            cascade_offset_spinbox: Spinbox::new(20, 0, 200, 1),
            edge_gap_spinbox: Spinbox::new(48, 0, 200, 1),
            top_gap_spinbox: Spinbox::new(48, 0, 200, 1),
            grid_gap_spinbox: Spinbox::new(6, 0, 200, 1),
            status_height_spinbox: Spinbox::new(24, 0, 100, 1),
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
    SetGridGap(u16),
    SetTransitionDuration(u16),
    SetStatusHeight(u16),
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
    let gg = parse_u16_from(&content, "grid_gap", 6);
    let sh = parse_u16_from(&content, "bar_height", 24);
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
        grid_gap: gg,
        status_height: sh,
        transition_duration: td,
        spinboxes: make_spinboxes(fs, ca, g, fl),
        cascade_offset_spinbox: Spinbox::new(co as i32, 0, 200, 1),
        edge_gap_spinbox: Spinbox::new(gl as i32, 0, 200, 1),
        top_gap_spinbox: Spinbox::new(gt as i32, 0, 200, 1),
        grid_gap_spinbox: Spinbox::new(gg as i32, 0, 200, 1),
        status_height_spinbox: Spinbox::new(sh as i32, 0, 100, 1),
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

fn apply_single_layout_param(key: &str, val: u16) {
    write_config_value(key, &val.to_string());
    send_ipc_command(&format!("layout {} {}", key, val));
}

fn apply_edge_gap(val: u16) {
    let val_str = val.to_string();
    write_config_value("gap_left", &val_str);
    write_config_value("gap_right", &val_str);
    write_config_value("gap_bottom", &val_str);
    send_ipc_command(&format!("layout gap_left {}", val));
    send_ipc_command(&format!("layout gap_right {}", val));
    send_ipc_command(&format!("layout gap_bottom {}", val));
}

#[derive(Debug, Clone)]
struct PreviewWindow {
    app_id: String,
    title: String,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    tags: u32,
    _minimized: bool,
    has_parent: bool,
    layout_mode: String,
}

struct LayoutStatusInfo {
    active_tags: u32,
    focused_tags: u32,
    _num_tags: u32,
    windows: Vec<PreviewWindow>,
    focused_title: String,
    focused_layout_mode: String,
}

fn read_current_layout_status() -> LayoutStatusInfo {
    let mut active_tags = 1;
    let mut focused_tags = 1;
    let mut _num_tags = 4;

    let display = std::env::var("WAYLAND_DISPLAY").unwrap_or_else(|_| "wayland-0".to_string());
    
    let tags_path = format!("/tmp/ccec-tags-{}", display);
    let tags_fallback = "/tmp/ccec-tags".to_string();
    let tags_content = fs::read_to_string(&tags_path)
        .or_else(|_| fs::read_to_string(&tags_fallback))
        .unwrap_or_default();

    if let Some(line) = tags_content.lines().next() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            active_tags = parts[0].parse().unwrap_or(1);
            focused_tags = parts[1].parse().unwrap_or(1);
            _num_tags = parts[2].parse().unwrap_or(4);
        }
    }

    let title_path = format!("/tmp/ccec-title-{}", display);
    let title_fallback = "/tmp/ccec-title".to_string();
    let focused_title = fs::read_to_string(&title_path)
        .or_else(|_| fs::read_to_string(&title_fallback))
        .unwrap_or_default()
        .trim()
        .to_string();

    let layout_path = format!("/tmp/ccec-layout-{}", display);
    let layout_fallback = "/tmp/ccec-layout".to_string();
    let focused_layout_mode = fs::read_to_string(&layout_path)
        .or_else(|_| fs::read_to_string(&layout_fallback))
        .unwrap_or_else(|_| "Cascade".to_string())
        .trim()
        .to_string();

    let windows_path = format!("/tmp/ccec-windows-{}", display);
    let windows_fallback = "/tmp/ccec-windows".to_string();
    let windows_content = fs::read_to_string(&windows_path)
        .or_else(|_| fs::read_to_string(&windows_fallback))
        .unwrap_or_default();

    let mut windows = Vec::new();
    for line in windows_content.lines() {
        if !line.starts_with("window ") { continue; }
        
        let mut app_id = String::new();
        let mut title = String::new();
        let mut x = 0.0;
        let mut y = 0.0;
        let mut w = 0.0;
        let mut h = 0.0;
        let mut tags = 0;
        let mut minimized = false;
        let mut has_parent = false;
        let mut layout_mode = "Cascade".to_string();
        
        let parts = line.strip_prefix("window ").unwrap_or(line);
        
        let get_val = |p: &str, k: &str| -> Option<String> {
            if let Some(idx) = p.find(k) {
                let start = idx + k.len();
                let mut end = p.len();
                let next_keys = [
                    " app_id=", " title=", " mode=", " decoration=", " presentation=",
                    " tags=", " x=", " y=", " w=", " h=", " has_parent=", " minimized="
                ];
                for nk in next_keys {
                    if nk != k {
                        if let Some(nidx) = p[start..].find(nk) {
                            end = end.min(start + nidx);
                        }
                    }
                }
                Some(p[start..end].trim().to_string())
            } else {
                None
            }
        };

        if let Some(val) = get_val(parts, "app_id=") { app_id = val; }
        if let Some(val) = get_val(parts, "title=") { title = val; }
        if let Some(val) = get_val(parts, "x=") { x = val.parse().unwrap_or(0.0); }
        if let Some(val) = get_val(parts, "y=") { y = val.parse().unwrap_or(0.0); }
        if let Some(val) = get_val(parts, "w=") { w = val.parse().unwrap_or(0.0); }
        if let Some(val) = get_val(parts, "h=") { h = val.parse().unwrap_or(0.0); }
        if let Some(val) = get_val(parts, "tags=") { tags = val.parse().unwrap_or(0); }
        if let Some(val) = get_val(parts, "minimized=") { minimized = val == "true"; }
        if let Some(val) = get_val(parts, "has_parent=") { has_parent = val == "true"; }
        if let Some(val) = get_val(parts, "mode=") { layout_mode = val; }

        windows.push(PreviewWindow {
            app_id,
            title,
            x,
            y,
            w,
            h,
            tags,
            _minimized: minimized,
            has_parent,
            layout_mode,
        });
    }

    LayoutStatusInfo {
        active_tags,
        focused_tags,
        _num_tags,
        windows,
        focused_title,
        focused_layout_mode,
    }
}

fn get_short_app_name(app_id: &str) -> String {
    let lower = app_id.to_lowercase();
    if lower.contains("foot") || lower.contains("terminal") || lower.contains("kitty") || lower.contains("alacritty") {
        "Term".to_string()
    } else if lower.contains("firefox") || lower.contains("chrome") || lower.contains("qutebrowser") || lower.contains("browser") {
        "Web".to_string()
    } else if lower.contains("code") || lower.contains("vscodium") || lower.contains("neovim") || lower.contains("nvim") {
        "Code".to_string()
    } else if lower.contains("spotify") || lower.contains("music") {
        "Musc".to_string()
    } else if lower.contains("discord") {
        "Disc".to_string()
    } else if lower.contains("interface") {
        "Intf".to_string()
    } else if lower.is_empty() {
        "Win".to_string()
    } else {
        let mut s = lower;
        s.truncate(4);
        if let Some(first) = s.chars().next() {
            let first_upper = first.to_uppercase().to_string();
            format!("{}{}", first_upper, &s[first.len_utf8()..])
        } else {
            "Win".to_string()
        }
    }
}

struct SimNode {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    label: String,
}

pub fn view(state: &mut LayoutState, cx: f32, cy: f32, cw: f32, ch: f32, sec_focused: &[bool], layout: &mut dyn LayoutStrategy) -> PageContent {
    let mut final_pc = PageContent::new();
    let sec_w = 320.0f32;
    let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(5);

    // Current Layout Section (Read-only visual preview)
    builder.add_section(&mut final_pc, |pc, rx, ry| {
        let mut sec_cl = Section::new(pc, rx, ry, sec_w, "Current Layout");
        sec_cl.spacing(8.0);
        
        let info = read_current_layout_status();
        
        let card_w = (sec_w - 24.0) / 2.0;
        let card_h = 135.0;
        
        for tag_idx in 0..4 {
            let col = tag_idx % 2;
            let row = tag_idx / 2;
            let tx = rx + 8.0 + col as f32 * (card_w + 8.0);
            let ty = sec_cl.ay() + row as f32 * (card_h + 8.0);
            
            // Draw card background
            let is_active = (info.active_tags & (1 << tag_idx)) != 0;
            let bg_col = if is_active { [0.12, 0.24, 0.14, 0.55] } else { [0.08, 0.08, 0.12, 0.35] };
            let border_col = if is_active { [0.36, 0.56, 0.38, 0.95] } else { [0.24, 0.24, 0.28, 0.45] };
            pc.rect(bg_col, tx, ty, card_w, card_h);
            // Card border
            pc.rect(border_col, tx, ty, card_w, 1.0);
            pc.rect(border_col, tx, ty + card_h - 1.0, card_w, 1.0);
            pc.rect(border_col, tx, ty, 1.0, card_h);
            pc.rect(border_col, tx + card_w - 1.0, ty, 1.0, card_h);
            
            // Tag index text
            pc.text(&format!("TAG {}", tag_idx + 1), tx + 8.0, ty + 8.0, 10.0, [0.55, 0.55, 0.60, 1.0]);
            
            // Layout Name
            let layout_idx = state.tag_layout_menus.get(tag_idx).map(|m| m.selected).unwrap_or(0);
            let layout_name = match layout_idx {
                1 => "Cascade",
                2 => "Stack",
                3 => "Grid",
                4 => "L-Tiled",
                5 => "R-Tiled",
                6 => "Equal",
                7 => "Spiral",
                8 => "Floating",
                _ => "Fullscreen",
            };
            pc.text(layout_name, tx + 8.0, ty + 20.0, 13.0, [0.90, 0.90, 0.95, 1.0]);
            
            // Visual nodes layout preview inside card
            let preview_x = tx + 8.0;
            let preview_y = ty + 38.0;
            let preview_w = card_w - 16.0;
            let preview_h = card_h - 46.0;
            
            // Gray border for preview box
            pc.rect([0.16, 0.16, 0.20, 0.6], preview_x, preview_y, preview_w, preview_h);
            pc.rect([0.22, 0.22, 0.26, 0.8], preview_x, preview_y, preview_w, 1.0);
            pc.rect([0.22, 0.22, 0.26, 0.8], preview_x, preview_y + preview_h - 1.0, preview_w, 1.0);
            pc.rect([0.22, 0.22, 0.26, 0.8], preview_x, preview_y, 1.0, preview_h);
            pc.rect([0.22, 0.22, 0.26, 0.8], preview_x + preview_w - 1.0, preview_y, 1.0, preview_h);
            
            // Simulate layout windows preview
            let mut nodes = Vec::new();
            match layout_idx {
                0 => { // Fullscreen
                    nodes.push(SimNode { x: 2.0, y: 2.0, w: preview_w - 4.0, h: preview_h - 4.0, label: "F".to_string() });
                }
                1 => { // Cascade
                    nodes.push(SimNode { x: 2.0, y: 2.0, w: preview_w - 12.0, h: preview_h - 12.0, label: "1".to_string() });
                    nodes.push(SimNode { x: 6.0, y: 6.0, w: preview_w - 12.0, h: preview_h - 12.0, label: "2".to_string() });
                    nodes.push(SimNode { x: 10.0, y: 10.0, w: preview_w - 12.0, h: preview_h - 12.0, label: "3".to_string() });
                }
                2 => { // Stack
                    nodes.push(SimNode { x: 2.0, y: 2.0, w: preview_w - 4.0, h: preview_h - 4.0, label: "Stack".to_string() });
                }
                3 => { // Grid
                    let hw = (preview_w - 6.0) / 2.0;
                    let hh = (preview_h - 6.0) / 2.0;
                    nodes.push(SimNode { x: 2.0, y: 2.0, w: hw, h: hh, label: "1".to_string() });
                    nodes.push(SimNode { x: 4.0 + hw, y: 2.0, w: hw, h: hh, label: "2".to_string() });
                    nodes.push(SimNode { x: 2.0, y: 4.0 + hh, w: hw, h: hh, label: "3".to_string() });
                    nodes.push(SimNode { x: 4.0 + hw, y: 4.0 + hh, w: hw, h: hh, label: "4".to_string() });
                }
                4 => { // Left Tiled (Main window on left, stack on right)
                    let mw = (preview_w - 6.0) * 0.55;
                    let sw = (preview_w - 6.0) - mw;
                    let sh = (preview_h - 6.0) / 2.0;
                    nodes.push(SimNode { x: 2.0, y: 2.0, w: mw, h: preview_h - 4.0, label: "M".to_string() });
                    nodes.push(SimNode { x: 4.0 + mw, y: 2.0, w: sw, h: sh, label: "1".to_string() });
                    nodes.push(SimNode { x: 4.0 + mw, y: 4.0 + sh, w: sw, h: sh, label: "2".to_string() });
                }
                5 => { // Right Tiled (Main window on right, stack on left)
                    let mw = (preview_w - 6.0) * 0.55;
                    let sw = (preview_w - 6.0) - mw;
                    let sh = (preview_h - 6.0) / 2.0;
                    nodes.push(SimNode { x: 2.0, y: 2.0, w: sw, h: sh, label: "1".to_string() });
                    nodes.push(SimNode { x: 2.0, y: 4.0 + sh, w: sw, h: sh, label: "2".to_string() });
                    nodes.push(SimNode { x: 4.0 + sw, y: 2.0, w: mw, h: preview_h - 4.0, label: "M".to_string() });
                }
                6 => { // Equal (Split evenly horizontally)
                    let ew = (preview_w - 8.0) / 3.0;
                    nodes.push(SimNode { x: 2.0, y: 2.0, w: ew, h: preview_h - 4.0, label: "1".to_string() });
                    nodes.push(SimNode { x: 4.0 + ew, y: 2.0, w: ew, h: preview_h - 4.0, label: "2".to_string() });
                    nodes.push(SimNode { x: 6.0 + 2.0 * ew, y: 2.0, w: ew, h: preview_h - 4.0, label: "3".to_string() });
                }
                7 => { // Spiral (Fibonacci layout)
                    let w1 = (preview_w - 6.0) * 0.5;
                    let w2 = (preview_w - 6.0) - w1;
                    let h2 = (preview_h - 6.0) * 0.5;
                    nodes.push(SimNode { x: 2.0, y: 2.0, w: w1, h: preview_h - 4.0, label: "1".to_string() });
                    nodes.push(SimNode { x: 4.0 + w1, y: 2.0, w: w2, h: h2, label: "2".to_string() });
                    nodes.push(SimNode { x: 4.0 + w1, y: 4.0 + h2, w: w2 * 0.5, h: h2, label: "3".to_string() });
                    nodes.push(SimNode { x: 4.0 + w1 + w2 * 0.5, y: 4.0 + h2, w: w2 * 0.5, h: h2, label: "4".to_string() });
                }
                8 => { // Floating (Scatter windows randomly)
                    nodes.push(SimNode { x: 4.0, y: 6.0, w: preview_w * 0.45, h: preview_h * 0.5, label: "1".to_string() });
                    nodes.push(SimNode { x: preview_w * 0.4, y: 12.0, w: preview_w * 0.5, h: preview_h * 0.45, label: "2".to_string() });
                    nodes.push(SimNode { x: 8.0, y: preview_h * 0.4, w: preview_w * 0.55, h: preview_h * 0.5, label: "3".to_string() });
                }
                _ => {}
            }
            
            // Draw simulated layout preview rectangles
            for node in nodes {
                let rect_x = preview_x + node.x;
                let rect_y = preview_y + node.y;
                
                // Semi-transparent blue for node backgrounds, slightly highlighted if active tag
                let node_bg = if is_active { [0.30, 0.45, 0.65, 0.45] } else { [0.20, 0.24, 0.30, 0.25] };
                let node_border = if is_active { [0.45, 0.65, 0.90, 0.85] } else { [0.35, 0.40, 0.45, 0.55] };
                
                pc.rect(node_bg, rect_x, rect_y, node.w, node.h);
                
                // Draw node border lines
                pc.rect(node_border, rect_x, rect_y, node.w, 1.0);
                pc.rect(node_border, rect_x, rect_y + node.h - 1.0, node.w, 1.0);
                pc.rect(node_border, rect_x, rect_y, 1.0, node.h);
                pc.rect(node_border, rect_x + node.w - 1.0, rect_y, 1.0, node.h);
                
                // Center the label text inside the simulated node
                let text_sz = 9.0;
                let text_w = node.label.len() as f32 * 6.0;
                let text_color = if is_active { [0.95, 0.95, 1.0, 0.95] } else { [0.70, 0.70, 0.75, 0.75] };
                let tx_offset = ((node.w - text_w) / 2.0).max(1.0);
                let ty_offset = ((node.h - text_sz) / 2.0).max(1.0);
                
                pc.text(&node.label, rect_x + tx_offset, rect_y + ty_offset, text_sz, text_color);
            }
        }
        
        sec_cl.content_y += 2.0 * (card_h + 8.0) + 4.0;
        sec_cl.finish(pc)
    });

    // 1. Border Width Section
    builder.add_section(&mut final_pc, |pc, rx, ry| {
        let mut sec_bw = Section::new(pc, rx, ry, sec_w, "Border Width");
        sec_bw.spacing(8.0);
        for (i, param) in WidthParam::ALL.iter().enumerate() {
            state.spinboxes[i].set_label(param.label());
            sec_bw.widget(pc, &mut state.spinboxes[i], 14.0, 200.0, 26.0);
            sec_bw.spacing(8.0);
        }
        sec_bw.finish_focused(pc, sec_focused.first().copied().unwrap_or(false))
    });

    // 2. Cascade Section
    builder.add_section(&mut final_pc, |pc, rx, ry| {
        let mut sec_cascade = Section::new(pc, rx, ry, sec_w, "Cascade");
        sec_cascade.spacing(8.0);
        state.cascade_offset_spinbox.set_label("Offset");
        sec_cascade.widget(pc, &mut state.cascade_offset_spinbox, 14.0, 200.0, 26.0);
        sec_cascade.spacing(8.0);
        state.edge_gap_spinbox.set_label("Edge Gap");
        sec_cascade.widget(pc, &mut state.edge_gap_spinbox, 14.0, 200.0, 26.0);
        sec_cascade.spacing(8.0);
        state.top_gap_spinbox.set_label("Top Gap");
        sec_cascade.widget(pc, &mut state.top_gap_spinbox, 14.0, 200.0, 26.0);
        sec_cascade.spacing(8.0);
        sec_cascade.finish_focused(pc, sec_focused.get(1).copied().unwrap_or(false))
    });

    // 3. Movement Section
    builder.add_section(&mut final_pc, |pc, rx, ry| {
        let mut movement_sec = Section::new(pc, rx, ry, sec_w, "Movement");
        movement_sec.spacing(8.0);
        state.transition_duration_spinbox.set_label("Duration (ms)");
        movement_sec.widget(pc, &mut state.transition_duration_spinbox, 14.0, 200.0, 26.0);
        movement_sec.spacing(8.0);
        movement_sec.finish_focused(pc, sec_focused.get(2).copied().unwrap_or(false))
    });

    // 4. Default Layouts Section
    builder.add_section(&mut final_pc, |pc, rx, ry| {
        let mut default_layouts_sec = Section::new(pc, rx, ry, sec_w, "Default Layouts");
        default_layouts_sec.spacing(8.0);
        for i in 0..4 {
            default_layouts_sec.widget(pc, &mut state.tag_layout_menus[i], 14.0, 200.0, 26.0);
            default_layouts_sec.spacing(8.0);
        }
        default_layouts_sec.finish_focused(pc, sec_focused.get(3).copied().unwrap_or(false))
    });

    final_pc
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
            apply_single_layout_param("cascade_offset", val);
        }
        LayoutMessage::SetEdgeGap(v) => {
            let val = v.min(200);
            state.edge_gap = val;
            state.edge_gap_spinbox.value = val as i32;
            apply_edge_gap(val);
        }
        LayoutMessage::SetTopGap(v) => {
            let val = v.min(200);
            state.top_gap = val;
            state.top_gap_spinbox.value = val as i32;
            apply_single_layout_param("gap_top", val);
        }
        LayoutMessage::SetGridGap(v) => {
            let val = v.min(200);
            state.grid_gap = val;
            state.grid_gap_spinbox.value = val as i32;
            apply_single_layout_param("grid_gap", val);
        }
        LayoutMessage::SetTransitionDuration(v) => {
            let val = v.min(2000);
            state.transition_duration = val;
            state.transition_duration_spinbox.value = val as i32;
            apply_single_layout_param("transition_duration", val);
        }
        LayoutMessage::SetStatusHeight(v) => {
            let val = v.min(100);
            state.status_height = val;
            state.status_height_spinbox.value = val as i32;
            apply_single_layout_param("bar_height", val);
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

    #[test]
    fn test_view_layout_grid() {
        let mut state = LayoutState::default();
        let mut layout = clear_ui::layout::ColumnLayout::new(20.0);
        let pc = view(&mut state, 10.0, 20.0, 800.0, 600.0, &[false, false, false, false], &mut layout);
        assert!(!pc.rects.is_empty() || !pc.texts.is_empty());
    }
}
