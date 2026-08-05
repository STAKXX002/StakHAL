//! Diagnostic binary to manually verify marker_scan and ioc::parser against a real CubeMX project.
//!
//! Usage:
//! cargo run -p stakhal-core --example verify_scan -- <path-to-ioc> <path-to-main.c>

use std::env;
use std::fs;
use std::path::Path;
use std::process;

use stakhal_core::ioc::parse_ioc;
use stakhal_core::source::{find_loop_body_gap, scan_file};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!(
            "Usage: cargo run -p stakhal-core --example verify_scan -- <path-to-ioc> <path-to-main.c>"
        );
        process::exit(1);
    }

    let ioc_path = Path::new(&args[1]);
    let main_c_path = Path::new(&args[2]);

    println!("=== Parsing .ioc File: {:?} ===", ioc_path);
    let ioc_project = match parse_ioc(ioc_path) {
        Ok(proj) => proj,
        Err(err) => {
            eprintln!("Failed to parse .ioc file: {}", err);
            process::exit(1);
        }
    };

    println!("MCU Family:        {}", ioc_project.mcu_family);
    println!("MCU Name:          {}", ioc_project.mcu_name);
    println!("Total Pins:        {}", ioc_project.pins.len());
    println!("Total Peripherals: {}", ioc_project.peripherals.len());
    println!("\nPeripherals:");
    for periph in &ioc_project.peripherals {
        let mode_str = periph.mode.as_deref().unwrap_or("N/A");
        println!(
            "  - {}: mode={}, parameters={}",
            periph.name,
            mode_str,
            periph.parameters.len()
        );
    }

    println!("\n=== Scanning C Source File: {:?} ===", main_c_path);
    let regions = match scan_file(main_c_path) {
        Ok(r) => r,
        Err(err) => {
            eprintln!("Failed to scan C source file: {}", err);
            process::exit(1);
        }
    };

    let source_content = match fs::read_to_string(main_c_path) {
        Ok(content) => content,
        Err(err) => {
            eprintln!("Failed to read C source file for preview: {}", err);
            process::exit(1);
        }
    };

    println!("Total User Regions Found: {}\n", regions.len());
    for (idx, region) in regions.iter().enumerate() {
        let (start, end) = region.byte_range;
        let (start_line, end_line) = region.line_range;
        let slice = if start <= end && end <= source_content.len() {
            &source_content[start..end]
        } else {
            ""
        };
        let preview: String = slice.chars().take(40).collect();
        let escaped_preview = preview.replace('\n', "\\n").replace('\r', "\\r");

        println!(
            "[{}] Tag: '{}' | Bytes: {:?} | Lines: {}-{} | Preview: \"{}\"",
            idx + 1,
            region.tag,
            region.byte_range,
            start_line,
            end_line,
            escaped_preview
        );
    }

    if let Some(gap) = find_loop_body_gap(&regions) {
        let (start, end) = gap.byte_range;
        let (start_line, end_line) = gap.line_range;
        let slice = if start <= end && end <= source_content.len() {
            &source_content[start..end]
        } else {
            ""
        };
        let preview: String = slice.chars().take(40).collect();
        let escaped_preview = preview.replace('\n', "\\n").replace('\r', "\\r");

        println!(
            "\n[LOOP BODY] Tag: '{}' | Bytes: {:?} | Lines: {}-{} | Preview: \"{}\"",
            gap.tag, gap.byte_range, start_line, end_line, escaped_preview
        );
    } else {
        println!("\n[LOOP BODY] Implicit loop body gap not detected.");
    }
}
