use std::fs;
use std::io::Write;
use crate::app::PageContent;
use clear_ui::layout::Section;
use clear_ui::widget::ColorSelector;

const CONFIG_PATH: &str = "/home/lsgalante/.config/clearwm/config.toml";
const CLEARWM_SOCK: &str = "/tmp/clearwm.sock";

#[derive(Debug, Clone)]
pub struct ColorsState {
    pub low_color: [u8; 3],
    pub high_color: [u8; 3],
    pub disabled_color: [u8; 3],
    pub color_selectors: Vec<ColorSelector>,
    pub preset_colors: Vec<(&'static str, [u8; 3])>,
}

impl Default for ColorsState {
    fn default() -> Self {
        Self {
            low_color: [0x0a, 0x1a, 0x0e],
            high_color: [0x3e, 0x3e, 0x3e],
            disabled_color: [0x55, 0x55, 0x55],
            color_selectors: vec![
                ColorSelector::new([0x0a, 0x1a, 0x0e]).with_label("Low Color"),
                ColorSelector::new([0x3e, 0x3e, 0x3e]).with_label("High Color"),
                ColorSelector::new([0x55, 0x55, 0x55]).with_label("Disabled"),
            ],
            preset_colors: preset_colors(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ColorsMessage {
    SetLowColor([u8; 3]),
    SetHighColor([u8; 3]),
    SetDisabledColor([u8; 3]),
    PickLowColor,
    PickHighColor,
    PickDisabledColor,
    Refreshed(ColorsState),
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

pub fn read_colors_config() -> ColorsState {
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
    
    ColorsState {
        low_color: bg,
        high_color: border,
        disabled_color: disabled,
        color_selectors: vec![
            ColorSelector::new(bg).with_label("Low Color"),
            ColorSelector::new(border).with_label("High Color"),
            ColorSelector::new(disabled).with_label("Disabled"),
        ],
        preset_colors: preset_colors(),
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
    write_config_value_path(CONFIG_PATH, key, value)
}

fn write_config_value_path(path: &str, key: &str, value: &str) -> bool {
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
    if let Ok(mut stream) = std::os::unix::net::UnixStream::connect(CLEARWM_SOCK) {
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

pub fn view(state: &mut ColorsState, cx: f32, cy: f32, cw: f32, _ch: f32) -> PageContent {
    let mut pc = PageContent::new();
    let mut y = cy + 12.0;

    // 1. Layout Section
    let mut sec = Section::new(&mut pc, cx, y, cw, "Layout");
    sec.spacing(8.0);
    state.color_selectors[0].color = state.low_color;
    sec.widget(&mut pc, &mut state.color_selectors[0], 12.0, 220.0, 22.0);
    sec.spacing(8.0);
    state.color_selectors[1].color = state.high_color;
    sec.widget(&mut pc, &mut state.color_selectors[1], 12.0, 220.0, 22.0);
    sec.spacing(8.0);
    y = sec.finish(&mut pc);

    // 2. Status Section
    let mut sec = Section::new(&mut pc, cx, y, cw, "Status");
    sec.spacing(8.0);
    state.color_selectors[2].color = state.disabled_color;
    sec.widget(&mut pc, &mut state.color_selectors[2], 12.0, 220.0, 22.0);
    sec.spacing(8.0);
    y = sec.finish(&mut pc);

    // 3. Preset Background Colors Grid
    let mut sec = Section::new(&mut pc, cx, y, cw, "Preset Backgrounds");
    let cols = 4;
    let gap = 8.0;
    let btn_w = (cw - 24.0 - (gap * (cols - 1) as f32)) / cols as f32;
    let btn_h = 28.0;

    for (i, (name, rgb)) in state.preset_colors.iter().enumerate() {
        let col = i % cols;
        let row = i / cols;
        let bx = cx + 12.0 + col as f32 * (btn_w + gap);
        let by = sec.ay() + row as f32 * (btn_h + gap);

        let r = rgb[0] as f32 / 255.0;
        let g = rgb[1] as f32 / 255.0;
        let b = rgb[2] as f32 / 255.0;
        let luminance = 0.299 * r + 0.587 * g + 0.114 * b;
        let text_color = if luminance > 0.5 { [0.08, 0.08, 0.12, 1.0] } else { [0.90, 0.90, 0.95, 1.0] };

        pc.button(
            name,
            bx,
            by,
            btn_w,
            btn_h,
            [r, g, b, 0.8],
            [r, g, b, 1.0],
            text_color,
            crate::app::AppAction::Colors(ColorsMessage::SetLowColor(*rgb)),
        );
    }

    let rows = (state.preset_colors.len() + cols - 1) / cols;
    sec.content_y += rows as f32 * (btn_h + gap) + 4.0;
    sec.finish(&mut pc);

    pc
}

pub fn update(state: &mut ColorsState, msg: ColorsMessage) {
    match msg {
        ColorsMessage::SetLowColor(rgb) => {
            state.low_color = rgb;
            apply_background(rgb);
        }
        ColorsMessage::SetHighColor(rgb) => {
            state.high_color = rgb;
            apply_border_color(rgb);
        }
        ColorsMessage::SetDisabledColor(rgb) => {
            state.disabled_color = rgb;
            apply_disabled_color(rgb);
        }
        ColorsMessage::PickLowColor | ColorsMessage::PickHighColor | ColorsMessage::PickDisabledColor => {}
        ColorsMessage::Refreshed(new) => {
            *state = new;
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
        let content = "\n[layout]\nlow_color = \"#112233\"\nhigh_color = \"#445566\"\ndisabled_color = \"#778899\"\n";
        assert_eq!(parse_color_from_key(content, "low_color", [0, 0, 0]), [17, 34, 51]);
        assert_eq!(parse_color_from_key(content, "high_color", [0, 0, 0]), [68, 85, 102]);
        assert_eq!(parse_color_from_key(content, "disabled_color", [0, 0, 0]), [119, 136, 153]);
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

        // Clean up
        let _ = fs::remove_file(path_str);
    }
}
