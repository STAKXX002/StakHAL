use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TargetId {
    AaNsStm,
    AaNsStm32,
}

impl fmt::Display for TargetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TargetId::AaNsStm => write!(f, "AA_NS_STM"),
            TargetId::AaNsStm32 => write!(f, "AA_NS_STM32"),
        }
    }
}

pub trait Target: Send + Sync {
    fn id(&self) -> TargetId;
    fn name(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    fn mcu_family(&self) -> &'static str;
    fn mcu_part(&self) -> &'static str;
    fn sysclk_freq_hz(&self) -> u32;
    fn num_axes(&self) -> usize;
    fn serial_prefix(&self) -> &'static str;
    fn header_filename(&self) -> &'static str;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetDefinition {
    pub id: TargetId,
    pub name: String,
    pub display_name: String,
    pub mcu_family: String,
    pub mcu_part: String,
    pub sysclk_freq_hz: u32,
    pub num_axes: usize,
    pub serial_prefix: String,
    pub header_filename: String,
}

impl Target for TargetDefinition {
    fn id(&self) -> TargetId {
        self.id
    }

    fn name(&self) -> &'static str {
        match self.id {
            TargetId::AaNsStm => "AA_NS_STM",
            TargetId::AaNsStm32 => "AA_NS_STM32",
        }
    }

    fn display_name(&self) -> &'static str {
        match self.id {
            TargetId::AaNsStm => "AA_NS_STM 2-Axis Legacy Port",
            TargetId::AaNsStm32 => "AA_NS_STM32 4-Axis Platform",
        }
    }

    fn mcu_family(&self) -> &'static str {
        "STM32F4"
    }

    fn mcu_part(&self) -> &'static str {
        "STM32F446RET6"
    }

    fn sysclk_freq_hz(&self) -> u32 {
        self.sysclk_freq_hz
    }

    fn num_axes(&self) -> usize {
        self.num_axes
    }

    fn serial_prefix(&self) -> &'static str {
        match self.id {
            TargetId::AaNsStm => "STAKHAL_BUILD:",
            TargetId::AaNsStm32 => "AA_BUILD:",
        }
    }

    fn header_filename(&self) -> &'static str {
        "stakhal_build_info.h"
    }
}

impl TargetDefinition {
    pub fn legacy() -> Self {
        Self {
            id: TargetId::AaNsStm,
            name: "AA_NS_STM".to_string(),
            display_name: "AA_NS_STM 2-Axis Legacy Port".to_string(),
            mcu_family: "STM32F4".to_string(),
            mcu_part: "STM32F446RET6".to_string(),
            sysclk_freq_hz: 180_000_000,
            num_axes: 2,
            serial_prefix: "STAKHAL_BUILD:".to_string(),
            header_filename: "stakhal_build_info.h".to_string(),
        }
    }

    pub fn aa_ns_stm32() -> Self {
        Self {
            id: TargetId::AaNsStm32,
            name: "AA_NS_STM32".to_string(),
            display_name: "AA_NS_STM32 4-Axis Platform".to_string(),
            mcu_family: "STM32F4".to_string(),
            mcu_part: "STM32F446RET6".to_string(),
            sysclk_freq_hz: 180_000_000,
            num_axes: 4,
            serial_prefix: "AA_BUILD:".to_string(),
            header_filename: "stakhal_build_info.h".to_string(),
        }
    }

    /// Return all supported target definitions.
    pub fn all() -> Vec<Self> {
        vec![Self::legacy(), Self::aa_ns_stm32()]
    }

    /// Resolve a target definition from a case-insensitive string name.
    pub fn from_name(name: &str) -> Option<Self> {
        let normalized = name.trim().to_ascii_uppercase();
        match normalized.as_str() {
            "AA_NS_STM" | "AA_NS_STM_V1" | "LEGACY" => Some(Self::legacy()),
            "AA_NS_STM32" | "AA_STM" => Some(Self::aa_ns_stm32()),
            _ => None,
        }
    }

    /// Render stakhal_build_info.h header content for this target.
    pub fn render_header(&self, hash: &str) -> String {
        match self.id {
            TargetId::AaNsStm => format!(
                "/* Generated automatically by StakHAL. Do not edit or commit. */\n\
                 #ifndef STAKHAL_BUILD_INFO_H\n\
                 #define STAKHAL_BUILD_INFO_H\n\n\
                 #define STAKHAL_BUILD_HASH \"{}\"\n\n\
                 #endif /* STAKHAL_BUILD_INFO_H */\n",
                hash
            ),
            TargetId::AaNsStm32 => format!(
                "/* Generated automatically by StakHAL. Do not edit or commit. */\n\
                 #ifndef STAKHAL_BUILD_INFO_H\n\
                 #define STAKHAL_BUILD_INFO_H\n\n\
                 #define STAKHAL_TARGET_NAME \"{}\"\n\
                 #define STAKHAL_BUILD_HASH \"{}\"\n\
                 #define AA_BUILD_HASH \"{}\"\n\
                 #define STAKHAL_MCU_FAMILY \"{}\"\n\
                 #define STAKHAL_MCU_PART \"{}\"\n\
                 #define STAKHAL_SYSCLK_FREQ_HZ {}UL\n\
                 #define STAKHAL_NUM_AXES {}\n\n\
                 #endif /* STAKHAL_BUILD_INFO_H */\n",
                self.name,
                hash,
                hash,
                self.mcu_family,
                self.mcu_part,
                self.sysclk_freq_hz,
                self.num_axes,
            ),
        }
    }
}

/// Detect target platform from project directory files and layout.
pub fn detect_target_from_project_dir(project_dir: &Path) -> Option<TargetDefinition> {
    // 1. Check for AA_NS_STM32 signatures
    if project_dir.join("AA_STM.ioc").is_file()
        || project_dir.join("Core/Inc/aa_config.h").is_file()
        || project_dir.join("Core").join("Inc").join("aa_config.h").is_file()
    {
        return Some(TargetDefinition::aa_ns_stm32());
    }

    if let Ok(cmake_content) = std::fs::read_to_string(project_dir.join("CMakeLists.txt")) {
        if cmake_content.contains("CMAKE_PROJECT_NAME AA_STM")
            || cmake_content.contains("project(AA_STM")
            || cmake_content.contains("add_executable(AA_STM")
        {
            return Some(TargetDefinition::aa_ns_stm32());
        }
    }

    // 2. Check for legacy AA_NS_STM signatures
    if project_dir.join("aa_ns_stm_port.ioc").is_file() {
        return Some(TargetDefinition::legacy());
    }

    if let Ok(cmake_content) = std::fs::read_to_string(project_dir.join("CMakeLists.txt")) {
        if cmake_content.contains("AA_NS_STM_V1") || cmake_content.contains("aa_ns_stm_port") {
            return Some(TargetDefinition::legacy());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_legacy_target_definition_properties() {
        let target = TargetDefinition::legacy();
        assert_eq!(target.id(), TargetId::AaNsStm);
        assert_eq!(target.name(), "AA_NS_STM");
        assert_eq!(target.mcu_family(), "STM32F4");
        assert_eq!(target.mcu_part(), "STM32F446RET6");
        assert_eq!(target.sysclk_freq_hz(), 180_000_000);
        assert_eq!(target.num_axes(), 2);
        assert_eq!(target.serial_prefix(), "STAKHAL_BUILD:");
        assert_eq!(target.header_filename(), "stakhal_build_info.h");
    }

    #[test]
    fn test_aa_ns_stm32_target_definition_properties() {
        let target = TargetDefinition::aa_ns_stm32();
        assert_eq!(target.id(), TargetId::AaNsStm32);
        assert_eq!(target.name(), "AA_NS_STM32");
        assert_eq!(target.display_name(), "AA_NS_STM32 4-Axis Platform");
        assert_eq!(target.mcu_family(), "STM32F4");
        assert_eq!(target.mcu_part(), "STM32F446RET6");
        assert_eq!(target.sysclk_freq_hz(), 180_000_000);
        assert_eq!(target.num_axes(), 4);
        assert_eq!(target.serial_prefix(), "AA_BUILD:");
        assert_eq!(target.header_filename(), "stakhal_build_info.h");
    }

    #[test]
    fn test_target_registry_all_and_from_name() {
        let all = TargetDefinition::all();
        assert_eq!(all.len(), 2);

        assert_eq!(
            TargetDefinition::from_name("AA_NS_STM").map(|t| t.id),
            Some(TargetId::AaNsStm)
        );
        assert_eq!(
            TargetDefinition::from_name("aa_ns_stm_v1").map(|t| t.id),
            Some(TargetId::AaNsStm)
        );
        assert_eq!(
            TargetDefinition::from_name("AA_NS_STM32").map(|t| t.id),
            Some(TargetId::AaNsStm32)
        );
        assert_eq!(
            TargetDefinition::from_name("aa_stm").map(|t| t.id),
            Some(TargetId::AaNsStm32)
        );
        assert!(TargetDefinition::from_name("unknown_target").is_none());
    }

    #[test]
    fn test_detect_target_from_project_dir_aa_ns_stm32() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path();

        // Initially unknown
        assert!(detect_target_from_project_dir(path).is_none());

        // Add AA_STM.ioc
        std::fs::write(path.join("AA_STM.ioc"), "Mcu.Family=STM32F4\n").unwrap();
        let detected = detect_target_from_project_dir(path);
        assert_eq!(detected.map(|t| t.id), Some(TargetId::AaNsStm32));
    }

    #[test]
    fn test_detect_target_from_project_dir_legacy_fixture() {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let fixture_path = manifest_dir.join("tests/fixtures/aa_ns_stm_port");
        if fixture_path.exists() {
            let detected = detect_target_from_project_dir(&fixture_path);
            assert_eq!(detected.map(|t| t.id), Some(TargetId::AaNsStm));
        }
    }
}
