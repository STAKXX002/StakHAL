//! Physical connector pinout for the WeAct Studio STM32F411CEU6 "Black Pill" board (V3.x).
//!
//! Source: WeAct Studio MiniF4x1Cx V3.1 schematic (`MiniF4x1Cx_V31 SchDoc.pdf`) and
//! hardware repository (<https://github.com/WeActStudio/WeActStudio.MiniSTM32F4x1> and
//! <https://github.com/WeActStudio/WeActStudio.BlackPill>). Cross-checked with
//! WeAct Studio Black Pill V3.x board silkscreen and stm32-base.org board reference.
//!
//! Physical layout:
//! - Left 20-pin edge header (P1): 5V, GND, 3V3, B10, B2, B1, B0, A7, A6, A5, A4, A3, A2, A1, A0, NRST, C15, C14, C13, VBAT
//! - Right 20-pin edge header (P2): B12, B13, B14, B15, A8, A9, A10, A11, A12, A15, B3, B4, B5, B6, B7, B8, B9, 5V, GND, 3V3
//! - 4-pin SWD header (P3): 3V3, SWDIO (PA13), SWCLK (PA14), GND

use crate::nucleo_pinout::{PinLocation, ReservedPin, ReservedSeverity};

/// Pinout table for WeAct Studio Black Pill STM32F411CEU6.
/// Contains all 34 GPIO pins physically broken out on the two 20-pin edge headers
/// and 4-pin SWD header. Power rails, GND, NRST, and VBAT are omitted since they
/// cannot appear as GPIO signal configurations from the .ioc parser.
pub const BLACKPILL_F411CE_PINOUT: &[PinLocation] = &[
    // --- Left 20-pin edge header (P1) ---
    PinLocation {
        mcu_pin: "PB10",
        morpho: None,
        arduino: None,
        header: Some(("Left", 4, "B10")),
    },
    PinLocation {
        mcu_pin: "PB2",
        morpho: None,
        arduino: None,
        header: Some(("Left", 5, "B2")),
    },
    PinLocation {
        mcu_pin: "PB1",
        morpho: None,
        arduino: None,
        header: Some(("Left", 6, "B1")),
    },
    PinLocation {
        mcu_pin: "PB0",
        morpho: None,
        arduino: None,
        header: Some(("Left", 7, "B0")),
    },
    PinLocation {
        mcu_pin: "PA7",
        morpho: None,
        arduino: None,
        header: Some(("Left", 8, "A7")),
    },
    PinLocation {
        mcu_pin: "PA6",
        morpho: None,
        arduino: None,
        header: Some(("Left", 9, "A6")),
    },
    PinLocation {
        mcu_pin: "PA5",
        morpho: None,
        arduino: None,
        header: Some(("Left", 10, "A5")),
    },
    PinLocation {
        mcu_pin: "PA4",
        morpho: None,
        arduino: None,
        header: Some(("Left", 11, "A4")),
    },
    PinLocation {
        mcu_pin: "PA3",
        morpho: None,
        arduino: None,
        header: Some(("Left", 12, "A3")),
    },
    PinLocation {
        mcu_pin: "PA2",
        morpho: None,
        arduino: None,
        header: Some(("Left", 13, "A2")),
    },
    PinLocation {
        mcu_pin: "PA1",
        morpho: None,
        arduino: None,
        header: Some(("Left", 14, "A1")),
    },
    PinLocation {
        mcu_pin: "PA0",
        morpho: None,
        arduino: None,
        header: Some(("Left", 15, "A0")),
    },
    PinLocation {
        mcu_pin: "PC15",
        morpho: None,
        arduino: None,
        header: Some(("Left", 17, "C15")),
    },
    PinLocation {
        mcu_pin: "PC14",
        morpho: None,
        arduino: None,
        header: Some(("Left", 18, "C14")),
    },
    PinLocation {
        mcu_pin: "PC13",
        morpho: None,
        arduino: None,
        header: Some(("Left", 19, "C13")),
    },
    // --- Right 20-pin edge header (P2) ---
    PinLocation {
        mcu_pin: "PB12",
        morpho: None,
        arduino: None,
        header: Some(("Right", 1, "B12")),
    },
    PinLocation {
        mcu_pin: "PB13",
        morpho: None,
        arduino: None,
        header: Some(("Right", 2, "B13")),
    },
    PinLocation {
        mcu_pin: "PB14",
        morpho: None,
        arduino: None,
        header: Some(("Right", 3, "B14")),
    },
    PinLocation {
        mcu_pin: "PB15",
        morpho: None,
        arduino: None,
        header: Some(("Right", 4, "B15")),
    },
    PinLocation {
        mcu_pin: "PA8",
        morpho: None,
        arduino: None,
        header: Some(("Right", 5, "A8")),
    },
    PinLocation {
        mcu_pin: "PA9",
        morpho: None,
        arduino: None,
        header: Some(("Right", 6, "A9")),
    },
    PinLocation {
        mcu_pin: "PA10",
        morpho: None,
        arduino: None,
        header: Some(("Right", 7, "A10")),
    },
    PinLocation {
        mcu_pin: "PA11",
        morpho: None,
        arduino: None,
        header: Some(("Right", 8, "A11")),
    },
    PinLocation {
        mcu_pin: "PA12",
        morpho: None,
        arduino: None,
        header: Some(("Right", 9, "A12")),
    },
    PinLocation {
        mcu_pin: "PA15",
        morpho: None,
        arduino: None,
        header: Some(("Right", 10, "A15")),
    },
    PinLocation {
        mcu_pin: "PB3",
        morpho: None,
        arduino: None,
        header: Some(("Right", 11, "B3")),
    },
    PinLocation {
        mcu_pin: "PB4",
        morpho: None,
        arduino: None,
        header: Some(("Right", 12, "B4")),
    },
    PinLocation {
        mcu_pin: "PB5",
        morpho: None,
        arduino: None,
        header: Some(("Right", 13, "B5")),
    },
    PinLocation {
        mcu_pin: "PB6",
        morpho: None,
        arduino: None,
        header: Some(("Right", 14, "B6")),
    },
    PinLocation {
        mcu_pin: "PB7",
        morpho: None,
        arduino: None,
        header: Some(("Right", 15, "B7")),
    },
    PinLocation {
        mcu_pin: "PB8",
        morpho: None,
        arduino: None,
        header: Some(("Right", 16, "B8")),
    },
    PinLocation {
        mcu_pin: "PB9",
        morpho: None,
        arduino: None,
        header: Some(("Right", 17, "B9")),
    },
    // --- 4-pin SWD header (P3) ---
    PinLocation {
        mcu_pin: "PA13",
        morpho: None,
        arduino: None,
        header: Some(("SWD", 2, "SWDIO")),
    },
    PinLocation {
        mcu_pin: "PA14",
        morpho: None,
        arduino: None,
        header: Some(("SWD", 3, "SWCLK")),
    },
];

pub fn lookup_pin(mcu_pin: &str) -> Option<&'static PinLocation> {
    BLACKPILL_F411CE_PINOUT
        .iter()
        .find(|p| p.mcu_pin == mcu_pin)
}

/// Hardware-reserved pins on the WeAct Studio Black Pill board.
pub const BLACKPILL_RESERVED_PINS: &[ReservedPin] = &[
    ReservedPin {
        mcu_pin: "PA13",
        reason: "PA13 is SWDIO • SWD header, reprogramming needs these.",
        severity: ReservedSeverity::Critical,
    },
    ReservedPin {
        mcu_pin: "PA14",
        reason: "PA14 is SWCLK • SWD header, reprogramming needs these.",
        severity: ReservedSeverity::Critical,
    },
    ReservedPin {
        mcu_pin: "PH0",
        reason: "PH0 is OSC_IN • 25 MHz HSE main oscillator input.",
        severity: ReservedSeverity::Caution,
    },
    ReservedPin {
        mcu_pin: "PH1",
        reason: "PH1 is OSC_OUT • 25 MHz HSE main oscillator output.",
        severity: ReservedSeverity::Caution,
    },
    ReservedPin {
        mcu_pin: "PC14",
        reason: "PC14 is OSC32_IN • 32.768 kHz LSE RTC oscillator input.",
        severity: ReservedSeverity::Caution,
    },
    ReservedPin {
        mcu_pin: "PC15",
        reason: "PC15 is OSC32_OUT • 32.768 kHz LSE RTC oscillator output.",
        severity: ReservedSeverity::Caution,
    },
    ReservedPin {
        mcu_pin: "PA11",
        reason: "PA11 is USB_DM • USB D- differential data line connected to USB-C.",
        severity: ReservedSeverity::Caution,
    },
    ReservedPin {
        mcu_pin: "PA12",
        reason: "PA12 is USB_DP • USB D+ differential data line connected to USB-C.",
        severity: ReservedSeverity::Caution,
    },
    ReservedPin {
        mcu_pin: "PB2",
        reason: "PB2 is BOOT1 • boot mode selection pin.",
        severity: ReservedSeverity::Caution,
    },
    ReservedPin {
        mcu_pin: "PC13",
        reason: "PC13 is connected to onboard LED (active low).",
        severity: ReservedSeverity::Caution,
    },
    ReservedPin {
        mcu_pin: "PA0",
        reason: "PA0 is connected to onboard KEY button.",
        severity: ReservedSeverity::Caution,
    },
];

pub fn check_reserved(mcu_pin: &str) -> Option<&'static ReservedPin> {
    BLACKPILL_RESERVED_PINS
        .iter()
        .find(|p| p.mcu_pin == mcu_pin)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn no_duplicate_mcu_pin_entries() {
        let mut seen = HashSet::new();
        for entry in BLACKPILL_F411CE_PINOUT {
            assert!(
                seen.insert(entry.mcu_pin),
                "Duplicate entry found for {}",
                entry.mcu_pin
            );
        }
    }

    #[test]
    fn only_existing_gpio_appear() {
        for entry in BLACKPILL_F411CE_PINOUT {
            let pin = entry.mcu_pin;
            // No PD pins exist on UFQFPN48
            assert!(
                !pin.starts_with("PD"),
                "PD pin {} should not exist on UFQFPN48 package",
                pin
            );
            // PB11 does not exist on UFQFPN48
            assert_ne!(pin, "PB11", "PB11 does not exist on UFQFPN48 package");
            // PC0-PC12 do not exist on UFQFPN48
            if pin.starts_with("PC") {
                let num: u8 = pin[2..].parse().expect("valid PC pin number");
                assert!(
                    num >= 13,
                    "PC{} should not exist on UFQFPN48 package (only PC13-PC15 exist)",
                    num
                );
            }
        }
    }

    #[test]
    fn pa13_pa14_map_to_swd_header() {
        let pa13 = lookup_pin("PA13").expect("PA13 should exist in Black Pill table");
        assert_eq!(pa13.header, Some(("SWD", 2, "SWDIO")));

        let pa14 = lookup_pin("PA14").expect("PA14 should exist in Black Pill table");
        assert_eq!(pa14.header, Some(("SWD", 3, "SWCLK")));
    }

    #[test]
    fn pa0_and_pc13_present() {
        let pa0 = lookup_pin("PA0").expect("PA0 should exist in Black Pill table");
        assert_eq!(pa0.header, Some(("Left", 15, "A0")));

        let pc13 = lookup_pin("PC13").expect("PC13 should exist in Black Pill table");
        assert_eq!(pc13.header, Some(("Left", 19, "C13")));
    }

    #[test]
    fn test_reserved_pins_critical_and_caution() {
        // Critical: PA13 and PA14 on SWD header
        let pa13 = check_reserved("PA13").expect("PA13 should be reserved");
        assert_eq!(pa13.severity, ReservedSeverity::Critical);
        assert!(pa13
            .reason
            .contains("SWD header, reprogramming needs these"));
        assert!(!pa13.reason.contains("onboard debugger"));

        let pa14 = check_reserved("PA14").expect("PA14 should be reserved");
        assert_eq!(pa14.severity, ReservedSeverity::Critical);
        assert!(pa14
            .reason
            .contains("SWD header, reprogramming needs these"));
        assert!(!pa14.reason.contains("onboard debugger"));

        // Caution: HSE, LSE, USB D-/D+, BOOT1, LED, KEY
        let caution_pins = [
            ("PH0", "HSE"),
            ("PH1", "HSE"),
            ("PC14", "LSE"),
            ("PC15", "LSE"),
            ("PA11", "USB"),
            ("PA12", "USB"),
            ("PB2", "BOOT1"),
            ("PC13", "LED"),
            ("PA0", "KEY"),
        ];
        for (pin, tag) in caution_pins {
            let res = check_reserved(pin).unwrap_or_else(|| panic!("{} should be reserved", pin));
            assert_eq!(res.severity, ReservedSeverity::Caution, "pin {}", pin);
            assert!(
                res.reason.contains(tag) || res.reason.to_uppercase().contains(tag),
                "pin {} reason {:?} should mention {}",
                pin,
                res.reason,
                tag
            );
            assert!(!res.reason.contains("onboard debugger"));
        }

        // Unreserved pins return None
        assert!(check_reserved("PB0").is_none());
        assert!(check_reserved("PA1").is_none());
        assert!(check_reserved("PB12").is_none());
    }
}
