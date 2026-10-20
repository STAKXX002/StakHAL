#![allow(clippy::let_unit_value)]

use crate::state::{AppState, AppWidgets};
use crate::ui::tokens;
use gtk4::cairo;
use gtk4::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub struct PinDef {
    pub pin_num: u8,
    pub mcu_pin: &'static str,
    pub default_label: Option<&'static str>,
}

pub struct ConnectorDef {
    pub name: &'static str,
    pub pins: &'static [PinDef],
}

const CN7_PINS: &[PinDef] = &[
    PinDef {
        pin_num: 1,
        mcu_pin: "PC10",
        default_label: None,
    },
    PinDef {
        pin_num: 2,
        mcu_pin: "PC11",
        default_label: None,
    },
    PinDef {
        pin_num: 3,
        mcu_pin: "PC12",
        default_label: None,
    },
    PinDef {
        pin_num: 4,
        mcu_pin: "PD2",
        default_label: None,
    },
    PinDef {
        pin_num: 5,
        mcu_pin: "VDD",
        default_label: Some("3V3"),
    },
    PinDef {
        pin_num: 6,
        mcu_pin: "E5V",
        default_label: Some("5V"),
    },
    PinDef {
        pin_num: 7,
        mcu_pin: "BOOT0",
        default_label: None,
    },
    PinDef {
        pin_num: 8,
        mcu_pin: "GND",
        default_label: None,
    },
    PinDef {
        pin_num: 9,
        mcu_pin: "NC",
        default_label: None,
    },
    PinDef {
        pin_num: 10,
        mcu_pin: "NC",
        default_label: None,
    },
    PinDef {
        pin_num: 11,
        mcu_pin: "NC",
        default_label: None,
    },
    PinDef {
        pin_num: 12,
        mcu_pin: "IOREF",
        default_label: None,
    },
    PinDef {
        pin_num: 13,
        mcu_pin: "PA13",
        default_label: Some("SWDIO"),
    },
    PinDef {
        pin_num: 14,
        mcu_pin: "RESET",
        default_label: None,
    },
    PinDef {
        pin_num: 15,
        mcu_pin: "PA14",
        default_label: Some("SWCLK"),
    },
    PinDef {
        pin_num: 16,
        mcu_pin: "+3V3",
        default_label: None,
    },
    PinDef {
        pin_num: 17,
        mcu_pin: "PA15",
        default_label: None,
    },
    PinDef {
        pin_num: 18,
        mcu_pin: "+5V",
        default_label: None,
    },
    PinDef {
        pin_num: 19,
        mcu_pin: "GND",
        default_label: None,
    },
    PinDef {
        pin_num: 20,
        mcu_pin: "GND",
        default_label: None,
    },
    PinDef {
        pin_num: 21,
        mcu_pin: "PB7",
        default_label: None,
    },
    PinDef {
        pin_num: 22,
        mcu_pin: "GND",
        default_label: None,
    },
    PinDef {
        pin_num: 23,
        mcu_pin: "PC13",
        default_label: Some("USER_BTN"),
    },
    PinDef {
        pin_num: 24,
        mcu_pin: "VIN",
        default_label: None,
    },
    PinDef {
        pin_num: 25,
        mcu_pin: "PC14",
        default_label: Some("OSC32_IN"),
    },
    PinDef {
        pin_num: 26,
        mcu_pin: "NC",
        default_label: None,
    },
    PinDef {
        pin_num: 27,
        mcu_pin: "PC15",
        default_label: Some("OSC32_OUT"),
    },
    PinDef {
        pin_num: 28,
        mcu_pin: "PA0",
        default_label: Some("A0"),
    },
    PinDef {
        pin_num: 29,
        mcu_pin: "PH0",
        default_label: Some("OSC_IN"),
    },
    PinDef {
        pin_num: 30,
        mcu_pin: "PA1",
        default_label: Some("A1"),
    },
    PinDef {
        pin_num: 31,
        mcu_pin: "PH1",
        default_label: Some("OSC_OUT"),
    },
    PinDef {
        pin_num: 32,
        mcu_pin: "PA4",
        default_label: Some("A2"),
    },
    PinDef {
        pin_num: 33,
        mcu_pin: "VBAT",
        default_label: None,
    },
    PinDef {
        pin_num: 34,
        mcu_pin: "PB0",
        default_label: Some("A3"),
    },
    PinDef {
        pin_num: 35,
        mcu_pin: "PC2",
        default_label: None,
    },
    PinDef {
        pin_num: 36,
        mcu_pin: "PC1",
        default_label: Some("A4"),
    },
    PinDef {
        pin_num: 37,
        mcu_pin: "PC3",
        default_label: None,
    },
    PinDef {
        pin_num: 38,
        mcu_pin: "PC0",
        default_label: Some("A5"),
    },
];

const CN6_PINS: &[PinDef] = &[
    PinDef {
        pin_num: 1,
        mcu_pin: "NC",
        default_label: None,
    },
    PinDef {
        pin_num: 2,
        mcu_pin: "IOREF",
        default_label: None,
    },
    PinDef {
        pin_num: 3,
        mcu_pin: "RESET",
        default_label: None,
    },
    PinDef {
        pin_num: 4,
        mcu_pin: "+3V3",
        default_label: None,
    },
    PinDef {
        pin_num: 5,
        mcu_pin: "+5V",
        default_label: None,
    },
    PinDef {
        pin_num: 6,
        mcu_pin: "GND",
        default_label: None,
    },
    PinDef {
        pin_num: 7,
        mcu_pin: "GND",
        default_label: None,
    },
    PinDef {
        pin_num: 8,
        mcu_pin: "VIN",
        default_label: None,
    },
];

const CN8_PINS: &[PinDef] = &[
    PinDef {
        pin_num: 1,
        mcu_pin: "PA0",
        default_label: Some("A0"),
    },
    PinDef {
        pin_num: 2,
        mcu_pin: "PA1",
        default_label: Some("A1"),
    },
    PinDef {
        pin_num: 3,
        mcu_pin: "PA4",
        default_label: Some("A2"),
    },
    PinDef {
        pin_num: 4,
        mcu_pin: "PB0",
        default_label: Some("A3"),
    },
    PinDef {
        pin_num: 5,
        mcu_pin: "PC1",
        default_label: Some("A4"),
    },
    PinDef {
        pin_num: 6,
        mcu_pin: "PC0",
        default_label: Some("A5"),
    },
];

const CN10_PINS: &[PinDef] = &[
    PinDef {
        pin_num: 1,
        mcu_pin: "PC9",
        default_label: None,
    },
    PinDef {
        pin_num: 2,
        mcu_pin: "PC8",
        default_label: None,
    },
    PinDef {
        pin_num: 3,
        mcu_pin: "PB8",
        default_label: Some("D15"),
    },
    PinDef {
        pin_num: 4,
        mcu_pin: "PC6",
        default_label: None,
    },
    PinDef {
        pin_num: 5,
        mcu_pin: "PB9",
        default_label: Some("D14"),
    },
    PinDef {
        pin_num: 6,
        mcu_pin: "PC5",
        default_label: None,
    },
    PinDef {
        pin_num: 7,
        mcu_pin: "AVDD",
        default_label: None,
    },
    PinDef {
        pin_num: 8,
        mcu_pin: "U5V",
        default_label: None,
    },
    PinDef {
        pin_num: 9,
        mcu_pin: "GND",
        default_label: None,
    },
    PinDef {
        pin_num: 10,
        mcu_pin: "NC",
        default_label: None,
    },
    PinDef {
        pin_num: 11,
        mcu_pin: "PA5",
        default_label: Some("D13"),
    },
    PinDef {
        pin_num: 12,
        mcu_pin: "PA12",
        default_label: None,
    },
    PinDef {
        pin_num: 13,
        mcu_pin: "PA6",
        default_label: Some("D12"),
    },
    PinDef {
        pin_num: 14,
        mcu_pin: "PA11",
        default_label: None,
    },
    PinDef {
        pin_num: 15,
        mcu_pin: "PA7",
        default_label: Some("D11"),
    },
    PinDef {
        pin_num: 16,
        mcu_pin: "PB12",
        default_label: None,
    },
    PinDef {
        pin_num: 17,
        mcu_pin: "PB6",
        default_label: Some("D10"),
    },
    PinDef {
        pin_num: 18,
        mcu_pin: "NC",
        default_label: None,
    },
    PinDef {
        pin_num: 19,
        mcu_pin: "PC7",
        default_label: Some("D9"),
    },
    PinDef {
        pin_num: 20,
        mcu_pin: "GND",
        default_label: None,
    },
    PinDef {
        pin_num: 21,
        mcu_pin: "PA9",
        default_label: Some("D8"),
    },
    PinDef {
        pin_num: 22,
        mcu_pin: "PB2",
        default_label: None,
    },
    PinDef {
        pin_num: 23,
        mcu_pin: "PA8",
        default_label: Some("D7"),
    },
    PinDef {
        pin_num: 24,
        mcu_pin: "PB1",
        default_label: None,
    },
    PinDef {
        pin_num: 25,
        mcu_pin: "PB10",
        default_label: Some("D6"),
    },
    PinDef {
        pin_num: 26,
        mcu_pin: "PB15",
        default_label: None,
    },
    PinDef {
        pin_num: 27,
        mcu_pin: "PB4",
        default_label: Some("D5"),
    },
    PinDef {
        pin_num: 28,
        mcu_pin: "PB14",
        default_label: None,
    },
    PinDef {
        pin_num: 29,
        mcu_pin: "PB5",
        default_label: Some("D4"),
    },
    PinDef {
        pin_num: 30,
        mcu_pin: "PB13",
        default_label: None,
    },
    PinDef {
        pin_num: 31,
        mcu_pin: "PB3",
        default_label: Some("D3"),
    },
    PinDef {
        pin_num: 32,
        mcu_pin: "AGND",
        default_label: None,
    },
    PinDef {
        pin_num: 33,
        mcu_pin: "PA10",
        default_label: Some("D2"),
    },
    PinDef {
        pin_num: 34,
        mcu_pin: "PC4",
        default_label: None,
    },
    PinDef {
        pin_num: 35,
        mcu_pin: "PA2",
        default_label: Some("D1"),
    },
    PinDef {
        pin_num: 36,
        mcu_pin: "NC",
        default_label: None,
    },
    PinDef {
        pin_num: 37,
        mcu_pin: "PA3",
        default_label: Some("D0"),
    },
    PinDef {
        pin_num: 38,
        mcu_pin: "NC",
        default_label: None,
    },
];

const CN5_PINS: &[PinDef] = &[
    PinDef {
        pin_num: 1,
        mcu_pin: "PA9",
        default_label: Some("D8"),
    },
    PinDef {
        pin_num: 2,
        mcu_pin: "PC7",
        default_label: Some("D9"),
    },
    PinDef {
        pin_num: 3,
        mcu_pin: "PB6",
        default_label: Some("D10"),
    },
    PinDef {
        pin_num: 4,
        mcu_pin: "PA7",
        default_label: Some("D11"),
    },
    PinDef {
        pin_num: 5,
        mcu_pin: "PA6",
        default_label: Some("D12"),
    },
    PinDef {
        pin_num: 6,
        mcu_pin: "PA5",
        default_label: Some("D13"),
    },
    PinDef {
        pin_num: 7,
        mcu_pin: "GND",
        default_label: None,
    },
    PinDef {
        pin_num: 8,
        mcu_pin: "AREF",
        default_label: None,
    },
    PinDef {
        pin_num: 9,
        mcu_pin: "PB9",
        default_label: Some("D14"),
    },
    PinDef {
        pin_num: 10,
        mcu_pin: "PB8",
        default_label: Some("D15"),
    },
];

const CN9_PINS: &[PinDef] = &[
    PinDef {
        pin_num: 1,
        mcu_pin: "PA3",
        default_label: Some("D0"),
    },
    PinDef {
        pin_num: 2,
        mcu_pin: "PA2",
        default_label: Some("D1"),
    },
    PinDef {
        pin_num: 3,
        mcu_pin: "PA10",
        default_label: Some("D2"),
    },
    PinDef {
        pin_num: 4,
        mcu_pin: "PB3",
        default_label: Some("D3"),
    },
    PinDef {
        pin_num: 5,
        mcu_pin: "PB5",
        default_label: Some("D4"),
    },
    PinDef {
        pin_num: 6,
        mcu_pin: "PB4",
        default_label: Some("D5"),
    },
    PinDef {
        pin_num: 7,
        mcu_pin: "PB10",
        default_label: Some("D6"),
    },
    PinDef {
        pin_num: 8,
        mcu_pin: "PA8",
        default_label: Some("D7"),
    },
];

const BLACKPILL_LEFT_PINS: &[PinDef] = &[
    PinDef {
        pin_num: 1,
        mcu_pin: "5V",
        default_label: None,
    },
    PinDef {
        pin_num: 2,
        mcu_pin: "GND",
        default_label: None,
    },
    PinDef {
        pin_num: 3,
        mcu_pin: "3V3",
        default_label: None,
    },
    PinDef {
        pin_num: 4,
        mcu_pin: "PB10",
        default_label: Some("B10"),
    },
    PinDef {
        pin_num: 5,
        mcu_pin: "PB2",
        default_label: Some("B2"),
    },
    PinDef {
        pin_num: 6,
        mcu_pin: "PB1",
        default_label: Some("B1"),
    },
    PinDef {
        pin_num: 7,
        mcu_pin: "PB0",
        default_label: Some("B0"),
    },
    PinDef {
        pin_num: 8,
        mcu_pin: "PA7",
        default_label: Some("A7"),
    },
    PinDef {
        pin_num: 9,
        mcu_pin: "PA6",
        default_label: Some("A6"),
    },
    PinDef {
        pin_num: 10,
        mcu_pin: "PA5",
        default_label: Some("A5"),
    },
    PinDef {
        pin_num: 11,
        mcu_pin: "PA4",
        default_label: Some("A4"),
    },
    PinDef {
        pin_num: 12,
        mcu_pin: "PA3",
        default_label: Some("A3"),
    },
    PinDef {
        pin_num: 13,
        mcu_pin: "PA2",
        default_label: Some("A2"),
    },
    PinDef {
        pin_num: 14,
        mcu_pin: "PA1",
        default_label: Some("A1"),
    },
    PinDef {
        pin_num: 15,
        mcu_pin: "PA0",
        default_label: Some("KEY"),
    },
    PinDef {
        pin_num: 16,
        mcu_pin: "NRST",
        default_label: None,
    },
    PinDef {
        pin_num: 17,
        mcu_pin: "PC15",
        default_label: Some("C15"),
    },
    PinDef {
        pin_num: 18,
        mcu_pin: "PC14",
        default_label: Some("C14"),
    },
    PinDef {
        pin_num: 19,
        mcu_pin: "PC13",
        default_label: Some("LED"),
    },
    PinDef {
        pin_num: 20,
        mcu_pin: "VBAT",
        default_label: None,
    },
];

const BLACKPILL_RIGHT_PINS: &[PinDef] = &[
    PinDef {
        pin_num: 1,
        mcu_pin: "PB12",
        default_label: Some("B12"),
    },
    PinDef {
        pin_num: 2,
        mcu_pin: "PB13",
        default_label: Some("B13"),
    },
    PinDef {
        pin_num: 3,
        mcu_pin: "PB14",
        default_label: Some("B14"),
    },
    PinDef {
        pin_num: 4,
        mcu_pin: "PB15",
        default_label: Some("B15"),
    },
    PinDef {
        pin_num: 5,
        mcu_pin: "PA8",
        default_label: Some("A8"),
    },
    PinDef {
        pin_num: 6,
        mcu_pin: "PA9",
        default_label: Some("A9"),
    },
    PinDef {
        pin_num: 7,
        mcu_pin: "PA10",
        default_label: Some("A10"),
    },
    PinDef {
        pin_num: 8,
        mcu_pin: "PA11",
        default_label: Some("A11"),
    },
    PinDef {
        pin_num: 9,
        mcu_pin: "PA12",
        default_label: Some("A12"),
    },
    PinDef {
        pin_num: 10,
        mcu_pin: "PA15",
        default_label: Some("A15"),
    },
    PinDef {
        pin_num: 11,
        mcu_pin: "PB3",
        default_label: Some("B3"),
    },
    PinDef {
        pin_num: 12,
        mcu_pin: "PB4",
        default_label: Some("B4"),
    },
    PinDef {
        pin_num: 13,
        mcu_pin: "PB5",
        default_label: Some("B5"),
    },
    PinDef {
        pin_num: 14,
        mcu_pin: "PB6",
        default_label: Some("B6"),
    },
    PinDef {
        pin_num: 15,
        mcu_pin: "PB7",
        default_label: Some("B7"),
    },
    PinDef {
        pin_num: 16,
        mcu_pin: "PB8",
        default_label: Some("B8"),
    },
    PinDef {
        pin_num: 17,
        mcu_pin: "PB9",
        default_label: Some("B9"),
    },
    PinDef {
        pin_num: 18,
        mcu_pin: "5V",
        default_label: None,
    },
    PinDef {
        pin_num: 19,
        mcu_pin: "GND",
        default_label: None,
    },
    PinDef {
        pin_num: 20,
        mcu_pin: "3V3",
        default_label: None,
    },
];

const BLACKPILL_SWD_PINS: &[PinDef] = &[
    PinDef {
        pin_num: 1,
        mcu_pin: "3V3",
        default_label: None,
    },
    PinDef {
        pin_num: 2,
        mcu_pin: "PA13",
        default_label: Some("SWDIO"),
    },
    PinDef {
        pin_num: 3,
        mcu_pin: "PA14",
        default_label: Some("SWCLK"),
    },
    PinDef {
        pin_num: 4,
        mcu_pin: "GND",
        default_label: None,
    },
];

pub const NUCLEO_CONNECTORS: &[ConnectorDef] = &[
    ConnectorDef {
        name: "CN7",
        pins: CN7_PINS,
    },
    ConnectorDef {
        name: "CN6",
        pins: CN6_PINS,
    },
    ConnectorDef {
        name: "CN8",
        pins: CN8_PINS,
    },
    ConnectorDef {
        name: "CN10",
        pins: CN10_PINS,
    },
    ConnectorDef {
        name: "CN5",
        pins: CN5_PINS,
    },
    ConnectorDef {
        name: "CN9",
        pins: CN9_PINS,
    },
];

pub const BLACKPILL_CONNECTORS: &[ConnectorDef] = &[
    ConnectorDef {
        name: "Left",
        pins: BLACKPILL_LEFT_PINS,
    },
    ConnectorDef {
        name: "Right",
        pins: BLACKPILL_RIGHT_PINS,
    },
    ConnectorDef {
        name: "SWD",
        pins: BLACKPILL_SWD_PINS,
    },
];

pub const CONNECTORS: &[ConnectorDef] = NUCLEO_CONNECTORS;

pub struct BoardLayout {
    pub board_id: stakhal_core::board::BoardId,
    pub title: &'static str,
    pub subtitle: &'static str,
    pub chip_label: &'static str,
    pub chip_sublabel_1: &'static str,
    pub chip_sublabel_2: &'static str,
    pub connectors: &'static [ConnectorDef],
    pub has_debugger_notch: bool,
    pub debugger_label: Option<&'static str>,
    pub led_label: &'static str,
    pub led_pin: &'static str,
    pub is_led_active: fn(&HashMap<(&'static str, u8), PinHighlightInfo>) -> bool,
}

impl BoardLayout {
    pub fn for_board(id: stakhal_core::board::BoardId) -> &'static BoardLayout {
        match id {
            stakhal_core::board::BoardId::NucleoF446re => &NUCLEO_LAYOUT,
            stakhal_core::board::BoardId::BlackPillF411ce => &BLACKPILL_LAYOUT,
        }
    }
}

fn is_nucleo_led_active(highlights: &HashMap<(&'static str, u8), PinHighlightInfo>) -> bool {
    highlights.contains_key(&("CN10", 11)) || highlights.contains_key(&("CN5", 6))
}

fn is_blackpill_led_active(highlights: &HashMap<(&'static str, u8), PinHighlightInfo>) -> bool {
    highlights.contains_key(&("Left", 19))
}

pub static NUCLEO_LAYOUT: BoardLayout = BoardLayout {
    board_id: stakhal_core::board::BoardId::NucleoF446re,
    title: "STMicroelectronics NUCLEO-F446RE",
    subtitle: "ARM® Cortex®-M4 MCU @ 180MHz • Physical 64-Pin Connector Pinout",
    chip_label: "STM32F446",
    chip_sublabel_1: "RET6",
    chip_sublabel_2: "LQFP64",
    connectors: NUCLEO_CONNECTORS,
    has_debugger_notch: true,
    debugger_label: Some("ST-LINK V2-1 ON-BOARD DEBUGGER"),
    led_label: "LD2",
    led_pin: "PA5",
    is_led_active: is_nucleo_led_active,
};

pub static BLACKPILL_LAYOUT: BoardLayout = BoardLayout {
    board_id: stakhal_core::board::BoardId::BlackPillF411ce,
    title: "WeAct Studio STM32F411CEU6 Black Pill",
    subtitle: "ARM® Cortex®-M4 MCU @ 100MHz • Physical 40-Pin Header Pinout",
    chip_label: "STM32F411",
    chip_sublabel_1: "CEU6",
    chip_sublabel_2: "UFQFPN48",
    connectors: BLACKPILL_CONNECTORS,
    has_debugger_notch: false,
    debugger_label: None,
    led_label: "LED",
    led_pin: "PC13",
    is_led_active: is_blackpill_led_active,
};

#[derive(Debug, Clone)]
pub struct PinHighlightInfo {
    pub signal: String,
    pub label: Option<String>,
    pub modules: Vec<String>,
    pub is_muted: bool,
}

pub fn get_active_pin_highlights(
    state: &AppState,
) -> HashMap<(&'static str, u8), PinHighlightInfo> {
    let mut map = HashMap::new();
    let project_guard = state.project.borrow();
    let project = match &project_guard.loaded_project {
        Some(p) => p,
        None => return map,
    };

    let selected_mod = state.with_canvas_state(|c| c.selected_pinout_module.clone());
    let selected_mod = selected_mod.as_deref();

    let board_id = stakhal_core::board::detect_board(&project.meta.mcu_name)
        .unwrap_or(stakhal_core::board::BoardId::NucleoF446re);
    let board_def = stakhal_core::board::BoardDef::for_id(board_id);

    for pin_cfg in &project.pins {
        let mcu_pin = &pin_cfg.pin;
        let signal = &pin_cfg.signal;
        let label = &pin_cfg.label;
        let modules = &pin_cfg.modules;

        let is_muted = match selected_mod {
            None => false,
            Some(target) => !modules.iter().any(|m| m == target),
        };

        if let Some(loc) = board_def.lookup_pin(mcu_pin) {
            if let Some((conn, pin_num)) = loc.morpho {
                map.insert(
                    (conn, pin_num),
                    PinHighlightInfo {
                        signal: signal.clone(),
                        label: label.clone(),
                        modules: modules.clone(),
                        is_muted,
                    },
                );
            }
            if let Some((conn, pin_num, _label)) = loc.arduino {
                map.insert(
                    (conn, pin_num),
                    PinHighlightInfo {
                        signal: signal.clone(),
                        label: label.clone(),
                        modules: modules.clone(),
                        is_muted,
                    },
                );
            }
            if let Some((conn, pin_num, _label)) = loc.header {
                map.insert(
                    (conn, pin_num),
                    PinHighlightInfo {
                        signal: signal.clone(),
                        label: label.clone(),
                        modules: modules.clone(),
                        is_muted,
                    },
                );
            }
        }
    }

    map
}

/// Determines if a configured pin's signal is its intended debug or oscillator function.
///
/// Per board specifications:
/// - PA13 is intended for SWD debug when signal is "SYS_JTMS-SWDIO"
/// - PA14 is intended for SWD clock when signal is "SYS_JTCK-SWCLK"
/// - PH0 / PH1 are intended for main crystal oscillator when signal starts with "RCC_OSC_"
/// - PC14 / PC15 are intended for RTC crystal oscillator when signal starts with "RCC_OSC32_"
///
/// Any other signal on these reserved pins constitutes a conflict/warning.
pub fn is_intended_pin_signal(mcu_pin: &str, signal: &str) -> bool {
    let base_pin = mcu_pin
        .split(['-', ' ', '\\'])
        .next()
        .unwrap_or(mcu_pin)
        .trim();
    match base_pin {
        "PA13" => signal == "SYS_JTMS-SWDIO",
        "PA14" => signal == "SYS_JTCK-SWCLK",
        "PH0" | "PH1" => signal.starts_with("RCC_OSC_"),
        "PC14" | "PC15" => signal.starts_with("RCC_OSC32_"),
        _ => false,
    }
}

/// Returns true if a reserved pin is being used in conflict with its intended function.
pub fn is_reserved_pin_conflict(mcu_pin: &str, signal: &str) -> bool {
    !is_intended_pin_signal(mcu_pin, signal)
}

pub fn get_pin_cell_rect(
    conn_name: &str,
    pin_idx: usize,
    canvas_w: f64,
    canvas_h: f64,
) -> (f64, f64, f64, f64) {
    let cw = canvas_w.max(800.0);
    let ch = canvas_h.max(600.0);

    let margin_x = (cw * 0.025).max(16.0);
    let margin_y = (ch * 0.035).max(20.0);

    let board_x = margin_x;
    let board_y = margin_y;
    let board_w = cw - 2.0 * margin_x;
    let board_h = ch - 2.0 * margin_y;

    match conn_name {
        "CN7" | "CN6" | "CN8" | "CN10" | "CN5" | "CN9" => {
            let pad_x = 24.0;
            let cell_gap = 4.0;
            let cell_w = ((board_w - 2.0 * pad_x - 36.0) / 7.2).clamp(135.0, 240.0);

            let row_start_y = board_y + 105.0;
            let avail_h = board_h - 145.0;
            let row_h = (avail_h / 19.0).clamp(22.0, 42.0);
            let cell_h = (row_h - 4.0).clamp(18.0, 36.0);

            let col_cn7_0 = board_x + pad_x;
            let col_cn7_1 = col_cn7_0 + cell_w + cell_gap;
            let col_cn6 = col_cn7_1 + cell_w + 16.0;

            let col_cn10_1 = board_x + board_w - pad_x - cell_w;
            let col_cn10_0 = col_cn10_1 - cell_w - cell_gap;
            let col_cn5 = col_cn10_0 - 16.0 - cell_w;

            match conn_name {
                "CN7" => {
                    let r = pin_idx / 2;
                    let c = pin_idx % 2;
                    let cell_x = if c == 0 { col_cn7_0 } else { col_cn7_1 };
                    let cell_y = row_start_y + (r as f64) * row_h;
                    (cell_x, cell_y, cell_w, cell_h)
                }
                "CN6" => {
                    let r = 4 + pin_idx;
                    let cell_x = col_cn6;
                    let cell_y = row_start_y + (r as f64) * row_h;
                    (cell_x, cell_y, cell_w, cell_h)
                }
                "CN8" => {
                    let r = 13 + pin_idx;
                    let cell_x = col_cn6;
                    let cell_y = row_start_y + (r as f64) * row_h;
                    (cell_x, cell_y, cell_w, cell_h)
                }
                "CN10" => {
                    let r = pin_idx / 2;
                    let c = pin_idx % 2;
                    let cell_x = if c == 0 { col_cn10_0 } else { col_cn10_1 };
                    let cell_y = row_start_y + (r as f64) * row_h;
                    (cell_x, cell_y, cell_w, cell_h)
                }
                "CN5" => {
                    let r = 9 - pin_idx;
                    let cell_x = col_cn5;
                    let cell_y = row_start_y + (r as f64) * row_h;
                    (cell_x, cell_y, cell_w, cell_h)
                }
                "CN9" => {
                    let r = 18 - pin_idx;
                    let cell_x = col_cn5;
                    let cell_y = row_start_y + (r as f64) * row_h;
                    (cell_x, cell_y, cell_w, cell_h)
                }
                _ => (0.0, 0.0, cell_w, cell_h),
            }
        }
        "Left" | "Right" | "SWD" => {
            let pad_x = 32.0;
            let cell_w = ((board_w - 2.0 * pad_x - 220.0) / 2.0).clamp(160.0, 260.0);

            let row_start_y = board_y + 105.0;
            let avail_h = board_h - 150.0;
            let row_h = (avail_h / 20.0).clamp(20.0, 38.0);
            let cell_h = (row_h - 3.0).clamp(17.0, 34.0);

            let col_left = board_x + pad_x;
            let col_right = board_x + board_w - pad_x - cell_w;

            match conn_name {
                "Left" => {
                    let cell_x = col_left;
                    let cell_y = row_start_y + (pin_idx as f64) * row_h;
                    (cell_x, cell_y, cell_w, cell_h)
                }
                "Right" => {
                    let cell_x = col_right;
                    let cell_y = row_start_y + (pin_idx as f64) * row_h;
                    (cell_x, cell_y, cell_w, cell_h)
                }
                "SWD" => {
                    let center_left = col_left + cell_w;
                    let center_right = col_right;
                    let center_w = center_right - center_left;
                    let swd_w = (center_w * 0.70).clamp(200.0, 360.0);
                    let swd_start_x = center_left + (center_w - swd_w) / 2.0;
                    let pin_gap = 4.0;
                    let swd_cell_w = (swd_w - 3.0 * pin_gap) / 4.0;
                    let cell_x = swd_start_x + (pin_idx as f64) * (swd_cell_w + pin_gap);
                    let cell_y = row_start_y + 17.5 * row_h;
                    (cell_x, cell_y, swd_cell_w, cell_h)
                }
                _ => (0.0, 0.0, cell_w, cell_h),
            }
        }
        _ => (0.0, 0.0, 100.0, 30.0),
    }
}

pub fn draw_nucleo_pinout_canvas(
    _area: &gtk4::DrawingArea,
    cr: &cairo::Context,
    width: f64,
    height: f64,
    state: &Rc<RefCell<AppState>>,
) {
    draw_nucleo_pinout(cr, width, height, state);
}

pub fn draw_nucleo_pinout(
    cr: &cairo::Context,
    width: f64,
    height: f64,
    state: &Rc<RefCell<AppState>>,
) {
    let board_id = {
        let proj = state.borrow().project.clone();
        let proj_guard = proj.borrow();
        proj_guard
            .loaded_project
            .as_ref()
            .and_then(|p| stakhal_core::board::detect_board(&p.meta.mcu_name))
            .unwrap_or(stakhal_core::board::BoardId::NucleoF446re)
    };

    match board_id {
        stakhal_core::board::BoardId::NucleoF446re => {
            draw_nucleo_f446_board(cr, width, height, state);
        }
        stakhal_core::board::BoardId::BlackPillF411ce => {
            draw_blackpill_f411_board(cr, width, height, state);
        }
    }
}

pub fn draw_nucleo_f446_board(
    cr: &cairo::Context,
    width: f64,
    height: f64,
    state: &Rc<RefCell<AppState>>,
) {
    let (hovered_pin, hovered_mouse, is_filtering_module) = state.borrow().with_canvas_state(|c| {
        (
            c.hovered_pinout_pin.clone(),
            c.hovered_pinout_mouse,
            c.selected_pinout_module.is_some(),
        )
    });
    let highlights = get_active_pin_highlights(&state.borrow());
    let hovered_pin = hovered_pin.as_ref();

    let board_id = {
        let proj = state.borrow().project.clone();
        let proj_guard = proj.borrow();
        proj_guard
            .loaded_project
            .as_ref()
            .and_then(|p| stakhal_core::board::detect_board(&p.meta.mcu_name))
            .unwrap_or(stakhal_core::board::BoardId::NucleoF446re)
    };
    let layout = BoardLayout::for_board(board_id);

    let active_conflicts: Vec<(&'static str, &stakhal_core::nucleo_pinout::ReservedPin)> = {
        let mut list = Vec::new();
        let proj = Rc::clone(&state.borrow().project);
        let proj_guard = proj.borrow();
        if let Some(project) = &proj_guard.loaded_project {
            for pin_cfg in &project.pins {
                if let Some(res) = stakhal_core::board::check_reserved(board_id, &pin_cfg.pin) {
                    if is_reserved_pin_conflict(&pin_cfg.pin, &pin_cfg.signal)
                        && !list.iter().any(|(p, _)| *p == res.mcu_pin)
                    {
                        list.push((res.mcu_pin, res));
                    }
                }
            }
        }
        list
    };

    let canvas_w = width.max(800.0);
    let canvas_h = height.max(600.0);

    let margin_x = (canvas_w * 0.025).max(16.0);
    let margin_y = (canvas_h * 0.035).max(20.0);

    let board_x = margin_x;
    let board_y = margin_y;
    let board_w = canvas_w - 2.0 * margin_x;
    let board_h = canvas_h - 2.0 * margin_y;

    let pad_x = 24.0;
    let cell_gap = 4.0;
    let cell_w = ((board_w - 2.0 * pad_x - 36.0) / 7.2).clamp(135.0, 240.0);

    let row_start_y = board_y + 105.0;
    let avail_h = board_h - 145.0;
    let row_h = (avail_h / 19.0).clamp(22.0, 42.0);

    let col_cn7_0 = board_x + pad_x;
    let col_cn7_1 = col_cn7_0 + cell_w + cell_gap;
    let col_cn6 = col_cn7_1 + cell_w + 16.0;

    let col_cn10_1 = board_x + board_w - pad_x - cell_w;
    let col_cn10_0 = col_cn10_1 - cell_w - cell_gap;
    let col_cn5 = col_cn10_0 - 16.0 - cell_w;

    // Canvas Background
    cr.set_source_rgb(
        tokens::color::BG_VOID.0,
        tokens::color::BG_VOID.1,
        tokens::color::BG_VOID.2,
    );
    cr.rectangle(0.0, 0.0, canvas_w, canvas_h);
    let _ = cr.fill();

    // 1. Board Silhouette (PCB Outline) - Surface BG_PANEL, hairline border BORDER_HAIR, 0px sharp corners
    cr.set_source_rgb(
        tokens::color::BG_PANEL.0,
        tokens::color::BG_PANEL.1,
        tokens::color::BG_PANEL.2,
    );
    cr.rectangle(board_x, board_y, board_w, board_h);
    let _ = cr.fill_preserve();
    cr.set_source_rgb(
        tokens::color::BORDER_HAIR.0,
        tokens::color::BORDER_HAIR.1,
        tokens::color::BORDER_HAIR.2,
    );
    cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
    let _ = cr.stroke();

    // Subtle PCB Grid Accent Texture Lines
    cr.set_source_rgba(
        tokens::color::BORDER_HAIR.0,
        tokens::color::BORDER_HAIR.1,
        tokens::color::BORDER_HAIR.2,
        0.25,
    );
    cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
    let mut gy = board_y + 40.0;
    while gy < board_y + board_h {
        let _ = cr.move_to(board_x + 10.0, gy);
        let _ = cr.line_to(board_x + board_w - 10.0, gy);
        let _ = cr.stroke();
        gy += 40.0;
    }

    // 2. Debugger Top Section Notch - 0px sharp corners (if board has one)
    if layout.has_debugger_notch {
        if let Some(dbg_label) = layout.debugger_label {
            let notch_w = (board_w * 0.24).clamp(220.0, 380.0);
            let notch_x = board_x + (board_w - notch_w) / 2.0;
            let notch_y = board_y + 8.0;
            cr.set_source_rgb(
                tokens::color::BG_VOID.0,
                tokens::color::BG_VOID.1,
                tokens::color::BG_VOID.2,
            );
            cr.rectangle(notch_x, notch_y, notch_w, 20.0);
            let _ = cr.fill_preserve();
            cr.set_source_rgb(
                tokens::color::BORDER_HAIR.0,
                tokens::color::BORDER_HAIR.1,
                tokens::color::BORDER_HAIR.2,
            );
            cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
            let _ = cr.stroke();

            cr.select_font_face(
                tokens::font::CAIRO_SANS,
                cairo::FontSlant::Normal,
                cairo::FontWeight::Bold,
            );
            cr.set_font_size(9.0);
            cr.set_source_rgb(
                tokens::color::TEXT_MUTED.0,
                tokens::color::TEXT_MUTED.1,
                tokens::color::TEXT_MUTED.2,
            );
            if let Ok(ext) = cr.text_extents(dbg_label) {
                let _ = cr.move_to(notch_x + (notch_w - ext.width()) / 2.0, notch_y + 14.0);
                let _ = cr.show_text(dbg_label);
            }
        }
    }

    // 3. Board Header Banner Silkscreen - TEXT_PRIMARY title, TEXT_MUTED subtitle
    cr.select_font_face(
        tokens::font::CAIRO_SANS,
        cairo::FontSlant::Normal,
        cairo::FontWeight::Bold,
    );
    cr.set_font_size(16.0);
    cr.set_source_rgb(
        tokens::color::TEXT_PRIMARY.0,
        tokens::color::TEXT_PRIMARY.1,
        tokens::color::TEXT_PRIMARY.2,
    );
    let title_str = layout.title;
    let title_y = board_y + 46.0;
    if let Ok(ext) = cr.text_extents(title_str) {
        let _ = cr.move_to(board_x + (board_w - ext.width()) / 2.0, title_y);
        let _ = cr.show_text(title_str);
    }

    cr.select_font_face(
        tokens::font::CAIRO_MONO,
        cairo::FontSlant::Normal,
        cairo::FontWeight::Normal,
    );
    cr.set_font_size(10.5);
    cr.set_source_rgb(
        tokens::color::TEXT_MUTED.0,
        tokens::color::TEXT_MUTED.1,
        tokens::color::TEXT_MUTED.2,
    );
    let subtitle_str = layout.subtitle;
    let subtitle_y = title_y + 20.0;
    if let Ok(ext) = cr.text_extents(subtitle_str) {
        let _ = cr.move_to(board_x + (board_w - ext.width()) / 2.0, subtitle_y);
        let _ = cr.show_text(subtitle_str);
    }

    if !active_conflicts.is_empty() {
        let count = active_conflicts.len();
        let has_critical = active_conflicts
            .iter()
            .any(|(_, r)| r.severity == stakhal_core::nucleo_pinout::ReservedSeverity::Critical);
        let (cr_r, cr_g, cr_b) = if has_critical {
            tokens::color::STATE_ERROR
        } else {
            tokens::color::STATE_ACTIVE
        };

        let banner_str = if count == 1 {
            "! 1 pin conflict detected • see highlighted pins below".to_string()
        } else {
            format!(
                "! {} pin conflicts detected • see highlighted pins below",
                count
            )
        };

        cr.select_font_face(
            tokens::font::CAIRO_SANS,
            cairo::FontSlant::Normal,
            cairo::FontWeight::Bold,
        );
        cr.set_font_size(11.0);
        cr.set_source_rgb(cr_r, cr_g, cr_b);
        if let Ok(ext) = cr.text_extents(&banner_str) {
            let _ = cr.move_to(board_x + (board_w - ext.width()) / 2.0, subtitle_y + 18.0);
            let _ = cr.show_text(&banner_str);
        }
    }

    // 4. Center Component Graphics (MCU IC Chip, Buttons & LEDs)
    let center_left = col_cn6 + cell_w;
    let center_right = col_cn5;
    let center_avail_w = center_right - center_left;
    let mcu_w = (center_avail_w * 0.65).clamp(90.0, 180.0);
    let mcu_x = center_left + (center_avail_w - mcu_w) / 2.0;

    let mcu_y = row_start_y + 7.5 * row_h;
    let mcu_h = (4.5 * row_h).clamp(100.0, 180.0);

    // MCU LQFP64 Chip Frame - Sharp 0px corners, BG_VOID fill, 1px BORDER_HAIR
    cr.set_source_rgb(
        tokens::color::BG_VOID.0,
        tokens::color::BG_VOID.1,
        tokens::color::BG_VOID.2,
    );
    cr.rectangle(mcu_x, mcu_y, mcu_w, mcu_h);
    let _ = cr.fill_preserve();
    cr.set_source_rgb(
        tokens::color::BORDER_HAIR.0,
        tokens::color::BORDER_HAIR.1,
        tokens::color::BORDER_HAIR.2,
    );
    cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
    let _ = cr.stroke();

    // MCU Orientation Pin 1 Dot
    cr.set_source_rgb(
        tokens::color::TEXT_MUTED.0,
        tokens::color::TEXT_MUTED.1,
        tokens::color::TEXT_MUTED.2,
    );
    cr.arc(
        mcu_x + 14.0,
        mcu_y + 14.0,
        3.5,
        0.0,
        2.0 * std::f64::consts::PI,
    );
    let _ = cr.fill();

    // MCU Text Labels (Data -> JetBrains Mono)
    cr.select_font_face(
        tokens::font::CAIRO_MONO,
        cairo::FontSlant::Normal,
        cairo::FontWeight::Bold,
    );
    cr.set_font_size(11.0);
    cr.set_source_rgb(
        tokens::color::TEXT_PRIMARY.0,
        tokens::color::TEXT_PRIMARY.1,
        tokens::color::TEXT_PRIMARY.2,
    );
    if let Ok(ext) = cr.text_extents(layout.chip_label) {
        let _ = cr.move_to(mcu_x + (mcu_w - ext.width()) / 2.0, mcu_y + mcu_h * 0.42);
        let _ = cr.show_text(layout.chip_label);
    }
    cr.set_font_size(10.0);
    cr.set_source_rgb(
        tokens::color::TEXT_PRIMARY.0,
        tokens::color::TEXT_PRIMARY.1,
        tokens::color::TEXT_PRIMARY.2,
    );
    if let Ok(ext) = cr.text_extents(layout.chip_sublabel_1) {
        let _ = cr.move_to(mcu_x + (mcu_w - ext.width()) / 2.0, mcu_y + mcu_h * 0.55);
        let _ = cr.show_text(layout.chip_sublabel_1);
    }
    cr.set_font_size(9.0);
    cr.set_source_rgb(
        tokens::color::TEXT_MUTED.0,
        tokens::color::TEXT_MUTED.1,
        tokens::color::TEXT_MUTED.2,
    );
    if let Ok(ext) = cr.text_extents(layout.chip_sublabel_2) {
        let _ = cr.move_to(mcu_x + (mcu_w - ext.width()) / 2.0, mcu_y + mcu_h * 0.68);
        let _ = cr.show_text(layout.chip_sublabel_2);
    }

    // User LED Indicator Box - sharp 0px corners
    let is_led_active = (layout.is_led_active)(&highlights);
    let led_y = row_start_y + 2.0 * row_h;
    let led_h = (1.2 * row_h).clamp(30.0, 42.0);
    cr.set_source_rgb(
        tokens::color::BG_PANEL.0,
        tokens::color::BG_PANEL.1,
        tokens::color::BG_PANEL.2,
    );
    cr.rectangle(mcu_x, led_y, mcu_w, led_h);
    let _ = cr.fill_preserve();

    if is_led_active {
        cr.set_source_rgb(
            tokens::color::STATE_READY.0,
            tokens::color::STATE_READY.1,
            tokens::color::STATE_READY.2,
        );
        cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
        let _ = cr.stroke();

        cr.set_source_rgb(
            tokens::color::STATE_READY.0,
            tokens::color::STATE_READY.1,
            tokens::color::STATE_READY.2,
        );
        cr.arc(
            mcu_x + 18.0,
            led_y + led_h / 2.0,
            6.0,
            0.0,
            2.0 * std::f64::consts::PI,
        );
        let _ = cr.fill();

        cr.select_font_face(
            tokens::font::CAIRO_MONO,
            cairo::FontSlant::Normal,
            cairo::FontWeight::Bold,
        );
        cr.set_font_size(10.0);
        cr.set_source_rgb(
            tokens::color::STATE_READY.0,
            tokens::color::STATE_READY.1,
            tokens::color::STATE_READY.2,
        );
        let _ = cr.move_to(mcu_x + 32.0, led_y + led_h / 2.0 + 4.0);
        let on_text = format!("{} [ON]", layout.led_label);
        let _ = cr.show_text(&on_text);
    } else {
        cr.set_source_rgb(
            tokens::color::BORDER_HAIR.0,
            tokens::color::BORDER_HAIR.1,
            tokens::color::BORDER_HAIR.2,
        );
        cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
        let _ = cr.stroke();

        cr.set_source_rgb(
            tokens::color::BORDER_HAIR.0,
            tokens::color::BORDER_HAIR.1,
            tokens::color::BORDER_HAIR.2,
        );
        cr.arc(
            mcu_x + 18.0,
            led_y + led_h / 2.0,
            5.0,
            0.0,
            2.0 * std::f64::consts::PI,
        );
        let _ = cr.fill();

        cr.select_font_face(
            tokens::font::CAIRO_MONO,
            cairo::FontSlant::Normal,
            cairo::FontWeight::Normal,
        );
        cr.set_font_size(9.5);
        cr.set_source_rgb(
            tokens::color::TEXT_MUTED.0,
            tokens::color::TEXT_MUTED.1,
            tokens::color::TEXT_MUTED.2,
        );
        let _ = cr.move_to(mcu_x + 32.0, led_y + led_h / 2.0 + 4.0);
        let off_text = format!("{} ({})", layout.led_label, layout.led_pin);
        let _ = cr.show_text(&off_text);
    }

    // User Button (B1 USER) & Reset Button (B2 RESET) - Sharp 0px corners, 1px border
    let btn_y = row_start_y + 13.0 * row_h;
    let btn_h = (1.0 * row_h).clamp(26.0, 34.0);
    let b1_w = (mcu_w - 6.0) / 2.0;

    // B1 Button (PC13)
    cr.set_source_rgb(
        tokens::color::BG_PANEL.0,
        tokens::color::BG_PANEL.1,
        tokens::color::BG_PANEL.2,
    );
    cr.rectangle(mcu_x, btn_y, b1_w, btn_h);
    let _ = cr.fill_preserve();
    cr.set_source_rgb(
        tokens::color::BORDER_HAIR.0,
        tokens::color::BORDER_HAIR.1,
        tokens::color::BORDER_HAIR.2,
    );
    cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
    let _ = cr.stroke();
    cr.select_font_face(
        tokens::font::CAIRO_SANS,
        cairo::FontSlant::Normal,
        cairo::FontWeight::Bold,
    );
    cr.set_font_size(9.0);
    cr.set_source_rgb(
        tokens::color::TEXT_PRIMARY.0,
        tokens::color::TEXT_PRIMARY.1,
        tokens::color::TEXT_PRIMARY.2,
    );
    let _ = cr.move_to(mcu_x + 8.0, btn_y + btn_h / 2.0 + 3.0);
    let _ = cr.show_text("B1 USER");

    // B2 Button (RESET)
    cr.set_source_rgb(
        tokens::color::BG_PANEL.0,
        tokens::color::BG_PANEL.1,
        tokens::color::BG_PANEL.2,
    );
    cr.rectangle(mcu_x + b1_w + 6.0, btn_y, b1_w, btn_h);
    let _ = cr.fill_preserve();
    cr.set_source_rgb(
        tokens::color::BORDER_HAIR.0,
        tokens::color::BORDER_HAIR.1,
        tokens::color::BORDER_HAIR.2,
    );
    cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
    let _ = cr.stroke();
    cr.select_font_face(
        tokens::font::CAIRO_SANS,
        cairo::FontSlant::Normal,
        cairo::FontWeight::Bold,
    );
    cr.set_font_size(9.0);
    cr.set_source_rgb(
        tokens::color::TEXT_PRIMARY.0,
        tokens::color::TEXT_PRIMARY.1,
        tokens::color::TEXT_PRIMARY.2,
    );
    let _ = cr.move_to(mcu_x + b1_w + 10.0, btn_y + btn_h / 2.0 + 3.0);
    let _ = cr.show_text("B2 RESET");

    // 5. Draw Connector Header Titles & Active Counts with Text Tags [MORPHO] / [ARDUINO]
    for conn in layout.connectors {
        let is_morpho = conn.name == "CN7" || conn.name == "CN10";
        let top_pin_idx = match conn.name {
            "CN5" => 9,
            "CN9" => 7,
            _ => 0,
        };

        let (cell_x, cell_y, cell_w, _cell_h) =
            get_pin_cell_rect(conn.name, top_pin_idx, canvas_w, canvas_h);
        let hy = cell_y - 8.0;

        let type_title = match conn.name {
            "CN7" => "Morpho Left",
            "CN6" => "Power",
            "CN8" => "Analog In",
            "CN10" => "Morpho Right",
            "CN5" => "Digital High",
            "CN9" => "Digital Low",
            _ => "",
        };

        let active_count = conn
            .pins
            .iter()
            .filter(|p| highlights.contains_key(&(conn.name, p.pin_num)))
            .count();

        let tag = if is_morpho { "[MORPHO]" } else { "[ARDUINO]" };
        let header_text = format!(
            "{} {} • {} ({} active)",
            conn.name, tag, type_title, active_count
        );

        cr.select_font_face(
            tokens::font::CAIRO_MONO,
            cairo::FontSlant::Normal,
            cairo::FontWeight::Bold,
        );
        cr.set_font_size(10.5);
        cr.set_source_rgb(
            tokens::color::TEXT_PRIMARY.0,
            tokens::color::TEXT_PRIMARY.1,
            tokens::color::TEXT_PRIMARY.2,
        );

        if conn.name == "CN10" {
            let morpho_right_edge = col_cn10_1 + cell_w;
            if let Ok(ext) = cr.text_extents(&header_text) {
                let hx = morpho_right_edge - ext.width();
                let _ = cr.move_to(hx, hy);
                let _ = cr.show_text(&header_text);
            }
        } else {
            let _ = cr.move_to(cell_x, hy);
            let _ = cr.show_text(&header_text);
        }
    }

    // 6. Draw Pin Cells for All Connectors - BG_PANEL surface, STATE_READY for active, ACCENT for hover/selection
    for conn in layout.connectors {
        for (idx, p) in conn.pins.iter().enumerate() {
            let (cell_x, cell_y, cell_w, cell_h) =
                get_pin_cell_rect(conn.name, idx, canvas_w, canvas_h);

            let is_hl = highlights.get(&(conn.name, p.pin_num));
            let is_hovered = hovered_pin
                .is_some_and(|(c_name, p_num)| c_name == conn.name && *p_num == p.pin_num);

            if let Some(hl_info) = is_hl {
                let reserved_info = stakhal_core::board::check_reserved(board_id, p.mcu_pin)
                    .filter(|_| is_reserved_pin_conflict(p.mcu_pin, &hl_info.signal));
                let (hl_r, hl_g, hl_b) = match reserved_info {
                    Some(res) => match res.severity {
                        stakhal_core::nucleo_pinout::ReservedSeverity::Critical => {
                            tokens::color::STATE_ERROR
                        }
                        stakhal_core::nucleo_pinout::ReservedSeverity::Caution => {
                            tokens::color::STATE_ACTIVE
                        }
                    },
                    None => tokens::color::STATE_READY,
                };

                // Highlighted Pin (Active in loaded project) - background BG_PANEL
                cr.set_source_rgb(
                    tokens::color::BG_PANEL.0,
                    tokens::color::BG_PANEL.1,
                    tokens::color::BG_PANEL.2,
                );
                cr.rectangle(cell_x, cell_y, cell_w, cell_h);
                let _ = cr.fill_preserve();

                // Interactive/Selection ONLY uses ACCENT
                if is_hovered {
                    cr.set_source_rgb(
                        tokens::color::ACCENT.0,
                        tokens::color::ACCENT.1,
                        tokens::color::ACCENT.2,
                    );
                } else if hl_info.is_muted {
                    cr.set_source_rgba(hl_r, hl_g, hl_b, 0.35);
                } else {
                    cr.set_source_rgb(hl_r, hl_g, hl_b);
                }
                cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
                let _ = cr.stroke();

                // Pin Number Badge Box - sharp 0px corners
                let badge_w = (cell_w * 0.16).clamp(24.0, 32.0);
                if hl_info.is_muted {
                    cr.set_source_rgb(
                        tokens::color::BG_VOID.0,
                        tokens::color::BG_VOID.1,
                        tokens::color::BG_VOID.2,
                    );
                    cr.rectangle(cell_x + 3.0, cell_y + 3.0, badge_w, cell_h - 6.0);
                    let _ = cr.fill_preserve();
                    cr.set_source_rgba(hl_r, hl_g, hl_b, 0.35);
                    cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
                    let _ = cr.stroke();

                    cr.select_font_face(
                        tokens::font::CAIRO_MONO,
                        cairo::FontSlant::Normal,
                        cairo::FontWeight::Normal,
                    );
                    cr.set_font_size(9.5);
                    cr.set_source_rgb(
                        tokens::color::TEXT_MUTED.0,
                        tokens::color::TEXT_MUTED.1,
                        tokens::color::TEXT_MUTED.2,
                    );
                } else {
                    cr.set_source_rgb(hl_r, hl_g, hl_b);
                    cr.rectangle(cell_x + 3.0, cell_y + 3.0, badge_w, cell_h - 6.0);
                    let _ = cr.fill();

                    cr.select_font_face(
                        tokens::font::CAIRO_MONO,
                        cairo::FontSlant::Normal,
                        cairo::FontWeight::Bold,
                    );
                    cr.set_font_size(9.5);
                    cr.set_source_rgb(
                        tokens::color::BG_VOID.0,
                        tokens::color::BG_VOID.1,
                        tokens::color::BG_VOID.2,
                    );
                }

                let pnum_str = format!("{}", p.pin_num);
                if let Ok(ext) = cr.text_extents(&pnum_str) {
                    let tx = cell_x + 3.0 + (badge_w - ext.width()) / 2.0;
                    let _ = cr.move_to(tx, cell_y + cell_h * 0.64);
                    let _ = cr.show_text(&pnum_str);
                }

                // MCU Pin Text
                cr.set_font_size(9.5);
                if hl_info.is_muted {
                    cr.set_source_rgb(
                        tokens::color::TEXT_MUTED.0,
                        tokens::color::TEXT_MUTED.1,
                        tokens::color::TEXT_MUTED.2,
                    );
                } else {
                    cr.set_source_rgb(
                        tokens::color::TEXT_PRIMARY.0,
                        tokens::color::TEXT_PRIMARY.1,
                        tokens::color::TEXT_PRIMARY.2,
                    );
                }
                let mcu_pin_x = cell_x + badge_w + 8.0;
                let _ = cr.move_to(mcu_pin_x, cell_y + cell_h * 0.64);
                let _ = cr.show_text(p.mcu_pin);

                let mcu_ext_w = cr
                    .text_extents(p.mcu_pin)
                    .map(|e| e.width())
                    .unwrap_or(24.0);

                // Primary Text: #define Label if present, else Signal Name
                let primary_text = match &hl_info.label {
                    Some(lbl) => {
                        if lbl.ends_with("_Pin") {
                            lbl.clone()
                        } else {
                            format!("{}_Pin", lbl)
                        }
                    }
                    None => hl_info.signal.clone(),
                };

                if hl_info.is_muted {
                    cr.select_font_face(
                        tokens::font::CAIRO_MONO,
                        cairo::FontSlant::Normal,
                        cairo::FontWeight::Normal,
                    );
                    cr.set_font_size(9.0);
                    cr.set_source_rgb(
                        tokens::color::TEXT_MUTED.0,
                        tokens::color::TEXT_MUTED.1,
                        tokens::color::TEXT_MUTED.2,
                    );
                } else {
                    cr.select_font_face(
                        tokens::font::CAIRO_MONO,
                        cairo::FontSlant::Normal,
                        cairo::FontWeight::Bold,
                    );
                    cr.set_font_size(9.0);
                    cr.set_source_rgb(hl_r, hl_g, hl_b);
                }
                if let Ok(ext) = cr.text_extents(&primary_text) {
                    let min_x = mcu_pin_x + mcu_ext_w + 6.0;
                    let right_x = cell_x + cell_w - ext.width() - 6.0;
                    let text_x = right_x.max(min_x);
                    let _ = cr.move_to(text_x, cell_y + cell_h * 0.64);
                    let _ = cr.show_text(&primary_text);
                }
            } else {
                // Neutral / Unused Pin - background BG_PANEL, border BORDER_HAIR
                cr.set_source_rgb(
                    tokens::color::BG_PANEL.0,
                    tokens::color::BG_PANEL.1,
                    tokens::color::BG_PANEL.2,
                );
                cr.rectangle(cell_x, cell_y, cell_w, cell_h);
                let _ = cr.fill_preserve();

                // Interactive/Selection ONLY uses ACCENT
                if is_hovered {
                    cr.set_source_rgb(
                        tokens::color::ACCENT.0,
                        tokens::color::ACCENT.1,
                        tokens::color::ACCENT.2,
                    );
                } else {
                    cr.set_source_rgb(
                        tokens::color::BORDER_HAIR.0,
                        tokens::color::BORDER_HAIR.1,
                        tokens::color::BORDER_HAIR.2,
                    );
                }
                cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
                let _ = cr.stroke();

                // Pin Number Box for Neutral Cell - sharp 0px corners
                let badge_w = (cell_w * 0.16).clamp(26.0, 34.0);
                cr.set_source_rgb(
                    tokens::color::BG_VOID.0,
                    tokens::color::BG_VOID.1,
                    tokens::color::BG_VOID.2,
                );
                cr.rectangle(cell_x + 4.0, cell_y + 4.0, badge_w, cell_h - 8.0);
                let _ = cr.fill();

                cr.select_font_face(
                    tokens::font::CAIRO_MONO,
                    cairo::FontSlant::Normal,
                    cairo::FontWeight::Normal,
                );
                cr.set_font_size(9.5);
                if is_hovered {
                    cr.set_source_rgb(
                        tokens::color::TEXT_PRIMARY.0,
                        tokens::color::TEXT_PRIMARY.1,
                        tokens::color::TEXT_PRIMARY.2,
                    );
                } else {
                    cr.set_source_rgb(
                        tokens::color::TEXT_MUTED.0,
                        tokens::color::TEXT_MUTED.1,
                        tokens::color::TEXT_MUTED.2,
                    );
                }
                let pnum_str = format!("{}", p.pin_num);
                if let Ok(ext) = cr.text_extents(&pnum_str) {
                    let tx = cell_x + 4.0 + (badge_w - ext.width()) / 2.0;
                    let _ = cr.move_to(tx, cell_y + cell_h * 0.60);
                    let _ = cr.show_text(&pnum_str);
                }

                // MCU Pin & Label Text
                cr.set_source_rgb(
                    tokens::color::TEXT_PRIMARY.0,
                    tokens::color::TEXT_PRIMARY.1,
                    tokens::color::TEXT_PRIMARY.2,
                );
                let label_part = match p.default_label {
                    Some(lbl) => format!(" ({})", lbl),
                    None => "".to_string(),
                };
                let left_str = format!("{}{}", p.mcu_pin, label_part);
                let text_start_x = cell_x + badge_w + 10.0;
                let _ = cr.move_to(text_start_x, cell_y + cell_h * 0.60);
                let _ = cr.show_text(&left_str);
            }
        }
    }

    // 7. Footer Legend Bar inside Board Outline
    let legend_y = board_y + board_h - 28.0;
    cr.select_font_face(
        tokens::font::CAIRO_SANS,
        cairo::FontSlant::Normal,
        cairo::FontWeight::Bold,
    );
    cr.set_font_size(10.0);

    let leg_x = board_x + (board_w * 0.02).max(16.0);

    // Active Signal Legend
    cr.set_source_rgb(
        tokens::color::STATE_READY.0,
        tokens::color::STATE_READY.1,
        tokens::color::STATE_READY.2,
    );
    cr.rectangle(leg_x, legend_y, 14.0, 14.0);
    let _ = cr.fill();

    cr.set_source_rgb(
        tokens::color::TEXT_PRIMARY.0,
        tokens::color::TEXT_PRIMARY.1,
        tokens::color::TEXT_PRIMARY.2,
    );
    let _ = cr.move_to(leg_x + 22.0, legend_y + 11.0);
    let active_label = if is_filtering_module {
        "Active in Module"
    } else {
        "Active Signal in Loaded Project"
    };
    let _ = cr.show_text(active_label);

    if is_filtering_module {
        let leg_x2 = leg_x + 150.0;
        cr.set_source_rgb(
            tokens::color::BG_PANEL.0,
            tokens::color::BG_PANEL.1,
            tokens::color::BG_PANEL.2,
        );
        cr.rectangle(leg_x2, legend_y, 14.0, 14.0);
        let _ = cr.fill_preserve();
        cr.set_source_rgba(
            tokens::color::STATE_READY.0,
            tokens::color::STATE_READY.1,
            tokens::color::STATE_READY.2,
            0.35,
        );
        cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
        let _ = cr.stroke();

        cr.set_source_rgb(
            tokens::color::TEXT_MUTED.0,
            tokens::color::TEXT_MUTED.1,
            tokens::color::TEXT_MUTED.2,
        );
        let _ = cr.move_to(leg_x2 + 22.0, legend_y + 11.0);
        let _ = cr.show_text("Active Elsewhere in Project");
    }

    // 8. FINAL PASS: Compact Floating Tooltip Card
    draw_pinout_tooltip_card(
        cr,
        hovered_pin,
        hovered_mouse,
        &highlights,
        layout,
        board_id,
        (board_x, board_y, board_w, board_h),
    );
}

pub fn draw_blackpill_f411_board(
    cr: &cairo::Context,
    width: f64,
    height: f64,
    state: &Rc<RefCell<AppState>>,
) {
    let (hovered_pin, hovered_mouse, is_filtering_module) = state.borrow().with_canvas_state(|c| {
        (
            c.hovered_pinout_pin.clone(),
            c.hovered_pinout_mouse,
            c.selected_pinout_module.is_some(),
        )
    });
    let highlights = get_active_pin_highlights(&state.borrow());
    let hovered_pin = hovered_pin.as_ref();

    let board_id = stakhal_core::board::BoardId::BlackPillF411ce;
    let layout = BoardLayout::for_board(board_id);

    let active_conflicts: Vec<(&'static str, &stakhal_core::nucleo_pinout::ReservedPin)> = {
        let mut list = Vec::new();
        let proj = Rc::clone(&state.borrow().project);
        let proj_guard = proj.borrow();
        if let Some(project) = &proj_guard.loaded_project {
            for pin_cfg in &project.pins {
                if let Some(res) = stakhal_core::board::check_reserved(board_id, &pin_cfg.pin) {
                    if is_reserved_pin_conflict(&pin_cfg.pin, &pin_cfg.signal)
                        && !list.iter().any(|(p, _)| *p == res.mcu_pin)
                    {
                        list.push((res.mcu_pin, res));
                    }
                }
            }
        }
        list
    };

    let canvas_w = width.max(800.0);
    let canvas_h = height.max(600.0);

    let margin_x = (canvas_w * 0.025).max(16.0);
    let margin_y = (canvas_h * 0.035).max(20.0);

    let board_x = margin_x;
    let board_y = margin_y;
    let board_w = canvas_w - 2.0 * margin_x;
    let board_h = canvas_h - 2.0 * margin_y;

    let pad_x = 32.0;
    let cell_w = ((board_w - 2.0 * pad_x - 220.0) / 2.0).clamp(160.0, 260.0);

    let row_start_y = board_y + 105.0;
    let avail_h = board_h - 150.0;
    let row_h = (avail_h / 20.0).clamp(20.0, 38.0);

    let col_left = board_x + pad_x;
    let col_right = board_x + board_w - pad_x - cell_w;

    // Canvas Background
    cr.set_source_rgb(
        tokens::color::BG_VOID.0,
        tokens::color::BG_VOID.1,
        tokens::color::BG_VOID.2,
    );
    cr.rectangle(0.0, 0.0, canvas_w, canvas_h);
    let _ = cr.fill();

    // 1. Board Silhouette (PCB Outline) - Surface BG_PANEL, hairline border BORDER_HAIR
    cr.set_source_rgb(
        tokens::color::BG_PANEL.0,
        tokens::color::BG_PANEL.1,
        tokens::color::BG_PANEL.2,
    );
    cr.rectangle(board_x, board_y, board_w, board_h);
    let _ = cr.fill_preserve();
    cr.set_source_rgb(
        tokens::color::BORDER_HAIR.0,
        tokens::color::BORDER_HAIR.1,
        tokens::color::BORDER_HAIR.2,
    );
    cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
    let _ = cr.stroke();

    // Subtle PCB Grid Accent Texture Lines
    cr.set_source_rgba(
        tokens::color::BORDER_HAIR.0,
        tokens::color::BORDER_HAIR.1,
        tokens::color::BORDER_HAIR.2,
        0.25,
    );
    cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
    let mut gy = board_y + 40.0;
    while gy < board_y + board_h {
        let _ = cr.move_to(board_x + 10.0, gy);
        let _ = cr.line_to(board_x + board_w - 10.0, gy);
        let _ = cr.stroke();
        gy += 40.0;
    }

    // 2. USB-C Receptacle Notch on Top Edge
    let usb_w = 120.0;
    let usb_h = 24.0;
    let usb_x = board_x + (board_w - usb_w) / 2.0;
    let usb_y = board_y + 8.0;
    cr.set_source_rgb(
        tokens::color::BG_VOID.0,
        tokens::color::BG_VOID.1,
        tokens::color::BG_VOID.2,
    );
    cr.rectangle(usb_x, usb_y, usb_w, usb_h);
    let _ = cr.fill_preserve();
    cr.set_source_rgb(
        tokens::color::BORDER_HAIR.0,
        tokens::color::BORDER_HAIR.1,
        tokens::color::BORDER_HAIR.2,
    );
    cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
    let _ = cr.stroke();

    cr.select_font_face(
        tokens::font::CAIRO_MONO,
        cairo::FontSlant::Normal,
        cairo::FontWeight::Bold,
    );
    cr.set_font_size(9.5);
    cr.set_source_rgb(
        tokens::color::TEXT_MUTED.0,
        tokens::color::TEXT_MUTED.1,
        tokens::color::TEXT_MUTED.2,
    );
    if let Ok(ext) = cr.text_extents("USB-C (DFU / PWR)") {
        let _ = cr.move_to(usb_x + (usb_w - ext.width()) / 2.0, usb_y + 16.0);
        let _ = cr.show_text("USB-C (DFU / PWR)");
    }

    // 3. Board Header Banner Silkscreen - TEXT_PRIMARY title, TEXT_MUTED subtitle
    cr.select_font_face(
        tokens::font::CAIRO_SANS,
        cairo::FontSlant::Normal,
        cairo::FontWeight::Bold,
    );
    cr.set_font_size(16.0);
    cr.set_source_rgb(
        tokens::color::TEXT_PRIMARY.0,
        tokens::color::TEXT_PRIMARY.1,
        tokens::color::TEXT_PRIMARY.2,
    );
    let title_str = layout.title;
    let title_y = board_y + 54.0;
    if let Ok(ext) = cr.text_extents(title_str) {
        let _ = cr.move_to(board_x + (board_w - ext.width()) / 2.0, title_y);
        let _ = cr.show_text(title_str);
    }

    cr.select_font_face(
        tokens::font::CAIRO_MONO,
        cairo::FontSlant::Normal,
        cairo::FontWeight::Normal,
    );
    cr.set_font_size(10.5);
    cr.set_source_rgb(
        tokens::color::TEXT_MUTED.0,
        tokens::color::TEXT_MUTED.1,
        tokens::color::TEXT_MUTED.2,
    );
    let subtitle_str = layout.subtitle;
    let subtitle_y = title_y + 20.0;
    if let Ok(ext) = cr.text_extents(subtitle_str) {
        let _ = cr.move_to(board_x + (board_w - ext.width()) / 2.0, subtitle_y);
        let _ = cr.show_text(subtitle_str);
    }

    // Conflict Banner if any
    if !active_conflicts.is_empty() {
        let count = active_conflicts.len();
        let has_critical = active_conflicts
            .iter()
            .any(|(_, r)| r.severity == stakhal_core::nucleo_pinout::ReservedSeverity::Critical);
        let (cr_r, cr_g, cr_b) = if has_critical {
            tokens::color::STATE_ERROR
        } else {
            tokens::color::STATE_ACTIVE
        };

        let banner_str = if count == 1 {
            "! 1 pin conflict detected • see highlighted pins below".to_string()
        } else {
            format!(
                "! {} pin conflicts detected • see highlighted pins below",
                count
            )
        };

        cr.select_font_face(
            tokens::font::CAIRO_SANS,
            cairo::FontSlant::Normal,
            cairo::FontWeight::Bold,
        );
        cr.set_font_size(11.0);
        cr.set_source_rgb(cr_r, cr_g, cr_b);
        if let Ok(ext) = cr.text_extents(&banner_str) {
            let _ = cr.move_to(board_x + (board_w - ext.width()) / 2.0, subtitle_y + 18.0);
            let _ = cr.show_text(&banner_str);
        }
    }

    // 4. Center Area Components: HSE Crystal, MCU Chip, LSE Crystal, Buttons & LED
    let center_left = col_left + cell_w;
    let center_right = col_right;
    let center_w = center_right - center_left;
    let center_x = center_left + center_w / 2.0;

    // A. HSE 25.000 MHz Crystal Box (above MCU)
    let hse_w = (center_w * 0.40).clamp(110.0, 160.0);
    let hse_h = 24.0;
    let hse_x = center_x - hse_w / 2.0;
    let hse_y = row_start_y + 1.2 * row_h;
    cr.set_source_rgb(
        tokens::color::BG_PANEL.0,
        tokens::color::BG_PANEL.1,
        tokens::color::BG_PANEL.2,
    );
    cr.rectangle(hse_x, hse_y, hse_w, hse_h);
    let _ = cr.fill_preserve();
    cr.set_source_rgb(
        tokens::color::BORDER_HAIR.0,
        tokens::color::BORDER_HAIR.1,
        tokens::color::BORDER_HAIR.2,
    );
    cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
    let _ = cr.stroke();

    cr.select_font_face(
        tokens::font::CAIRO_MONO,
        cairo::FontSlant::Normal,
        cairo::FontWeight::Normal,
    );
    cr.set_font_size(9.0);
    cr.set_source_rgb(
        tokens::color::TEXT_MUTED.0,
        tokens::color::TEXT_MUTED.1,
        tokens::color::TEXT_MUTED.2,
    );
    if let Ok(ext) = cr.text_extents("HSE 25.000 MHz") {
        let _ = cr.move_to(hse_x + (hse_w - ext.width()) / 2.0, hse_y + 16.0);
        let _ = cr.show_text("HSE 25.000 MHz");
    }

    // B. STM32F411 MCU IC Package (UFQFPN48)
    let mcu_w = (center_w * 0.55).clamp(110.0, 170.0);
    let mcu_h = mcu_w;
    let mcu_x = center_x - mcu_w / 2.0;
    let mcu_y = row_start_y + 4.5 * row_h;

    cr.set_source_rgb(
        tokens::color::BG_VOID.0,
        tokens::color::BG_VOID.1,
        tokens::color::BG_VOID.2,
    );
    cr.rectangle(mcu_x, mcu_y, mcu_w, mcu_h);
    let _ = cr.fill_preserve();
    cr.set_source_rgb(
        tokens::color::BORDER_HAIR.0,
        tokens::color::BORDER_HAIR.1,
        tokens::color::BORDER_HAIR.2,
    );
    cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
    let _ = cr.stroke();

    // Orientation dot
    cr.set_source_rgb(
        tokens::color::TEXT_MUTED.0,
        tokens::color::TEXT_MUTED.1,
        tokens::color::TEXT_MUTED.2,
    );
    cr.arc(
        mcu_x + 12.0,
        mcu_y + 12.0,
        3.0,
        0.0,
        2.0 * std::f64::consts::PI,
    );
    let _ = cr.fill();

    cr.select_font_face(
        tokens::font::CAIRO_MONO,
        cairo::FontSlant::Normal,
        cairo::FontWeight::Bold,
    );
    cr.set_font_size(11.0);
    cr.set_source_rgb(
        tokens::color::TEXT_PRIMARY.0,
        tokens::color::TEXT_PRIMARY.1,
        tokens::color::TEXT_PRIMARY.2,
    );
    if let Ok(ext) = cr.text_extents(layout.chip_label) {
        let _ = cr.move_to(mcu_x + (mcu_w - ext.width()) / 2.0, mcu_y + mcu_h * 0.40);
        let _ = cr.show_text(layout.chip_label);
    }
    cr.set_font_size(10.0);
    if let Ok(ext) = cr.text_extents(layout.chip_sublabel_1) {
        let _ = cr.move_to(mcu_x + (mcu_w - ext.width()) / 2.0, mcu_y + mcu_h * 0.55);
        let _ = cr.show_text(layout.chip_sublabel_1);
    }
    cr.set_font_size(9.0);
    cr.set_source_rgb(
        tokens::color::TEXT_MUTED.0,
        tokens::color::TEXT_MUTED.1,
        tokens::color::TEXT_MUTED.2,
    );
    if let Ok(ext) = cr.text_extents(layout.chip_sublabel_2) {
        let _ = cr.move_to(mcu_x + (mcu_w - ext.width()) / 2.0, mcu_y + mcu_h * 0.70);
        let _ = cr.show_text(layout.chip_sublabel_2);
    }

    // C. LSE 32.768 kHz Crystal Box (below MCU)
    let lse_w = (center_w * 0.40).clamp(110.0, 160.0);
    let lse_h = 24.0;
    let lse_x = center_x - lse_w / 2.0;
    let lse_y = mcu_y + mcu_h + 12.0;
    cr.set_source_rgb(
        tokens::color::BG_PANEL.0,
        tokens::color::BG_PANEL.1,
        tokens::color::BG_PANEL.2,
    );
    cr.rectangle(lse_x, lse_y, lse_w, lse_h);
    let _ = cr.fill_preserve();
    cr.set_source_rgb(
        tokens::color::BORDER_HAIR.0,
        tokens::color::BORDER_HAIR.1,
        tokens::color::BORDER_HAIR.2,
    );
    cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
    let _ = cr.stroke();

    cr.select_font_face(
        tokens::font::CAIRO_MONO,
        cairo::FontSlant::Normal,
        cairo::FontWeight::Normal,
    );
    cr.set_font_size(9.0);
    cr.set_source_rgb(
        tokens::color::TEXT_MUTED.0,
        tokens::color::TEXT_MUTED.1,
        tokens::color::TEXT_MUTED.2,
    );
    if let Ok(ext) = cr.text_extents("LSE 32.768 kHz") {
        let _ = cr.move_to(lse_x + (lse_w - ext.width()) / 2.0, lse_y + 16.0);
        let _ = cr.show_text("LSE 32.768 kHz");
    }

    // D. Onboard LED (PC13 - active low)
    let is_led_active = (layout.is_led_active)(&highlights);
    let led_w = (center_w * 0.45).clamp(120.0, 180.0);
    let led_h = 28.0;
    let led_x = center_x - led_w / 2.0;
    let led_y = lse_y + lse_h + 10.0;
    cr.set_source_rgb(
        tokens::color::BG_PANEL.0,
        tokens::color::BG_PANEL.1,
        tokens::color::BG_PANEL.2,
    );
    cr.rectangle(led_x, led_y, led_w, led_h);
    let _ = cr.fill_preserve();

    if is_led_active {
        cr.set_source_rgb(
            tokens::color::STATE_READY.0,
            tokens::color::STATE_READY.1,
            tokens::color::STATE_READY.2,
        );
        cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
        let _ = cr.stroke();

        cr.set_source_rgb(
            tokens::color::STATE_READY.0,
            tokens::color::STATE_READY.1,
            tokens::color::STATE_READY.2,
        );
        cr.arc(
            led_x + 16.0,
            led_y + led_h / 2.0,
            5.0,
            0.0,
            2.0 * std::f64::consts::PI,
        );
        let _ = cr.fill();

        cr.select_font_face(
            tokens::font::CAIRO_MONO,
            cairo::FontSlant::Normal,
            cairo::FontWeight::Bold,
        );
        cr.set_font_size(9.5);
        cr.set_source_rgb(
            tokens::color::STATE_READY.0,
            tokens::color::STATE_READY.1,
            tokens::color::STATE_READY.2,
        );
        let _ = cr.move_to(led_x + 28.0, led_y + led_h / 2.0 + 3.5);
        let _ = cr.show_text("LED PC13 [ON]");
    } else {
        cr.set_source_rgb(
            tokens::color::BORDER_HAIR.0,
            tokens::color::BORDER_HAIR.1,
            tokens::color::BORDER_HAIR.2,
        );
        cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
        let _ = cr.stroke();

        cr.set_source_rgb(
            tokens::color::BORDER_HAIR.0,
            tokens::color::BORDER_HAIR.1,
            tokens::color::BORDER_HAIR.2,
        );
        cr.arc(
            led_x + 16.0,
            led_y + led_h / 2.0,
            4.0,
            0.0,
            2.0 * std::f64::consts::PI,
        );
        let _ = cr.fill();

        cr.select_font_face(
            tokens::font::CAIRO_MONO,
            cairo::FontSlant::Normal,
            cairo::FontWeight::Normal,
        );
        cr.set_font_size(9.0);
        cr.set_source_rgb(
            tokens::color::TEXT_MUTED.0,
            tokens::color::TEXT_MUTED.1,
            tokens::color::TEXT_MUTED.2,
        );
        let _ = cr.move_to(led_x + 28.0, led_y + led_h / 2.0 + 3.5);
        let _ = cr.show_text("LED (PC13)");
    }

    // E. Onboard Buttons: KEY (PA0), NRST, BOOT0
    let btn_row_y = led_y + led_h + 10.0;
    let btn_h = 24.0;
    let btn_w = (led_w - 8.0) / 3.0;

    // KEY button (PA0)
    let is_key_active = highlights.contains_key(&("Left", 15));
    cr.set_source_rgb(
        tokens::color::BG_PANEL.0,
        tokens::color::BG_PANEL.1,
        tokens::color::BG_PANEL.2,
    );
    cr.rectangle(led_x, btn_row_y, btn_w, btn_h);
    let _ = cr.fill_preserve();
    if is_key_active {
        cr.set_source_rgb(
            tokens::color::STATE_READY.0,
            tokens::color::STATE_READY.1,
            tokens::color::STATE_READY.2,
        );
    } else {
        cr.set_source_rgb(
            tokens::color::BORDER_HAIR.0,
            tokens::color::BORDER_HAIR.1,
            tokens::color::BORDER_HAIR.2,
        );
    }
    cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
    let _ = cr.stroke();

    cr.select_font_face(
        tokens::font::CAIRO_SANS,
        cairo::FontSlant::Normal,
        cairo::FontWeight::Bold,
    );
    cr.set_font_size(8.5);
    if is_key_active {
        cr.set_source_rgb(
            tokens::color::STATE_READY.0,
            tokens::color::STATE_READY.1,
            tokens::color::STATE_READY.2,
        );
    } else {
        cr.set_source_rgb(
            tokens::color::TEXT_PRIMARY.0,
            tokens::color::TEXT_PRIMARY.1,
            tokens::color::TEXT_PRIMARY.2,
        );
    }
    let _ = cr.move_to(led_x + 4.0, btn_row_y + btn_h / 2.0 + 3.0);
    let _ = cr.show_text("KEY PA0");

    // BOOT0 button
    let boot_x = led_x + btn_w + 4.0;
    cr.set_source_rgb(
        tokens::color::BG_PANEL.0,
        tokens::color::BG_PANEL.1,
        tokens::color::BG_PANEL.2,
    );
    cr.rectangle(boot_x, btn_row_y, btn_w, btn_h);
    let _ = cr.fill_preserve();
    cr.set_source_rgb(
        tokens::color::BORDER_HAIR.0,
        tokens::color::BORDER_HAIR.1,
        tokens::color::BORDER_HAIR.2,
    );
    cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
    let _ = cr.stroke();
    cr.set_source_rgb(
        tokens::color::TEXT_MUTED.0,
        tokens::color::TEXT_MUTED.1,
        tokens::color::TEXT_MUTED.2,
    );
    let _ = cr.move_to(boot_x + 4.0, btn_row_y + btn_h / 2.0 + 3.0);
    let _ = cr.show_text("BOOT0");

    // NRST button
    let nrst_x = boot_x + btn_w + 4.0;
    cr.set_source_rgb(
        tokens::color::BG_PANEL.0,
        tokens::color::BG_PANEL.1,
        tokens::color::BG_PANEL.2,
    );
    cr.rectangle(nrst_x, btn_row_y, btn_w, btn_h);
    let _ = cr.fill_preserve();
    cr.set_source_rgb(
        tokens::color::BORDER_HAIR.0,
        tokens::color::BORDER_HAIR.1,
        tokens::color::BORDER_HAIR.2,
    );
    cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
    let _ = cr.stroke();
    cr.set_source_rgb(
        tokens::color::TEXT_MUTED.0,
        tokens::color::TEXT_MUTED.1,
        tokens::color::TEXT_MUTED.2,
    );
    let _ = cr.move_to(nrst_x + 4.0, btn_row_y + btn_h / 2.0 + 3.0);
    let _ = cr.show_text("NRST");

    // 5. Draw Connector Header Titles & Active Counts
    for conn in layout.connectors {
        let (cell_x, cell_y, cell_w, _cell_h) = get_pin_cell_rect(conn.name, 0, canvas_w, canvas_h);
        let hy = cell_y - 8.0;

        let active_count = conn
            .pins
            .iter()
            .filter(|p| highlights.contains_key(&(conn.name, p.pin_num)))
            .count();

        let (tag, type_title) = match conn.name {
            "Left" => ("[P1]", "Left 20-Pin Edge Header"),
            "Right" => ("[P2]", "Right 20-Pin Edge Header"),
            "SWD" => ("[P3]", "SWD 4-Pin Debug Header"),
            _ => ("", ""),
        };

        let header_text = format!(
            "{} {} • {} ({} active)",
            conn.name, tag, type_title, active_count
        );

        cr.select_font_face(
            tokens::font::CAIRO_MONO,
            cairo::FontSlant::Normal,
            cairo::FontWeight::Bold,
        );
        cr.set_font_size(10.5);
        cr.set_source_rgb(
            tokens::color::TEXT_PRIMARY.0,
            tokens::color::TEXT_PRIMARY.1,
            tokens::color::TEXT_PRIMARY.2,
        );

        if conn.name == "Right" {
            let right_edge = col_right + cell_w;
            if let Ok(ext) = cr.text_extents(&header_text) {
                let hx = right_edge - ext.width();
                let _ = cr.move_to(hx, hy);
                let _ = cr.show_text(&header_text);
            }
        } else if conn.name == "SWD" {
            if let Ok(ext) = cr.text_extents(&header_text) {
                let hx = center_x - ext.width() / 2.0;
                let _ = cr.move_to(hx, hy);
                let _ = cr.show_text(&header_text);
            }
        } else {
            let _ = cr.move_to(cell_x, hy);
            let _ = cr.show_text(&header_text);
        }
    }

    // 6. Draw Pin Cells for All Connectors
    for conn in layout.connectors {
        for (idx, p) in conn.pins.iter().enumerate() {
            let (cell_x, cell_y, cell_w, cell_h) =
                get_pin_cell_rect(conn.name, idx, canvas_w, canvas_h);

            let is_hl = highlights.get(&(conn.name, p.pin_num));
            let is_hovered = hovered_pin
                .is_some_and(|(c_name, p_num)| c_name == conn.name && *p_num == p.pin_num);

            if let Some(hl_info) = is_hl {
                let reserved_info = stakhal_core::board::check_reserved(board_id, p.mcu_pin)
                    .filter(|_| is_reserved_pin_conflict(p.mcu_pin, &hl_info.signal));
                let (hl_r, hl_g, hl_b) = match reserved_info {
                    Some(res) => match res.severity {
                        stakhal_core::nucleo_pinout::ReservedSeverity::Critical => {
                            tokens::color::STATE_ERROR
                        }
                        stakhal_core::nucleo_pinout::ReservedSeverity::Caution => {
                            tokens::color::STATE_ACTIVE
                        }
                    },
                    None => tokens::color::STATE_READY,
                };

                cr.set_source_rgb(
                    tokens::color::BG_PANEL.0,
                    tokens::color::BG_PANEL.1,
                    tokens::color::BG_PANEL.2,
                );
                cr.rectangle(cell_x, cell_y, cell_w, cell_h);
                let _ = cr.fill_preserve();

                if is_hovered {
                    cr.set_source_rgb(
                        tokens::color::ACCENT.0,
                        tokens::color::ACCENT.1,
                        tokens::color::ACCENT.2,
                    );
                } else if hl_info.is_muted {
                    cr.set_source_rgba(hl_r, hl_g, hl_b, 0.35);
                } else {
                    cr.set_source_rgb(hl_r, hl_g, hl_b);
                }
                cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
                let _ = cr.stroke();

                let badge_w = (cell_w * 0.16).clamp(24.0, 32.0);
                if hl_info.is_muted {
                    cr.set_source_rgb(
                        tokens::color::BG_VOID.0,
                        tokens::color::BG_VOID.1,
                        tokens::color::BG_VOID.2,
                    );
                    cr.rectangle(cell_x + 3.0, cell_y + 3.0, badge_w, cell_h - 6.0);
                    let _ = cr.fill_preserve();
                    cr.set_source_rgba(hl_r, hl_g, hl_b, 0.35);
                    cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
                    let _ = cr.stroke();

                    cr.select_font_face(
                        tokens::font::CAIRO_MONO,
                        cairo::FontSlant::Normal,
                        cairo::FontWeight::Normal,
                    );
                    cr.set_font_size(9.5);
                    cr.set_source_rgb(
                        tokens::color::TEXT_MUTED.0,
                        tokens::color::TEXT_MUTED.1,
                        tokens::color::TEXT_MUTED.2,
                    );
                } else {
                    cr.set_source_rgb(hl_r, hl_g, hl_b);
                    cr.rectangle(cell_x + 3.0, cell_y + 3.0, badge_w, cell_h - 6.0);
                    let _ = cr.fill();

                    cr.select_font_face(
                        tokens::font::CAIRO_MONO,
                        cairo::FontSlant::Normal,
                        cairo::FontWeight::Bold,
                    );
                    cr.set_font_size(9.5);
                    cr.set_source_rgb(
                        tokens::color::BG_VOID.0,
                        tokens::color::BG_VOID.1,
                        tokens::color::BG_VOID.2,
                    );
                }

                let pnum_str = format!("{}", p.pin_num);
                if let Ok(ext) = cr.text_extents(&pnum_str) {
                    let tx = cell_x + 3.0 + (badge_w - ext.width()) / 2.0;
                    let _ = cr.move_to(tx, cell_y + cell_h * 0.64);
                    let _ = cr.show_text(&pnum_str);
                }

                if conn.name == "SWD" {
                    let swd_signal_text = if p.mcu_pin == "PA13" || hl_info.signal.contains("SWDIO")
                    {
                        "SWDIO".to_string()
                    } else if p.mcu_pin == "PA14" || hl_info.signal.contains("SWCLK") {
                        "SWCLK".to_string()
                    } else if let Some(lbl) = &hl_info.label {
                        lbl.clone()
                    } else {
                        hl_info.signal.clone()
                    };

                    let _ = cr.save();
                    cr.rectangle(cell_x, cell_y, cell_w, cell_h);
                    let _ = cr.clip();

                    cr.select_font_face(
                        tokens::font::CAIRO_MONO,
                        cairo::FontSlant::Normal,
                        if hl_info.is_muted {
                            cairo::FontWeight::Normal
                        } else {
                            cairo::FontWeight::Bold
                        },
                    );
                    cr.set_font_size(9.0);
                    if hl_info.is_muted {
                        cr.set_source_rgb(
                            tokens::color::TEXT_MUTED.0,
                            tokens::color::TEXT_MUTED.1,
                            tokens::color::TEXT_MUTED.2,
                        );
                    } else {
                        cr.set_source_rgb(hl_r, hl_g, hl_b);
                    }
                    let text_x = cell_x + badge_w + 6.0;
                    let _ = cr.move_to(text_x, cell_y + cell_h * 0.64);
                    let _ = cr.show_text(&swd_signal_text);
                    let _ = cr.restore();
                } else {
                    cr.set_font_size(9.5);
                    if hl_info.is_muted {
                        cr.set_source_rgb(
                            tokens::color::TEXT_MUTED.0,
                            tokens::color::TEXT_MUTED.1,
                            tokens::color::TEXT_MUTED.2,
                        );
                    } else {
                        cr.set_source_rgb(
                            tokens::color::TEXT_PRIMARY.0,
                            tokens::color::TEXT_PRIMARY.1,
                            tokens::color::TEXT_PRIMARY.2,
                        );
                    }
                    let mcu_pin_x = cell_x + badge_w + 8.0;
                    let _ = cr.move_to(mcu_pin_x, cell_y + cell_h * 0.64);
                    let _ = cr.show_text(p.mcu_pin);

                    let mcu_ext_w = cr
                        .text_extents(p.mcu_pin)
                        .map(|e| e.width())
                        .unwrap_or(24.0);

                    let primary_text = match &hl_info.label {
                        Some(lbl) => {
                            if lbl.ends_with("_Pin") {
                                lbl.clone()
                            } else {
                                format!("{}_Pin", lbl)
                            }
                        }
                        None => hl_info.signal.clone(),
                    };

                    if hl_info.is_muted {
                        cr.select_font_face(
                            tokens::font::CAIRO_MONO,
                            cairo::FontSlant::Normal,
                            cairo::FontWeight::Normal,
                        );
                        cr.set_font_size(9.0);
                        cr.set_source_rgb(
                            tokens::color::TEXT_MUTED.0,
                            tokens::color::TEXT_MUTED.1,
                            tokens::color::TEXT_MUTED.2,
                        );
                    } else {
                        cr.select_font_face(
                            tokens::font::CAIRO_MONO,
                            cairo::FontSlant::Normal,
                            cairo::FontWeight::Bold,
                        );
                        cr.set_font_size(9.0);
                        cr.set_source_rgb(hl_r, hl_g, hl_b);
                    }
                    if let Ok(ext) = cr.text_extents(&primary_text) {
                        let min_x = mcu_pin_x + mcu_ext_w + 6.0;
                        let right_x = cell_x + cell_w - ext.width() - 6.0;
                        let text_x = right_x.max(min_x);
                        let _ = cr.move_to(text_x, cell_y + cell_h * 0.64);
                        let _ = cr.show_text(&primary_text);
                    }
                }
            } else {
                cr.set_source_rgb(
                    tokens::color::BG_PANEL.0,
                    tokens::color::BG_PANEL.1,
                    tokens::color::BG_PANEL.2,
                );
                cr.rectangle(cell_x, cell_y, cell_w, cell_h);
                let _ = cr.fill_preserve();

                if is_hovered {
                    cr.set_source_rgb(
                        tokens::color::ACCENT.0,
                        tokens::color::ACCENT.1,
                        tokens::color::ACCENT.2,
                    );
                } else {
                    cr.set_source_rgb(
                        tokens::color::BORDER_HAIR.0,
                        tokens::color::BORDER_HAIR.1,
                        tokens::color::BORDER_HAIR.2,
                    );
                }
                cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
                let _ = cr.stroke();

                let badge_w = (cell_w * 0.16).clamp(24.0, 32.0);
                cr.set_source_rgb(
                    tokens::color::BG_VOID.0,
                    tokens::color::BG_VOID.1,
                    tokens::color::BG_VOID.2,
                );
                cr.rectangle(cell_x + 4.0, cell_y + 4.0, badge_w, cell_h - 8.0);
                let _ = cr.fill();

                cr.select_font_face(
                    tokens::font::CAIRO_MONO,
                    cairo::FontSlant::Normal,
                    cairo::FontWeight::Normal,
                );
                cr.set_font_size(9.5);
                if is_hovered {
                    cr.set_source_rgb(
                        tokens::color::TEXT_PRIMARY.0,
                        tokens::color::TEXT_PRIMARY.1,
                        tokens::color::TEXT_PRIMARY.2,
                    );
                } else {
                    cr.set_source_rgb(
                        tokens::color::TEXT_MUTED.0,
                        tokens::color::TEXT_MUTED.1,
                        tokens::color::TEXT_MUTED.2,
                    );
                }
                let pnum_str = format!("{}", p.pin_num);
                if let Ok(ext) = cr.text_extents(&pnum_str) {
                    let tx = cell_x + 4.0 + (badge_w - ext.width()) / 2.0;
                    let _ = cr.move_to(tx, cell_y + cell_h * 0.60);
                    let _ = cr.show_text(&pnum_str);
                }

                let left_str = if conn.name == "SWD" {
                    p.default_label.unwrap_or(p.mcu_pin).to_string()
                } else {
                    let label_part = match p.default_label {
                        Some(lbl) => format!(" ({})", lbl),
                        None => "".to_string(),
                    };
                    format!("{}{}", p.mcu_pin, label_part)
                };
                let text_start_x = cell_x + badge_w + 8.0;
                let _ = cr.save();
                cr.rectangle(cell_x, cell_y, cell_w, cell_h);
                let _ = cr.clip();
                let _ = cr.move_to(text_start_x, cell_y + cell_h * 0.60);
                let _ = cr.show_text(&left_str);
                let _ = cr.restore();
            }
        }
    }

    // 7. Footer Legend Bar inside Board Outline
    let legend_y = board_y + board_h - 28.0;
    cr.select_font_face(
        tokens::font::CAIRO_SANS,
        cairo::FontSlant::Normal,
        cairo::FontWeight::Bold,
    );
    cr.set_font_size(10.0);

    let leg_x = board_x + (board_w * 0.02).max(16.0);

    cr.set_source_rgb(
        tokens::color::STATE_READY.0,
        tokens::color::STATE_READY.1,
        tokens::color::STATE_READY.2,
    );
    cr.rectangle(leg_x, legend_y, 14.0, 14.0);
    let _ = cr.fill();

    cr.set_source_rgb(
        tokens::color::TEXT_PRIMARY.0,
        tokens::color::TEXT_PRIMARY.1,
        tokens::color::TEXT_PRIMARY.2,
    );
    let _ = cr.move_to(leg_x + 22.0, legend_y + 11.0);
    let active_label = if is_filtering_module {
        "Active in Module"
    } else {
        "Active Signal in Loaded Project"
    };
    let _ = cr.show_text(active_label);

    if is_filtering_module {
        let leg_x2 = leg_x + 150.0;
        cr.set_source_rgb(
            tokens::color::BG_PANEL.0,
            tokens::color::BG_PANEL.1,
            tokens::color::BG_PANEL.2,
        );
        cr.rectangle(leg_x2, legend_y, 14.0, 14.0);
        let _ = cr.fill_preserve();
        cr.set_source_rgba(
            tokens::color::STATE_READY.0,
            tokens::color::STATE_READY.1,
            tokens::color::STATE_READY.2,
            0.35,
        );
        cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
        let _ = cr.stroke();

        cr.set_source_rgb(
            tokens::color::TEXT_MUTED.0,
            tokens::color::TEXT_MUTED.1,
            tokens::color::TEXT_MUTED.2,
        );
        let _ = cr.move_to(leg_x2 + 22.0, legend_y + 11.0);
        let _ = cr.show_text("Active Elsewhere in Project");
    }

    // 8. Tooltip card
    draw_pinout_tooltip_card(
        cr,
        hovered_pin,
        hovered_mouse,
        &highlights,
        layout,
        board_id,
        (board_x, board_y, board_w, board_h),
    );
}

/// Computes the bounding rectangle of a connector header's title banner.
pub fn get_header_title_rect(
    conn_name: &str,
    canvas_w: f64,
    canvas_h: f64,
) -> (f64, f64, f64, f64) {
    let top_idx = match conn_name {
        "CN5" => 9,
        "CN9" => 7,
        _ => 0,
    };
    let (cell_x, cell_y, cell_w, _cell_h) =
        get_pin_cell_rect(conn_name, top_idx, canvas_w, canvas_h);
    let title_y = cell_y - 24.0;
    let title_h = 24.0;

    let (title_x, title_w) = match conn_name {
        "Right" | "CN10" => ((cell_x + cell_w - 260.0).max(0.0), 260.0),
        "SWD" => {
            let total_swd_w = cell_w * 4.0 + 12.0;
            (cell_x - 40.0, total_swd_w + 80.0)
        }
        _ => (cell_x, 260.0),
    };
    (title_x, title_y, title_w, title_h)
}

/// Computes the floating position of the pin hover tooltip card, ensuring it never covers
/// the hovered header's title.
pub fn compute_tooltip_card_rect(
    mx: f64,
    my: f64,
    tt_w: f64,
    tt_h: f64,
    conn_name: &str,
    board_bounds: (f64, f64, f64, f64),
) -> (f64, f64, f64, f64) {
    let (board_x, board_y, board_w, board_h) = board_bounds;
    let canvas_w = board_w + 2.0 * board_x;
    let canvas_h = board_h + 2.0 * board_y;
    let title_rect = get_header_title_rect(conn_name, canvas_w, canvas_h);

    let mut tt_x = mx + 12.0;
    let mut tt_y = my - (tt_h + 6.0);

    if tt_x + tt_w > board_x + board_w - 15.0 {
        tt_x = (mx - tt_w - 12.0).max(board_x + 15.0);
    }
    if tt_y < board_y + 15.0 {
        tt_y = my + 20.0;
    }
    if tt_y + tt_h > board_y + board_h - 15.0 {
        tt_y = (my - tt_h - 8.0).max(board_y + 15.0);
    }

    // Ensure tooltip never covers the hovered header's title
    let (tx, ty, tw, th) = title_rect;
    let overlaps_title = tt_x < tx + tw && tt_x + tt_w > tx && tt_y < ty + th && tt_y + tt_h > ty;
    if overlaps_title {
        let below_y = my + 20.0;
        if below_y + tt_h <= board_y + board_h - 15.0 {
            tt_y = below_y;
        } else if ty - (tt_h + 6.0) >= board_y + 15.0 {
            tt_y = ty - (tt_h + 6.0);
        } else if tt_x < tx + tw / 2.0 {
            tt_x = (tx - tt_w - 12.0).max(board_x + 15.0);
        } else {
            tt_x = (tx + tw + 12.0).min(board_x + board_w - tt_w - 15.0);
        }
    }

    (tt_x, tt_y, tt_w, tt_h)
}

fn draw_pinout_tooltip_card(
    cr: &cairo::Context,
    hovered_pin: Option<&(String, u8)>,
    hovered_mouse: Option<(f64, f64)>,
    highlights: &HashMap<(&'static str, u8), PinHighlightInfo>,
    layout: &BoardLayout,
    board_id: stakhal_core::board::BoardId,
    board_bounds: (f64, f64, f64, f64),
) {
    let (board_x, board_y, board_w, board_h) = board_bounds;
    if let (Some((conn_name, pin_num)), Some((mx, my))) = (hovered_pin, hovered_mouse) {
        if let Some(conn) = layout.connectors.iter().find(|c| c.name == conn_name) {
            if let Some(p) = conn.pins.iter().find(|p| p.pin_num == *pin_num) {
                let default_lbl_str = p
                    .default_label
                    .map(|l| format!(" ({})", l))
                    .unwrap_or_default();
                let line1 = format!(
                    "{}-{} : {}{}",
                    conn_name, pin_num, p.mcu_pin, default_lbl_str
                );

                let is_hl = highlights.get(&(conn_name.as_str(), *pin_num));
                let line2 = match is_hl {
                    Some(hl) => {
                        let mut parts = Vec::new();
                        parts.push(format!("Signal: {}", hl.signal));
                        if let Some(lbl) = &hl.label {
                            parts.push(format!("Label: {}", lbl));
                        }
                        if !hl.modules.is_empty() {
                            parts.push(format!("Module: {}", hl.modules.join(", ")));
                        }
                        parts.join(" • ")
                    }
                    None => "Unused in Project".to_string(),
                };

                let reserved_info = if let Some(hl) = is_hl {
                    stakhal_core::board::check_reserved(board_id, p.mcu_pin)
                        .filter(|_| is_reserved_pin_conflict(p.mcu_pin, &hl.signal))
                } else {
                    None
                };

                let line3 = reserved_info.map(|res| match res.severity {
                    stakhal_core::nucleo_pinout::ReservedSeverity::Critical => {
                        format!("WARNING: {}", res.reason)
                    }
                    stakhal_core::nucleo_pinout::ReservedSeverity::Caution => {
                        format!("CAUTION: {}", res.reason)
                    }
                });

                cr.select_font_face(
                    tokens::font::CAIRO_MONO,
                    cairo::FontSlant::Normal,
                    cairo::FontWeight::Bold,
                );
                cr.set_font_size(10.0);
                let w1 = cr.text_extents(&line1).map(|e| e.width()).unwrap_or(120.0);

                cr.select_font_face(
                    tokens::font::CAIRO_MONO,
                    cairo::FontSlant::Normal,
                    cairo::FontWeight::Bold,
                );
                cr.set_font_size(9.5);
                let w2 = cr.text_extents(&line2).map(|e| e.width()).unwrap_or(120.0);

                let w3 = line3
                    .as_ref()
                    .map(|l3| cr.text_extents(l3).map(|e| e.width()).unwrap_or(120.0))
                    .unwrap_or(0.0);

                let tt_w = (w1.max(w2).max(w3) + 24.0).max(160.0);
                let tt_h = if line3.is_some() { 58.0 } else { 42.0 };

                let (tt_x, tt_y, tt_w, tt_h) = compute_tooltip_card_rect(
                    mx,
                    my,
                    tt_w,
                    tt_h,
                    conn_name,
                    (board_x, board_y, board_w, board_h),
                );

                // Card Background BG_PANEL, sharp 0px corners
                cr.set_source_rgb(
                    tokens::color::BG_PANEL.0,
                    tokens::color::BG_PANEL.1,
                    tokens::color::BG_PANEL.2,
                );
                cr.rectangle(tt_x, tt_y, tt_w, tt_h);
                let _ = cr.fill_preserve();

                // Border color (1px hairline)
                let (border_r, border_g, border_b) = match reserved_info {
                    Some(res) => match res.severity {
                        stakhal_core::nucleo_pinout::ReservedSeverity::Critical => {
                            tokens::color::STATE_ERROR
                        }
                        stakhal_core::nucleo_pinout::ReservedSeverity::Caution => {
                            tokens::color::STATE_ACTIVE
                        }
                    },
                    None => {
                        if is_hl.is_some() {
                            tokens::color::STATE_READY
                        } else {
                            tokens::color::BORDER_HAIR
                        }
                    }
                };

                cr.set_source_rgb(border_r, border_g, border_b);
                cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
                let _ = cr.stroke();

                // Line 1: Header Info (TEXT_PRIMARY, JetBrains Mono)
                cr.select_font_face(
                    tokens::font::CAIRO_MONO,
                    cairo::FontSlant::Normal,
                    cairo::FontWeight::Bold,
                );
                cr.set_font_size(10.0);
                cr.set_source_rgb(
                    tokens::color::TEXT_PRIMARY.0,
                    tokens::color::TEXT_PRIMARY.1,
                    tokens::color::TEXT_PRIMARY.2,
                );
                let _ = cr.move_to(tt_x + 10.0, tt_y + 16.0);
                let _ = cr.show_text(&line1);

                // Line 2: Signal / Status Info
                cr.select_font_face(
                    tokens::font::CAIRO_MONO,
                    cairo::FontSlant::Normal,
                    cairo::FontWeight::Bold,
                );
                cr.set_font_size(9.5);
                if is_hl.is_some() {
                    cr.set_source_rgb(border_r, border_g, border_b);
                } else {
                    cr.set_source_rgb(
                        tokens::color::TEXT_MUTED.0,
                        tokens::color::TEXT_MUTED.1,
                        tokens::color::TEXT_MUTED.2,
                    );
                }
                let _ = cr.move_to(tt_x + 10.0, tt_y + 32.0);
                let _ = cr.show_text(&line2);

                // Line 3: Conflict Reason Warning
                if let Some(ref l3) = line3 {
                    cr.select_font_face(
                        tokens::font::CAIRO_MONO,
                        cairo::FontSlant::Normal,
                        cairo::FontWeight::Bold,
                    );
                    cr.set_font_size(9.5);
                    cr.set_source_rgb(border_r, border_g, border_b);
                    let _ = cr.move_to(tt_x + 10.0, tt_y + 48.0);
                    let _ = cr.show_text(l3);
                }
            }
        }
    }
}

pub fn setup_nucleo_pinout_drawing_and_gestures(
    state: &Rc<RefCell<AppState>>,
    widgets: &Rc<AppWidgets>,
) {
    let state_draw = Rc::clone(state);
    widgets
        .pinout_drawing_area
        .set_draw_func(move |area, cr, w, h| {
            draw_nucleo_pinout_canvas(area, cr, w as f64, h as f64, &state_draw);
        });

    let motion = gtk4::EventControllerMotion::new();
    let state_motion = Rc::clone(state);
    let widgets_motion = Rc::clone(widgets);

    motion.connect_motion(move |_, x, y| {
        let area = &widgets_motion.pinout_drawing_area;
        let cw = area.width().max(800) as f64;
        let ch = area.height().max(600) as f64;

        let board_id = {
            let proj = state_motion.borrow().project.clone();
            let proj_guard = proj.borrow();
            proj_guard
                .loaded_project
                .as_ref()
                .and_then(|p| stakhal_core::board::detect_board(&p.meta.mcu_name))
                .unwrap_or(stakhal_core::board::BoardId::NucleoF446re)
        };
        let layout = BoardLayout::for_board(board_id);

        let mut hit_pin: Option<(&'static str, u8, &'static str, Option<&'static str>)> = None;

        for conn in layout.connectors {
            for (idx, p) in conn.pins.iter().enumerate() {
                let (cell_x, cell_y, cell_w, cell_h) = get_pin_cell_rect(conn.name, idx, cw, ch);

                if x >= cell_x && x <= cell_x + cell_w && y >= cell_y && y <= cell_y + cell_h {
                    hit_pin = Some((conn.name, p.pin_num, p.mcu_pin, p.default_label));
                    break;
                }
            }
            if hit_pin.is_some() {
                break;
            }
        }

        match hit_pin {
            Some((conn_name, pin_num, _mcu_pin, _default_label)) => {
                let mut needs_draw = false;
                state_motion.borrow().with_canvas_state_mut(|c| {
                    let current_hovered = c.hovered_pinout_pin.clone();
                    let new_hovered = Some((conn_name.to_string(), pin_num));

                    c.hovered_pinout_mouse = Some((x, y));

                    if current_hovered != new_hovered {
                        c.hovered_pinout_pin = new_hovered;
                    }
                    needs_draw = true;
                });
                if needs_draw {
                    area.queue_draw();
                }
            }
            None => {
                let mut needs_draw = false;
                state_motion.borrow().with_canvas_state_mut(|c| {
                    if c.hovered_pinout_pin.is_some() || c.hovered_pinout_mouse.is_some() {
                        c.hovered_pinout_pin = None;
                        c.hovered_pinout_mouse = None;
                        needs_draw = true;
                    }
                });
                if needs_draw {
                    area.queue_draw();
                }
            }
        }
    });

    widgets.pinout_drawing_area.add_controller(motion);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connectors_definition() {
        assert_eq!(CONNECTORS.len(), 6);
        let names: Vec<&str> = CONNECTORS.iter().map(|c| c.name).collect();
        assert!(names.contains(&"CN5"));
        assert!(names.contains(&"CN6"));
        assert!(names.contains(&"CN7"));
        assert!(names.contains(&"CN8"));
        assert!(names.contains(&"CN9"));
        assert!(names.contains(&"CN10"));

        for conn in CONNECTORS {
            assert!(!conn.pins.is_empty());
            for p in conn.pins {
                assert!(p.pin_num >= 1);
            }
        }
    }

    #[test]
    fn test_get_pin_cell_rect_bounds() {
        for conn in CONNECTORS {
            for (idx, p) in conn.pins.iter().enumerate() {
                let (x, y, w, h) = get_pin_cell_rect(conn.name, idx, 1600.0, 980.0);
                assert!(
                    x >= 30.0,
                    "Pin cell X {} out of bounds for {} pin {}",
                    x,
                    conn.name,
                    p.pin_num
                );
                assert!(
                    x + w <= 1570.0,
                    "Pin cell X+W {} out of bounds for {} pin {}",
                    x + w,
                    conn.name,
                    p.pin_num
                );
                assert!(
                    y >= 30.0,
                    "Pin cell Y {} out of bounds for {} pin {}",
                    y,
                    conn.name,
                    p.pin_num
                );
                assert!(
                    y + h <= 960.0,
                    "Pin cell Y+H {} out of bounds for {} pin {}",
                    y + h,
                    conn.name,
                    p.pin_num
                );
            }
        }

        for conn in BLACKPILL_CONNECTORS {
            for (idx, p) in conn.pins.iter().enumerate() {
                let (x, y, w, h) = get_pin_cell_rect(conn.name, idx, 1600.0, 980.0);
                assert!(
                    x >= 30.0,
                    "Black Pill pin cell X {} out of bounds for {} pin {}",
                    x,
                    conn.name,
                    p.pin_num
                );
                assert!(
                    x + w <= 1570.0,
                    "Black Pill pin cell X+W {} out of bounds for {} pin {}",
                    x + w,
                    conn.name,
                    p.pin_num
                );
                assert!(
                    y >= 30.0,
                    "Black Pill pin cell Y {} out of bounds for {} pin {}",
                    y,
                    conn.name,
                    p.pin_num
                );
                assert!(
                    y + h <= 960.0,
                    "Black Pill pin cell Y+H {} out of bounds for {} pin {}",
                    y + h,
                    conn.name,
                    p.pin_num
                );
            }
        }
    }

    #[test]
    fn test_board_layout_selection() {
        let nucleo = BoardLayout::for_board(stakhal_core::board::BoardId::NucleoF446re);
        assert_eq!(nucleo.title, "STMicroelectronics NUCLEO-F446RE");
        assert_eq!(nucleo.chip_label, "STM32F446");
        assert_eq!(nucleo.connectors.len(), 6);

        let blackpill = BoardLayout::for_board(stakhal_core::board::BoardId::BlackPillF411ce);
        assert_eq!(blackpill.title, "WeAct Studio STM32F411CEU6 Black Pill");
        assert_eq!(blackpill.chip_label, "STM32F411");
        assert_eq!(blackpill.connectors.len(), 3);
    }

    #[test]
    fn test_draw_nucleo_pinout_canvas_rendering() {
        let mut surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 1200, 750)
            .expect("Failed to create surface");
        let state = Rc::new(RefCell::new(AppState::default()));
        let fixture_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../stakhal-core/tests/fixtures/stakhal_blink_f446re");
        let ioc_path = fixture_dir.join("stakhal_blink_f446re.ioc");
        let main_c_path = fixture_dir.join("Core/Src/main.c");
        if let Ok(project) = stakhal_core::ir::schema::load_project(&ioc_path, &main_c_path) {
            state.borrow().project.borrow_mut().loaded_project = Some(project);
        }

        {
            let cr = cairo::Context::new(&surface).expect("Failed to create context");
            draw_nucleo_pinout(&cr, 1200.0, 750.0, &state);
            surface.flush();
        }

        // Deterministic render assertion: render a second time on identical state and verify identical buffer
        let mut surface2 = cairo::ImageSurface::create(cairo::Format::ARgb32, 1200, 750)
            .expect("Failed to create surface 2");
        {
            let cr2 = cairo::Context::new(&surface2).expect("Failed to create context 2");
            draw_nucleo_pinout(&cr2, 1200.0, 750.0, &state);
            surface2.flush();
        }

        let data1 = surface.data().expect("Failed to get surface data 1");
        let data2 = surface2.data().expect("Failed to get surface data 2");
        assert_eq!(
            &data1[..],
            &data2[..],
            "Rendering must be deterministic and pixel-identical"
        );
        let non_zero_pixels = data1.iter().filter(|&&b| b != 0).count();
        assert!(
            non_zero_pixels > 10000,
            "Rendered canvas has too few non-zero pixels: {}",
            non_zero_pixels
        );
        drop(data1);
        drop(data2);

        let existing_project = state.borrow().project.borrow().loaded_project.clone();
        if let Some(mut project) = existing_project {
            project.pins.push(stakhal_core::ioc::parser::PinConfig {
                pin: "PA13".to_string(),
                signal: "GPIO_Output".to_string(),
                label: Some("DBG_SWDIO".to_string()),
                modules: Vec::new(),
            });
            state.borrow().project.borrow_mut().loaded_project = Some(project);
            state.borrow().with_canvas_state_mut(|c| {
                c.hovered_pinout_pin = Some(("CN7".to_string(), 13));
                c.hovered_pinout_mouse = Some((100.0, 200.0));
            });
        }

        {
            let cr = cairo::Context::new(&surface).expect("Failed to create context");
            draw_nucleo_pinout(&cr, 1200.0, 750.0, &state);
            surface.flush();
        }
    }

    #[test]
    fn test_nucleo_pinout_module_filter() {
        let fixture_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../stakhal-core/tests/fixtures/aa_ns_stm_port");
        let ioc_path = fixture_dir.join("aa_ns_stm_port.ioc");
        let main_c_path = fixture_dir.join("Core/Src/main.c");
        let project = stakhal_core::ir::schema::load_project(&ioc_path, &main_c_path)
            .expect("Failed to load aa_ns_stm_port");

        let state = AppState::default();
        state.project.borrow_mut().loaded_project = Some(project);

        // When selected_pinout_module is None ("All Modules"), no active pins are muted
        let all_hl = get_active_pin_highlights(&state);
        assert!(!all_hl.is_empty());
        for hl in all_hl.values() {
            assert!(!hl.is_muted);
        }

        // When selected_pinout_module is Some("hatch"), pins in hatch are active, other pins are muted
        let hatch_state = AppState::default();
        hatch_state.with_canvas_state_mut(|c| {
            c.selected_pinout_module = Some("hatch".to_string());
        });
        hatch_state.project.borrow_mut().loaded_project =
            state.project.borrow().loaded_project.clone();
        let hatch_hl = get_active_pin_highlights(&hatch_state);

        // Find GRIP_IN1
        let grip = hatch_hl
            .values()
            .find(|h| h.label.as_deref() == Some("GRIP_IN1"))
            .expect("GRIP_IN1 missing");
        assert!(
            !grip.is_muted,
            "GRIP_IN1 should not be muted for hatch module"
        );
        assert!(grip.modules.contains(&"hatch".to_string()));

        // Find Z1_LIMIT
        let z1 = hatch_hl
            .values()
            .find(|h| h.label.as_deref() == Some("Z1_LIMIT"))
            .expect("Z1_LIMIT missing");
        assert!(
            z1.is_muted,
            "Z1_LIMIT should be muted when hatch is selected"
        );
        assert!(z1.modules.contains(&"alignment".to_string()));

        // Render to canvas to verify drawing with module filter doesn't panic
        let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 1200, 750)
            .expect("Failed to create surface");
        let cr = cairo::Context::new(&surface).expect("Failed to create context");
        let rc_state = Rc::new(RefCell::new(hatch_state));
        draw_nucleo_pinout(&cr, 1200.0, 750.0, &rc_state);
        surface.flush();
    }

    #[test]
    fn test_draw_blackpill_pinout_canvas_rendering() {
        let mut surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 1200, 750)
            .expect("Failed to create surface");
        let state = Rc::new(RefCell::new(AppState::default()));
        let bp_project = stakhal_core::ir::schema::Project {
            meta: stakhal_core::ir::schema::ProjectMeta {
                name: "test_blackpill".to_string(),
                mcu_family: "STM32F4".to_string(),
                mcu_name: "STM32F411CEUx".to_string(),
                ioc_path: std::path::PathBuf::from("test.ioc"),
                main_c_path: std::path::PathBuf::from("main.c"),
            },
            pins: vec![
                stakhal_core::ioc::parser::PinConfig {
                    pin: "PC13".to_string(),
                    signal: "GPIO_Output".to_string(),
                    label: Some("USER_LED".to_string()),
                    modules: vec!["led".to_string()],
                },
                stakhal_core::ioc::parser::PinConfig {
                    pin: "PA0".to_string(),
                    signal: "GPIO_Input".to_string(),
                    label: Some("KEY".to_string()),
                    modules: vec!["button".to_string()],
                },
                stakhal_core::ioc::parser::PinConfig {
                    pin: "PA13".to_string(),
                    signal: "SYS_JTMS-SWDIO".to_string(),
                    label: None,
                    modules: Vec::new(),
                },
            ],
            peripherals: Vec::new(),
            user_regions: Vec::new(),
            loop_body: None,
            call_graph_edges: Vec::new(),
            pv_declarations: Vec::new(),
            state_machines: Vec::new(),
            modules: vec!["led".to_string(), "button".to_string()],
        };
        state.borrow().project.borrow_mut().loaded_project = Some(bp_project);

        {
            let cr = cairo::Context::new(&surface).expect("Failed to create context");
            draw_nucleo_pinout(&cr, 1200.0, 750.0, &state);
            surface.flush();
        }

        // Deterministic render assertion
        let mut surface2 = cairo::ImageSurface::create(cairo::Format::ARgb32, 1200, 750)
            .expect("Failed to create surface 2");
        {
            let cr2 = cairo::Context::new(&surface2).expect("Failed to create context 2");
            draw_nucleo_pinout(&cr2, 1200.0, 750.0, &state);
            surface2.flush();
        }

        let data1 = surface.data().expect("Failed to get surface data 1");
        let data2 = surface2.data().expect("Failed to get surface data 2");
        assert_eq!(
            &data1[..],
            &data2[..],
            "Black Pill rendering must be deterministic and pixel-identical"
        );
        let non_zero_pixels = data1.iter().filter(|&&b| b != 0).count();
        assert!(
            non_zero_pixels > 10000,
            "Black Pill canvas has too few non-zero pixels: {}",
            non_zero_pixels
        );
        drop(data1);
        drop(data2);

        // Hover over Left-19 (PC13)
        state.borrow().with_canvas_state_mut(|c| {
            c.hovered_pinout_pin = Some(("Left".to_string(), 19));
            c.hovered_pinout_mouse = Some((250.0, 300.0));
        });
        {
            let cr = cairo::Context::new(&surface).expect("Failed to create context");
            draw_nucleo_pinout(&cr, 1200.0, 750.0, &state);
            surface.flush();
        }

        // Hover over SWD-2 (PA14/SWCLK)
        state.borrow().with_canvas_state_mut(|c| {
            c.hovered_pinout_pin = Some(("SWD".to_string(), 2));
            c.hovered_pinout_mouse = Some((600.0, 650.0));
        });
        {
            let cr = cairo::Context::new(&surface).expect("Failed to create context");
            draw_nucleo_pinout(&cr, 1200.0, 750.0, &state);
            surface.flush();
        }

        // Filter by module "led"
        state.borrow().with_canvas_state_mut(|c| {
            c.selected_pinout_module = Some("led".to_string());
            c.hovered_pinout_pin = None;
            c.hovered_pinout_mouse = None;
        });
        let highlights = get_active_pin_highlights(&state.borrow());
        let led_hl = highlights
            .get(&("Left", 19))
            .expect("PC13 highlight missing");
        assert!(!led_hl.is_muted, "PC13 should be active for 'led' module");

        let key_hl = highlights
            .get(&("Left", 15))
            .expect("PA0 highlight missing");
        assert!(key_hl.is_muted, "PA0 should be muted for 'led' module");

        {
            let cr = cairo::Context::new(&surface).expect("Failed to create context");
            draw_nucleo_pinout(&cr, 1200.0, 750.0, &state);
            surface.flush();
        }
    }

    #[test]
    fn test_reserved_pin_conflict_filtering() {
        // Intended debug/oscillator signals should NOT be conflicts
        assert!(!is_reserved_pin_conflict("PA13", "SYS_JTMS-SWDIO"));
        assert!(!is_reserved_pin_conflict("PA14", "SYS_JTCK-SWCLK"));
        assert!(!is_reserved_pin_conflict("PH0", "RCC_OSC_IN"));
        assert!(!is_reserved_pin_conflict("PH1", "RCC_OSC_OUT"));
        assert!(!is_reserved_pin_conflict("PC14", "RCC_OSC32_IN"));
        assert!(!is_reserved_pin_conflict("PC15", "RCC_OSC32_OUT"));
        assert!(!is_reserved_pin_conflict("PH0 - OSC_IN", "RCC_OSC_IN"));
        assert!(!is_reserved_pin_conflict("PH1-OSC_OUT", "RCC_OSC_OUT"));

        // Other signals on reserved pins MUST be conflicts
        assert!(is_reserved_pin_conflict("PA13", "GPIO_Output"));
        assert!(is_reserved_pin_conflict("PA13", "TIM1_CH1"));
        assert!(is_reserved_pin_conflict("PA14", "GPIO_Input"));
        assert!(is_reserved_pin_conflict("PH0", "GPIO_Output"));
        assert!(is_reserved_pin_conflict("PH1", "GPIO_Input"));
        assert!(is_reserved_pin_conflict("PC14", "GPIO_Output"));
        assert!(is_reserved_pin_conflict("PC15", "GPIO_Output"));

        // Other reserved pins (e.g. PB2 BOOT1, PC13 LED) are conflicts for general GPIO
        assert!(is_reserved_pin_conflict("PB2", "GPIO_Output"));
        assert!(is_reserved_pin_conflict("PC13", "GPIO_Output"));
    }

    #[test]
    fn test_swd_cell_text_shortening() {
        let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 1200, 750)
            .expect("Failed to create surface");
        let state = Rc::new(RefCell::new(AppState::default()));
        let bp_project = stakhal_core::ir::schema::Project {
            meta: stakhal_core::ir::schema::ProjectMeta {
                name: "test_swd".to_string(),
                mcu_family: "STM32F4".to_string(),
                mcu_name: "STM32F411CEUx".to_string(),
                ioc_path: std::path::PathBuf::from("test.ioc"),
                main_c_path: std::path::PathBuf::from("main.c"),
            },
            pins: vec![
                stakhal_core::ioc::parser::PinConfig {
                    pin: "PA13".to_string(),
                    signal: "SYS_JTMS-SWDIO".to_string(),
                    label: None,
                    modules: Vec::new(),
                },
                stakhal_core::ioc::parser::PinConfig {
                    pin: "PA14".to_string(),
                    signal: "SYS_JTCK-SWCLK".to_string(),
                    label: None,
                    modules: Vec::new(),
                },
            ],
            peripherals: Vec::new(),
            user_regions: Vec::new(),
            loop_body: None,
            call_graph_edges: Vec::new(),
            pv_declarations: Vec::new(),
            state_machines: Vec::new(),
            modules: Vec::new(),
        };
        state.borrow().project.borrow_mut().loaded_project = Some(bp_project);

        let cr = cairo::Context::new(&surface).expect("Failed to create context");
        draw_blackpill_f411_board(&cr, 1200.0, 750.0, &state);
        surface.flush();
    }

    #[test]
    fn test_tooltip_never_covers_header_title() {
        let cw: f64 = 1200.0;
        let ch: f64 = 750.0;
        let margin_x = (cw * 0.025).max(16.0);
        let margin_y = (ch * 0.035).max(20.0);
        let bounds = (margin_x, margin_y, cw - 2.0 * margin_x, ch - 2.0 * margin_y);

        let connectors = [
            "Left", "Right", "SWD", "CN7", "CN10", "CN6", "CN8", "CN5", "CN9",
        ];
        for conn in connectors {
            let title_rect = get_header_title_rect(conn, cw, ch);
            let top_idx = match conn {
                "CN5" => 9,
                "CN9" => 7,
                _ => 0,
            };
            let (cx, cy, _cw, _ch) = get_pin_cell_rect(conn, top_idx, cw, ch);
            let hover_x = cx + 20.0;
            let hover_y = cy + 10.0;

            let (tt_x, tt_y, tt_w, tt_h) =
                compute_tooltip_card_rect(hover_x, hover_y, 180.0, 58.0, conn, bounds);

            let (tx, ty, tw, th) = title_rect;
            let overlaps = tt_x < tx + tw && tt_x + tt_w > tx && tt_y < ty + th && tt_y + tt_h > ty;
            assert!(
                !overlaps,
                "Tooltip at ({}, {}, {}, {}) overlaps header title for '{}' at ({}, {}, {}, {})",
                tt_x, tt_y, tt_w, tt_h, conn, tx, ty, tw, th
            );
        }
    }
}
