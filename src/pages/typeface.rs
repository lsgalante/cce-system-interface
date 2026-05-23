use std::fs;
use crate::app::PageContent;
use clear_ui::layout::Section;
use clear_ui::widget::{Widget, TextLabel};
use winit::event::{ElementState, KeyEvent, MouseButton};
use winit::keyboard::{Key, NamedKey};

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
        }
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
    fn set_hovered(&mut self, v: bool) { self.hovered = v; }
    fn hovered(&self) -> bool { self.hovered }

    fn color(&self) -> [f32; 4] {
        [0.10, 0.10, 0.16, 1.0]
    }

    fn cursor_moved(&mut self, px: f32, py: f32) -> bool {
        let was = self.hovered;
        self.hovered = self.hit_test(px, py);
        was != self.hovered
    }

    fn mouse_input(&mut self, button: MouseButton, state: ElementState, px: f32, py: f32) -> bool {
        if button != MouseButton::Left { return false; }
        if state != ElementState::Pressed { return false; }
        if !self.hit_test(px, py) { return false; }
        self.focus();
        true
    }

    fn focus(&mut self) {
        if !self.editing {
            self.editing = true;
            self.edit_buffer = self.text.clone();
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
        if self.editing {
            Some([1.0, 1.0, 1.0, 0.12])
        } else if self.hovered {
            Some([1.0, 1.0, 1.0, 0.06])
        } else {
            None
        }
    }

    fn extra_quads(&self) -> Vec<(f32, f32, f32, f32, [f32; 4])> {
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
        vec![
            (self.x, self.y, self.w, self.h, border_color),
            (self.x + 1.0, self.y + 1.0, self.w - 2.0, self.h - 2.0, bg_color),
        ]
    }

    fn text_labels(&self) -> Vec<TextLabel> {
        let val_text = if self.editing {
            format!("{}|", self.edit_buffer)
        } else {
            self.text.clone()
        };
        vec![TextLabel {
            text: val_text,
            x: self.x + 8.0,
            y: self.y + (self.h - 12.0) / 2.0,
            font_size: 13.0,
            color: if self.editing { [0xee, 0xee, 0xf5] } else { [0xcc, 0xcc, 0xd4] },
        }]
    }
}

// ── TypefaceState and TypefaceMessage ──

#[derive(Debug, Clone)]
pub struct TypefaceState {
    pub loaded: bool,
    pub sans_serif: String,
    pub serif: String,
    pub monospace: String,
    pub all_fonts: Vec<String>,
    pub mono_fonts: Vec<String>,
    pub sans_box: TextBox,
    pub serif_box: TextBox,
    pub mono_box: TextBox,
}

impl Default for TypefaceState {
    fn default() -> Self {
        Self {
            loaded: false,
            sans_serif: String::new(),
            serif: String::new(),
            monospace: String::new(),
            all_fonts: Vec::new(),
            mono_fonts: Vec::new(),
            sans_box: TextBox::default(),
            serif_box: TextBox::default(),
            mono_box: TextBox::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum TypefaceMessage {
    Refreshed(TypefaceState),
    SetSans(String),
    SetSerif(String),
    SetMono(String),
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

pub fn read_preferred_fonts() -> (String, String, String) {
    let content = fs::read_to_string(FONTS_CONF_PATH).unwrap_or_default();
    
    let sans = parse_font_for_alias(&content, "sans-serif").unwrap_or_else(|| "Noto Sans".to_string());
    let serif = parse_font_for_alias(&content, "serif").unwrap_or_else(|| "Noto Serif".to_string());
    let mono = parse_font_for_alias(&content, "monospace").unwrap_or_else(|| "Noto Sans Mono".to_string());
    
    (sans, serif, mono)
}

pub fn save_preferred_fonts(sans: &str, serif: &str, mono: &str) {
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
    let (sans, serif, mono) = read_preferred_fonts();
    
    let all_output = tokio::process::Command::new("fc-list")
        .args([":", "family"])
        .output().await.ok();
    let all_fonts = parse_families(all_output);
    
    let mono_output = tokio::process::Command::new("fc-list")
        .args([":spacing=100", "family"])
        .output().await.ok();
    let mono_fonts = parse_families(mono_output);
    
    TypefaceState {
        loaded: true,
        sans_serif: sans.clone(),
        serif: serif.clone(),
        monospace: mono.clone(),
        all_fonts,
        mono_fonts,
        sans_box: TextBox::new(sans),
        serif_box: TextBox::new(serif),
        mono_box: TextBox::new(mono),
    }
}

const TEXT_DIM: [f32; 4] = [0.53, 0.53, 0.60, 1.0];

pub fn view(state: &mut TypefaceState, cx: f32, cy: f32, cw: f32, _ch: f32) -> PageContent {
    let mut pc = PageContent::new();
    let mut y = cy + 12.0;

    let widget_w = cw - 24.0;
    let widget_h = 26.0;

    // ── Sans-Serif ──
    let mut sec = Section::new(&mut pc, cx, y, cw, "Sans-Serif");
    sec.spacing(8.0);
    if !state.loaded {
        sec.text(&mut pc, "Loading typefaces...", 12.0, 0.0, 12.0, TEXT_DIM);
        sec.spacing(18.0);
    } else {
        sec.widget(&mut pc, &mut state.sans_box, 12.0, widget_w, widget_h);
        sec.spacing(8.0);
    }
    y = sec.finish(&mut pc);

    // ── Serif ──
    let mut sec = Section::new(&mut pc, cx, y, cw, "Serif");
    sec.spacing(8.0);
    if !state.loaded {
        sec.text(&mut pc, "Loading typefaces...", 12.0, 0.0, 12.0, TEXT_DIM);
        sec.spacing(18.0);
    } else {
        sec.widget(&mut pc, &mut state.serif_box, 12.0, widget_w, widget_h);
        sec.spacing(8.0);
    }
    y = sec.finish(&mut pc);

    // ── Monospace ──
    let mut sec = Section::new(&mut pc, cx, y, cw, "Monospace");
    sec.spacing(8.0);
    if !state.loaded {
        sec.text(&mut pc, "Loading typefaces...", 12.0, 0.0, 12.0, TEXT_DIM);
        sec.spacing(18.0);
    } else {
        sec.widget(&mut pc, &mut state.mono_box, 12.0, widget_w, widget_h);
        sec.spacing(8.0);
    }
    sec.finish(&mut pc);

    pc
}

pub fn update(state: &mut TypefaceState, msg: TypefaceMessage) {
    match msg {
        TypefaceMessage::Refreshed(new) => {
            state.loaded = new.loaded;
            state.all_fonts = new.all_fonts;
            state.mono_fonts = new.mono_fonts;
            if !state.sans_box.editing {
                state.sans_serif = new.sans_serif.clone();
                state.sans_box.text = new.sans_serif;
            }
            if !state.serif_box.editing {
                state.serif = new.serif.clone();
                state.serif_box.text = new.serif;
            }
            if !state.mono_box.editing {
                state.monospace = new.monospace.clone();
                state.mono_box.text = new.monospace;
            }
        }
        TypefaceMessage::SetSans(sans) => {
            state.sans_serif = sans.clone();
            state.sans_box.text = sans;
            save_preferred_fonts(&state.sans_serif, &state.serif, &state.monospace);
        }
        TypefaceMessage::SetSerif(serif) => {
            state.serif = serif.clone();
            state.serif_box.text = serif;
            save_preferred_fonts(&state.sans_serif, &state.serif, &state.monospace);
        }
        TypefaceMessage::SetMono(mono) => {
            state.monospace = mono.clone();
            state.mono_box.text = mono;
            save_preferred_fonts(&state.sans_serif, &state.serif, &state.monospace);
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
