use std::fs;
use std::io::Write;
use crate::app::PageContent;
use clear_ui::layout::{PageLayoutBuilder, LayoutStrategy};
use clear_ui::widget::{
    ColorSelector, Spinbox, Element, ScrollingList, Dropdown, TextBox, Button, InfoBox, FontPreview, InteractiveListItem
};

const CONFIG_PATH: &str = "/home/lsgalante/.config/ccec/config.toml";

fn get_socket_path() -> String {
    match std::env::var("WAYLAND_DISPLAY") {
        Ok(display) => format!("/tmp/ccec-{}.sock", display),
        Err(_) => "/tmp/ccec.sock".to_string(),
    }
}

#[derive(Debug, Clone)]
pub struct InterfaceState {
    pub low_color: [u8; 3],
    pub high_color: [u8; 3],
    pub disabled_color: [u8; 3],
    pub separator_color: [u8; 3],
    pub visual_guides_color: [u8; 3],
    pub slider_track_color: [u8; 3],
    pub page_low_color: [u8; 3],
    pub color_borders_color: [u8; 3],
    pub normal_color: [u8; 3],
    pub paginator_sidebar_color: [u8; 3],
    pub primary_highlight_color: [u8; 3],
    pub paginator_tab_label_color: [u8; 3],
    pub toggle_enabled_color: [u8; 3],
    pub toggle_disabled_color: [u8; 3],
    pub color_selectors: Vec<ColorSelector>,
    pub paginator_tab_margin_x: u16,
    pub paginator_tab_margin_y: u16,
    pub tab_margin_spinbox_x: Spinbox,
    pub tab_margin_spinbox_y: Spinbox,
    pub paginator_tab_padding_x: u16,
    pub paginator_tab_padding_y: u16,
    pub tab_padding_spinbox_x: Spinbox,
    pub tab_padding_spinbox_y: Spinbox,
    // Typeface state fields
    pub typeface_loaded: bool,
    pub sans_serif: String,
    pub serif: String,
    pub monospace: String,
    pub window_borders: String,
    pub status_interface: String,
    pub fuzzel: String,
    pub terminal: String,
    pub paginator: String,
    pub all_fonts: Vec<String>,
    pub mono_fonts: Vec<String>,
    pub sans_box: TextBox,
    pub serif_box: TextBox,
    pub mono_box: TextBox,
    pub borders_box: TextBox,
    pub status_box: TextBox,
    pub fuzzel_box: TextBox,
    pub terminal_box: TextBox,
    pub paginator_box: TextBox,
    pub search_box: TextBox,
    pub selected_font: Option<String>,
    pub list_box: ScrollingList,
    pub borders_menu: Dropdown,
    pub status_menu: Dropdown,
    pub fuzzel_menu: Dropdown,
    pub terminal_menu: Dropdown,
    pub paginator_menu: Dropdown,
    pub borders_size_box: Spinbox,
    pub status_size_box: Spinbox,
    pub fuzzel_size_box: Spinbox,
    pub terminal_size_box: Spinbox,
    pub paginator_size_box: Spinbox,
    pub font_buttons: Vec<InteractiveListItem>,
    pub copy_buttons: Vec<Button>,
}

impl Default for InterfaceState {
    fn default() -> Self {
        Self {
            low_color: [0x0a, 0x1a, 0x0e],
            high_color: [0x3e, 0x3e, 0x3e],
            disabled_color: [0x55, 0x55, 0x55],
            separator_color: [124, 124, 137],
            visual_guides_color: [0xff, 0x8c, 0x00],
            slider_track_color: [116, 116, 128],
            page_low_color: [71, 71, 81],
            color_borders_color: [124, 124, 137],
            normal_color: [0xcc, 0xcc, 0xd8],
            paginator_sidebar_color: [90, 90, 101],
            primary_highlight_color: [255, 255, 255],
            paginator_tab_label_color: [230, 230, 242],
            toggle_enabled_color: [104, 217, 165],
            toggle_disabled_color: [135, 135, 148],
            color_selectors: vec![
                ColorSelector::new([71, 71, 81]).with_label("Low Color"), // 0: Pages - Low Color
                ColorSelector::new([0x3e, 0x3e, 0x3e]).with_label("High Color"), // 1: Layout - High Color
                ColorSelector::new([0xff, 0x8c, 0x00]).with_label("Visual Guides"), // 2: Layout - Visual Guides
                ColorSelector::new([0x55, 0x55, 0x55]).with_label("Disabled"), // 3: Status - Disabled
                ColorSelector::new([124, 124, 137]).with_label("Separators"), // 4: Status - Separators
                ColorSelector::new([116, 116, 128]).with_label("Slider Track"), // 5: Controls - Slider Track
                ColorSelector::new([124, 124, 137]).with_label("Borders"), // 6: Controls - Borders
                ColorSelector::new([0x0a, 0x1a, 0x0e]).with_label("Low Color"), // 7: Layout - Low Color
                ColorSelector::new([0xcc, 0xcc, 0xd8]).with_label("Normal"), // 8: Status - Normal
                ColorSelector::new([90, 90, 101]).with_label("Paginator Sidebar"), // 9: Controls - Paginator Sidebar
                ColorSelector::new([255, 255, 255]).with_label("Primary Highlight"), // 10: Controls - Primary Highlight
                ColorSelector::new([230, 230, 242]).with_label("Paginator Tab Label"), // 11: Controls - Paginator Tab Label
                ColorSelector::new([104, 217, 165]).with_label("Enabled"), // 12: Toggles - Enabled
                ColorSelector::new([135, 135, 148]).with_label("Disabled"), // 13: Toggles - Disabled
            ],
            paginator_tab_margin_x: 5,
            paginator_tab_margin_y: 10,
            tab_margin_spinbox_x: Spinbox::new(5, 0, 100, 1).with_label("Tab Margin X").with_unit("px"),
            tab_margin_spinbox_y: Spinbox::new(10, 0, 100, 1).with_label("Tab Margin Y").with_unit("px"),
            paginator_tab_padding_x: 10,
            paginator_tab_padding_y: 14,
            tab_padding_spinbox_x: Spinbox::new(10, 0, 100, 1).with_label("Tab Padding X").with_unit("px"),
            tab_padding_spinbox_y: Spinbox::new(14, 0, 100, 1).with_label("Tab Padding Y").with_unit("px"),
            typeface_loaded: false,
            sans_serif: String::new(),
            serif: String::new(),
            monospace: String::new(),
            window_borders: String::new(),
            status_interface: String::new(),
            fuzzel: String::new(),
            terminal: String::new(),
            paginator: String::new(),
            all_fonts: Vec::new(),
            mono_fonts: Vec::new(),
            sans_box: TextBox::default(),
            serif_box: TextBox::default(),
            mono_box: TextBox::default(),
            borders_box: TextBox::default(),
            status_box: TextBox::default(),
            fuzzel_box: TextBox::default(),
            terminal_box: TextBox::default(),
            paginator_box: TextBox::default(),
            search_box: TextBox::default(),
            selected_font: None,
            list_box: ScrollingList::new(24.0, 4.0),
            borders_menu: Dropdown::default(),
            status_menu: Dropdown::default(),
            fuzzel_menu: Dropdown::default(),
            terminal_menu: Dropdown::default(),
            paginator_menu: Dropdown::default(),
            borders_size_box: Spinbox::new(11, 6, 72, 1),
            status_size_box: Spinbox::new(11, 6, 72, 1),
            fuzzel_size_box: Spinbox::new(14, 6, 72, 1),
            terminal_size_box: Spinbox::new(12, 6, 72, 1),
            paginator_size_box: Spinbox::new(12, 6, 72, 1),
            font_buttons: Vec::new(),
            copy_buttons: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum InterfaceMessage {
    SetLowColor([u8; 3]),
    SetHighColor([u8; 3]),
    SetDisabledColor([u8; 3]),
    SetSeparatorColor([u8; 3]),
    SetVisualGuidesColor([u8; 3]),
    SetSliderTrackColor([u8; 3]),
    SetPageLowColor([u8; 3]),
    SetColorBordersColor([u8; 3]),
    SetNormalColor([u8; 3]),
    SetPaginatorSidebarColor([u8; 3]),
    SetPrimaryHighlightColor([u8; 3]),
    SetPaginatorTabLabelColor([u8; 3]),
    SetToggleEnabledColor([u8; 3]),
    SetToggleDisabledColor([u8; 3]),
    SetTabMarginX(u16),
    SetTabMarginY(u16),
    SetTabPaddingX(u16),
    SetTabPaddingY(u16),
    PickLowColor,
    PickHighColor,
    PickDisabledColor,
    PickSeparatorColor,
    PickVisualGuides,
    PickSliderTrackColor,
    PickPageLowColor,
    PickColorBordersColor,
    PickNormalColor,
    PickPaginatorSidebarColor,
    PickPrimaryHighlightColor,
    PickPaginatorTabLabelColor,
    PickToggleEnabledColor,
    PickToggleDisabledColor,
    Refreshed(InterfaceState),
    TypefaceRefreshed(InterfaceState),
    SetSans(String),
    SetSerif(String),
    SetMono(String),
    SetBorders(String),
    SetStatus(String),
    SetFuzzel(String),
    SetTerminal(String),
    SetPaginator(String),
    SetSearch(String),
    SelectFont(String),
    CopyFontName(String),
    SetBordersMenu(usize),
    SetStatusMenu(usize),
    SetFuzzelMenu(usize),
    SetTerminalMenu(usize),
    SetPaginatorMenu(usize),
    SetBordersSize(i32),
    SetStatusSize(i32),
    SetFuzzelSize(i32),
    SetTerminalSize(i32),
    SetPaginatorSize(i32),
}

pub fn read_interface_config() -> InterfaceState {
    let content = fs::read_to_string(CONFIG_PATH).unwrap_or_default();
    let has_low = content.lines().any(|l| l.trim().starts_with("low_color"));
    let bg = if has_low {
        parse_color_from_key(&content, "low_color", [0x0a, 0x1a, 0x0e])
    } else {
        parse_color_from_key(&content, "background_color", [0x0a, 0x1a, 0x0e])
    };
    
    let has_high = content.lines().any(|l| l.trim().starts_with("high_color"));
    let border = if has_high {
        parse_color_from_key(&content, "high_color", [0x3e, 0x3e, 0x3e])
    } else {
        parse_color_from_key(&content, "border_color", [0x3e, 0x3e, 0x3e])
    };
    
    let disabled = parse_color_from_key(&content, "disabled_color", [0x55, 0x55, 0x55]);
    
    let separator = parse_color_from_key(&content, "status_separator_color", [124, 124, 137]);
    
    let visual_guides = parse_color_from_key(&content, "visual_guides_color", [0xff, 0x8c, 0x00]);
    
    let slider_track = parse_color_from_key(&content, "slider_track_color", [116, 116, 128]);

    let page_low = parse_color_from_key(&content, "page_low_color", [71, 71, 81]);

    let color_borders = parse_color_from_key(&content, "color_borders_color", [124, 124, 137]);

    let normal = parse_color_from_key(&content, "status_normal_color", [0xcc, 0xcc, 0xd8]);

    let paginator_sidebar = parse_color_from_key(&content, "paginator_sidebar_color", [90, 90, 101]);

    let primary_highlight = parse_color_from_key(&content, "primary_highlight_color", [255, 255, 255]);

    let paginator_tab_label = parse_color_from_key(&content, "paginator_tab_label_color", [230, 230, 242]);

    let toggle_enabled = parse_color_from_key(&content, "toggle_enabled_color", [104, 217, 165]);

    let toggle_disabled = parse_color_from_key(&content, "toggle_disabled_color", [135, 135, 148]);
    
    let paginator_tab_margin_general = parse_u16_from(&content, "paginator_tab_margin", 999);
    let paginator_tab_margin_x = parse_u16_from(&content, "paginator_tab_margin_x", if paginator_tab_margin_general != 999 { paginator_tab_margin_general } else { 5 });
    let paginator_tab_margin_y = parse_u16_from(&content, "paginator_tab_margin_y", if paginator_tab_margin_general != 999 { paginator_tab_margin_general } else { 10 });
    let paginator_tab_padding_x = parse_u16_from(&content, "paginator_tab_padding_x", 10);
    let paginator_tab_padding_y = parse_u16_from(&content, "paginator_tab_padding_y", 14);
    
    InterfaceState {
        low_color: bg,
        high_color: border,
        disabled_color: disabled,
        separator_color: separator,
        visual_guides_color: visual_guides,
        slider_track_color: slider_track,
        page_low_color: page_low,
        color_borders_color: color_borders,
        normal_color: normal,
        paginator_sidebar_color: paginator_sidebar,
        primary_highlight_color: primary_highlight,
        paginator_tab_label_color: paginator_tab_label,
        toggle_enabled_color: toggle_enabled,
        toggle_disabled_color: toggle_disabled,
        color_selectors: vec![
            ColorSelector::new(page_low).with_label("Low Color"), // 0: Pages - Low Color
            ColorSelector::new(border).with_label("High Color"), // 1: Layout - High Color
            ColorSelector::new(visual_guides).with_label("Visual Guides"), // 2: Layout - Visual Guides
            ColorSelector::new(disabled).with_label("Disabled"), // 3: Status - Disabled
            ColorSelector::new(separator).with_label("Separators"), // 4: Status - Separators
            ColorSelector::new(slider_track).with_label("Slider Track"), // 5: Controls - Slider Track
            ColorSelector::new(color_borders).with_label("Borders"), // 6: Controls - Borders
            ColorSelector::new(bg).with_label("Low Color"), // 7: Layout - Low Color
            ColorSelector::new(normal).with_label("Normal"), // 8: Status - Normal
            ColorSelector::new(paginator_sidebar).with_label("Paginator Sidebar"), // 9: Controls - Paginator Sidebar
            ColorSelector::new(primary_highlight).with_label("Primary Highlight"), // 10: Controls - Primary Highlight
            ColorSelector::new(paginator_tab_label).with_label("Paginator Tab Label"), // 11: Controls - Paginator Tab Label
            ColorSelector::new(toggle_enabled).with_label("Enabled"), // 12: Toggles - Enabled
            ColorSelector::new(toggle_disabled).with_label("Disabled"), // 13: Toggles - Disabled
        ],
        paginator_tab_margin_x,
        paginator_tab_margin_y,
        tab_margin_spinbox_x: Spinbox::new(paginator_tab_margin_x as i32, 0, 100, 1).with_label("Tab Margin X").with_unit("px"),
        tab_margin_spinbox_y: Spinbox::new(paginator_tab_margin_y as i32, 0, 100, 1).with_label("Tab Margin Y").with_unit("px"),
        paginator_tab_padding_x,
        paginator_tab_padding_y,
        tab_padding_spinbox_x: Spinbox::new(paginator_tab_padding_x as i32, 0, 100, 1).with_label("Tab Padding X").with_unit("px"),
        tab_padding_spinbox_y: Spinbox::new(paginator_tab_padding_y as i32, 0, 100, 1).with_label("Tab Padding Y").with_unit("px"),
        typeface_loaded: false,
        sans_serif: String::new(),
        serif: String::new(),
        monospace: String::new(),
        window_borders: String::new(),
        status_interface: String::new(),
        fuzzel: String::new(),
        terminal: String::new(),
        paginator: String::new(),
        all_fonts: Vec::new(),
        mono_fonts: Vec::new(),
        sans_box: TextBox::default(),
        serif_box: TextBox::default(),
        mono_box: TextBox::default(),
        borders_box: TextBox::default(),
        status_box: TextBox::default(),
        fuzzel_box: TextBox::default(),
        terminal_box: TextBox::default(),
        paginator_box: TextBox::default(),
        search_box: TextBox::default(),
        selected_font: None,
        list_box: ScrollingList::new(24.0, 4.0),
        borders_menu: Dropdown::default(),
        status_menu: Dropdown::default(),
        fuzzel_menu: Dropdown::default(),
        terminal_menu: Dropdown::default(),
        paginator_menu: Dropdown::default(),
        borders_size_box: Spinbox::new(11, 6, 72, 1),
        status_size_box: Spinbox::new(11, 6, 72, 1),
        fuzzel_size_box: Spinbox::new(14, 6, 72, 1),
        terminal_size_box: Spinbox::new(12, 6, 72, 1),
        paginator_size_box: Spinbox::new(12, 6, 72, 1),
        font_buttons: Vec::new(),
        copy_buttons: Vec::new(),
    }
}

fn parse_color_from_key(content: &str, key: &str, default: [u8; 3]) -> [u8; 3] {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(key) {
            let rest = rest.trim_start_matches(|c: char| c == ' ' || c == '=' || c == '"');
            let hex = rest.trim_end_matches('"').trim().trim_start_matches('#');
            return parse_hex(hex);
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

pub fn write_config_value(key: &str, value: &str) -> bool {
    write_config_value_path(CONFIG_PATH, key, value)
}

pub fn write_config_value_path(path: &str, key: &str, value: &str) -> bool {
    let content = fs::read_to_string(path).unwrap_or_default();
    let old_key = match key {
        "low_color" => "background_color",
        "high_color" => "border_color",
        _ => "",
    };
    let new_line = format!("{} = {}", key, value);
    let mut found = false;
    let updated: String = content.lines()
        .map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with(key) {
                found = true;
                new_line.clone()
            } else if !old_key.is_empty() && trimmed.starts_with(old_key) {
                found = true;
                new_line.clone()
            } else {
                line.to_string()
            }
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
        fs::write(path, result).is_ok()
    } else { fs::write(path, updated).is_ok() }
}

fn send_ipc_command(cmd: &str) {
    if let Ok(mut stream) = std::os::unix::net::UnixStream::connect(get_socket_path()) {
        let _ = stream.write_all(format!("{}\n", cmd).as_bytes());
    }
}

fn apply_background(rgb: [u8; 3]) {
    let _ = std::process::Command::new("pkill").args(["-x", "swaybg"]).status();
    std::thread::sleep(std::time::Duration::from_millis(100));
    let hex = format!("{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2]);
    let _ = std::process::Command::new("swaybg").arg("-c").arg(&hex).spawn();
    write_config_value("low_color", &format!("\"#{}\"", hex));
    send_ipc_command(&format!("layout low_color #{}", hex));
}

fn apply_border_color(rgb: [u8; 3]) {
    let hex = format!("\"#{:02x}{:02x}{:02x}\"", rgb[0], rgb[1], rgb[2]);
    write_config_value("high_color", &hex);
    send_ipc_command(&format!("layout high_color #{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2]));
}

fn apply_disabled_color(rgb: [u8; 3]) {
    let hex = format!("\"#{:02x}{:02x}{:02x}\"", rgb[0], rgb[1], rgb[2]);
    write_config_value("disabled_color", &hex);
    send_ipc_command(&format!("layout disabled_color #{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2]));
}

fn status_interface_reload() {
    let _ = std::process::Command::new("pkill")
        .args(["-f", "clear-status-interface"])
        .status();
    std::thread::sleep(std::time::Duration::from_millis(150));
    send_ipc_command("spawn clear-status-interface");
}

fn apply_separator_color(rgb: [u8; 3]) {
    let hex = format!("\"#{:02x}{:02x}{:02x}\"", rgb[0], rgb[1], rgb[2]);
    write_config_value("status_separator_color", &hex);
    send_ipc_command(&format!("layout status_separator_color #{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2]));
    status_interface_reload();
}

fn apply_visual_guides_color(rgb: [u8; 3]) {
    let hex = format!("\"#{:02x}{:02x}{:02x}\"", rgb[0], rgb[1], rgb[2]);
    write_config_value("visual_guides_color", &hex);
}

fn apply_slider_track_color(rgb: [u8; 3]) {
    let hex = format!("\"#{:02x}{:02x}{:02x}\"", rgb[0], rgb[1], rgb[2]);
    write_config_value("slider_track_color", &hex);
    let r = clear_ui::color::srgb_to_linear(rgb[0] as f32 / 255.0);
    let g = clear_ui::color::srgb_to_linear(rgb[1] as f32 / 255.0);
    let b = clear_ui::color::srgb_to_linear(rgb[2] as f32 / 255.0);
    clear_ui::color::set_slider_track([r, g, b, 1.0]);
}

fn apply_page_low_color(rgb: [u8; 3]) {
    let hex = format!("\"#{:02x}{:02x}{:02x}\"", rgb[0], rgb[1], rgb[2]);
    write_config_value("page_low_color", &hex);
    let r = clear_ui::color::srgb_to_linear(rgb[0] as f32 / 255.0);
    let g = clear_ui::color::srgb_to_linear(rgb[1] as f32 / 255.0);
    let b = clear_ui::color::srgb_to_linear(rgb[2] as f32 / 255.0);
    clear_ui::color::set_page_low_color([r, g, b, 1.0]);
}

fn apply_color_borders_color(rgb: [u8; 3]) {
    let hex = format!("\"#{:02x}{:02x}{:02x}\"", rgb[0], rgb[1], rgb[2]);
    write_config_value("color_borders_color", &hex);
    let r = clear_ui::color::srgb_to_linear(rgb[0] as f32 / 255.0);
    let g = clear_ui::color::srgb_to_linear(rgb[1] as f32 / 255.0);
    let b = clear_ui::color::srgb_to_linear(rgb[2] as f32 / 255.0);
    clear_ui::color::set_color_borders_color([r, g, b, 1.0]);
}

fn apply_normal_color(rgb: [u8; 3]) {
    let hex = format!("\"#{:02x}{:02x}{:02x}\"", rgb[0], rgb[1], rgb[2]);
    write_config_value("status_normal_color", &hex);
    status_interface_reload();
}

fn apply_paginator_sidebar_color(rgb: [u8; 3]) {
    let hex = format!("\"#{:02x}{:02x}{:02x}\"", rgb[0], rgb[1], rgb[2]);
    write_config_value("paginator_sidebar_color", &hex);
    let r = clear_ui::color::srgb_to_linear(rgb[0] as f32 / 255.0);
    let g = clear_ui::color::srgb_to_linear(rgb[1] as f32 / 255.0);
    let b = clear_ui::color::srgb_to_linear(rgb[2] as f32 / 255.0);
    clear_ui::color::set_sidebar_bg_color([r, g, b, 1.0]);
}

fn apply_primary_highlight_color(rgb: [u8; 3]) {
    let hex = format!("\"#{:02x}{:02x}{:02x}\"", rgb[0], rgb[1], rgb[2]);
    write_config_value("primary_highlight_color", &hex);
    let r = clear_ui::color::srgb_to_linear(rgb[0] as f32 / 255.0);
    let g = clear_ui::color::srgb_to_linear(rgb[1] as f32 / 255.0);
    let b = clear_ui::color::srgb_to_linear(rgb[2] as f32 / 255.0);
    clear_ui::color::set_highlight_primary_color([r, g, b, 0.12]);
}

fn apply_paginator_tab_label_color(rgb: [u8; 3]) {
    let hex = format!("\"#{:02x}{:02x}{:02x}\"", rgb[0], rgb[1], rgb[2]);
    write_config_value("paginator_tab_label_color", &hex);
    let r = clear_ui::color::srgb_to_linear(rgb[0] as f32 / 255.0);
    let g = clear_ui::color::srgb_to_linear(rgb[1] as f32 / 255.0);
    let b = clear_ui::color::srgb_to_linear(rgb[2] as f32 / 255.0);
    clear_ui::color::set_paginator_tab_label_color([r, g, b, 1.0]);
}

fn apply_toggle_enabled_color(rgb: [u8; 3]) {
    let hex = format!("\"#{:02x}{:02x}{:02x}\"", rgb[0], rgb[1], rgb[2]);
    write_config_value("toggle_enabled_color", &hex);
    let r = clear_ui::color::srgb_to_linear(rgb[0] as f32 / 255.0);
    let g = clear_ui::color::srgb_to_linear(rgb[1] as f32 / 255.0);
    let b = clear_ui::color::srgb_to_linear(rgb[2] as f32 / 255.0);
    clear_ui::color::set_toggle_on_color([r, g, b, 1.0]);
}

fn apply_toggle_disabled_color(rgb: [u8; 3]) {
    let hex = format!("\"#{:02x}{:02x}{:02x}\"", rgb[0], rgb[1], rgb[2]);
    write_config_value("toggle_disabled_color", &hex);
    let r = clear_ui::color::srgb_to_linear(rgb[0] as f32 / 255.0);
    let g = clear_ui::color::srgb_to_linear(rgb[1] as f32 / 255.0);
    let b = clear_ui::color::srgb_to_linear(rgb[2] as f32 / 255.0);
    clear_ui::color::set_toggle_off_color([r, g, b, 1.0]);
}

fn apply_paginator_tab_margin_x(margin: u16) {
    write_config_value("paginator_tab_margin_x", &margin.to_string());
    send_ipc_command(&format!("layout paginator_tab_margin_x {}", margin));
}

fn apply_paginator_tab_margin_y(margin: u16) {
    write_config_value("paginator_tab_margin_y", &margin.to_string());
    send_ipc_command(&format!("layout paginator_tab_margin_y {}", margin));
}

fn apply_paginator_tab_padding_x(padding: u16) {
    write_config_value("paginator_tab_padding_x", &padding.to_string());
    send_ipc_command(&format!("layout paginator_tab_padding_x {}", padding));
}

fn apply_paginator_tab_padding_y(padding: u16) {
    write_config_value("paginator_tab_padding_y", &padding.to_string());
    send_ipc_command(&format!("layout paginator_tab_padding_y {}", padding));
}

const FONTS_CONF_PATH: &str = "/home/lsgalante/.config/fontconfig/fonts.conf";

fn parse_font_for_alias(content: &str, alias: &str) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    for i in 0..lines.len() {
        let line = lines[i].trim();
        if line.contains("<test") && line.contains("name=\"family\"") && line.contains(&format!("<string>{}</string>", alias)) {
            for j in (i + 1)..(i + 6).min(lines.len()) {
                let next_line = lines[j].trim();
                if next_line.contains("<edit") {
                    for k in (j + 1)..(j + 6).min(lines.len()) {
                        let str_line = lines[k].trim();
                        if str_line.contains("<string>") && str_line.contains("</string>") {
                            if let Some(start) = str_line.find("<string>") {
                                if let Some(end) = str_line.find("</string>") {
                                    let font = &str_line[start + 8..end];
                                    return Some(font.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

pub fn read_preferred_fonts() -> (String, String, String, String, String, String, String, String) {
    let content = fs::read_to_string(FONTS_CONF_PATH).unwrap_or_default();
    
    let sans = parse_font_for_alias(&content, "sans-serif").unwrap_or_else(|| "Noto Sans".to_string());
    let serif = parse_font_for_alias(&content, "serif").unwrap_or_else(|| "Noto Serif".to_string());
    let mono = parse_font_for_alias(&content, "monospace").unwrap_or_else(|| "Noto Sans Mono".to_string());
    let borders = parse_font_for_alias(&content, "window-borders").unwrap_or_else(|| "Noto Sans".to_string());
    let status = parse_font_for_alias(&content, "status-interface").unwrap_or_else(|| "Noto Sans".to_string());
    let fuzzel_font = parse_font_for_alias(&content, "fuzzel").unwrap_or_else(|| "Noto Sans".to_string());
    let term = parse_font_for_alias(&content, "terminal").unwrap_or_else(|| "Noto Sans Mono".to_string());
    let paginator = parse_font_for_alias(&content, "paginator-tab-labels").unwrap_or_else(|| "Noto Sans Mono".to_string());
    
    (sans, serif, mono, borders, status, fuzzel_font, term, paginator)
}

pub fn save_preferred_fonts(
    sans: &str,
    serif: &str,
    mono: &str,
    borders: &str,
    status: &str,
    fuzzel: &str,
    terminal: &str,
    paginator: &str,
) {
    let content = fs::read_to_string(FONTS_CONF_PATH).unwrap_or_default();
    
    let mut dirs = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("<dir>") && trimmed.ends_with("</dir>") {
            dirs.push(trimmed.to_string());
        }
    }
    if dirs.is_empty() {
        dirs.push("<dir>~/Dropbox/Fonts</dir>".to_string());
    }
    
    let mut new_content = String::new();
    new_content.push_str("<?xml version=\"1.0\"?>\n");
    new_content.push_str("<!DOCTYPE fontconfig SYSTEM \"fonts.dtd\">\n");
    new_content.push_str("<fontconfig>\n");
    
    for dir in dirs {
        new_content.push_str(&format!("    {}\n", dir));
    }
    
    // Sans-Serif
    new_content.push_str("    <match target=\"pattern\">\n");
    new_content.push_str("        <test qual=\"any\" name=\"family\"><string>sans-serif</string></test>\n");
    new_content.push_str("        <edit name=\"family\" mode=\"assign\" binding=\"same\">\n");
    new_content.push_str(&format!("            <string>{}</string>\n", sans));
    new_content.push_str("        </edit>\n");
    new_content.push_str("    </match>\n");
    
    // Serif
    new_content.push_str("    <match target=\"pattern\">\n");
    new_content.push_str("        <test qual=\"any\" name=\"family\"><string>serif</string></test>\n");
    new_content.push_str("        <edit name=\"family\" mode=\"assign\" binding=\"same\">\n");
    new_content.push_str(&format!("            <string>{}</string>\n", serif));
    new_content.push_str("        </edit>\n");
    new_content.push_str("    </match>\n");
    
    // Monospace
    new_content.push_str("    <match target=\"pattern\">\n");
    new_content.push_str("        <test qual=\"any\" name=\"family\"><string>monospace</string></test>\n");
    new_content.push_str("        <edit name=\"family\" mode=\"assign\" binding=\"same\">\n");
    new_content.push_str(&format!("            <string>{}</string>\n", mono));
    new_content.push_str("        </edit>\n");
    new_content.push_str("    </match>\n");
    
    // Window Borders
    new_content.push_str("    <match target=\"pattern\">\n");
    new_content.push_str("        <test qual=\"any\" name=\"family\"><string>window-borders</string></test>\n");
    new_content.push_str("        <edit name=\"family\" mode=\"assign\" binding=\"same\">\n");
    new_content.push_str(&format!("            <string>{}</string>\n", borders));
    new_content.push_str("        </edit>\n");
    new_content.push_str("    </match>\n");
    
    // Status Interface
    new_content.push_str("    <match target=\"pattern\">\n");
    new_content.push_str("        <test qual=\"any\" name=\"family\"><string>status-interface</string></test>\n");
    new_content.push_str("        <edit name=\"family\" mode=\"assign\" binding=\"same\">\n");
    new_content.push_str(&format!("            <string>{}</string>\n", status));
    new_content.push_str("        </edit>\n");
    new_content.push_str("    </match>\n");
    
    // Fuzzel
    new_content.push_str("    <match target=\"pattern\">\n");
    new_content.push_str("        <test qual=\"any\" name=\"family\"><string>fuzzel</string></test>\n");
    new_content.push_str("        <edit name=\"family\" mode=\"assign\" binding=\"same\">\n");
    new_content.push_str(&format!("            <string>{}</string>\n", fuzzel));
    new_content.push_str("        </edit>\n");
    new_content.push_str("    </match>\n");
    
    // Terminal
    new_content.push_str("    <match target=\"pattern\">\n");
    new_content.push_str("        <test qual=\"any\" name=\"family\"><string>terminal</string></test>\n");
    new_content.push_str("        <edit name=\"family\" mode=\"assign\" binding=\"same\">\n");
    new_content.push_str(&format!("            <string>{}</string>\n", terminal));
    new_content.push_str("        </edit>\n");
    new_content.push_str("    </match>\n");
    
    // Paginator Tab Labels
    new_content.push_str("    <match target=\"pattern\">\n");
    new_content.push_str("        <test qual=\"any\" name=\"family\"><string>paginator-tab-labels</string></test>\n");
    new_content.push_str("        <edit name=\"family\" mode=\"assign\" binding=\"same\">\n");
    new_content.push_str(&format!("            <string>{}</string>\n", paginator));
    new_content.push_str("        </edit>\n");
    new_content.push_str("    </match>\n");
    
    new_content.push_str("</fontconfig>\n");
    
    let _ = fs::write(FONTS_CONF_PATH, new_content);
}

pub fn parse_u16_from(content: &str, key: &str, default: u16) -> u16 {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(key) {
            let rest = rest.trim_start_matches(|c: char| c == ' ' || c == '=' || c == '"');
            let val_str = rest.trim_end_matches('"').trim();
            if let Ok(val) = val_str.parse::<u16>() {
                return val;
            }
        }
    }
    default
}

fn read_border_font_size() -> Option<u16> {
    let content = fs::read_to_string("/home/lsgalante/.config/ccec/config.toml").ok()?;
    Some(parse_u16_from(&content, "border_font_size", 11))
}

fn read_status_size() -> Option<u16> {
    let content = fs::read_to_string("/home/lsgalante/.config/ccec/config.toml").ok()?;
    Some(parse_u16_from(&content, "status_font_size", 11))
}

fn write_status_size(size: u16) {
    write_config_value("status_font_size", &size.to_string());
    let _ = std::process::Command::new("pkill")
        .args(["-f", "clear-status-interface"])
        .spawn();
}

fn read_fuzzel_size() -> Option<u16> {
    let ini = fs::read_to_string("/home/lsgalante/.config/fuzzel/fuzzel.ini").ok()?;
    for line in ini.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("font") {
            if let Some(pos) = trimmed.find("size=") {
                let size_str = &trimmed[pos + 5..];
                let end_pos = size_str.find(|c: char| !c.is_ascii_digit()).unwrap_or(size_str.len());
                return size_str[..end_pos].parse::<u16>().ok();
            }
        }
    }
    None
}

fn write_fuzzel_size(size: u16) {
    let path = "/home/lsgalante/.config/fuzzel/fuzzel.ini";
    let ini = fs::read_to_string(path).unwrap_or_default();
    let mut new_lines = Vec::new();
    for line in ini.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("font") {
            if let Some(pos) = line.find("size=") {
                let mut new_line = line[..pos + 5].to_string();
                new_line.push_str(&size.to_string());
                let size_str = &line[pos + 5..];
                let skip = size_str.find(|c: char| !c.is_ascii_digit()).unwrap_or(size_str.len());
                new_line.push_str(&size_str[skip..]);
                new_lines.push(new_line);
            } else {
                new_lines.push(line.to_string());
            }
        } else {
            new_lines.push(line.to_string());
        }
    }
    let _ = fs::write(path, new_lines.join("\n"));
}

fn read_terminal_size() -> Option<u16> {
    let ini = fs::read_to_string("/home/lsgalante/.config/foot/foot.ini").ok()?;
    for line in ini.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("font") {
            if let Some(pos) = trimmed.find("terminal:size=") {
                let size_str = &trimmed[pos + 14..];
                let end_pos = size_str.find(|c: char| !c.is_ascii_digit()).unwrap_or(size_str.len());
                return size_str[..end_pos].parse::<u16>().ok();
            }
        }
    }
    None
}

fn write_terminal_size(size: u16) {
    let path = "/home/lsgalante/.config/foot/foot.ini";
    let ini = fs::read_to_string(path).unwrap_or_default();
    let mut new_lines = Vec::new();
    for line in ini.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("font") {
            if let Some(pos) = line.find("terminal:size=") {
                let mut new_line = line[..pos + 14].to_string();
                new_line.push_str(&size.to_string());
                let size_str = &line[pos + 14..];
                let skip = size_str.find(|c: char| !c.is_ascii_digit()).unwrap_or(size_str.len());
                new_line.push_str(&size_str[skip..]);
                new_lines.push(new_line);
            } else {
                new_lines.push(line.to_string());
            }
        } else {
            new_lines.push(line.to_string());
        }
    }
    let _ = fs::write(path, new_lines.join("\n"));
}

fn read_paginator_size() -> Option<u16> {
    let content = fs::read_to_string("/home/lsgalante/.config/ccec/config.toml").ok()?;
    Some(parse_u16_from(&content, "paginator_font_size", 12))
}

fn write_paginator_size(size: u16) {
    write_config_value("paginator_font_size", &size.to_string());
}

fn parse_families(output: Option<std::process::Output>) -> Vec<String> {
    let mut families = Vec::new();
    if let Some(o) = output {
        let text = String::from_utf8_lossy(&o.stdout);
        for line in text.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                let family = trimmed.split(',').next().unwrap_or(trimmed).to_string();
                if !family.is_empty() && !families.contains(&family) {
                    families.push(family);
                }
            }
        }
    }
    families.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
    families
}

pub async fn fetch_typeface_state() -> InterfaceState {
    let (sans, serif, mono, borders, status, fuzzel_font, term, paginator_font) = read_preferred_fonts();
    
    let all_output = tokio::process::Command::new("fc-list")
        .args([":", "family"])
        .output().await.ok();
    let all_fonts = parse_families(all_output);
    
    let mono_output = tokio::process::Command::new("fc-list")
        .args([":spacing=100", "family"])
        .output().await.ok();
    let mono_fonts = parse_families(mono_output);
    
    let selected_font = all_fonts.first().cloned();

    let determine_dropdown_index = |font: &str, sans: &str, serif: &str, mono: &str| -> usize {
        if font == sans {
            0
        } else if font == serif {
            1
        } else if font == mono {
            2
        } else {
            3
        }
    };

    let borders_idx = determine_dropdown_index(&borders, &sans, &serif, &mono);
    let status_idx = determine_dropdown_index(&status, &sans, &serif, &mono);
    let fuzzel_idx = determine_dropdown_index(&fuzzel_font, &sans, &serif, &mono);
    let terminal_idx = determine_dropdown_index(&term, &sans, &serif, &mono);
    let paginator_idx = determine_dropdown_index(&paginator_font, &sans, &serif, &mono);

    let menu_options = vec![
        "Sans-Serif".to_string(),
        "Serif".to_string(),
        "Monospace".to_string(),
        "Other".to_string(),
    ];

    let mut borders_box = TextBox::new(borders.clone()).with_label("Window Borders").with_width(300.0);
    borders_box.disabled = borders_idx != 3;

    let mut status_box = TextBox::new(status.clone()).with_label("Status Interface").with_width(300.0);
    status_box.disabled = status_idx != 3;

    let mut fuzzel_box = TextBox::new(fuzzel_font.clone()).with_label("Fuzzel").with_width(300.0);
    fuzzel_box.disabled = fuzzel_idx != 3;

    let mut terminal_box = TextBox::new(term.clone()).with_label("Terminal").with_width(300.0);
    terminal_box.disabled = terminal_idx != 3;

    let mut paginator_box = TextBox::new(paginator_font.clone()).with_label("Paginator Tab Labels").with_width(300.0);
    paginator_box.disabled = paginator_idx != 3;

    let borders_size = read_border_font_size().unwrap_or(11);
    let status_size = read_status_size().unwrap_or(11);
    let fuzzel_size = read_fuzzel_size().unwrap_or(14);
    let terminal_size = read_terminal_size().unwrap_or(12);
    let paginator_size = read_paginator_size().unwrap_or(12);

    let mut state = InterfaceState::default();
    state.typeface_loaded = true;
    state.sans_serif = sans.clone();
    state.serif = serif.clone();
    state.monospace = mono.clone();
    state.window_borders = borders;
    state.status_interface = status;
    state.fuzzel = fuzzel_font;
    state.terminal = term;
    state.paginator = paginator_font;
    state.all_fonts = all_fonts;
    state.mono_fonts = mono_fonts;
    state.sans_box = TextBox::new(sans).with_label("Sans-Serif");
    state.serif_box = TextBox::new(serif).with_label("Serif");
    state.mono_box = TextBox::new(mono).with_label("Monospace");
    state.borders_box = borders_box;
    state.status_box = status_box;
    state.fuzzel_box = fuzzel_box;
    state.terminal_box = terminal_box;
    state.paginator_box = paginator_box;
    state.search_box = TextBox::new(String::new()).with_label("Filter Fonts");
    state.selected_font = selected_font;
    state.list_box = ScrollingList::new(24.0, 4.0);
    state.borders_menu = Dropdown::new(menu_options.clone(), borders_idx);
    state.status_menu = Dropdown::new(menu_options.clone(), status_idx);
    state.fuzzel_menu = Dropdown::new(menu_options.clone(), fuzzel_idx);
    state.terminal_menu = Dropdown::new(menu_options.clone(), terminal_idx);
    state.paginator_menu = Dropdown::new(menu_options, paginator_idx);
    state.borders_size_box = Spinbox::new(borders_size as i32, 6, 72, 1);
    state.status_size_box = Spinbox::new(status_size as i32, 6, 72, 1);
    state.fuzzel_size_box = Spinbox::new(fuzzel_size as i32, 6, 72, 1);
    state.terminal_size_box = Spinbox::new(terminal_size as i32, 6, 72, 1);
    state.paginator_size_box = Spinbox::new(paginator_size as i32, 6, 72, 1);
    state
}

pub fn view(state: &mut InterfaceState, cx: f32, cy: f32, cw: f32, ch: f32, sec_focused: &[bool], layout: &mut dyn LayoutStrategy) -> PageContent {
    let mut final_pc = PageContent::new();
    let sec_w = 320.0f32;
    let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(10);

    // 1. Pages Section
    builder.add_section(&mut final_pc, "Pages", false, |sec| {
        sec.spacing(8.0);
        state.color_selectors[0].color = state.page_low_color;
        sec.widget(&mut state.color_selectors[0], 12.0, 220.0, 40.0);
        sec.spacing(8.0);
    });

    // 2. Layout Section
    builder.add_section(&mut final_pc, "Layout", false, |sec| {
        sec.spacing(8.0);
        state.color_selectors[7].color = state.low_color;
        sec.widget(&mut state.color_selectors[7], 12.0, 220.0, 40.0);
        sec.spacing(8.0);
        state.color_selectors[1].color = state.high_color;
        sec.widget(&mut state.color_selectors[1], 12.0, 220.0, 40.0);
        sec.spacing(8.0);
        state.color_selectors[2].color = state.visual_guides_color;
        sec.widget(&mut state.color_selectors[2], 12.0, 220.0, 40.0);
        sec.spacing(8.0);
    });

    // 3. Status Section
    builder.add_section(&mut final_pc, "Status", false, |sec| {
        sec.spacing(8.0);
        state.color_selectors[8].color = state.normal_color;
        sec.widget(&mut state.color_selectors[8], 12.0, 220.0, 40.0);
        sec.spacing(8.0);
        state.color_selectors[3].color = state.disabled_color;
        sec.widget(&mut state.color_selectors[3], 12.0, 220.0, 40.0);
        sec.spacing(8.0);
        state.color_selectors[4].color = state.separator_color;
        sec.widget(&mut state.color_selectors[4], 12.0, 220.0, 40.0);
        sec.spacing(8.0);
    });

    // 4. Controls Section
    builder.add_section(&mut final_pc, "Controls", false, |sec| {
        sec.spacing(8.0);
        state.color_selectors[5].color = state.slider_track_color;
        sec.widget(&mut state.color_selectors[5], 12.0, 220.0, 40.0);
        sec.spacing(8.0);
        state.color_selectors[6].color = state.color_borders_color;
        sec.widget(&mut state.color_selectors[6], 12.0, 220.0, 40.0);
        sec.spacing(8.0);
    });

    // 5. Primary Highlight Section
    builder.add_section(&mut final_pc, "Primary Highlight", false, |sec| {
        sec.spacing(8.0);
        state.color_selectors[10].color = state.primary_highlight_color;
        sec.widget(&mut state.color_selectors[10], 12.0, 220.0, 40.0);
        sec.spacing(8.0);
    });

    // 5. Paginator Section
    builder.add_section(&mut final_pc, "Paginator", false, |sec| {
        sec.spacing(8.0);
        state.color_selectors[9].color = state.paginator_sidebar_color;
        sec.widget(&mut state.color_selectors[9], 12.0, 220.0, 40.0);
        sec.spacing(8.0);
        state.color_selectors[11].color = state.paginator_tab_label_color;
        sec.widget(&mut state.color_selectors[11], 12.0, 220.0, 40.0);
        state.tab_margin_spinbox_x.value = state.paginator_tab_margin_x as i32;
        sec.widget(&mut state.tab_margin_spinbox_x, 12.0, 200.0, 44.0);
        sec.spacing(8.0);
        state.tab_margin_spinbox_y.value = state.paginator_tab_margin_y as i32;
        sec.widget(&mut state.tab_margin_spinbox_y, 12.0, 200.0, 44.0);
        sec.spacing(8.0);
        state.tab_padding_spinbox_x.value = state.paginator_tab_padding_x as i32;
        sec.widget(&mut state.tab_padding_spinbox_x, 12.0, 200.0, 44.0);
        sec.spacing(8.0);
        state.tab_padding_spinbox_y.value = state.paginator_tab_padding_y as i32;
        sec.widget(&mut state.tab_padding_spinbox_y, 12.0, 200.0, 44.0);
        sec.spacing(8.0);
    });

    // 6. Toggles Section
    builder.add_section(&mut final_pc, "Toggles", false, |sec| {
        sec.spacing(8.0);
        state.color_selectors[12].color = state.toggle_enabled_color;
        sec.widget(&mut state.color_selectors[12], 12.0, 220.0, 40.0);
        sec.spacing(8.0);
        state.color_selectors[13].color = state.toggle_disabled_color;
        sec.widget(&mut state.color_selectors[13], 12.0, 220.0, 40.0);
        sec.spacing(8.0);
    });

    let widget_h = 26.0;
    const TEXT_DIM: [f32; 4] = [0.53, 0.53, 0.60, 1.0];

    // 7. System Typefaces Section
    builder.add_section(&mut final_pc, "System Typefaces", sec_focused.get(7).copied().unwrap_or(false), |sec| {
        sec.spacing(8.0);

        if !state.typeface_loaded {
            sec.text("Loading typefaces...", 12.0, 0.0, 12.0, TEXT_DIM);
            sec.spacing(18.0);
        } else {
            let inner_w = sec_w - 24.0;
            // Sans-Serif
            sec.widget(&mut state.sans_box, 12.0, inner_w, 44.0);
            sec.spacing(12.0);

            // Serif
            sec.widget(&mut state.serif_box, 12.0, inner_w, 44.0);
            sec.spacing(12.0);

            // Monospace
            sec.widget(&mut state.mono_box, 12.0, inner_w, 44.0);
            sec.spacing(8.0);
        }
    });

    // 8. Program Typefaces Section
    builder.add_section(&mut final_pc, "Program Typefaces", sec_focused.get(8).copied().unwrap_or(false), |sec| {
        sec.spacing(8.0);

        if !state.typeface_loaded {
            sec.text("Loading typefaces...", 12.0, 0.0, 12.0, TEXT_DIM);
            sec.spacing(18.0);
        } else {
            let inner_w = sec_w - 24.0;

            // Window Borders
            let start_y = sec.ay();
            let cols = sec.row_layout(2, 10.0);
            if cols.len() == 2 {
                state.borders_menu.set_row_rect(cols[0].0, cols[0].1);
                clear_ui::layout::render_widget(sec.pc, &mut state.borders_menu, cols[0].0, start_y, cols[0].1, widget_h);
                state.borders_size_box.set_row_rect(cols[1].0, cols[1].1);
                clear_ui::layout::render_widget(sec.pc, &mut state.borders_size_box, cols[1].0, start_y, cols[1].1, widget_h);
            }
            sec.spacing(widget_h);
            sec.widget(&mut state.borders_box, 12.0, inner_w, 44.0);
            sec.spacing(16.0);

            // Status Interface
            let start_y = sec.ay();
            let cols = sec.row_layout(2, 10.0);
            if cols.len() == 2 {
                state.status_menu.set_row_rect(cols[0].0, cols[0].1);
                clear_ui::layout::render_widget(sec.pc, &mut state.status_menu, cols[0].0, start_y, cols[0].1, widget_h);
                state.status_size_box.set_row_rect(cols[1].0, cols[1].1);
                clear_ui::layout::render_widget(sec.pc, &mut state.status_size_box, cols[1].0, start_y, cols[1].1, widget_h);
            }
            sec.spacing(widget_h);
            sec.widget(&mut state.status_box, 12.0, inner_w, 44.0);
            sec.spacing(16.0);

            // Fuzzel
            let start_y = sec.ay();
            let cols = sec.row_layout(2, 10.0);
            if cols.len() == 2 {
                state.fuzzel_menu.set_row_rect(cols[0].0, cols[0].1);
                clear_ui::layout::render_widget(sec.pc, &mut state.fuzzel_menu, cols[0].0, start_y, cols[0].1, widget_h);
                state.fuzzel_size_box.set_row_rect(cols[1].0, cols[1].1);
                clear_ui::layout::render_widget(sec.pc, &mut state.fuzzel_size_box, cols[1].0, start_y, cols[1].1, widget_h);
            }
            sec.spacing(widget_h);
            sec.widget(&mut state.fuzzel_box, 12.0, inner_w, 44.0);
            sec.spacing(16.0);

            // Terminal
            let start_y = sec.ay();
            let cols = sec.row_layout(2, 10.0);
            if cols.len() == 2 {
                state.terminal_menu.set_row_rect(cols[0].0, cols[0].1);
                clear_ui::layout::render_widget(sec.pc, &mut state.terminal_menu, cols[0].0, start_y, cols[0].1, widget_h);
                state.terminal_size_box.set_row_rect(cols[1].0, cols[1].1);
                clear_ui::layout::render_widget(sec.pc, &mut state.terminal_size_box, cols[1].0, start_y, cols[1].1, widget_h);
            }
            sec.spacing(widget_h);
            sec.widget(&mut state.terminal_box, 12.0, inner_w, 44.0);
            sec.spacing(16.0);

            // Paginator Tab Labels
            let start_y = sec.ay();
            let cols = sec.row_layout(2, 10.0);
            if cols.len() == 2 {
                state.paginator_menu.set_row_rect(cols[0].0, cols[0].1);
                clear_ui::layout::render_widget(sec.pc, &mut state.paginator_menu, cols[0].0, start_y, cols[0].1, widget_h);
                state.paginator_size_box.set_row_rect(cols[1].0, cols[1].1);
                clear_ui::layout::render_widget(sec.pc, &mut state.paginator_size_box, cols[1].0, start_y, cols[1].1, widget_h);
            }
            sec.spacing(widget_h);
            sec.widget(&mut state.paginator_box, 12.0, inner_w, 44.0);
            sec.spacing(8.0);
        }
    });

    // 9. Typefaces Section (List & Preview)
    builder.add_section(&mut final_pc, "Typefaces", sec_focused.get(9).copied().unwrap_or(false), |sec| {
        sec.spacing(12.0);

        if !state.typeface_loaded {
            sec.text("Loading installed fonts...", 12.0, 0.0, 12.0, TEXT_DIM);
            sec.spacing(18.0);
        } else {
            let inner_w = sec_w - 24.0;

            // 1. Search Box
            let search_x = sec.left + 12.0;
            state.search_box.set_row_rect(search_x, inner_w);
            let search_y = sec.ay();
            clear_ui::layout::render_widget(
                sec.pc,
                &mut state.search_box,
                search_x,
                search_y,
                inner_w,
                44.0,
            );
            sec.spacing(44.0 + 12.0);

            // 2. Scrolling List Box
            let list_box_x = sec.left + 12.0;
            let list_box_y = sec.ay();
            let list_box_h = 200.0;
            
            clear_ui::layout::render_widget(sec.pc, &mut state.list_box, list_box_x, list_box_y, inner_w, list_box_h);

            let query = state.search_box.text.to_lowercase();
            let matching_fonts: Vec<&String> = state.all_fonts.iter()
                .filter(|font| font.to_lowercase().contains(&query))
                .collect();

            if state.font_buttons.len() != matching_fonts.len() {
                state.font_buttons.clear();
                state.copy_buttons.clear();
                for _ in 0..matching_fonts.len() {
                    state.font_buttons.push(InteractiveListItem::new(""));
                    state.copy_buttons.push(Button::new_copy_icon(0.0, 0.0, 0.0, 0.0));
                }
            }

            let btn_h = 24.0;
            let list_inner_x = sec.left + 16.0;
            let list_inner_w = inner_w - 16.0;

            state.list_box.update_bounds(matching_fonts.len(), list_box_y, list_box_h);

            for (idx, font_name) in matching_fonts.iter().enumerate() {
                if let Some(draw_y) = state.list_box.get_item_draw_y(idx, 0.0) {
                    let is_selected = state.selected_font.as_ref() == Some(*font_name);
                    
                    let font_btn = &mut state.font_buttons[idx];
                    font_btn.title = font_name.to_string();
                    font_btn.selected = is_selected;
                    clear_ui::layout::render_widget(sec.pc, font_btn, list_inner_x, draw_y, list_inner_w - 44.0, btn_h);

                    let copy_btn = &mut state.copy_buttons[idx];
                    copy_btn.set_text("📋");
                    copy_btn.selected = is_selected;
                    clear_ui::layout::render_widget(sec.pc, copy_btn, list_inner_x + list_inner_w - 40.0, draw_y, 40.0, btn_h);
                }
            }

            if matching_fonts.is_empty() {
                sec.pc.text("No fonts match query", list_inner_x + 8.0, list_box_y + 16.0, 12.0, TEXT_DIM);
            }

            sec.spacing(list_box_h + 12.0);

            // 3. Info Box
            let info_h = 96.0;
            let info_y = sec.ay();
            let info_x = sec.left + 12.0;
            let mut info_box = InfoBox::new(
                "Font Directories & Installation",
                vec![
                    "• Active Directory: ~/Dropbox/Fonts".to_string(),
                    "• Place TTF/OTF files there to install new fonts.".to_string(),
                    "• Changes will be cached automatically by fontconfig.".to_string(),
                ],
            );
            clear_ui::layout::render_widget(sec.pc, &mut info_box, info_x, info_y, inner_w, info_h);
            sec.spacing(info_h + 12.0);

            // 4. Preview Card
            let card_x = sec.left + 12.0;
            if let Some(ref font_name) = state.selected_font {
                let card_h = 240.0;
                let card_y = sec.ay();
                let mut font_preview = FontPreview::new(font_name.clone());
                clear_ui::layout::render_widget(sec.pc, &mut font_preview, card_x, card_y, inner_w, card_h);
                sec.spacing(card_h + 8.0);
            } else {
                let text_y = sec.ay() + 20.0;
                sec.pc.text("Select a font to preview", card_x + 12.0, text_y, 13.0, TEXT_DIM);
                sec.spacing(40.0);
            }
        }
    });

    final_pc
}


pub fn update(state: &mut InterfaceState, msg: InterfaceMessage) {
    match msg {
        InterfaceMessage::SetLowColor(rgb) => {
            state.low_color = rgb;
            apply_background(rgb);
        }
        InterfaceMessage::SetPageLowColor(rgb) => {
            state.page_low_color = rgb;
            apply_page_low_color(rgb);
        }
        InterfaceMessage::SetHighColor(rgb) => {
            state.high_color = rgb;
            apply_border_color(rgb);
        }
        InterfaceMessage::SetDisabledColor(rgb) => {
            state.disabled_color = rgb;
            apply_disabled_color(rgb);
        }
        InterfaceMessage::SetSeparatorColor(rgb) => {
            state.separator_color = rgb;
            apply_separator_color(rgb);
        }
        InterfaceMessage::SetVisualGuidesColor(rgb) => {
            state.visual_guides_color = rgb;
            apply_visual_guides_color(rgb);
        }
        InterfaceMessage::SetSliderTrackColor(rgb) => {
            state.slider_track_color = rgb;
            apply_slider_track_color(rgb);
        }
        InterfaceMessage::SetColorBordersColor(rgb) => {
            state.color_borders_color = rgb;
            apply_color_borders_color(rgb);
        }
        InterfaceMessage::SetNormalColor(rgb) => {
            state.normal_color = rgb;
            apply_normal_color(rgb);
        }
        InterfaceMessage::SetPaginatorSidebarColor(rgb) => {
            state.paginator_sidebar_color = rgb;
            apply_paginator_sidebar_color(rgb);
        }
        InterfaceMessage::SetPrimaryHighlightColor(rgb) => {
            state.primary_highlight_color = rgb;
            apply_primary_highlight_color(rgb);
        }
        InterfaceMessage::SetPaginatorTabLabelColor(rgb) => {
            state.paginator_tab_label_color = rgb;
            apply_paginator_tab_label_color(rgb);
        }
        InterfaceMessage::SetToggleEnabledColor(rgb) => {
            state.toggle_enabled_color = rgb;
            apply_toggle_enabled_color(rgb);
        }
        InterfaceMessage::SetToggleDisabledColor(rgb) => {
            state.toggle_disabled_color = rgb;
            apply_toggle_disabled_color(rgb);
        }
        InterfaceMessage::SetTabMarginX(margin) => {
            state.paginator_tab_margin_x = margin;
            apply_paginator_tab_margin_x(margin);
        }
        InterfaceMessage::SetTabMarginY(margin) => {
            state.paginator_tab_margin_y = margin;
            apply_paginator_tab_margin_y(margin);
        }
        InterfaceMessage::SetTabPaddingX(padding) => {
            state.paginator_tab_padding_x = padding;
            apply_paginator_tab_padding_x(padding);
        }
        InterfaceMessage::SetTabPaddingY(padding) => {
            state.paginator_tab_padding_y = padding;
            apply_paginator_tab_padding_y(padding);
        }
        InterfaceMessage::PickLowColor | InterfaceMessage::PickHighColor | InterfaceMessage::PickDisabledColor | InterfaceMessage::PickSeparatorColor | InterfaceMessage::PickVisualGuides | InterfaceMessage::PickSliderTrackColor | InterfaceMessage::PickPageLowColor | InterfaceMessage::PickColorBordersColor | InterfaceMessage::PickNormalColor | InterfaceMessage::PickPaginatorSidebarColor | InterfaceMessage::PickPrimaryHighlightColor | InterfaceMessage::PickPaginatorTabLabelColor | InterfaceMessage::PickToggleEnabledColor | InterfaceMessage::PickToggleDisabledColor => {}
        InterfaceMessage::Refreshed(new) => {
            let was_mx_hovered = state.tab_margin_spinbox_x.hovered();
            let was_my_hovered = state.tab_margin_spinbox_y.hovered();
            let was_px_hovered = state.tab_padding_spinbox_x.hovered();
            let was_py_hovered = state.tab_padding_spinbox_y.hovered();
            // Preserve typeface fields
            let typeface_loaded = state.typeface_loaded;
            let sans_serif = state.sans_serif.clone();
            let serif = state.serif.clone();
            let monospace = state.monospace.clone();
            let window_borders = state.window_borders.clone();
            let status_interface = state.status_interface.clone();
            let fuzzel = state.fuzzel.clone();
            let terminal = state.terminal.clone();
            let paginator = state.paginator.clone();
            let all_fonts = state.all_fonts.clone();
            let mono_fonts = state.mono_fonts.clone();
            let sans_box = state.sans_box.clone();
            let serif_box = state.serif_box.clone();
            let mono_box = state.mono_box.clone();
            let borders_box = state.borders_box.clone();
            let status_box = state.status_box.clone();
            let fuzzel_box = state.fuzzel_box.clone();
            let terminal_box = state.terminal_box.clone();
            let paginator_box = state.paginator_box.clone();
            let search_box = state.search_box.clone();
            let selected_font = state.selected_font.clone();
            let list_box = state.list_box.clone();
            let borders_menu = state.borders_menu.clone();
            let status_menu = state.status_menu.clone();
            let fuzzel_menu = state.fuzzel_menu.clone();
            let terminal_menu = state.terminal_menu.clone();
            let paginator_menu = state.paginator_menu.clone();
            let borders_size_box = state.borders_size_box.clone();
            let status_size_box = state.status_size_box.clone();
            let fuzzel_size_box = state.fuzzel_size_box.clone();
            let terminal_size_box = state.terminal_size_box.clone();
            let paginator_size_box = state.paginator_size_box.clone();
            let font_buttons = state.font_buttons.clone();
            let copy_buttons = state.copy_buttons.clone();

            *state = new;

            state.tab_margin_spinbox_x.set_hovered(was_mx_hovered);
            state.tab_margin_spinbox_y.set_hovered(was_my_hovered);
            state.tab_padding_spinbox_x.set_hovered(was_px_hovered);
            state.tab_padding_spinbox_y.set_hovered(was_py_hovered);

            state.typeface_loaded = typeface_loaded;
            state.sans_serif = sans_serif;
            state.serif = serif;
            state.monospace = monospace;
            state.window_borders = window_borders;
            state.status_interface = status_interface;
            state.fuzzel = fuzzel;
            state.terminal = terminal;
            state.paginator = paginator;
            state.all_fonts = all_fonts;
            state.mono_fonts = mono_fonts;
            state.sans_box = sans_box;
            state.serif_box = serif_box;
            state.mono_box = mono_box;
            state.borders_box = borders_box;
            state.status_box = status_box;
            state.fuzzel_box = fuzzel_box;
            state.terminal_box = terminal_box;
            state.paginator_box = paginator_box;
            state.search_box = search_box;
            state.selected_font = selected_font;
            state.list_box = list_box;
            state.borders_menu = borders_menu;
            state.status_menu = status_menu;
            state.fuzzel_menu = fuzzel_menu;
            state.terminal_menu = terminal_menu;
            state.paginator_menu = paginator_menu;
            state.borders_size_box = borders_size_box;
            state.status_size_box = status_size_box;
            state.fuzzel_size_box = fuzzel_size_box;
            state.terminal_size_box = terminal_size_box;
            state.paginator_size_box = paginator_size_box;
            state.font_buttons = font_buttons;
            state.copy_buttons = copy_buttons;
        }
        InterfaceMessage::TypefaceRefreshed(new) => {
            state.typeface_loaded = new.typeface_loaded;
            state.all_fonts = new.all_fonts;
            state.mono_fonts = new.mono_fonts;
            if state.selected_font.is_none() {
                state.selected_font = new.selected_font.clone();
            }
            if !state.sans_box.editing {
                state.sans_serif = new.sans_serif.clone();
                state.sans_box = new.sans_box;
            }
            if !state.serif_box.editing {
                state.serif = new.serif.clone();
                state.serif_box = new.serif_box;
            }
            if !state.mono_box.editing {
                state.monospace = new.monospace.clone();
                state.mono_box = new.mono_box;
            }
            if !state.borders_box.editing {
                state.window_borders = new.window_borders.clone();
                state.borders_box = new.borders_box;
                state.borders_menu = new.borders_menu;
            }
            if !state.status_box.editing {
                state.status_interface = new.status_interface.clone();
                state.status_box = new.status_box;
                state.status_menu = new.status_menu;
            }
            if !state.fuzzel_box.editing {
                state.fuzzel = new.fuzzel.clone();
                state.fuzzel_box = new.fuzzel_box;
                state.fuzzel_menu = new.fuzzel_menu;
            }
            if !state.terminal_box.editing {
                state.terminal = new.terminal.clone();
                state.terminal_box = new.terminal_box;
                state.terminal_menu = new.terminal_menu;
            }
            if !state.paginator_box.editing {
                state.paginator = new.paginator.clone();
                state.paginator_box = new.paginator_box;
                state.paginator_menu = new.paginator_menu;
            }
            if !state.search_box.editing {
                state.search_box = new.search_box;
            }
            state.borders_size_box = new.borders_size_box;
            state.status_size_box = new.status_size_box;
            state.fuzzel_size_box = new.fuzzel_size_box;
            state.terminal_size_box = new.terminal_size_box;
            state.paginator_size_box = new.paginator_size_box;
            state.font_buttons = new.font_buttons;
            state.copy_buttons = new.copy_buttons;
            let old_scroll = state.list_box.scroll_y();
            state.list_box = new.list_box;
            state.list_box.set_scroll_y(old_scroll);
        }
        InterfaceMessage::SetSans(sans) => {
            state.sans_serif = sans.clone();
            state.sans_box.text = sans;
            if state.borders_menu.selected == 0 {
                state.window_borders = state.sans_serif.clone();
                state.borders_box.text = state.sans_serif.clone();
            }
            if state.status_menu.selected == 0 {
                state.status_interface = state.sans_serif.clone();
                state.status_box.text = state.sans_serif.clone();
            }
            if state.fuzzel_menu.selected == 0 {
                state.fuzzel = state.sans_serif.clone();
                state.fuzzel_box.text = state.sans_serif.clone();
            }
            if state.terminal_menu.selected == 0 {
                state.terminal = state.sans_serif.clone();
                state.terminal_box.text = state.sans_serif.clone();
            }
            if state.paginator_menu.selected == 0 {
                state.paginator = state.sans_serif.clone();
                state.paginator_box.text = state.sans_serif.clone();
            }
            save_preferred_fonts(
                &state.sans_serif,
                &state.serif,
                &state.monospace,
                &state.window_borders,
                &state.status_interface,
                &state.fuzzel,
                &state.terminal,
                &state.paginator,
            );
        }
        InterfaceMessage::SetSerif(serif) => {
            state.serif = serif.clone();
            state.serif_box.text = serif;
            if state.borders_menu.selected == 1 {
                state.window_borders = state.serif.clone();
                state.borders_box.text = state.serif.clone();
            }
            if state.status_menu.selected == 1 {
                state.status_interface = state.serif.clone();
                state.status_box.text = state.serif.clone();
            }
            if state.fuzzel_menu.selected == 1 {
                state.fuzzel = state.serif.clone();
                state.fuzzel_box.text = state.serif.clone();
            }
            if state.terminal_menu.selected == 1 {
                state.terminal = state.serif.clone();
                state.terminal_box.text = state.serif.clone();
            }
            if state.paginator_menu.selected == 1 {
                state.paginator = state.serif.clone();
                state.paginator_box.text = state.serif.clone();
            }
            save_preferred_fonts(
                &state.sans_serif,
                &state.serif,
                &state.monospace,
                &state.window_borders,
                &state.status_interface,
                &state.fuzzel,
                &state.terminal,
                &state.paginator,
            );
        }
        InterfaceMessage::SetMono(mono) => {
            state.monospace = mono.clone();
            state.mono_box.text = mono;
            if state.borders_menu.selected == 2 {
                state.window_borders = state.monospace.clone();
                state.borders_box.text = state.monospace.clone();
            }
            if state.status_menu.selected == 2 {
                state.status_interface = state.monospace.clone();
                state.status_box.text = state.monospace.clone();
            }
            if state.fuzzel_menu.selected == 2 {
                state.fuzzel = state.monospace.clone();
                state.fuzzel_box.text = state.monospace.clone();
            }
            if state.terminal_menu.selected == 2 {
                state.terminal = state.monospace.clone();
                state.terminal_box.text = state.monospace.clone();
            }
            if state.paginator_menu.selected == 2 {
                state.paginator = state.monospace.clone();
                state.paginator_box.text = state.monospace.clone();
            }
            save_preferred_fonts(
                &state.sans_serif,
                &state.serif,
                &state.monospace,
                &state.window_borders,
                &state.status_interface,
                &state.fuzzel,
                &state.terminal,
                &state.paginator,
            );
        }
        InterfaceMessage::SetBorders(borders) => {
            state.window_borders = borders.clone();
            state.borders_box.text = borders;
            save_preferred_fonts(
                &state.sans_serif,
                &state.serif,
                &state.monospace,
                &state.window_borders,
                &state.status_interface,
                &state.fuzzel,
                &state.terminal,
                &state.paginator,
            );
        }
        InterfaceMessage::SetStatus(status) => {
            state.status_interface = status.clone();
            state.status_box.text = status;
            save_preferred_fonts(
                &state.sans_serif,
                &state.serif,
                &state.monospace,
                &state.window_borders,
                &state.status_interface,
                &state.fuzzel,
                &state.terminal,
                &state.paginator,
            );
        }
        InterfaceMessage::SetFuzzel(fuzzel) => {
            state.fuzzel = fuzzel.clone();
            state.fuzzel_box.text = fuzzel;
            save_preferred_fonts(
                &state.sans_serif,
                &state.serif,
                &state.monospace,
                &state.window_borders,
                &state.status_interface,
                &state.fuzzel,
                &state.terminal,
                &state.paginator,
            );
        }
        InterfaceMessage::SetTerminal(term) => {
            state.terminal = term.clone();
            state.terminal_box.text = term;
            save_preferred_fonts(
                &state.sans_serif,
                &state.serif,
                &state.monospace,
                &state.window_borders,
                &state.status_interface,
                &state.fuzzel,
                &state.terminal,
                &state.paginator,
            );
        }
        InterfaceMessage::SetPaginator(paginator) => {
            state.paginator = paginator.clone();
            state.paginator_box.text = paginator;
            save_preferred_fonts(
                &state.sans_serif,
                &state.serif,
                &state.monospace,
                &state.window_borders,
                &state.status_interface,
                &state.fuzzel,
                &state.terminal,
                &state.paginator,
            );
        }
        InterfaceMessage::SetSearch(search) => {
            state.search_box.text = search;
        }
        InterfaceMessage::SelectFont(font) => {
            state.selected_font = Some(font);
        }
        InterfaceMessage::CopyFontName(font) => {
            use std::io::Write;
            std::thread::spawn({
                let text = font.clone();
                move || {
                    let mut copied = false;
                    let child = std::process::Command::new("wl-copy")
                        .stdin(std::process::Stdio::piped())
                        .stderr(std::process::Stdio::piped())
                        .spawn();
                    match child {
                        Ok(mut child) => {
                            if let Some(mut stdin) = child.stdin.take() {
                                let _ = stdin.write_all(text.as_bytes());
                            }
                            match child.wait_with_output() {
                                Ok(output) => {
                                    if output.status.success() {
                                        copied = true;
                                    } else {
                                        let err_msg = String::from_utf8_lossy(&output.stderr);
                                        eprintln!("wl-copy exited with error status: {:?}, stderr: {}", output.status, err_msg);
                                    }
                                }
                                Err(e) => {
                                    eprintln!("wl-copy wait failed: {:?}", e);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("wl-copy spawn failed: {:?}", e);
                        }
                    }
                    if !copied {
                        if let Ok(mut child) = std::process::Command::new("xclip")
                            .arg("-selection")
                            .arg("clipboard")
                            .stdin(std::process::Stdio::piped())
                            .spawn()
                        {
                            if let Some(mut stdin) = child.stdin.take() {
                                let _ = stdin.write_all(text.as_bytes());
                            }
                            let _ = child.wait();
                        }
                    }
                }
            });
        }
        InterfaceMessage::SetBordersMenu(idx) => {
            state.borders_menu.selected = idx;
            state.borders_box.disabled = idx != 3;
            if idx == 0 {
                state.window_borders = state.sans_serif.clone();
                state.borders_box.text = state.sans_serif.clone();
            } else if idx == 1 {
                state.window_borders = state.serif.clone();
                state.borders_box.text = state.serif.clone();
            } else if idx == 2 {
                state.window_borders = state.monospace.clone();
                state.borders_box.text = state.monospace.clone();
            }
            save_preferred_fonts(
                &state.sans_serif,
                &state.serif,
                &state.monospace,
                &state.window_borders,
                &state.status_interface,
                &state.fuzzel,
                &state.terminal,
                &state.paginator,
            );
        }
        InterfaceMessage::SetStatusMenu(idx) => {
            state.status_menu.selected = idx;
            state.status_box.disabled = idx != 3;
            if idx == 0 {
                state.status_interface = state.sans_serif.clone();
                state.status_box.text = state.sans_serif.clone();
            } else if idx == 1 {
                state.status_interface = state.serif.clone();
                state.status_box.text = state.serif.clone();
            } else if idx == 2 {
                state.status_interface = state.monospace.clone();
                state.status_box.text = state.monospace.clone();
            }
            save_preferred_fonts(
                &state.sans_serif,
                &state.serif,
                &state.monospace,
                &state.window_borders,
                &state.status_interface,
                &state.fuzzel,
                &state.terminal,
                &state.paginator,
            );
        }
        InterfaceMessage::SetFuzzelMenu(idx) => {
            state.fuzzel_menu.selected = idx;
            state.fuzzel_box.disabled = idx != 3;
            if idx == 0 {
                state.fuzzel = state.sans_serif.clone();
                state.fuzzel_box.text = state.sans_serif.clone();
            } else if idx == 1 {
                state.fuzzel = state.serif.clone();
                state.fuzzel_box.text = state.serif.clone();
            } else if idx == 2 {
                state.fuzzel = state.monospace.clone();
                state.fuzzel_box.text = state.monospace.clone();
            }
            save_preferred_fonts(
                &state.sans_serif,
                &state.serif,
                &state.monospace,
                &state.window_borders,
                &state.status_interface,
                &state.fuzzel,
                &state.terminal,
                &state.paginator,
            );
        }
        InterfaceMessage::SetTerminalMenu(idx) => {
            state.terminal_menu.selected = idx;
            state.terminal_box.disabled = idx != 3;
            if idx == 0 {
                state.terminal = state.sans_serif.clone();
                state.terminal_box.text = state.sans_serif.clone();
            } else if idx == 1 {
                state.terminal = state.serif.clone();
                state.terminal_box.text = state.serif.clone();
            } else if idx == 2 {
                state.terminal = state.monospace.clone();
                state.terminal_box.text = state.monospace.clone();
            }
            save_preferred_fonts(
                &state.sans_serif,
                &state.serif,
                &state.monospace,
                &state.window_borders,
                &state.status_interface,
                &state.fuzzel,
                &state.terminal,
                &state.paginator,
            );
        }
        InterfaceMessage::SetPaginatorMenu(idx) => {
            state.paginator_menu.selected = idx;
            state.paginator_box.disabled = idx != 3;
            if idx == 0 {
                state.paginator = state.sans_serif.clone();
                state.paginator_box.text = state.sans_serif.clone();
            } else if idx == 1 {
                state.paginator = state.serif.clone();
                state.paginator_box.text = state.serif.clone();
            } else if idx == 2 {
                state.paginator = state.monospace.clone();
                state.paginator_box.text = state.monospace.clone();
            }
            save_preferred_fonts(
                &state.sans_serif,
                &state.serif,
                &state.monospace,
                &state.window_borders,
                &state.status_interface,
                &state.fuzzel,
                &state.terminal,
                &state.paginator,
            );
        }
        InterfaceMessage::SetBordersSize(val) => {
            state.borders_size_box.value = val;
            write_config_value("border_font_size", &val.to_string());
            send_ipc_command(&format!("layout border_font_size {}", val));
        }
        InterfaceMessage::SetStatusSize(val) => {
            state.status_size_box.value = val;
            write_status_size(val as u16);
        }
        InterfaceMessage::SetFuzzelSize(val) => {
            state.fuzzel_size_box.value = val;
            write_fuzzel_size(val as u16);
        }
        InterfaceMessage::SetTerminalSize(val) => {
            state.terminal_size_box.value = val;
            write_terminal_size(val as u16);
        }
        InterfaceMessage::SetPaginatorSize(val) => {
            state.paginator_size_box.value = val;
            write_paginator_size(val as u16);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex() {
        assert_eq!(parse_hex("#ffffff"), [255, 255, 255]);
        assert_eq!(parse_hex("000000"), [0, 0, 0]);
        assert_eq!(parse_hex("#123456"), [18, 52, 86]);
        assert_eq!(parse_hex("invalid"), [0x0a, 0x1a, 0x0e]);
    }

    #[test]
    fn test_parse_color_from_key() {
        let content = "\n[layout]\nlow_color = \"#112233\"\nhigh_color = \"#445566\"\ndisabled_color = \"#778899\"\nstatus_separator_color = \"#aabbcc\"\nvisual_guides_color = \"#ddeeff\"\nslider_track_color = \"#123456\"\npage_low_color = \"#474751\"\ncolor_borders_color = \"#abcdef\"\nstatus_normal_color = \"#ccccd8\"\npaginator_sidebar_color = \"#5a5a65\"\nprimary_highlight_color = \"#ffffff\"\npaginator_tab_label_color = \"#e6e6f2\"\ntoggle_enabled_color = \"#68d8a5\"\ntoggle_disabled_color = \"#878794\"\n";
        assert_eq!(parse_color_from_key(content, "low_color", [0, 0, 0]), [17, 34, 51]);
        assert_eq!(parse_color_from_key(content, "high_color", [0, 0, 0]), [68, 85, 102]);
        assert_eq!(parse_color_from_key(content, "disabled_color", [0, 0, 0]), [119, 136, 153]);
        assert_eq!(parse_color_from_key(content, "status_separator_color", [0, 0, 0]), [170, 187, 204]);
        assert_eq!(parse_color_from_key(content, "visual_guides_color", [0, 0, 0]), [221, 238, 255]);
        assert_eq!(parse_color_from_key(content, "slider_track_color", [0, 0, 0]), [18, 52, 86]);
        assert_eq!(parse_color_from_key(content, "page_low_color", [0, 0, 0]), [71, 71, 81]);
        assert_eq!(parse_color_from_key(content, "color_borders_color", [0, 0, 0]), [171, 205, 239]);
        assert_eq!(parse_color_from_key(content, "status_normal_color", [0, 0, 0]), [204, 204, 216]);
        assert_eq!(parse_color_from_key(content, "paginator_sidebar_color", [0, 0, 0]), [90, 90, 101]);
        assert_eq!(parse_color_from_key(content, "primary_highlight_color", [0, 0, 0]), [255, 255, 255]);
        assert_eq!(parse_color_from_key(content, "paginator_tab_label_color", [0, 0, 0]), [230, 230, 242]);
        assert_eq!(parse_color_from_key(content, "toggle_enabled_color", [0, 0, 0]), [104, 216, 165]);
        assert_eq!(parse_color_from_key(content, "toggle_disabled_color", [0, 0, 0]), [135, 135, 148]);
        assert_eq!(parse_color_from_key(content, "non_existent", [1, 2, 3]), [1, 2, 3]);
    }

    #[test]
    fn test_write_config_value_path() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_config.toml");
        let path_str = path.to_str().unwrap();

        // 1. Write initial file content with [layout] and other keys
        let initial_content = "[layout]\ngap = 18\nborder_color = \"#374673\"\n\n[output]\nscale = 2\n";
        fs::write(path_str, initial_content).unwrap();

        // 2. Write disabled_color which does not exist yet (key not found case)
        assert!(write_config_value_path(path_str, "disabled_color", "\"#555555\""));
        let updated = fs::read_to_string(path_str).unwrap();
        assert!(updated.contains("disabled_color = \"#555555\""));
        // Check it was inserted before [output]
        assert!(updated.find("disabled_color = \"#555555\"").unwrap() < updated.find("[output]").unwrap());

        // 3. Update disabled_color (key found case)
        assert!(write_config_value_path(path_str, "disabled_color", "\"#666666\""));
        let updated2 = fs::read_to_string(path_str).unwrap();
        assert!(updated2.contains("disabled_color = \"#666666\""));
        assert!(!updated2.contains("disabled_color = \"#555555\""));

        // 4. Write visual_guides_color which does not exist yet
        assert!(write_config_value_path(path_str, "visual_guides_color", "\"#ff8c00\""));
        let updated3 = fs::read_to_string(path_str).unwrap();
        assert!(updated3.contains("visual_guides_color = \"#ff8c00\""));

        // 5. Write slider_track_color which does not exist yet
        assert!(write_config_value_path(path_str, "slider_track_color", "\"#123456\""));
        let updated4 = fs::read_to_string(path_str).unwrap();
        assert!(updated4.contains("slider_track_color = \"#123456\""));

        // Clean up
        let _ = fs::remove_file(path_str);
    }

    #[test]
    fn test_parse_font_for_alias() {
        let content = r#"<?xml version="1.0"?>
<!DOCTYPE fontconfig SYSTEM "fonts.dtd">
<fontconfig>
    <dir>~/Dropbox/Fonts</dir>
    <match target="pattern">
        <test qual="any" name="family"><string>sans-serif</string></test>
        <edit name="family" mode="assign" binding="same">
            <string>Adwaita Sans</string>
        </edit>
    </match>
    <match target="pattern">
        <test qual="any" name="family"><string>monospace</string></test>
        <edit name="family" mode="assign" binding="same">
            <string>Berkeley Mono</string>
        </edit>
    </match>
</fontconfig>
"#;

        assert_eq!(parse_font_for_alias(content, "sans-serif"), Some("Adwaita Sans".to_string()));
        assert_eq!(parse_font_for_alias(content, "monospace"), Some("Berkeley Mono".to_string()));
        assert_eq!(parse_font_for_alias(content, "serif"), None);
    }
}
