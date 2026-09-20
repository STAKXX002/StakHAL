use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsoleUartInfo {
    pub uart_instance: String,
    pub baud_rate: u32,
}

/// Collects all candidate .c source files in the project directory hierarchy.
fn collect_c_source_files(main_c_path: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut search_dirs = Vec::new();

    if let Some(parent) = main_c_path.parent() {
        search_dirs.push(parent.to_path_buf());
        if let Some(grandparent) = parent.parent() {
            search_dirs.push(grandparent.to_path_buf());
            let src_dir = grandparent.join("Src");
            if src_dir.exists() && src_dir.is_dir() && !search_dirs.contains(&src_dir) {
                search_dirs.push(src_dir);
            }
            let core_src = grandparent.join("Core").join("Src");
            if core_src.exists() && core_src.is_dir() && !search_dirs.contains(&core_src) {
                search_dirs.push(core_src);
            }
            if let Some(great_grandparent) = grandparent.parent() {
                search_dirs.push(great_grandparent.to_path_buf());
                let core_src2 = great_grandparent.join("Core").join("Src");
                if core_src2.exists() && core_src2.is_dir() && !search_dirs.contains(&core_src2) {
                    search_dirs.push(core_src2);
                }
                let src_dir2 = great_grandparent.join("Src");
                if src_dir2.exists() && src_dir2.is_dir() && !search_dirs.contains(&src_dir2) {
                    search_dirs.push(src_dir2);
                }
            }
        }
    }

    for dir in search_dirs {
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() && p.extension().map_or(false, |ext| ext == "c") {
                    if !files.contains(&p) {
                        files.push(p);
                    }
                }
            }
        }
    }

    if !files.contains(&main_c_path.to_path_buf()) {
        files.push(main_c_path.to_path_buf());
    }

    files
}

/// Detects the console UART instance name and baud rate.
/// 1. Finds the retarget function (_write, __io_putchar, _io_putchar) that calls HAL_UART_Transmit(&huart<N>, ...)
/// 2. Extracts the UART instance name (e.g. "huart2")
/// 3. Finds `<uart_instance>.Init.BaudRate = <value>;` in the C source files
/// 4. If no retarget function is found, falls back to scanning for any `huart<N>.Init.BaudRate = <value>;`
pub fn detect_console_uart(main_c_path: &Path) -> Option<ConsoleUartInfo> {
    let c_files = collect_c_source_files(main_c_path);
    let mut file_contents: Vec<(PathBuf, String)> = Vec::new();

    for file_path in &c_files {
        if let Ok(content) = fs::read_to_string(file_path) {
            file_contents.push((file_path.clone(), content));
        }
    }

    // Pass 1: Look for retarget function calling HAL_UART_Transmit(&huartX, ...)
    let mut detected_instance = None;

    for (_path, content) in &file_contents {
        if let Some(inst) = find_uart_in_retarget_function(content) {
            detected_instance = Some(inst);
            break;
        }
    }

    // Pass 2: If an instance was found, look for its BaudRate
    if let Some(ref inst) = detected_instance {
        if let Some(baud) = find_baud_rate_for_instance(inst, &file_contents) {
            return Some(ConsoleUartInfo {
                uart_instance: inst.clone(),
                baud_rate: baud,
            });
        }
    }

    // Pass 3: Fallback - look for any huart<N>.Init.BaudRate = <value>;
    for (_path, content) in &file_contents {
        if let Some((inst, baud)) = find_any_uart_baud_rate(content) {
            return Some(ConsoleUartInfo {
                uart_instance: detected_instance.unwrap_or(inst),
                baud_rate: baud,
            });
        }
    }

    // If instance was detected but baud was not found, default to 115200
    if let Some(inst) = detected_instance {
        return Some(ConsoleUartInfo {
            uart_instance: inst,
            baud_rate: 115200,
        });
    }

    None
}

/// Finds the UART handle passed to HAL_UART_Transmit inside _write, __io_putchar, or _io_putchar
fn find_uart_in_retarget_function(content: &str) -> Option<String> {
    let retarget_names = ["_write", "__io_putchar", "_io_putchar", "fputc"];

    for name in &retarget_names {
        if let Some(fn_start) = find_function_body(content, name) {
            let body = &content[fn_start..];
            // Find end of function block
            let body_block = extract_matching_braces(body).unwrap_or(body);
            if let Some(inst) = extract_hal_uart_transmit_handle(body_block) {
                return Some(inst);
            }
        }
    }

    None
}

/// Finds the index where the body `{` of a function begins
fn find_function_body<'a>(content: &'a str, fn_name: &str) -> Option<usize> {
    let mut search_from = 0;
    while let Some(pos) = content[search_from..].find(fn_name) {
        let abs_pos = search_from + pos;
        let pre_char_ok = if abs_pos == 0 {
            true
        } else {
            let prev = content[..abs_pos].chars().last().unwrap();
            !prev.is_alphanumeric() && prev != '_'
        };

        let post_pos = abs_pos + fn_name.len();
        let post_char_ok = if post_pos >= content.len() {
            false
        } else {
            let next = content[post_pos..].chars().next().unwrap();
            !next.is_alphanumeric() && next != '_'
        };

        if pre_char_ok && post_char_ok {
            // Find open parenthesis and then open brace
            if let Some(paren_pos) = content[post_pos..].find('(') {
                let abs_paren = post_pos + paren_pos;
                // Ensure parenthesis is close
                if abs_paren - post_pos < 60 {
                    if let Some(brace_pos) = content[abs_paren..].find('{') {
                        let abs_brace = abs_paren + brace_pos;
                        return Some(abs_brace);
                    }
                }
            }
        }

        search_from = post_pos;
    }

    None
}

/// Extracts content enclosed by matching outer braces `{ ... }`
fn extract_matching_braces(s: &str) -> Option<&str> {
    let mut depth = 0;
    let mut started = false;
    let mut start_idx = 0;

    for (idx, ch) in s.char_indices() {
        if ch == '{' {
            if !started {
                started = true;
                start_idx = idx;
            }
            depth += 1;
        } else if ch == '}' && started {
            depth -= 1;
            if depth == 0 {
                return Some(&s[start_idx..idx + 1]);
            }
        }
    }

    None
}

/// Extracts the handle identifier from `HAL_UART_Transmit(&<ident>, ...)`
fn extract_hal_uart_transmit_handle(text: &str) -> Option<String> {
    let patterns = ["HAL_UART_Transmit", "HAL_UART_Transmit_IT", "HAL_UART_Transmit_DMA"];

    for pat in &patterns {
        let mut search_from = 0;
        while let Some(pos) = text[search_from..].find(pat) {
            let abs_pos = search_from + pos + pat.len();
            if let Some(open_paren) = text[abs_pos..].find('(') {
                let args_start = abs_pos + open_paren + 1;
                if let Some(amp_pos) = text[args_start..].find('&') {
                    let ident_start = args_start + amp_pos + 1;
                    let remaining = &text[ident_start..];
                    let trimmed_remaining = remaining.trim_start();
                    let ident: String = trimmed_remaining
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '_')
                        .collect();
                    if !ident.is_empty() {
                        return Some(ident);
                    }
                }
            }
            search_from = abs_pos;
        }
    }

    None
}

/// Finds `<instance>.Init.BaudRate = <value>;` across file contents
fn find_baud_rate_for_instance(instance: &str, files: &[(PathBuf, String)]) -> Option<u32> {
    let target = format!("{}.Init.BaudRate", instance);

    for (_path, content) in files {
        if let Some(pos) = content.find(&target) {
            let after = &content[pos + target.len()..];
            if let Some(eq_pos) = after.find('=') {
                let val_str = &after[eq_pos + 1..];
                let trimmed = val_str.trim_start();
                let digits: String = trimmed.chars().take_while(|c| c.is_ascii_digit()).collect();
                if let Ok(num) = digits.parse::<u32>() {
                    return Some(num);
                }
            }
        }
    }

    None
}

/// Finds any `huart<N>.Init.BaudRate = <value>;` in the content
fn find_any_uart_baud_rate(content: &str) -> Option<(String, u32)> {
    let mut search_from = 0;
    while let Some(pos) = content[search_from..].find(".Init.BaudRate") {
        let abs_pos = search_from + pos;
        let before = &content[search_from..abs_pos];
        let inst: String = before
            .chars()
            .rev()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect::<String>()
            .chars()
            .rev()
            .collect();

        let after = &content[abs_pos + ".Init.BaudRate".len()..];
        if let Some(eq_pos) = after.find('=') {
            let val_str = &after[eq_pos + 1..];
            let trimmed = val_str.trim_start();
            let digits: String = trimmed.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(num) = digits.parse::<u32>() {
                if !inst.is_empty() {
                    return Some((inst, num));
                }
            }
        }

        search_from = abs_pos + ".Init.BaudRate".len();
    }

    None
}

/// Extracts command names from commandTable or Command arrays across project C files.
pub fn extract_project_command_names(main_c_path: &Path) -> Vec<String> {
    let c_files = collect_c_source_files(main_c_path);
    let mut commands = Vec::new();

    for file_path in &c_files {
        if let Ok(content) = fs::read_to_string(file_path) {
            let extracted = parse_command_table_entries(&content);
            for cmd in extracted {
                if !commands.contains(&cmd) {
                    commands.push(cmd);
                }
            }
        }
    }

    commands
}

/// Parses entries like { "GO", cmd_go }, { "CAL", cmd_cal } from C source.
fn parse_command_table_entries(content: &str) -> Vec<String> {
    let mut commands = Vec::new();

    let mut search_from = 0;
    while let Some(pos) = content[search_from..].find('{') {
        let abs_pos = search_from + pos;
        if let Some(close_pos) = content[abs_pos..].find('}') {
            let block = &content[abs_pos..abs_pos + close_pos + 1];
            if let Some(str_start) = block.find('"') {
                if let Some(str_end) = block[str_start + 1..].find('"') {
                    let cmd_str = &block[str_start + 1..str_start + 1 + str_end];
                    let after_str = &block[str_start + 1 + str_end + 1..];
                    if let Some(comma_pos) = after_str.find(',') {
                        let after_comma = after_str[comma_pos + 1..].trim();
                        let ident: String = after_comma
                            .chars()
                            .take_while(|c| c.is_alphanumeric() || *c == '_')
                            .collect();
                        if !ident.is_empty() && !cmd_str.is_empty() && cmd_str.len() <= 16 {
                            if cmd_str.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
                                commands.push(cmd_str.to_string());
                            }
                        }
                    }
                }
            }
            search_from = abs_pos + close_pos + 1;
        } else {
            break;
        }
    }

    commands
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_console_uart_aa_ns_stm_port() {
        let fixture_main_c = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/aa_ns_stm_port/Core/Src/main.c");
        let info = detect_console_uart(&fixture_main_c).expect("should detect console uart in aa_ns_stm_port");
        assert_eq!(info.uart_instance, "huart2");
        assert_eq!(info.baud_rate, 115200);
    }

    #[test]
    fn test_detect_console_uart_docking_firmware_v2() {
        let fixture_main_c = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/docking_firmware_v2/Core/Src/main.c");
        let info = detect_console_uart(&fixture_main_c).expect("should detect console uart in docking_firmware_v2");
        assert_eq!(info.uart_instance, "huart2");
        assert_eq!(info.baud_rate, 115200);
    }

    #[test]
    fn test_extract_project_command_names_aa_ns_stm_port() {
        let fixture_main_c = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/aa_ns_stm_port/Core/Src/main.c");
        let commands = extract_project_command_names(&fixture_main_c);
        let expected = ["CAL", "GO", "RET", "RST", "OPEN", "CLOSE", "STOP", "ON", "OFF"];
        for exp in &expected {
            assert!(
                commands.iter().any(|c| c == exp),
                "Expected command '{}' in extracted commands: {:?}",
                exp,
                commands
            );
        }
    }
}
