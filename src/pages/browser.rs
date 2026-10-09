//! Browser (cce-browser) settings: homepage, search engine, download
//! directory, history recording, Raindrop bookmark sync, vi mode, navigation-bar
//! position, page color scheme. Edits the
//! browser's own app config (`~/.config/cce/cce-browser/config.kdl`,
//! section "browser") — the browser reloads it when its window regains
//! focus.

use std::fs;

use cce_ui::context::UiContext;
use cce_ui::widget::Handle;
use cce_ui::layout::{PageFlow, PageLayoutBuilder};
use cce_ui::widget::input::{Dropdown, Toggle};
use cce_ui::widget::TextBox;

use crate::app::{AppAction, PageContent};
use crate::pages::AppPage;

/// config key, menu label — the browser maps the key onto a query URL.
pub const SEARCH_ENGINES: [(&str, &str); 4] = [
    ("duckduckgo", "DuckDuckGo"),
    ("google", "Google"),
    ("bing", "Bing"),
    ("wikipedia", "Wikipedia"),
];

/// config key, menu label — the window edge the browser's floating
/// navigation bar is anchored to.
pub const BAR_POSITIONS: [(&str, &str); 2] = [("top", "Top"), ("bottom", "Bottom")];

/// config key, menu label — the first two are reported to pages as
/// `prefers-color-scheme`, so sites that ship a dark stylesheet use it.
/// "Force Dark" additionally inverts the page, for sites that ship no dark
/// theme; it fights their palette, so it is a deliberate last resort.
pub const COLOR_SCHEMES: [(&str, &str); 3] =
    [("dark", "Dark"), ("light", "Light"), ("force-dark", "Force Dark")];

const DEFAULT_HOMEPAGE: &str = "https://servo.org";

#[derive(Debug, Clone)]
pub struct BrowserConfig {
    pub homepage: String,
    pub search: String,
    pub download_dir: String,
    pub history: bool,
    /// `browser.raindrop`: sync bookmarks with Raindrop.io's Unsorted
    /// collection (cce-browser's RAINDROP-SYNC.md). Off unless set.
    pub raindrop: bool,
    /// `browser.vi-mode`: qutebrowser-style modal keys (cce-browser's
    /// `src/vi.rs`). Off unless set.
    pub vi_mode: bool,
    pub bar_position: String,
    pub color_scheme: String,
}

pub struct BrowserState {
    pub loaded: bool,
    pub homepage: String,
    pub search: String,
    pub download_dir: String,
    pub history: bool,
    pub raindrop: bool,
    pub vi_mode: bool,
    pub bar_position: String,
    pub color_scheme: String,
    pub homepage_box: Handle<cce_ui::widget::Adapted<TextBox>>,
    pub search_menu: Handle<cce_ui::widget::Adapted<Dropdown>>,
    pub bar_position_menu: Handle<cce_ui::widget::Adapted<Dropdown>>,
    pub color_scheme_menu: Handle<cce_ui::widget::Adapted<Dropdown>>,
    pub download_dir_box: Handle<cce_ui::widget::Adapted<TextBox>>,
    pub history_toggle: Handle<cce_ui::widget::Adapted<Toggle>>,
    pub raindrop_toggle: Handle<cce_ui::widget::Adapted<Toggle>>,
    pub vi_mode_toggle: Handle<cce_ui::widget::Adapted<Toggle>>,
}

impl Default for BrowserState {
    /// The data half, read from the config; the widgets are `new`'s.
    fn default() -> Self {
        let config = read_browser_config();
        Self {
            loaded: true,
            homepage: config.homepage,
            search: config.search.clone(),
            download_dir: config.download_dir,
            history: config.history,
            raindrop: config.raindrop,
            vi_mode: config.vi_mode,
            bar_position: config.bar_position.clone(),
            color_scheme: config.color_scheme.clone(),
            homepage_box: Handle::none(),
            search_menu: Handle::none(),
            bar_position_menu: Handle::none(),
            color_scheme_menu: Handle::none(),
            download_dir_box: Handle::none(),
            history_toggle: Handle::none(),
            raindrop_toggle: Handle::none(),
            vi_mode_toggle: Handle::none(),
        }
    }
}

impl BrowserState {
    /// The page's state, its widgets inserted into `ctx` and filled from the config.
    pub fn new(ctx: &mut UiContext) -> Self {
        let s = Self::default();
        let mut homepage_box = TextBox::new(s.homepage.clone())
            .with_draw_bg_border(true)
            .with_label("Homepage");
        homepage_box.edit_buffer = s.homepage.clone();
        let mut download_dir_box = TextBox::new(s.download_dir.clone())
            .with_draw_bg_border(true)
            .with_label("Download Directory");
        download_dir_box.edit_buffer = s.download_dir.clone();
        Self {
            homepage_box: ctx.insert(homepage_box),
            search_menu: ctx.insert(Dropdown::new(
                SEARCH_ENGINES.iter().map(|(_, label)| label.to_string()).collect(),
                search_index(&s.search),
            )
            .with_label("Search Engine")),
            bar_position_menu: ctx.insert(Dropdown::new(
                BAR_POSITIONS.iter().map(|(_, label)| label.to_string()).collect(),
                bar_position_index(&s.bar_position),
            )
            .with_label("Navigation Bar Position")),
            color_scheme_menu: ctx.insert(Dropdown::new(
                COLOR_SCHEMES.iter().map(|(_, label)| label.to_string()).collect(),
                color_scheme_index(&s.color_scheme),
            )
            .with_label("Page Color Scheme")),
            download_dir_box: ctx.insert(download_dir_box),
            // Left-aligned, as the designer's parameter pane sets its toggles:
            // a toggle's run is half its width, so the seam where run meets
            // well sits at the midpoint, and a centred label on a row-wide
            // toggle had it drawn straight through the text.
            history_toggle: ctx.insert(Toggle::new().with_label("Record History").with_left_align(true)),
            raindrop_toggle: ctx.insert(Toggle::new().with_label("Sync Bookmarks with Raindrop").with_left_align(true)),
            vi_mode_toggle: ctx.insert(Toggle::new().with_label("Vi Keys (qutebrowser-style)").with_left_align(true)),
            ..s
        }
    }
}

fn search_index(key: &str) -> usize {
    SEARCH_ENGINES.iter().position(|(k, _)| *k == key).unwrap_or(0)
}

fn bar_position_index(key: &str) -> usize {
    BAR_POSITIONS.iter().position(|(k, _)| *k == key).unwrap_or(0)
}

fn color_scheme_index(key: &str) -> usize {
    COLOR_SCHEMES.iter().position(|(k, _)| *k == key).unwrap_or(0)
}

#[derive(Debug, Clone)]
pub enum BrowserMessage {
    SetSearch(String),
    SetBarPosition(String),
    SetColorScheme(String),
    ToggleHistory,
    ToggleRaindrop,
    ToggleViMode,
    /// Commit the homepage / download-dir text fields.
    Apply,
    Refreshed(BrowserConfig),
}

/// A TextBox's live contents: the in-progress edit buffer while focused,
/// the committed text otherwise (the recurring TextBox landmine).
fn live_text(tb: &cce_ui::widget::Adapted<TextBox>) -> String {
    if tb.editing {
        tb.edit_buffer.trim().to_string()
    } else {
        tb.text.trim().to_string()
    }
}

pub fn update(state: &mut BrowserState, msg: BrowserMessage, ctx: &mut UiContext) {
    match msg {
        BrowserMessage::SetSearch(key) => {
            state.search = key.clone();
            write_config_value("search", &key);
        }
        BrowserMessage::SetBarPosition(key) => {
            state.bar_position = key.clone();
            write_config_value("bar-position", &key);
        }
        BrowserMessage::SetColorScheme(key) => {
            state.color_scheme = key.clone();
            write_config_value("color-scheme", &key);
        }
        BrowserMessage::ToggleHistory => {
            state.history = !state.history;
            write_config_value("history", &state.history.to_string());
        }
        BrowserMessage::ToggleRaindrop => {
            state.raindrop = !state.raindrop;
            write_config_value("raindrop", &state.raindrop.to_string());
        }
        BrowserMessage::ToggleViMode => {
            state.vi_mode = !state.vi_mode;
            write_config_value("vi-mode", &state.vi_mode.to_string());
        }
        BrowserMessage::Apply => {
            state.homepage = live_text(&ctx[state.homepage_box]);
            if state.homepage.is_empty() {
                state.homepage = DEFAULT_HOMEPAGE.to_string();
                ctx[state.homepage_box].text = state.homepage.clone();
                ctx[state.homepage_box].edit_buffer = state.homepage.clone();
            }
            state.download_dir = live_text(&ctx[state.download_dir_box]);
            write_config_value("homepage", &state.homepage);
            write_config_value("download-dir", &state.download_dir);
        }
        BrowserMessage::Refreshed(new) => {
            state.loaded = true;
            state.search = new.search;
            state.history = new.history;
            state.raindrop = new.raindrop;
            state.vi_mode = new.vi_mode;
            state.bar_position = new.bar_position;
            state.color_scheme = new.color_scheme;
            // Don't clobber fields mid-edit with watcher refreshes.
            if !ctx[state.homepage_box].editing && state.homepage != new.homepage {
                state.homepage = new.homepage.clone();
                ctx[state.homepage_box].text = new.homepage.clone();
                ctx[state.homepage_box].edit_buffer = new.homepage;
            }
            if !ctx[state.download_dir_box].editing && state.download_dir != new.download_dir {
                state.download_dir = new.download_dir.clone();
                ctx[state.download_dir_box].text = new.download_dir.clone();
                ctx[state.download_dir_box].edit_buffer = new.download_dir;
            }
        }
    }
}

fn get_config_path() -> String {
    cce_ui::config::get_app_config_path("cce-browser")
        .to_string_lossy()
        .into_owned()
}

pub fn read_browser_config() -> BrowserConfig {
    let content = fs::read_to_string(get_config_path()).unwrap_or_default();
    let val = cce_ui::config::parse_kdl_to_json(&content);
    BrowserConfig {
        homepage: val["browser"]["homepage"]
            .as_str()
            .unwrap_or(DEFAULT_HOMEPAGE)
            .to_string(),
        search: val["browser"]["search"].as_str().unwrap_or("duckduckgo").to_string(),
        download_dir: val["browser"]["download-dir"].as_str().unwrap_or("").to_string(),
        history: val["browser"]["history"].as_bool().unwrap_or(true),
        raindrop: val["browser"]["raindrop"].as_bool().unwrap_or(false),
        vi_mode: val["browser"]["vi-mode"].as_bool().unwrap_or(false),
        bar_position: val["browser"]["bar-position"].as_str().unwrap_or("top").to_string(),
        color_scheme: val["browser"]["color-scheme"].as_str().unwrap_or("dark").to_string(),
    }
}

fn write_config_value(key: &str, value: &str) {
    let path = get_config_path();
    // The per-app config dir may not exist yet.
    if let Some(dir) = std::path::Path::new(&path).parent() {
        let _ = fs::create_dir_all(dir);
    }
    // Section nesting comes from the dotted key path (the section arg of
    // write_config_value is vestigial).
    cce_ui::config::write_config_value(&path, &format!("browser.{key}"), value, "browser");
}

impl AppPage for BrowserState {
    // Sections: [Browser Settings]
    fn section_widgets(&mut self) -> Vec<Vec<cce_ui::widget::WidgetId>> {
        vec![vec![
            self.homepage_box.id(),
            self.search_menu.id(),
            self.bar_position_menu.id(),
            self.color_scheme_menu.id(),
            self.download_dir_box.id(),
            self.history_toggle.id(),
            self.raindrop_toggle.id(),
            self.vi_mode_toggle.id(),
        ]]
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
    ) -> PageContent {
        let mut final_pc = PageContent::new();
        let sec_w = 320.0f32;
        let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(1);

        builder.add_section(&mut final_pc, "Browser Settings", sec_focused.first().copied().unwrap_or(false), |sec| {
            ctx[self.search_menu].selected = search_index(&self.search);
            ctx[self.bar_position_menu].selected = bar_position_index(&self.bar_position);
            ctx[self.color_scheme_menu].selected = color_scheme_index(&self.color_scheme);
            ctx[self.history_toggle].set_toggled(self.history);
            // Needs a Raindrop token in the keyring (service=raindrop.io);
            // the browser shows the sync's status on cce://bookmarks.
            ctx[self.raindrop_toggle].set_toggled(self.raindrop);
            ctx[self.vi_mode_toggle].set_toggled(self.vi_mode);

            let (field_h, menu_h, toggle_h) = (
                cce_ui::layout::textbox_height(),
                cce_ui::layout::dropdown_height(),
                cce_ui::layout::toggle_height(),
            );
            let mut form = sec.form();
            form.column()
                .widget_h(ctx, self.homepage_box, field_h)
                .widget_h(ctx, self.search_menu, menu_h)
                .widget_h(ctx, self.bar_position_menu, menu_h)
                .widget_h(ctx, self.color_scheme_menu, menu_h)
                .widget_h(ctx, self.download_dir_box, field_h)
                .widget_h(ctx, self.history_toggle, toggle_h)
                .widget_h(ctx, self.raindrop_toggle, toggle_h)
                .widget_h(ctx, self.vi_mode_toggle, toggle_h)
                .draw(0.0, cce_ui::layout::button_height(), false, |pc, r, _| {
                    pc.button(
                        "Apply",
                        r.x,
                        r.y,
                        r.width,
                        r.height,
                        [0.20, 0.40, 0.65, 1.0],
                        [0.28, 0.50, 0.78, 1.0],
                        [1.0, 1.0, 1.0, 1.0],
                        AppAction::Browser(BrowserMessage::Apply),
                    );
                });
            sec.place(form, ctx);
        });

        final_pc
    }

    fn propagate_widget_changes(&mut self, actions: &mut Vec<AppAction>, ctx: &mut UiContext) {
        if ctx[self.search_menu].take_change() {
            let key = SEARCH_ENGINES
                .get(ctx[self.search_menu].selected)
                .map(|(k, _)| k.to_string())
                .unwrap_or_else(|| "duckduckgo".to_string());
            actions.push(AppAction::Browser(BrowserMessage::SetSearch(key)));
        }
        if ctx[self.bar_position_menu].take_change() {
            let key = BAR_POSITIONS
                .get(ctx[self.bar_position_menu].selected)
                .map(|(k, _)| k.to_string())
                .unwrap_or_else(|| "top".to_string());
            actions.push(AppAction::Browser(BrowserMessage::SetBarPosition(key)));
        }
        if ctx[self.color_scheme_menu].take_change() {
            let key = COLOR_SCHEMES
                .get(ctx[self.color_scheme_menu].selected)
                .map(|(k, _)| k.to_string())
                .unwrap_or_else(|| "dark".to_string());
            actions.push(AppAction::Browser(BrowserMessage::SetColorScheme(key)));
        }
        if ctx[self.history_toggle].take_change() {
            actions.push(AppAction::Browser(BrowserMessage::ToggleHistory));
        }
        if ctx[self.raindrop_toggle].take_change() {
            actions.push(AppAction::Browser(BrowserMessage::ToggleRaindrop));
        }
        if ctx[self.vi_mode_toggle].take_change() {
            actions.push(AppAction::Browser(BrowserMessage::ToggleViMode));
        }
        // Enter in either text field commits both.
        if ctx[self.homepage_box].take_change() || ctx[self.download_dir_box].take_change() {
            actions.push(AppAction::Browser(BrowserMessage::Apply));
        }
    }
}
