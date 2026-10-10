use crate::app::{button_need, form_button, form_divider, wrap_to_width, AppAction, PageContent};
use cce_ui::context::UiContext;
use cce_ui::widget::Handle;
use cce_ui::widget::ScrollRegion;
use cce_ui::compose::{lay_row, render_widget_h, Cell, PageLayoutBuilder, PageFlow};
use cce_ui::scene::paint::{RenderTarget};
use cce_ui::scene::layout::Rect;
use cce_ui::widget::{TextBox, InteractiveListItem};

#[derive(Debug, Clone, Default)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    /// `pacman -Qe`: installed on purpose rather than pulled in as a dependency.
    pub explicit: bool,
    /// `pacman -Qdt`: a dependency nothing installed requires or optionally
    /// requires any more — what `pacman -Rns $(pacman -Qdtq)` would sweep.
    pub orphan: bool,
}

#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub name: String,
    pub old_version: String,
    pub new_version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PackageTab {
    #[default]
    Installed,
    Updates,
}

/// Which slice of the installed list the Installed tab shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InstalledFilter {
    #[default]
    All,
    Explicit,
    Orphans,
}

impl InstalledFilter {
    fn admits(self, p: &PackageInfo) -> bool {
        match self {
            InstalledFilter::All => true,
            InstalledFilter::Explicit => p.explicit,
            InstalledFilter::Orphans => p.orphan,
        }
    }
}

/// What a removal would take, asked of `pacman -Rs --print` before anything
/// is removed: the targets plus every dependency only they needed, or the
/// error that stops it (a target another package still requires).
#[derive(Debug, Clone)]
pub struct RemovalPlan {
    pub targets: Vec<String>,
    pub outcome: Result<Vec<(String, u64)>, String>,
}

#[derive(Debug, Clone)]
pub struct PackagesState {
    pub loaded: bool,
    pub installed: Vec<PackageInfo>,
    pub updates: Vec<UpdateInfo>,
    pub active_tab: PackageTab,
    pub search_box: Handle<cce_ui::widget::Adapted<TextBox>>,
    pub installed_list: ScrollRegion,
    pub installed_items: Vec<Handle<cce_ui::widget::Adapted<cce_ui::widget::InteractiveListItem>>>,
    pub updates_list: ScrollRegion,
    pub updates_items: Vec<Handle<cce_ui::widget::Adapted<cce_ui::widget::InteractiveListItem>>>,
    pub updating: bool,
    pub last_update_res: Option<Result<(), String>>,
    pub selected_package: Option<String>,
    pub selected_package_info: Option<String>,
    pub loading_info: bool,
    pub uninstalling: bool,
    pub filter: InstalledFilter,
    /// Row clicks toggle membership in `checked` instead of opening details.
    pub select_mode: bool,
    pub checked: std::collections::BTreeSet<String>,
    /// The confirmation step of a removal; `previewing` while pacman is asked.
    pub removal: Option<RemovalPlan>,
    pub previewing: bool,
    pub marking: bool,
    /// Outcome of the last remove / mark, shown until the next one starts.
    pub last_action: Option<Result<String, String>>,
}

impl Default for PackagesState {
    fn default() -> Self {
        Self {
            loaded: false,
            installed: Vec::new(),
            updates: Vec::new(),
            active_tab: PackageTab::Installed,
            search_box: Handle::none(),
            installed_list: ScrollRegion::new(32.0, 4.0).with_frame(false).with_sink_behind(true),
            installed_items: Vec::new(),
            updates_list: ScrollRegion::new(32.0, 4.0).with_frame(false).with_sink_behind(true),
            updates_items: Vec::new(),
            updating: false,
            last_update_res: None,
            selected_package: None,
            selected_package_info: None,
            loading_info: false,
            uninstalling: false,
            filter: InstalledFilter::All,
            select_mode: false,
            checked: std::collections::BTreeSet::new(),
            removal: None,
            previewing: false,
            marking: false,
            last_action: None,
        }
    }
}

impl PackagesState {
    /// The page's state, its widgets inserted into `ctx`.
    pub fn new(ctx: &mut UiContext) -> Self {
        Self {
            search_box: ctx.insert(TextBox::new(String::new()).with_placeholder("Filter Packages...")),
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone)]
pub enum PackagesMessage {
    Refreshed(Box<PackagesState>),
    SetTab(PackageTab),
    StartUpdate,
    UpdateFinished(Result<(), String>),
    SelectPackage(Option<String>),
    SelectAndScrollPackage(String),
    InfoFetched(String, Result<String, String>),
    SetFilter(InstalledFilter),
    ToggleSelectMode,
    ToggleChecked(String),
    CheckAllVisible,
    ClearChecked,
    /// Ask pacman what removing these would take; nothing is removed yet.
    PreviewRemoval(Vec<String>),
    RemovalPreviewed(Vec<String>, Result<Vec<(String, u64)>, String>),
    CancelRemoval,
    /// Remove a previewed plan's targets (and the dependencies only they need).
    StartUninstall(Vec<String>),
    UninstallFinished(Vec<String>, Result<(), String>),
    /// `true` = mark explicitly installed, `false` = mark as a dependency.
    SetInstallReason(Vec<String>, bool),
    InstallReasonSet(Vec<String>, bool, Result<(), String>),
}

pub async fn fetch_packages_state() -> PackagesState {
    let installed = fetch_installed_packages().await;
    let updates = fetch_available_updates().await;
    PackagesState {
        loaded: true,
        installed,
        updates,
        updating: false,
        last_update_res: None,
        ..Default::default()
    }
}

async fn fetch_installed_packages() -> Vec<PackageInfo> {
    let (all, explicit, orphans) = tokio::join!(
        pacman_stdout(&["-Q"]),
        pacman_stdout(&["-Qeq"]),
        // Exits 1 when there are none; the empty stdout is the right answer.
        pacman_stdout(&["-Qdtq"]),
    );
    parse_installed(&all, &explicit, &orphans)
}

async fn pacman_stdout(args: &[&str]) -> String {
    match tokio::process::Command::new("pacman").args(args).output().await {
        Ok(o) => String::from_utf8_lossy(&o.stdout).into_owned(),
        Err(_) => String::new(),
    }
}

/// `pacman -Q` lines joined with the `-Qeq` and `-Qdtq` name lists.
pub fn parse_installed(all: &str, explicit: &str, orphans: &str) -> Vec<PackageInfo> {
    let explicit: std::collections::HashSet<&str> = explicit.lines().map(str::trim).collect();
    let orphans: std::collections::HashSet<&str> = orphans.lines().map(str::trim).collect();
    let mut list: Vec<PackageInfo> = all
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let name = parts.next()?;
            let version = parts.next()?;
            Some(PackageInfo {
                name: name.to_string(),
                version: version.to_string(),
                explicit: explicit.contains(name),
                orphan: orphans.contains(name),
            })
        })
        .collect();
    list.sort_by_key(|a| a.name.to_lowercase());
    list
}

async fn fetch_available_updates() -> Vec<UpdateInfo> {
    let mut list = Vec::new();
    if let Ok(output) = tokio::process::Command::new("checkupdates")
        .output()
        .await
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 && parts[2] == "->" {
                list.push(UpdateInfo {
                    name: parts[0].to_string(),
                    old_version: parts[1].to_string(),
                    new_version: parts[3].to_string(),
                });
            }
        }
    }
    list.sort_by_key(|a| a.name.to_lowercase());
    list
}

pub async fn run_update() -> Result<(), String> {
    let output = tokio::process::Command::new("pkexec")
        .args(["pacman", "-Syu", "--noconfirm"])
        .output()
        .await
        .map_err(|e| format!("Failed to run update: {}", e))?;
        
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(format!("Update process failed: {}", err));
    }
    
    Ok(())
}

/// pacman's own explanation of a failure. It writes the reason to stderr,
/// and for a broken dependency the lines that NAME the dependency (`:: removing
/// x breaks dependency 'x' required by y`) are what make the error useful —
/// so keep every line, not just the first.
fn pacman_error(output: &std::process::Output) -> String {
    let text = summarize_pacman_failure(
        &String::from_utf8_lossy(&output.stderr),
        &String::from_utf8_lossy(&output.stdout),
    );
    match output.status.code() {
        // pkexec's own codes (pacman itself exits 1): 126 = not authorized,
        // 127 = the dialog was dismissed or authentication failed.
        Some(126) | Some(127) => {
            if text.is_empty() {
                "Authorization was cancelled or denied".to_string()
            } else {
                format!("Authorization was cancelled or denied ({text})")
            }
        }
        _ if text.is_empty() => format!("pacman exited with {}", output.status),
        _ => text,
    }
}

/// pacman splits a refusal across both streams: the `error:` headline goes to
/// stderr, the `:: removing x breaks dependency 'x' required by y` lines that
/// say WHY go to stdout (with progress chatter like "checking dependencies...").
/// Keep the headline, fold the break lines into one "x is still required by
/// a, b, c" per package, and drop the chatter.
pub fn summarize_pacman_failure(stderr: &str, stdout: &str) -> String {
    let mut lines: Vec<String> = stderr.lines().map(str::trim).filter(|l| !l.is_empty()).map(String::from).collect();
    let mut required_by: Vec<(String, Vec<String>)> = Vec::new();
    for line in stdout.lines().chain(stderr.lines()).map(str::trim) {
        let Some(rest) = line.strip_prefix(":: removing ") else {
            if line.starts_with("error:") && !lines.iter().any(|l| l == line) {
                lines.push(line.to_string());
            }
            continue;
        };
        // "acl breaks dependency 'acl' required by coreutils"
        if let (Some(pkg), Some(by)) = (rest.split_whitespace().next(), rest.rsplit(" required by ").next()) {
            // One line per broken dependency, and a package can depend on
            // the target twice over (`acl` and the `libacl.so` it provides).
            match required_by.iter_mut().find(|(p, _)| p == pkg) {
                Some((_, list)) if list.iter().any(|b| b == by) => {}
                Some((_, list)) => list.push(by.to_string()),
                None => required_by.push((pkg.to_string(), vec![by.to_string()])),
            }
        }
    }
    // The break lines were also counted from stderr above when pacman wrote
    // them there; they are summarized below instead.
    lines.retain(|l| !l.starts_with(":: removing "));
    for (pkg, by) in required_by {
        lines.push(format!("{pkg} is still required by {}", by.join(", ")));
    }
    lines.join("\n")
}

/// What `pacman -Rs` would remove for `targets`, as (name, installed bytes).
/// Runs unprivileged: `--print` resolves the transaction without locking or
/// touching the database.
pub async fn preview_removal(targets: Vec<String>) -> Result<Vec<(String, u64)>, String> {
    let output = tokio::process::Command::new("pacman")
        .args(["-Rs", "--print", "--print-format", "%n %s", "--"])
        .args(&targets)
        .output()
        .await
        .map_err(|e| format!("Failed to run pacman: {}", e))?;
    if !output.status.success() {
        return Err(pacman_error(&output));
    }
    Ok(parse_removal_preview(&String::from_utf8_lossy(&output.stdout)))
}

pub fn parse_removal_preview(stdout: &str) -> Vec<(String, u64)> {
    stdout
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let name = parts.next()?;
            let size = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
            Some((name.to_string(), size))
        })
        .collect()
}

/// Removes `targets` and the dependencies only they needed (`-Rs`), in ONE
/// transaction — pacman orders it, so a target and the package that required
/// it can go together. Removing them one call at a time fails on whichever
/// comes first and silently leaves its dependencies behind as orphans.
pub async fn run_uninstall(targets: Vec<String>) -> Result<(), String> {
    let output = tokio::process::Command::new("pkexec")
        .args(["pacman", "-Rs", "--noconfirm", "--"])
        .args(&targets)
        .output()
        .await
        .map_err(|e| format!("Failed to run uninstall: {}", e))?;
    if !output.status.success() {
        return Err(pacman_error(&output));
    }
    Ok(())
}

/// `pacman -D --asexplicit` / `--asdeps`: changes only the install reason.
pub async fn run_set_install_reason(targets: Vec<String>, explicit: bool) -> Result<(), String> {
    let flag = if explicit { "--asexplicit" } else { "--asdeps" };
    let output = tokio::process::Command::new("pkexec")
        .args(["pacman", "-D", flag, "--"])
        .args(&targets)
        .output()
        .await
        .map_err(|e| format!("Failed to run pacman: {}", e))?;
    if !output.status.success() {
        return Err(pacman_error(&output));
    }
    Ok(())
}

pub fn human_size(bytes: u64) -> String {
    let b = bytes as f64;
    if b >= 1024.0 * 1024.0 * 1024.0 {
        format!("{:.1} GiB", b / (1024.0 * 1024.0 * 1024.0))
    } else if b >= 1024.0 * 1024.0 {
        format!("{:.1} MiB", b / (1024.0 * 1024.0))
    } else {
        format!("{:.0} KiB", b / 1024.0)
    }
}

/// "3 packages" / "1 package".
fn count_noun(n: usize) -> String {
    if n == 1 { "1 package".to_string() } else { format!("{n} packages") }
}

pub async fn fetch_package_info(name: String, installed: bool) -> Result<String, String> {
    let arg = if installed { "-Qi" } else { "-Si" };
    let output = tokio::process::Command::new("pacman")
        .args([arg, &name])
        .output()
        .await
        .map_err(|e| format!("Failed to run pacman: {}", e))?;
        
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(format!("Command failed: {}", err));
    }
    
    let mut raw_info = String::from_utf8_lossy(&output.stdout).to_string();
    if installed {
        if let Ok(ql_out) = tokio::process::Command::new("pacman")
            .args(["-Ql", &name])
            .output()
            .await
        {
            let stdout = String::from_utf8_lossy(&ql_out.stdout);
            let mut binaries = Vec::new();
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let path = parts[1];
                    if !path.ends_with('/') && (
                        path.starts_with("/usr/bin/") || 
                        path.starts_with("/bin/") || 
                        path.starts_with("/usr/sbin/") || 
                        path.starts_with("/sbin/")
                    ) {
                        if let Some(filename) = path.split('/').next_back() {
                            binaries.push(filename.to_string());
                        }
                    }
                }
            }
            if !binaries.is_empty() {
                binaries.sort();
                binaries.dedup();
                raw_info.push_str(&format!("\nCommands        : {}\n", binaries.join("  ")));
            }
        }
    }
    Ok(raw_info)
}

#[derive(Debug, Clone, Default)]
pub struct ParsedPackageInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub website: String,
    pub size: String,
    pub licenses: String,
    pub packager: String,
    pub build_date: String,
    pub required_by: String,
    pub install_reason: String,
    pub commands: String,
}

pub fn parse_package_info(raw: &str) -> ParsedPackageInfo {
    let mut current_key = String::new();
    let mut map = std::collections::HashMap::new();

    for line in raw.lines() {
        if line.is_empty() {
            continue;
        }
        if !line.starts_with(' ') {
            if let Some(pos) = line.find(':') {
                let key = line[..pos].trim().to_string();
                let val = line[pos + 1..].trim().to_string();
                current_key = key.clone();
                map.insert(key, val);
            }
        } else if !current_key.is_empty() {
            if let Some(val) = map.get_mut(&current_key) {
                val.push(' ');
                val.push_str(line.trim());
            }
        }
    }

    let get_val = |k: &str| map.get(k).cloned().unwrap_or_default();

    let name = get_val("Name");
    let version = get_val("Version");
    let description = get_val("Description");
    let website = get_val("URL");
    
    let mut size = get_val("Installed Size");
    if size.is_empty() {
        size = get_val("Download Size");
    }
    
    let licenses = get_val("Licenses");
    let packager = get_val("Packager");
    let build_date = get_val("Build Date");
    let required_by = get_val("Required By");
    let install_reason = get_val("Install Reason");
    let commands = get_val("Commands");

    ParsedPackageInfo {
        name,
        version,
        description,
        website,
        size,
        licenses,
        packager,
        build_date,
        required_by,
        install_reason,
        commands,
    }
}

pub fn wrap_text(text: &str, max_chars: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let mut current_line = String::new();
        for word in paragraph.split_whitespace() {
            if current_line.is_empty() {
                current_line.push_str(word);
            } else if current_line.len() + 1 + word.len() > max_chars {
                lines.push(current_line);
                current_line = word.to_string();
            } else {
                current_line.push(' ');
                current_line.push_str(word);
            }
        }
        if !current_line.is_empty() {
            lines.push(current_line);
        }
    }
    lines
}

const TEXT_FG: [f32; 4] = [0.83, 0.83, 0.83, 1.0];
const TEXT_DIM: [f32; 4] = [0.53, 0.53, 0.60, 1.0];
const ACCENT: [f32; 4] = [0.36, 0.56, 0.38, 1.0];
const TOGGLE_OFF: [f32; 4] = [0.16, 0.16, 0.24, 1.0];
const BTN_HOVER: [f32; 4] = [0.25, 0.30, 0.26, 1.0];
const RED: [f32; 4] = [0.85, 0.25, 0.25, 1.0];
/// The least room the list keeps on a short window.
const LIST_MIN_H: f32 = 120.0;

pub fn view(
    state: &mut PackagesState,
    cx: f32,
    cy: f32,
    cw: f32,
    ch: f32,
    sec_focused: &[bool],
    layout: &mut PageFlow,
    ctx: &mut cce_ui::context::UiContext,
) -> PageContent {
    let mut final_pc = PageContent::new();
    let sec_w = 260.0f32;
    let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(1);

    builder.add_section_spanned(&mut final_pc, "", 1, sec_focused.first().copied().unwrap_or(false), |sec| {
        let mut form = sec.form();
        if !state.loaded {
            form.column().text("Loading package lists...", 12.0, TEXT_DIM);
            sec.place(form, ctx);
            return;
        }
        // The list fills the page like the services list: the well's floor lands at the
        // page's bottom.
        form.fill_height((cy + ch) - form.top() - sec.bottom_inset());
        let wrap_w = form.width();

        let active_bg = [0.20, 0.40, 0.65, 0.4];
        let inactive_bg = [0.10, 0.10, 0.16, 0.3];
        let hover_bg = [0.20, 0.20, 0.25, 0.15];
        let text_on = [0.90, 0.90, 0.95, 1.0];
        let busy = state.busy();

        let query = if ctx[state.search_box].editing {
            ctx[state.search_box].edit_buffer.to_lowercase()
        } else {
            ctx[state.search_box].text.to_lowercase()
        };
        // What the list shows, worked out before the widgets are lent to the form.
        let visible_installed = state.visible_installed(ctx);
        if state.installed_items.len() != visible_installed.len() {
            for h in state.installed_items.drain(..) {
                ctx.remove(h);
            }
            for _ in 0..visible_installed.len() {
                state.installed_items.push(ctx.insert(InteractiveListItem::new("")));
            }
        }
        let filtered_updates: Vec<&UpdateInfo> = state.updates.iter().filter(|p| p.name.to_lowercase().contains(&query)).collect();
        if state.updates_items.len() != filtered_updates.len() {
            for h in state.updates_items.drain(..) {
                ctx.remove(h);
            }
            for _ in 0..filtered_updates.len() {
                state.updates_items.push(ctx.insert(InteractiveListItem::new("")));
            }
        }

        let mut col = form.column();

        // ── Tabs (with counts) ──
        let label1 = format!("Installed ({})", state.installed.len());
        let label2 = format!("Updates ({})", state.updates.len());
        let tab = state.active_tab;
        col.row(|r| {
            form_button(r, label1, 0.0, (if tab == PackageTab::Installed { active_bg } else { inactive_bg }, hover_bg, text_on),
                AppAction::Packages(PackagesMessage::SetTab(PackageTab::Installed)));
            form_button(r, label2, 0.0, (if tab == PackageTab::Updates { active_bg } else { inactive_bg }, hover_bg, text_on),
                AppAction::Packages(PackagesMessage::SetTab(PackageTab::Updates)));
        });

        col.widget_h(ctx, state.search_box, cce_ui::layout::textbox_height());

        // ── Update System: a button as wide as its label, its status beside it ──
        let status_line = if state.updating {
            "Updating system...".to_string()
        } else if state.updates.is_empty() {
            "System is up to date".to_string()
        } else {
            format!("{} updates available", state.updates.len())
        };
        let status_color = if state.updating || !state.updates.is_empty() { ACCENT } else { TEXT_DIM };
        let (btn_lbl, bg, hover) = if state.updating {
            ("Updating...", [0.15, 0.15, 0.20, 1.0], [0.15, 0.15, 0.20, 1.0])
        } else {
            ("Update System", [0.13, 0.18, 0.14, 1.0], [0.25, 0.30, 0.26, 1.0])
        };
        col.row(|r| {
            form_button(r, btn_lbl, button_need(btn_lbl), (bg, hover, text_on), AppAction::Packages(PackagesMessage::StartUpdate));
            r.text_fill(status_line, 12.0, status_color);
        });

        if let Some(ref res) = state.last_update_res {
            match res {
                Ok(_) => {
                    col.text("Last update succeeded", 12.0, [0.56, 0.83, 0.56, 1.0]);
                }
                Err(err) => {
                    col.block(|b| {
                        b.text("Last update failed:", 12.0, RED);
                        b.lines(wrap_to_width(err, wrap_w, 11.0), 11.0, RED);
                    });
                }
            }
        }

        // ── Filter row: All / Explicit / Orphans, and the select-mode switch ──
        if state.active_tab == PackageTab::Installed {
            let n_explicit = state.installed.iter().filter(|p| p.explicit).count();
            let n_orphans = state.installed.iter().filter(|p| p.orphan).count();
            let filters = [
                (InstalledFilter::All, format!("All ({})", state.installed.len())),
                (InstalledFilter::Explicit, format!("Explicit ({n_explicit})")),
                (InstalledFilter::Orphans, format!("Orphans ({n_orphans})")),
            ];
            let select_label = if state.select_mode { "Done Selecting" } else { "Select..." };
            let filter = state.filter;
            let select_mode = state.select_mode;
            col.row(|r| {
                for (f, label) in filters {
                    let face = if filter == f { active_bg } else { inactive_bg };
                    form_button(r, label, 0.0, (face, hover_bg, text_on), AppAction::Packages(PackagesMessage::SetFilter(f)));
                }
                let face = if select_mode { active_bg } else { inactive_bg };
                form_button(r, select_label, 0.0, (face, hover_bg, text_on), AppAction::Packages(PackagesMessage::ToggleSelectMode));
            });

            // ── Bulk actions over the checked rows ──
            if state.select_mode {
                let n = state.checked.len();
                let summary = if n == 0 { "Click rows to select".to_string() } else { format!("{} selected", count_noun(n)) };
                let targets: Vec<String> = state.checked.iter().cloned().collect();
                let can_act = n > 0 && !busy;
                let act_text = if can_act { TEXT_FG } else { TEXT_DIM };
                let mark_label = if state.marking { "Marking..." } else { "Mark Explicit" };
                col.row(|r| {
                    r.text_fill(summary, 12.0, if n > 0 { ACCENT } else { TEXT_DIM });
                    form_button(r, "Select Visible", button_need("Select Visible"), (TOGGLE_OFF, BTN_HOVER, TEXT_FG),
                        AppAction::Packages(PackagesMessage::CheckAllVisible));
                    form_button(r, "Clear", button_need("Clear"), (TOGGLE_OFF, BTN_HOVER, TEXT_FG),
                        AppAction::Packages(PackagesMessage::ClearChecked));
                    form_button(r, mark_label, button_need(mark_label),
                        (TOGGLE_OFF, if can_act { BTN_HOVER } else { TOGGLE_OFF }, act_text),
                        AppAction::Packages(PackagesMessage::SetInstallReason(targets.clone(), true)));
                    form_button(r, "Remove...", button_need("Remove..."),
                        (
                            if can_act { [0.25, 0.14, 0.14, 1.0] } else { TOGGLE_OFF },
                            if can_act { [0.40, 0.20, 0.20, 1.0] } else { TOGGLE_OFF },
                            if can_act { [0.95, 0.55, 0.55, 1.0] } else { TEXT_DIM },
                        ),
                        AppAction::Packages(PackagesMessage::PreviewRemoval(targets)));
                });
            }
        }

        // ── Removal confirmation: what pacman says it would take ──
        if state.previewing {
            col.text("Checking what would be removed...", 12.0, TEXT_DIM);
        } else if let Some(plan) = state.removal.clone() {
            form_divider(&mut col);
            match &plan.outcome {
                Ok(pkgs) => {
                    let total: u64 = pkgs.iter().map(|(_, s)| *s).sum();
                    let extra = pkgs.len().saturating_sub(plan.targets.len());
                    let head = if extra > 0 {
                        format!("Remove {} ({}), including {} no longer needed:",
                            count_noun(pkgs.len()), human_size(total),
                            if extra == 1 { "1 dependency".to_string() } else { format!("{extra} dependencies") })
                    } else {
                        format!("Remove {} ({}):", count_noun(pkgs.len()), human_size(total))
                    };
                    let names = pkgs.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>().join(", ");
                    let lines = wrap_to_width(&names, wrap_w, 11.0);
                    const MAX_LINES: usize = 6;
                    col.block(|b| {
                        b.text(head, 12.0, [0.95, 0.75, 0.55, 1.0]);
                        b.lines(lines.iter().take(MAX_LINES).cloned().collect(), 11.0, TEXT_FG);
                        if lines.len() > MAX_LINES {
                            let shown: usize = lines.iter().take(MAX_LINES).map(|l| l.split_whitespace().count()).sum();
                            b.text(format!("... and {} more", pkgs.len().saturating_sub(shown)), 11.0, TEXT_DIM);
                        }
                    });
                    let confirm = if state.uninstalling { "Removing...".to_string() } else { format!("Remove {}", count_noun(pkgs.len())) };
                    let targets = plan.targets.clone();
                    col.row(|r| {
                        let need = button_need(&confirm);
                        form_button(r, confirm, need, ([0.30, 0.14, 0.14, 1.0], [0.45, 0.20, 0.20, 1.0], [0.98, 0.65, 0.65, 1.0]),
                            AppAction::Packages(PackagesMessage::StartUninstall(targets)));
                        form_button(r, "Cancel", button_need("Cancel"), (TOGGLE_OFF, BTN_HOVER, TEXT_FG),
                            AppAction::Packages(PackagesMessage::CancelRemoval));
                    });
                }
                Err(err) => {
                    let lines: Vec<String> = err.lines().flat_map(|l| wrap_to_width(l, wrap_w, 11.0)).take(8).collect();
                    col.block(|b| {
                        b.text(format!("Can't remove {}:", plan.targets.join(", ")), 12.0, RED);
                        b.lines(lines, 11.0, [0.95, 0.55, 0.55, 1.0]);
                    });
                    col.row(|r| {
                        form_button(r, "Dismiss", button_need("Dismiss"), (TOGGLE_OFF, BTN_HOVER, TEXT_FG),
                            AppAction::Packages(PackagesMessage::CancelRemoval));
                    });
                }
            }
        }

        // ── Outcome of the last remove / mark ──
        if let Some(ref res) = state.last_action {
            match res {
                Ok(msg) => {
                    col.text(msg.clone(), 12.0, [0.56, 0.83, 0.56, 1.0]);
                }
                Err(err) => {
                    let lines: Vec<String> = err.lines().flat_map(|l| wrap_to_width(l, wrap_w, 11.0)).take(8).collect();
                    col.lines(lines, 11.0, RED);
                }
            }
        }

        // ── Selected package details ──
        // Set aside while a removal is being confirmed: the plan names the
        // package, and both together push the list off the page.
        let confirming = state.previewing || state.removal.is_some();
        if !confirming && (state.loading_info || state.selected_package.is_some()) {
            form_divider(&mut col);
        }
        if confirming {
            // Nothing: the confirmation above stands in for the details.
        } else if state.loading_info {
            col.text("Loading package details...", 12.0, TEXT_DIM);
        } else if let Some(pkg_name) = state.selected_package.clone() {
            if let Some(info_raw) = state.selected_package_info.clone() {
                let mut parsed = parse_package_info(&info_raw);
                if parsed.name.is_empty() {
                    parsed.name = pkg_name.clone();
                }

                // Heading row: the name, then the install-reason flip and Uninstall at the
                // row's end. An explicit package can be handed back as a dependency (so it
                // goes when nothing needs it) and a dependency kept for good.
                let installed = state.installed.iter().find(|p| p.name == pkg_name).map(|p| p.explicit);
                let on_installed = state.active_tab == PackageTab::Installed;
                col.row(|r| {
                    r.text_fill(parsed.name.clone(), 14.0, [0.35, 0.65, 0.90, 1.0]);
                    if let (true, Some(explicit)) = (on_installed, installed) {
                        let (mark_lbl, to_explicit) = if explicit { ("Mark as Dependency", false) } else { ("Mark Explicit", true) };
                        form_button(r, mark_lbl, button_need(mark_lbl),
                            (TOGGLE_OFF, if busy { TOGGLE_OFF } else { BTN_HOVER }, if busy { TEXT_DIM } else { TEXT_FG }),
                            AppAction::Packages(PackagesMessage::SetInstallReason(vec![pkg_name.clone()], to_explicit)));
                        let colors = if busy {
                            (TOGGLE_OFF, TOGGLE_OFF, TEXT_DIM)
                        } else {
                            ([0.25, 0.14, 0.14, 1.0], [0.40, 0.20, 0.20, 1.0], [0.95, 0.55, 0.55, 1.0])
                        };
                        form_button(r, "Uninstall", button_need("Uninstall"), colors,
                            AppAction::Packages(PackagesMessage::PreviewRemoval(vec![pkg_name.clone()])));
                    }
                });

                let orphan = state.installed.iter().any(|p| p.name == pkg_name && p.orphan);
                let reason = if orphan {
                    format!("{} (orphan: nothing requires it now)", parsed.install_reason)
                } else {
                    parsed.install_reason.clone()
                };
                let pairs = [
                    ("Version", parsed.version.clone()),
                    ("Install Reason", reason),
                    ("Size", parsed.size.clone()),
                    ("Licenses", parsed.licenses.clone()),
                    ("Website", parsed.website.clone()),
                    ("Packager", parsed.packager.clone()),
                    ("Built", parsed.build_date.clone()),
                    ("Description", parsed.description.clone()),
                    ("Commands", parsed.commands.clone()),
                ]
                .into_iter()
                .filter(|(_, v)| !v.is_empty())
                .map(|(k, v)| (k.to_string(), TEXT_DIM, v, TEXT_FG))
                .collect();
                crate::app::form_pairs(&mut col, 11.0, pairs);

                if !parsed.required_by.is_empty() && parsed.required_by != "None" {
                    let reqs: Vec<String> = parsed.required_by.split_whitespace().map(str::to_string).collect();
                    const COLUMNS: usize = 4;
                    // A core library is required by hundreds (glibc: ~300). Every
                    // row of buttons pushes the list down, and past the page's
                    // bottom the list loses its box; a few rows say enough.
                    const MAX_REQ_ROWS: usize = 4;
                    let shown = reqs.len().min(COLUMNS * MAX_REQ_ROWS);
                    col.text("Required By", 11.0, TEXT_DIM);
                    for chunk in reqs[..shown].chunks(COLUMNS) {
                        col.row(|r| {
                            for pkg in chunk {
                                let action = AppAction::Packages(PackagesMessage::SelectAndScrollPackage(pkg.clone()));
                                form_button(r, pkg.clone(), 0.0, (TOGGLE_OFF, BTN_HOVER, TEXT_FG), action);
                            }
                            // A short last row keeps the columns of the rows above it.
                            for _ in chunk.len()..COLUMNS {
                                r.space(0.0, true);
                            }
                        });
                    }
                    if reqs.len() > shown {
                        col.text(format!("... and {} more", reqs.len() - shown), 11.0, TEXT_DIM);
                    }
                }
            } else {
                col.text("No details available.", 12.0, TEXT_DIM);
            }
        }

        form_divider(&mut col);

        // ── The list fills the rest of the page ──
        let tab = state.active_tab;
        let empty_msg = if state.filter == InstalledFilter::Orphans && query.is_empty() {
            "No orphaned packages"
        } else {
            "No packages match the query"
        };
        let select_mode = state.select_mode;
        let checked = &state.checked;
        let selected = state.selected_package.clone();
        let installed = &state.installed;
        let (installed_list, installed_items) = (&mut state.installed_list, &state.installed_items);
        let (updates_list, updates_items) = (&mut state.updates_list, &state.updates_items);
        col.fill(LIST_MIN_H, move |pc, rect, ctx| {
            let (list_box_x, list_box_y, list_box_w, list_box_h) = (rect.x, rect.y, rect.width, rect.height);
            let inset = cce_ui::layout::plate_padding();
            match tab {
                PackageTab::Installed => {
                    let filtered: Vec<&PackageInfo> = visible_installed.iter().map(|&i| &installed[i]).collect();
                    // Dissolved List (Phase 6v): scroll state + frame prims are app-owned.
                    installed_list.set_rect(list_box_x, list_box_y, list_box_w, list_box_h);
                    installed_list.update_bounds(filtered.len(), list_box_y, list_box_h);
                    installed_list.push_prims(pc);
                    let item_h = installed_list.item_height;

                    pc.push_clip_rect(list_box_x, list_box_y, list_box_w, list_box_h);
                    for (idx, pkg) in filtered.iter().enumerate() {
                        if let Some(draw_y) = installed_list.get_item_draw_y(idx, 4.0) {
                            // Rows dispatch as extra roots (the dissolved list is no parent).
                            let row = installed_items[idx];
                            let item = &mut ctx[row];
                            item.title = pkg.name.clone();
                            let reason = match (pkg.explicit, pkg.orphan) {
                                (true, _) => "explicit",
                                (false, true) => "orphan",
                                (false, false) => "dependency",
                            };
                            item.subtitle = Some(format!("{}  ·  {}", pkg.version, reason));
                            item.selected = if select_mode {
                                checked.contains(&pkg.name)
                            } else {
                                Some(&pkg.name) == selected.as_ref()
                            };
                            let cell = lay_row(Rect { x: list_box_x, y: draw_y, width: list_box_w, height: item_h }, &[Cell::grow(item_h)])[0];
                            render_widget_h(pc, row, cell.x, cell.y, cell.width, cell.height, ctx);
                        }
                    }
                    pc.pop_clip_rect();
                    // The scrollbar's fore copy, over the rows at the raise's fade.
                    installed_list.push_scrollbar_fore(pc);
                    if filtered.is_empty() {
                        pc.text(empty_msg, list_box_x + inset, list_box_y + inset, 12.0, TEXT_DIM);
                    }
                }
                PackageTab::Updates => {
                    // Dissolved List (Phase 6v): scroll state + frame prims are app-owned.
                    updates_list.set_rect(list_box_x, list_box_y, list_box_w, list_box_h);
                    updates_list.update_bounds(filtered_updates.len(), list_box_y, list_box_h);
                    updates_list.push_prims(pc);
                    let item_h = updates_list.item_height;

                    pc.push_clip_rect(list_box_x, list_box_y, list_box_w, list_box_h);
                    for (idx, pkg) in filtered_updates.iter().enumerate() {
                        if let Some(draw_y) = updates_list.get_item_draw_y(idx, 4.0) {
                            // Rows dispatch as extra roots (the dissolved list is no parent).
                            let row = updates_items[idx];
                            let item = &mut ctx[row];
                            item.title = pkg.name.clone();
                            item.subtitle = Some(format!("{}  ->  {}", pkg.old_version, pkg.new_version));
                            item.selected = Some(&pkg.name) == selected.as_ref();
                            let cell = lay_row(Rect { x: list_box_x, y: draw_y, width: list_box_w, height: item_h }, &[Cell::grow(item_h)])[0];
                            render_widget_h(pc, row, cell.x, cell.y, cell.width, cell.height, ctx);
                        }
                    }
                    pc.pop_clip_rect();
                    // The scrollbar's fore copy, over the rows at the raise's fade.
                    updates_list.push_scrollbar_fore(pc);
                    if filtered_updates.is_empty() {
                        pc.text("No updates match the query", list_box_x + inset, list_box_y + inset, 12.0, TEXT_DIM);
                    }
                }
            }
        });
        sec.place(form, ctx);
    });

    final_pc
}

pub fn update(state: &mut PackagesState, msg: PackagesMessage, ctx: &mut UiContext) {
    match msg {
        PackagesMessage::Refreshed(new) => {
            state.loaded = new.loaded;
            state.installed = new.installed;
            state.updates = new.updates;
            // Selection, fetched info, and in-flight update/uninstall flags are
            // LOCAL state — the periodic background refresh must not clear them.
            // Only drop a selection whose package no longer exists anywhere.
            let installed = &state.installed;
            state.checked.retain(|n| installed.iter().any(|p| &p.name == n));
            if let Some(sel) = state.selected_package.clone() {
                let still_exists = state.installed.iter().any(|p| p.name == sel)
                    || state.updates.iter().any(|u| u.name == sel);
                if !still_exists {
                    state.selected_package = None;
                    state.selected_package_info = None;
                    state.loading_info = false;
                }
            }
        }
        PackagesMessage::SetTab(tab) => {
            state.active_tab = tab;
            state.installed_list.set_scroll_y(0.0);
            state.updates_list.set_scroll_y(0.0);
            for h in state.installed_items.drain(..) {
                ctx.remove(h);
            }
            for h in state.updates_items.drain(..) {
                ctx.remove(h);
            }
            state.selected_package = None;
            state.selected_package_info = None;
            state.loading_info = false;
        }
        PackagesMessage::StartUpdate => {
            state.updating = true;
            state.last_update_res = None;
        }
        PackagesMessage::UpdateFinished(res) => {
            state.updating = false;
            state.last_update_res = Some(res);
        }
        PackagesMessage::SelectPackage(name) => {
            if name != state.selected_package {
                state.selected_package = name;
                state.selected_package_info = None;
                state.loading_info = state.selected_package.is_some();
            }
        }
        PackagesMessage::InfoFetched(name, res) => {
            if state.selected_package.as_ref() == Some(&name) {
                state.loading_info = false;
                match res {
                    Ok(info) => {
                        state.selected_package_info = Some(info);
                    }
                    Err(err) => {
                        state.selected_package_info = Some(format!("Error loading package info: {}", err));
                    }
                }
            }
        }
        PackagesMessage::SetFilter(filter) => {
            if state.filter != filter {
                state.filter = filter;
                state.installed_list.set_scroll_y(0.0);
                for h in state.installed_items.drain(..) {
                    ctx.remove(h);
                }
            }
        }
        PackagesMessage::ToggleSelectMode => {
            state.select_mode = !state.select_mode;
            if state.select_mode {
                // The details pane belongs to single selection; free its space.
                state.selected_package = None;
                state.selected_package_info = None;
                state.loading_info = false;
            } else {
                state.checked.clear();
            }
        }
        PackagesMessage::ToggleChecked(name) => {
            if !state.checked.remove(&name) {
                state.checked.insert(name);
            }
        }
        PackagesMessage::CheckAllVisible => {
            for i in state.visible_installed(ctx) {
                state.checked.insert(state.installed[i].name.clone());
            }
        }
        PackagesMessage::ClearChecked => state.checked.clear(),
        PackagesMessage::PreviewRemoval(_) => {
            state.previewing = true;
            state.removal = None;
            state.last_action = None;
        }
        PackagesMessage::RemovalPreviewed(targets, outcome) => {
            state.previewing = false;
            state.removal = Some(RemovalPlan { targets, outcome });
        }
        PackagesMessage::CancelRemoval => {
            state.removal = None;
            state.previewing = false;
        }
        PackagesMessage::StartUninstall(_) => {
            state.uninstalling = true;
            state.last_action = None;
        }
        PackagesMessage::UninstallFinished(targets, res) => {
            state.uninstalling = false;
            let removed = state.removal.as_ref()
                .filter(|p| p.targets == targets)
                .and_then(|p| p.outcome.as_ref().ok())
                .map(|pkgs| pkgs.len())
                .unwrap_or(targets.len());
            state.removal = None;
            match res {
                Ok(()) => {
                    for t in &targets {
                        state.checked.remove(t);
                    }
                    if state.selected_package.as_ref().is_some_and(|s| targets.contains(s)) {
                        state.selected_package = None;
                        state.selected_package_info = None;
                    }
                    state.last_action = Some(Ok(format!("Removed {}", count_noun(removed))));
                }
                Err(err) => {
                    state.last_action = Some(Err(format!("Removal failed: {err}")));
                }
            }
        }
        PackagesMessage::SetInstallReason(_, _) => {
            state.marking = true;
            state.last_action = None;
        }
        PackagesMessage::InstallReasonSet(targets, explicit, res) => {
            state.marking = false;
            let what = if explicit { "explicitly installed" } else { "dependencies" };
            state.last_action = Some(match res {
                Ok(()) => {
                    // Reflect it now; the refresh that follows confirms it (and
                    // recomputes orphans, which a reason change can create).
                    for p in state.installed.iter_mut().filter(|p| targets.contains(&p.name)) {
                        p.explicit = explicit;
                        if explicit {
                            p.orphan = false;
                        }
                    }
                    if targets.len() == 1 {
                        Ok(format!("Marked {} as {}", targets[0], if explicit { "explicitly installed" } else { "a dependency" }))
                    } else {
                        Ok(format!("Marked {} as {}", count_noun(targets.len()), what))
                    }
                }
                Err(err) => Err(format!("Marking failed: {err}")),
            });
        }
        PackagesMessage::SelectAndScrollPackage(name) => {
            state.select_and_scroll_to(&name, ctx);
        }
    }
}

impl PackagesState {
    /// A pacman transaction (or pkexec prompt) is in flight. pacman holds one
    /// database lock, so a second one started now would only fail.
    pub fn busy(&self) -> bool {
        self.updating || self.uninstalling || self.marking
    }

    /// The search query as typed so far (the box commits only on Enter).
    fn query(&self, ctx: &UiContext) -> String {
        if ctx[self.search_box].editing {
            ctx[self.search_box].edit_buffer.to_lowercase()
        } else {
            ctx[self.search_box].text.to_lowercase()
        }
    }

    /// Indices into `installed` of the rows the Installed tab shows, in order:
    /// the filter, then the search. The view paints exactly these and the click
    /// mapping resolves against them, so the two can never disagree.
    pub fn visible_installed(&self, ctx: &UiContext) -> Vec<usize> {
        let query = self.query(ctx);
        self.installed
            .iter()
            .enumerate()
            .filter(|(_, p)| self.filter.admits(p))
            .filter(|(_, p)| p.name.to_lowercase().contains(&query) || p.version.to_lowercase().contains(&query))
            .map(|(i, _)| i)
            .collect()
    }

    pub fn select_and_scroll_to(&mut self, pkg_name: &str, ctx: &mut UiContext) {
        self.active_tab = PackageTab::Installed;
        // The scroll target below is an index into the whole list, so the
        // list must be unfiltered for it to land on the package.
        self.filter = InstalledFilter::All;
        self.select_mode = false;
        self.checked.clear();
        for h in self.installed_items.drain(..) {
            ctx.remove(h);
        }
        ctx[self.search_box].text.clear();
        ctx[self.search_box].edit_buffer.clear();
        ctx[self.search_box].editing = false;
        self.selected_package = Some(pkg_name.to_string());
        self.selected_package_info = None;
        self.loading_info = true;
        if let Some(idx) = self.installed.iter().position(|p| p.name == pkg_name) {
            let item_height_full = self.installed_list.item_height + self.installed_list.item_gap;
            let target_y = idx as f32 * item_height_full - 164.0;
            self.installed_list.set_scroll_y(target_y);
            self.installed_list.notify_scrolled();
        }
    }
}

impl PackagesState {
    /// The active tab's dissolved list region (only one is laid out per frame).
    fn active_list(&mut self) -> &mut ScrollRegion {
        match self.active_tab {
            PackageTab::Installed => &mut self.installed_list,
            PackageTab::Updates => &mut self.updates_list,
        }
    }
}

impl crate::pages::AppPage for PackagesState {
    // Sections: [the one well]
    fn section_widgets(&mut self) -> Vec<Vec<cce_ui::widget::WidgetId>> {
        // Mirrors the view's `!loaded` early return: the search box is only painted
        // (and so only registered) once `pacman -Q` + `checkupdates` land, which is
        // the longest load window of any page. The group count stays 1 either way —
        // an empty outer Vec would kill the ctrl-nav entry point.
        if self.loaded {
            vec![vec![self.search_box.id()]]
        } else {
            vec![Vec::new()]
        }
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
    ) -> crate::app::PageContent {
        view(self, cx, cy, cw, ch, sec_focused, layout, ctx)
    }

    fn propagate_widget_changes(&mut self, actions: &mut Vec<crate::app::AppAction>, ctx: &mut UiContext) {
        let query = if ctx[self.search_box].editing {
            ctx[self.search_box].edit_buffer.to_lowercase()
        } else {
            ctx[self.search_box].text.to_lowercase()
        };

        match self.active_tab {
            PackageTab::Installed => {
                let visible = self.visible_installed(ctx);
                for (idx, &row) in self.installed_items.iter().enumerate() {
                    let item = &mut ctx[row];
                    if item.just_clicked {
                        item.just_clicked = false;
                        if let Some(&i) = visible.get(idx) {
                            let name = self.installed[i].name.clone();
                            actions.push(AppAction::Packages(if self.select_mode {
                                PackagesMessage::ToggleChecked(name)
                            } else {
                                PackagesMessage::SelectPackage(Some(name))
                            }));
                        }
                    }
                }
            }
            PackageTab::Updates => {
                let filtered: Vec<&UpdateInfo> = self.updates.iter()
                    .filter(|p| p.name.to_lowercase().contains(&query))
                    .collect();
                for (idx, &row) in self.updates_items.iter().enumerate() {
                    let item = &mut ctx[row];
                    if item.just_clicked {
                        item.just_clicked = false;
                        if idx < filtered.len() {
                            let pkg = filtered[idx];
                            actions.push(AppAction::Packages(PackagesMessage::SelectPackage(Some(pkg.name.clone()))));
                        }
                    }
                }
            }
        }
    }

    // Both root methods below filter by `get_item_draw_y`, the SAME predicate the view's
    // paint loop uses to virtualize rows. Only a row that was drawn had `render_widget`
    // refresh its rect; a scrolled-out row keeps the rect from the last frame it was
    // visible, and since `dispatch_page_event` takes the first root that hit-tests true
    // in index order, an unfiltered list let a stale low-index row swallow clicks meant
    // for the row actually on screen (click "fontforge", select "appstream").
    fn extra_dispatch_roots(&mut self) -> Vec<cce_ui::widget::WidgetId> {
        let (list, items) = match self.active_tab {
            PackageTab::Installed => (&self.installed_list, &self.installed_items),
            PackageTab::Updates => (&self.updates_list, &self.updates_items),
        };
        items
            .iter()
            .enumerate()
            .filter(|(idx, _)| list.get_item_draw_y(*idx, 4.0).is_some())
            .map(|(_, i)| i.id())
            .collect()
    }


    fn handle_pointer_move(
        &mut self,
        lx: f32,
        ly: f32,
        _actions: &mut Vec<crate::app::AppAction>,
        _ctx: &mut cce_ui::context::UiContext,
    ) -> bool {
        self.loaded && self.active_list().cursor_moved(lx, ly)
    }

    fn handle_pointer_down(&mut self, lx: f32, ly: f32, _ctx: &mut cce_ui::context::UiContext) -> bool {
        self.loaded && self.active_list().press(lx, ly)
    }

    fn handle_pointer_up(&mut self, _ctx: &mut cce_ui::context::UiContext) -> bool {
        self.active_list().release()
    }

    fn handle_mouse_wheel(&mut self, delta: &cce_ui::widget::MouseScrollDelta, lx: f32, ly: f32) -> bool {
        self.loaded && self.active_list().wheel(delta, lx, ly)
    }

    fn handle_key_input(&mut self, event: &cce_ui::widget::KeyEvent) -> bool {
        self.loaded && self.active_list().keyboard(event)
    }

    fn tick(&mut self, dt: f32) -> bool {
        self.installed_list.tick(dt) | self.updates_list.tick(dt)
    }

    /// Only the active tab's list is laid out and drawn.
    fn frameless_lists(&self) -> Vec<&ScrollRegion> {
        vec![match self.active_tab {
            PackageTab::Installed => &self.installed_list,
            PackageTab::Updates => &self.updates_list,
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refresh_preserves_selection_and_flags() {
        let mut ui = cce_ui::context::UiContext::new();
        let mut st = PackagesState::new(&mut ui);
        st.loaded = true;
        st.installed = vec![PackageInfo { name: "foo".into(), version: "1".into(), ..Default::default() }];
        st.selected_package = Some("foo".into());
        st.selected_package_info = Some("info".into());
        st.updating = true;

        let fresh = PackagesState {
            loaded: true,
            installed: vec![PackageInfo { name: "foo".into(), version: "2".into(), ..Default::default() }],
            ..Default::default()
        };
        update(&mut st, PackagesMessage::Refreshed(Box::new(fresh)), &mut ui);
        assert_eq!(st.selected_package.as_deref(), Some("foo"), "refresh must keep the selection");
        assert!(st.selected_package_info.is_some(), "refresh must keep fetched info");
        assert!(st.updating, "refresh must not clear the in-flight update flag");

        // A package that vanished from both lists does clear the selection.
        let fresh2 = PackagesState { loaded: true, ..Default::default() };
        update(&mut st, PackagesMessage::Refreshed(Box::new(fresh2)), &mut ui);
        assert!(st.selected_package.is_none());
        assert!(st.selected_package_info.is_none());
    }

    fn pkg(name: &str, explicit: bool, orphan: bool) -> PackageInfo {
        PackageInfo { name: name.into(), version: "1".into(), explicit, orphan }
    }

    fn loaded_state(ui: &mut UiContext) -> PackagesState {
        let mut st = PackagesState::new(ui);
        st.loaded = true;
        st.installed = vec![
            pkg("alpha", true, false),
            pkg("beta", false, true),
            pkg("gamma", false, false),
            pkg("gamma-orphan", false, true),
        ];
        st
    }

    #[test]
    fn parse_installed_joins_reason_and_orphan_lists() {
        let list = parse_installed("zed 1.0\nRust 2.0\ncmake 3.0\n", "zed\n", "cmake\n");
        let names: Vec<_> = list.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["cmake", "Rust", "zed"], "sorted case-insensitively");
        assert!(list[2].explicit && !list[2].orphan);
        assert!(!list[1].explicit && !list[1].orphan);
        assert!(!list[0].explicit && list[0].orphan);
    }

    #[test]
    fn filter_and_search_compose() {
        let mut ui = cce_ui::context::UiContext::new();
        let mut st = loaded_state(&mut ui);
        assert_eq!(st.visible_installed(&ui).len(), 4);
        update(&mut st, PackagesMessage::SetFilter(InstalledFilter::Orphans), &mut ui);
        assert_eq!(st.visible_installed(&ui), vec![1, 3]);
        update(&mut st, PackagesMessage::SetFilter(InstalledFilter::Explicit), &mut ui);
        assert_eq!(st.visible_installed(&ui), vec![0]);
        update(&mut st, PackagesMessage::SetFilter(InstalledFilter::Orphans), &mut ui);
        ui[st.search_box].text = "gamma".into();
        assert_eq!(st.visible_installed(&ui), vec![3]);
    }

    /// A click on visible row N must name the package painted at row N, under
    /// a filter too — the view and the click mapping share `visible_installed`.
    #[test]
    fn row_click_resolves_through_the_filter() {
        let mut ui = cce_ui::context::UiContext::new();
        let mut st = loaded_state(&mut ui);
        update(&mut st, PackagesMessage::SetFilter(InstalledFilter::Orphans), &mut ui);
        let mut layout = cce_ui::compose::PageFlow::new();
        view(&mut st, 10.0, 20.0, 800.0, 600.0, &[false], &mut layout, &mut ui);
        assert_eq!(st.installed_items.len(), 2);
        ui[st.installed_items[1]].just_clicked = true;
        let mut actions = Vec::new();
        crate::pages::AppPage::propagate_widget_changes(&mut st, &mut actions, &mut ui);
        match actions.as_slice() {
            [AppAction::Packages(PackagesMessage::SelectPackage(Some(n)))] => assert_eq!(n, "gamma-orphan"),
            other => panic!("unexpected actions: {other:?}"),
        }

        // In select mode the same click toggles the row instead.
        update(&mut st, PackagesMessage::ToggleSelectMode, &mut ui);
        ui[st.installed_items[1]].just_clicked = true;
        let mut actions = Vec::new();
        crate::pages::AppPage::propagate_widget_changes(&mut st, &mut actions, &mut ui);
        match actions.as_slice() {
            [AppAction::Packages(PackagesMessage::ToggleChecked(n))] => assert_eq!(n, "gamma-orphan"),
            other => panic!("unexpected actions: {other:?}"),
        }
    }

    #[test]
    fn select_mode_checks_and_clears() {
        let mut ui = cce_ui::context::UiContext::new();
        let mut st = loaded_state(&mut ui);
        st.selected_package = Some("alpha".into());
        update(&mut st, PackagesMessage::ToggleSelectMode, &mut ui);
        assert!(st.selected_package.is_none(), "select mode frees the details pane");
        update(&mut st, PackagesMessage::ToggleChecked("beta".into()), &mut ui);
        update(&mut st, PackagesMessage::ToggleChecked("beta".into()), &mut ui);
        assert!(st.checked.is_empty(), "a second click unchecks");
        update(&mut st, PackagesMessage::SetFilter(InstalledFilter::Orphans), &mut ui);
        update(&mut st, PackagesMessage::CheckAllVisible, &mut ui);
        assert_eq!(st.checked.iter().cloned().collect::<Vec<_>>(), ["beta", "gamma-orphan"]);
        // A refresh drops checked names that are gone.
        let fresh = PackagesState { loaded: true, installed: vec![pkg("beta", false, true)], ..Default::default() };
        update(&mut st, PackagesMessage::Refreshed(Box::new(fresh)), &mut ui);
        assert_eq!(st.checked.iter().cloned().collect::<Vec<_>>(), ["beta"]);
        update(&mut st, PackagesMessage::ToggleSelectMode, &mut ui);
        assert!(st.checked.is_empty(), "leaving select mode clears the selection");
    }

    /// The failure that prompted this page's rework: a removal pacman refuses
    /// (a dependency still requires it) used to land in the details text,
    /// where the info parser found no known key and showed nothing at all.
    #[test]
    fn refused_removal_stays_visible() {
        let mut ui = cce_ui::context::UiContext::new();
        let mut st = loaded_state(&mut ui);
        let err = "error: failed to prepare transaction (could not satisfy dependencies)\n\
                   :: removing beta breaks dependency 'beta' required by alpha".to_string();
        update(&mut st, PackagesMessage::PreviewRemoval(vec!["beta".into()]), &mut ui);
        assert!(st.previewing);
        update(&mut st, PackagesMessage::RemovalPreviewed(vec!["beta".into()], Err(err.clone())), &mut ui);
        assert!(!st.previewing);
        let mut layout = cce_ui::compose::PageFlow::new();
        let pc = view(&mut st, 10.0, 20.0, 800.0, 600.0, &[false], &mut layout, &mut ui);
        let texts: Vec<String> = pc.texts.iter().map(|t| t.0.clone()).collect();
        assert!(texts.iter().any(|t| t.contains("required by alpha")), "the reason is painted: {texts:?}");

        // And a failed transaction reports into last_action, not the details.
        update(&mut st, PackagesMessage::StartUninstall(vec!["beta".into()]), &mut ui);
        update(&mut st, PackagesMessage::UninstallFinished(vec!["beta".into()], Err(err)), &mut ui);
        assert!(!st.uninstalling);
        assert!(matches!(&st.last_action, Some(Err(e)) if e.contains("required by alpha")));
    }

    #[test]
    fn successful_removal_reports_the_whole_plan() {
        let mut ui = cce_ui::context::UiContext::new();
        let mut st = loaded_state(&mut ui);
        st.checked.insert("beta".into());
        update(&mut st, PackagesMessage::RemovalPreviewed(
            vec!["beta".into()],
            Ok(parse_removal_preview("beta 1024\nlibbeta 2048\n")),
        ), &mut ui);
        update(&mut st, PackagesMessage::StartUninstall(vec!["beta".into()]), &mut ui);
        update(&mut st, PackagesMessage::UninstallFinished(vec!["beta".into()], Ok(())), &mut ui);
        assert!(st.removal.is_none());
        assert!(st.checked.is_empty());
        assert!(matches!(&st.last_action, Some(Ok(m)) if m == "Removed 2 packages"));
    }

    #[test]
    fn marking_explicit_updates_rows_at_once() {
        let mut ui = cce_ui::context::UiContext::new();
        let mut st = loaded_state(&mut ui);
        update(&mut st, PackagesMessage::SetInstallReason(vec!["beta".into()], true), &mut ui);
        assert!(st.busy());
        update(&mut st, PackagesMessage::InstallReasonSet(vec!["beta".into()], true, Ok(())), &mut ui);
        assert!(!st.busy());
        assert!(st.installed[1].explicit && !st.installed[1].orphan);
        assert!(matches!(&st.last_action, Some(Ok(m)) if m == "Marked beta as explicitly installed"));
    }

    #[test]
    fn jump_to_package_clears_the_filter() {
        let mut ui = cce_ui::context::UiContext::new();
        let mut st = loaded_state(&mut ui);
        update(&mut st, PackagesMessage::SetFilter(InstalledFilter::Orphans), &mut ui);
        update(&mut st, PackagesMessage::SelectAndScrollPackage("alpha".into()), &mut ui);
        assert_eq!(st.filter, InstalledFilter::All);
        assert_eq!(st.selected_package.as_deref(), Some("alpha"));
    }

    /// Captured from `pacman -Rs --print -- acl` (stdout/stderr as pacman split them).
    #[test]
    fn dependency_refusal_is_summarized() {
        let stderr = "error: failed to prepare transaction (could not satisfy dependencies)\n";
        let stdout = "checking dependencies...\n\
                      :: removing acl breaks dependency 'acl' required by coreutils\n\
                      :: removing acl breaks dependency 'acl' required by cups\n\
                      :: removing acl breaks dependency 'libacl.so=1-64' required by cups\n\
                      :: removing attr breaks dependency 'attr' required by acl\n";
        assert_eq!(
            summarize_pacman_failure(stderr, stdout),
            "error: failed to prepare transaction (could not satisfy dependencies)\n\
             acl is still required by coreutils, cups\n\
             attr is still required by acl"
        );
        // Lines pacman wrote to stderr instead are not duplicated.
        let both = format!("{stderr}:: removing acl breaks dependency 'acl' required by cups\n");
        assert_eq!(
            summarize_pacman_failure(&both, ""),
            "error: failed to prepare transaction (could not satisfy dependencies)\nacl is still required by cups"
        );
    }

    #[test]
    fn preview_parse_and_sizes() {
        assert_eq!(parse_removal_preview("a 10\nb x\n\n"), vec![("a".to_string(), 10), ("b".to_string(), 0)]);
        assert_eq!(human_size(2048), "2 KiB");
        assert_eq!(human_size(5 * 1024 * 1024), "5.0 MiB");
    }

    #[test]
    fn test_view_layout_grid() {
        let mut ui = cce_ui::context::UiContext::new();
        let mut state = PackagesState::new(&mut ui);
        let mut layout = cce_ui::compose::PageFlow::new();
        let pc = view(&mut state, 10.0, 20.0, 800.0, 600.0, &[false, false], &mut layout, &mut ui);
        assert!(!pc.rects.is_empty() || !pc.texts.is_empty() || !pc.buttons.is_empty());
    }
}
