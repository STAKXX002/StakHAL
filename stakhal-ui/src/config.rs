use crate::toolchain::flasher::FlashBackend;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub project_dir: Option<String>,
    #[serde(default)]
    pub flash_backend: FlashBackend,
}

pub fn get_config_file_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    let dir = PathBuf::from(home).join(".config").join("stakhal");
    let new_path = dir.join("config.json");
    if new_path.exists() {
        return Some(new_path);
    }
    let old_path = dir.join("last_project.json");
    if old_path.exists() {
        return Some(old_path);
    }
    Some(new_path)
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

pub fn save_config(config: &AppConfig) {
    if let Some(config_path) = get_config_file_path() {
        if let Some(parent) = config_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(config) {
            let _ = fs::write(config_path, json);
        }
    }
}

pub fn save_app_config(dir: &str) {
    let mut config = load_app_config();
    config.project_dir = Some(dir.to_string());
    save_config(&config);
}

pub fn save_flash_backend_config(backend: FlashBackend) {
    let mut config = load_app_config();
    config.flash_backend = backend;
    save_config(&config);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_config_default() {
        let config = AppConfig::default();
        assert_eq!(config.project_dir, None);
        assert_eq!(config.flash_backend, FlashBackend::StLink);
    }

    #[test]
    fn test_app_config_backward_compat_deserialization() {
        let json = r#"{"project_dir": "/home/user/project"}"#;
        let config: AppConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.project_dir, Some("/home/user/project".to_string()));
        assert_eq!(config.flash_backend, FlashBackend::StLink);

        let empty_json = r#"{}"#;
        let empty_config: AppConfig = serde_json::from_str(empty_json).unwrap();
        assert_eq!(empty_config.project_dir, None);
        assert_eq!(empty_config.flash_backend, FlashBackend::StLink);
    }

    #[test]
    fn test_app_config_round_trip() {
        let config = AppConfig {
            project_dir: Some("/path/to/my_proj".to_string()),
            flash_backend: FlashBackend::Dfu,
        };
        let json = serde_json::to_string_pretty(&config).unwrap();
        let loaded: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, loaded);
        assert_eq!(loaded.flash_backend, FlashBackend::Dfu);
    }
}
