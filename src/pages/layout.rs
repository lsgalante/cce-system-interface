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

pub fn view(state: &mut LayoutState, cx: f32, cy: f32, cw: f32, _ch: f32, sec_focused: &[bool]) -> PageContent {
    let mut pc = PageContent::new();
    let mut y = cy + 12.0;

    // Current Layout Section (Read-only visual preview)
    let mut sec_cl = Section::new(&mut pc, cx, y, cw, "Current Layout");
    sec_cl.spacing(8.0);
    
    let info = read_current_layout_status();
    
    let card_w = (cw - 24.0) / 2.0;
    let card_h = 135.0;
    
    for tag_idx in 0..4 {
        let col = tag_idx % 2;
        let row = tag_idx / 2;
        let tx = cx + 8.0 + col as f32 * (card_w + 8.0);
        let ty = sec_cl.ay() + row as f32 * (card_h + 8.0);
        
        let is_active = (info.active_tags & (1 << tag_idx)) != 0;
        let is_focused = (info.focused_tags & (1 << tag_idx)) != 0;
        
        // Draw card border and background
        let border_color = if is_focused {
            [0.2, 0.6, 1.0, 1.0]
        } else if is_active {
            [0.28, 0.28, 0.32, 1.0]
        } else {
            [0.16, 0.16, 0.18, 1.0]
        };
        
        let bg_color = if is_active {
            [0.08, 0.08, 0.11, 0.9]
        } else {
            [0.05, 0.05, 0.07, 0.9]
        };
        
        pc.rect(border_color, tx, ty, card_w, card_h);
        pc.rect(bg_color, tx + 1.0, ty + 1.0, card_w - 2.0, card_h - 2.0);
        
        // Render tag label in top-left of the card
        let tag_label = format!("T{}", tag_idx + 1);
        let tag_label_color = if is_focused {
            [1.0, 1.0, 1.0, 1.0]
        } else if is_active {
            [0.8, 0.8, 0.85, 1.0]
        } else {
            [0.4, 0.4, 0.45, 1.0]
        };
        pc.text(&tag_label, tx + 8.0, ty + 6.0, 9.5, tag_label_color);
        
        // Render miniature screen preview inside the box (on the left side)
        let px = tx + 8.0;
        let py = ty + 24.0;
        let p_w = 70.0;
        let p_h = 44.0;
        
        // Screen background
        pc.rect([0.04, 0.04, 0.06, 1.0], px, py, p_w, p_h);
        pc.rect([0.16, 0.16, 0.18, 1.0], px, py, p_w, 1.0); // Top border
        pc.rect([0.16, 0.16, 0.18, 1.0], px, py + p_h - 1.0, p_w, 1.0); // Bottom border
        pc.rect([0.16, 0.16, 0.18, 1.0], px, py, 1.0, p_h); // Left border
        pc.rect([0.16, 0.16, 0.18, 1.0], px + p_w - 1.0, py, 1.0, p_h); // Right border
        
        // Find windows for this tag
        let tag_windows: Vec<&PreviewWindow> = info.windows.iter()
            .filter(|w| w.app_id != "clear-status-interface" && ((w.tags & (1 << tag_idx)) != 0 || w.tags == u32::MAX))
            .collect();
            
        // Reference screen size
        let screen_w = 1920.0;
        let screen_h = 1200.0;
        let scale_x = p_w / screen_w;
        let scale_y = p_h / screen_h;
        
        for win in &tag_windows {
            let wx = (px + win.x * scale_x).max(px).min(px + p_w);
            let wy = (py + win.y * scale_y).max(py).min(py + p_h);
            let ww = (win.w * scale_x).min(p_w - (wx - px));
            let wh = (win.h * scale_y).min(p_h - (wy - py));
            
            let is_win_focused = !win.title.is_empty() && win.title == info.focused_title;
            let color = if is_win_focused {
                [0.2, 0.6, 1.0, 1.0]
            } else {
                [0.4, 0.4, 0.45, 1.0]
            };
            let fill = if is_win_focused {
                [0.10, 0.32, 0.55, 0.4]
            } else {
                [0.12, 0.12, 0.15, 0.4]
            };
            
            pc.rect(color, wx, wy, ww, wh);
            pc.rect(fill, wx + 0.5, wy + 0.5, ww - 1.0, wh - 1.0);
        }
        
        // Determine Tag layout mode
        let layout_name = if is_focused {
            info.focused_layout_mode.clone()
        } else if let Some(win) = tag_windows.first() {
            win.layout_mode.clone()
        } else {
            let layout_idx = state.tag_layout_menus.get(tag_idx).map(|m| m.selected).unwrap_or(0);
            let layout_modes = ["Cascade", "Grid", "Fullscreen", "Floating", "Popup"];
            layout_modes.get(layout_idx).copied().unwrap_or("Cascade").to_string()
        };

        // Render Visual Focus Hierarchy Tree on the right side of the card
        let tx_tree = tx + 84.0;
        let ty_tree = ty + 18.0;
        let t_w = card_w - 92.0;
        let t_h = card_h - 26.0;
        
        struct TreeNode {
            label: String,
            is_focused: bool,
            is_layout: bool,
            is_role: bool,
            x: f32,
            y: f32,
            w: f32,
            h: f32,
        }
        
        struct TreeLine {
            x0: f32,
            y0: f32,
            x1: f32,
            y1: f32,
            is_dotted: bool,
        }
        
        let mut nodes = Vec::new();
        let mut lines = Vec::new();
        
        // 1. Root Node: Layout mode
        let layout_label = match layout_name.as_str() {
            "Grid" => "GRID",
            "Cascade" => "CASC",
            "Fullscreen" => "FULL",
            "Floating" => "FLOT",
            "Popup" => "POP",
            _ => "CASC",
        };
        nodes.push(TreeNode {
            label: layout_label.to_string(),
            is_focused: false,
            is_layout: true,
            is_role: false,
            x: tx_tree + 2.0,
            y: ty_tree + (t_h - 14.0) / 2.0,
            w: 28.0,
            h: 14.0,
        });
        
        // 2. Window hierarchy
        if tag_windows.is_empty() {
            nodes.push(TreeNode {
                label: "(empty)".to_string(),
                is_focused: false,
                is_layout: false,
                is_role: true,
                x: tx_tree + 46.0,
                y: ty_tree + (t_h - 12.0) / 2.0,
                w: 42.0,
                h: 12.0,
            });
            lines.push(TreeLine {
                x0: tx_tree + 30.0,
                y0: ty_tree + t_h / 2.0,
                x1: tx_tree + 46.0,
                y1: ty_tree + t_h / 2.0,
                is_dotted: true,
            });
        } else {
            // Group transient/child windows under their parent
            struct WinNode {
                win: PreviewWindow,
                children: Vec<PreviewWindow>,
            }
            let mut groups: Vec<WinNode> = Vec::new();
            for w in &tag_windows {
                if w.has_parent && !groups.is_empty() {
                    groups.last_mut().unwrap().children.push((*w).clone());
                } else {
                    groups.push(WinNode {
                        win: (*w).clone(),
                        children: Vec::new(),
                    });
                }
            }
            
            let num_roots = groups.len();
            if num_roots == 1 {
                // One root group: Layout -> Root -> Children
                let root_g = &groups[0];
                let rx = tx_tree + 44.0;
                let ry = ty_tree + (t_h - 14.0) / 2.0;
                let is_root_focused = !root_g.win.title.is_empty() && root_g.win.title == info.focused_title;
                nodes.push(TreeNode {
                    label: get_short_app_name(&root_g.win.app_id),
                    is_focused: is_root_focused,
                    is_layout: false,
                    is_role: false,
                    x: rx,
                    y: ry,
                    w: 42.0,
                    h: 14.0,
                });
                lines.push(TreeLine {
                    x0: tx_tree + 30.0,
                    y0: ty_tree + t_h / 2.0,
                    x1: rx,
                    y1: ty_tree + t_h / 2.0,
                    is_dotted: false,
                });
                
                let child_count = root_g.children.len();
                for (ci, child) in root_g.children.iter().enumerate() {
                    let cx = tx_tree + 104.0;
                    let cy = if child_count == 1 {
                        ty_tree + (t_h - 12.0) / 2.0
                    } else {
                        ty_tree + 6.0 + (ci as f32 / (child_count - 1) as f32) * (t_h - 24.0)
                    };
                    let is_child_focused = !child.title.is_empty() && child.title == info.focused_title;
                    nodes.push(TreeNode {
                        label: get_short_app_name(&child.app_id),
                        is_focused: is_child_focused,
                        is_layout: false,
                        is_role: false,
                        x: cx,
                        y: cy,
                        w: 32.0,
                        h: 12.0,
                    });
                    lines.push(TreeLine {
                        x0: rx + 42.0,
                        y0: ry + 7.0,
                        x1: cx,
                        y1: cy + 6.0,
                        is_dotted: true,
                    });
                }
            } else if num_roots == 2 {
                // Two root groups (Master & Stack): C -> M & S -> Windows -> Children
                let m_y = ty_tree + (t_h / 2.0) - 20.0;
                let s_y = ty_tree + (t_h / 2.0) + 20.0;
                
                // M indicator
                nodes.push(TreeNode {
                    label: "M".to_string(),
                    is_focused: false,
                    is_layout: false,
                    is_role: true,
                    x: tx_tree + 44.0,
                    y: m_y - 6.0,
                    w: 12.0,
                    h: 12.0,
                });
                lines.push(TreeLine {
                    x0: tx_tree + 30.0,
                    y0: ty_tree + t_h / 2.0,
                    x1: tx_tree + 44.0,
                    y1: m_y,
                    is_dotted: false,
                });
                
                // Master Window
                let m_win = &groups[0];
                let is_m_focused = !m_win.win.title.is_empty() && m_win.win.title == info.focused_title;
                nodes.push(TreeNode {
                    label: get_short_app_name(&m_win.win.app_id),
                    is_focused: is_m_focused,
                    is_layout: false,
                    is_role: false,
                    x: tx_tree + 68.0,
                    y: m_y - 7.0,
                    w: 36.0,
                    h: 14.0,
                });
                lines.push(TreeLine {
                    x0: tx_tree + 56.0,
                    y0: m_y,
                    x1: tx_tree + 68.0,
                    y1: m_y,
                    is_dotted: false,
                });
                
                // S indicator
                nodes.push(TreeNode {
                    label: "S".to_string(),
                    is_focused: false,
                    is_layout: false,
                    is_role: true,
                    x: tx_tree + 44.0,
                    y: s_y - 6.0,
                    w: 12.0,
                    h: 12.0,
                });
                lines.push(TreeLine {
                    x0: tx_tree + 30.0,
                    y0: ty_tree + t_h / 2.0,
                    x1: tx_tree + 44.0,
                    y1: s_y,
                    is_dotted: false,
                });
                
                // Stack Window
                let s_win = &groups[1];
                let is_s_focused = !s_win.win.title.is_empty() && s_win.win.title == info.focused_title;
                nodes.push(TreeNode {
                    label: get_short_app_name(&s_win.win.app_id),
                    is_focused: is_s_focused,
                    is_layout: false,
                    is_role: false,
                    x: tx_tree + 68.0,
                    y: s_y - 7.0,
                    w: 36.0,
                    h: 14.0,
                });
                lines.push(TreeLine {
                    x0: tx_tree + 56.0,
                    y0: s_y,
                    x1: tx_tree + 68.0,
                    y1: s_y,
                    is_dotted: false,
                });
                
                // Master children
                let m_child_count = m_win.children.len();
                for (ci, child) in m_win.children.iter().enumerate() {
                    let cx = tx_tree + 114.0;
                    let cy = if m_child_count == 1 {
                        m_y - 6.0
                    } else {
                        m_y - 20.0 + (ci as f32 / (m_child_count - 1) as f32) * 30.0
                    };
                    let is_child_focused = !child.title.is_empty() && child.title == info.focused_title;
                    nodes.push(TreeNode {
                        label: get_short_app_name(&child.app_id),
                        is_focused: is_child_focused,
                        is_layout: false,
                        is_role: false,
                        x: cx,
                        y: cy,
                        w: 28.0,
                        h: 11.0,
                    });
                    lines.push(TreeLine {
                        x0: tx_tree + 104.0,
                        y0: m_y,
                        x1: cx,
                        y1: cy + 5.5,
                        is_dotted: true,
                    });
                }
                
                // Stack children
                let s_child_count = s_win.children.len();
                for (ci, child) in s_win.children.iter().enumerate() {
                    let cx = tx_tree + 114.0;
                    let cy = if s_child_count == 1 {
                        s_y - 6.0
                    } else {
                        s_y - 20.0 + (ci as f32 / (s_child_count - 1) as f32) * 30.0
                    };
                    let is_child_focused = !child.title.is_empty() && child.title == info.focused_title;
                    nodes.push(TreeNode {
                        label: get_short_app_name(&child.app_id),
                        is_focused: is_child_focused,
                        is_layout: false,
                        is_role: false,
                        x: cx,
                        y: cy,
                        w: 28.0,
                        h: 11.0,
                    });
                    lines.push(TreeLine {
                        x0: tx_tree + 104.0,
                        y0: s_y,
                        x1: cx,
                        y1: cy + 5.5,
                        is_dotted: true,
                    });
                }
            } else {
                // More than 2 roots: Master & Stacks (list of stack items)
                let m_y = ty_tree + (t_h / 2.0) - 24.0;
                let s_y = ty_tree + (t_h / 2.0) + 20.0;
                
                // M indicator
                nodes.push(TreeNode {
                    label: "M".to_string(),
                    is_focused: false,
                    is_layout: false,
                    is_role: true,
                    x: tx_tree + 44.0,
                    y: m_y - 6.0,
                    w: 12.0,
                    h: 12.0,
                });
                lines.push(TreeLine {
                    x0: tx_tree + 30.0,
                    y0: ty_tree + t_h / 2.0,
                    x1: tx_tree + 44.0,
                    y1: m_y,
                    is_dotted: false,
                });
                
                // Master Window
                let m_win = &groups[0];
                let is_m_focused = !m_win.win.title.is_empty() && m_win.win.title == info.focused_title;
                nodes.push(TreeNode {
                    label: get_short_app_name(&m_win.win.app_id),
                    is_focused: is_m_focused,
                    is_layout: false,
                    is_role: false,
                    x: tx_tree + 68.0,
                    y: m_y - 7.0,
                    w: 36.0,
                    h: 14.0,
                });
                lines.push(TreeLine {
                    x0: tx_tree + 56.0,
                    y0: m_y,
                    x1: tx_tree + 68.0,
                    y1: m_y,
                    is_dotted: false,
                });
                
                // S indicator
                nodes.push(TreeNode {
                    label: "S".to_string(),
                    is_focused: false,
                    is_layout: false,
                    is_role: true,
                    x: tx_tree + 44.0,
                    y: s_y - 6.0,
                    w: 12.0,
                    h: 12.0,
                });
                lines.push(TreeLine {
                    x0: tx_tree + 30.0,
                    y0: ty_tree + t_h / 2.0,
                    x1: tx_tree + 44.0,
                    y1: s_y,
                    is_dotted: false,
                });
                
                // Render first 2 Stack items vertically spaced
                let s1_win = &groups[1];
                let is_s1_focused = !s1_win.win.title.is_empty() && s1_win.win.title == info.focused_title;
                let s1_y = s_y - 14.0;
                nodes.push(TreeNode {
                    label: get_short_app_name(&s1_win.win.app_id),
                    is_focused: is_s1_focused,
                    is_layout: false,
                    is_role: false,
                    x: tx_tree + 68.0,
                    y: s1_y - 7.0,
                    w: 36.0,
                    h: 14.0,
                });
                lines.push(TreeLine {
                    x0: tx_tree + 56.0,
                    y0: s_y,
                    x1: tx_tree + 68.0,
                    y1: s1_y,
                    is_dotted: false,
                });
                
                let s2_win = &groups[2];
                let is_s2_focused = !s2_win.win.title.is_empty() && s2_win.win.title == info.focused_title;
                let s2_y = s_y + 14.0;
                nodes.push(TreeNode {
                    label: get_short_app_name(&s2_win.win.app_id),
                    is_focused: is_s2_focused,
                    is_layout: false,
                    is_role: false,
                    x: tx_tree + 68.0,
                    y: s2_y - 7.0,
                    w: 36.0,
                    h: 14.0,
                });
                lines.push(TreeLine {
                    x0: tx_tree + 56.0,
                    y0: s_y,
                    x1: tx_tree + 68.0,
                    y1: s2_y,
                    is_dotted: false,
                });
            }
        }
        
        let line_color = if is_active {
            [0.30, 0.30, 0.35, 0.8]
        } else {
            [0.18, 0.18, 0.20, 0.6]
        };
        let draw_line = |pc: &mut PageContent, x0: f32, y0: f32, x1: f32, y1: f32, is_dotted: bool, color: [f32; 4]| {
            if is_dotted {
                if (x0 - x1).abs() < 0.1 {
                    let sy = y0.min(y1);
                    let ey = y0.max(y1);
                    let mut curr_y = sy;
                    while curr_y <= ey {
                        pc.rect(color, x0 - 0.5, curr_y, 1.0, 1.0);
                        curr_y += 3.0;
                    }
                } else if (y0 - y1).abs() < 0.1 {
                    let sx = x0.min(x1);
                    let ex = x0.max(x1);
                    let mut curr_x = sx;
                    while curr_x <= ex {
                        pc.rect(color, curr_x, y0 - 0.5, 1.0, 1.0);
                        curr_x += 3.0;
                    }
                } else {
                    pc.rect(color, x0.min(x1), y0.min(y1), (x0 - x1).abs().max(1.0), (y0 - y1).abs().max(1.0));
                }
            } else {
                if (x0 - x1).abs() < 0.1 {
                    pc.rect(color, x0 - 0.5, y0.min(y1), 1.0, (y0 - y1).abs());
                } else if (y0 - y1).abs() < 0.1 {
                    pc.rect(color, x0.min(x1), y0 - 0.5, (x0 - x1).abs(), 1.0);
                } else {
                    pc.rect(color, x0.min(x1), y0.min(y1), (x0 - x1).abs().max(1.0), (y0 - y1).abs().max(1.0));
                }
            }
        };
        
        for line in &lines {
            let mid_x = (line.x0 + line.x1) / 2.0;
            draw_line(&mut pc, line.x0, line.y0, mid_x, line.y0, line.is_dotted, line_color);
            draw_line(&mut pc, mid_x, line.y0, mid_x, line.y1, line.is_dotted, line_color);
            draw_line(&mut pc, mid_x, line.y1, line.x1, line.y1, line.is_dotted, line_color);
        }
        
        // Draw nodes
        for node in nodes {
            let border = if node.is_layout {
                [0.32, 0.32, 0.38, 1.0]
            } else if node.is_role {
                [0.20, 0.20, 0.24, 0.8]
            } else if node.is_focused {
                [0.2, 0.6, 1.0, 1.0]
            } else {
                [0.22, 0.22, 0.26, 1.0]
            };
            
            let bg = if node.is_layout {
                [0.14, 0.14, 0.18, 1.0]
            } else if node.is_role {
                [0.09, 0.09, 0.11, 0.9]
            } else if node.is_focused {
                [0.10, 0.32, 0.55, 1.0]
            } else {
                [0.11, 0.11, 0.15, 1.0]
            };
            
            pc.rect(border, node.x, node.y, node.w, node.h);
            pc.rect(bg, node.x + 1.0, node.y + 1.0, node.w - 2.0, node.h - 2.0);
            
            let text_color = if node.is_focused {
                [1.0, 1.0, 1.0, 1.0]
            } else if node.is_role {
                [0.48, 0.48, 0.52, 1.0]
            } else {
                [0.78, 0.78, 0.82, 1.0]
            };
            
            let text_sz = if node.is_layout {
                7.5
            } else if node.is_role {
                7.0
            } else {
                7.0
            };
            
            let char_width = text_sz * 0.52;
            let text_w = node.label.len() as f32 * char_width;
            let tx_offset = ((node.w - text_w) / 2.0).max(1.0);
            let ty_offset = ((node.h - text_sz) / 2.0).max(1.0);
            
            pc.text(&node.label, node.x + tx_offset, node.y + ty_offset, text_sz, text_color);
        }
    }
    
    sec_cl.content_y += 2.0 * (card_h + 8.0) + 4.0;
    y = sec_cl.finish(&mut pc);

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
}
