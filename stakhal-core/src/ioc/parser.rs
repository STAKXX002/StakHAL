use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IocProject {
    pub mcu_family: String,
    pub mcu_name: String, // e.g. "STM32F446RETx"
    pub pins: Vec<PinConfig>,
    pub peripherals: Vec<PeripheralConfig>,
    pub raw: HashMap<String, String>, // full unparsed key=value map
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinConfig {
    pub pin: String,    // e.g. "PA2"
    pub signal: String, // e.g. "USART2_TX"
    pub label: Option<String>, // e.g. "LD2" or "TMS"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeripheralConfig {
    pub name: String,                        // e.g. "USART2"
    pub mode: Option<String>,                // e.g. "Asynchronous"
    pub parameters: HashMap<String, String>, // any "<Name>.<Param>" keys
}

#[derive(thiserror::Error, Debug)]
pub enum IocParseError {
    #[error("file not found: {0}")]
    FileNotFound(PathBuf),
    #[error("I/O error: {0}")]
    IoError(String),
    #[error("malformed line {0}: expected key=value, got '{1}'")]
    MalformedLine(usize, String),
    #[error("missing required key: {0}")]
    MissingRequiredKey(String),
}

const PERIPHERAL_PREFIXES: &[&str] = &[
    "USART", "UART", "SPI", "I2C", "TIM", "ADC", "DMA", "GPIO", "CAN", "USB", "RTC", "DAC", "CRC",
    "RNG", "I2S", "SAI", "FMC", "SDMMC", "SDIO",
];

fn is_pin_signal(key: &str) -> Option<&str> {
    let (pin_part, attr) = key.split_once('.')?;
    if attr != "Signal" {
        return None;
    }
    let bytes = pin_part.as_bytes();
    if bytes.len() < 3 || bytes[0] != b'P' {
        return None;
    }
    if !matches!(bytes[1], b'A'..=b'H') {
        return None;
    }
    if !bytes[2..].iter().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some(pin_part)
}

fn get_peripheral_and_param(key: &str) -> Option<(&str, &str)> {
    let (name, param) = key.split_once('.')?;
    if is_pin_signal(key).is_some() {
        return None;
    }
    for prefix in PERIPHERAL_PREFIXES {
        if name.starts_with(prefix) {
            return Some((name, param));
        }
    }
    None
}

pub fn parse_ioc(path: &Path) -> Result<IocProject, IocParseError> {
    let content = fs::read_to_string(path).map_err(|err| match err.kind() {
        std::io::ErrorKind::NotFound => IocParseError::FileNotFound(path.to_path_buf()),
        _ => IocParseError::IoError(err.to_string()),
    })?;
    parse_ioc_str(&content)
}

pub fn parse_ioc_str(source: &str) -> Result<IocProject, IocParseError> {
    let mut raw = HashMap::new();
    let mut pins = Vec::new();
    let mut peripherals_map: HashMap<String, PeripheralConfig> = HashMap::new();

    for (line_idx, line) in source.lines().enumerate() {
        let line_num = line_idx + 1;
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let (key_part, val_part) = trimmed
            .split_once('=')
            .ok_or_else(|| IocParseError::MalformedLine(line_num, line.to_string()))?;

        let key = key_part.trim().to_string();
        let value = val_part.to_string();

        raw.insert(key.clone(), value.clone());

        if let Some(pin_name) = is_pin_signal(&key) {
            pins.push(PinConfig {
                pin: pin_name.to_string(),
                signal: value.clone(),
                label: None,
            });
        } else if let Some((periph_name, param_name)) = get_peripheral_and_param(&key) {
            let entry = peripherals_map
                .entry(periph_name.to_string())
                .or_insert_with(|| PeripheralConfig {
                    name: periph_name.to_string(),
                    mode: None,
                    parameters: HashMap::new(),
                });

            if param_name == "Mode" {
                entry.mode = Some(value.clone());
            }
            entry
                .parameters
                .insert(param_name.to_string(), value.clone());
        }
    }

    for pin in &mut pins {
        let label_key = format!("{}.GPIO_Label", pin.pin);
        if let Some(lbl) = raw.get(&label_key) {
            let trimmed_lbl = lbl.trim();
            if !trimmed_lbl.is_empty() {
                pin.label = Some(trimmed_lbl.to_string());
            }
        }
    }

    let mcu_family = raw
        .get("Mcu.Family")
        .cloned()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| IocParseError::MissingRequiredKey("Mcu.Family".to_string()))?;

    let mcu_name = raw
        .get("Mcu.UserName")
        .or_else(|| raw.get("Mcu.Name"))
        .cloned()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            IocParseError::MissingRequiredKey("Mcu.UserName or Mcu.Name".to_string())
        })?;

    let peripherals = peripherals_map.into_values().collect();

    Ok(IocProject {
        mcu_family,
        mcu_name,
        pins,
        peripherals,
        raw,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const REALISTIC_IOC_FIXTURE: &str = r#"
# MicroXplorer Configuration settings - Do not modify
Mcu.Family=STM32F4
Mcu.IP0=NVIC
Mcu.IP1=RCC
Mcu.IP2=SYS
Mcu.IP3=USART2
Mcu.Name=STM32F446RETx
Mcu.UserName=STM32F446RETx
PA2.Signal=USART2_TX
PA3.Signal=USART2_RX
ProjectManager.CustomerFirmwarePackage=STM32Cube_FW_F4_V1.27.1
USART2.BaudRate=115200
USART2.IPParameters=VirtualMode,BaudRate
USART2.Mode=Asynchronous
USART2.VirtualMode=VM_ASYNC
"#;

    #[test]
    fn test_parse_well_formed_ioc() {
        let project = parse_ioc_str(REALISTIC_IOC_FIXTURE).unwrap();
        assert_eq!(project.mcu_family, "STM32F4");
        assert_eq!(project.mcu_name, "STM32F446RETx");

        assert_eq!(project.pins.len(), 2);
        let pa2 = project.pins.iter().find(|p| p.pin == "PA2").unwrap();
        assert_eq!(pa2.signal, "USART2_TX");

        let usart2 = project
            .peripherals
            .iter()
            .find(|p| p.name == "USART2")
            .unwrap();
        assert_eq!(usart2.mode, Some("Asynchronous".to_string()));
        assert_eq!(usart2.parameters.get("BaudRate").unwrap(), "115200");
        assert_eq!(
            usart2.parameters.get("IPParameters").unwrap(),
            "VirtualMode,BaudRate"
        );

        assert_eq!(
            project.raw.get("ProjectManager.CustomerFirmwarePackage").unwrap(),
            "STM32Cube_FW_F4_V1.27.1"
        );
        assert_eq!(project.raw.get("PA2.Signal").unwrap(), "USART2_TX");
        assert_eq!(project.raw.get("Mcu.Family").unwrap(), "STM32F4");
    }

    #[test]
    fn test_malformed_line_no_equals() {
        let fixture = r#"
Mcu.Family=STM32F4
Mcu.UserName=STM32F446RETx
This is a malformed line with no equals sign
PA2.Signal=USART2_TX
"#;
        let res = parse_ioc_str(fixture);
        assert!(matches!(res, Err(IocParseError::MalformedLine(4, ref line)) if line.contains("This is a malformed line")));
    }

    #[test]
    fn test_parse_gpio_label() {
        let fixture = r#"
Mcu.Family=STM32F4
Mcu.UserName=STM32F446RETx
PA5.Signal=GPIO_Output
PA5.GPIO_Label=LD2
PB3.Signal=GPIO_Output
"#;
        let project = parse_ioc_str(fixture).unwrap();
        let pa5 = project.pins.iter().find(|p| p.pin == "PA5").unwrap();
        assert_eq!(pa5.label, Some("LD2".to_string()));

        let pb3 = project.pins.iter().find(|p| p.pin == "PB3").unwrap();
        assert_eq!(pb3.label, None);
    }
}
