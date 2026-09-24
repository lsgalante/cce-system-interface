use crate::app::{AppAction, PageContent};
use cce_ui::layout::{render_widget, PageLayoutBuilder, LayoutStrategy};
use std::fs;

/// What the background poll produces — the fetched numbers only, never the
/// widgets. `Refreshed` carries this rather than a whole `StorageState` so a
/// refresh cannot clobber `backup_button` (the notifications-page pattern; the
/// old `*state = new` would have swapped the live widget for a fresh one with a
/// different id, mid-frame, every ten seconds).
#[derive(Debug, Clone)]
pub struct StorageInfo {
    pub disk_total: f64,
    pub disk_used: f64,
    pub ram_total: f64,
    pub ram_used: f64,
    pub last_backup_time: String,
    pub backup_size: String,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StorageState {
    pub disk_total: f64,
    pub disk_used: f64,
    pub ram_total: f64,
    pub ram_used: f64,
    pub loaded: bool,

    // Backup states
    pub backup_loaded: bool,
    pub backup_in_progress: bool,
    pub last_backup_time: String,
    pub backup_size: String,
    pub error_message: Option<String>,
    /// Retained so it can hold keyboard focus: ctrl+i descends into the Full
    /// System Backup section and lands here, and Enter/Space runs the backup.
    pub backup_button: cce_ui::widget::Adapted<cce_ui::widget::Button>,
}

impl Default for StorageState {
    fn default() -> Self {
        Self {
            disk_total: 0.0,
            disk_used: 0.0,
            ram_total: 0.0,
            ram_used: 0.0,
            loaded: false,
            backup_loaded: false,
            backup_in_progress: false,
            last_backup_time: "Never".to_string(),
            backup_size: "0 B".to_string(),
            error_message: None,
            backup_button: cce_ui::widget::Button::new(0.0, 0.0, 0.0, 32.0)
                // Flat fill + border, not the SDF bevel: PageContent's RenderTarget
                // has no `bevel`, so a raised plate silently draws nothing here
                // (the label renders, the plate does not). The border path is also
                // what carries the keyboard focus ring.
                .with_raised(false)
                .with_label("Run Backup")
                .with_bg(BTN_BG)
                .with_hover_bg(BTN_HOVER)
                .with_label_color(WHITE),
        }
    }
}

#[derive(Debug, Clone)]
pub enum StorageMessage {
    Refreshed(StorageInfo),
    StartBackup,
    BackupFinished(Result<(String, String), String>),
}

/// Where the privileged helper records the last backup's outcome. Passed to
/// the script as an argument rather than hardcoded on both sides: the script
/// runs as root under pkexec, which scrubs the environment, so it cannot
/// resolve this itself — and when both sides did hardcode it they drifted
/// apart across the app's renames (the script wrote
/// `~/.config/clear-system-interface/`, this read `cce-settings/`, and neither
/// path existed).
fn status_path() -> String {
    cce_ui::config::cce_config_dir()
        .join("backup_status.txt")
        .to_string_lossy()
        .into_owned()
}

/// Parse the status file the privileged helper writes. Split out from the read
/// so the format — the one contract shared across the pkexec boundary — can be
/// tested against the exact bytes `scripts/backup-system.sh` emits.
///
/// `splitn(2, '=')` and not `split('=').nth(1)`: an error message is free text
/// and may well contain an `=`, which the latter silently truncated.
fn parse_backup_status(content: &str) -> (String, String, Option<String>) {
    let mut last_backup = "Never".to_string();
    let mut size = "0 B".to_string();
    let mut err_msg = None;

    for line in content.lines() {
        let trimmed = line.trim();
        let Some((key, val)) = trimmed.split_once('=') else { continue };
        let val = val.trim().to_string();
        match key.trim() {
            "last_backup_time" => last_backup = val,
            "backup_size" => size = val,
            "error_message" => {
                if !val.is_empty() {
                    err_msg = Some(val);
                }
            }
            _ => {}
        }
    }

    (last_backup, size, err_msg)
}

pub fn read_backup_status() -> (String, String, Option<String>) {
    parse_backup_status(&fs::read_to_string(status_path()).unwrap_or_default())
}

pub async fn fetch_storage_state() -> StorageInfo {
    let disk_output = tokio::process::Command::new("df")
        .args(["-BG", "/"])
        .output().await.ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();
    let (disk_total, disk_used) = parse_disk(&disk_output);

    let mem_output = tokio::process::Command::new("free")
        .args(["-b"])
        .output().await.ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();
    let (ram_total, ram_used) = parse_mem(&mem_output);

    let (last_backup, size, err) = read_backup_status();

    StorageInfo {
        disk_total,
        disk_used,
        ram_total,
        ram_used,
        last_backup_time: last_backup,
        backup_size: size,
        error_message: err,
    }
}

pub async fn run_backup() -> Result<(String, String), String> {
    // Runs the SAME helper the nightly restic-backup.timer runs, so this
    // button means "run tonight's backup now" rather than a second, different
    // backup. It used to invoke backup-system.sh under pkexec, which archived
    // only the root btrfs subvolume — /home has its own st_dev, so
    // `tar --one-file-system` stopped there and none of the user's data was in
    // it — and then wrote its result into the same status file, so the page
    // reported an OS-only tarball as if it were the nightly job.
    //
    // No pkexec: restic backs up the user's own files as the user, so the
    // privilege prompt bought nothing and made unattended runs impossible.
    // `$CCE_PREFIX/bin`, else `~/.local/bin` — ccebuild's own BINDIR, which is
    // where it installs every crate's scripts/.
    let helper = std::env::var("CCE_PREFIX")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".local")
        })
        .join("bin")
        .join("restic-backup.sh");
    let output = tokio::process::Command::new(helper)
        .arg(status_path())
        .output()
        .await
        .map_err(|e| format!("Failed to run backup script: {}", e))?;
        
    if !output.status.success() {
        // Retrieve any specific error message written to the status file by the script
        let (_, _, err_msg) = read_backup_status();
        if let Some(msg) = err_msg {
            return Err(msg);
        }
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(format!("Backup process failed: {}", err));
    }
    
    let (last_backup, size, _) = read_backup_status();
    Ok((last_backup, size))
}

fn parse_disk(info: &str) -> (f64, f64) {
    for line in info.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 4 {
            let total = parts[1].trim_end_matches('G').parse::<f64>().unwrap_or(0.0);
            let used = parts[2].trim_end_matches('G').parse::<f64>().unwrap_or(0.0);
            return (total, used);
        }
    }
    (0.0, 0.0)
}

fn parse_mem(info: &str) -> (f64, f64) {
    for line in info.lines() {
        if line.starts_with("Mem:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let total = parts[1].parse::<f64>().unwrap_or(0.0) / 1_073_741_824.0;
                let used = parts[2].parse::<f64>().unwrap_or(0.0) / 1_073_741_824.0;
                return (total, used);
            }
        }
    }
    (0.0, 0.0)
}

const LABEL_FG: [f32; 4] = [0.56, 0.83, 0.56, 1.0];
const TEXT_FG: [f32; 4] = [0.83, 0.83, 0.83, 1.0];
const TEXT_DIM: [f32; 4] = [0.53, 0.53, 0.60, 1.0];
const RED: [f32; 4] = [1.0, 0.33, 0.33, 1.0];
const GREEN: [f32; 4] = [0.36, 0.56, 0.38, 1.0];
const BTN_BG: [f32; 4] = [0.20, 0.40, 0.65, 1.0];
const BTN_HOVER: [f32; 4] = [0.28, 0.50, 0.78, 1.0];
const BTN_DISABLED: [f32; 4] = [0.15, 0.18, 0.22, 1.0];
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

pub fn view(state: &mut StorageState, cx: f32, cy: f32, cw: f32, ch: f32, sec_focused: &[bool], layout: &mut dyn LayoutStrategy, ctx: &mut cce_ui::context::UiContext) -> PageContent {
    let mut final_pc = PageContent::new();
    let sec_w = 320.0f32;
    let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(3);

    // Section 1: Local Storage
    builder.add_section(&mut final_pc, "Local Storage", sec_focused.first().copied().unwrap_or(false), |sec| {
        let sec_w = sec.cw;
        if !state.loaded {
            sec.text("Loading storage usage...", 12.0, 0.0, 12.0, TEXT_FG);
        } else {
            let disk_pct = if state.disk_total > 0.0 {
                state.disk_used / state.disk_total * 100.0
            } else {
                0.0
            };

            sec.text("Disk", 12.0, 0.0, 12.0, LABEL_FG);
            sec.text(
                &format!("{:.0} / {:.0} GiB  ({:.0}%)", state.disk_used, state.disk_total, disk_pct),
                100.0, 0.0, 12.0, TEXT_FG,
            );

            let bar_w = sec_w - 2.0 * crate::app::section_margin();
            let yt = sec.ay();
            let disk_bar_x = sec.ax(crate::app::section_margin());
            let mut disk_bar = cce_ui::widget::UsageBar::new((disk_pct as f32 / 100.0).min(1.0))
                .with_colors([0.36, 0.60, 0.36, 1.0], [0.15, 0.15, 0.25, 1.0]);
            render_widget(sec.pc, &mut disk_bar, disk_bar_x, yt, bar_w, 8.0, ctx);
        }
    });

    // Section 2: Memory
    builder.add_section(&mut final_pc, "Memory", sec_focused.get(1).copied().unwrap_or(false), |sec| {
        let sec_w = sec.cw;
        if !state.loaded {
            sec.text("Loading memory usage...", 12.0, 0.0, 12.0, TEXT_FG);
        } else {
            let ram_pct = if state.ram_total > 0.0 {
                state.ram_used / state.ram_total * 100.0
            } else {
                0.0
            };

            sec.text("RAM", 12.0, 0.0, 12.0, LABEL_FG);
            sec.text(
                &format!("{:.1} / {:.1} GiB  ({:.0}%)", state.ram_used, state.ram_total, ram_pct),
                100.0, 0.0, 12.0, TEXT_FG,
            );

            let bar_w = sec_w - 2.0 * crate::app::section_margin();
            let yt = sec.ay();
            let ram_bar_x = sec.ax(crate::app::section_margin());
            let mut ram_bar = cce_ui::widget::UsageBar::new((ram_pct as f32 / 100.0).min(1.0))
                .with_colors([0.50, 0.50, 0.65, 1.0], [0.15, 0.15, 0.25, 1.0]);
            render_widget(sec.pc, &mut ram_bar, ram_bar_x, yt, bar_w, 8.0, ctx);
        }
    });

    // Section 2: Full System Backup
    builder.add_section(&mut final_pc, "Full System Backup", sec_focused.get(2).copied().unwrap_or(false), |sec| {
        if !state.backup_loaded {
            sec.text("Loading backup state...", 12.0, 0.0, 12.0, TEXT_DIM);
        } else {
            // Status Row
            sec.text("Backup Status", 12.0, 0.0, 12.0, LABEL_FG);
            let status_text = if state.backup_in_progress { "Backing up..." } else { "Idle" };
            let status_color = if state.backup_in_progress { GREEN } else { TEXT_FG };
            sec.text(status_text, 120.0, 0.0, 12.0, status_color);

            // Last Backup Row
            sec.text("Last Backup", 12.0, 0.0, 12.0, LABEL_FG);
            sec.text(&state.last_backup_time, 120.0, 0.0, 12.0, TEXT_FG);

            // Backup Size Row
            sec.text("Archive Size", 12.0, 0.0, 12.0, LABEL_FG);
            sec.text(&state.backup_size, 120.0, 0.0, 12.0, TEXT_FG);

            // Target Directories Row
            sec.text("Backup Targets", 12.0, 0.0, 12.0, LABEL_FG);
            sec.text("Entire Filesystem (/)  [Preserving attributes]", 120.0, 0.0, 12.0, TEXT_DIM);

            // Destination Archive Row
            sec.text("Destination", 12.0, 0.0, 12.0, LABEL_FG);
            sec.text("USB Drive (/mnt/usb or /run/media/...)", 120.0, 0.0, 12.0, TEXT_DIM);

            // Error message if present
            if let Some(ref err) = state.error_message {
                sec.text("Error:", 12.0, 0.0, 12.0, RED);
                sec.text(err, 60.0, 0.0, 11.0, RED);
            }

            // Action Button
            let mut stack = sec.vstack(cce_ui::layout::plate_gap());
            let btn_h = 32.0;
            
            // Retained widget rather than an immediate `sec.button`, so it can hold
            // keyboard focus. Its label/colours are re-synced each frame from the
            // backup state, the way cce-mail drives its retained btn_unread.
            let (btn_label, bg, hover) = if state.backup_in_progress {
                ("Backing up...", BTN_DISABLED, BTN_DISABLED)
            } else {
                ("Run Backup", BTN_BG, BTN_HOVER)
            };
            state.backup_button.set_label(btn_label);
            state.backup_button.bg = Some(bg);
            state.backup_button.hover_bg = Some(hover);

            let btn = &mut state.backup_button;
            stack.add_row(1, 0.0, btn_h, |sctx, _, x, w| {
                let y = sctx.ay();
                render_widget(sctx.pc, btn, x, y, w, btn_h, ctx);
            });
        }
    });

    final_pc
}

pub fn update(state: &mut StorageState, msg: StorageMessage) {
    match msg {
        StorageMessage::Refreshed(new) => {
            // Field-wise, so the retained button and an in-flight backup survive.
            state.disk_total = new.disk_total;
            state.disk_used = new.disk_used;
            state.ram_total = new.ram_total;
            state.ram_used = new.ram_used;
            state.last_backup_time = new.last_backup_time;
            state.backup_size = new.backup_size;
            state.error_message = new.error_message;
            state.loaded = true;
            state.backup_loaded = true;
        }
        StorageMessage::StartBackup => {
            state.backup_in_progress = true;
            state.error_message = None;
        }
        StorageMessage::BackupFinished(res) => {
            state.backup_in_progress = false;
            match res {
                Ok((date, size)) => {
                    state.last_backup_time = date;
                    state.backup_size = size;
                    state.error_message = None;
                }
                Err(err) => {
                    state.error_message = Some(err);
                }
            }
        }
    }
}

impl crate::pages::AppPage for StorageState {
    // Sections: [Local Storage, Memory, Full System Backup] — no evented widgets.
    fn section_widgets(&mut self) -> Vec<Vec<cce_ui::widget::WidgetId>> {
        // Only the third section (Full System Backup) has anything focusable;
        // the first two are read-only readouts, so ctrl+i there has no target.
        // Gated on `backup_loaded` to mirror the view: the button is only painted
        // (and so only registered) in that branch, and an id reported here while
        // unregistered is a dead root the router drops with a warning.
        let backup = if self.backup_loaded {
            vec![self.backup_button.id()]
        } else {
            Vec::new()
        };
        vec![Vec::new(), Vec::new(), backup]
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
        // Mouse click and Enter/Space on the focused button both land here.
        if self.backup_button.take_click() && !self.backup_in_progress {
            actions.push(AppAction::Storage(StorageMessage::StartBackup));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Byte-for-byte what `scripts/backup-system.sh` writes on a failed run
    /// (its `write_status` heredoc). This pins the one contract that crosses
    /// the pkexec boundary — the two sides previously drifted onto different
    /// paths AND different spellings without anything noticing.
    #[test]
    fn parses_the_helpers_failure_status() {
        let written = "last_backup_time = Never\nbackup_size = 0 B\nerror_message = No external drive mounted at /mnt/usb or /run/media/lsgalante/*\n";
        let (time, size, err) = parse_backup_status(written);
        assert_eq!(time, "Never");
        assert_eq!(size, "0 B");
        assert_eq!(err.as_deref(), Some("No external drive mounted at /mnt/usb or /run/media/lsgalante/*"));
    }

    #[test]
    fn parses_the_helpers_success_status() {
        // A success run writes an EMPTY error_message; that must read as None,
        // not as Some(""), or the page reports a failure after a good backup.
        let written = "last_backup_time = 2026-08-23 11:04:12\nbackup_size = 41G\nerror_message = \n";
        let (time, size, err) = parse_backup_status(written);
        assert_eq!(time, "2026-08-23 11:04:12");
        assert_eq!(size, "41G");
        assert_eq!(err, None);
    }

    #[test]
    fn an_error_message_may_contain_an_equals_sign() {
        // `split('=').nth(1)` truncated at the second '='; free text can hold one.
        let (_, _, err) = parse_backup_status("error_message = tar: bad option --foo=bar\n");
        assert_eq!(err.as_deref(), Some("tar: bad option --foo=bar"));
    }

    #[test]
    fn a_missing_status_file_reads_as_never() {
        let (time, size, err) = parse_backup_status("");
        assert_eq!((time.as_str(), size.as_str(), err), ("Never", "0 B", None));
    }
}
