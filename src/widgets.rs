use clear_ui::color;

pub struct SectionStyle {
    pub label: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

pub fn section_rects(_label: &str, x: f32, y: f32, w: f32, h: f32) -> Vec<([f32; 4], f32, f32, f32, f32)> {
    let mut rects = Vec::new();
    // Section background
    rects.push((color::CONTENT_BG, x, y, w, h));
    // Section border top
    rects.push(([0.20, 0.33, 0.22, 1.0], x, y, w, 1.0));
    rects
}

pub fn label_color() -> [f32; 4] {
    color::TEXT_ACCENT
}

pub fn text_dim_color() -> [f32; 4] {
    color::TEXT_DIM
}

pub fn text_fg_color() -> [f32; 4] {
    color::TEXT_FG
}
