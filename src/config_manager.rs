use std::fs;
use serde_json::Value;

pub const CONFIG_PATH: &str = "/home/lsgalante/.config/cce/config.json";

pub fn read_config_file() -> String {
    fs::read_to_string(CONFIG_PATH).unwrap_or_default()
}

pub fn write_config_file(content: &str) -> bool {
    fs::write(CONFIG_PATH, content).is_ok()
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
