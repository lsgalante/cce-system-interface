use std::fs;
use std::io::Write;

use crate::app::PageContent;
use cce_ui::layout::{render_widget, PageLayoutBuilder, LayoutStrategy};
use cce_ui::widget::{Spinbox, Dropdown, LayoutPreview, PreviewLayoutMode, Toggle, Element};
use crate::pages::interface::{parse_bool_from, parse_transparency_opacity, write_transparency_config_value, status_interface_reload};


const CONFIG_PATH: &str = "/home/lsgalante/.config/cce/config.toml";

fn get_socket_path() -> String {
    match std::env::var("WAYLAND_DISPLAY") {
        Ok(display) => format!("/tmp/cce-client-{}.sock", display),
        Err(_) => "/tmp/cce-client.sock".to_string(),
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
    pub side_panel_behavior_menu: Dropdown,
    pub side_panel_position_menu: Dropdown,
    pub side_panel_width: u16,
    pub side_panel_width_spinbox: Spinbox,
    pub side_panel_border_gap: u16,
    pub side_panel_border_gap_spinbox: Spinbox,
    pub side_panel_border_opacity: u16,
    pub side_panel_border_opacity_spinbox: Spinbox,
    pub transparency_enabled: bool,
    pub transparency_toggle: Toggle,
    pub blur_enabled: bool,
    pub blur_toggle: Toggle,
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
            side_panel_behavior_menu: Dropdown::new(
                vec!["Above".to_string(), "Inline".to_string()],
                1,
            ).with_label("Behavior"),
            side_panel_position_menu: Dropdown::new(
                vec!["Left".to_string(), "Right".to_string()],
                0,
            ).with_label("Position"),
            side_panel_width: 360,
            side_panel_width_spinbox: Spinbox::new(360, 0, 2000, 10),
            side_panel_border_gap: 0,
            side_panel_border_gap_spinbox: Spinbox::new(0, 0, 500, 1),
            side_panel_border_opacity: 100,
            side_panel_border_opacity_spinbox: Spinbox::new(100, 0, 100, 5),
            transparency_enabled: true,
            transparency_toggle: Toggle::new().with_label("Transparency"),
            blur_enabled: true,
            blur_toggle: Toggle::new().with_label("Blur"),
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
    SetSidePanelBehavior(usize),
    SetSidePanelPosition(usize),
    SetSidePanelWidth(u16),
    SetSidePanelBorderGap(u16),
    SetSidePanelBorderOpacity(u16),
    ToggleTransparency,
    ToggleBlur,
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

    let side_panel_behavior = parse_string_from(&content, "side_panel_behavior", "inline");
    let side_panel_behavior_idx = if side_panel_behavior == "above" { 0 } else { 1 };
    let side_panel_behavior_menu = Dropdown::new(
        vec!["Above".to_string(), "Inline".to_string()],
        side_panel_behavior_idx,
    ).with_label("Behavior");

    let side_panel_position = parse_string_from(&content, "side_panel_position", "left");
    let side_panel_position_idx = if side_panel_position == "right" { 1 } else { 0 };
    let side_panel_position_menu = Dropdown::new(
        vec!["Left".to_string(), "Right".to_string()],
        side_panel_position_idx,
    ).with_label("Position");

    let spw = parse_u16_from(&content, "side_panel_width", 360);
    let spbg = parse_u16_from(&content, "side_panel_border_gap", 0);
    let spbo = parse_u16_from(&content, "side_panel_border_opacity", 100);

    let transparency_opacity = parse_transparency_opacity(&content);
    let transparency_enabled = transparency_opacity < 1.0;

    let window_blur = parse_bool_from(&content, "window_blur", false);
    let border_blur = parse_bool_from(&content, "border_blur", false);
    let blur_enabled = window_blur || border_blur;

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
        side_panel_behavior_menu,
        side_panel_position_menu,
        side_panel_width: spw,
        side_panel_width_spinbox: Spinbox::new(spw as i32, 0, 2000, 10),
        side_panel_border_gap: spbg,
        side_panel_border_gap_spinbox: Spinbox::new(spbg as i32, 0, 500, 1),
        side_panel_border_opacity: spbo,
        side_panel_border_opacity_spinbox: Spinbox::new(spbo as i32, 0, 100, 5),
        transparency_enabled,
        transparency_toggle: Toggle::new().with_label("Transparency"),
        blur_enabled,
        blur_toggle: Toggle::new().with_label("Blur"),
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

fn parse_string_from(content: &str, key: &str, default: &str) -> String {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(key) {
            let rest = rest.trim_start_matches(|c: char| c == ' ' || c == '=' || c == '"');
            return rest.trim_end_matches('"').trim().to_string();
        }
    }
    default.to_string()
}

fn write_config_value(key: &str, value: &str) -> bool {
    super::interface::write_config_value_path(CONFIG_PATH, key, value)
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
#[allow(dead_code)]
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

#[allow(dead_code)]
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
    
    let tags_path = format!("/tmp/cce-client-tags-{}", display);
    let tags_fallback = "/tmp/cce-client-tags".to_string();
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

    let title_path = format!("/tmp/cce-client-title-{}", display);
    let title_fallback = "/tmp/cce-client-title".to_string();
    let focused_title = fs::read_to_string(&title_path)
        .or_else(|_| fs::read_to_string(&title_fallback))
        .unwrap_or_default()
        .trim()
        .to_string();

    let layout_path = format!("/tmp/cce-client-layout-{}", display);
    let layout_fallback = "/tmp/cce-client-layout".to_string();
    let focused_layout_mode = fs::read_to_string(&layout_path)
        .or_else(|_| fs::read_to_string(&layout_fallback))
        .unwrap_or_else(|_| "Cascade".to_string())
        .trim()
        .to_string();

    let windows_path = format!("/tmp/cce-client-windows-{}", display);
    let windows_fallback = "/tmp/cce-client-windows".to_string();
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

pub fn view(state: &mut LayoutState, cx: f32, cy: f32, cw: f32, ch: f32, sec_focused: &[bool], layout: &mut dyn LayoutStrategy, ctx: &mut cce_ui::context::UiContext) -> PageContent {
    let mut final_pc = PageContent::new();
    let sec_w = 320.0f32;
    let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(9);

    // Current Layout Section (Read-only visual preview)
    builder.add_section(&mut final_pc, "Current Layout", false, |sec_cl| {
        sec_cl.spacing(8.0);
        
        let info = read_current_layout_status();
        
        let card_w = (sec_w - 24.0) / 2.0;
        let card_h = 135.0;
        let rx = sec_cl.left;
        
        for tag_idx in 0..4 {
            let col = tag_idx % 2;
            let row = tag_idx / 2;
            let tx = rx + 8.0 + col as f32 * (card_w + 8.0);
            let ty = sec_cl.ay() + row as f32 * (card_h + 8.0);
            
            let is_active = (info.active_tags & (1 << tag_idx)) != 0;
            let layout_idx = state.tag_layout_menus.get(tag_idx).map(|m| m.selected).unwrap_or(0);
            let mode = match layout_idx {
                1 => PreviewLayoutMode::Cascade,
                2 => PreviewLayoutMode::Stack,
                3 => PreviewLayoutMode::Grid,
                4 => PreviewLayoutMode::LeftTiled,
                5 => PreviewLayoutMode::RightTiled,
                6 => PreviewLayoutMode::Equal,
                7 => PreviewLayoutMode::Spiral,
                8 => PreviewLayoutMode::Floating,
                _ => PreviewLayoutMode::Fullscreen,
            };

            let mut preview = LayoutPreview::new(mode)
                .with_active(is_active)
                .with_label(&format!("TAG {}", tag_idx + 1));
            render_widget(sec_cl.pc, &mut preview, tx, ty, card_w, card_h, ctx);
        }
        
        sec_cl.content_y += 2.0 * (card_h + 8.0) + 4.0;
    });

    // 1. Fullscreen Section
    builder.add_section(&mut final_pc, "Fullscreen", sec_focused.get(0).copied().unwrap_or(false), |sec_fs| {
        sec_fs.spacing(8.0);
        state.spinboxes[0].set_label("Border Width");
        sec_fs.widget_full(&mut state.spinboxes[0], 44.0, ctx);
        sec_fs.spacing(8.0);
    });

    // 2. Cascade Section
    builder.add_section(&mut final_pc, "Cascade", sec_focused.get(1).copied().unwrap_or(false), |sec_cascade| {
        sec_cascade.spacing(8.0);
        state.spinboxes[1].set_label("Border Width");
        sec_cascade.widget_full(&mut state.spinboxes[1], 44.0, ctx);
        sec_cascade.spacing(8.0);
        state.cascade_offset_spinbox.set_label("Offset");
        sec_cascade.widget_full(&mut state.cascade_offset_spinbox, 44.0, ctx);
        sec_cascade.spacing(8.0);
        state.edge_gap_spinbox.set_label("Edge Gap");
        sec_cascade.widget_full(&mut state.edge_gap_spinbox, 44.0, ctx);
        sec_cascade.spacing(8.0);
        state.top_gap_spinbox.set_label("Top Gap");
        sec_cascade.widget_full(&mut state.top_gap_spinbox, 44.0, ctx);
        sec_cascade.spacing(8.0);
    });

    // 3. Grid Section
    builder.add_section(&mut final_pc, "Grid", sec_focused.get(2).copied().unwrap_or(false), |sec_grid| {
        sec_grid.spacing(8.0);
        state.spinboxes[2].set_label("Border Width");
        sec_grid.widget_full(&mut state.spinboxes[2], 44.0, ctx);
        sec_grid.spacing(8.0);
        state.grid_gap_spinbox.set_label("Gap");
        sec_grid.widget_full(&mut state.grid_gap_spinbox, 44.0, ctx);
        sec_grid.spacing(8.0);
    });

    // 4. Floating Section
    builder.add_section(&mut final_pc, "Floating", sec_focused.get(3).copied().unwrap_or(false), |sec_float| {
        sec_float.spacing(8.0);
        state.spinboxes[3].set_label("Border Width");
        sec_float.widget_full(&mut state.spinboxes[3], 44.0, ctx);
        sec_float.spacing(8.0);
    });

    // 5. Movement Section
    builder.add_section(&mut final_pc, "Movement", sec_focused.get(4).copied().unwrap_or(false), |movement_sec| {
        movement_sec.spacing(8.0);
        state.transition_duration_spinbox.set_label("Duration (ms)");
        movement_sec.widget_full(&mut state.transition_duration_spinbox, 44.0, ctx);
        movement_sec.spacing(8.0);
    });

    // 6. Default Layouts Section
    builder.add_section(&mut final_pc, "Default Layouts", true, |default_layouts_sec| {
        default_layouts_sec.spacing(8.0);
        for i in 0..4 {
            default_layouts_sec.widget_full(&mut state.tag_layout_menus[i], 44.0, ctx);
            default_layouts_sec.spacing(8.0);
        }
    });

    // 7. Side Panel Section
    builder.add_section(&mut final_pc, "Side Panel", true, |side_panel_sec| {
        side_panel_sec.spacing(8.0);
        side_panel_sec.widget_full(&mut state.side_panel_behavior_menu, 44.0, ctx);
        side_panel_sec.spacing(8.0);
        side_panel_sec.widget_full(&mut state.side_panel_position_menu, 44.0, ctx);
        side_panel_sec.spacing(8.0);
        state.side_panel_width_spinbox.set_label("Default Width");
        side_panel_sec.widget_full(&mut state.side_panel_width_spinbox, 44.0, ctx);
        side_panel_sec.spacing(8.0);
        state.side_panel_border_gap_spinbox.set_label("Border Gap");
        side_panel_sec.widget_full(&mut state.side_panel_border_gap_spinbox, 44.0, ctx);
        side_panel_sec.spacing(8.0);
        state.side_panel_border_opacity_spinbox.set_label("Border Opacity");
        side_panel_sec.widget_full(&mut state.side_panel_border_opacity_spinbox, 44.0, ctx);
        side_panel_sec.spacing(8.0);
    });

    // 8. Effects Section
    builder.add_section(&mut final_pc, "Effects", true, |effects_sec| {
        effects_sec.spacing(8.0);
        state.transparency_toggle.set_toggled(state.transparency_enabled);
        effects_sec.widget_full(&mut state.transparency_toggle, cce_ui::layout::toggle_height(), ctx);
        effects_sec.spacing(8.0);
        state.blur_toggle.set_toggled(state.blur_enabled);
        effects_sec.widget_full(&mut state.blur_toggle, cce_ui::layout::toggle_height(), ctx);
        effects_sec.spacing(8.0);
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
        LayoutMessage::SetSidePanelBehavior(idx) => {
            if idx < 2 {
                state.side_panel_behavior_menu.selected = idx;
                let val = if idx == 0 { "above" } else { "inline" };
                write_config_value("side_panel_behavior", &format!("\"{}\"", val));
                send_ipc_command(&format!("layout side_panel_behavior {}", val));
            }
        }
        LayoutMessage::SetSidePanelPosition(idx) => {
            if idx < 2 {
                state.side_panel_position_menu.selected = idx;
                let val = if idx == 1 { "right" } else { "left" };
                write_config_value("side_panel_position", &format!("\"{}\"", val));
                send_ipc_command(&format!("layout side_panel_position {}", val));
            }
        }
        LayoutMessage::SetSidePanelWidth(v) => {
            let val = v.min(2000);
            state.side_panel_width = val;
            state.side_panel_width_spinbox.value = val as i32;
            write_config_value("side_panel_width", &val.to_string());
            send_ipc_command(&format!("layout side_panel_width {}", val));
        }
        LayoutMessage::SetSidePanelBorderGap(v) => {
            let val = v.min(500);
            state.side_panel_border_gap = val;
            state.side_panel_border_gap_spinbox.value = val as i32;
            write_config_value("side_panel_border_gap", &val.to_string());
            send_ipc_command(&format!("layout side_panel_border_gap {}", val));
        }
        LayoutMessage::SetSidePanelBorderOpacity(v) => {
            let val = v.min(100);
            state.side_panel_border_opacity = val;
            state.side_panel_border_opacity_spinbox.value = val as i32;
            write_config_value("side_panel_border_opacity", &val.to_string());
            send_ipc_command(&format!("layout side_panel_border_opacity {}", val));
        }
        LayoutMessage::ToggleTransparency => {
            state.transparency_enabled = !state.transparency_enabled;
            if state.transparency_enabled {
                write_transparency_config_value("opacity", "0.85");
            } else {
                write_transparency_config_value("opacity", "1.00");
            }
            send_ipc_command("reload");
            status_interface_reload();
        }
        LayoutMessage::ToggleBlur => {
            state.blur_enabled = !state.blur_enabled;
            let val = state.blur_enabled.to_string();
            write_config_value("window_blur", &val);
            write_config_value("border_blur", &val);
            send_ipc_command("reload");
        }
        LayoutMessage::Refreshed(new) => {
            let transparency_hover = state.transparency_toggle.hovered();
            let blur_hover = state.blur_toggle.hovered();
            *state = new;
            state.transparency_toggle.set_hovered(transparency_hover);
            state.blur_toggle.set_hovered(blur_hover);
        }
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
    fn test_parse_side_panel_width_default() {
        let content = "";
        let width = parse_u16_from(content, "side_panel_width", 360);
        assert_eq!(width, 360);
    }

    #[test]
    fn test_parse_side_panel_width_explicit() {
        let content = "side_panel_width = 450";
        let width = parse_u16_from(content, "side_panel_width", 360);
        assert_eq!(width, 450);
    }

    #[test]
    fn test_parse_side_panel_border_opacity_default() {
        let content = "";
        let opacity = parse_u16_from(content, "side_panel_border_opacity", 100);
        assert_eq!(opacity, 100);
    }

    #[test]
    fn test_parse_side_panel_border_opacity_explicit() {
        let content = "side_panel_border_opacity = 75";
        let opacity = parse_u16_from(content, "side_panel_border_opacity", 100);
        assert_eq!(opacity, 75);
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
        let mut layout = cce_ui::layout::ColumnLayout::new(20.0);
        let pc = view(&mut state, 10.0, 20.0, 800.0, 600.0, &[false, false, false, false], &mut layout, &mut cce_ui::context::UiContext::new());
        assert!(!pc.rects.is_empty() || !pc.texts.is_empty());
    }

    #[test]
    fn test_spinbox_right_click() {
        use cce_ui::widget::Element;
        let mut state = LayoutState::default();
        let mut ctx = cce_ui::context::UiContext::new();
        let sb = &mut state.spinboxes[0];
        sb.set_rect(0.0, 0.0, 100.0, 44.0);
        let res = sb.mouse_input(
            cce_ui::widget::MouseButton::Right,
            cce_ui::widget::ElementState::Pressed,
            50.0,
            20.0,
            &mut ctx,
        );
        assert!(res);
    }
}
