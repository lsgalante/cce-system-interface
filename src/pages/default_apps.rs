//! XDG default applications: curated categories over `~/.config/mimeapps.list`.
//! Candidates come from the installed `.desktop` entries that claim the
//! category's MIME types; picks are applied through ONE `xdg-mime default`
//! call naming every type, so a browser pick covers http/https/text-html at
//! once (and atomically — parallel calls clobber each other's writes).
//!
//! The Terminal row is the one non-MIME category: terminals have no MIME type,
//! so candidates come from entries declaring `Categories=TerminalEmulator`,
//! and the pick is stored as a COMMAND in the shared config.kdl
//! (`default_terminal`) — which the launcher reads to host `Terminal=true`
//! entries, and startcce exports as `$TERMINAL` for everything else.

use crate::app::{AppAction, PageContent};
use cce_ui::layout::{PageLayoutBuilder, LayoutStrategy};
use cce_ui::widget::{Dropdown, WidgetHost};
use std::collections::HashMap;
use std::path::PathBuf;

/// How a category resolves candidates, its current default, and a pick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CategoryKind {
    /// The first type is the one queried for the current default; a pick sets
    /// every type in the list via `xdg-mime default`.
    Mime(&'static [&'static str]),
    /// The DE's default terminal (see module docs).
    Terminal,
}

const CATEGORIES: &[(&str, CategoryKind)] = &[
    ("Web Browser", CategoryKind::Mime(&["x-scheme-handler/http", "x-scheme-handler/https", "text/html"])),
    ("Mail", CategoryKind::Mime(&["x-scheme-handler/mailto"])),
    ("File Manager", CategoryKind::Mime(&["inode/directory"])),
    ("Terminal", CategoryKind::Terminal),
    ("Text Editor", CategoryKind::Mime(&["text/plain"])),
    ("Images", CategoryKind::Mime(&["image/png", "image/jpeg", "image/gif", "image/webp", "image/svg+xml"])),
    ("Audio", CategoryKind::Mime(&["audio/mpeg", "audio/flac", "audio/ogg", "audio/x-wav"])),
    ("Video", CategoryKind::Mime(&["video/mp4", "video/x-matroska", "video/webm"])),
    ("PDF", CategoryKind::Mime(&["application/pdf"])),
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
    pub kind: CategoryKind,
    pub info: CategoryInfo,
    /// Applied value per dropdown option (desktop id for MIME categories, a
    /// command for Terminal; None = the "not set" placeholder row).
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
                .map(|&(label, kind)| CategoryEntry {
                    label,
                    kind,
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
            log::info!("[default_apps] Set(cat={cat_idx}, opt={opt_idx})");
            let Some(entry) = state.categories.get_mut(cat_idx) else { return };
            let Some(Some(id)) = entry.option_ids.get(opt_idx).cloned() else { return };
            log::info!("[default_apps] applying {:?} -> {id}", entry.label);
            match entry.kind {
                CategoryKind::Mime(mimes) => {
                    // ONE xdg-mime invocation for every type. Parallel
                    // invocations race on the shared `mimeapps.list.new`
                    // temp file and drop each other's writes: a browser pick
                    // left text/html unset (falling through to whatever
                    // mimeinfo.cache lists first), so Chrome's
                    // `xdg-settings check default-web-browser` said "no"
                    // while this page — reading only the first type — said
                    // Chrome.
                    let id = id.clone();
                    tokio::spawn(async move {
                        let out = tokio::process::Command::new("xdg-mime")
                            .arg("default")
                            .arg(&id)
                            .args(mimes)
                            .output()
                            .await;
                        match out {
                            Ok(o) if o.status.success() => {}
                            Ok(o) => log::error!(
                                "[default_apps] xdg-mime default {id} failed: {}",
                                String::from_utf8_lossy(&o.stderr).trim()
                            ),
                            Err(e) => log::error!("[default_apps] xdg-mime spawn failed: {e}"),
                        }
                    });
                }
                CategoryKind::Terminal => set_default_terminal(&id),
            }
            entry.info.current = Some(id);
            rebuild_entry_options(entry);
        }
    }
}

/// Write the terminal pick into the SHARED config.kdl (`default_terminal` at
/// the top level) — the launcher and startcce read it from there, so the
/// per-app override file would hide it from both.
fn set_default_terminal(cmd: &str) {
    let path = cce_ui::config::get_config_path();
    let content = std::fs::read_to_string(&path).unwrap_or_default();
    let Ok(mut doc) = content.parse::<kdl::KdlDocument>() else {
        log::error!("[default_apps] config.kdl did not parse; terminal pick dropped");
        return;
    };
    if cce_ui::config::update_kdl_in_memory(&mut doc, "default_terminal", cmd, "") {
        if let Err(e) = std::fs::write(&path, doc.to_string()) {
            log::error!("[default_apps] config.kdl write failed: {e}");
        }
    } else {
        log::error!("[default_apps] update_kdl_in_memory refused default_terminal");
    }
}

// ── Background fetching ──

struct DesktopApp {
    name: String,
    mimes: Vec<String>,
    /// `Categories=` entries (the Terminal row keys off `TerminalEmulator`).
    categories: Vec<String>,
    /// Basename of `Exec=`'s first token — the command a terminal pick stores.
    exec_cmd: Option<String>,
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
    let mut categories = Vec::new();
    let mut exec_cmd = None;
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
                "Categories" => {
                    categories = v.split(';').map(str::trim).filter(|c| !c.is_empty()).map(String::from).collect();
                }
                "Exec" => {
                    exec_cmd = v
                        .split_whitespace()
                        .next()
                        .and_then(|t| t.rsplit('/').next())
                        .map(String::from);
                }
                _ => {}
            }
        }
    }
    if hidden || app_type.as_deref() != Some("Application") {
        return None;
    }
    Some(DesktopApp { name: name?, mimes, categories, exec_cmd, no_display })
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
    for &(_, kind) in CATEGORIES {
        let info = match kind {
            CategoryKind::Mime(mimes) => fetch_mime_category(&apps, mimes).await,
            CategoryKind::Terminal => fetch_terminal_category(&apps),
        };
        cats.push(info);
    }
    DefaultAppsInfo(cats)
}

async fn query_default(mime: &str) -> Option<String> {
    tokio::process::Command::new("xdg-mime")
        .args(["query", "default", mime])
        .output()
        .await
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
}

async fn fetch_mime_category(apps: &HashMap<String, DesktopApp>, mimes: &[&str]) -> CategoryInfo {
    // The category's default is only real if EVERY type agrees — a partial
    // set (one type lost to the write race above, or set by hand) shows as
    // "not set" so re-picking repairs it, instead of reporting an app the
    // other types don't actually resolve to.
    let mut current = query_default(mimes[0]).await;
    for mime in &mimes[1..] {
        if current.is_none() {
            break;
        }
        if query_default(mime).await != current {
            current = None;
        }
    }

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
    CategoryInfo { candidates, current }
}

/// Candidates are `Categories=TerminalEmulator` entries keyed by COMMAND
/// (deduped — one terminal often ships several entries); the current value is
/// config.kdl's `default_terminal`, falling back to the launcher's compiled
/// foot fallback so the row shows what actually happens today.
fn fetch_terminal_category(apps: &HashMap<String, DesktopApp>) -> CategoryInfo {
    let mut by_cmd: HashMap<&str, &str> = HashMap::new();
    for app in apps.values() {
        if app.no_display || !app.categories.iter().any(|c| c == "TerminalEmulator") {
            continue;
        }
        if let Some(cmd) = app.exec_cmd.as_deref() {
            // Prefer the shortest display name for a command (foot ships
            // "Foot" and "Foot (server)" — the plain one reads best).
            let name = by_cmd.entry(cmd).or_insert(&app.name);
            if app.name.len() < name.len() {
                *name = &app.name;
            }
        }
    }
    let mut candidates: Vec<(String, String)> =
        by_cmd.into_iter().map(|(cmd, name)| (cmd.to_string(), name.to_string())).collect();
    candidates.sort_by(|a, b| a.1.to_lowercase().cmp(&b.1.to_lowercase()));

    let current = cce_ui::config::get_string("/default_terminal")
        .filter(|s| !s.is_empty())
        .or_else(|| Some("foot".to_string()).filter(|_| candidates.iter().any(|(c, _)| c == "foot")));

    // A configured command with no matching entry still shows, as itself.
    if let Some(cur) = &current {
        if !candidates.iter().any(|(cmd, _)| cmd == cur) {
            candidates.push((cur.clone(), cur.clone()));
        }
    }
    CategoryInfo { candidates, current }
}

impl crate::pages::AppPage for DefaultAppsState {
    // Sections: [Default Apps]
    // Mirrors the view's load gate (d13a901): the category dropdowns are added
    // to the stack only in the `else` of `if !state.loaded`, so reporting them
    // during the application scan is one dead root per category.
    fn section_widgets(&mut self) -> Vec<Vec<cce_ui::widget::WidgetId>> {
        if !self.loaded {
            return vec![Vec::new()];
        }
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
    fn section_widgets_mirror_load_gate() {
        use crate::pages::AppPage;
        let mut st = DefaultAppsState::default();
        // Still scanning: the dropdowns are not in the stack yet, so none of
        // them may be reported as a dispatch root.
        assert_eq!(st.section_widgets(), vec![Vec::new()]);
        st.loaded = true;
        assert_eq!(st.section_widgets()[0].len(), st.categories.len());
    }

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
