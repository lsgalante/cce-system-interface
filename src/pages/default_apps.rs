//! XDG default applications: curated categories over `~/.config/mimeapps.list`.
//! Candidates come from the installed `.desktop` entries that claim the
//! category's MIME types; picks are applied through `xdg-mime default`, one
//! call per type, so a browser pick covers http/https/text-html at once.

use crate::app::{AppAction, PageContent};
use cce_ui::layout::{PageLayoutBuilder, LayoutStrategy};
use cce_ui::widget::{Dropdown, WidgetHost};
use std::collections::HashMap;
use std::path::PathBuf;

/// (label, MIME types) — the first type is the one queried for the current
/// default; a pick sets every type in the list.
const CATEGORIES: &[(&str, &[&str])] = &[
    ("Web Browser", &["x-scheme-handler/http", "x-scheme-handler/https", "text/html"]),
    ("Mail", &["x-scheme-handler/mailto"]),
    ("File Manager", &["inode/directory"]),
    ("Text Editor", &["text/plain"]),
    ("Images", &["image/png", "image/jpeg", "image/gif", "image/webp", "image/svg+xml"]),
    ("Audio", &["audio/mpeg", "audio/flac", "audio/ogg", "audio/x-wav"]),
    ("Video", &["video/mp4", "video/x-matroska", "video/webm"]),
    ("PDF", &["application/pdf"]),
];

const NOT_SET: &str = "— not set —";

/// One category's fetched facts: (desktop id, display name) candidates and the
/// current default's desktop id. Plain data — the widget state lives in
/// [`DefaultAppsState`].
#[derive(Debug, Clone, PartialEq)]
pub struct CategoryInfo {
    pub candidates: Vec<(String, String)>,
    pub current: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DefaultAppsInfo(pub Vec<CategoryInfo>);

#[derive(Debug, Clone)]
pub struct CategoryEntry {
    pub label: &'static str,
    pub mimes: &'static [&'static str],
    pub info: CategoryInfo,
    /// Desktop id per dropdown option (None = the "not set" placeholder row).
    pub option_ids: Vec<Option<String>>,
    pub dropdown: cce_ui::widget::Adapted<Dropdown>,
}

#[derive(Debug, Clone)]
pub struct DefaultAppsState {
    pub loaded: bool,
    pub categories: Vec<CategoryEntry>,
}

impl Default for DefaultAppsState {
    fn default() -> Self {
        Self {
            loaded: false,
            categories: CATEGORIES
                .iter()
                .map(|&(label, mimes)| CategoryEntry {
                    label,
                    mimes,
                    info: CategoryInfo { candidates: Vec::new(), current: None },
                    option_ids: vec![None],
                    dropdown: Dropdown::new(vec![NOT_SET.to_string()], 0).with_label(label),
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum DefaultAppsMessage {
    Refreshed(DefaultAppsInfo),
    /// The user picked dropdown option `1` in category `0`.
    Set(usize, usize),
}

const TEXT_DIM: [f32; 4] = [0.53, 0.53, 0.60, 1.0];

fn rebuild_entry_options(entry: &mut CategoryEntry) {
    let mut options = Vec::new();
    let mut ids = Vec::new();
    let current_idx = entry
        .info
        .current
        .as_ref()
        .and_then(|cur| entry.info.candidates.iter().position(|(id, _)| id == cur));
    if entry.info.current.is_none() || current_idx.is_none() {
        options.push(NOT_SET.to_string());
        ids.push(None);
    }
    for (id, name) in &entry.info.candidates {
        options.push(name.clone());
        ids.push(Some(id.clone()));
    }
    let selected = entry
        .info
        .current
        .as_ref()
        .and_then(|cur| ids.iter().position(|i| i.as_deref() == Some(cur)))
        .unwrap_or(0);
    entry.dropdown.options = options;
    entry.dropdown.selected = selected;
    entry.option_ids = ids;
}

pub fn view(state: &mut DefaultAppsState, cx: f32, cy: f32, cw: f32, ch: f32, _root_focused: bool, sec_focused: &[bool], layout: &mut dyn LayoutStrategy, ctx: &mut cce_ui::context::UiContext) -> PageContent {
    let mut final_pc = PageContent::new();
    let sec_w = 320.0f32;
    let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(1);

    builder.add_section_spanned(&mut final_pc, "", 1, sec_focused.first().copied().unwrap_or(false), |sec| {
        let sec_w = sec.cw;
        if !state.loaded {
            sec.text("Scanning installed applications...", 12.0, 0.0, 12.0, TEXT_DIM);
        } else {
            let mut stack = sec.vstack(8.0);
            for entry in state.categories.iter_mut() {
                entry.dropdown.set_row_rect(stack.context.left + 14.0, sec_w - 28.0);
                stack.add_widget(&mut entry.dropdown, sec_w - 28.0, 44.0, ctx);
            }
        }
    });

    final_pc
}

pub fn update(state: &mut DefaultAppsState, msg: DefaultAppsMessage) {
    match msg {
        DefaultAppsMessage::Refreshed(info) => {
            state.loaded = true;
            for (entry, cat) in state.categories.iter_mut().zip(info.0.into_iter()) {
                // Leave an open dropdown alone — the next refresh normalizes it.
                if entry.info == cat || entry.dropdown.open {
                    continue;
                }
                entry.info = cat;
                rebuild_entry_options(entry);
            }
        }
        DefaultAppsMessage::Set(cat_idx, opt_idx) => {
            let Some(entry) = state.categories.get_mut(cat_idx) else { return };
            let Some(Some(id)) = entry.option_ids.get(opt_idx).cloned() else { return };
            for mime in entry.mimes {
                let _ = tokio::process::Command::new("xdg-mime")
                    .args(["default", &id, mime])
                    .spawn();
            }
            entry.info.current = Some(id);
            rebuild_entry_options(entry);
        }
    }
}

// ── Background fetching ──

struct DesktopApp {
    name: String,
    mimes: Vec<String>,
    no_display: bool,
}

fn desktop_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let data_home = std::env::var("XDG_DATA_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var("HOME").ok().map(|h| PathBuf::from(h).join(".local/share")));
    if let Some(h) = data_home {
        dirs.push(h.join("applications"));
    }
    let data_dirs = std::env::var("XDG_DATA_DIRS").ok().filter(|s| !s.is_empty())
        .unwrap_or_else(|| "/usr/local/share:/usr/share".to_string());
    for d in data_dirs.split(':').filter(|d| !d.is_empty()) {
        dirs.push(PathBuf::from(d).join("applications"));
    }
    dirs
}

/// Parse the `[Desktop Entry]` group of one `.desktop` file. Returns None for
/// non-applications and `Hidden=true` entries (spec: treated as nonexistent).
fn parse_desktop_file(path: &std::path::Path) -> Option<DesktopApp> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut in_entry = false;
    let mut name = None;
    let mut mimes = Vec::new();
    let mut app_type = None;
    let mut hidden = false;
    let mut no_display = false;
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_entry {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            match k.trim() {
                "Name" if name.is_none() => name = Some(v.trim().to_string()),
                "Type" => app_type = Some(v.trim().to_string()),
                "Hidden" => hidden = v.trim().eq_ignore_ascii_case("true"),
                "NoDisplay" => no_display = v.trim().eq_ignore_ascii_case("true"),
                "MimeType" => {
                    mimes = v.split(';').map(str::trim).filter(|m| !m.is_empty()).map(String::from).collect();
                }
                _ => {}
            }
        }
    }
    if hidden || app_type.as_deref() != Some("Application") {
        return None;
    }
    Some(DesktopApp { name: name?, mimes, no_display })
}

/// Scan every XDG applications dir with spec ID shadowing: the first dir that
/// defines an ID wins; subdirectories join the ID with `-`.
fn scan_desktop_entries() -> HashMap<String, DesktopApp> {
    fn walk(dir: &std::path::Path, prefix: &str, out: &mut HashMap<String, DesktopApp>) {
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        for e in rd.flatten() {
            let path = e.path();
            let Some(fname) = path.file_name().and_then(|n| n.to_str()) else { continue };
            if path.is_dir() {
                walk(&path, &format!("{prefix}{fname}-"), out);
            } else if fname.ends_with(".desktop") {
                let id = format!("{prefix}{fname}");
                if out.contains_key(&id) {
                    continue; // shadowed by an earlier data dir
                }
                if let Some(app) = parse_desktop_file(&path) {
                    out.insert(id, app);
                }
            }
        }
    }
    let mut out = HashMap::new();
    for dir in desktop_dirs() {
        walk(&dir, "", &mut out);
    }
    out
}

pub async fn fetch_default_apps() -> DefaultAppsInfo {
    let apps = tokio::task::spawn_blocking(scan_desktop_entries).await.unwrap_or_default();

    let mut cats = Vec::with_capacity(CATEGORIES.len());
    for &(_, mimes) in CATEGORIES {
        let current = tokio::process::Command::new("xdg-mime")
            .args(["query", "default", mimes[0]])
            .output()
            .await
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .filter(|s| !s.is_empty());

        let mut candidates: Vec<(String, String)> = apps
            .iter()
            .filter(|(id, app)| {
                let claims = app.mimes.iter().any(|m| mimes.contains(&m.as_str()));
                // NoDisplay apps stay hidden unless they ARE the default.
                claims && (!app.no_display || current.as_deref() == Some(id.as_str()))
            })
            .map(|(id, app)| (id.clone(), app.name.clone()))
            .collect();
        candidates.sort_by(|a, b| a.1.to_lowercase().cmp(&b.1.to_lowercase()));

        // A current default we didn't scan (odd install) still shows, by id.
        if let Some(cur) = &current {
            if !candidates.iter().any(|(id, _)| id == cur) {
                candidates.push((cur.clone(), cur.trim_end_matches(".desktop").to_string()));
            }
        }
        cats.push(CategoryInfo { candidates, current });
    }
    DefaultAppsInfo(cats)
}

impl crate::pages::AppPage for DefaultAppsState {
    // Sections: [Default Apps]
    fn section_widgets(&mut self) -> Vec<Vec<cce_ui::widget::WidgetId>> {
        vec![self.categories.iter().map(|c| c.dropdown.id()).collect()]
    }

    fn view(
        &mut self,
        cx: f32,
        cy: f32,
        cw: f32,
        ch: f32,
        root_focused: bool,
        sec_focused: &[bool],
        layout: &mut dyn LayoutStrategy,
        ctx: &mut cce_ui::context::UiContext,
    ) -> crate::app::PageContent {
        view(self, cx, cy, cw, ch, root_focused, sec_focused, layout, ctx)
    }

    fn propagate_widget_changes(&mut self, actions: &mut Vec<AppAction>) {
        for (i, entry) in self.categories.iter_mut().enumerate() {
            if entry.dropdown.take_change() {
                actions.push(AppAction::DefaultApps(DefaultAppsMessage::Set(i, entry.dropdown.selected)));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_desktop_file_basics() {
        let dir = std::env::temp_dir().join("cce-da-test");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("t.desktop");
        std::fs::write(&p, "[Desktop Entry]\nType=Application\nName=Test App\nMimeType=text/html;image/png;\n\n[Desktop Action new]\nName=Other\n").unwrap();
        let app = parse_desktop_file(&p).unwrap();
        assert_eq!(app.name, "Test App");
        assert_eq!(app.mimes, vec!["text/html", "image/png"]);
        assert!(!app.no_display);

        std::fs::write(&p, "[Desktop Entry]\nType=Application\nName=H\nHidden=true\n").unwrap();
        assert!(parse_desktop_file(&p).is_none());
        std::fs::write(&p, "[Desktop Entry]\nType=Link\nName=L\n").unwrap();
        assert!(parse_desktop_file(&p).is_none());
    }

    #[test]
    fn rebuild_options_maps_current() {
        let mut st = DefaultAppsState::default();
        let e = &mut st.categories[0];
        e.info = CategoryInfo {
            candidates: vec![
                ("a.desktop".into(), "Alpha".into()),
                ("b.desktop".into(), "Beta".into()),
            ],
            current: Some("b.desktop".into()),
        };
        rebuild_entry_options(e);
        assert_eq!(e.dropdown.options, vec!["Alpha".to_string(), "Beta".to_string()]);
        assert_eq!(e.dropdown.selected, 1);
        assert_eq!(e.option_ids[1].as_deref(), Some("b.desktop"));

        // No current: placeholder row leads and is selected.
        e.info.current = None;
        rebuild_entry_options(e);
        assert_eq!(e.dropdown.options[0], NOT_SET);
        assert_eq!(e.dropdown.selected, 0);
        assert!(e.option_ids[0].is_none());
    }
}
