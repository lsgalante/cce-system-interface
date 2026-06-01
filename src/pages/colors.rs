use std::fs;
use std::io::Write;
use crate::app::PageContent;
use clear_ui::layout::Section;
use clear_ui::widget::ColorSelector;

const CONFIG_PATH: &str = "/home/lsgalante/.config/ccec/config.toml";

fn get_socket_path() -> String {
    match std::env::var("WAYLAND_DISPLAY") {
        Ok(display) => format!("/tmp/ccec-{}.sock", display),
        Err(_) => "/tmp/ccec.sock".to_string(),
    }
}

#[derive(Debug, Clone)]
pub struct ColorsState {
    pub low_color: [u8; 3],
    pub high_color: [u8; 3],
    pub disabled_color: [u8; 3],
    pub separator_color: [u8; 3],
    pub visual_guides_color: [u8; 3],
    pub slider_track_color: [u8; 3],
    pub page_low_color: [u8; 3],
    pub color_borders_color: [u8; 3],
    pub color_selectors: Vec<ColorSelector>,
}

impl Default for ColorsState {
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
            color_selectors: vec![
                ColorSelector::new([71, 71, 81]).with_label("Low Color"), // 0: Pages - Low Color
                ColorSelector::new([0x3e, 0x3e, 0x3e]).with_label("High Color"), // 1: Layout - High Color
                ColorSelector::new([0xff, 0x8c, 0x00]).with_label("Visual Guides"), // 2: Layout - Visual Guides
                ColorSelector::new([0x55, 0x55, 0x55]).with_label("Disabled"), // 3: Status - Disabled
                ColorSelector::new([124, 124, 137]).with_label("Separators"), // 4: Status - Separators
                ColorSelector::new([116, 116, 128]).with_label("Slider Track"), // 5: Controls - Slider Track
                ColorSelector::new([124, 124, 137]).with_label("Borders"), // 6: Controls - Borders
                ColorSelector::new([0x0a, 0x1a, 0x0e]).with_label("Low Color"), // 7: Layout - Low Color
            ],
        }
    }
}

#[derive(Debug, Clone)]
pub enum ColorsMessage {
    SetLowColor([u8; 3]),
    SetHighColor([u8; 3]),
    SetDisabledColor([u8; 3]),
    SetSeparatorColor([u8; 3]),
    SetVisualGuidesColor([u8; 3]),
    SetSliderTrackColor([u8; 3]),
    SetPageLowColor([u8; 3]),
    SetColorBordersColor([u8; 3]),
    PickLowColor,
    PickHighColor,
    PickDisabledColor,
    PickSeparatorColor,
    PickVisualGuides,
    PickSliderTrackColor,
    PickPageLowColor,
    PickColorBordersColor,
    Refreshed(ColorsState),
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
    
    let separator = parse_color_from_key(&content, "status_separator_color", [124, 124, 137]);
    
    let visual_guides = parse_color_from_key(&content, "visual_guides_color", [0xff, 0x8c, 0x00]);
    
    let slider_track = parse_color_from_key(&content, "slider_track_color", [116, 116, 128]);

    let page_low = parse_color_from_key(&content, "page_low_color", [71, 71, 81]);

    let color_borders = parse_color_from_key(&content, "color_borders_color", [124, 124, 137]);
    
    ColorsState {
        low_color: bg,
        high_color: border,
        disabled_color: disabled,
        separator_color: separator,
        visual_guides_color: visual_guides,
        slider_track_color: slider_track,
        page_low_color: page_low,
        color_borders_color: color_borders,
        color_selectors: vec![
            ColorSelector::new(page_low).with_label("Low Color"), // 0: Pages - Low Color
            ColorSelector::new(border).with_label("High Color"), // 1: Layout - High Color
            ColorSelector::new(visual_guides).with_label("Visual Guides"), // 2: Layout - Visual Guides
            ColorSelector::new(disabled).with_label("Disabled"), // 3: Status - Disabled
            ColorSelector::new(separator).with_label("Separators"), // 4: Status - Separators
            ColorSelector::new(slider_track).with_label("Slider Track"), // 5: Controls - Slider Track
            ColorSelector::new(color_borders).with_label("Borders"), // 6: Controls - Borders
            ColorSelector::new(bg).with_label("Low Color"), // 7: Layout - Low Color
        ],
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

pub fn view(state: &mut ColorsState, cx: f32, cy: f32, cw: f32, _ch: f32) -> PageContent {
    let mut pc = PageContent::new();
    let mut y = cy + 12.0;

    // 1. Pages Section
    let mut sec = Section::new(&mut pc, cx, y, cw, "Pages");
    sec.spacing(8.0);
    state.color_selectors[0].color = state.page_low_color;
    sec.widget(&mut pc, &mut state.color_selectors[0], 12.0, 220.0, 22.0);
    sec.spacing(8.0);
    y = sec.finish(&mut pc);

    // 2. Layout Section
    let mut sec = Section::new(&mut pc, cx, y, cw, "Layout");
    sec.spacing(8.0);
    state.color_selectors[7].color = state.low_color;
    sec.widget(&mut pc, &mut state.color_selectors[7], 12.0, 220.0, 22.0);
    sec.spacing(8.0);
    state.color_selectors[1].color = state.high_color;
    sec.widget(&mut pc, &mut state.color_selectors[1], 12.0, 220.0, 22.0);
    sec.spacing(8.0);
    state.color_selectors[2].color = state.visual_guides_color;
    sec.widget(&mut pc, &mut state.color_selectors[2], 12.0, 220.0, 22.0);
    sec.spacing(8.0);
    y = sec.finish(&mut pc);

    // 3. Status Section
    let mut sec = Section::new(&mut pc, cx, y, cw, "Status");
    sec.spacing(8.0);
    state.color_selectors[3].color = state.disabled_color;
    sec.widget(&mut pc, &mut state.color_selectors[3], 12.0, 220.0, 22.0);
    sec.spacing(8.0);
    state.color_selectors[4].color = state.separator_color;
    sec.widget(&mut pc, &mut state.color_selectors[4], 12.0, 220.0, 22.0);
    sec.spacing(8.0);
    y = sec.finish(&mut pc);

    // 4. Controls Section
    let mut sec = Section::new(&mut pc, cx, y, cw, "Controls");
    sec.spacing(8.0);
    state.color_selectors[5].color = state.slider_track_color;
    sec.widget(&mut pc, &mut state.color_selectors[5], 12.0, 220.0, 22.0);
    sec.spacing(8.0);
    state.color_selectors[6].color = state.color_borders_color;
    sec.widget(&mut pc, &mut state.color_selectors[6], 12.0, 220.0, 22.0);
    sec.spacing(8.0);
    sec.finish(&mut pc);

    pc
}

pub fn update(state: &mut ColorsState, msg: ColorsMessage) {
    match msg {
        ColorsMessage::SetLowColor(rgb) => {
            state.low_color = rgb;
            apply_background(rgb);
        }
        ColorsMessage::SetPageLowColor(rgb) => {
            state.page_low_color = rgb;
            apply_page_low_color(rgb);
        }
        ColorsMessage::SetHighColor(rgb) => {
            state.high_color = rgb;
            apply_border_color(rgb);
        }
        ColorsMessage::SetDisabledColor(rgb) => {
            state.disabled_color = rgb;
            apply_disabled_color(rgb);
        }
        ColorsMessage::SetSeparatorColor(rgb) => {
            state.separator_color = rgb;
            apply_separator_color(rgb);
        }
        ColorsMessage::SetVisualGuidesColor(rgb) => {
            state.visual_guides_color = rgb;
            apply_visual_guides_color(rgb);
        }
        ColorsMessage::SetSliderTrackColor(rgb) => {
            state.slider_track_color = rgb;
            apply_slider_track_color(rgb);
        }
        ColorsMessage::SetColorBordersColor(rgb) => {
            state.color_borders_color = rgb;
            apply_color_borders_color(rgb);
        }
        ColorsMessage::PickLowColor | ColorsMessage::PickHighColor | ColorsMessage::PickDisabledColor | ColorsMessage::PickSeparatorColor | ColorsMessage::PickVisualGuides | ColorsMessage::PickSliderTrackColor | ColorsMessage::PickPageLowColor | ColorsMessage::PickColorBordersColor => {}
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
        let content = "\n[layout]\nlow_color = \"#112233\"\nhigh_color = \"#445566\"\ndisabled_color = \"#778899\"\nstatus_separator_color = \"#aabbcc\"\nvisual_guides_color = \"#ddeeff\"\nslider_track_color = \"#123456\"\npage_low_color = \"#474751\"\ncolor_borders_color = \"#abcdef\"\n";
        assert_eq!(parse_color_from_key(content, "low_color", [0, 0, 0]), [17, 34, 51]);
        assert_eq!(parse_color_from_key(content, "high_color", [0, 0, 0]), [68, 85, 102]);
        assert_eq!(parse_color_from_key(content, "disabled_color", [0, 0, 0]), [119, 136, 153]);
        assert_eq!(parse_color_from_key(content, "status_separator_color", [0, 0, 0]), [170, 187, 204]);
        assert_eq!(parse_color_from_key(content, "visual_guides_color", [0, 0, 0]), [221, 238, 255]);
        assert_eq!(parse_color_from_key(content, "slider_track_color", [0, 0, 0]), [18, 52, 86]);
        assert_eq!(parse_color_from_key(content, "page_low_color", [0, 0, 0]), [71, 71, 81]);
        assert_eq!(parse_color_from_key(content, "color_borders_color", [0, 0, 0]), [171, 205, 239]);
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
}
