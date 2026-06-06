use crate::app::{AppAction, PageContent};
use clear_ui::layout::{Section, PageLayoutBuilder, LayoutStrategy};
use std::fs;

#[derive(Debug, Clone, Default)]
pub struct BackupState {
    pub loaded: bool,
    pub in_progress: bool,
    pub last_backup_time: String,
    pub backup_size: String,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub enum BackupMessage {
    Refreshed(BackupState),
    StartBackup,
    BackupFinished(Result<(String, String), String>),
}

fn status_path() -> String {
    format!("{}/.config/clear-system-interface/backup_status.txt", std::env::var("HOME").unwrap_or_default())
}

pub fn read_backup_status() -> (String, String, Option<String>) {
    let path_str = status_path();
    let content = fs::read_to_string(path_str).unwrap_or_default();
    
    let mut last_backup = "Never".to_string();
    let mut size = "0 B".to_string();
    let mut err_msg = None;
    
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("last_backup_time") {
            if let Some(val) = trimmed.split('=').nth(1) {
                last_backup = val.trim().to_string();
            }
        } else if trimmed.starts_with("backup_size") {
            if let Some(val) = trimmed.split('=').nth(1) {
                size = val.trim().to_string();
            }
        } else if trimmed.starts_with("error_message") {
            if let Some(val) = trimmed.split('=').nth(1) {
                let v = val.trim().to_string();
                if !v.is_empty() {
                    err_msg = Some(v);
                }
            }
        }
    }
    
    (last_backup, size, err_msg)
}

pub async fn fetch_backup_state() -> BackupState {
    let (last_backup, size, err) = read_backup_status();
    BackupState {
        loaded: true,
        in_progress: false,
        last_backup_time: last_backup,
        backup_size: size,
        error_message: err,
    }
}

pub async fn run_backup() -> Result<(String, String), String> {
    // Run the backup system helper script via pkexec (graphical auth prompt)
    let output = tokio::process::Command::new("pkexec")
        .arg("/home/lsgalante/.local/share/clear-system-interface/helpers/backup-system.sh")
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

const LABEL_FG: [f32; 4] = [0.56, 0.83, 0.56, 1.0];
const TEXT_FG: [f32; 4] = [0.83, 0.83, 0.83, 1.0];
const TEXT_DIM: [f32; 4] = [0.53, 0.53, 0.60, 1.0];
const RED: [f32; 4] = [1.0, 0.33, 0.33, 1.0];
const GREEN: [f32; 4] = [0.36, 0.56, 0.38, 1.0];
const BTN_BG: [f32; 4] = [0.20, 0.40, 0.65, 1.0];
const BTN_HOVER: [f32; 4] = [0.28, 0.50, 0.78, 1.0];
const BTN_DISABLED: [f32; 4] = [0.15, 0.18, 0.22, 1.0];
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

pub fn view(state: &BackupState, cx: f32, cy: f32, cw: f32, ch: f32, layout: &mut dyn LayoutStrategy) -> PageContent {
    let mut final_pc = PageContent::new();
    let sec_w = 320.0f32;
    let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(1);

    builder.add_section(&mut final_pc, |pc, rx, ry| {
        let mut sec = Section::new(pc, rx, ry, sec_w, "Full System Backup");

        if !state.loaded {
            sec.text(pc, "Loading backup state...", 12.0, 0.0, 12.0, TEXT_DIM);
            sec.spacing(18.0);
        } else {
            // Status Row
            sec.text(pc, "Backup Status", 12.0, 0.0, 12.0, LABEL_FG);
            let status_text = if state.in_progress { "Backing up..." } else { "Idle" };
            let status_color = if state.in_progress { GREEN } else { TEXT_FG };
            sec.text(pc, status_text, 120.0, 0.0, 12.0, status_color);
            sec.spacing(18.0);

            // Last Backup Row
            sec.text(pc, "Last Backup", 12.0, 0.0, 12.0, LABEL_FG);
            sec.text(pc, &state.last_backup_time, 120.0, 0.0, 12.0, TEXT_FG);
            sec.spacing(18.0);

            // Backup Size Row
            sec.text(pc, "Archive Size", 12.0, 0.0, 12.0, LABEL_FG);
            sec.text(pc, &state.backup_size, 120.0, 0.0, 12.0, TEXT_FG);
            sec.spacing(18.0);

            // Target Directories Row
            sec.text(pc, "Backup Targets", 12.0, 0.0, 12.0, LABEL_FG);
            sec.text(pc, "Entire Filesystem (/)  [Preserving attributes]", 120.0, 0.0, 12.0, TEXT_DIM);
            sec.spacing(18.0);

            // Destination Archive Row
            sec.text(pc, "Destination", 12.0, 0.0, 12.0, LABEL_FG);
            sec.text(pc, "USB Drive (/mnt/usb or /run/media/...)", 120.0, 0.0, 12.0, TEXT_DIM);
            sec.spacing(24.0);

            // Error message if present
            if let Some(ref err) = state.error_message {
                sec.text(pc, "Error:", 12.0, 0.0, 12.0, RED);
                sec.text(pc, err, 60.0, 0.0, 11.0, RED);
                sec.spacing(18.0);
            }

            // Action Button
            let btn_w = 120.0;
            let btn_h = 32.0;
            let yt = sec.ay();
            
            let (btn_label, bg, hover, action) = if state.in_progress {
                ("Backing up...", BTN_DISABLED, BTN_DISABLED, AppAction::Backup(BackupMessage::StartBackup))
            } else {
                ("Run Backup", BTN_BG, BTN_HOVER, AppAction::Backup(BackupMessage::StartBackup))
            };
            
            sec.row(1, 0.0, btn_h, |_, x, _| {
                pc.button(btn_label, x, yt, btn_w, btn_h, bg, hover, WHITE, action.clone());
            });
            sec.spacing(12.0);
        }
        sec.finish(pc)
    });

    final_pc
}

pub fn update(state: &mut BackupState, msg: BackupMessage) {
    match msg {
        BackupMessage::Refreshed(new) => {
            let in_prog = state.in_progress;
            *state = new;
            state.in_progress = in_prog;
        }
        BackupMessage::StartBackup => {
            state.in_progress = true;
            state.error_message = None;
        }
        BackupMessage::BackupFinished(res) => {
            state.in_progress = false;
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
