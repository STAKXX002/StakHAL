use std::process::Command;
use super::runner::is_executable_on_path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StlinkProbe {
    pub serial: String,
    pub version: Option<String>,
    pub dev_type: Option<String>,
    pub chip_id: Option<String>,
    pub flash_size: Option<String>,
}

impl StlinkProbe {
    pub fn display_label(&self) -> String {
        let desc = self.dev_type.as_deref().unwrap_or("STM32 Target");
        let ver = self.version.as_deref().unwrap_or("ST-Link");
        format!("{} [{}] (SN: {})", desc, ver, self.serial)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeError {
    ToolNotFound,
    ExecutionFailed(String),
    ZeroProbesFound,
}

impl std::fmt::Display for ProbeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProbeError::ToolNotFound => write!(
                f,
                "`st-info` not found on PATH. Please install stlink-tools (e.g. `sudo apt install stlink-tools`)."
            ),
            ProbeError::ExecutionFailed(msg) => write!(f, "Failed to run `st-info --probe`: {}", msg),
            ProbeError::ZeroProbesFound => write!(
                f,
                "No ST-Link detected. Please connect an ST-Link probe or Nucleo board via USB."
            ),
        }
    }
}

/// Parse stdout of `st-info --probe` into a list of StlinkProbe structs.
pub fn parse_st_info_probe(output: &str) -> Vec<StlinkProbe> {
    let mut probes = Vec::new();
    let mut current_serial: Option<String> = None;
    let mut current_version: Option<String> = None;
    let mut current_dev: Option<String> = None;
    let mut current_chipid: Option<String> = None;
    let mut current_flash: Option<String> = None;

    let flush_current = |probes: &mut Vec<StlinkProbe>,
                         serial: &mut Option<String>,
                         version: &mut Option<String>,
                         dev: &mut Option<String>,
                         chipid: &mut Option<String>,
                         flash: &mut Option<String>| {
        if let Some(s) = serial.take() {
            probes.push(StlinkProbe {
                serial: s,
                version: version.take(),
                dev_type: dev.take(),
                chip_id: chipid.take(),
                flash_size: flash.take(),
            });
        } else {
            *version = None;
            *dev = None;
            *chipid = None;
            *flash = None;
        }
    };

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            flush_current(
                &mut probes,
                &mut current_serial,
                &mut current_version,
                &mut current_dev,
                &mut current_chipid,
                &mut current_flash,
            );
            continue;
        }

        if let Some((key, val)) = trimmed.split_once(':') {
            let key = key.trim().to_ascii_lowercase();
            let val = val.trim().to_string();
            match key.as_str() {
                "serial" => {
                    // If we already had a serial in progress, flush previous
                    if current_serial.is_some() {
                        flush_current(
                            &mut probes,
                            &mut current_serial,
                            &mut current_version,
                            &mut current_dev,
                            &mut current_chipid,
                            &mut current_flash,
                        );
                    }
                    current_serial = Some(val);
                }
                "version" => current_version = Some(val),
                "dev-type" | "descr" => current_dev = Some(val),
                "chipid" => current_chipid = Some(val),
                "flash" => current_flash = Some(val),
                _ => {}
            }
        }
    }

    flush_current(
        &mut probes,
        &mut current_serial,
        &mut current_version,
        &mut current_dev,
        &mut current_chipid,
        &mut current_flash,
    );

    probes
}

/// Detect connected ST-Link probes by calling `st-info --probe`.
pub fn detect_stlink_probes() -> Result<Vec<StlinkProbe>, ProbeError> {
    if !is_executable_on_path("st-info") {
        return Err(ProbeError::ToolNotFound);
    }

    let output = match Command::new("st-info").arg("--probe").output() {
        Ok(out) => out,
        Err(e) => return Err(ProbeError::ExecutionFailed(e.to_string())),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let probes = parse_st_info_probe(&stdout);

    if probes.is_empty() {
        Err(ProbeError::ZeroProbesFound)
    } else {
        Ok(probes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_probe() {
        let text = r#"
Found 1 stlink programmers
version: V3J8
serial: 003800333433510937363934
flash: 2097152 (pagesize: 131072)
sram: 131072
chipid: 0x450
dev-type: STM32H74x_H75x
"#;
        let probes = parse_st_info_probe(text);
        assert_eq!(probes.len(), 1);
        let p = &probes[0];
        assert_eq!(p.serial, "003800333433510937363934");
        assert_eq!(p.version.as_deref(), Some("V3J8"));
        assert_eq!(p.dev_type.as_deref(), Some("STM32H74x_H75x"));
        assert_eq!(p.chip_id.as_deref(), Some("0x450"));
        assert_eq!(p.display_label(), "STM32H74x_H75x [V3J8] (SN: 003800333433510937363934)");
    }

    #[test]
    fn test_parse_multiple_probes() {
        let text = r#"
Found 2 stlink programmers
  version: V2J29S7
  serial: 563f67066772565631130667
  flash: 262144 (pagesize: 2048)
  sram: 65536
  chipid: 0x0414
  descr: F1 High-density device

  version: V2J37M27
  serial: 493f6d066670485136151067
  flash: 1048576 (pagesize: 16384)
  sram: 196608
  chipid: 0x0413
  descr: F4 device
"#;
        let probes = parse_st_info_probe(text);
        assert_eq!(probes.len(), 2);
        assert_eq!(probes[0].serial, "563f67066772565631130667");
        assert_eq!(probes[0].dev_type.as_deref(), Some("F1 High-density device"));
        assert_eq!(probes[1].serial, "493f6d066670485136151067");
        assert_eq!(probes[1].dev_type.as_deref(), Some("F4 device"));
    }

    #[test]
    fn test_parse_zero_probes() {
        let text = "Found 0 stlink programmers\n";
        let probes = parse_st_info_probe(text);
        assert!(probes.is_empty());
    }
}
