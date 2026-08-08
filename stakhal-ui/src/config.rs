use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub project_dir: Option<String>,
}

pub fn get_config_file_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join(".config").join("stakhal").join("last_project.json"))
}

pub fn load_app_config() -> AppConfig {
    if let Some(config_path) = get_config_file_path() {
        if let Ok(content) = fs::read_to_string(config_path) {
            if let Ok(config) = serde_json::from_str::<AppConfig>(&content) {
                return config;
            }
        }
    }
    AppConfig::default()
}

pub fn save_app_config(dir: &str) {
    if let Some(config_path) = get_config_file_path() {
        if let Some(parent) = config_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let config = AppConfig {
            project_dir: Some(dir.to_string()),
        };
        if let Ok(json) = serde_json::to_string_pretty(&config) {
            let _ = fs::write(config_path, json);
        }
    }
}
