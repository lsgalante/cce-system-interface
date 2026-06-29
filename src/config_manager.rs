use std::fs;
use serde_json::Value;

pub const CONFIG_PATH: &str = "/home/lsgalante/.config/cce/config.kdl";

pub fn read_config_file() -> String {
    fs::read_to_string(CONFIG_PATH).unwrap_or_default()
}

fn perform_rolling_backup(path: &str) {
    if path != CONFIG_PATH {
        return;
    }
    if !std::path::Path::new(path).exists() {
        return;
    }
    let backup_dir = "/home/lsgalante/.config/cce/backups";
    if let Err(_) = fs::create_dir_all(backup_dir) {
        return;
    }
    for i in (1..=4).rev() {
        let src = format!("{}/config.kdl.{}.bak", backup_dir, i);
        let dst = format!("{}/config.kdl.{}.bak", backup_dir, i + 1);
        if std::path::Path::new(&src).exists() {
            let _ = fs::rename(src, dst);
        }
    }
    let dst = format!("{}/config.kdl.1.bak", backup_dir);
    let _ = fs::copy(path, dst);
}

fn safe_write(path: &str, content: &str) -> bool {
    perform_rolling_backup(path);
    let temp_path = format!("{}.tmp", path);
    if fs::write(&temp_path, content).is_ok() {
        if fs::rename(&temp_path, path).is_ok() {
            return true;
        }
        let _ = fs::remove_file(&temp_path);
    }
    false
}

pub fn write_config_file(content: &str) -> bool {
    safe_write(CONFIG_PATH, content)
}

pub fn update_json_value<F>(update_fn: F) -> bool 
where
    F: FnOnce(&mut Value)
{
    let content = read_config_file();
    let mut val: Value = serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}));
    update_fn(&mut val);
    if let Ok(updated_str) = serde_json::to_string_pretty(&val) {
        write_config_file(&updated_str)
    } else {
        false
    }
}
