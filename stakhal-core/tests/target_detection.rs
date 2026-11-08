use stakhal_core::target::{detect_target_from_project_dir, TargetId};
use std::fs;
use std::path::Path;

#[test]
fn test_detect_target_aa_ns_stm32_ioc() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();

    fs::write(root.join("AA_STM.ioc"), "Mcu.Family=STM32F4\n").unwrap();
    let target = detect_target_from_project_dir(root).expect("Should detect AA_NS_STM32");
    assert_eq!(target.id, TargetId::AaNsStm32);
    assert_eq!(target.name, "AA_NS_STM32");
}

#[test]
fn test_detect_target_aa_ns_stm32_aa_config() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();

    let inc = root.join("Core").join("Inc");
    fs::create_dir_all(&inc).unwrap();
    fs::write(inc.join("aa_config.h"), "#define NUM_AXES 4\n").unwrap();

    let target = detect_target_from_project_dir(root).expect("Should detect AA_NS_STM32");
    assert_eq!(target.id, TargetId::AaNsStm32);
}

#[test]
fn test_detect_target_aa_ns_stm32_cmakelists() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();

    let cmake = r#"
cmake_minimum_required(VERSION 3.22)
set(CMAKE_PROJECT_NAME AA_STM)
project(${CMAKE_PROJECT_NAME})
"#;
    fs::write(root.join("CMakeLists.txt"), cmake).unwrap();

    let target = detect_target_from_project_dir(root).expect("Should detect AA_NS_STM32");
    assert_eq!(target.id, TargetId::AaNsStm32);
}

#[test]
fn test_detect_target_legacy_fixture() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture_dir = manifest_dir.join("tests/fixtures/aa_ns_stm_port");
    if fixture_dir.exists() {
        let target =
            detect_target_from_project_dir(&fixture_dir).expect("Should detect legacy AA_NS_STM");
        assert_eq!(target.id, TargetId::AaNsStm);
        assert_eq!(target.name, "AA_NS_STM");
    }
}

#[test]
fn test_detect_target_external_ref_if_present() {
    let ref_var = match std::env::var("STAKHAL_TEST_AA_NS_STM32") {
        Ok(val) => val,
        Err(_) => {
            eprintln!(
                "skipping test_detect_target_external_ref_if_present: STAKHAL_TEST_AA_NS_STM32 not set or path missing"
            );
            return;
        }
    };
    let ref_path = std::path::PathBuf::from(ref_var);
    if !ref_path.is_dir() {
        eprintln!(
            "skipping test_detect_target_external_ref_if_present: STAKHAL_TEST_AA_NS_STM32 not set or path missing"
        );
        return;
    }

    let target =
        detect_target_from_project_dir(&ref_path).expect("Should detect AA_NS_STM32 for ref repo");
    assert_eq!(target.id, TargetId::AaNsStm32);
    assert_eq!(target.num_axes, 4);
}

#[test]
fn test_detect_target_unknown_returns_none() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();

    fs::write(root.join("other.txt"), "hello").unwrap();
    assert!(detect_target_from_project_dir(root).is_none());
}
