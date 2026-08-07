// This integration test suite exists because synthetic string fixtures did not catch that
// CubeMX's main loop body lives in an unmarked gap between markers (USER CODE END WHILE and USER CODE BEGIN 3).
// This test suite validates marker_scan, ioc::parser, find_loop_body_gap, and graph::builder against real generated output
// from CubeMX projects (stakhal_blink_f446re and stm32_03_timers) to catch regressions going forward.

use std::fs;
use std::path::Path;

use stakhal_core::graph::build_call_graph;
use stakhal_core::ioc::parse_ioc;
use stakhal_core::source::{extract_pv_declarations, find_loop_body_gap, scan_file};

#[test]
fn test_real_cubemx_project_regression() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture_dir = manifest_dir.join("tests/fixtures/stakhal_blink_f446re");
    let ioc_path = fixture_dir.join("stakhal_blink_f446re.ioc");
    let main_c_path = fixture_dir.join("Core/Src/main.c");

    // 1. Validate .ioc parsing
    let ioc_project = parse_ioc(&ioc_path).expect("failed to parse real fixture .ioc file");
    assert_eq!(ioc_project.mcu_family, "STM32F4");
    assert_eq!(ioc_project.mcu_name, "STM32F446RETx");
    assert_eq!(ioc_project.pins.len(), 7);
    assert_eq!(ioc_project.peripherals.len(), 0);

    // 2. Validate C source scanning
    let regions = scan_file(&main_c_path).expect("failed to scan real fixture main.c file");
    assert_eq!(regions.len(), 19);

    let spot_check_tags = ["Header", "PV", "WHILE", "Error_Handler_Debug"];
    for tag in spot_check_tags {
        assert!(
            regions.iter().any(|r| r.tag == tag),
            "Expected tag '{}' to be present in scanned regions",
            tag
        );
    }

    // 3. Validate implicit loop body gap detection
    let loop_body_gap = find_loop_body_gap(&regions)
        .expect("find_loop_body_gap returned None on real CubeMX project");
    assert_eq!(loop_body_gap.tag, "__loop_body__");

    let source_content =
        fs::read_to_string(&main_c_path).expect("failed to read main.c content for slice assertion");
    let gap_content = &source_content[loop_body_gap.byte_range.0..loop_body_gap.byte_range.1];

    assert!(
        gap_content.contains("HAL_GPIO_TogglePin"),
        "Expected loop body gap content to contain 'HAL_GPIO_TogglePin', but got: {:?}",
        gap_content
    );
}

#[test]
fn test_stm32_03_timers_regression() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture_dir = manifest_dir.join("tests/fixtures/stm32_03_timers");
    let ioc_path = fixture_dir.join("03_timers.ioc");
    let main_c_path = fixture_dir.join("Core/Src/main.c");

    // 1. Validate .ioc parsing for multi-timer project
    let ioc_project = parse_ioc(&ioc_path).expect("failed to parse 03_timers .ioc file");
    assert_eq!(ioc_project.peripherals.len(), 5);

    let expected_timers = ["TIM1", "TIM2", "TIM3", "TIM4", "TIM6"];
    for timer in expected_timers {
        assert!(
            ioc_project.peripherals.iter().any(|p| p.name == timer),
            "Expected peripheral '{}' to be present in parsed peripherals",
            timer
        );
    }

    // 2. Validate C source scanning
    let regions = scan_file(&main_c_path).expect("failed to scan 03_timers main.c file");
    assert_eq!(regions.len(), 34);

    // 3. Validate implicit loop body gap detection
    let loop_body_gap = find_loop_body_gap(&regions)
        .expect("find_loop_body_gap returned None on 03_timers project");
    assert_eq!(loop_body_gap.tag, "__loop_body__");

    let source_content =
        fs::read_to_string(&main_c_path).expect("failed to read main.c content for slice assertion");
    let gap_content = &source_content[loop_body_gap.byte_range.0..loop_body_gap.byte_range.1];

    assert!(
        gap_content.contains("encZ1"),
        "Expected loop body gap content to contain 'encZ1', but got: {:?}",
        gap_content
    );
}

#[test]
fn test_stm32_03_timers_graph_builder_regression() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture_dir = manifest_dir.join("tests/fixtures/stm32_03_timers");
    let ioc_path = fixture_dir.join("03_timers.ioc");

    let ioc_project = parse_ioc(&ioc_path).expect("failed to parse 03_timers .ioc file");
    let edges = build_call_graph(&ioc_project);

    // Assert edges exist from TIM6_DAC_IRQHandler and TIM2_IRQHandler
    assert!(
        edges.iter().any(|e| e.from == "TIM6_DAC_IRQHandler"),
        "Expected IRQ edges for TIM6_DAC_IRQHandler"
    );
    assert!(
        edges.iter().any(|e| e.from == "TIM2_IRQHandler"),
        "Expected IRQ edges for TIM2_IRQHandler"
    );

    // Assert NO edge exists whose from starts with "TIM1_", "TIM3_", or "TIM4_"
    assert!(
        !edges.iter().any(|e| e.from.starts_with("TIM1_")),
        "Expected no IRQ edges for encoder-mode TIM1"
    );
    assert!(
        !edges.iter().any(|e| e.from.starts_with("TIM3_")),
        "Expected no IRQ edges for encoder-mode TIM3"
    );
    assert!(
        !edges.iter().any(|e| e.from.starts_with("TIM4_")),
        "Expected no IRQ edges for encoder-mode TIM4"
    );

    // Confirm Init edges exist for MX_TIM1_Init, MX_TIM3_Init, MX_TIM4_Init
    assert!(edges.iter().any(|e| e.to == "MX_TIM1_Init"));
    assert!(edges.iter().any(|e| e.to == "MX_TIM3_Init"));
    assert!(edges.iter().any(|e| e.to == "MX_TIM4_Init"));
}

#[test]
fn test_stm32_03_timers_pv_extract_regression() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture_dir = manifest_dir.join("tests/fixtures/stm32_03_timers");
    let main_c_path = fixture_dir.join("Core/Src/main.c");

    let decls = extract_pv_declarations(&main_c_path).expect("failed to extract PV declarations");

    assert_eq!(decls.len(), 8, "Expected 8 declarations in PV region");

    let isr_count = decls.iter().find(|d| d.name == "isrCount").unwrap();
    assert_eq!(isr_count.type_str, "volatile uint32_t");
    assert_eq!(isr_count.initial_value, Some("0".to_string()));
    assert_eq!(isr_count.line, 52);

    let step_interval = decls.iter().find(|d| d.name == "stepInterval").unwrap();
    assert_eq!(step_interval.type_str, "uint32_t");
    assert_eq!(step_interval.initial_value, Some("90000".to_string()));
    assert_eq!(step_interval.line, 55);

    let enc_z1 = decls.iter().find(|d| d.name == "encZ1").unwrap();
    assert_eq!(enc_z1.type_str, "volatile int32_t");
    assert_eq!(enc_z1.initial_value, Some("0".to_string()));
    assert_eq!(enc_z1.line, 58);

    let prev_z1 = decls.iter().find(|d| d.name == "prevZ1").unwrap();
    assert_eq!(prev_z1.type_str, "uint16_t");
    assert_eq!(prev_z1.initial_value, Some("0".to_string()));
    assert_eq!(prev_z1.line, 63);
}

use stakhal_core::source::find_variable_usages;

#[test]
fn test_stm32_03_timers_usage_finder_regression() {

    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture_dir = manifest_dir.join("tests/fixtures/stm32_03_timers");
    let main_c_path = fixture_dir.join("Core/Src/main.c");

    let decls = extract_pv_declarations(&main_c_path).expect("failed to extract PV declarations");
    let isr_count = decls.iter().find(|d| d.name == "isrCount").unwrap();

    let usages = find_variable_usages(&main_c_path, &isr_count.name, isr_count.byte_range)
        .expect("find_variable_usages failed");

    assert!(!usages.is_empty(), "Expected at least one usage of isrCount");

    let callback_usage = usages.iter().find(|u| u.line == 496).expect("Expected usage at line 496 in HAL_TIM_PeriodElapsedCallback");
    assert_eq!(callback_usage.context_snippet, "isrCount++;");
}



