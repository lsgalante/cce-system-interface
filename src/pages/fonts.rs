use std::fs;
use cce_ui::layout::LayoutStrategy;
use cce_ui::widget::{TextBox, Dropdown, Spinbox};
use crate::app::{PageContent, AppAction};
use crate::pages::AppPage;

fn fonts_conf_path() -> String {
    cce_ui::config::config_home().join("fontconfig").join("fonts.conf").to_string_lossy().into_owned()
}
fn fuzzel_ini_path() -> String {
    cce_ui::config::config_home().join("fuzzel").join("fuzzel.ini").to_string_lossy().into_owned()
}
fn foot_ini_path() -> String {
    cce_ui::config::config_home().join("foot").join("foot.ini").to_string_lossy().into_owned()
}

#[derive(Debug, Clone)]
pub enum FontsMessage {
    TypefaceRefreshed(FontsState),
    Refreshed(FontsState),
    SetSans(String),
    SetSerif(String),
    SetMono(String),
    SetBorders(String),
    SetStatus(String),
    SetFuzzel(String),
    SetTerminal(String),
    SetBordersIdx(usize),
    SetStatusIdx(usize),
    SetFuzzelIdx(usize),
    SetTerminalIdx(usize),
    SetFuzzelSize(u16),
    SetTerminalSize(u16),
}

#[derive(Clone)]
pub struct FontsState {
    pub typeface_loaded: bool,
    pub sans_serif: String,
    pub serif: String,
    pub monospace: String,
    pub window_borders: String,
    pub status_interface: String,
    pub fuzzel: String,
    pub terminal: String,
    pub all_fonts: Vec<String>,
    pub mono_fonts: Vec<String>,
    pub sans_box: cce_ui::widget::Adapted<TextBox>,
    pub serif_box: cce_ui::widget::Adapted<TextBox>,
    pub mono_box: cce_ui::widget::Adapted<TextBox>,
    pub borders_box: cce_ui::widget::Adapted<TextBox>,
    pub status_box: cce_ui::widget::Adapted<TextBox>,
    pub fuzzel_box: cce_ui::widget::Adapted<TextBox>,
    pub terminal_box: cce_ui::widget::Adapted<TextBox>,
    pub borders_menu: cce_ui::widget::Adapted<Dropdown>,
    pub status_menu: cce_ui::widget::Adapted<Dropdown>,
    pub fuzzel_menu: cce_ui::widget::Adapted<Dropdown>,
    pub terminal_menu: cce_ui::widget::Adapted<Dropdown>,
    pub fuzzel_size_box: cce_ui::widget::Adapted<cce_ui::widget::Spinbox>,
    pub terminal_size_box: cce_ui::widget::Adapted<cce_ui::widget::Spinbox>,
}

impl std::fmt::Debug for FontsState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FontsState").finish()
    }
}

impl Default for FontsState {
    fn default() -> Self {
        Self {
            typeface_loaded: false,
            sans_serif: "Noto Sans".to_string(),
            serif: "Noto Serif".to_string(),
            monospace: "Noto Sans Mono".to_string(),
            window_borders: "Noto Sans".to_string(),
            status_interface: "Noto Sans".to_string(),
            fuzzel: "Noto Sans".to_string(),
            terminal: "Noto Sans Mono".to_string(),
            all_fonts: Vec::new(),
            mono_fonts: Vec::new(),
            sans_box: TextBox::new(String::new()).with_label("Sans-Serif").with_config(&fonts_conf_path() , "sans-serif"),
            serif_box: TextBox::new(String::new()).with_label("Serif").with_config(&fonts_conf_path() , "serif"),
            mono_box: TextBox::new(String::new()).with_label("Monospace").with_config(&fonts_conf_path() , "monospace"),
            borders_box: TextBox::new(String::new()).with_label("Window Borders").with_config(&fonts_conf_path() , "window_borders"),
            status_box: TextBox::new(String::new()).with_label("Status Interface").with_config(&fonts_conf_path() , "status_interface"),
            fuzzel_box: TextBox::new(String::new()).with_label("Fuzzel").with_config(&fonts_conf_path() , "fuzzel"),
            terminal_box: TextBox::new(String::new()).with_label("Terminal").with_config(&fonts_conf_path() , "terminal"),
            borders_menu: Dropdown::new(Vec::new(), 0).with_config(&fonts_conf_path() , "window_borders"),
            status_menu: Dropdown::new(Vec::new(), 0).with_config(&fonts_conf_path() , "status_interface"),
            fuzzel_menu: Dropdown::new(Vec::new(), 0).with_config(&fonts_conf_path() , "fuzzel"),
            terminal_menu: Dropdown::new(Vec::new(), 0).with_config(&fonts_conf_path() , "terminal"),
            fuzzel_size_box: Spinbox::new(14, 6, 72, 1).with_config(&fuzzel_ini_path() , "size"),
            terminal_size_box: Spinbox::new(12, 6, 72, 1).with_config(&foot_ini_path() , "size"),
        }
    }
}

impl AppPage for FontsState {
    // Sections: [Preferred Fonts, Borders, Status Interface, Fuzzel, Terminal]
    fn section_widgets(&mut self) -> Vec<Vec<cce_ui::widget::WidgetId>> {
        vec![
            vec![self.sans_box.id(), self.serif_box.id(), self.mono_box.id()],
            vec![self.borders_menu.id(), self.borders_box.id()],
            vec![self.status_menu.id(), self.status_box.id()],
            vec![self.fuzzel_menu.id(), self.fuzzel_box.id(), self.fuzzel_size_box.id()],
            vec![self.terminal_menu.id(), self.terminal_box.id(), self.terminal_size_box.id()],
        ]
    }

    // Not every section widget passes through `render_widget`'s registration side
    // effect (the menus draw custom); the id-rooted router needs them all resolvable.
    fn register_extra_dispatch_roots(&mut self, ctx: &mut cce_ui::context::UiContext) {
        let (id, ptr) = (self.sans_box.id(), self.sans_box.as_ptr_mut());
        ctx.register_widget(id, ptr);
        let (id, ptr) = (self.serif_box.id(), self.serif_box.as_ptr_mut());
        ctx.register_widget(id, ptr);
        let (id, ptr) = (self.mono_box.id(), self.mono_box.as_ptr_mut());
        ctx.register_widget(id, ptr);
        let (id, ptr) = (self.borders_menu.id(), self.borders_menu.as_ptr_mut());
        ctx.register_widget(id, ptr);
        let (id, ptr) = (self.borders_box.id(), self.borders_box.as_ptr_mut());
        ctx.register_widget(id, ptr);
        let (id, ptr) = (self.status_menu.id(), self.status_menu.as_ptr_mut());
        ctx.register_widget(id, ptr);
        let (id, ptr) = (self.status_box.id(), self.status_box.as_ptr_mut());
        ctx.register_widget(id, ptr);
        let (id, ptr) = (self.fuzzel_menu.id(), self.fuzzel_menu.as_ptr_mut());
        ctx.register_widget(id, ptr);
        let (id, ptr) = (self.fuzzel_box.id(), self.fuzzel_box.as_ptr_mut());
        ctx.register_widget(id, ptr);
        let (id, ptr) = (self.fuzzel_size_box.id(), self.fuzzel_size_box.as_ptr_mut());
        ctx.register_widget(id, ptr);
        let (id, ptr) = (self.terminal_menu.id(), self.terminal_menu.as_ptr_mut());
        ctx.register_widget(id, ptr);
        let (id, ptr) = (self.terminal_box.id(), self.terminal_box.as_ptr_mut());
        ctx.register_widget(id, ptr);
        let (id, ptr) = (self.terminal_size_box.id(), self.terminal_size_box.as_ptr_mut());
        ctx.register_widget(id, ptr);
    }

    fn view(
        &mut self,
        cx: f32,
        cy: f32,
        cw: f32,
        ch: f32,
        _root_focused: bool,
        sec_focused: &[bool],
        layout: &mut dyn LayoutStrategy,
        ctx: &mut cce_ui::context::UiContext,
    ) -> PageContent {
        let mut final_pc = PageContent::new();
        let sec_w = 260.0f32;
        let mut builder = cce_ui::layout::PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(5);

        builder.add_section(&mut final_pc, "Preferred Fonts", sec_focused.get(0).copied().unwrap_or(false), |sec| {
            let mut stack = sec.vstack(8.0);
            let sec_w = stack.context.cw;
            stack.add_widget(&mut self.sans_box, sec_w - 28.0, 44.0, ctx);
            stack.add_widget(&mut self.serif_box, sec_w - 28.0, 44.0, ctx);
            stack.add_widget(&mut self.mono_box, sec_w - 28.0, 44.0, ctx);
        });

        builder.add_section(&mut final_pc, "Borders", sec_focused.get(1).copied().unwrap_or(false), |sec| {
            let mut stack = sec.vstack(8.0);
            let sec_w = stack.context.cw;
            stack.add_widget(&mut self.borders_menu, sec_w - 28.0, 44.0, ctx);
            if self.borders_menu.selected == 3 {
                stack.add_widget(&mut self.borders_box, sec_w - 28.0, 44.0, ctx);
            }
        });

        builder.add_section(&mut final_pc, "Status Interface", sec_focused.get(2).copied().unwrap_or(false), |sec| {
            let mut stack = sec.vstack(8.0);
            let sec_w = stack.context.cw;
            stack.add_widget(&mut self.status_menu, sec_w - 28.0, 44.0, ctx);
            if self.status_menu.selected == 3 {
                stack.add_widget(&mut self.status_box, sec_w - 28.0, 44.0, ctx);
            }
        });

        builder.add_section(&mut final_pc, "Fuzzel", sec_focused.get(3).copied().unwrap_or(false), |sec| {
            let mut stack = sec.vstack(8.0);
            let sec_w = stack.context.cw;
            stack.add_widget(&mut self.fuzzel_menu, sec_w - 28.0, 44.0, ctx);
            if self.fuzzel_menu.selected == 3 {
                stack.add_widget(&mut self.fuzzel_box, sec_w - 28.0, 44.0, ctx);
            }
            stack.add_widget(&mut self.fuzzel_size_box, sec_w - 28.0, 44.0, ctx);
        });

        builder.add_section(&mut final_pc, "Terminal", sec_focused.get(4).copied().unwrap_or(false), |sec| {
            let mut stack = sec.vstack(8.0);
            let sec_w = stack.context.cw;
            stack.add_widget(&mut self.terminal_menu, sec_w - 28.0, 44.0, ctx);
            if self.terminal_menu.selected == 3 {
                stack.add_widget(&mut self.terminal_box, sec_w - 28.0, 44.0, ctx);
            }
            stack.add_widget(&mut self.terminal_size_box, sec_w - 28.0, 44.0, ctx);
        });

        final_pc
    }

    fn propagate_widget_changes(&mut self, actions: &mut Vec<AppAction>) {
        if self.sans_box.take_change() {
            actions.push(AppAction::Fonts(FontsMessage::SetSans(self.sans_box.text.clone())));
        }
        if self.serif_box.take_change() {
            actions.push(AppAction::Fonts(FontsMessage::SetSerif(self.serif_box.text.clone())));
        }
        if self.mono_box.take_change() {
            actions.push(AppAction::Fonts(FontsMessage::SetMono(self.mono_box.text.clone())));
        }
        if self.borders_box.take_change() {
            actions.push(AppAction::Fonts(FontsMessage::SetBorders(self.borders_box.text.clone())));
        }
        if self.status_box.take_change() {
            actions.push(AppAction::Fonts(FontsMessage::SetStatus(self.status_box.text.clone())));
        }
        if self.fuzzel_box.take_change() {
            actions.push(AppAction::Fonts(FontsMessage::SetFuzzel(self.fuzzel_box.text.clone())));
        }
        if self.terminal_box.take_change() {
            actions.push(AppAction::Fonts(FontsMessage::SetTerminal(self.terminal_box.text.clone())));
        }

        if self.borders_menu.take_change() {
            actions.push(AppAction::Fonts(FontsMessage::SetBordersIdx(self.borders_menu.selected)));
        }
        if self.status_menu.take_change() {
            actions.push(AppAction::Fonts(FontsMessage::SetStatusIdx(self.status_menu.selected)));
        }
        if self.fuzzel_menu.take_change() {
            actions.push(AppAction::Fonts(FontsMessage::SetFuzzelIdx(self.fuzzel_menu.selected)));
        }
        if self.terminal_menu.take_change() {
            actions.push(AppAction::Fonts(FontsMessage::SetTerminalIdx(self.terminal_menu.selected)));
        }

        if self.fuzzel_size_box.take_change() {
            actions.push(AppAction::Fonts(FontsMessage::SetFuzzelSize(self.fuzzel_size_box.value as u16)));
        }
        if self.terminal_size_box.take_change() {
            actions.push(AppAction::Fonts(FontsMessage::SetTerminalSize(self.terminal_size_box.value as u16)));
        }
    }
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
    let content = fs::read_to_string(&fonts_conf_path() ).unwrap_or_default();
    
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
    let content = fs::read_to_string(&fonts_conf_path() ).unwrap_or_default();
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
    let _ = fs::write(&fonts_conf_path() , new_content);
}

fn read_fuzzel_size() -> Option<u16> {
    let ini = fs::read_to_string(&fuzzel_ini_path() ).ok()?;
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
    let path = &fuzzel_ini_path() ;
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
    let ini = fs::read_to_string(&foot_ini_path() ).ok()?;
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
    let path = &foot_ini_path() ;
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

pub async fn fetch_typeface_state() -> FontsState {
    read_typeface_config()
}

fn determine_dropdown_index(val: &str, sans: &str, serif: &str, mono: &str) -> usize {
    if val == sans { 0 }
    else if val == serif { 1 }
    else if val == mono { 2 }
    else { 3 }
}

fn determine_font_from_index(idx: usize, state: &FontsState) -> String {
    match idx {
        0 => state.sans_serif.clone(),
        1 => state.serif.clone(),
        2 => state.monospace.clone(),
        _ => "Noto Sans".to_string(),
    }
}

pub fn read_typeface_config() -> FontsState {
    let (sans, serif, mono, borders, status, fuzzel_font, term) = read_preferred_fonts();
    
    let font_dirs = vec![
        "/usr/share/fonts".to_string(),
        cce_ui::fonts_dir(),
    ];
    let mut all_fonts = Vec::new();
    let mut mono_fonts = Vec::new();
    for d in font_dirs {
        if let Ok(entries) = fs::read_dir(d) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.ends_with(".ttf") || name.ends_with(".otf") {
                        let name_clean = name.trim_end_matches(".ttf").trim_end_matches(".otf").replace("-", " ");
                        all_fonts.push(name_clean.clone());
                        if name_clean.to_lowercase().contains("mono") {
                            mono_fonts.push(name_clean);
                        }
                    }
                }
            }
        }
    }
    all_fonts.sort();
    all_fonts.dedup();
    mono_fonts.sort();
    mono_fonts.dedup();

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

    let mut borders_box = TextBox::new(borders.clone()).with_label("Window Borders").with_config(&fonts_conf_path() , "window_borders");
    borders_box.disabled = borders_idx != 3;

    let mut status_box = TextBox::new(status.clone()).with_label("Status Interface").with_config(&fonts_conf_path() , "status_interface");
    status_box.disabled = status_idx != 3;

    let mut fuzzel_box = TextBox::new(fuzzel_font.clone()).with_label("Fuzzel").with_config(&fonts_conf_path() , "fuzzel");
    fuzzel_box.disabled = fuzzel_idx != 3;

    let mut terminal_box = TextBox::new(term.clone()).with_label("Terminal").with_config(&fonts_conf_path() , "terminal");
    terminal_box.disabled = terminal_idx != 3;

    let fuzzel_size = read_fuzzel_size().unwrap_or(14);
    let terminal_size = read_terminal_size().unwrap_or(12);

    let mut state = FontsState::default();
    state.typeface_loaded = true;
    state.sans_serif = sans.clone();
    state.serif = serif.clone();
    state.monospace = mono.clone();
    state.window_borders = borders;
    state.status_interface = status;
    state.fuzzel = fuzzel_font;
    state.terminal = term;
    state.all_fonts = all_fonts;
    state.mono_fonts = mono_fonts;
    state.sans_box = TextBox::new(sans).with_label("Sans-Serif").with_config(&fonts_conf_path() , "sans-serif");
    state.serif_box = TextBox::new(serif).with_label("Serif").with_config(&fonts_conf_path() , "serif");
    state.mono_box = TextBox::new(mono).with_label("Monospace").with_config(&fonts_conf_path() , "monospace");
    state.borders_box = borders_box;
    state.status_box = status_box;
    state.fuzzel_box = fuzzel_box;
    state.terminal_box = terminal_box;
    state.borders_menu = Dropdown::new(menu_options.clone(), borders_idx).with_config(&fonts_conf_path() , "window_borders");
    state.status_menu = Dropdown::new(menu_options.clone(), status_idx).with_config(&fonts_conf_path() , "status_interface");
    state.fuzzel_menu = Dropdown::new(menu_options.clone(), fuzzel_idx).with_config(&fonts_conf_path() , "fuzzel");
    state.terminal_menu = Dropdown::new(menu_options, terminal_idx).with_config(&fonts_conf_path() , "terminal");
    state.fuzzel_size_box = Spinbox::new(fuzzel_size as i32, 6, 72, 1).with_config(&fuzzel_ini_path() , "size");
    state.terminal_size_box = Spinbox::new(terminal_size as i32, 6, 72, 1).with_config(&foot_ini_path() , "size");
    state
}

pub fn update(state: &mut FontsState, msg: FontsMessage) {
    match msg {
        FontsMessage::TypefaceRefreshed(refreshed) | FontsMessage::Refreshed(refreshed) => {
            state.typeface_loaded = refreshed.typeface_loaded;
            state.sans_serif = refreshed.sans_serif;
            state.serif = refreshed.serif;
            state.monospace = refreshed.monospace;
            state.window_borders = refreshed.window_borders;
            state.status_interface = refreshed.status_interface;
            state.fuzzel = refreshed.fuzzel;
            state.terminal = refreshed.terminal;
            state.all_fonts = refreshed.all_fonts;
            state.mono_fonts = refreshed.mono_fonts;
        }
        FontsMessage::SetSans(font) => {
            state.sans_serif = font;
            save_preferred_fonts(&state.sans_serif, &state.serif, &state.monospace, &state.window_borders, &state.status_interface, &state.fuzzel, &state.terminal);
        }
        FontsMessage::SetSerif(font) => {
            state.serif = font;
            save_preferred_fonts(&state.sans_serif, &state.serif, &state.monospace, &state.window_borders, &state.status_interface, &state.fuzzel, &state.terminal);
        }
        FontsMessage::SetMono(font) => {
            state.monospace = font;
            save_preferred_fonts(&state.sans_serif, &state.serif, &state.monospace, &state.window_borders, &state.status_interface, &state.fuzzel, &state.terminal);
        }
        FontsMessage::SetBorders(font) => {
            state.window_borders = font;
            save_preferred_fonts(&state.sans_serif, &state.serif, &state.monospace, &state.window_borders, &state.status_interface, &state.fuzzel, &state.terminal);
        }
        FontsMessage::SetStatus(font) => {
            state.status_interface = font;
            save_preferred_fonts(&state.sans_serif, &state.serif, &state.monospace, &state.window_borders, &state.status_interface, &state.fuzzel, &state.terminal);
        }
        FontsMessage::SetFuzzel(font) => {
            state.fuzzel = font;
            save_preferred_fonts(&state.sans_serif, &state.serif, &state.monospace, &state.window_borders, &state.status_interface, &state.fuzzel, &state.terminal);
        }
        FontsMessage::SetTerminal(font) => {
            state.terminal = font;
            save_preferred_fonts(&state.sans_serif, &state.serif, &state.monospace, &state.window_borders, &state.status_interface, &state.fuzzel, &state.terminal);
        }
        FontsMessage::SetBordersIdx(idx) => {
            state.borders_menu.selected = idx;
            state.borders_box.disabled = idx != 3;
            if idx < 3 {
                state.window_borders = determine_font_from_index(idx, state);
                state.borders_box.set_value_string(&state.window_borders);
                save_preferred_fonts(&state.sans_serif, &state.serif, &state.monospace, &state.window_borders, &state.status_interface, &state.fuzzel, &state.terminal);
            }
        }
        FontsMessage::SetStatusIdx(idx) => {
            state.status_menu.selected = idx;
            state.status_box.disabled = idx != 3;
            if idx < 3 {
                state.status_interface = determine_font_from_index(idx, state);
                state.status_box.set_value_string(&state.status_interface);
                save_preferred_fonts(&state.sans_serif, &state.serif, &state.monospace, &state.window_borders, &state.status_interface, &state.fuzzel, &state.terminal);
            }
        }
        FontsMessage::SetFuzzelIdx(idx) => {
            state.fuzzel_menu.selected = idx;
            state.fuzzel_box.disabled = idx != 3;
            if idx < 3 {
                state.fuzzel = determine_font_from_index(idx, state);
                state.fuzzel_box.set_value_string(&state.fuzzel);
                save_preferred_fonts(&state.sans_serif, &state.serif, &state.monospace, &state.window_borders, &state.status_interface, &state.fuzzel, &state.terminal);
            }
        }
        FontsMessage::SetTerminalIdx(idx) => {
            state.terminal_menu.selected = idx;
            state.terminal_box.disabled = idx != 3;
            if idx < 3 {
                state.terminal = determine_font_from_index(idx, state);
                state.terminal_box.set_value_string(&state.terminal);
                save_preferred_fonts(&state.sans_serif, &state.serif, &state.monospace, &state.window_borders, &state.status_interface, &state.fuzzel, &state.terminal);
            }
        }
        FontsMessage::SetFuzzelSize(size) => {
            write_fuzzel_size(size);
        }
        FontsMessage::SetTerminalSize(size) => {
            write_terminal_size(size);
        }
    }
}
