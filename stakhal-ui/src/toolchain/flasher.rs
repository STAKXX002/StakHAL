use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::Path;
use std::str::FromStr;

pub const STM32_FLASH_BASE_ADDR: &str = "0x08000000";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FlashBackend {
    #[default]
    #[serde(
        rename = "stlink",
        alias = "StLink",
        alias = "st_link",
        alias = "ST-Link",
        alias = "st-link"
    )]
    StLink,
    #[serde(rename = "dfu", alias = "Dfu", alias = "DFU")]
    Dfu,
}

impl fmt::Display for FlashBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FlashBackend::StLink => write!(f, "ST-Link"),
            FlashBackend::Dfu => write!(f, "DFU"),
        }
    }
}

impl FromStr for FlashBackend {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().replace(['-', '_'], "").as_str() {
            "stlink" => Ok(FlashBackend::StLink),
            "dfu" => Ok(FlashBackend::Dfu),
            other => Err(format!("Unknown flash backend: {}", other)),
        }
    }
}

/// Construct the arguments for `st-flash write`.
/// Uses `--reset` prior to `write` so st-flash resets the MCU immediately after programming.
pub fn build_flash_command(
    probe_serial: Option<&str>,
    artifact_path: &Path,
) -> (String, Vec<String>) {
    let mut args = Vec::new();
    if let Some(serial) = probe_serial {
        if !serial.is_empty() {
            args.push("--serial".to_string());
            args.push(serial.to_string());
        }
    }
    args.push("--reset".to_string());
    args.push("write".to_string());
    args.push(artifact_path.display().to_string());
    args.push(STM32_FLASH_BASE_ADDR.to_string());

    ("st-flash".to_string(), args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_build_flash_command_without_serial() {
        let path = PathBuf::from("/path/to/build/firmware.bin");
        let (cmd, args) = build_flash_command(None, &path);
        assert_eq!(cmd, "st-flash");
        assert_eq!(
            args,
            vec![
                "--reset".to_string(),
                "write".to_string(),
                "/path/to/build/firmware.bin".to_string(),
                "0x08000000".to_string(),
            ]
        );
    }

    #[test]
    fn test_build_flash_command_with_serial() {
        let path = PathBuf::from("/path/to/build/firmware.bin");
        let (cmd, args) = build_flash_command(Some("003800333433510937363934"), &path);
        assert_eq!(cmd, "st-flash");
        assert_eq!(
            args,
            vec![
                "--serial".to_string(),
                "003800333433510937363934".to_string(),
                "--reset".to_string(),
                "write".to_string(),
                "/path/to/build/firmware.bin".to_string(),
                "0x08000000".to_string(),
            ]
        );
    }

    #[test]
    fn test_flash_backend_default_and_display() {
        assert_eq!(FlashBackend::default(), FlashBackend::StLink);
        assert_eq!(FlashBackend::StLink.to_string(), "ST-Link");
        assert_eq!(FlashBackend::Dfu.to_string(), "DFU");
    }

    #[test]
    fn test_flash_backend_from_str() {
        assert_eq!(
            "stlink".parse::<FlashBackend>().unwrap(),
            FlashBackend::StLink
        );
        assert_eq!(
            "ST-Link".parse::<FlashBackend>().unwrap(),
            FlashBackend::StLink
        );
        assert_eq!(
            "st_link".parse::<FlashBackend>().unwrap(),
            FlashBackend::StLink
        );
        assert_eq!("dfu".parse::<FlashBackend>().unwrap(), FlashBackend::Dfu);
        assert_eq!("DFU".parse::<FlashBackend>().unwrap(), FlashBackend::Dfu);
        assert!("invalid".parse::<FlashBackend>().is_err());
    }

    #[test]
    fn test_flash_backend_serde_round_trip() {
        let serialized = serde_json::to_string(&FlashBackend::StLink).unwrap();
        assert_eq!(serialized, "\"stlink\"");
        let deserialized: FlashBackend = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, FlashBackend::StLink);

        let dfu_ser = serde_json::to_string(&FlashBackend::Dfu).unwrap();
        assert_eq!(dfu_ser, "\"dfu\"");
        let dfu_de: FlashBackend = serde_json::from_str(&dfu_ser).unwrap();
        assert_eq!(dfu_de, FlashBackend::Dfu);

        // Also test alias deserialization
        let from_pascal: FlashBackend = serde_json::from_str("\"StLink\"").unwrap();
        assert_eq!(from_pascal, FlashBackend::StLink);
        let from_caps: FlashBackend = serde_json::from_str("\"DFU\"").unwrap();
        assert_eq!(from_caps, FlashBackend::Dfu);
    }
}
