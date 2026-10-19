use super::runner::is_executable_on_path;
use std::process::Command;

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
pub struct DfuDevice {
    pub vendor_id: String,
    pub product_id: String,
    pub devnum: Option<String>,
    pub alt: Option<u32>,
    pub alt_name: Option<String>,
    pub serial: Option<String>,
}

impl DfuDevice {
    pub fn display_label(&self) -> String {
        let name = self.alt_name.as_deref().unwrap_or("DFU Device");
        let sn = self.serial.as_deref().unwrap_or("N/A");
        format!(
            "{} [{}:{}] (SN: {})",
            name, self.vendor_id, self.product_id, sn
        )
    }

    pub fn is_stm32_bootloader(&self) -> bool {
        self.vendor_id.eq_ignore_ascii_case("0483") && self.product_id.eq_ignore_ascii_case("df11")
    }

    pub fn is_internal_flash(&self) -> bool {
        self.alt_name
            .as_deref()
            .is_some_and(|n| n.contains("Internal Flash"))
            || (self.is_stm32_bootloader() && self.alt == Some(0))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeError {
    ToolNotFound,
    DfuToolNotFound,
    ExecutionFailed(String),
    ZeroProbesFound,
    ZeroDfuDevicesFound,
}

impl std::fmt::Display for ProbeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProbeError::ToolNotFound => write!(
                f,
                "`st-info` not found on PATH. Please install stlink-tools (e.g. `sudo apt install stlink-tools`)."
            ),
            ProbeError::DfuToolNotFound => write!(
                f,
                "`dfu-util` not found on PATH. Please install dfu-util (e.g. `sudo apt install dfu-util`)."
            ),
            ProbeError::ExecutionFailed(msg) => write!(f, "Failed to run probe tool: {}", msg),
            ProbeError::ZeroProbesFound => write!(
                f,
                "No ST-Link detected. Please connect an ST-Link probe or Nucleo board via USB."
            ),
            ProbeError::ZeroDfuDevicesFound => write!(
                f,
                "No DFU device detected. Hold BOOT0, tap NRST, then release BOOT0 to enter DFU mode."
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

/// Parse stdout of `dfu-util -l` into a list of DfuDevice structs.
pub fn parse_dfu_util_list(output: &str) -> Vec<DfuDevice> {
    let mut devices = Vec::new();

    for line in output.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("Found DFU:") && !trimmed.starts_with("Found Runtime:") {
            continue;
        }

        // Extract [vendor:product]
        let (vid, pid) = match trimmed.find('[').and_then(|open| {
            trimmed[open + 1..].find(']').and_then(|close| {
                let id_pair = &trimmed[open + 1..open + 1 + close];
                id_pair.split_once(':')
            })
        }) {
            Some((v, p)) => (v.trim().to_string(), p.trim().to_string()),
            None => continue,
        };

        // Extract devnum
        let devnum = trimmed.split("devnum=").nth(1).and_then(|s| {
            s.split(|c: char| c == ',' || c.is_whitespace())
                .next()
                .filter(|v| !v.is_empty())
                .map(|v| v.to_string())
        });

        // Extract alt number
        let alt = trimmed.split("alt=").nth(1).and_then(|s| {
            s.split(|c: char| c == ',' || c.is_whitespace())
                .next()
                .and_then(|num| num.parse::<u32>().ok())
        });

        // Extract name="..." or name=...
        let alt_name = trimmed.split("name=").nth(1).and_then(|after_name| {
            if let Some(stripped) = after_name.strip_prefix('"') {
                stripped.split('"').next().map(|s| s.to_string())
            } else {
                let s = after_name.split(',').next().unwrap_or("").trim();
                if s.is_empty() {
                    None
                } else {
                    Some(s.to_string())
                }
            }
        });

        // Extract serial="..." or serial=...
        let serial = trimmed.split("serial=").nth(1).and_then(|after_ser| {
            if let Some(stripped) = after_ser.strip_prefix('"') {
                stripped.split('"').next().map(|s| s.to_string())
            } else {
                let s = after_ser
                    .split(|c: char| c == ',' || c.is_whitespace())
                    .next()
                    .unwrap_or("")
                    .trim();
                if s.is_empty() {
                    None
                } else {
                    Some(s.to_string())
                }
            }
        });

        devices.push(DfuDevice {
            vendor_id: vid,
            product_id: pid,
            devnum,
            alt,
            alt_name,
            serial,
        });
    }

    devices
}

/// Detect connected DFU devices by calling `dfu-util -l`.
pub fn detect_dfu_devices() -> Result<Vec<DfuDevice>, ProbeError> {
    if !is_executable_on_path("dfu-util") {
        return Err(ProbeError::DfuToolNotFound);
    }

    let output = match Command::new("dfu-util").arg("-l").output() {
        Ok(out) => out,
        Err(e) => return Err(ProbeError::ExecutionFailed(e.to_string())),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let devices = parse_dfu_util_list(&stdout);

    if devices.is_empty() {
        Err(ProbeError::ZeroDfuDevicesFound)
    } else {
        Ok(devices)
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
        assert_eq!(
            p.display_label(),
            "STM32H74x_H75x [V3J8] (SN: 003800333433510937363934)"
        );
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
        assert_eq!(
            probes[0].dev_type.as_deref(),
            Some("F1 High-density device")
        );
        assert_eq!(probes[1].serial, "493f6d066670485136151067");
        assert_eq!(probes[1].dev_type.as_deref(), Some("F4 device"));
    }

    #[test]
    fn test_parse_zero_probes() {
        let text = "Found 0 stlink programmers\n";
        let probes = parse_st_info_probe(text);
        assert!(probes.is_empty());
    }

    #[test]
    fn test_parse_dfu_single_stm32() {
        let text = r#"Found DFU: [0483:df11] ver=2200, devnum=12, cfg=1, intf=0, path="1-1.2", alt=0, name="@Internal Flash  /0x08000000/04*016Kg,01*064Kg,03*128Kg", serial="347535793038""#;
        let devices = parse_dfu_util_list(text);
        assert_eq!(devices.len(), 1);
        let dev = &devices[0];
        assert_eq!(dev.vendor_id, "0483");
        assert_eq!(dev.product_id, "df11");
        assert_eq!(dev.devnum.as_deref(), Some("12"));
        assert_eq!(dev.alt, Some(0));
        assert_eq!(
            dev.alt_name.as_deref(),
            Some("@Internal Flash  /0x08000000/04*016Kg,01*064Kg,03*128Kg")
        );
        assert_eq!(dev.serial.as_deref(), Some("347535793038"));
        assert!(dev.is_stm32_bootloader());
        assert!(dev.is_internal_flash());
        assert_eq!(
            dev.display_label(),
            "@Internal Flash  /0x08000000/04*016Kg,01*064Kg,03*128Kg [0483:df11] (SN: 347535793038)"
        );
    }

    #[test]
    fn test_parse_dfu_non_stm32() {
        let text = r#"Found DFU: [1234:5678] ver=0100, devnum=5, cfg=1, intf=0, path="1-2", alt=0, name="Custom Bootloader", serial="ABC123XYZ""#;
        let devices = parse_dfu_util_list(text);
        assert_eq!(devices.len(), 1);
        let dev = &devices[0];
        assert_eq!(dev.vendor_id, "1234");
        assert_eq!(dev.product_id, "5678");
        assert!(!dev.is_stm32_bootloader());
        assert!(!dev.is_internal_flash());
    }

    #[test]
    fn test_parse_dfu_zero_devices() {
        let text = "dfu-util 0.11\nNo DFU capable USB device found\n";
        let devices = parse_dfu_util_list(text);
        assert!(devices.is_empty());
    }

    #[test]
    fn test_parse_dfu_malformed() {
        let text = "Found DFU: invalid line with no brackets\nSome [1234] random text\nFound DFU: [missing_colon] ver=10\n";
        let devices = parse_dfu_util_list(text);
        assert!(devices.is_empty());
    }
}
