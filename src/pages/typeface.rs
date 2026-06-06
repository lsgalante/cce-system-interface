use std::fs;
use crate::app::PageContent;
use clear_ui::layout::{Section, PageLayoutBuilder, LayoutStrategy};
use clear_ui::widget::{Widget, TextLabel, ScrollBox, ScrollingList, Dropdown, TextBox, Spinbox, Button};
use clear_ui::widget::{ElementState, KeyEvent, MouseButton, Key, NamedKey};

const FONTS_CONF_PATH: &str = "/home/lsgalante/.config/fontconfig/fonts.conf";

// ── TypefaceState and TypefaceMessage ──

#[derive(Debug, Clone)]
pub struct TypefaceState {
    pub loaded: bool,
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
    pub font_buttons: Vec<Button>,
    pub copy_buttons: Vec<Button>,
}

impl Default for TypefaceState {
    fn default() -> Self {
        Self {
            loaded: false,
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
pub enum TypefaceMessage {
    Refreshed(TypefaceState),
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

    // Paginator
    new_content.push_str("    <match target=\"pattern\">\n");
    new_content.push_str("        <test qual=\"any\" name=\"family\"><string>paginator-tab-labels</string></test>\n");
    new_content.push_str("        <edit name=\"family\" mode=\"assign\" binding=\"same\">\n");
    new_content.push_str(&format!("            <string>{}</string>\n", paginator));
    new_content.push_str("        </edit>\n");
    new_content.push_str("    </match>\n");
    
    new_content.push_str("</fontconfig>\n");
    
    if let Some(parent) = std::path::Path::new(FONTS_CONF_PATH).parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(FONTS_CONF_PATH, new_content);
    
    let _ = std::process::Command::new("fc-cache")
        .arg("-f")
        .spawn();
}

pub fn parse_u16_from(content: &str, key: &str, default: u16) -> u16 {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(key) {
            let rest = rest.trim_start_matches(|c: char| c == ' ' || c == '=' || c == '"');
            return rest.trim_end_matches('"').trim().parse::<u16>().unwrap_or(default);
        }
    }
    default
}

pub fn write_config_value(key: &str, value: &str) -> bool {
    write_config_value_path("/home/lsgalante/.config/ccec/config.toml", key, value)
}

pub fn write_config_value_path(path: &str, key: &str, value: &str) -> bool {
    let content = fs::read_to_string(path).unwrap_or_default();
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
        fs::write(path, result).is_ok()
    } else { fs::write(path, updated).is_ok() }
}

fn get_socket_path() -> String {
    match std::env::var("WAYLAND_DISPLAY") {
        Ok(display) => format!("/tmp/ccec-{}.sock", display),
        Err(_) => "/tmp/ccec.sock".to_string(),
    }
}

fn send_ipc_command(cmd: &str) {
    if let Ok(mut stream) = std::os::unix::net::UnixStream::connect(get_socket_path()) {
        use std::io::Write;
        let _ = stream.write_all(format!("{}\n", cmd).as_bytes());
    }
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

pub async fn fetch_typeface_state() -> TypefaceState {
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

    TypefaceState {
        loaded: true,
        sans_serif: sans.clone(),
        serif: serif.clone(),
        monospace: mono.clone(),
        window_borders: borders,
        status_interface: status,
        fuzzel: fuzzel_font,
        terminal: term,
        paginator: paginator_font,
        all_fonts,
        mono_fonts,
        sans_box: TextBox::new(sans).with_label("Sans-Serif"),
        serif_box: TextBox::new(serif).with_label("Serif"),
        mono_box: TextBox::new(mono).with_label("Monospace"),
        borders_box,
        status_box,
        fuzzel_box,
        terminal_box,
        paginator_box,
        search_box: TextBox::new(String::new()).with_label("Filter Fonts"),
        selected_font,
        list_box: ScrollingList::new(24.0, 4.0),
        borders_menu: Dropdown::new(menu_options.clone(), borders_idx),
        status_menu: Dropdown::new(menu_options.clone(), status_idx),
        fuzzel_menu: Dropdown::new(menu_options.clone(), fuzzel_idx),
        terminal_menu: Dropdown::new(menu_options.clone(), terminal_idx),
        paginator_menu: Dropdown::new(menu_options.clone(), paginator_idx),
        borders_size_box: Spinbox::new(borders_size as i32, 6, 72, 1),
        status_size_box: Spinbox::new(status_size as i32, 6, 72, 1),
        fuzzel_size_box: Spinbox::new(fuzzel_size as i32, 6, 72, 1),
        terminal_size_box: Spinbox::new(terminal_size as i32, 6, 72, 1),
        paginator_size_box: Spinbox::new(paginator_size as i32, 6, 72, 1),
        font_buttons: Vec::new(),
        copy_buttons: Vec::new(),
    }
}

const TEXT_DIM: [f32; 4] = [0.53, 0.53, 0.60, 1.0];

pub fn view(state: &mut TypefaceState, cx: f32, cy: f32, cw: f32, ch: f32, sec_focused: &[bool], layout: &mut dyn LayoutStrategy) -> PageContent {
    let mut final_pc = PageContent::new();
    let sec_w = 320.0f32;
    let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(3);

    let widget_h = 26.0;

    // ── System Typefaces Section ──
    builder.add_section(&mut final_pc, |pc, rx, ry| {
        let mut sec = Section::new(pc, rx, ry, sec_w, "System Typefaces");
        sec.spacing(8.0);

        if !state.loaded {
            sec.text(pc, "Loading typefaces...", 12.0, 0.0, 12.0, TEXT_DIM);
            sec.spacing(18.0);
        } else {
            let inner_w = sec_w - 24.0;
            // Sans-Serif
            sec.widget(pc, &mut state.sans_box, 12.0, inner_w, widget_h);
            sec.spacing(12.0);

            // Serif
            sec.widget(pc, &mut state.serif_box, 12.0, inner_w, widget_h);
            sec.spacing(12.0);

            // Monospace
            sec.widget(pc, &mut state.mono_box, 12.0, inner_w, widget_h);
            sec.spacing(8.0);
        }
        let sys_focused = sec_focused.get(0).copied().unwrap_or(false);
        sec.finish_focused(pc, sys_focused)
    });

    // ── Program Typefaces Section ──
    builder.add_section(&mut final_pc, |pc, rx, ry| {
        let mut sec = Section::new(pc, rx, ry, sec_w, "Program Typefaces");
        sec.spacing(8.0);

        if !state.loaded {
            sec.text(pc, "Loading typefaces...", 12.0, 0.0, 12.0, TEXT_DIM);
            sec.spacing(18.0);
        } else {
            let inner_w = sec_w - 24.0;

            // Window Borders
            let start_y = sec.ay();
            sec.row(2, 10.0, widget_h, |idx, x, w| {
                if idx == 0 {
                    state.borders_menu.set_row_rect(x, w);
                    clear_ui::layout::render_widget(pc, &mut state.borders_menu, x, start_y, w, widget_h);
                } else {
                    state.borders_size_box.set_row_rect(x, w);
                    clear_ui::layout::render_widget(pc, &mut state.borders_size_box, x, start_y, w, widget_h);
                }
            });
            sec.widget(pc, &mut state.borders_box, 12.0, inner_w, widget_h);
            sec.spacing(16.0);

            // Status Interface
            let start_y = sec.ay();
            sec.row(2, 10.0, widget_h, |idx, x, w| {
                if idx == 0 {
                    state.status_menu.set_row_rect(x, w);
                    clear_ui::layout::render_widget(pc, &mut state.status_menu, x, start_y, w, widget_h);
                } else {
                    state.status_size_box.set_row_rect(x, w);
                    clear_ui::layout::render_widget(pc, &mut state.status_size_box, x, start_y, w, widget_h);
                }
            });
            sec.widget(pc, &mut state.status_box, 12.0, inner_w, widget_h);
            sec.spacing(16.0);

            // Fuzzel
            let start_y = sec.ay();
            sec.row(2, 10.0, widget_h, |idx, x, w| {
                if idx == 0 {
                    state.fuzzel_menu.set_row_rect(x, w);
                    clear_ui::layout::render_widget(pc, &mut state.fuzzel_menu, x, start_y, w, widget_h);
                } else {
                    state.fuzzel_size_box.set_row_rect(x, w);
                    clear_ui::layout::render_widget(pc, &mut state.fuzzel_size_box, x, start_y, w, widget_h);
                }
            });
            sec.widget(pc, &mut state.fuzzel_box, 12.0, inner_w, widget_h);
            sec.spacing(16.0);

            // Terminal
            let start_y = sec.ay();
            sec.row(2, 10.0, widget_h, |idx, x, w| {
                if idx == 0 {
                    state.terminal_menu.set_row_rect(x, w);
                    clear_ui::layout::render_widget(pc, &mut state.terminal_menu, x, start_y, w, widget_h);
                } else {
                    state.terminal_size_box.set_row_rect(x, w);
                    clear_ui::layout::render_widget(pc, &mut state.terminal_size_box, x, start_y, w, widget_h);
                }
            });
            sec.widget(pc, &mut state.terminal_box, 12.0, inner_w, widget_h);
            sec.spacing(16.0);

            // Paginator Tab Labels
            let start_y = sec.ay();
            sec.row(2, 10.0, widget_h, |idx, x, w| {
                if idx == 0 {
                    state.paginator_menu.set_row_rect(x, w);
                    clear_ui::layout::render_widget(pc, &mut state.paginator_menu, x, start_y, w, widget_h);
                } else {
                    state.paginator_size_box.set_row_rect(x, w);
                    clear_ui::layout::render_widget(pc, &mut state.paginator_size_box, x, start_y, w, widget_h);
                }
            });
            sec.widget(pc, &mut state.paginator_box, 12.0, inner_w, widget_h);
            sec.spacing(8.0);
        }
        let prog_focused = sec_focused.get(1).copied().unwrap_or(false);
        sec.finish_focused(pc, prog_focused)
    });

    // ── Typefaces Section (List & Preview) ──
    builder.add_section(&mut final_pc, |pc, rx, ry| {
        let mut sec = Section::new(pc, rx, ry, sec_w, "Typefaces");
        sec.spacing(12.0);

        if !state.loaded {
            sec.text(pc, "Loading installed fonts...", 12.0, 0.0, 12.0, TEXT_DIM);
            sec.spacing(18.0);
        } else {
            let inner_w = sec_w - 24.0;

            // 1. Search Box
            let top_room = state.search_box.top_room();
            state.search_box.set_row_rect(rx + 12.0, inner_w);
            clear_ui::layout::render_widget(
                pc,
                &mut state.search_box,
                rx + 12.0,
                sec.ay() + top_room,
                inner_w,
                widget_h,
            );
            sec.spacing(widget_h + top_room + 12.0);

            // 2. Scrolling List Box
            let list_box_y = sec.ay();
            let list_box_h = 200.0;
            
            clear_ui::layout::render_widget(pc, &mut state.list_box, rx + 12.0, list_box_y, inner_w, list_box_h);

            let query = state.search_box.text.to_lowercase();
            let matching_fonts: Vec<&String> = state.all_fonts.iter()
                .filter(|font| font.to_lowercase().contains(&query))
                .collect();

            if state.font_buttons.len() != matching_fonts.len() {
                state.font_buttons.clear();
                state.copy_buttons.clear();
                for _ in 0..matching_fonts.len() {
                    state.font_buttons.push(Button::new_list_row(0.0, 0.0, 0.0, 0.0));
                    state.copy_buttons.push(Button::new_copy_icon(0.0, 0.0, 0.0, 0.0));
                }
            }

            let btn_h = 24.0;
            let list_inner_x = rx + 16.0;
            let list_inner_w = inner_w - 16.0;

            state.list_box.update_bounds(matching_fonts.len(), list_box_y, list_box_h);

            for (idx, font_name) in matching_fonts.iter().enumerate() {
                if let Some(draw_y) = state.list_box.get_item_draw_y(idx, 0.0) {
                    let is_selected = state.selected_font.as_ref() == Some(*font_name);
                    
                    let font_btn = &mut state.font_buttons[idx];
                    font_btn.set_text(font_name);
                    font_btn.selected = is_selected;
                    clear_ui::layout::render_widget(pc, font_btn, list_inner_x, draw_y, list_inner_w - 44.0, btn_h);

                    let copy_btn = &mut state.copy_buttons[idx];
                    copy_btn.set_text("📋");
                    copy_btn.selected = is_selected;
                    clear_ui::layout::render_widget(pc, copy_btn, list_inner_x + list_inner_w - 40.0, draw_y, 40.0, btn_h);
                }
            }

            if matching_fonts.is_empty() {
                pc.text("No fonts match query", list_inner_x + 8.0, list_box_y + 16.0, 12.0, TEXT_DIM);
            }

            sec.spacing(list_box_h + 12.0);

            // 3. Info Box
            let info_h = 96.0;
            let info_bg = [0.12, 0.18, 0.28, 0.3];
            let info_border = [0.25, 0.40, 0.60, 0.5];
            let info_y = sec.ay();

            pc.rect(info_bg, rx + 12.0, info_y, inner_w, info_h);
            pc.rect(info_border, rx + 12.0, info_y, inner_w, 1.0);
            pc.rect(info_border, rx + 12.0, info_y + info_h - 1.0, inner_w, 1.0);
            pc.rect(info_border, rx + 12.0, info_y, 1.0, info_h);
            pc.rect(info_border, rx + 12.0 + inner_w - 1.0, info_y, 1.0, info_h);

            let text_padding_x = 16.0;
            let mut text_y = info_y + 12.0;

            pc.text("Font Directories & Installation", rx + 12.0 + text_padding_x, text_y, 12.0, [0.35, 0.65, 0.90, 1.0]);
            text_y += 20.0;

            pc.text("• Active Directory: ~/Dropbox/Fonts", rx + 12.0 + text_padding_x, text_y, 11.0, [0.80, 0.80, 0.85, 1.0]);
            text_y += 16.0;

            pc.text("• Place TTF/OTF files there to install new fonts.", rx + 12.0 + text_padding_x, text_y, 11.0, [0.80, 0.80, 0.85, 1.0]);
            text_y += 16.0;

            pc.text("• Changes will be cached automatically by fontconfig.", rx + 12.0 + text_padding_x, text_y, 11.0, [0.55, 0.55, 0.60, 1.0]);

            sec.spacing(info_h + 12.0);

            // 4. Preview Card
            if let Some(ref font_name) = state.selected_font {
                let card_h = 240.0;
                let card_y = sec.ay();
                pc.rect([0.10, 0.10, 0.14, 0.3], rx + 12.0, card_y, inner_w, card_h);
                pc.rect([0.25, 0.25, 0.35, 0.5], rx + 12.0, card_y, inner_w, 1.0);
                pc.rect([0.25, 0.25, 0.35, 0.5], rx + 12.0, card_y + card_h - 1.0, inner_w, 1.0);
                pc.rect([0.25, 0.25, 0.35, 0.5], rx + 12.0, card_y, 1.0, card_h);
                pc.rect([0.25, 0.25, 0.35, 0.5], rx + 12.0 + inner_w - 1.0, card_y, 1.0, card_h);

                let mut p_text_y = card_y + 16.0;
                pc.text(&format!("Family: {}", font_name), rx + 12.0 + text_padding_x, p_text_y, 15.0, [0.90, 0.90, 0.95, 1.0]);
                p_text_y += 28.0;

                pc.rect([0.22, 0.22, 0.30, 0.8], rx + 12.0 + text_padding_x, p_text_y, inner_w - (text_padding_x * 2.0), 1.0);
                p_text_y += 16.0;

                pc.text_with_font(
                    "abcdefghijklmnopqrstuvwxyz",
                    rx + 12.0 + text_padding_x,
                    p_text_y,
                    13.0,
                    [0.75, 0.75, 0.80, 1.0],
                    font_name,
                );
                p_text_y += 22.0;

                pc.text_with_font(
                    "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
                    rx + 12.0 + text_padding_x,
                    p_text_y,
                    13.0,
                    [0.75, 0.75, 0.80, 1.0],
                    font_name,
                );
                p_text_y += 22.0;

                pc.text_with_font(
                    "0123456789 (!@#$%&*?)",
                    rx + 12.0 + text_padding_x,
                    p_text_y,
                    13.0,
                    [0.75, 0.75, 0.80, 1.0],
                    font_name,
                );
                p_text_y += 26.0;

                pc.text_with_font(
                    "The quick brown fox jumps over the lazy dog.",
                    rx + 12.0 + text_padding_x,
                    p_text_y,
                    16.0,
                    [0.90, 0.90, 0.95, 1.0],
                    font_name,
                );
                p_text_y += 32.0;

                pc.text_with_font(
                    "The five boxing wizards jump quickly.",
                    rx + 12.0 + text_padding_x,
                    p_text_y,
                    20.0,
                    [0.95, 0.95, 1.0, 1.0],
                    font_name,
                );
                sec.spacing(card_h + 8.0);
            } else {
                pc.text("Select a font to preview", rx + 24.0, sec.ay() + 20.0, 13.0, TEXT_DIM);
                sec.spacing(40.0);
            }
        }
        let list_focused = sec_focused.get(2).copied().unwrap_or(false);
        sec.finish_focused(pc, list_focused)
    });

    final_pc
}

pub fn update(state: &mut TypefaceState, msg: TypefaceMessage) {
    match msg {
        TypefaceMessage::Refreshed(new) => {
            state.loaded = new.loaded;
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
        TypefaceMessage::SetSans(sans) => {
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
        TypefaceMessage::SetSerif(serif) => {
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
        TypefaceMessage::SetMono(mono) => {
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
        TypefaceMessage::SetBorders(borders) => {
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
        TypefaceMessage::SetStatus(status) => {
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
        TypefaceMessage::SetFuzzel(fuzzel) => {
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
        TypefaceMessage::SetTerminal(term) => {
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
        TypefaceMessage::SetPaginator(paginator) => {
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
        TypefaceMessage::SetSearch(search) => {
            state.search_box.text = search;
        }
        TypefaceMessage::SelectFont(font) => {
            state.selected_font = Some(font);
        }
        TypefaceMessage::CopyFontName(font) => {
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
        TypefaceMessage::SetBordersMenu(idx) => {
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
        TypefaceMessage::SetStatusMenu(idx) => {
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
        TypefaceMessage::SetFuzzelMenu(idx) => {
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
        TypefaceMessage::SetTerminalMenu(idx) => {
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
        TypefaceMessage::SetPaginatorMenu(idx) => {
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
        TypefaceMessage::SetBordersSize(val) => {
            state.borders_size_box.value = val;
            write_config_value("border_font_size", &val.to_string());
            send_ipc_command(&format!("layout border_font_size {}", val));
        }
        TypefaceMessage::SetStatusSize(val) => {
            state.status_size_box.value = val;
            write_status_size(val as u16);
        }
        TypefaceMessage::SetFuzzelSize(val) => {
            state.fuzzel_size_box.value = val;
            write_fuzzel_size(val as u16);
        }
        TypefaceMessage::SetTerminalSize(val) => {
            state.terminal_size_box.value = val;
            write_terminal_size(val as u16);
        }
        TypefaceMessage::SetPaginatorSize(val) => {
            state.paginator_size_box.value = val;
            write_paginator_size(val as u16);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
