use crate::app::{AppAction, PageContent, SectionContextExt};
use crate::scroll_region::ScrollRegion;
use cce_ui::layout::{render_widget, PageLayoutBuilder, LayoutStrategy, SectionContext, RenderTarget};
use cce_ui::widget::{WidgetHost, TextBox, InteractiveListItem};

#[derive(Debug, Clone)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
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
}

impl Default for PackagesState {
    fn default() -> Self {
        Self {
            loaded: false,
            installed: Vec::new(),
            updates: Vec::new(),
            active_tab: PackageTab::Installed,
            search_box: TextBox::new(String::new()).with_placeholder("Filter Packages..."),
            installed_list: ScrollRegion::new(32.0, 4.0),
            installed_items: Vec::new(),
            updates_list: ScrollRegion::new(32.0, 4.0),
            updates_items: Vec::new(),
            updating: false,
            last_update_res: None,
            selected_package: None,
            selected_package_info: None,
            loading_info: false,
            uninstalling: false,
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
    StartUninstall(String),
    UninstallFinished(Result<(), String>),
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
    let mut list = Vec::new();
    if let Ok(output) = tokio::process::Command::new("pacman")
        .arg("-Q")
        .output()
        .await
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                list.push(PackageInfo {
                    name: parts[0].to_string(),
                    version: parts[1].to_string(),
                });
            }
        }
    }
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

pub async fn run_uninstall(name: String) -> Result<(), String> {
    let output = tokio::process::Command::new("pkexec")
        .args(["pacman", "-R", "--noconfirm", &name])
        .output()
        .await
        .map_err(|e| format!("Failed to run uninstall: {}", e))?;
        
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(format!("Uninstall process failed: {}", err));
    }
    
    Ok(())
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
const TOGGLE_ON: [f32; 4] = [0.16, 0.41, 0.18, 1.0];
const TOGGLE_OFF: [f32; 4] = [0.16, 0.16, 0.24, 1.0];
const BTN_HOVER: [f32; 4] = [0.25, 0.30, 0.26, 1.0];
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
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
    let mut final_pc = PageContent::new();
    let sec_w = 260.0f32;
    let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(3);

    // Section 1: Packages List
    builder.add_section(&mut final_pc, "Packages", sec_focused.first().copied().unwrap_or(false), |sec| {
        let sec_w = sec.cw;
        if !state.loaded {
            sec.text("Loading package lists...", 12.0, 0.0, 12.0, TEXT_DIM);
        } else {
            // Tab header: Installed, Updates
            let mut stack = sec.vstack(8.0);
            let tab_h = 28.0;
            let active_bg = [0.20, 0.40, 0.65, 0.4];
            let inactive_bg = [0.10, 0.10, 0.16, 0.3];
            let hover_bg = [0.20, 0.20, 0.25, 0.15];

            let label1 = "Installed";
            let label2 = "Updates";

            stack.add_row(2, 8.0, tab_h, |ctx, i, x, w| {
                if i == 0 {
                    ctx.button(
                        label1,
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
                        label2,
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

            // Search box
            let search_w = sec_w - 24.0;
            let search_h = 46.0;

            state.search_box.set_row_rect(stack.context.left + 12.0, search_w);
            stack.add_widget(&mut state.search_box, search_w, search_h, ctx);
            stack.context.spacing(8.0);

            // List area
            let list_box_x = sec.left + 12.0;
            let list_box_y = sec.ay();
            let list_box_w = sec_w - 24.0;
            let list_box_h = 360.0;

            let query = if state.search_box.editing {
                state.search_box.edit_buffer.to_lowercase()
            } else {
                state.search_box.text.to_lowercase()
            };

            match state.active_tab {
                PackageTab::Installed => {
                    let filtered: Vec<&PackageInfo> = state.installed.iter()
                        .filter(|p| p.name.to_lowercase().contains(&query) || p.version.to_lowercase().contains(&query))
                        .collect();

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
                            item.subtitle = Some(format!("Version: {}", pkg.version));
                            item.selected = Some(&pkg.name) == state.selected_package.as_ref();
                            render_widget(sec.pc, item, list_box_x + 24.0, draw_y, list_box_w - 44.0, item_h, ctx);
                        }
                    }
                    sec.pc.pop_clip_rect();

                    if filtered.is_empty() {
                        sec.pc.text("No packages match the query", list_box_x + 16.0, list_box_y + 16.0, 12.0, TEXT_DIM);
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

                    if filtered.is_empty() {
                        sec.pc.text("No updates match the query", list_box_x + 16.0, list_box_y + 16.0, 12.0, TEXT_DIM);
                    }
                }
            }

            sec.content_y += list_box_h;
        }
    });

    // Section 2: Package Details Info
    builder.add_section(&mut final_pc, "Package Info", sec_focused.get(1).copied().unwrap_or(false), |sec1| {
        if state.loading_info {
            sec1.text("Loading package details...", 12.0, 0.0, 12.0, TEXT_DIM);
        } else if let Some(ref pkg_name) = state.selected_package {
            if let Some(ref info_raw) = state.selected_package_info {
                let mut parsed = parse_package_info(info_raw);
                if parsed.name.is_empty() {
                    parsed.name = pkg_name.clone();
                }

                sec1.text(&parsed.name, 12.0, 0.0, 13.0, WHITE);

                let render_detail = |sub: &mut SectionContext<'_, PageContent>, key: &str, val: &str| {
                    sub.text(key, 12.0, 0.0, 11.0, TEXT_DIM);
                    let val_start_x = 90.0f32;
                    let usable_w = sub.cw - val_start_x - 12.0;
                    let char_w = 6.0f32;
                    let max_chars = (usable_w / char_w).max(15.0) as usize;

                    let lines = wrap_text(val, max_chars);
                    for line in &lines {
                        sub.text(line, val_start_x, 0.0, 11.0, TEXT_FG);
                    }
                };

                render_detail(sec1, "Version:", &parsed.version);
                if !parsed.size.is_empty() {
                    render_detail(sec1, "Size:", &parsed.size);
                }
                if !parsed.licenses.is_empty() {
                    render_detail(sec1, "Licenses:", &parsed.licenses);
                }
                if !parsed.website.is_empty() {
                    render_detail(sec1, "Website:", &parsed.website);
                }
                if !parsed.packager.is_empty() {
                    render_detail(sec1, "Packager:", &parsed.packager);
                }
                if !parsed.build_date.is_empty() {
                    render_detail(sec1, "Build Date:", &parsed.build_date);
                }
                if !parsed.description.is_empty() {
                    sec1.separator();
                    render_detail(sec1, "Description:", &parsed.description);
                }
                if !parsed.required_by.is_empty() && parsed.required_by != "None" {
                    sec1.separator();
                    sec1.text("Required By:", 12.0, 0.0, 11.0, TEXT_DIM);

                    let reqs: Vec<&str> = parsed.required_by.split_whitespace().collect();
                    let cols_count = 3;
                    let gap = 6.0;
                    let btn_h = 24.0;

                    for chunk in reqs.chunks(cols_count) {
                        let btn_y = sec1.ay();
                        let cols = sec1.row_layout(cols_count, gap);
                        for (i, &pkg) in chunk.iter().enumerate() {
                            if let Some(&(x, w)) = cols.get(i) {
                                let action = AppAction::Packages(PackagesMessage::SelectAndScrollPackage(pkg.to_string()));
                                sec1.button(pkg, x, btn_y, w, btn_h, TOGGLE_OFF, BTN_HOVER, TEXT_FG, action);
                            }
                        }
                    }
                }
                if !parsed.commands.is_empty() {
                    sec1.separator();
                    render_detail(sec1, "Commands:", &parsed.commands);
                }

                if state.active_tab == PackageTab::Installed {
                    let mut stack = sec1.vstack(8.0);
                    let btn_h = 32.0;
                    let (btn_lbl, bg, hover, action) = if state.uninstalling {
                        ("Uninstalling...", TOGGLE_OFF, TOGGLE_OFF, AppAction::Packages(PackagesMessage::StartUninstall(pkg_name.clone())))
                    } else {
                        ("Uninstall Package", RED, BTN_HOVER, AppAction::Packages(PackagesMessage::StartUninstall(pkg_name.clone())))
                    };
                    stack.add_row(1, 0.0, btn_h, |ctx, _, x, w| {
                        ctx.button(btn_lbl, x, ctx.ay(), w, btn_h, bg, hover, WHITE, action.clone());
                    });
                }
            } else {
                sec1.text("No details available.", 12.0, 0.0, 12.0, TEXT_DIM);
            }
        } else {
            sec1.text("Select a package to view details.", 12.0, 0.0, 12.0, TEXT_DIM);
        }
    });

    // Section 3: Update Actions / Status
    builder.add_section(&mut final_pc, "System Update", sec_focused.get(2).copied().unwrap_or(false), |sec2| {
        if !state.loaded {
            sec2.text("Loading update status...", 12.0, 0.0, 12.0, TEXT_DIM);
        } else {
            // Display summaries
            sec2.text("Installed Packages:", 12.0, 0.0, 12.0, TEXT_DIM);
            sec2.text(&format!("{}", state.installed.len()), 150.0, 0.0, 12.0, TEXT_FG);

            sec2.text("Available Updates:", 12.0, 0.0, 12.0, TEXT_DIM);
            let updates_color = if state.updates.is_empty() { TEXT_FG } else { ACCENT };
            sec2.text(&format!("{}", state.updates.len()), 150.0, 0.0, 12.0, updates_color);

            let status_lbl = if state.updating {
                "Updating..."
            } else if state.updates.is_empty() {
                "System is up to date"
            } else {
                "Updates available"
            };
            sec2.text("Status:", 12.0, 0.0, 12.0, TEXT_DIM);
            sec2.text(status_lbl, 150.0, 0.0, 12.0, if state.updating { ACCENT } else { TEXT_FG });

            if let Some(ref res) = state.last_update_res {
                match res {
                    Ok(_) => {
                        sec2.text("Last update succeeded!", 12.0, 0.0, 12.0, ACCENT);
                    }
                    Err(err) => {
                        sec2.text("Last update failed:", 12.0, 0.0, 12.0, RED);
                        sec2.text(err, 12.0, 0.0, 11.0, RED);
                    }
                }
            }

            let mut stack = sec2.vstack(8.0);
            let btn_h = 32.0;

            let (btn_lbl, bg, hover, action) = if state.updating {
                ("Updating...", TOGGLE_OFF, TOGGLE_OFF, AppAction::Packages(PackagesMessage::StartUpdate))
            } else {
                ("Update System", TOGGLE_ON, BTN_HOVER, AppAction::Packages(PackagesMessage::StartUpdate))
            };

            stack.add_row(1, 0.0, btn_h, |ctx, _, x, w| {
                ctx.button(btn_lbl, x, ctx.ay(), w, btn_h, bg, hover, WHITE, action.clone());
            });
        }
    });

    final_pc
}

pub fn update(state: &mut PackagesState, msg: PackagesMessage) {
    match msg {
        PackagesMessage::Refreshed(new) => {
            state.loaded = new.loaded;
            state.installed = new.installed;
            state.updates = new.updates;
            state.updating = new.updating;
            state.last_update_res = new.last_update_res;
            state.selected_package = new.selected_package;
            state.selected_package_info = new.selected_package_info;
            state.loading_info = new.loading_info;
            state.uninstalling = new.uninstalling;
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
        PackagesMessage::StartUninstall(_name) => {
            state.uninstalling = true;
        }
        PackagesMessage::UninstallFinished(res) => {
            state.uninstalling = false;
            match res {
                Ok(_) => {
                    state.selected_package = None;
                    state.selected_package_info = None;
                }
                Err(err) => {
                    state.selected_package_info = Some(format!("Uninstall failed: {}", err));
                }
            }
        }
        PackagesMessage::SelectAndScrollPackage(name) => {
            state.select_and_scroll_to(&name);
        }
    }
}

impl PackagesState {
    pub fn select_and_scroll_to(&mut self, pkg_name: &str) {
        self.active_tab = PackageTab::Installed;
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
    // Sections: [Packages, Package Info, System Update]
    fn section_widgets(&mut self) -> Vec<Vec<cce_ui::widget::WidgetId>> {
        vec![
            vec![self.search_box.id()],
            Vec::new(),
            Vec::new(),
        ]
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
                let filtered: Vec<&PackageInfo> = self.installed.iter()
                    .filter(|p| p.name.to_lowercase().contains(&query) || p.version.to_lowercase().contains(&query))
                    .collect();
                for (idx, item) in self.installed_items.iter_mut().enumerate() {
                    if item.just_clicked {
                        item.just_clicked = false;
                        if idx < filtered.len() {
                            let pkg = filtered[idx];
                            actions.push(AppAction::Packages(PackagesMessage::SelectPackage(Some(pkg.name.clone()))));
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

    fn extra_dispatch_roots(&mut self) -> Vec<cce_ui::widget::WidgetId> {
        match self.active_tab {
            PackageTab::Installed => self.installed_items.iter().map(|i| i.id()).collect(),
            PackageTab::Updates => self.updates_items.iter().map(|i| i.id()).collect(),
        }
    }

    fn register_extra_dispatch_roots(&mut self, ctx: &mut cce_ui::context::UiContext) {
        let items = match self.active_tab {
            PackageTab::Installed => &mut self.installed_items,
            PackageTab::Updates => &mut self.updates_items,
        };
        for i in items.iter_mut() {
            let (id, ptr) = (i.id(), i.as_ptr_mut());
            ctx.register_widget(id, ptr);
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_layout_grid() {
        let mut state = PackagesState::default();
        let mut layout = cce_ui::layout::ColumnLayout::new(20.0);
        let pc = view(&mut state, 10.0, 20.0, 800.0, 600.0, &[false, false], &mut layout, &mut cce_ui::context::UiContext::new());
        assert!(!pc.rects.is_empty() || !pc.texts.is_empty() || !pc.buttons.is_empty());
    }
}
