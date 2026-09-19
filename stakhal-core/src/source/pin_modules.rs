//! Pin-to-module mapping for firmware projects.
//!
//! Scans C source files across the project (e.g. Core/Src/*.c) to determine
//! which firmware modules (file stems such as "alignment", "hatch", "relay")
//! reference each active pin or peripheral signal.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use tree_sitter::Parser;

use crate::ioc::parser::PinConfig;

/// Check if a given C source file is standard CubeMX / CMSIS infrastructure boilerplate.
pub fn is_infra_file(path: &Path) -> bool {
    let name = match path.file_name().and_then(|s| s.to_str()) {
        Some(n) => n,
        None => return false,
    };
    if name == "syscalls.c" || name == "sysmem.c" {
        return true;
    }
    if name.starts_with("system_stm32") {
        return true;
    }
    if name.ends_with("_it.c") || name.ends_with("_hal_msp.c") {
        return true;
    }
    false
}

/// Discover all candidate application module .c files in the project.
/// When sibling module .c files exist, main.c (HAL init boilerplate) and
/// infrastructure files (ISR, MSP, syscalls, sysmem, CMSIS) are excluded.
/// In single-file projects, main.c is retained as the sole module.
pub fn discover_module_files(main_c_path: &Path) -> Vec<PathBuf> {
    let src_dir = main_c_path.parent().unwrap_or_else(|| Path::new("."));
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(src_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() && p.extension().map_or(false, |ext| ext == "c") && !is_infra_file(&p) {
                if p.file_name().map_or(false, |n| n == "main.c") {
                    continue;
                }
                files.push(p);
            }
        }
    }
    // If no sibling module files were found, main.c is the sole module
    if files.is_empty() && main_c_path.exists() && !is_infra_file(main_c_path) {
        files.push(main_c_path.to_path_buf());
    }
    files.sort();
    files
}

#[derive(Debug, Clone)]
struct PinPattern {
    pin_index: usize,
    label_macros: Vec<String>,
    raw_pin: Option<String>,
    raw_port: Option<String>,
    periph_tokens: Vec<String>,
    periph_family: Option<String>,
}

fn build_pin_patterns(pins: &[PinConfig]) -> (Vec<PinPattern>, HashMap<String, usize>) {
    let mut periph_family_counts: HashMap<String, usize> = HashMap::new();
    for p in pins {
        for prefix in &["USART", "UART", "SPI", "I2C", "TIM", "ADC", "CAN"] {
            if p.signal.starts_with(prefix) {
                *periph_family_counts.entry(prefix.to_string()).or_insert(0) += 1;
                break;
            }
        }
    }

    let mut patterns = Vec::new();

    for (idx, pin) in pins.iter().enumerate() {
        let mut label_macros = Vec::new();
        if let Some(lbl) = &pin.label {
            let base = lbl
                .split(|c: char| c.is_whitespace() || c == '[' || c == '(')
                .next()
                .unwrap_or(lbl)
                .trim();
            if !base.is_empty() {
                if base.ends_with("_Pin") {
                    label_macros.push(base.to_string());
                    let stem = base.strip_suffix("_Pin").unwrap();
                    label_macros.push(format!("{}_GPIO_Port", stem));
                    label_macros.push(stem.to_string());
                } else {
                    label_macros.push(format!("{}_Pin", base));
                    label_macros.push(format!("{}_GPIO_Port", base));
                    label_macros.push(base.to_string());
                }
            }
        }

        let mut raw_port = None;
        let mut raw_pin = None;
        if pin.pin.starts_with('P') && pin.pin.len() >= 3 {
            let port_char = pin.pin.chars().nth(1).unwrap();
            let pin_num_str = &pin.pin[2..];
            if port_char.is_ascii_alphabetic() && pin_num_str.chars().all(|c| c.is_ascii_digit()) {
                raw_port = Some(format!("GPIO{}", port_char));
                raw_pin = Some(format!("GPIO_PIN_{}", pin_num_str));
            }
        }

        let mut periph_tokens = Vec::new();
        let mut periph_family = None;
        for prefix in &["USART", "UART", "SPI", "I2C", "TIM", "ADC", "CAN"] {
            if pin.signal.starts_with(prefix) {
                periph_family = Some(prefix.to_string());
                let periph_name: String = pin.signal
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric())
                    .collect();
                if !periph_name.is_empty() {
                    periph_tokens.push(periph_name.clone());
                    let handle = if periph_name.starts_with("USART") {
                        format!("huart{}", &periph_name[5..])
                    } else if periph_name.starts_with("UART") {
                        format!("huart{}", &periph_name[4..])
                    } else if periph_name.starts_with("TIM") {
                        format!("htim{}", &periph_name[3..])
                    } else if periph_name.starts_with("SPI") {
                        format!("hspi{}", &periph_name[3..])
                    } else if periph_name.starts_with("I2C") {
                        format!("hi2c{}", &periph_name[3..])
                    } else if periph_name.starts_with("ADC") {
                        format!("hadc{}", &periph_name[3..])
                    } else if periph_name.starts_with("CAN") {
                        format!("hcan{}", &periph_name[3..])
                    } else {
                        format!("h{}", periph_name.to_lowercase())
                    };
                    periph_tokens.push(handle);
                }
                break;
            }
        }

        patterns.push(PinPattern {
            pin_index: idx,
            label_macros,
            raw_pin,
            raw_port,
            periph_tokens,
            periph_family,
        });
    }

    (patterns, periph_family_counts)
}

/// Extract all identifiers from a C source AST.
/// If `is_main` is true, functions starting with "MX_" or "mx_" are ignored.
pub fn extract_ast_identifiers(source: &str, is_main: bool) -> HashSet<String> {
    let mut idents = HashSet::new();
    let mut parser = Parser::new();
    if parser.set_language(&tree_sitter_c::language()).is_err() {
        return idents;
    }
    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return idents,
    };
    let source_bytes = source.as_bytes();

    walk_node(tree.root_node(), source_bytes, is_main, &mut idents);
    idents
}

fn walk_node(
    node: tree_sitter::Node,
    source_bytes: &[u8],
    is_main: bool,
    idents: &mut HashSet<String>,
) {
    if is_main && node.kind() == "function_definition" {
        if let Some(decl) = node.child_by_field_name("declarator") {
            let fn_name = get_declarator_name(decl, source_bytes);
            if fn_name.starts_with("MX_") || fn_name.starts_with("mx_") {
                return; // Skip CubeMX boilerplate initialization functions in main.c
            }
        }
    }

    if node.kind() == "identifier" || node.kind() == "field_identifier" || node.kind() == "type_identifier" {
        if let Ok(text) = node.utf8_text(source_bytes) {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                idents.insert(trimmed.to_string());
            }
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_node(child, source_bytes, is_main, idents);
    }
}

fn get_declarator_name(mut node: tree_sitter::Node, source_bytes: &[u8]) -> String {
    while let Some(child) = node.child_by_field_name("declarator") {
        node = child;
    }
    if node.kind() == "identifier" {
        node.utf8_text(source_bytes).unwrap_or("").to_string()
    } else {
        node.child_by_field_name("declarator")
            .and_then(|c| c.utf8_text(source_bytes).ok())
            .unwrap_or("")
            .to_string()
    }
}

/// Scan candidate C files in project to map each pin to its owning module(s).
/// Returns updated PinConfig list and sorted list of discovered modules.
pub fn scan_pin_modules(
    pins: &[PinConfig],
    main_c_path: &Path,
) -> (Vec<PinConfig>, Vec<String>) {
    let mut updated_pins: Vec<PinConfig> = pins.to_vec();
    let candidate_files = discover_module_files(main_c_path);

    let (patterns, periph_family_counts) = build_pin_patterns(pins);

    let mut discovered_modules: HashSet<String> = HashSet::new();

    for file_path in &candidate_files {
        let file_stem = match file_path.file_stem().and_then(|s| s.to_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };

        let is_main = file_stem == "main";

        let source = match fs::read_to_string(file_path) {
            Ok(s) => s,
            Err(_) => continue,
        };

        let idents = extract_ast_identifiers(&source, is_main);

        for pattern in &patterns {
            let mut matched = false;

            // 1. Direct label macros (e.g. Z1_LIMIT_Pin, Z1_LIMIT_GPIO_Port)
            for m in &pattern.label_macros {
                if idents.contains(m) {
                    matched = true;
                    break;
                }
            }

            // 2. Peripheral tokens (e.g. USART2, huart2, TIM3, htim3)
            if !matched {
                for token in &pattern.periph_tokens {
                    if idents.contains(token) {
                        matched = true;
                        break;
                    }
                }
            }

            // 3. Generic peripheral calls if this peripheral family is present in module
            if !matched {
                if let Some(family) = &pattern.periph_family {
                    let is_unique_family = periph_family_counts.get(family).copied().unwrap_or(0) <= 2;
                    if is_unique_family {
                        let family_matched = match family.as_str() {
                            "USART" | "UART" => {
                                idents.contains("UART_HandleTypeDef")
                                    || idents.contains("HAL_UART_Receive_IT")
                                    || idents.contains("HAL_UART_Transmit")
                                    || idents.contains("HAL_UART_RxCpltCallback")
                                    || idents.contains("huart")
                            }
                            "TIM" => {
                                idents.contains("TIM_HandleTypeDef")
                                    || idents.contains("HAL_TIM_PeriodElapsedCallback")
                                    || idents.contains("HAL_TIM_Base_Start_IT")
                            }
                            "SPI" => {
                                idents.contains("SPI_HandleTypeDef")
                                    || idents.contains("HAL_SPI_Transmit")
                                    || idents.contains("HAL_SPI_Receive")
                            }
                            "I2C" => {
                                idents.contains("I2C_HandleTypeDef")
                                    || idents.contains("HAL_I2C_Master_Transmit")
                                    || idents.contains("HAL_I2C_Master_Receive")
                            }
                            _ => false,
                        };
                        if family_matched {
                            matched = true;
                        }
                    }
                }
            }

            // 4. Raw port and pin (if no user label, e.g. GPIOA and GPIO_PIN_5)
            if !matched && pattern.label_macros.is_empty() {
                if let (Some(port), Some(pin)) = (&pattern.raw_port, &pattern.raw_pin) {
                    if idents.contains(port) && idents.contains(pin) {
                        matched = true;
                    }
                }
            }

            if matched {
                let target_pin = &mut updated_pins[pattern.pin_index];
                if !target_pin.modules.contains(&file_stem) {
                    target_pin.modules.push(file_stem.clone());
                }
            }
        }

        discovered_modules.insert(file_stem);
    }

    if discovered_modules.is_empty() {
        discovered_modules.insert("main".to_string());
    }

    for pin in &mut updated_pins {
        pin.modules.sort();
        pin.modules.dedup();
    }

    let mut sorted_modules: Vec<String> = discovered_modules.into_iter().collect();
    sorted_modules.sort();

    (updated_pins, sorted_modules)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_infra_file() {
        assert!(is_infra_file(Path::new("Core/Src/stm32f4xx_it.c")));
        assert!(is_infra_file(Path::new("Core/Src/stm32f4xx_hal_msp.c")));
        assert!(is_infra_file(Path::new("Core/Src/system_stm32f4xx.c")));
        assert!(is_infra_file(Path::new("Core/Src/syscalls.c")));
        assert!(is_infra_file(Path::new("Core/Src/sysmem.c")));

        assert!(!is_infra_file(Path::new("Core/Src/system.c")));
        assert!(!is_infra_file(Path::new("Core/Src/alignment.c")));
        assert!(!is_infra_file(Path::new("Core/Src/hatch.c")));
        assert!(!is_infra_file(Path::new("Core/Src/main.c")));
    }

    #[test]
    fn test_discover_modules_aa_ns_stm_port() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/aa_ns_stm_port/Core/Src/main.c");
        let modules = discover_module_files(&root);
        let stems: Vec<String> = modules
            .iter()
            .map(|p| p.file_stem().unwrap().to_str().unwrap().to_string())
            .collect();

        assert!(stems.contains(&"alignment".to_string()));
        assert!(stems.contains(&"commands".to_string()));
        assert!(stems.contains(&"hatch".to_string()));
        assert!(stems.contains(&"relay".to_string()));
        assert!(stems.contains(&"system".to_string()));
        assert!(!stems.contains(&"syscalls".to_string()));
        assert!(!stems.contains(&"stm32f4xx_it".to_string()));
    }

    #[test]
    fn test_shared_pin_ownership() {
        let pins = vec![
            PinConfig {
                pin: "PB10".to_string(),
                signal: "GPIO_Input".to_string(),
                label: Some("SHARED_FLAG".to_string()),
                modules: Vec::new(),
            },
        ];

        let (patterns, _) = build_pin_patterns(&pins);
        let source1 = "void f1(void) { if (HAL_GPIO_ReadPin(SHARED_FLAG_GPIO_Port, SHARED_FLAG_Pin)) {} }";
        let source2 = "void f2(void) { HAL_GPIO_WritePin(SHARED_FLAG_GPIO_Port, SHARED_FLAG_Pin, 1); }";

        let idents1 = extract_ast_identifiers(source1, false);
        let idents2 = extract_ast_identifiers(source2, false);

        assert!(idents1.contains("SHARED_FLAG_Pin"));
        assert!(idents2.contains("SHARED_FLAG_Pin"));
        assert!(patterns[0].label_macros.contains(&"SHARED_FLAG_Pin".to_string()));
    }
}
