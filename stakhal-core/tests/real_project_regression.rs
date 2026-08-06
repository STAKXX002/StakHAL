// This integration test suite exists because synthetic string fixtures did not catch that
// CubeMX's main loop body lives in an unmarked gap between markers (USER CODE END WHILE and USER CODE BEGIN 3).
// This test suite validates marker_scan, ioc::parser, and find_loop_body_gap against real generated output
// from CubeMX projects (stakhal_blink_f446re and stm32_03_timers) to catch regressions going forward.

use std::fs;
use std::path::Path;

use stakhal_core::ioc::parse_ioc;
use stakhal_core::source::{find_loop_body_gap, scan_file};

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
