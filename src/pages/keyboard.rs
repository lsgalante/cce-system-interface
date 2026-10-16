//! Keyboard: the shortcuts in force, read-only. Bindings live in
//! `~/.config/cce/input.kdl` and are edited there (cce-data-editor, by hand) —
//! the DE deliberately has no binding editor, and cce-core's domain writer
//! (`input::write_domain`) regenerates a whole block, dropping its comments,
//! its `input { }` child and the `(gesture)` tags, so it is not one to put
//! behind a button.
//!
//! What can be shown honestly: the window manager's table, merged the way
//! the compositor merges it (the file's entries in order, then the stock
//! `DEFAULT_BINDINGS` whose chord the file leaves free); this app's own
//! chords; and the overrides the file holds for other apps. Another app's
//! defaults are compiled into that app and are not listed anywhere this page
//! could read, so that section says so instead of guessing.

use cce_ui::compose::{PageFlow, PageLayoutBuilder};
use cce_ui::context::UiContext;
use cce_ui::input::{InputConfig, UI_DOMAIN, WINDOW_MANAGER_DOMAIN};
use cce_window_manager::api::Action;
use cce_window_manager::bindings::{mods, parse_chord, parse_gesture, DEFAULT_BINDINGS};

use crate::app::{form_button_fit, form_divider, form_pairs, AppAction, PageCanvas};
use crate::pages::AppPage;

const TEXT_FG: [f32; 4] = [0.83, 0.83, 0.83, 1.0];
const TEXT_DIM: [f32; 4] = [0.53, 0.53, 0.60, 1.0];
const WARN: [f32; 4] = [0.85, 0.45, 0.40, 1.0];
const BTN_BG: [f32; 4] = [0.20, 0.33, 0.22, 1.0];
const BTN_HOVER: [f32; 4] = [0.25, 0.30, 0.26, 1.0];
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

/// One of this app's configurable shortcuts: its input.kdl name, the chord
/// it has when the file is silent, and what it does. The key handler
/// (`input_handler::settings_keys`) takes its defaults from here, so the page
/// cannot list a default the app does not use.
pub struct AppShortcut {
    pub name: &'static str,
    pub default: &'static str,
    pub label: &'static str,
}

pub const APP_SHORTCUTS: [AppShortcut; 7] = [
    AppShortcut { name: "open_search", default: "/", label: "Search this page" },
    AppShortcut { name: "page_next", default: "pagedown", label: "Next page" },
    AppShortcut { name: "page_prev", default: "pageup", label: "Previous page" },
    AppShortcut { name: "focus_next", default: "ctrl+j", label: "Focus next section or control" },
    AppShortcut { name: "focus_prev", default: "ctrl+k", label: "Focus previous section or control" },
    AppShortcut { name: "focus_ascend", default: "ctrl+u", label: "Focus the section around the control" },
    AppShortcut { name: "focus_descend", default: "ctrl+i", label: "Focus the section's first control" },
];

/// The compiled-in chord for one of [`APP_SHORTCUTS`].
pub fn default_chord(name: &str) -> &'static str {
    APP_SHORTCUTS.iter().find(|s| s.name == name).map_or("", |s| s.default)
}

/// Where a row's binding comes from.
#[derive(Debug, Clone, PartialEq)]
pub enum Source {
    /// Written in input.kdl.
    File,
    /// Compiled in: nothing in the file takes its place.
    Default,
    /// In the file, but the reader skips it, for the reason given.
    Ignored(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShortcutRow {
    /// The chord as a person reads it: "Super+Shift+H", "3-finger swipe left".
    pub chord: String,
    /// What it does.
    pub action: String,
    pub source: Source,
    /// The running app's chord differs from the file's: it took effect at
    /// startup and the file has changed since.
    pub restart_needed: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct KeyboardSnapshot {
    pub path: String,
    pub exists: bool,
    /// The file does not parse; every reader falls back to its defaults.
    pub parse_error: Option<String>,
    pub window_manager: Vec<ShortcutRow>,
    pub this_app: Vec<ShortcutRow>,
    /// Other domains' entries, in file order: (domain, rows).
    pub other_apps: Vec<(String, Vec<ShortcutRow>)>,
}

#[derive(Default)]
pub struct KeyboardState {
    pub loaded: bool,
    pub snapshot: KeyboardSnapshot,
    /// The last Open failed to start the editor.
    pub open_error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum KeyboardMessage {
    Refreshed(KeyboardSnapshot),
    OpenFile,
}

pub fn update(state: &mut KeyboardState, msg: KeyboardMessage, _ctx: &mut UiContext) {
    match msg {
        KeyboardMessage::Refreshed(s) => {
            state.snapshot = s;
            state.loaded = true;
        }
        KeyboardMessage::OpenFile => {
            let mut cmd = std::process::Command::new("cce-data-editor");
            cmd.arg(&state.snapshot.path);
            state.open_error = crate::spawn_detached(cmd).err().map(|e| format!("Could not start cce-data-editor: {e}"));
        }
    }
}

/// The page worker's read: the file as it is now.
pub fn read_keyboard_snapshot() -> KeyboardSnapshot {
    let path = cce_ui::input::get_input_path();
    let content = std::fs::read_to_string(&path);
    let app = cce_ui::config::get_app_name().unwrap_or_else(|| "cce-system-interface".to_string());
    let mut snap = build_snapshot(content.as_deref().unwrap_or(""), &app, |name, default| {
        cce_ui::input::app_chord(name, default)
    });
    snap.path = path.to_string_lossy().into_owned();
    snap.exists = content.is_ok();
    snap
}

/// The snapshot for `content` (input.kdl's text). `app` is this app's
/// domain; `active` is the chord the running app resolved for one of its
/// shortcuts (`input::app_chord`, cached at first use), which the file may
/// have moved on from.
pub fn build_snapshot(content: &str, app: &str, active: impl Fn(&str, &str) -> String) -> KeyboardSnapshot {
    let (config, parse_error) = if content.trim().is_empty() {
        (InputConfig::default(), None)
    } else {
        match InputConfig::parse(content) {
            Ok(c) => (c, None),
            Err(e) => (InputConfig::default(), Some(e)),
        }
    };

    let this_app = APP_SHORTCUTS
        .iter()
        .map(|s| {
            let entry = config.resolve(app, s.name);
            let chord = entry.map_or(s.default, |e| e.chord.as_str());
            ShortcutRow {
                chord: display_chord(chord),
                action: s.label.to_string(),
                source: if entry.is_some() { Source::File } else { Source::Default },
                restart_needed: !same_chord(&active(s.name, s.default), chord),
            }
        })
        .collect();

    let other_apps = domain_names(content)
        .into_iter()
        .filter(|d| d != WINDOW_MANAGER_DOMAIN && d != app)
        .map(|d| {
            let rows = config
                .domain(&d)
                .iter()
                .map(|e| ShortcutRow {
                    chord: display_chord(&e.chord),
                    action: humanize(&e.name),
                    source: Source::File,
                    restart_needed: false,
                })
                .collect::<Vec<_>>();
            (d, rows)
        })
        .filter(|(_, rows)| !rows.is_empty())
        .collect();

    KeyboardSnapshot {
        path: String::new(),
        exists: true,
        parse_error,
        window_manager: window_manager_rows(&config),
        this_app,
        other_apps,
    }
}

/// The compositor's merge (`config/apply.rs`): the file's entries in order —
/// an unknown action or a chord that is neither a gesture nor a key chord is
/// skipped, and a chord bound twice fires only its first entry — then each
/// stock default whose chord is still free. Legacy `config.kdl` key_bindings
/// still load in the compositor but are not read here.
fn window_manager_rows(config: &InputConfig) -> Vec<ShortcutRow> {
    let mut rows = Vec::new();
    let mut taken: Vec<String> = Vec::new();
    for e in config.domain(WINDOW_MANAGER_DOMAIN) {
        let action = Action::from_name(&e.name);
        let key = chord_key(&e.chord);
        let source = match (&action, &key) {
            (None, _) => Source::Ignored(format!("unknown action \"{}\"", e.name)),
            (_, None) => Source::Ignored("not a valid chord".to_string()),
            (Some(_), Some(k)) if taken.contains(k) => Source::Ignored("bound above; never fires".to_string()),
            _ => Source::File,
        };
        if let (Source::File, Some(k)) = (&source, key) {
            taken.push(k);
        }
        rows.push(ShortcutRow {
            chord: display_chord(&e.chord),
            action: action.map_or_else(|| humanize(&e.name), |a| describe(a, e.command.as_deref())),
            source,
            restart_needed: false,
        });
    }
    for d in DEFAULT_BINDINGS {
        let chord = default_chord_string(d.mods, d.key);
        let Some(key) = chord_key(&chord) else { continue };
        if taken.contains(&key) {
            continue;
        }
        taken.push(key);
        rows.push(ShortcutRow {
            chord: display_chord(&chord),
            action: describe(d.action, None),
            source: Source::Default,
            restart_needed: false,
        });
    }
    rows
}

/// A comparable identity for a chord: gestures and key chords in one space,
/// modifiers as their mask. Keysym names compare case-insensitively, as a
/// person writing "escape" or "Escape" means the same key.
fn chord_key(chord: &str) -> Option<String> {
    if let Some(edge) = chord.trim().strip_prefix("edge_") {
        return Some(format!("edge:{}", edge.to_lowercase()));
    }
    if let Some(g) = parse_gesture(chord) {
        let fingers = g.fingers.map_or("34".to_string(), |n| n.to_string());
        return Some(format!("g:{}:{}:{}:{}", g.mods, g.kind.as_str(), fingers, g.direction));
    }
    parse_chord(chord).map(|c| format!("k:{}:{}", c.mods, c.key.to_lowercase()))
}

fn same_chord(a: &str, b: &str) -> bool {
    match (chord_key(a), chord_key(b)) {
        (Some(x), Some(y)) => x == y,
        _ => a.trim().eq_ignore_ascii_case(b.trim()),
    }
}

/// A `DefaultBinding`'s mask and key as the chord a person would write.
fn default_chord_string(m: u32, key: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for (bit, name) in [(mods::SUPER, "super"), (mods::CTRL, "ctrl"), (mods::ALT, "alt"), (mods::SHIFT, "shift")] {
        if m & bit != 0 {
            parts.push(name);
        }
    }
    parts.push(key);
    parts.join("+")
}

/// The top-level domain blocks of `content`, in file order (the global
/// `input { }` settings block is not a domain).
fn domain_names(content: &str) -> Vec<String> {
    let Ok(doc) = content.parse::<kdl::KdlDocument>() else { return Vec::new() };
    let mut names: Vec<String> = Vec::new();
    for n in doc.nodes() {
        let name = n.name().value();
        if name != "input" && n.children().is_some() && !names.iter().any(|x| x == name) {
            names.push(name.to_string());
        }
    }
    names
}

/// "super+shift+h" → "Super+Shift+H"; "swipe3_left" → "3-finger swipe left".
pub fn display_chord(chord: &str) -> String {
    let chord = chord.trim();
    let (mod_part, last) = match chord.rfind('+') {
        // A trailing "+" is the plus key itself ("ctrl++").
        Some(i) if i + 1 == chord.len() && i > 0 => (&chord[..i - 1], "+"),
        Some(i) => (&chord[..i], &chord[i + 1..]),
        None => ("", chord),
    };
    let mut out: Vec<String> = mod_part
        .split('+')
        .filter(|s| !s.is_empty())
        .map(|m| match m.to_lowercase().as_str() {
            "super" | "mod4" | "logo" | "win" => "Super".to_string(),
            "ctrl" | "control" => "Ctrl".to_string(),
            "alt" | "mod1" | "meta" => "Alt".to_string(),
            "shift" => "Shift".to_string(),
            other => other.to_string(),
        })
        .collect();
    out.push(display_key(last));
    out.join("+")
}

fn display_key(key: &str) -> String {
    let lower = key.to_lowercase().replace('-', "_");
    if let Some(edge) = lower.strip_prefix("edge_") {
        return format!("Swipe in from the {edge} edge");
    }
    for (kind, word) in [("swipe", "swipe"), ("pinch", "pinch")] {
        if let Some(rest) = lower.strip_prefix(kind) {
            if let Some((count, dir)) = rest.split_once('_') {
                let fingers = if count.is_empty() { "3- or 4-finger".to_string() } else { format!("{count}-finger") };
                return format!("{fingers} {word} {dir}");
            }
        }
    }
    let named = match lower.as_str() {
        "slash" => "/",
        "backslash" => "\\",
        "equal" => "=",
        "minus" => "-",
        "plus" => "+",
        "comma" => ",",
        "period" => ".",
        "semicolon" => ";",
        "apostrophe" => "'",
        "grave" => "`",
        "bracketleft" => "[",
        "bracketright" => "]",
        "space" => "Space",
        "escape" | "esc" => "Esc",
        "return" | "enter" => "Enter",
        "tab" => "Tab",
        "backspace" => "Backspace",
        "delete" => "Delete",
        "pageup" | "prior" | "page_up" => "Page Up",
        "pagedown" | "next" | "page_down" => "Page Down",
        "print" => "Print Screen",
        "up" => "Up",
        "down" => "Down",
        "left" => "Left",
        "right" => "Right",
        "home" => "Home",
        "end" => "End",
        _ => "",
    };
    if !named.is_empty() {
        return named.to_string();
    }
    if let Some(rest) = key.strip_prefix("XF86") {
        // XF86AudioRaiseVolume → "Audio Raise Volume", as the key is labelled.
        let rest = rest.strip_prefix("Mon").unwrap_or(rest);
        let mut words = String::new();
        for (i, c) in rest.chars().enumerate() {
            if i > 0 && c.is_uppercase() {
                words.push(' ');
            }
            words.push(c);
        }
        return words;
    }
    if key.chars().count() == 1 {
        return key.to_uppercase();
    }
    let mut c = key.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().chain(c).collect())
}

/// "move_window_left" → "Move window left".
fn humanize(name: &str) -> String {
    let spaced = name.replace(['_', '-'], " ");
    let mut c = spaced.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().chain(c).collect())
}

/// What a window-manager action does, with the command it runs.
fn describe(action: Action, command: Option<&str>) -> String {
    let run = |verb: &str, c: &str| {
        // A launcher path reads as its program: "/home/u/.local/bin/cce-cloud --apps" → "cce-cloud --apps".
        let mut words = c.split_whitespace();
        let program = words.next().unwrap_or("");
        let program = program.rsplit('/').next().unwrap_or(program);
        let rest: Vec<&str> = words.collect();
        if rest.is_empty() { format!("{verb} {program}") } else { format!("{verb} {program} {}", rest.join(" ")) }
    };
    match (action, command) {
        (Action::Spawn, Some(c)) => run("Run", c),
        (Action::Toggle, Some(c)) => run("Toggle", c),
        (Action::Spawn | Action::Toggle, None) => format!("{} (no command set)", humanize(action.name())),
        (_, Some(c)) => format!("{} — runs {}", humanize(action.name()), run("", c).trim_start()),
        (Action::Close, None) => "Close window".to_string(),
        (Action::Fullscreen, None) => "Toggle fullscreen".to_string(),
        (Action::Overview, None) => "Toggle overview".to_string(),
        (Action::Reload, None) => "Reload the window manager's config".to_string(),
        _ => humanize(action.name()),
    }
}

/// The rows as `form_pairs` lines: the chord, then what it does, a
/// default dimmed and a skipped entry in the warning colour with why.
fn pairs(rows: &[ShortcutRow]) -> Vec<(String, [f32; 4], String, [f32; 4])> {
    rows.iter()
        .map(|r| match &r.source {
            Source::File if r.restart_needed => (r.chord.clone(), TEXT_FG, format!("{} — restart this app to apply", r.action), WARN),
            Source::File => (r.chord.clone(), TEXT_FG, r.action.clone(), TEXT_FG),
            Source::Default => (r.chord.clone(), TEXT_DIM, r.action.clone(), TEXT_DIM),
            Source::Ignored(why) => (r.chord.clone(), WARN, format!("{} — ignored: {why}", r.action), WARN),
        })
        .collect()
}

fn wrapped(g: &mut cce_ui::compose::FormGroup<'_, '_, PageCanvas>, text: &str, size: f32, color: [f32; 4]) {
    let lines = crate::app::wrap_to_width(text, g.form_width(), size);
    g.lines(lines, size, color);
}

impl AppPage for KeyboardState {
    // Sections: [Shortcuts, Window Manager, This App, Other Apps] — read-only,
    // so no widgets; the Open button is a page button.
    fn section_widgets(&mut self) -> Vec<Vec<cce_ui::widget::WidgetId>> {
        vec![Vec::new(), Vec::new(), Vec::new(), Vec::new()]
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
        ctx: &mut UiContext,
    ) -> PageCanvas {
        let mut final_pc = PageCanvas::new();
        let sec_w = 320.0f32;
        let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(4);
        let snap = &self.snapshot;
        let focused = |i: usize| sec_focused.get(i).copied().unwrap_or(false);

        builder.add_section(&mut final_pc, "Shortcuts", focused(0), |sec| {
            let mut form = sec.form();
            let mut col = form.column();
            if !self.loaded {
                col.text("Reading input.kdl...", 12.0, TEXT_FG);
            } else {
                wrapped(&mut col, "Read-only. Shortcuts are changed in input.kdl.", 12.0, TEXT_FG);
                // Name the reload chord as it is bound now, not as it ships.
                let reload = snap
                    .window_manager
                    .iter()
                    .find(|r| r.action == describe(Action::Reload, None) && !matches!(r.source, Source::Ignored(_)))
                    .map_or("a reload".to_string(), |r| r.chord.clone());
                wrapped(
                    &mut col,
                    &format!("Apps read it when they start. The window manager rereads it on {reload}."),
                    11.0,
                    TEXT_DIM,
                );
                if let Some(e) = &snap.parse_error {
                    wrapped(&mut col, &format!("input.kdl does not parse, so every app is using its defaults: {e}"), 11.0, WARN);
                }
                if !snap.exists {
                    wrapped(&mut col, "There is no input.kdl yet: every shortcut is at its default.", 11.0, TEXT_DIM);
                } else {
                    wrapped(&mut col, &snap.path, 11.0, TEXT_DIM);
                    let label = "Open input.kdl";
                    let need = crate::app::button_need(label);
                    col.row(|r| {
                        form_button_fit(r, label, need, (BTN_BG, BTN_HOVER, WHITE), AppAction::Keyboard(KeyboardMessage::OpenFile));
                    });
                }
                if let Some(e) = &self.open_error {
                    wrapped(&mut col, e, 11.0, WARN);
                }
            }
            sec.place(form, ctx);
        });

        builder.add_section(&mut final_pc, "Window Manager", focused(1), |sec| {
            let mut form = sec.form();
            let mut col = form.column();
            if self.loaded {
                form_pairs(&mut col, 11.0, pairs(&snap.window_manager));
                form_divider(&mut col);
                wrapped(&mut col, "Dimmed: built in, not set in input.kdl.", 11.0, TEXT_DIM);
            }
            sec.place(form, ctx);
        });

        builder.add_section(&mut final_pc, "This App", focused(2), |sec| {
            let mut form = sec.form();
            let mut col = form.column();
            if self.loaded {
                let mut rows = pairs(&snap.this_app);
                rows.insert(1, ("Esc".to_string(), TEXT_DIM, "Close the search (fixed)".to_string(), TEXT_DIM));
                form_pairs(&mut col, 11.0, rows);
            }
            sec.place(form, ctx);
        });

        builder.add_section(&mut final_pc, "Other Apps", focused(3), |sec| {
            let mut form = sec.form();
            let mut col = form.column();
            if self.loaded {
                wrapped(
                    &mut col,
                    "Only what input.kdl changes. Each app's own defaults are built into it and are not listed here.",
                    11.0,
                    TEXT_DIM,
                );
                if snap.other_apps.is_empty() {
                    col.text("No other app's shortcuts are changed.", 11.0, TEXT_DIM);
                }
                for (domain, rows) in &snap.other_apps {
                    form_divider(&mut col);
                    let title = if domain == UI_DOMAIN { "Every app (cce-ui)".to_string() } else { domain.clone() };
                    col.text(title, 12.0, TEXT_FG);
                    form_pairs(&mut col, 11.0, pairs(rows));
                }
            }
            sec.place(form, ctx);
        });

        final_pc
    }

    fn propagate_widget_changes(
        &mut self,
        _events: &[(cce_ui::widget::WidgetId, cce_ui::widget::WidgetEvent)],
        _actions: &mut Vec<AppAction>,
        _ctx: &mut UiContext,
    ) {
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE: &str = r#"
input {
    trackpad {
        natural_scroll (bool)true
    }
}
cce-window-manager {
    // a comment
    spawn (keybind)"super+d" command="/home/u/.local/bin/cce-cloud --apps"
    close_window (keybind)"super+q"
    focus_left (gesture)"swipe3_left"
    overview (gesture)"swipe_down"
    focus_up (keybind)"super+Up"
    fly_away (keybind)"super+z"
    close_window (keybind)"super+q"
    toggle_fullscreen (keybind)"super+frob+f"
}
cce-ui {
    open_search (keybind)"ctrl+f"
}
cce-system-interface {
    page_next (keybind)"ctrl+n"
}
"#;

    fn snap() -> KeyboardSnapshot {
        build_snapshot(FILE, "cce-system-interface", |name, default| {
            // The running app resolved the file as it was at startup: page_next
            // was still its default.
            if name == "page_next" { default.to_string() } else { "ctrl+f".to_string() }
        })
    }

    #[test]
    fn window_manager_merges_as_the_compositor_does() {
        let wm = snap().window_manager;
        let find = |chord: &str| wm.iter().filter(|r| r.chord == chord).collect::<Vec<_>>();

        assert_eq!(find("Super+D")[0].action, "Run cce-cloud --apps");
        assert_eq!(find("3-finger swipe left")[0].action, "Focus left");
        assert_eq!(find("3- or 4-finger swipe down")[0].action, "Toggle overview");

        let close = find("Super+Q");
        assert_eq!(close.len(), 2);
        assert_eq!(close[0].source, Source::File);
        assert!(matches!(close[1].source, Source::Ignored(_)), "the second binding never fires");

        assert!(matches!(find("Super+Z")[0].source, Source::Ignored(ref w) if w.contains("fly_away")));
        assert!(matches!(wm.iter().find(|r| r.action == "Toggle fullscreen").unwrap().source, Source::Ignored(_)));

        // super+Up is taken by the file, so the stock focus_up on it is not
        // listed again; super+k, which the file leaves free, is.
        assert_eq!(find("Super+Up").len(), 1);
        assert_eq!(find("Super+Up")[0].source, Source::File);
        assert_eq!(find("Super+K")[0].source, Source::Default);
        assert_eq!(find("Audio Raise Volume")[0].source, Source::Default);
    }

    /// A default is listed under exactly the chord that shadows it, so every
    /// stock binding appears once when the file binds nothing.
    #[test]
    fn an_empty_file_lists_every_default_once() {
        let s = build_snapshot("", "cce-system-interface", |_, d| d.to_string());
        assert_eq!(s.window_manager.len(), DEFAULT_BINDINGS.len());
        assert!(s.window_manager.iter().all(|r| r.source == Source::Default));
        assert!(s.this_app.iter().all(|r| r.source == Source::Default && !r.restart_needed));
        assert!(s.other_apps.is_empty());
    }

    #[test]
    fn this_app_resolves_through_cce_ui_and_flags_a_stale_chord() {
        let s = snap();
        let row = |label: &str| s.this_app.iter().find(|r| r.action == label).unwrap();
        // open_search comes from the cce-ui domain, and the app already uses it.
        assert_eq!(row("Search this page").chord, "Ctrl+F");
        assert_eq!(row("Search this page").source, Source::File);
        assert!(!row("Search this page").restart_needed);
        // page_next changed in the file after the app read it.
        assert_eq!(row("Next page").chord, "Ctrl+N");
        assert!(row("Next page").restart_needed);
        assert_eq!(row("Previous page").source, Source::Default);
    }

    #[test]
    fn other_apps_lists_only_other_domains() {
        let s = snap();
        let domains: Vec<&str> = s.other_apps.iter().map(|(d, _)| d.as_str()).collect();
        assert_eq!(domains, ["cce-ui"]);
        assert_eq!(s.other_apps[0].1[0].chord, "Ctrl+F");
        assert_eq!(s.other_apps[0].1[0].action, "Open search");
    }

    #[test]
    fn a_broken_file_is_reported_and_defaults_stand() {
        let s = build_snapshot("cce-window-manager { spawn \"super+d", "cce-system-interface", |_, d| d.to_string());
        assert!(s.parse_error.is_some());
        assert_eq!(s.window_manager.len(), DEFAULT_BINDINGS.len());
    }

    #[test]
    fn chords_read_as_they_are_labelled() {
        assert_eq!(display_chord("super+shift+h"), "Super+Shift+H");
        assert_eq!(display_chord("super+ctrl+shift+equal"), "Super+Ctrl+Shift+=");
        assert_eq!(display_chord("super+slash"), "Super+/");
        assert_eq!(display_chord("escape"), "Esc");
        assert_eq!(display_chord("pagedown"), "Page Down");
        assert_eq!(display_chord("alt+shift+Tab"), "Alt+Shift+Tab");
        assert_eq!(display_chord("XF86MonBrightnessUp"), "Brightness Up");
        assert_eq!(display_chord("super+pinch_out"), "Super+3- or 4-finger pinch out");
        assert_eq!(display_chord("swipe4_down"), "4-finger swipe down");
        assert_eq!(display_chord("ctrl++"), "Ctrl++");
    }

    /// The key handler's defaults are this table's: one place to change.
    #[test]
    fn every_app_shortcut_has_a_default() {
        for s in &APP_SHORTCUTS {
            assert_eq!(default_chord(s.name), s.default);
            assert!(parse_chord(s.default).is_some(), "{} parses", s.name);
        }
    }
}
