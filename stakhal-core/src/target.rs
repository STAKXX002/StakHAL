use serde::{Deserialize, Serialize};
use std::fmt;

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
}
