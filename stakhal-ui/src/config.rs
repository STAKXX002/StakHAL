use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigValidationError {
    InvalidTarget {
        field: &'static str,
        value: String,
        expected: &'static str,
    },
    InvalidProjectDir {
        field: &'static str,
        value: String,
    },
}

impl fmt::Display for ConfigValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigValidationError::InvalidTarget {
                field,
                value,
                expected,
            } => {
                write!(
                    f,
                    "invalid value for field '{field}': unknown target '{value}', expected one of {expected}"
                )
            }
            ConfigValidationError::InvalidProjectDir { field, value } => {
                write!(
                    f,
                    "invalid value for field '{field}': project directory does not exist '{value}'"
                )
            }
        }
    }
}

impl Error for ConfigValidationError {}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct AppConfig {
    pub project_dir: Option<String>,
    #[serde(default)]
    pub target: Option<String>,
}

impl AppConfig {
    /// Validate config values, naming the offending field on error.
    pub fn validate(&self) -> Result<(), ConfigValidationError> {
        if let Some(ref tgt) = self.target {
            if stakhal_core::target::TargetDefinition::from_name(tgt).is_none() {
                return Err(ConfigValidationError::InvalidTarget {
                    field: "target",
                    value: tgt.clone(),
                    expected: "AA_NS_STM, AA_NS_STM32",
                });
            }
        }
        Ok(())
    }
}

pub fn get_config_file_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(
        PathBuf::from(home)
            .join(".config")
            .join("stakhal")
            .join("last_project.json"),
    )
}

pub fn load_app_config() -> AppConfig {
    if let Some(config_path) = get_config_file_path() {
        if let Ok(content) = fs::read_to_string(config_path) {
            if let Ok(config) = serde_json::from_str::<AppConfig>(&content) {
                if config.validate().is_ok() {
                    return config;
                }
            }
        }
    }
    AppConfig::default()
}

pub fn save_app_config(dir: &str) {
    save_app_config_with_target(dir, None);
}

pub fn save_app_config_with_target(dir: &str, target: Option<String>) {
    if let Some(config_path) = get_config_file_path() {
        if let Some(parent) = config_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let config = AppConfig {
            project_dir: Some(dir.to_string()),
            target,
        };
        if let Ok(json) = serde_json::to_string_pretty(&config) {
            let _ = fs::write(config_path, json);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_config_validation_valid_targets() {
        let mut config = AppConfig {
            project_dir: Some("/tmp/project".to_string()),
            target: Some("AA_NS_STM".to_string()),
        };
        assert!(config.validate().is_ok());

        config.target = Some("AA_NS_STM32".to_string());
        assert!(config.validate().is_ok());

        config.target = None;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_app_config_validation_invalid_target_names_field() {
        let config = AppConfig {
            project_dir: Some("/tmp/project".to_string()),
            target: Some("NON_EXISTENT_TARGET".to_string()),
        };
        let err = config.validate().unwrap_err();
        match err {
            ConfigValidationError::InvalidTarget {
                field, ref value, ..
            } => {
                assert_eq!(field, "target");
                assert_eq!(value, "NON_EXISTENT_TARGET");
            }
            _ => panic!("Expected InvalidTarget error variant"),
        }
        assert!(err.to_string().contains("field 'target'"));
    }

    #[test]
    fn test_app_config_serialization_roundtrip() {
        let config = AppConfig {
            project_dir: Some("/path/to/firmware".to_string()),
            target: Some("AA_NS_STM32".to_string()),
        };
        let json = serde_json::to_string(&config).unwrap();
        let decoded: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, decoded);
    }

    #[test]
    fn test_app_config_serialization_does_not_contain_easter_egg_unlocked() {
        let config = AppConfig {
            project_dir: Some("/path/to/project".to_string()),
            target: Some("AA_NS_STM32".to_string()),
        };
        let serialized = serde_json::to_string(&config).unwrap();
        assert!(
            !serialized.contains("easter_egg"),
            "AppConfig serialization must not contain easter egg fields"
        );
        assert!(
            !serialized.contains("unlocked"),
            "AppConfig serialization must not contain unlock state"
        );

        let roundtrip: AppConfig = serde_json::from_str(&serialized).unwrap();
        assert_eq!(config, roundtrip);
    }

    #[test]
    fn test_app_config_serialization_contains_no_skin_state() {
        let config = AppConfig {
            project_dir: Some("/path/to/project".to_string()),
            target: Some("AA_NS_STM32".to_string()),
        };
        let serialized = serde_json::to_string_pretty(&config).unwrap();
        let lower = serialized.to_ascii_lowercase();
        for forbidden in [
            "skin",
            "stage",
            "palette",
            "theme",
            "easter_egg",
            "unlocked",
        ] {
            assert!(
                !lower.contains(forbidden),
                "AppConfig serialization must not contain skin/easter-egg field '{forbidden}'"
            );
        }

        let parsed: serde_json::Value = serde_json::from_str(&serialized).unwrap();
        let obj = parsed.as_object().expect("AppConfig serializes as object");
        assert_eq!(obj.len(), 2);
        assert!(obj.contains_key("project_dir"));
        assert!(obj.contains_key("target"));
    }
}
