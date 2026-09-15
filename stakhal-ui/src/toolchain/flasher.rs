use std::path::Path;

pub const STM32_FLASH_BASE_ADDR: &str = "0x08000000";

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

/// Construct the standalone reset command in case a separate reset pulse is needed.
#[allow(dead_code)]
pub fn build_reset_command(probe_serial: Option<&str>) -> (String, Vec<String>) {
    let mut args = Vec::new();
    if let Some(serial) = probe_serial {
        if !serial.is_empty() {
            args.push("--serial".to_string());
            args.push(serial.to_string());
        }
    }
    args.push("reset".to_string());
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
    fn test_build_reset_command() {
        let (cmd, args) = build_reset_command(Some("12345"));
        assert_eq!(cmd, "st-flash");
        assert_eq!(args, vec!["--serial".to_string(), "12345".to_string(), "reset".to_string()]);
    }
}
