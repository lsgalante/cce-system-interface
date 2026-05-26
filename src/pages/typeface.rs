use std::fs;
use crate::app::PageContent;
use clear_ui::layout::Section;
use clear_ui::widget::{Widget, TextLabel, ScrollBox, ScrollingList, Dropdown};
use clear_ui::widget::{ElementState, KeyEvent, MouseButton, Key, NamedKey};

const FONTS_CONF_PATH: &str = "/home/lsgalante/.config/fontconfig/fonts.conf";

// ── TextBox Widget ──

#[derive(Debug, Clone)]
pub struct TextBox {
    x: f32, y: f32, w: f32, h: f32,
    pub text: String,
    pub editing: bool,
    pub edit_buffer: String,
    hovered: bool,
    just_changed: bool,
    label: Option<String>,
    row_x: f32,
    row_w: f32,
    pub disabled: bool,
    pub parent: Option<*mut (dyn Widget + 'static)>,
    pub children: Vec<*mut (dyn Widget + 'static)>,
}

impl TextBox {
    pub fn new(text: String) -> Self {
        Self {
            x: 0.0, y: 0.0, w: 0.0, h: 0.0,
            text,
            editing: false,
            edit_buffer: String::new(),
            hovered: false,
            just_changed: false,
            label: None,
            row_x: 0.0,
            row_w: 0.0,
            disabled: false,
            parent: None,
            children: Vec::new(),
        }
    }

    pub fn with_label(mut self, label: &str) -> Self {
        self.label = Some(label.to_string());
        self
    }

    pub fn set_label(&mut self, label: &str) {
        self.label = Some(label.to_string());
    }

    pub fn take_change(&mut self) -> bool {
        let changed = self.just_changed;
        self.just_changed = false;
        changed
    }
}

impl Default for TextBox {
    fn default() -> Self {
        Self::new(String::new())
    }
}

impl Widget for TextBox {
    fn rect(&self) -> (f32, f32, f32, f32) { (self.x, self.y, self.w, self.h) }
    fn set_rect(&mut self, x: f32, y: f32, w: f32, h: f32) { self.x = x; self.y = y; self.w = w; self.h = h; }
    fn set_row_rect(&mut self, x: f32, w: f32) { self.row_x = x; self.row_w = w; }
    fn set_hovered(&mut self, v: bool) { self.hovered = v; }
    fn hovered(&self) -> bool { self.hovered }

    fn color(&self) -> [f32; 4] {
        [0.10, 0.10, 0.16, 1.0]
    }

    fn hit_test(&self, px: f32, py: f32) -> bool {
        let (x, y, w, h) = self.rect();
        let hx = if self.row_w > 0.0 { self.row_x } else { x };
        let hw = if self.row_w > 0.0 { self.row_w } else { w };
        let (hy, hh) = if self.label.is_some() {
            (y - 18.0, h + 18.0)
        } else {
            (y, h)
        };
        px >= hx && px <= hx + hw && py >= hy && py <= hy + hh
    }

    fn top_room(&self) -> f32 { if self.label.is_some() { 18.0 } else { 0.0 } }

    fn cursor_moved(&mut self, px: f32, py: f32) -> bool {
        if self.disabled {
            let was = self.hovered;
            self.hovered = false;
            return was;
        }
        let was = self.hovered;
        self.hovered = self.hit_test(px, py);
        was != self.hovered
    }

    fn mouse_input(&mut self, button: MouseButton, state: ElementState, px: f32, py: f32) -> bool {
        if self.disabled { return false; }
        if button != MouseButton::Left { return false; }
        if state != ElementState::Pressed { return false; }
        let (x, y, w, h) = self.rect();
        let (hy, hh) = if self.label.is_some() {
            (y - 18.0, h + 18.0)
        } else {
            (y, h)
        };
        if !(px >= x && px <= x + w && py >= hy && py <= hy + hh) {
            return false;
        }
        self.focus();
        true
    }

    fn focus(&mut self) {
        if self.disabled { return; }
        if !self.editing {
            self.editing = true;
            self.edit_buffer = self.text.clone();
            clear_ui::widget::focus::set_focused(self);
        }
    }

    fn unfocus(&mut self) {
        if self.editing {
            self.editing = false;
            if self.text != self.edit_buffer {
                self.text = self.edit_buffer.clone();
                self.just_changed = true;
            }
        }
    }

    fn keyboard_input(&mut self, event: &KeyEvent) -> bool {
        if self.disabled { return false; }
        if !self.editing { return false; }
        if event.state != ElementState::Pressed { return false; }
        match &event.logical_key {
            Key::Named(NamedKey::Backspace) => {
                self.edit_buffer.pop();
                true
            }
            Key::Named(NamedKey::Enter) => {
                self.text = self.edit_buffer.clone();
                self.editing = false;
                self.just_changed = true;
                true
            }
            Key::Named(NamedKey::Escape) => {
                self.editing = false;
                true
            }
            _ => {
                if let Some(text) = &event.text {
                    if !event.repeat {
                        for ch in text.chars() {
                            if ch.is_alphanumeric() || ch == ' ' || ch == '-' || ch == '_' || ch == '*' {
                                self.edit_buffer.push(ch);
                            }
                        }
                    }
                }
                true
            }
        }
    }

    fn hover_highlight(&self) -> Option<[f32; 4]> {
        None
    }

    fn extra_quads(&self) -> Vec<(f32, f32, f32, f32, [f32; 4])> {
        let mut quads = Vec::new();
        if self.disabled {
            quads.push((self.x, self.y, self.w, self.h, [0.12, 0.12, 0.16, 1.0])); // border
            quads.push((self.x + 1.0, self.y + 1.0, self.w - 2.0, self.h - 2.0, [0.06, 0.06, 0.08, 1.0])); // bg
            return quads;
        }
        if self.hovered {
            let (hy, hh) = if self.label.is_some() {
                (self.y - 18.0, self.h + 18.0)
            } else {
                (self.y, self.h)
            };
            let hx = if self.row_w > 0.0 { self.row_x } else { self.x };
            let hw = if self.row_w > 0.0 { self.row_w } else { self.w };
            quads.push((hx, hy, hw, hh, [1.0, 1.0, 1.0, 0.06]));
        }
        let bg_color = if self.editing {
            [0.12, 0.12, 0.18, 1.0]
        } else {
            [0.08, 0.08, 0.12, 1.0]
        };
        let border_color = if self.editing {
            [0.30, 0.50, 0.32, 1.0]
        } else if self.hovered {
            [0.25, 0.25, 0.35, 1.0]
        } else {
            [0.18, 0.18, 0.24, 1.0]
        };
        quads.push((self.x, self.y, self.w, self.h, border_color));
        quads.push((self.x + 1.0, self.y + 1.0, self.w - 2.0, self.h - 2.0, bg_color));
        quads
    }

    fn text_labels(&self) -> Vec<TextLabel> {
        let mut labels = Vec::new();
        if let Some(ref label) = self.label {
            labels.push(TextLabel {
                text: label.clone(),
                x: self.x + 4.0,
                y: self.y - 14.0,
                font_size: 12.0,
                color: [0x83, 0x83, 0x8a],
            });
        }
        let val_text = if self.editing {
            format!("{}|", self.edit_buffer)
        } else {
            self.text.clone()
        };
        labels.push(TextLabel {
            text: val_text,
            x: self.x + 8.0,
            y: self.y + (self.h - 12.0) / 2.0,
            font_size: 13.0,
            color: if self.disabled { [0x53, 0x53, 0x5a] } else if self.editing { [0xee, 0xee, 0xf5] } else { [0xcc, 0xcc, 0xd4] },
        });
        labels
    }

    fn parent(&self) -> Option<*mut (dyn Widget + 'static)> { self.parent }
    fn set_parent(&mut self, parent: Option<*mut (dyn Widget + 'static)>) { self.parent = parent; }
    fn children(&self) -> Vec<*mut (dyn Widget + 'static)> { self.children.clone() }
    fn add_child(&mut self, child: *mut (dyn Widget + 'static)) { self.children.push(child); }
    fn clear_children(&mut self) { self.children.clear(); }
}

impl Drop for TextBox {
    fn drop(&mut self) {
        clear_ui::widget::focus::clear_if_matches(self);
    }
}

unsafe impl Send for TextBox {}
unsafe impl Sync for TextBox {}

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
    pub all_fonts: Vec<String>,
    pub mono_fonts: Vec<String>,
    pub sans_box: TextBox,
    pub serif_box: TextBox,
    pub mono_box: TextBox,
    pub borders_box: TextBox,
    pub status_box: TextBox,
    pub fuzzel_box: TextBox,
    pub terminal_box: TextBox,
    pub search_box: TextBox,
    pub selected_font: Option<String>,
    pub list_box: ScrollingList,
    pub borders_menu: Dropdown,
    pub status_menu: Dropdown,
    pub fuzzel_menu: Dropdown,
    pub terminal_menu: Dropdown,
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
            all_fonts: Vec::new(),
            mono_fonts: Vec::new(),
            sans_box: TextBox::default(),
            serif_box: TextBox::default(),
            mono_box: TextBox::default(),
            borders_box: TextBox::default(),
            status_box: TextBox::default(),
            fuzzel_box: TextBox::default(),
            terminal_box: TextBox::default(),
            search_box: TextBox::default(),
            selected_font: None,
            list_box: ScrollingList::new(24.0, 4.0),
            borders_menu: Dropdown::default(),
            status_menu: Dropdown::default(),
            fuzzel_menu: Dropdown::default(),
            terminal_menu: Dropdown::default(),
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
    SetSearch(String),
    SelectFont(String),
    CopyFontName(String),
    SetBordersMenu(usize),
    SetStatusMenu(usize),
    SetFuzzelMenu(usize),
    SetTerminalMenu(usize),
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

pub fn read_preferred_fonts() -> (String, String, String, String, String, String, String) {
    let content = fs::read_to_string(FONTS_CONF_PATH).unwrap_or_default();
    
    let sans = parse_font_for_alias(&content, "sans-serif").unwrap_or_else(|| "Noto Sans".to_string());
    let serif = parse_font_for_alias(&content, "serif").unwrap_or_else(|| "Noto Serif".to_string());
    let mono = parse_font_for_alias(&content, "monospace").unwrap_or_else(|| "Noto Sans Mono".to_string());
    let borders = parse_font_for_alias(&content, "window-borders").unwrap_or_else(|| "Noto Sans".to_string());
    let status = parse_font_for_alias(&content, "status-interface").unwrap_or_else(|| "Noto Sans".to_string());
    let fuzzel_font = parse_font_for_alias(&content, "fuzzel").unwrap_or_else(|| "Noto Sans".to_string());
    let term = parse_font_for_alias(&content, "terminal").unwrap_or_else(|| "Noto Sans Mono".to_string());
    
    (sans, serif, mono, borders, status, fuzzel_font, term)
}

pub fn save_preferred_fonts(
    sans: &str,
    serif: &str,
    mono: &str,
    borders: &str,
    status: &str,
    fuzzel: &str,
    terminal: &str,
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
    
    new_content.push_str("</fontconfig>\n");
    
    if let Some(parent) = std::path::Path::new(FONTS_CONF_PATH).parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(FONTS_CONF_PATH, new_content);
    
    let _ = std::process::Command::new("fc-cache")
        .arg("-f")
        .spawn();
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
    let (sans, serif, mono, borders, status, fuzzel_font, term) = read_preferred_fonts();
    
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

    let menu_options = vec![
        "Sans-Serif".to_string(),
        "Serif".to_string(),
        "Monospace".to_string(),
        "Other".to_string(),
    ];

    let mut borders_box = TextBox::new(borders.clone()).with_label("Window Borders");
    borders_box.disabled = borders_idx != 3;

    let mut status_box = TextBox::new(status.clone()).with_label("Status Interface");
    status_box.disabled = status_idx != 3;

    let mut fuzzel_box = TextBox::new(fuzzel_font.clone()).with_label("Fuzzel");
    fuzzel_box.disabled = fuzzel_idx != 3;

    let mut terminal_box = TextBox::new(term.clone()).with_label("Terminal");
    terminal_box.disabled = terminal_idx != 3;

    TypefaceState {
        loaded: true,
        sans_serif: sans.clone(),
        serif: serif.clone(),
        monospace: mono.clone(),
        window_borders: borders,
        status_interface: status,
        fuzzel: fuzzel_font,
        terminal: term,
        all_fonts,
        mono_fonts,
        sans_box: TextBox::new(sans).with_label("Sans-Serif"),
        serif_box: TextBox::new(serif).with_label("Serif"),
        mono_box: TextBox::new(mono).with_label("Monospace"),
        borders_box,
        status_box,
        fuzzel_box,
        terminal_box,
        search_box: TextBox::new(String::new()).with_label("Filter Fonts"),
        selected_font,
        list_box: ScrollingList::new(24.0, 4.0),
        borders_menu: Dropdown::new(menu_options.clone(), borders_idx),
        status_menu: Dropdown::new(menu_options.clone(), status_idx),
        fuzzel_menu: Dropdown::new(menu_options.clone(), fuzzel_idx),
        terminal_menu: Dropdown::new(menu_options.clone(), terminal_idx),
    }
}

const TEXT_DIM: [f32; 4] = [0.53, 0.53, 0.60, 1.0];

pub fn view(state: &mut TypefaceState, cx: f32, cy: f32, cw: f32, _ch: f32, sec_focused: &[bool]) -> PageContent {
    let mut pc = PageContent::new();
    let mut y = cy + 12.0;

    let widget_w = cw - 24.0;
    let widget_h = 26.0;

    // ── System Typefaces Section ──
    let mut sec = Section::new(&mut pc, cx, y, cw, "System Typefaces");
    sec.spacing(8.0);

    if !state.loaded {
        sec.text(&mut pc, "Loading typefaces...", 12.0, 0.0, 12.0, TEXT_DIM);
        sec.spacing(18.0);
    } else {
        // Sans-Serif
        sec.widget(&mut pc, &mut state.sans_box, 12.0, widget_w, widget_h);
        sec.spacing(12.0);

        // Serif
        sec.widget(&mut pc, &mut state.serif_box, 12.0, widget_w, widget_h);
        sec.spacing(12.0);

        // Monospace
        sec.widget(&mut pc, &mut state.mono_box, 12.0, widget_w, widget_h);
        sec.spacing(8.0);
    }
    let sys_focused = sec_focused.get(0).copied().unwrap_or(false);
    y = sec.finish_focused(&mut pc, sys_focused);

    // ── Program Typefaces Section ──
    let mut sec = Section::new(&mut pc, cx, y, cw, "Program Typefaces");
    sec.spacing(8.0);

    if !state.loaded {
        sec.text(&mut pc, "Loading typefaces...", 12.0, 0.0, 12.0, TEXT_DIM);
        sec.spacing(18.0);
    } else {
        let dropdown_w = 120.0;
        let textbox_w = widget_w - dropdown_w - 12.0;

        // Window Borders
        let start_y = sec.ay();
        let top_room = state.borders_box.top_room();
        state.borders_menu.set_row_rect(cx + 8.0, cw - 16.0);
        clear_ui::layout::render_widget(&mut pc, &mut state.borders_menu, cx + 12.0, start_y + top_room, dropdown_w, widget_h);
        state.borders_box.set_row_rect(cx + 8.0, cw - 16.0);
        clear_ui::layout::render_widget(&mut pc, &mut state.borders_box, cx + 12.0 + dropdown_w + 12.0, start_y + top_room, textbox_w, widget_h);
        sec.spacing(widget_h + top_room + 12.0);

        // Status Interface
        let start_y = sec.ay();
        let top_room = state.status_box.top_room();
        state.status_menu.set_row_rect(cx + 8.0, cw - 16.0);
        clear_ui::layout::render_widget(&mut pc, &mut state.status_menu, cx + 12.0, start_y + top_room, dropdown_w, widget_h);
        state.status_box.set_row_rect(cx + 8.0, cw - 16.0);
        clear_ui::layout::render_widget(&mut pc, &mut state.status_box, cx + 12.0 + dropdown_w + 12.0, start_y + top_room, textbox_w, widget_h);
        sec.spacing(widget_h + top_room + 12.0);

        // Fuzzel
        let start_y = sec.ay();
        let top_room = state.fuzzel_box.top_room();
        state.fuzzel_menu.set_row_rect(cx + 8.0, cw - 16.0);
        clear_ui::layout::render_widget(&mut pc, &mut state.fuzzel_menu, cx + 12.0, start_y + top_room, dropdown_w, widget_h);
        state.fuzzel_box.set_row_rect(cx + 8.0, cw - 16.0);
        clear_ui::layout::render_widget(&mut pc, &mut state.fuzzel_box, cx + 12.0 + dropdown_w + 12.0, start_y + top_room, textbox_w, widget_h);
        sec.spacing(widget_h + top_room + 12.0);

        // Terminal
        let start_y = sec.ay();
        let top_room = state.terminal_box.top_room();
        state.terminal_menu.set_row_rect(cx + 8.0, cw - 16.0);
        clear_ui::layout::render_widget(&mut pc, &mut state.terminal_menu, cx + 12.0, start_y + top_room, dropdown_w, widget_h);
        state.terminal_box.set_row_rect(cx + 8.0, cw - 16.0);
        clear_ui::layout::render_widget(&mut pc, &mut state.terminal_box, cx + 12.0 + dropdown_w + 12.0, start_y + top_room, textbox_w, widget_h);
        sec.spacing(widget_h + top_room + 8.0);
    }
    let prog_focused = sec_focused.get(1).copied().unwrap_or(false);
    y = sec.finish_focused(&mut pc, prog_focused);

    // ── Typefaces Section (List & Preview) ──
    let mut sec = Section::new(&mut pc, cx, y, cw, "Typefaces");
    sec.spacing(12.0);

    if !state.loaded {
        sec.text(&mut pc, "Loading installed fonts...", 12.0, 0.0, 12.0, TEXT_DIM);
        sec.spacing(18.0);
    } else {
        let start_y = sec.ay();
        let usable_w = cw - 24.0;
        let gap = 24.0;
        let left_w = (usable_w - gap) * 0.40;
        let right_w = (usable_w - gap) * 0.60;
        let left_x = cx + 12.0;
        let right_x = left_x + left_w + gap;

        // 1. Render Left Column (Search + List Box)
        let mut left_y = start_y;
        
        let top_room = state.search_box.top_room();
        let search_box_h = widget_h + top_room;
        state.search_box.set_row_rect(left_x, left_w);
        clear_ui::layout::render_widget(
            &mut pc,
            &mut state.search_box,
            left_x,
            left_y + top_room,
            left_w,
            widget_h,
        );
        left_y += search_box_h + 12.0;

        // Scrolling box configuration
        let list_box_y = left_y;
        let list_box_h = 320.0;
        
        // Render the standardized ScrollingList widget
        clear_ui::layout::render_widget(&mut pc, &mut state.list_box, left_x, list_box_y, left_w, list_box_h);

        let query = state.search_box.text.to_lowercase();
        let matching_fonts: Vec<&String> = state.all_fonts.iter()
            .filter(|font| font.to_lowercase().contains(&query))
            .collect();

        let btn_h = 24.0;
        let inner_x = left_x + 4.0;
        let inner_w = left_w - 16.0; // leave room for scrollbar
        
        // Update ScrollingList bounds to clamp and render correctly
        state.list_box.update_bounds(matching_fonts.len(), list_box_y, list_box_h);

        // Render visible buttons inside scroll region
        
        for (idx, font_name) in matching_fonts.iter().enumerate() {
            // Only render buttons that are completely within the visible area
            if let Some(draw_y) = state.list_box.get_item_draw_y(idx, 0.0) {
                let is_selected = state.selected_font.as_ref() == Some(*font_name);
                
                let (bg, hover_bg, text_color) = if is_selected {
                    ([0.20, 0.40, 0.65, 0.4], [0.30, 0.52, 0.78, 0.6], [0.90, 0.90, 0.95, 1.0])
                } else {
                    ([0.0, 0.0, 0.0, 0.0], [0.20, 0.20, 0.25, 0.15], [0.70, 0.70, 0.75, 1.0])
                };
                
                pc.button(
                    font_name,
                    inner_x,
                    draw_y,
                    inner_w - 44.0,
                    btn_h,
                    bg,
                    hover_bg,
                    text_color,
                    crate::app::AppAction::Typeface(TypefaceMessage::SelectFont((*font_name).clone())),
                );

                let (copy_bg, copy_hover_bg, copy_text_color) = if is_selected {
                    ([0.20, 0.40, 0.65, 0.2], [0.30, 0.52, 0.78, 0.5], [0.90, 0.90, 0.95, 1.0])
                } else {
                    ([0.0, 0.0, 0.0, 0.0], [0.20, 0.20, 0.25, 0.25], [0.70, 0.70, 0.75, 1.0])
                };

                pc.button(
                    "📋",
                    inner_x + inner_w - 40.0,
                    draw_y,
                    40.0,
                    btn_h,
                    copy_bg,
                    copy_hover_bg,
                    copy_text_color,
                    crate::app::AppAction::Typeface(TypefaceMessage::CopyFontName((*font_name).clone())),
                );
            }
        }
        
        if matching_fonts.is_empty() {
            pc.text("No fonts match query", inner_x + 8.0, list_box_y + 16.0, 12.0, TEXT_DIM);
        }

        left_y += list_box_h;

        // 2. Render Right Column (Live Preview)
        let mut right_y = start_y;
        
        // Render Info Box
        let info_h = 96.0;
        let info_bg = [0.12, 0.18, 0.28, 0.3]; // Sleek translucent blue-ish background
        let info_border = [0.25, 0.40, 0.60, 0.5]; // Soft blue border
        
        pc.rect(info_bg, right_x, right_y, right_w, info_h);
        pc.rect(info_border, right_x, right_y, right_w, 1.0);
        pc.rect(info_border, right_x, right_y + info_h - 1.0, right_w, 1.0);
        pc.rect(info_border, right_x, right_y, 1.0, info_h);
        pc.rect(info_border, right_x + right_w - 1.0, right_y, 1.0, info_h);
        
        let text_padding_x = 16.0;
        let mut text_y = right_y + 12.0;
        
        pc.text("Font Directories & Installation", right_x + text_padding_x, text_y, 12.0, [0.35, 0.65, 0.90, 1.0]);
        text_y += 20.0;
        
        pc.text("• Active Directory: ~/Dropbox/Fonts", right_x + text_padding_x, text_y, 11.0, [0.80, 0.80, 0.85, 1.0]);
        text_y += 16.0;
        
        pc.text("• Place TTF/OTF files there to install new fonts.", right_x + text_padding_x, text_y, 11.0, [0.80, 0.80, 0.85, 1.0]);
        text_y += 16.0;

        pc.text("• Changes will be cached automatically by fontconfig.", right_x + text_padding_x, text_y, 11.0, [0.55, 0.55, 0.60, 1.0]);
        
        right_y += info_h + 12.0;
        
        if let Some(ref font_name) = state.selected_font {
            let card_h = 240.0;
            pc.rect([0.10, 0.10, 0.14, 0.3], right_x, right_y, right_w, card_h);
            pc.rect([0.25, 0.25, 0.35, 0.5], right_x, right_y, right_w, 1.0);
            pc.rect([0.25, 0.25, 0.35, 0.5], right_x, right_y + card_h - 1.0, right_w, 1.0);
            pc.rect([0.25, 0.25, 0.35, 0.5], right_x, right_y, 1.0, card_h);
            pc.rect([0.25, 0.25, 0.35, 0.5], right_x + right_w - 1.0, right_y, 1.0, card_h);
            
            let text_padding_x = 16.0;
            let mut text_y = right_y + 16.0;
            
            pc.text(&format!("Family: {}", font_name), right_x + text_padding_x, text_y, 15.0, [0.90, 0.90, 0.95, 1.0]);
            text_y += 28.0;
            
            pc.rect([0.22, 0.22, 0.30, 0.8], right_x + text_padding_x, text_y, right_w - (text_padding_x * 2.0), 1.0);
            text_y += 16.0;
            
            pc.text_with_font(
                "abcdefghijklmnopqrstuvwxyz",
                right_x + text_padding_x,
                text_y,
                13.0,
                [0.75, 0.75, 0.80, 1.0],
                font_name,
            );
            text_y += 22.0;
            
            pc.text_with_font(
                "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
                right_x + text_padding_x,
                text_y,
                13.0,
                [0.75, 0.75, 0.80, 1.0],
                font_name,
            );
            text_y += 22.0;
            
            pc.text_with_font(
                "0123456789 (!@#$%&*?)",
                right_x + text_padding_x,
                text_y,
                13.0,
                [0.75, 0.75, 0.80, 1.0],
                font_name,
            );
            text_y += 26.0;
            
            pc.text_with_font(
                "The quick brown fox jumps over the lazy dog.",
                right_x + text_padding_x,
                text_y,
                18.0,
                [0.90, 0.90, 0.95, 1.0],
                font_name,
            );
            text_y += 32.0;

            pc.text_with_font(
                "The five boxing wizards jump quickly.",
                right_x + text_padding_x,
                text_y,
                24.0,
                [0.95, 0.95, 1.0, 1.0],
                font_name,
            );
            
            right_y += card_h;
        } else {
            pc.text("Select a font to preview", right_x + 12.0, right_y + 20.0, 13.0, TEXT_DIM);
            right_y += 40.0;
        }

        sec.content_y = left_y.max(right_y);
    }
    let list_focused = sec_focused.get(2).copied().unwrap_or(false);
    sec.finish_focused(&mut pc, list_focused);

    // Filter out base text items covered by any open popover to prevent showing through
    let mut popovers = Vec::new();
    if state.borders_menu.open {
        let (x, y, w, h) = state.borders_menu.rect();
        popovers.push((x, y + h, w, state.borders_menu.options.len() as f32 * 24.0));
    }
    if state.status_menu.open {
        let (x, y, w, h) = state.status_menu.rect();
        popovers.push((x, y + h, w, state.status_menu.options.len() as f32 * 24.0));
    }
    if state.fuzzel_menu.open {
        let (x, y, w, h) = state.fuzzel_menu.rect();
        popovers.push((x, y + h, w, state.fuzzel_menu.options.len() as f32 * 24.0));
    }
    if state.terminal_menu.open {
        let (x, y, w, h) = state.terminal_menu.rect();
        popovers.push((x, y + h, w, state.terminal_menu.options.len() as f32 * 24.0));
    }

    if !popovers.is_empty() {
        pc.texts.retain(|(_, _, tx, ty, _, _)| {
            for &(px, py, pw, ph) in &popovers {
                if *tx >= px && *tx <= px + pw && *ty >= py && *ty <= py + ph {
                    return false;
                }
            }
            true
        });
    }

    // Render dropdown popovers on top of all other widgets
    state.borders_menu.render_popover(&mut pc);
    state.status_menu.render_popover(&mut pc);
    state.fuzzel_menu.render_popover(&mut pc);
    state.terminal_menu.render_popover(&mut pc);

    pc
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
            if !state.search_box.editing {
                state.search_box = new.search_box;
            }
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
            save_preferred_fonts(
                &state.sans_serif,
                &state.serif,
                &state.monospace,
                &state.window_borders,
                &state.status_interface,
                &state.fuzzel,
                &state.terminal,
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
            save_preferred_fonts(
                &state.sans_serif,
                &state.serif,
                &state.monospace,
                &state.window_borders,
                &state.status_interface,
                &state.fuzzel,
                &state.terminal,
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
            save_preferred_fonts(
                &state.sans_serif,
                &state.serif,
                &state.monospace,
                &state.window_borders,
                &state.status_interface,
                &state.fuzzel,
                &state.terminal,
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
                    if let Ok(mut child) = std::process::Command::new("wl-copy")
                        .stdin(std::process::Stdio::piped())
                        .spawn()
                    {
                        if let Some(mut stdin) = child.stdin.take() {
                            if stdin.write_all(text.as_bytes()).is_ok() {
                                copied = true;
                            }
                        }
                        let _ = child.wait();
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
            );
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
