use crate::app::{AppAction, PageContent};
use clear_ui::layout::Section;
use std::fs;
use std::path::Path;

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

pub fn read_backup_status() -> (String, String) {
    let path_str = status_path();
    let content = fs::read_to_string(path_str).unwrap_or_default();
    
    let mut last_backup = "Never".to_string();
    let mut size = "0 B".to_string();
    
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
        }
    }
    
    (last_backup, size)
}

fn write_backup_status(last_backup: &str, size: &str) {
    let path_str = status_path();
    let path = Path::new(&path_str);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let content = format!("last_backup_time = {}\nbackup_size = {}\n", last_backup, size);
    let _ = fs::write(path, content);
}

pub async fn fetch_backup_state() -> BackupState {
    let (last_backup, size) = read_backup_status();
    BackupState {
        loaded: true,
        in_progress: false,
        last_backup_time: last_backup,
        backup_size: size,
        error_message: None,
    }
}

pub async fn run_backup() -> Result<(String, String), String> {
    let home = std::env::var("HOME").map_err(|_| "HOME env var not set".to_string())?;
    
    // 1. Ensure target backups directory exists
    let backup_dir = format!("{}/Dropbox/Backups", home);
    tokio::fs::create_dir_all(&backup_dir).await
        .map_err(|e| format!("Failed to create backup dir: {}", e))?;
        
    let archive_path = format!("{}/clear-backup.tar.gz", backup_dir);
    
    // 2. Execute tar command to compress clearwm config and clear projects
    let output = tokio::process::Command::new("tar")
        .args([
            "-czf",
            &archive_path,
            "-C",
            &home,
            ".config/clearwm",
            "Dropbox/Clear",
        ])
        .output()
        .await
        .map_err(|e| format!("Failed to execute tar: {}", e))?;
        
    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(format!("tar error: {}", err_msg));
    }
    
    // 3. Get archive file size
    let size_output = tokio::process::Command::new("du")
        .args(["-sh", &archive_path])
        .output()
        .await
        .map_err(|e| format!("Failed to get archive size: {}", e))?;
        
    let size_str = if size_output.status.success() {
        let stdout = String::from_utf8_lossy(&size_output.stdout);
        stdout.split_whitespace().next().unwrap_or("Unknown").to_string()
    } else {
        "Unknown".to_string()
    };
    
    // 4. Get current formatted time
    let date_output = tokio::process::Command::new("date")
        .arg("+%Y-%m-%d %H:%M:%S")
        .output()
        .await
        .map_err(|e| format!("Failed to get current date: {}", e))?;
        
    let date_str = if date_output.status.success() {
        String::from_utf8_lossy(&date_output.stdout).trim().to_string()
    } else {
        "Unknown Date".to_string()
    };
    
    // 5. Write status back to config file
    write_backup_status(&date_str, &size_str);
        
    Ok((date_str, size_str))
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

pub fn view(state: &BackupState, cx: f32, cy: f32, cw: f32, _ch: f32) -> PageContent {
    let mut pc = PageContent::new();
    let y = cy + 12.0;

    let mut sec = Section::new(&mut pc, cx, y, cw, "System Backup");

    if !state.loaded {
        sec.text(&mut pc, "Loading backup state...", 12.0, 0.0, 12.0, TEXT_DIM);
        sec.spacing(18.0);
    } else {
        // Status Row
        sec.text(&mut pc, "Backup Status", 12.0, 0.0, 12.0, LABEL_FG);
        let status_text = if state.in_progress { "Backing up..." } else { "Idle" };
        let status_color = if state.in_progress { GREEN } else { TEXT_FG };
        sec.text(&mut pc, status_text, 120.0, 0.0, 12.0, status_color);
        sec.spacing(18.0);

        // Last Backup Row
        sec.text(&mut pc, "Last Backup", 12.0, 0.0, 12.0, LABEL_FG);
        sec.text(&mut pc, &state.last_backup_time, 120.0, 0.0, 12.0, TEXT_FG);
        sec.spacing(18.0);

        // Backup Size Row
        sec.text(&mut pc, "Archive Size", 12.0, 0.0, 12.0, LABEL_FG);
        sec.text(&mut pc, &state.backup_size, 120.0, 0.0, 12.0, TEXT_FG);
        sec.spacing(18.0);

        // Target Directories Row
        sec.text(&mut pc, "Backup Targets", 12.0, 0.0, 12.0, LABEL_FG);
        sec.text(&mut pc, "~/.config/clearwm/  •  ~/Dropbox/Clear/", 120.0, 0.0, 12.0, TEXT_DIM);
        sec.spacing(18.0);

        // Destination Archive Row
        sec.text(&mut pc, "Destination", 12.0, 0.0, 12.0, LABEL_FG);
        sec.text(&mut pc, "~/Dropbox/Backups/clear-backup.tar.gz", 120.0, 0.0, 12.0, TEXT_DIM);
        sec.spacing(24.0);

        // Error message if present
        if let Some(ref err) = state.error_message {
            sec.text(&mut pc, "Error:", 12.0, 0.0, 12.0, RED);
            sec.text(&mut pc, err, 60.0, 0.0, 11.0, RED);
            sec.spacing(18.0);
        }

        // Action Button
        let btn_w = 120.0;
        let btn_h = 32.0;
        let yt = sec.ay();
        
        let (btn_label, bg, hover, action) = if state.in_progress {
            ("Backing up...", BTN_DISABLED, BTN_DISABLED, AppAction::Backup(BackupMessage::StartBackup)) // no-op when in progress
        } else {
            ("Run Backup", BTN_BG, BTN_HOVER, AppAction::Backup(BackupMessage::StartBackup))
        };
        
        pc.button(btn_label, sec.ax(12.0), yt, btn_w, btn_h, bg, hover, WHITE, action);
        sec.content_y += btn_h + 12.0;
    }
    sec.finish(&mut pc);

    pc
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
