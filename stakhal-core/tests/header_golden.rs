use stakhal_core::target::TargetDefinition;
use std::fs;
use std::path::Path;

#[test]
fn test_golden_file_aa_ns_stm32_header_output() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let golden_path = manifest_dir.join("tests/fixtures/golden_aa_ns_stm32_build_info.h");
    let golden_bytes =
        fs::read(&golden_path).expect("Failed to read AA_NS_STM32 golden header fixture");

    let target = TargetDefinition::aa_ns_stm32();
    let generated = target.render_header("abcdef12");

    assert_eq!(
        generated.as_bytes(),
        golden_bytes.as_slice(),
        "Generated AA_NS_STM32 header must be byte-identical to golden fixture"
    );
}

#[test]
fn test_regression_guard_aa_ns_stm_header_unchanged() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let golden_path = manifest_dir.join("tests/fixtures/golden_aa_ns_stm_build_info.h");
    let golden_bytes =
        fs::read(&golden_path).expect("Failed to read AA_NS_STM golden header fixture");

    let target = TargetDefinition::legacy();
    let generated = target.render_header("test1234");

    assert_eq!(
        generated.as_bytes(),
        golden_bytes.as_slice(),
        "Legacy AA_NS_STM header must remain 100% byte-identical to original output"
    );
}
