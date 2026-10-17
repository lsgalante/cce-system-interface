//! Appearance: the desktop-wide color scheme. Edits `style { color_scheme }`
//! in the SHARED config.kdl, which cce-desktop-portal serves as the Settings
//! portal's `org.freedesktop.appearance` `color-scheme` — what Chromium,
//! Firefox, Electron and libadwaita apps report to pages and use for their
//! own chrome, and what cce-browser follows on its "Follow System" scheme.
//! The portal watches the file and signals the change, so apps listening
//! for it switch without a restart.
//!
//! And the DE's animation speed, `style { animation_speed }`: the pace of
//! whatever animates for as long as it runs (the spinners and indeterminate
//! progress bars), read by every cce-ui client through `motion::speed`,
//! which re-reads it within half a second, so running apps follow a pick
//! without a restart. Picked from presets; the config holds the number, so
//! a hand-written value between them shows as the nearest.

use std::fs;

use cce_ui::compose::{PageFlow, PageLayoutBuilder};
use cce_ui::context::UiContext;
use cce_ui::widget::input::Dropdown;
use cce_ui::widget::Handle;

use crate::app::{AppAction, PageCanvas};
use crate::pages::AppPage;

/// config key, menu label. "No Preference" is the portal's 0: apps fall back
/// to their own default, which for web pages is their light styles.
pub const COLOR_SCHEMES: [(&str, &str); 3] = [("dark", "Dark"), ("light", "Light"), ("default", "No Preference")];

/// What cce-desktop-portal publishes for an absent key: cce is a dark desktop.
const DEFAULT_COLOR_SCHEME: &str = "dark";

/// Animation speeds offered: the multiplier written, menu label.
pub const SPEEDS: [(f32, &str); 5] = [(0.5, "Slower"), (0.75, "Slow"), (1.0, "Normal"), (1.5, "Fast"), (2.0, "Faster")];

const TEXT_DIM: [f32; 4] = [0.53, 0.53, 0.60, 1.0];

#[derive(Debug, Clone)]
pub struct AppearanceConfig {
    /// A `COLOR_SCHEMES` key, normalized the way the portal reads it.
    pub color_scheme: String,
    /// `style.animation_speed`, clamped as `motion::speed` reads it; 1 when unset.
    pub speed: f32,
}

pub struct AppearanceState {
    pub color_scheme: String,
    pub color_scheme_menu: Handle<cce_ui::widget::Adapted<Dropdown>>,
    pub speed: f32,
    pub speed_menu: Handle<cce_ui::widget::Adapted<Dropdown>>,
}

impl AppearanceState {
    /// The page's state, its dropdown inserted into `ctx` and set from the config.
    pub fn new(ctx: &mut UiContext) -> Self {
        let AppearanceConfig { color_scheme, speed } = read_appearance_config();
        Self {
            speed_menu: ctx.insert(
                Dropdown::new(SPEEDS.iter().map(|(_, label)| label.to_string()).collect(), speed_index(speed))
                    .with_label("Animation Speed"),
            ),
            speed,
            color_scheme_menu: ctx.insert(
                Dropdown::new(
                    COLOR_SCHEMES.iter().map(|(_, label)| label.to_string()).collect(),
                    color_scheme_index(&color_scheme),
                )
                .with_label("Color Scheme"),
            ),
            color_scheme,
        }
    }
}

/// The preset nearest `speed`, by ratio: 0.65 is nearer Slow (0.75) than Slower (0.5).
fn speed_index(speed: f32) -> usize {
    let off = |preset: f32| (speed / preset).ln().abs();
    (0..SPEEDS.len()).min_by(|&a, &b| off(SPEEDS[a].0).total_cmp(&off(SPEEDS[b].0))).unwrap_or(2)
}

fn color_scheme_index(key: &str) -> usize {
    COLOR_SCHEMES.iter().position(|(k, _)| *k == key).unwrap_or(0)
}

/// The portal's reading of a `color_scheme` value (cce-desktop-portal's
/// `parse_config`), so the menu shows what apps are actually told.
fn normalize(value: Option<&str>) -> &'static str {
    match value {
        Some("light" | "prefer-light") => "light",
        Some("default" | "none") => "default",
        _ => DEFAULT_COLOR_SCHEME,
    }
}

#[derive(Debug, Clone)]
pub enum AppearanceMessage {
    SetColorScheme(String),
    SetSpeed(f32),
    Refreshed(AppearanceConfig),
}

pub fn update(state: &mut AppearanceState, msg: AppearanceMessage, _ctx: &mut UiContext) {
    match msg {
        AppearanceMessage::SetColorScheme(key) => {
            if write_style_value("style.color_scheme", &key, "color scheme") {
                state.color_scheme = key;
            }
        }
        AppearanceMessage::SetSpeed(speed) => {
            // `{:?}`: `2.0`, never `2`, so the config holds a float.
            if write_style_value(cce_ui::motion::SPEED_KEY, &format!("{speed:?}"), "animation speed") {
                state.speed = speed;
            }
        }
        AppearanceMessage::Refreshed(new) => {
            state.color_scheme = new.color_scheme;
            state.speed = new.speed;
        }
    }
}

pub fn read_appearance_config() -> AppearanceConfig {
    let content = fs::read_to_string(cce_ui::config::get_config_path()).unwrap_or_default();
    AppearanceConfig { color_scheme: parse_color_scheme(&content).to_string(), speed: parse_speed(&content) }
}

/// `style { animation_speed }` as `motion::speed` reads it: clamped, and 1 when absent or
/// not a number.
fn parse_speed(content: &str) -> f32 {
    let doc = content.parse::<kdl::KdlDocument>().ok();
    let value = doc
        .as_ref()
        .and_then(|d| d.get("style"))
        .and_then(|n| n.children())
        .and_then(|style| style.get_arg("animation_speed"))
        .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64)));
    value.map_or(1.0, cce_ui::motion::clamp_speed)
}

fn parse_color_scheme(content: &str) -> &'static str {
    let doc = content.parse::<kdl::KdlDocument>().ok();
    let value = doc
        .as_ref()
        .and_then(|d| d.get("style"))
        .and_then(|n| n.children())
        .and_then(|style| style.get_arg("color_scheme"))
        .and_then(|v| v.as_string());
    normalize(value)
}

/// Write one `style { }` key (`style.color_scheme`, `style.animation_speed`). The shared
/// config holds the whole desktop's style, and `write_config_value` starts from an EMPTY
/// document when the file does not parse — so an unparseable file is left alone and the
/// pick dropped, never replaced by a config holding only this key.
fn write_style_value(key: &str, value: &str, what: &str) -> bool {
    let path = cce_ui::config::get_config_path();
    let content = fs::read_to_string(&path).unwrap_or_default();
    if content.parse::<kdl::KdlDocument>().is_err() {
        log::error!("[appearance] config.kdl did not parse; {what} pick dropped");
        return false;
    }
    let ok = cce_ui::config::write_config_value(&path.to_string_lossy(), key, value, "style");
    if !ok {
        log::error!("[appearance] writing {key} failed");
    }
    ok
}

impl AppPage for AppearanceState {
    // Sections: [Appearance]
    fn section_widgets(&mut self) -> Vec<Vec<cce_ui::widget::WidgetId>> {
        vec![vec![self.color_scheme_menu.id(), self.speed_menu.id()]]
    }

    fn view(
        &mut self,
        cx: f32,
        cy: f32,
        cw: f32,
        ch: f32,
        _root_focused: bool,
        sec_focused: &[bool],
        layout: &mut PageFlow,
        ctx: &mut cce_ui::context::UiContext,
    ) -> PageCanvas {
        let mut final_pc = PageCanvas::new();
        let sec_w = 320.0f32;
        let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(1);

        builder.add_section(&mut final_pc, "Appearance", sec_focused.first().copied().unwrap_or(false), |sec| {
            ctx[self.color_scheme_menu].selected = color_scheme_index(&self.color_scheme);
            ctx[self.speed_menu].selected = speed_index(self.speed);
            let mut form = sec.form();
            form.column()
                .widget_h(ctx, self.color_scheme_menu, cce_ui::layout::dropdown_height())
                .text("What apps and web pages are told to prefer.", crate::app::text_body(), TEXT_DIM)
                .widget_h(ctx, self.speed_menu, cce_ui::layout::dropdown_height())
                .text("The pace of what animates while it runs: spinners and busy bars.", crate::app::text_body(), TEXT_DIM);
            sec.place(form, ctx);
        });

        final_pc
    }

    fn propagate_widget_changes(
        &mut self,
        events: &[(cce_ui::widget::WidgetId, cce_ui::widget::WidgetEvent)],
        actions: &mut Vec<AppAction>,
        ctx: &mut UiContext,
    ) {
        for &(id, ref ev) in events {
            if *ev == cce_ui::widget::WidgetEvent::Changed && id == self.color_scheme_menu.id() {
                let key = COLOR_SCHEMES
                    .get(ctx[self.color_scheme_menu].selected)
                    .map_or(DEFAULT_COLOR_SCHEME, |(k, _)| k)
                    .to_string();
                actions.push(AppAction::Appearance(AppearanceMessage::SetColorScheme(key)));
            } else if *ev == cce_ui::widget::WidgetEvent::Changed && id == self.speed_menu.id() {
                let speed = SPEEDS.get(ctx[self.speed_menu].selected).map_or(1.0, |(v, _)| *v);
                if speed_index(self.speed) != ctx[self.speed_menu].selected {
                    actions.push(AppAction::Appearance(AppearanceMessage::SetSpeed(speed)));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The menu reads the key the way cce-desktop-portal does: absent or
    /// unknown is dark, and the portal's aliases land on their rows.
    #[test]
    fn reads_the_scheme_as_the_portal_does() {
        let with = |v: &str| parse_color_scheme(&format!("style {{\n    color_scheme \"{v}\"\n}}\n"));
        assert_eq!(with("light"), "light");
        assert_eq!(with("prefer-light"), "light");
        assert_eq!(with("none"), "default");
        assert_eq!(with("dark"), "dark");
        assert_eq!(with("purple"), "dark");
        assert_eq!(parse_color_scheme("style {\n    list corner_radius=(i64)12\n}\n"), "dark");
        assert_eq!(parse_color_scheme(""), "dark");
        assert_eq!(parse_color_scheme("style { this is not { kdl"), "dark");
    }

    /// The speed reads as `motion::speed` does (clamped, 1 when absent or nonsense), and a
    /// value between presets shows as the nearest.
    #[test]
    fn reads_the_speed_as_the_toolkit_does() {
        let with = |v: &str| parse_speed(&format!("style {{\n    animation_speed {v}\n}}\n"));
        assert_eq!(with("0.5"), 0.5);
        assert_eq!(with("2"), 2.0);
        assert_eq!(with("100.0"), cce_ui::motion::MAX_SPEED);
        assert_eq!(with("\"fast\""), 1.0);
        assert_eq!(parse_speed(""), 1.0);
        assert_eq!(SPEEDS[speed_index(1.0)].1, "Normal");
        assert_eq!(SPEEDS[speed_index(0.65)].1, "Slow");
        assert_eq!(SPEEDS[speed_index(4.0)].1, "Faster");
        assert_eq!(SPEEDS[speed_index(0.25)].1, "Slower");
    }

    /// Every preset is written as a float the reader takes back unchanged, beside the scheme.
    #[test]
    fn every_speed_preset_round_trips_through_the_config() {
        for (speed, label) in SPEEDS {
            let mut doc: kdl::KdlDocument = "style {\n    color_scheme \"light\"\n}\n".parse().unwrap();
            assert!(cce_ui::config::update_kdl_in_memory(&mut doc, cce_ui::motion::SPEED_KEY, &format!("{speed:?}"), "style"));
            let text = doc.to_string();
            assert_eq!(parse_speed(&text), speed, "{label}: {text}");
            assert_eq!(parse_color_scheme(&text), "light");
        }
    }

    /// The written key is a child node of `style`, the shape the portal
    /// reads (`style.get_arg("color_scheme")`), and the rest of the block
    /// survives the edit.
    #[test]
    fn the_written_key_is_the_one_the_portal_reads() {
        let mut doc: kdl::KdlDocument =
            "style {\n    list corner_radius=(i64)12\n    highlight primary=(rgb)\"#7dffff\"\n}\n".parse().unwrap();
        assert!(cce_ui::config::update_kdl_in_memory(&mut doc, "style.color_scheme", "light", "style"));
        let text = doc.to_string();
        assert_eq!(parse_color_scheme(&text), "light");
        assert!(text.contains("corner_radius=(i64)12") && text.contains("(rgb)\"#7dffff\""), "{text}");
    }
}
