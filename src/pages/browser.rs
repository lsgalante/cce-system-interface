//! Browser (cce-browser) settings: homepage, search engine, download
//! directory, history recording. Edits the browser's own app config
//! (`~/.config/cce/cce-browser/config.kdl`, section "browser") — the
//! browser reloads it when its window regains focus.

use std::fs;

use cce_ui::layout::{LayoutStrategy, PageLayoutBuilder};
use cce_ui::widget::input::{Dropdown, Toggle};
use cce_ui::widget::{TextBox, WidgetHost};

use crate::app::{AppAction, PageContent, SectionContextExt};
use crate::pages::AppPage;

/// config key, menu label — the browser maps the key onto a query URL.
pub const SEARCH_ENGINES: [(&str, &str); 4] = [
    ("duckduckgo", "DuckDuckGo"),
    ("google", "Google"),
    ("bing", "Bing"),
    ("wikipedia", "Wikipedia"),
];

const DEFAULT_HOMEPAGE: &str = "https://servo.org";

#[derive(Debug, Clone)]
pub struct BrowserConfig {
    pub homepage: String,
    pub search: String,
    pub download_dir: String,
    pub history: bool,
}

pub struct BrowserState {
    pub loaded: bool,
    pub homepage: String,
    pub search: String,
    pub download_dir: String,
    pub history: bool,
    pub homepage_box: cce_ui::widget::Adapted<TextBox>,
    pub search_menu: cce_ui::widget::Adapted<Dropdown>,
    pub download_dir_box: cce_ui::widget::Adapted<TextBox>,
    pub history_toggle: cce_ui::widget::Adapted<Toggle>,
}

impl Default for BrowserState {
    fn default() -> Self {
        let config = read_browser_config();
        let mut homepage_box = TextBox::new(config.homepage.clone())
            .with_draw_bg_border(true)
            .with_label("Homepage");
        homepage_box.edit_buffer = config.homepage.clone();
        let mut download_dir_box = TextBox::new(config.download_dir.clone())
            .with_draw_bg_border(true)
            .with_label("Download Directory");
        download_dir_box.edit_buffer = config.download_dir.clone();
        Self {
            loaded: true,
            homepage: config.homepage,
            search: config.search.clone(),
            download_dir: config.download_dir,
            history: config.history,
            homepage_box,
            search_menu: Dropdown::new(
                SEARCH_ENGINES.iter().map(|(_, label)| label.to_string()).collect(),
                search_index(&config.search),
            )
            .with_label("Search Engine"),
            download_dir_box,
            history_toggle: Toggle::new().with_label("Record History"),
        }
    }
}

fn search_index(key: &str) -> usize {
    SEARCH_ENGINES.iter().position(|(k, _)| *k == key).unwrap_or(0)
}

#[derive(Debug, Clone)]
pub enum BrowserMessage {
    SetSearch(String),
    ToggleHistory,
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

pub fn update(state: &mut BrowserState, msg: BrowserMessage) {
    match msg {
        BrowserMessage::SetSearch(key) => {
            state.search = key.clone();
            write_config_value("search", &key);
        }
        BrowserMessage::ToggleHistory => {
            state.history = !state.history;
            write_config_value("history", &state.history.to_string());
        }
        BrowserMessage::Apply => {
            state.homepage = live_text(&state.homepage_box);
            if state.homepage.is_empty() {
                state.homepage = DEFAULT_HOMEPAGE.to_string();
                state.homepage_box.text = state.homepage.clone();
                state.homepage_box.edit_buffer = state.homepage.clone();
            }
            state.download_dir = live_text(&state.download_dir_box);
            write_config_value("homepage", &state.homepage);
            write_config_value("download-dir", &state.download_dir);
        }
        BrowserMessage::Refreshed(new) => {
            state.loaded = true;
            state.search = new.search;
            state.history = new.history;
            // Don't clobber fields mid-edit with watcher refreshes.
            if !state.homepage_box.editing && state.homepage != new.homepage {
                state.homepage = new.homepage.clone();
                state.homepage_box.text = new.homepage.clone();
                state.homepage_box.edit_buffer = new.homepage;
            }
            if !state.download_dir_box.editing && state.download_dir != new.download_dir {
                state.download_dir = new.download_dir.clone();
                state.download_dir_box.text = new.download_dir.clone();
                state.download_dir_box.edit_buffer = new.download_dir;
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
            self.download_dir_box.id(),
            self.history_toggle.id(),
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
        layout: &mut dyn LayoutStrategy,
        ctx: &mut cce_ui::context::UiContext,
    ) -> PageContent {
        let mut final_pc = PageContent::new();
        let sec_w = 320.0f32;
        let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(1);

        builder.add_section(&mut final_pc, "Browser Settings", sec_focused.first().copied().unwrap_or(false), |sec| {
            let mut stack = sec.vstack(8.0);
            let sec_w = stack.context.cw;
            let row_w = sec_w - 28.0;

            self.homepage_box.set_row_rect(stack.context.left + 14.0, row_w);
            stack.add_widget(&mut self.homepage_box, row_w, 44.0, ctx);

            self.search_menu.selected = search_index(&self.search);
            self.search_menu.set_row_rect(stack.context.left + 14.0, row_w);
            stack.add_widget(&mut self.search_menu, row_w, 44.0, ctx);

            self.download_dir_box.set_row_rect(stack.context.left + 14.0, row_w);
            stack.add_widget(&mut self.download_dir_box, row_w, 44.0, ctx);

            self.history_toggle.set_toggled(self.history);
            stack.add_widget(&mut self.history_toggle, row_w, cce_ui::layout::toggle_height(), ctx);

            let btn_h = 32.0;
            stack.add_row(1, 0.0, btn_h, |ctx, _, x, w| {
                ctx.button(
                    "Apply",
                    x,
                    ctx.ay(),
                    w,
                    btn_h,
                    [0.20, 0.40, 0.65, 1.0],
                    [0.28, 0.50, 0.78, 1.0],
                    [1.0, 1.0, 1.0, 1.0],
                    AppAction::Browser(BrowserMessage::Apply),
                );
            });
        });

        final_pc
    }

    fn propagate_widget_changes(&mut self, actions: &mut Vec<AppAction>) {
        if self.search_menu.take_change() {
            let key = SEARCH_ENGINES
                .get(self.search_menu.selected)
                .map(|(k, _)| k.to_string())
                .unwrap_or_else(|| "duckduckgo".to_string());
            actions.push(AppAction::Browser(BrowserMessage::SetSearch(key)));
        }
        if self.history_toggle.take_change() {
            actions.push(AppAction::Browser(BrowserMessage::ToggleHistory));
        }
        // Enter in either text field commits both.
        if self.homepage_box.take_change() || self.download_dir_box.take_change() {
            actions.push(AppAction::Browser(BrowserMessage::Apply));
        }
    }
}
