//! Board abstractions and board detection.
//!
//! Models supported hardware development boards (e.g. ST NUCLEO-F446RE,
//! WeAct Studio Black Pill STM32F411CEU6) and provides hardware definitions
//! including connector layouts, pinout tables, and reserved pin rules.

use crate::blackpill_pinout::{BLACKPILL_F411CE_PINOUT, BLACKPILL_RESERVED_PINS};
use crate::nucleo_pinout::{
    PinLocation, ReservedPin, NUCLEO_F446RE_PINOUT, RESERVED_PINS as NUCLEO_RESERVED_PINS,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BoardId {
    NucleoF446re,
    BlackPillF411ce,
}

#[derive(Debug, Clone)]
pub struct BoardDef {
    pub id: BoardId,
    pub display_name: &'static str,
    pub hint: &'static str,
    pub chip_label: &'static str,
    pub connectors: &'static [&'static str],
    pub pin_table: &'static [PinLocation],
    pub reserved_pins: &'static [ReservedPin],
}

impl BoardDef {
    pub fn for_id(id: BoardId) -> &'static BoardDef {
        match id {
            BoardId::NucleoF446re => &NUCLEO_F446RE_DEF,
            BoardId::BlackPillF411ce => &BLACK_PILL_F411CE_DEF,
        }
    }

    pub fn lookup_pin(&self, mcu_pin: &str) -> Option<&'static PinLocation> {
        self.pin_table.iter().find(|p| p.mcu_pin == mcu_pin)
    }

    pub fn check_reserved(&self, mcu_pin: &str) -> Option<&'static ReservedPin> {
        self.reserved_pins.iter().find(|p| p.mcu_pin == mcu_pin)
    }
}

pub static NUCLEO_F446RE_DEF: BoardDef = BoardDef {
    id: BoardId::NucleoF446re,
    display_name: "[ NUCLEO-F446RE PHYSICAL CONNECTOR PINOUT ]",
    hint: "Highlighted pins indicate active signals in loaded project",
    chip_label: "STM32F446",
    connectors: &["CN7", "CN6", "CN8", "CN10", "CN5", "CN9"],
    pin_table: NUCLEO_F446RE_PINOUT,
    reserved_pins: NUCLEO_RESERVED_PINS,
};

pub static BLACK_PILL_F411CE_DEF: BoardDef = BoardDef {
    id: BoardId::BlackPillF411ce,
    display_name: "[ STM32F411 BLACK PILL PHYSICAL CONNECTOR PINOUT ]",
    hint: "Highlighted pins indicate active signals in loaded project",
    chip_label: "STM32F411",
    connectors: &["Left", "Right", "SWD"],
    pin_table: BLACKPILL_F411CE_PINOUT,
    reserved_pins: BLACKPILL_RESERVED_PINS,
};

/// Detect board ID from MCU name string (from CubeMX .ioc or similar).
///
/// Returns:
/// - `Some(BoardId::NucleoF446re)` for F446 variants
/// - `Some(BoardId::BlackPillF411ce)` for F411CE variants (e.g. STM32F411CEUx, STM32F411C(C-E)Ux)
/// - `None` for other variants (e.g. STM32F411RETx or unrecognized names)
pub fn detect_board(mcu_name: &str) -> Option<BoardId> {
    let upper = mcu_name.to_uppercase();
    if upper.contains("F446") {
        Some(BoardId::NucleoF446re)
    } else if upper.contains("F411CE") || upper.contains("F411C(C-E)U") {
        Some(BoardId::BlackPillF411ce)
    } else {
        None
    }
}

pub fn lookup_pin(board_id: BoardId, mcu_pin: &str) -> Option<&'static PinLocation> {
    BoardDef::for_id(board_id).lookup_pin(mcu_pin)
}

pub fn check_reserved(board_id: BoardId, mcu_pin: &str) -> Option<&'static ReservedPin> {
    BoardDef::for_id(board_id).check_reserved(mcu_pin)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_board_variants() {
        assert_eq!(detect_board("STM32F446RETx"), Some(BoardId::NucleoF446re));
        assert_eq!(
            detect_board("STM32F411CEUx"),
            Some(BoardId::BlackPillF411ce)
        );
        assert_eq!(
            detect_board("STM32F411C(C-E)Ux"),
            Some(BoardId::BlackPillF411ce)
        );
        assert_eq!(detect_board("STM32F411RETx"), None);
        assert_eq!(detect_board(""), None);
    }

    #[test]
    fn test_nucleo_board_def_wired() {
        let def = BoardDef::for_id(BoardId::NucleoF446re);
        assert_eq!(def.chip_label, "STM32F446");
        assert_eq!(
            def.display_name,
            "[ NUCLEO-F446RE PHYSICAL CONNECTOR PINOUT ]"
        );
        assert!(def.lookup_pin("PA5").is_some());
        assert!(def.check_reserved("PA13").is_some());
    }

    #[test]
    fn test_blackpill_board_def_wired() {
        let def = BoardDef::for_id(BoardId::BlackPillF411ce);
        assert_eq!(def.chip_label, "STM32F411");
        assert_eq!(
            def.display_name,
            "[ STM32F411 BLACK PILL PHYSICAL CONNECTOR PINOUT ]"
        );
        assert!(def.lookup_pin("PA0").is_some());
        let pa13_res = def.check_reserved("PA13").expect("PA13 should be reserved");
        assert_eq!(
            pa13_res.severity,
            crate::nucleo_pinout::ReservedSeverity::Critical
        );
        let pc13_res = def.check_reserved("PC13").expect("PC13 should be reserved");
        assert_eq!(
            pc13_res.severity,
            crate::nucleo_pinout::ReservedSeverity::Caution
        );
    }
}
