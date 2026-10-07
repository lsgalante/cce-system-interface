use crate::app::{AppAction, PageContent, SectionContextExt, section_divider};
use cce_ui::widget::ScrollRegion;
use cce_ui::layout::{render_widget, PageLayoutBuilder, LayoutStrategy, SectionContext, RenderTarget};
use cce_ui::widget::{WidgetHost, TextBox, InteractiveListItem};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageTab {
    Installed,
    Updates,
}

impl Default for PackageTab {
    fn default() -> Self {
        PackageTab::Installed
    }
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
    pub search_box: cce_ui::widget::Adapted<TextBox>,
    pub installed_list: ScrollRegion,
    pub installed_items: Vec<cce_ui::widget::Adapted<cce_ui::widget::InteractiveListItem>>,
    pub updates_list: ScrollRegion,
    pub updates_items: Vec<cce_ui::widget::Adapted<cce_ui::widget::InteractiveListItem>>,
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
            search_box: TextBox::new(String::new()).with_placeholder("Filter Packages..."),
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

#[derive(Debug, Clone)]
pub enum PackagesMessage {
    Refreshed(PackagesState),
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
    list.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
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
    list.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
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

/// A button wide enough for its label in the (monospace) button font.
fn label_button_w(label: &str) -> f32 {
    label.chars().count() as f32 * 9.5 + 28.0
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
                        if let Some(filename) = path.split('/').last() {
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

pub fn view(
    state: &mut PackagesState,
    cx: f32,
    cy: f32,
    cw: f32,
    ch: f32,
    sec_focused: &[bool],
    layout: &mut dyn LayoutStrategy,
    ctx: &mut cce_ui::context::UiContext,
) -> PageContent {
    let m = crate::app::section_margin();
    let mut final_pc = PageContent::new();
    let sec_w = 260.0f32;
    let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(1);

    builder.add_section_spanned(&mut final_pc, "", 1, sec_focused.first().copied().unwrap_or(false), |sec| {
        let sec_w = sec.cw;
        if !state.loaded {
            sec.text("Loading package lists...", 12.0, 0.0, 12.0, TEXT_DIM);
            return;
        }

        // ── Tabs (with counts), filter, compact update row ──
        let mut stack = sec.vstack(cce_ui::layout::plate_gap());
        let tab_h = cce_ui::layout::button_height();
        let active_bg = [0.20, 0.40, 0.65, 0.4];
        let inactive_bg = [0.10, 0.10, 0.16, 0.3];
        let hover_bg = [0.20, 0.20, 0.25, 0.15];

        let label1 = format!("Installed ({})", state.installed.len());
        let label2 = format!("Updates ({})", state.updates.len());

        stack.add_row(2, cce_ui::layout::plate_gap(), tab_h, |ctx, i, x, w| {
            if i == 0 {
                ctx.button(
                    &label1,
                    x,
                    ctx.ay(),
                    w,
                    tab_h,
                    if state.active_tab == PackageTab::Installed { active_bg } else { inactive_bg },
                    hover_bg,
                    [0.90, 0.90, 0.95, 1.0],
                    AppAction::Packages(PackagesMessage::SetTab(PackageTab::Installed)),
                );
            } else {
                ctx.button(
                    &label2,
                    x,
                    ctx.ay(),
                    w,
                    tab_h,
                    if state.active_tab == PackageTab::Updates { active_bg } else { inactive_bg },
                    hover_bg,
                    [0.90, 0.90, 0.95, 1.0],
                    AppAction::Packages(PackagesMessage::SetTab(PackageTab::Updates)),
                );
            }
        });

        stack.context.spacing(4.0);

        let search_w = sec_w - 2.0 * m;
        let search_h = 46.0;
        state.search_box.set_row_rect(stack.context.left + m, search_w);
        stack.add_widget(&mut state.search_box, search_w, search_h, ctx);
        stack.context.spacing(4.0);

        // Update System: compact button + status text on one row.
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
        stack.add_row(3, cce_ui::layout::plate_gap(), tab_h, |c, i, x, w| {
            if i == 0 {
                c.button(btn_lbl, x, c.ay(), w, tab_h, bg, hover, [0.90, 0.90, 0.95, 1.0],
                    AppAction::Packages(PackagesMessage::StartUpdate));
            } else if i == 1 {
                let y = c.ay();
                c.pc.text(&status_line, x, y + (tab_h - 14.0) / 2.0, 12.0, status_color);
            }
        });

        if let Some(ref res) = state.last_update_res {
            match res {
                Ok(_) => stack.context.text("Last update succeeded", 12.0, 0.0, 12.0, [0.56, 0.83, 0.56, 1.0]),
                Err(err) => {
                    stack.context.text("Last update failed:", 12.0, 0.0, 12.0, RED);
                    stack.context.text(err, 12.0, 0.0, 11.0, RED);
                }
            }
        }

        // Characters per line for wrapped status text at 11px.
        let wrap_chars = (((sec_w - 2.0 * m - 24.0) / 6.5) as usize).max(20);
        let busy = state.busy();

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
            stack.add_row(4, cce_ui::layout::plate_gap(), tab_h, |c, i, x, w| {
                if let Some((f, label)) = filters.get(i) {
                    let bg = if filter == *f { active_bg } else { inactive_bg };
                    c.button(label, x, c.ay(), w, tab_h, bg, hover_bg, [0.90, 0.90, 0.95, 1.0],
                        AppAction::Packages(PackagesMessage::SetFilter(*f)));
                } else {
                    let bg = if select_mode { active_bg } else { inactive_bg };
                    c.button(select_label, x, c.ay(), w, tab_h, bg, hover_bg, [0.90, 0.90, 0.95, 1.0],
                        AppAction::Packages(PackagesMessage::ToggleSelectMode));
                }
            });

            // ── Bulk actions over the checked rows ──
            if state.select_mode {
                let n = state.checked.len();
                let summary = if n == 0 {
                    "Click rows to select".to_string()
                } else {
                    format!("{} selected", count_noun(n))
                };
                let targets: Vec<String> = state.checked.iter().cloned().collect();
                let can_act = n > 0 && !busy;
                let (act_bg, act_text) = if can_act { (TOGGLE_OFF, TEXT_FG) } else { (TOGGLE_OFF, TEXT_DIM) };
                stack.add_row(5, cce_ui::layout::plate_gap(), tab_h, |c, i, x, w| {
                    let y = c.ay();
                    match i {
                        0 => c.pc.text(&summary, x, y + (tab_h - 14.0) / 2.0, 12.0, if n > 0 { ACCENT } else { TEXT_DIM }),
                        1 => c.button("Select Visible", x, y, w, tab_h, TOGGLE_OFF, BTN_HOVER, TEXT_FG,
                            AppAction::Packages(PackagesMessage::CheckAllVisible)),
                        2 => c.button("Clear", x, y, w, tab_h, TOGGLE_OFF, BTN_HOVER, TEXT_FG,
                            AppAction::Packages(PackagesMessage::ClearChecked)),
                        3 => c.button(if state.marking { "Marking..." } else { "Mark Explicit" }, x, y, w, tab_h,
                            act_bg, if can_act { BTN_HOVER } else { act_bg }, act_text,
                            AppAction::Packages(PackagesMessage::SetInstallReason(targets.clone(), true))),
                        _ => c.button("Remove...", x, y, w, tab_h,
                            if can_act { [0.25, 0.14, 0.14, 1.0] } else { TOGGLE_OFF },
                            if can_act { [0.40, 0.20, 0.20, 1.0] } else { TOGGLE_OFF },
                            if can_act { [0.95, 0.55, 0.55, 1.0] } else { TEXT_DIM },
                            AppAction::Packages(PackagesMessage::PreviewRemoval(targets.clone()))),
                    }
                });
            }
        }

        // ── Removal confirmation: what pacman says it would take ──
        if state.previewing {
            stack.context.text("Checking what would be removed...", 12.0, 0.0, 12.0, TEXT_DIM);
        } else if let Some(plan) = state.removal.clone() {
            section_divider(stack.context);
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
                    stack.context.text(&head, 12.0, 0.0, 12.0, [0.95, 0.75, 0.55, 1.0]);
                    let names = pkgs.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>().join(", ");
                    let lines = wrap_text(&names, wrap_chars);
                    const MAX_LINES: usize = 6;
                    for line in lines.iter().take(MAX_LINES) {
                        stack.context.text(line, 12.0, 0.0, 11.0, TEXT_FG);
                    }
                    if lines.len() > MAX_LINES {
                        let shown: usize = lines.iter().take(MAX_LINES).map(|l| l.split_whitespace().count()).sum();
                        stack.context.text(&format!("... and {} more", pkgs.len().saturating_sub(shown)), 12.0, 0.0, 11.0, TEXT_DIM);
                    }
                    let confirm = if state.uninstalling { "Removing...".to_string() } else { format!("Remove {}", count_noun(pkgs.len())) };
                    let targets = plan.targets.clone();
                    stack.add_row(3, cce_ui::layout::plate_gap(), tab_h, |c, i, x, w| {
                        let y = c.ay();
                        if i == 0 {
                            c.button(&confirm, x, y, w, tab_h, [0.30, 0.14, 0.14, 1.0], [0.45, 0.20, 0.20, 1.0], [0.98, 0.65, 0.65, 1.0],
                                AppAction::Packages(PackagesMessage::StartUninstall(targets.clone())));
                        } else if i == 1 {
                            c.button("Cancel", x, y, w, tab_h, TOGGLE_OFF, BTN_HOVER, TEXT_FG,
                                AppAction::Packages(PackagesMessage::CancelRemoval));
                        }
                    });
                }
                Err(err) => {
                    stack.context.text(&format!("Can't remove {}:", plan.targets.join(", ")), 12.0, 0.0, 12.0, RED);
                    for line in err.lines().flat_map(|l| wrap_text(l, wrap_chars)).take(8) {
                        stack.context.text(&line, 12.0, 0.0, 11.0, [0.95, 0.55, 0.55, 1.0]);
                    }
                    stack.add_row(3, cce_ui::layout::plate_gap(), tab_h, |c, i, x, w| {
                        if i == 0 {
                            c.button("Dismiss", x, c.ay(), w, tab_h, TOGGLE_OFF, BTN_HOVER, TEXT_FG,
                                AppAction::Packages(PackagesMessage::CancelRemoval));
                        }
                    });
                }
            }
        }

        // ── Outcome of the last remove / mark ──
        if let Some(ref res) = state.last_action {
            match res {
                Ok(msg) => stack.context.text(msg, 12.0, 0.0, 12.0, [0.56, 0.83, 0.56, 1.0]),
                Err(err) => {
                    for (i, line) in err.lines().flat_map(|l| wrap_text(l, wrap_chars)).take(8).enumerate() {
                        stack.context.text(&line, 12.0, 0.0, if i == 0 { 12.0 } else { 11.0 }, RED);
                    }
                }
            }
        }

        // ── Selected package details ──
        // Set aside while a removal is being confirmed: the plan names the
        // package, and both together push the list off the page.
        let confirming = state.previewing || state.removal.is_some();
        if !confirming && (state.loading_info || state.selected_package.is_some()) {
            section_divider(stack.context);
        }
        if confirming {
            // Nothing: the confirmation above stands in for the details.
        } else if state.loading_info {
            stack.context.text("Loading package details...", 12.0, 0.0, 12.0, TEXT_DIM);
        } else if let Some(pkg_name) = state.selected_package.clone() {
            if let Some(info_raw) = state.selected_package_info.clone() {
                let mut parsed = parse_package_info(&info_raw);
                if parsed.name.is_empty() {
                    parsed.name = pkg_name.clone();
                }

                // Heading row: name left, Uninstall (compact, quiet-red) right.
                {
                    let sc = &mut *stack.context;
                    let mut y = sc.content_y;
                    if y > sc.content_start_y {
                        y += sc.row_gap;
                    }
                    let lx = sc.ax(m);
                    sc.pc.text(&parsed.name, lx, y, 14.0, [0.35, 0.65, 0.90, 1.0]);
                    sc.content_y = y + 20.0;
                    for h in &mut sc.grid.col_heights {
                        *h = sc.content_y;
                    }
                    let is_installed = state.installed.iter().find(|p| p.name == pkg_name);
                    if let (PackageTab::Installed, Some(info)) = (state.active_tab, is_installed) {
                        // Right-aligned to the box every button row lays out in
                        // (`row_layout`); a width derived from cw by hand ran
                        // a few px past the well and cut the button's edge.
                        let right = sc.row_layout(1, 0.0).first().map(|&(x, w)| x + w).unwrap_or(lx);
                        let bw = label_button_w("Uninstall");
                        let bx = right - bw;
                        let (btn_lbl, bg, hover, text_col) = if busy {
                            ("Uninstall", TOGGLE_OFF, TOGGLE_OFF, TEXT_DIM)
                        } else {
                            ("Uninstall", [0.25, 0.14, 0.14, 1.0], [0.40, 0.20, 0.20, 1.0], [0.95, 0.55, 0.55, 1.0])
                        };
                        // Centred on the 14px name's line.
                        let bh = cce_ui::layout::button_height();
                        let by = y + (17.0 - bh) / 2.0;
                        sc.button(btn_lbl, bx, by, bw, bh, bg, hover, text_col,
                            AppAction::Packages(PackagesMessage::PreviewRemoval(vec![pkg_name.clone()])));
                        // The install-reason flip beside it: an explicit package can
                        // be handed back as a dependency (so it goes when nothing
                        // needs it) and a dependency kept for good.
                        let (mark_lbl, to_explicit) = if info.explicit {
                            ("Mark as Dependency", false)
                        } else {
                            ("Mark Explicit", true)
                        };
                        let mw = label_button_w(mark_lbl);
                        let text_col = if busy { TEXT_DIM } else { TEXT_FG };
                        sc.button(mark_lbl, bx - 4.0 - mw, by, mw, bh, TOGGLE_OFF, if busy { TOGGLE_OFF } else { BTN_HOVER }, text_col,
                            AppAction::Packages(PackagesMessage::SetInstallReason(vec![pkg_name.clone()], to_explicit)));
                    }
                }

                // Same-line kv rows, values wrapping at the value column.
                let kv_wrap = |sc: &mut SectionContext<'_, PageContent>, key: &str, val: &str| {
                    if val.is_empty() {
                        return;
                    }
                    let mut y = sc.content_y + 4.0;
                    let lx = sc.ax(m);
                    let vx = lx + 118.0;
                    let usable_w = sc.cw - 118.0 - 2.0 * (sc.padding() + m);
                    let max_chars = ((usable_w / 6.0) as usize).max(15);
                    sc.pc.text(key, lx, y, 11.0, TEXT_DIM);
                    let lines = wrap_text(val, max_chars);
                    for line in &lines {
                        sc.pc.text(line, vx, y, 11.0, TEXT_FG);
                        y += 15.0;
                    }
                    if lines.is_empty() {
                        y += 15.0;
                    }
                    sc.content_y = y + 1.0;
                    for h in &mut sc.grid.col_heights {
                        *h = sc.content_y;
                    }
                };

                kv_wrap(stack.context, "Version", &parsed.version);
                let orphan = state.installed.iter().any(|p| p.name == pkg_name && p.orphan);
                let reason = if orphan {
                    format!("{} (orphan: nothing requires it now)", parsed.install_reason)
                } else {
                    parsed.install_reason.clone()
                };
                kv_wrap(stack.context, "Install Reason", &reason);
                kv_wrap(stack.context, "Size", &parsed.size);
                kv_wrap(stack.context, "Licenses", &parsed.licenses);
                kv_wrap(stack.context, "Website", &parsed.website);
                kv_wrap(stack.context, "Packager", &parsed.packager);
                kv_wrap(stack.context, "Built", &parsed.build_date);
                kv_wrap(stack.context, "Description", &parsed.description);
                kv_wrap(stack.context, "Commands", &parsed.commands);

                if !parsed.required_by.is_empty() && parsed.required_by != "None" {
                    stack.context.text("Required By", 12.0, 0.0, 11.0, TEXT_DIM);

                    let reqs: Vec<&str> = parsed.required_by.split_whitespace().collect();
                    let cols_count = 4;
                    let gap = 4.0;
                    let btn_h = cce_ui::layout::button_height();
                    // A core library is required by hundreds (glibc: ~300). Every
                    // row of buttons pushes the list down, and past the page's
                    // bottom the list loses its box; a few rows say enough.
                    const MAX_REQ_ROWS: usize = 4;
                    let shown = reqs.len().min(cols_count * MAX_REQ_ROWS);

                    for chunk in reqs[..shown].chunks(cols_count) {
                        let btn_y = stack.context.ay();
                        let cols = stack.context.row_layout(cols_count, gap);
                        for (i, &pkg) in chunk.iter().enumerate() {
                            if let Some(&(x, w)) = cols.get(i) {
                                let action = AppAction::Packages(PackagesMessage::SelectAndScrollPackage(pkg.to_string()));
                                stack.context.button(pkg, x, btn_y, w, btn_h, TOGGLE_OFF, BTN_HOVER, TEXT_FG, action);
                            }
                        }
                    }
                    if reqs.len() > shown {
                        stack.context.text(&format!("... and {} more", reqs.len() - shown), 12.0, 0.0, 11.0, TEXT_DIM);
                    }
                }
            } else {
                stack.context.text("No details available.", 12.0, 0.0, 12.0, TEXT_DIM);
            }
        }

        section_divider(stack.context);

        // ── List fills the rest of the page, across the section's content
        // box (the box the section clips to) like the services list ──
        let list_box_x = sec.content_left();
        let list_box_y = sec.ay();
        let list_box_w = sec.content_width();
        let list_box_h = ((cy + ch) - m - list_box_y).max(120.0);

        let query = if state.search_box.editing {
            state.search_box.edit_buffer.to_lowercase()
        } else {
            state.search_box.text.to_lowercase()
        };

        match state.active_tab {
            PackageTab::Installed => {
                let visible = state.visible_installed();
                let installed = &state.installed;
                let filtered: Vec<&PackageInfo> = visible.iter().map(|&i| &installed[i]).collect();

                // Dissolved List (Phase 6v): scroll state + frame prims are app-owned.
                state.installed_list.set_rect(list_box_x, list_box_y, list_box_w, list_box_h);
                state.installed_list.update_bounds(filtered.len(), list_box_y, list_box_h);
                state.installed_list.push_prims(sec.pc);
                let item_h = state.installed_list.item_height;

                if state.installed_items.len() != filtered.len() {
                    state.installed_items.clear();
                    for _ in 0..filtered.len() {
                        state.installed_items.push(InteractiveListItem::new(""));
                    }
                }

                sec.pc.push_clip_rect(list_box_x, list_box_y, list_box_w, list_box_h);
                for (idx, pkg) in filtered.iter().enumerate() {
                    if let Some(draw_y) = state.installed_list.get_item_draw_y(idx, 4.0) {
                        // Rows dispatch as extra roots (the dissolved list is no parent).
                        let item = &mut state.installed_items[idx];
                        item.title = pkg.name.clone();
                        let reason = match (pkg.explicit, pkg.orphan) {
                            (true, _) => "explicit",
                            (false, true) => "orphan",
                            (false, false) => "dependency",
                        };
                        item.subtitle = Some(format!("{}  ·  {}", pkg.version, reason));
                        item.selected = if state.select_mode {
                            state.checked.contains(&pkg.name)
                        } else {
                            Some(&pkg.name) == state.selected_package.as_ref()
                        };
                        render_widget(sec.pc, item, list_box_x + 24.0, draw_y, list_box_w - 44.0, item_h, ctx);
                    }
                }
                sec.pc.pop_clip_rect();
                // The scrollbar's fore copy, over the rows at the raise's fade.
                state.installed_list.push_scrollbar_fore(sec.pc);

                if filtered.is_empty() {
                    let msg = if state.filter == InstalledFilter::Orphans && query.is_empty() {
                        "No orphaned packages"
                    } else {
                        "No packages match the query"
                    };
                    sec.pc.text(msg, list_box_x + 16.0, list_box_y + 16.0, 12.0, TEXT_DIM);
                }
            }
            PackageTab::Updates => {
                let filtered: Vec<&UpdateInfo> = state.updates.iter()
                    .filter(|p| p.name.to_lowercase().contains(&query))
                    .collect();

                // Dissolved List (Phase 6v): scroll state + frame prims are app-owned.
                state.updates_list.set_rect(list_box_x, list_box_y, list_box_w, list_box_h);
                state.updates_list.update_bounds(filtered.len(), list_box_y, list_box_h);
                state.updates_list.push_prims(sec.pc);
                let item_h = state.updates_list.item_height;

                if state.updates_items.len() != filtered.len() {
                    state.updates_items.clear();
                    for _ in 0..filtered.len() {
                        state.updates_items.push(InteractiveListItem::new(""));
                    }
                }

                sec.pc.push_clip_rect(list_box_x, list_box_y, list_box_w, list_box_h);
                for (idx, pkg) in filtered.iter().enumerate() {
                    if let Some(draw_y) = state.updates_list.get_item_draw_y(idx, 4.0) {
                        // Rows dispatch as extra roots (the dissolved list is no parent).
                        let item = &mut state.updates_items[idx];
                        item.title = pkg.name.clone();
                        item.subtitle = Some(format!("{}  ->  {}", pkg.old_version, pkg.new_version));
                        item.selected = Some(&pkg.name) == state.selected_package.as_ref();
                        render_widget(sec.pc, item, list_box_x + 24.0, draw_y, list_box_w - 44.0, item_h, ctx);
                    }
                }
                sec.pc.pop_clip_rect();
                // The scrollbar's fore copy, over the rows at the raise's fade.
                state.updates_list.push_scrollbar_fore(sec.pc);

                if filtered.is_empty() {
                    sec.pc.text("No updates match the query", list_box_x + 16.0, list_box_y + 16.0, 12.0, TEXT_DIM);
                }
            }
        }

        // End the section so the well's bottom wall sits one margin below
        // the list: finish() places the wall at content_y + padding + margin,
        // so the list's own bottom margin and that one cancel.
        sec.content_y = list_box_y + list_box_h - sec.padding();
    });

    final_pc
}

pub fn update(state: &mut PackagesState, msg: PackagesMessage) {
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
            state.installed_items.clear();
            state.updates_items.clear();
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
                state.installed_items.clear();
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
            for i in state.visible_installed() {
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
            state.select_and_scroll_to(&name);
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
    fn query(&self) -> String {
        if self.search_box.editing {
            self.search_box.edit_buffer.to_lowercase()
        } else {
            self.search_box.text.to_lowercase()
        }
    }

    /// Indices into `installed` of the rows the Installed tab shows, in order:
    /// the filter, then the search. The view paints exactly these and the click
    /// mapping resolves against them, so the two can never disagree.
    pub fn visible_installed(&self) -> Vec<usize> {
        let query = self.query();
        self.installed
            .iter()
            .enumerate()
            .filter(|(_, p)| self.filter.admits(p))
            .filter(|(_, p)| p.name.to_lowercase().contains(&query) || p.version.to_lowercase().contains(&query))
            .map(|(i, _)| i)
            .collect()
    }

    pub fn select_and_scroll_to(&mut self, pkg_name: &str) {
        self.active_tab = PackageTab::Installed;
        // The scroll target below is an index into the whole list, so the
        // list must be unfiltered for it to land on the package.
        self.filter = InstalledFilter::All;
        self.select_mode = false;
        self.checked.clear();
        self.installed_items.clear();
        self.search_box.text.clear();
        self.search_box.edit_buffer.clear();
        self.search_box.editing = false;
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
        layout: &mut dyn LayoutStrategy,
        ctx: &mut cce_ui::context::UiContext,
    ) -> crate::app::PageContent {
        view(self, cx, cy, cw, ch, sec_focused, layout, ctx)
    }

    fn propagate_widget_changes(&mut self, actions: &mut Vec<crate::app::AppAction>) {
        let query = if self.search_box.editing {
            self.search_box.edit_buffer.to_lowercase()
        } else {
            self.search_box.text.to_lowercase()
        };

        match self.active_tab {
            PackageTab::Installed => {
                let visible = self.visible_installed();
                for (idx, item) in self.installed_items.iter_mut().enumerate() {
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
                for (idx, item) in self.updates_items.iter_mut().enumerate() {
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

    fn register_extra_dispatch_roots(&mut self, ctx: &mut cce_ui::context::UiContext) {
        let (list, items) = match self.active_tab {
            PackageTab::Installed => (&self.installed_list, &mut self.installed_items),
            PackageTab::Updates => (&self.updates_list, &mut self.updates_items),
        };
        for (idx, i) in items.iter_mut().enumerate() {
            if list.get_item_draw_y(idx, 4.0).is_none() {
                continue;
            }
            ctx.register_host(i);
        }
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
        let mut st = PackagesState::default();
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
        update(&mut st, PackagesMessage::Refreshed(fresh));
        assert_eq!(st.selected_package.as_deref(), Some("foo"), "refresh must keep the selection");
        assert!(st.selected_package_info.is_some(), "refresh must keep fetched info");
        assert!(st.updating, "refresh must not clear the in-flight update flag");

        // A package that vanished from both lists does clear the selection.
        let fresh2 = PackagesState { loaded: true, ..Default::default() };
        update(&mut st, PackagesMessage::Refreshed(fresh2));
        assert!(st.selected_package.is_none());
        assert!(st.selected_package_info.is_none());
    }

    fn pkg(name: &str, explicit: bool, orphan: bool) -> PackageInfo {
        PackageInfo { name: name.into(), version: "1".into(), explicit, orphan }
    }

    fn loaded_state() -> PackagesState {
        let mut st = PackagesState::default();
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
        let mut st = loaded_state();
        assert_eq!(st.visible_installed().len(), 4);
        update(&mut st, PackagesMessage::SetFilter(InstalledFilter::Orphans));
        assert_eq!(st.visible_installed(), vec![1, 3]);
        update(&mut st, PackagesMessage::SetFilter(InstalledFilter::Explicit));
        assert_eq!(st.visible_installed(), vec![0]);
        update(&mut st, PackagesMessage::SetFilter(InstalledFilter::Orphans));
        st.search_box.text = "gamma".into();
        assert_eq!(st.visible_installed(), vec![3]);
    }

    /// A click on visible row N must name the package painted at row N, under
    /// a filter too — the view and the click mapping share `visible_installed`.
    #[test]
    fn row_click_resolves_through_the_filter() {
        let mut st = loaded_state();
        update(&mut st, PackagesMessage::SetFilter(InstalledFilter::Orphans));
        let mut layout = cce_ui::layout::ColumnLayout::new(20.0);
        let mut ctx = cce_ui::context::UiContext::new();
        view(&mut st, 10.0, 20.0, 800.0, 600.0, &[false], &mut layout, &mut ctx);
        assert_eq!(st.installed_items.len(), 2);
        st.installed_items[1].just_clicked = true;
        let mut actions = Vec::new();
        crate::pages::AppPage::propagate_widget_changes(&mut st, &mut actions);
        match actions.as_slice() {
            [AppAction::Packages(PackagesMessage::SelectPackage(Some(n)))] => assert_eq!(n, "gamma-orphan"),
            other => panic!("unexpected actions: {other:?}"),
        }

        // In select mode the same click toggles the row instead.
        update(&mut st, PackagesMessage::ToggleSelectMode);
        st.installed_items[1].just_clicked = true;
        let mut actions = Vec::new();
        crate::pages::AppPage::propagate_widget_changes(&mut st, &mut actions);
        match actions.as_slice() {
            [AppAction::Packages(PackagesMessage::ToggleChecked(n))] => assert_eq!(n, "gamma-orphan"),
            other => panic!("unexpected actions: {other:?}"),
        }
    }

    #[test]
    fn select_mode_checks_and_clears() {
        let mut st = loaded_state();
        st.selected_package = Some("alpha".into());
        update(&mut st, PackagesMessage::ToggleSelectMode);
        assert!(st.selected_package.is_none(), "select mode frees the details pane");
        update(&mut st, PackagesMessage::ToggleChecked("beta".into()));
        update(&mut st, PackagesMessage::ToggleChecked("beta".into()));
        assert!(st.checked.is_empty(), "a second click unchecks");
        update(&mut st, PackagesMessage::SetFilter(InstalledFilter::Orphans));
        update(&mut st, PackagesMessage::CheckAllVisible);
        assert_eq!(st.checked.iter().cloned().collect::<Vec<_>>(), ["beta", "gamma-orphan"]);
        // A refresh drops checked names that are gone.
        let fresh = PackagesState { loaded: true, installed: vec![pkg("beta", false, true)], ..Default::default() };
        update(&mut st, PackagesMessage::Refreshed(fresh));
        assert_eq!(st.checked.iter().cloned().collect::<Vec<_>>(), ["beta"]);
        update(&mut st, PackagesMessage::ToggleSelectMode);
        assert!(st.checked.is_empty(), "leaving select mode clears the selection");
    }

    /// The failure that prompted this page's rework: a removal pacman refuses
    /// (a dependency still requires it) used to land in the details text,
    /// where the info parser found no known key and showed nothing at all.
    #[test]
    fn refused_removal_stays_visible() {
        let mut st = loaded_state();
        let err = "error: failed to prepare transaction (could not satisfy dependencies)\n\
                   :: removing beta breaks dependency 'beta' required by alpha".to_string();
        update(&mut st, PackagesMessage::PreviewRemoval(vec!["beta".into()]));
        assert!(st.previewing);
        update(&mut st, PackagesMessage::RemovalPreviewed(vec!["beta".into()], Err(err.clone())));
        assert!(!st.previewing);
        let mut layout = cce_ui::layout::ColumnLayout::new(20.0);
        let pc = view(&mut st, 10.0, 20.0, 800.0, 600.0, &[false], &mut layout, &mut cce_ui::context::UiContext::new());
        let texts: Vec<String> = pc.texts.iter().map(|t| t.0.clone()).collect();
        assert!(texts.iter().any(|t| t.contains("required by alpha")), "the reason is painted: {texts:?}");

        // And a failed transaction reports into last_action, not the details.
        update(&mut st, PackagesMessage::StartUninstall(vec!["beta".into()]));
        update(&mut st, PackagesMessage::UninstallFinished(vec!["beta".into()], Err(err)));
        assert!(!st.uninstalling);
        assert!(matches!(&st.last_action, Some(Err(e)) if e.contains("required by alpha")));
    }

    #[test]
    fn successful_removal_reports_the_whole_plan() {
        let mut st = loaded_state();
        st.checked.insert("beta".into());
        update(&mut st, PackagesMessage::RemovalPreviewed(
            vec!["beta".into()],
            Ok(parse_removal_preview("beta 1024\nlibbeta 2048\n")),
        ));
        update(&mut st, PackagesMessage::StartUninstall(vec!["beta".into()]));
        update(&mut st, PackagesMessage::UninstallFinished(vec!["beta".into()], Ok(())));
        assert!(st.removal.is_none());
        assert!(st.checked.is_empty());
        assert!(matches!(&st.last_action, Some(Ok(m)) if m == "Removed 2 packages"));
    }

    #[test]
    fn marking_explicit_updates_rows_at_once() {
        let mut st = loaded_state();
        update(&mut st, PackagesMessage::SetInstallReason(vec!["beta".into()], true));
        assert!(st.busy());
        update(&mut st, PackagesMessage::InstallReasonSet(vec!["beta".into()], true, Ok(())));
        assert!(!st.busy());
        assert!(st.installed[1].explicit && !st.installed[1].orphan);
        assert!(matches!(&st.last_action, Some(Ok(m)) if m == "Marked beta as explicitly installed"));
    }

    #[test]
    fn jump_to_package_clears_the_filter() {
        let mut st = loaded_state();
        update(&mut st, PackagesMessage::SetFilter(InstalledFilter::Orphans));
        update(&mut st, PackagesMessage::SelectAndScrollPackage("alpha".into()));
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
        let mut state = PackagesState::default();
        let mut layout = cce_ui::layout::ColumnLayout::new(20.0);
        let pc = view(&mut state, 10.0, 20.0, 800.0, 600.0, &[false, false], &mut layout, &mut cce_ui::context::UiContext::new());
        assert!(!pc.rects.is_empty() || !pc.texts.is_empty() || !pc.buttons.is_empty());
    }
}
