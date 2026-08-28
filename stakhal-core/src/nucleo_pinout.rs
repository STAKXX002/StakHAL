//! Physical connector pinout for the NUCLEO-F446RE board.
//!
//! Source: ST UM1724 "STM32 Nucleo-64 boards" user manual -
//! Table 19 "Arduino connectors on NUCLEO-F446RE" (CN5, CN6, CN8, CN9) and
//! Table 27 "STMicroelectronics Morpho connector on NUCLEO-F401RE,
//! NUCLEO-F411RE, NUCLEO-F446RE" (CN7, CN10). Cross-checked against both
//! the official document text and the board silkscreen reference diagram.
//!
//! This module is intentionally board-specific (F446RE only, per current
//! scope) rather than generic across the whole Nucleo-64 family - other
//! MCU variants remap a handful of these pins (notably around CN10 pins
//! 15-18 and 25-30) and folding that in later is a straightforward
//! extension of this same shape, not a redesign.

/// Where a given MCU GPIO pin physically lands on the Nucleo-64 board.
/// A pin may appear on the Morpho header, the Arduino header, both, or
/// (rarely) neither (power/ground/reserved pins aren't included here).
#[derive(Debug, Clone, Copy)]
pub struct PinLocation {
    /// The MCU pin name as it appears in the .ioc file, e.g. "PA5".
    pub mcu_pin: &'static str,
    /// Morpho connector name + physical pin number, if present there.
    pub morpho: Option<(&'static str, u8)>,
    /// Arduino connector name + physical pin number + Arduino label
    /// (e.g. "D13", "A0"), if present there.
    pub arduino: Option<(&'static str, u8, &'static str)>,
}

/// Full pinout table for NUCLEO-F446RE. GPIO/peripheral-capable pins only
/// (power rails, GND, VIN, RESET, BOOT0, IOREF etc. are omitted since they
/// can never appear as a PinConfig::pin value from the .ioc parser).
pub const NUCLEO_F446RE_PINOUT: &[PinLocation] = &[
    // --- CN7 (Morpho, left) only ---
    PinLocation { mcu_pin: "PC10", morpho: Some(("CN7", 1)), arduino: None },
    PinLocation { mcu_pin: "PC11", morpho: Some(("CN7", 2)), arduino: None },
    PinLocation { mcu_pin: "PC12", morpho: Some(("CN7", 3)), arduino: None },
    PinLocation { mcu_pin: "PD2",  morpho: Some(("CN7", 4)), arduino: None },
    PinLocation { mcu_pin: "PA13", morpho: Some(("CN7", 13)), arduino: None },
    PinLocation { mcu_pin: "PA14", morpho: Some(("CN7", 15)), arduino: None },
    PinLocation { mcu_pin: "PA15", morpho: Some(("CN7", 17)), arduino: None },
    PinLocation { mcu_pin: "PB7",  morpho: Some(("CN7", 21)), arduino: None },
    PinLocation { mcu_pin: "PC13", morpho: Some(("CN7", 23)), arduino: None },
    PinLocation { mcu_pin: "PC14", morpho: Some(("CN7", 25)), arduino: None },
    PinLocation { mcu_pin: "PC15", morpho: Some(("CN7", 27)), arduino: None },
    PinLocation { mcu_pin: "PH0",  morpho: Some(("CN7", 29)), arduino: None },
    PinLocation { mcu_pin: "PH1",  morpho: Some(("CN7", 31)), arduino: None },
    // PC2/PC3 (CN7 pins 35/37) are ADC-only breakouts, not general GPIO
    // signals in the .ioc pin-signal sense on this variant; included for
    // completeness of the physical header.
    PinLocation { mcu_pin: "PC2",  morpho: Some(("CN7", 35)), arduino: None },
    PinLocation { mcu_pin: "PC3",  morpho: Some(("CN7", 37)), arduino: None },

    // --- Shared between CN7/CN8 (Morpho + Arduino analog) ---
    PinLocation { mcu_pin: "PA0", morpho: Some(("CN7", 28)), arduino: Some(("CN8", 1, "A0")) },
    PinLocation { mcu_pin: "PA1", morpho: Some(("CN7", 30)), arduino: Some(("CN8", 2, "A1")) },
    PinLocation { mcu_pin: "PA4", morpho: Some(("CN7", 32)), arduino: Some(("CN8", 3, "A2")) },
    PinLocation { mcu_pin: "PB0", morpho: Some(("CN7", 34)), arduino: Some(("CN8", 4, "A3")) },
    // PC1/PC0 share pins with PB9/PB8 via solder bridge (SB56/SB51) -
    // PC1/PC0 is the default (as-shipped) routing.
    PinLocation { mcu_pin: "PC1", morpho: Some(("CN7", 36)), arduino: Some(("CN8", 5, "A4")) },
    PinLocation { mcu_pin: "PC0", morpho: Some(("CN7", 38)), arduino: Some(("CN8", 6, "A5")) },

    // --- CN10 (Morpho, right) only ---
    PinLocation { mcu_pin: "PC9",  morpho: Some(("CN10", 1)), arduino: None },
    PinLocation { mcu_pin: "PC8",  morpho: Some(("CN10", 2)), arduino: None },
    PinLocation { mcu_pin: "PC6",  morpho: Some(("CN10", 4)), arduino: None },
    PinLocation { mcu_pin: "PC5",  morpho: Some(("CN10", 6)), arduino: None },
    PinLocation { mcu_pin: "PA12", morpho: Some(("CN10", 12)), arduino: None },
    PinLocation { mcu_pin: "PA11", morpho: Some(("CN10", 14)), arduino: None },
    PinLocation { mcu_pin: "PB12", morpho: Some(("CN10", 16)), arduino: None },
    PinLocation { mcu_pin: "PB2",  morpho: Some(("CN10", 22)), arduino: None },
    PinLocation { mcu_pin: "PB1",  morpho: Some(("CN10", 24)), arduino: None },
    PinLocation { mcu_pin: "PB15", morpho: Some(("CN10", 26)), arduino: None },
    PinLocation { mcu_pin: "PB14", morpho: Some(("CN10", 28)), arduino: None },
    PinLocation { mcu_pin: "PB13", morpho: Some(("CN10", 30)), arduino: None },
    PinLocation { mcu_pin: "PC4",  morpho: Some(("CN10", 34)), arduino: None },

    // --- Shared between CN10/CN5 (Morpho + Arduino digital) ---
    PinLocation { mcu_pin: "PB8", morpho: Some(("CN10", 3)),  arduino: Some(("CN5", 10, "D15")) },
    PinLocation { mcu_pin: "PB9", morpho: Some(("CN10", 5)),  arduino: Some(("CN5", 9, "D14")) },
    PinLocation { mcu_pin: "PA5", morpho: Some(("CN10", 11)), arduino: Some(("CN5", 6, "D13")) },
    PinLocation { mcu_pin: "PA6", morpho: Some(("CN10", 13)), arduino: Some(("CN5", 5, "D12")) },
    PinLocation { mcu_pin: "PA7", morpho: Some(("CN10", 15)), arduino: Some(("CN5", 4, "D11")) },
    PinLocation { mcu_pin: "PB6", morpho: Some(("CN10", 17)), arduino: Some(("CN5", 3, "D10")) },
    PinLocation { mcu_pin: "PC7", morpho: Some(("CN10", 19)), arduino: Some(("CN5", 2, "D9")) },
    PinLocation { mcu_pin: "PA9", morpho: Some(("CN10", 21)), arduino: Some(("CN5", 1, "D8")) },

    // --- Shared between CN10/CN9 (Morpho + Arduino digital) ---
    PinLocation { mcu_pin: "PA8",  morpho: Some(("CN10", 23)), arduino: Some(("CN9", 8, "D7")) },
    PinLocation { mcu_pin: "PB10", morpho: Some(("CN10", 25)), arduino: Some(("CN9", 7, "D6")) },
    PinLocation { mcu_pin: "PB4",  morpho: Some(("CN10", 27)), arduino: Some(("CN9", 6, "D5")) },
    PinLocation { mcu_pin: "PB5",  morpho: Some(("CN10", 29)), arduino: Some(("CN9", 5, "D4")) },
    PinLocation { mcu_pin: "PB3",  morpho: Some(("CN10", 31)), arduino: Some(("CN9", 4, "D3")) },
    PinLocation { mcu_pin: "PA10", morpho: Some(("CN10", 33)), arduino: Some(("CN9", 3, "D2")) },
    PinLocation { mcu_pin: "PA2",  morpho: Some(("CN10", 35)), arduino: Some(("CN9", 2, "D1")) },
    PinLocation { mcu_pin: "PA3",  morpho: Some(("CN10", 37)), arduino: Some(("CN9", 1, "D0")) },
];

/// Look up the physical board location(s) for an MCU pin name (e.g. "PA5").
/// Returns None if the pin isn't broken out on this board (rare - most
/// STM32F446RETx GPIO pins are, since it's a LQFP64 package on a Nucleo-64).
pub fn lookup_pin(mcu_pin: &str) -> Option<&'static PinLocation> {
    NUCLEO_F446RE_PINOUT.iter().find(|p| p.mcu_pin == mcu_pin)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReservedSeverity {
    Critical,
    Caution,
}

#[derive(Debug, Clone, Copy)]
pub struct ReservedPin {
    pub mcu_pin: &'static str,
    pub reason: &'static str,
    pub severity: ReservedSeverity,
}

pub const RESERVED_PINS: &[ReservedPin] = &[
    ReservedPin {
        mcu_pin: "PA13",
        reason: "PA13 is SWDIO — used by the onboard debugger. Reusing this pin will break reprogramming.",
        severity: ReservedSeverity::Critical,
    },
    ReservedPin {
        mcu_pin: "PA14",
        reason: "PA14 is SWCLK — used by the onboard debugger. Reusing this pin will break reprogramming.",
        severity: ReservedSeverity::Critical,
    },
    ReservedPin {
        mcu_pin: "PH0",
        reason: "PH0 is OSC_IN — system main oscillator input.",
        severity: ReservedSeverity::Caution,
    },
    ReservedPin {
        mcu_pin: "PH1",
        reason: "PH1 is OSC_OUT — system main oscillator output.",
        severity: ReservedSeverity::Caution,
    },
    ReservedPin {
        mcu_pin: "PC14",
        reason: "PC14 is OSC32_IN — 32.768 kHz RTC oscillator input.",
        severity: ReservedSeverity::Caution,
    },
    ReservedPin {
        mcu_pin: "PC15",
        reason: "PC15 is OSC32_OUT — 32.768 kHz RTC oscillator output.",
        severity: ReservedSeverity::Caution,
    },
];

pub fn check_reserved(mcu_pin: &str) -> Option<&'static ReservedPin> {
    RESERVED_PINS.iter().find(|p| p.mcu_pin == mcu_pin)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_pin_has_both_headers() {
        // PA5 / D13 is the most commonly referenced pin (onboard LED on
        // many Nucleo boards' Arduino silkscreen) - good canary for both
        // Morpho and Arduino sides being wired up correctly.
        let loc = lookup_pin("PA5").expect("PA5 should be in the table");
        assert_eq!(loc.morpho, Some(("CN10", 11)));
        assert_eq!(loc.arduino, Some(("CN5", 6, "D13")));
    }

    #[test]
    fn morpho_only_pin_has_no_arduino_entry() {
        let loc = lookup_pin("PC10").expect("PC10 should be in the table");
        assert_eq!(loc.morpho, Some(("CN7", 1)));
        assert_eq!(loc.arduino, None);
    }

    #[test]
    fn unknown_pin_returns_none() {
        assert!(lookup_pin("PZ99").is_none());
    }

    #[test]
    fn no_duplicate_mcu_pin_entries() {
        let mut seen = std::collections::HashSet::new();
        for entry in NUCLEO_F446RE_PINOUT {
            assert!(seen.insert(entry.mcu_pin), "duplicate entry for {}", entry.mcu_pin);
        }
    }

    #[test]
    fn test_reserved_pins_critical_and_none() {
        let pa13 = check_reserved("PA13").expect("PA13 should be reserved");
        assert_eq!(pa13.severity, ReservedSeverity::Critical);

        let pa14 = check_reserved("PA14").expect("PA14 should be reserved");
        assert_eq!(pa14.severity, ReservedSeverity::Critical);

        let ph0 = check_reserved("PH0").expect("PH0 should be reserved");
        assert_eq!(ph0.severity, ReservedSeverity::Caution);

        assert!(check_reserved("PB0").is_none());
    }
}