/// Physical board dimensions and normalized pin coordinates for NUCLEO-F446RE (MB1136 rev C).
///
/// Dimensions directly verified from ST UM1724 User Manual, Section 6, Figure 5:
/// "STM32 Nucleo board mechanical dimensions" (page 15).
/// Board size: 70.00 mm x 82.50 mm.

pub const BOARD_WIDTH_MM: f64 = 70.0;
pub const BOARD_HEIGHT_MM: f64 = 82.5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PinCoord {
    pub norm_x: f64,
    pub norm_y: f64,
}

impl PinCoord {
    pub const fn new(norm_x: f64, norm_y: f64) -> Self {
        Self { norm_x, norm_y }
    }
}

/// Normalized (x, y) coordinates for CN7 (Left Morpho 2x19, pins 1..38).
pub const CN7_COORDS: &[PinCoord] = &[
    PinCoord::new(0.0464286, 0.384727), // Pin  1 (r= 0, outer)
    PinCoord::new(0.0827143, 0.384727), // Pin  2 (r= 0, inner)
    PinCoord::new(0.0464286, 0.415515), // Pin  3 (r= 1, outer)
    PinCoord::new(0.0827143, 0.415515), // Pin  4 (r= 1, inner)
    PinCoord::new(0.0464286, 0.446303), // Pin  5 (r= 2, outer)
    PinCoord::new(0.0827143, 0.446303), // Pin  6 (r= 2, inner)
    PinCoord::new(0.0464286, 0.477091), // Pin  7 (r= 3, outer)
    PinCoord::new(0.0827143, 0.477091), // Pin  8 (r= 3, inner)
    PinCoord::new(0.0464286, 0.507879), // Pin  9 (r= 4, outer)
    PinCoord::new(0.0827143, 0.507879), // Pin 10 (r= 4, inner)
    PinCoord::new(0.0464286, 0.538667), // Pin 11 (r= 5, outer)
    PinCoord::new(0.0827143, 0.538667), // Pin 12 (r= 5, inner)
    PinCoord::new(0.0464286, 0.569455), // Pin 13 (r= 6, outer)
    PinCoord::new(0.0827143, 0.569455), // Pin 14 (r= 6, inner)
    PinCoord::new(0.0464286, 0.600242), // Pin 15 (r= 7, outer)
    PinCoord::new(0.0827143, 0.600242), // Pin 16 (r= 7, inner)
    PinCoord::new(0.0464286, 0.63103), // Pin 17 (r= 8, outer)
    PinCoord::new(0.0827143, 0.63103), // Pin 18 (r= 8, inner)
    PinCoord::new(0.0464286, 0.661818), // Pin 19 (r= 9, outer)
    PinCoord::new(0.0827143, 0.661818), // Pin 20 (r= 9, inner)
    PinCoord::new(0.0464286, 0.692606), // Pin 21 (r=10, outer)
    PinCoord::new(0.0827143, 0.692606), // Pin 22 (r=10, inner)
    PinCoord::new(0.0464286, 0.723394), // Pin 23 (r=11, outer)
    PinCoord::new(0.0827143, 0.723394), // Pin 24 (r=11, inner)
    PinCoord::new(0.0464286, 0.754182), // Pin 25 (r=12, outer)
    PinCoord::new(0.0827143, 0.754182), // Pin 26 (r=12, inner)
    PinCoord::new(0.0464286, 0.78497), // Pin 27 (r=13, outer)
    PinCoord::new(0.0827143, 0.78497), // Pin 28 (r=13, inner)
    PinCoord::new(0.0464286, 0.815758), // Pin 29 (r=14, outer)
    PinCoord::new(0.0827143, 0.815758), // Pin 30 (r=14, inner)
    PinCoord::new(0.0464286, 0.846545), // Pin 31 (r=15, outer)
    PinCoord::new(0.0827143, 0.846545), // Pin 32 (r=15, inner)
    PinCoord::new(0.0464286, 0.877333), // Pin 33 (r=16, outer)
    PinCoord::new(0.0827143, 0.877333), // Pin 34 (r=16, inner)
    PinCoord::new(0.0464286, 0.908121), // Pin 35 (r=17, outer)
    PinCoord::new(0.0827143, 0.908121), // Pin 36 (r=17, inner)
    PinCoord::new(0.0464286, 0.938909), // Pin 37 (r=18, outer)
    PinCoord::new(0.0827143, 0.938909), // Pin 38 (r=18, inner)
];

/// Normalized (x, y) coordinates for CN10 (Right Morpho 2x19, pins 1..38).
pub const CN10_COORDS: &[PinCoord] = &[
    PinCoord::new(0.917286, 0.384727), // Pin  1 (r= 0, inner)
    PinCoord::new(0.953571, 0.384727), // Pin  2 (r= 0, outer)
    PinCoord::new(0.917286, 0.415515), // Pin  3 (r= 1, inner)
    PinCoord::new(0.953571, 0.415515), // Pin  4 (r= 1, outer)
    PinCoord::new(0.917286, 0.446303), // Pin  5 (r= 2, inner)
    PinCoord::new(0.953571, 0.446303), // Pin  6 (r= 2, outer)
    PinCoord::new(0.917286, 0.477091), // Pin  7 (r= 3, inner)
    PinCoord::new(0.953571, 0.477091), // Pin  8 (r= 3, outer)
    PinCoord::new(0.917286, 0.507879), // Pin  9 (r= 4, inner)
    PinCoord::new(0.953571, 0.507879), // Pin 10 (r= 4, outer)
    PinCoord::new(0.917286, 0.538667), // Pin 11 (r= 5, inner)
    PinCoord::new(0.953571, 0.538667), // Pin 12 (r= 5, outer)
    PinCoord::new(0.917286, 0.569455), // Pin 13 (r= 6, inner)
    PinCoord::new(0.953571, 0.569455), // Pin 14 (r= 6, outer)
    PinCoord::new(0.917286, 0.600242), // Pin 15 (r= 7, inner)
    PinCoord::new(0.953571, 0.600242), // Pin 16 (r= 7, outer)
    PinCoord::new(0.917286, 0.63103), // Pin 17 (r= 8, inner)
    PinCoord::new(0.953571, 0.63103), // Pin 18 (r= 8, outer)
    PinCoord::new(0.917286, 0.661818), // Pin 19 (r= 9, inner)
    PinCoord::new(0.953571, 0.661818), // Pin 20 (r= 9, outer)
    PinCoord::new(0.917286, 0.692606), // Pin 21 (r=10, inner)
    PinCoord::new(0.953571, 0.692606), // Pin 22 (r=10, outer)
    PinCoord::new(0.917286, 0.723394), // Pin 23 (r=11, inner)
    PinCoord::new(0.953571, 0.723394), // Pin 24 (r=11, outer)
    PinCoord::new(0.917286, 0.754182), // Pin 25 (r=12, inner)
    PinCoord::new(0.953571, 0.754182), // Pin 26 (r=12, outer)
    PinCoord::new(0.917286, 0.78497), // Pin 27 (r=13, inner)
    PinCoord::new(0.953571, 0.78497), // Pin 28 (r=13, outer)
    PinCoord::new(0.917286, 0.815758), // Pin 29 (r=14, inner)
    PinCoord::new(0.953571, 0.815758), // Pin 30 (r=14, outer)
    PinCoord::new(0.917286, 0.846545), // Pin 31 (r=15, inner)
    PinCoord::new(0.953571, 0.846545), // Pin 32 (r=15, outer)
    PinCoord::new(0.917286, 0.877333), // Pin 33 (r=16, inner)
    PinCoord::new(0.953571, 0.877333), // Pin 34 (r=16, outer)
    PinCoord::new(0.917286, 0.908121), // Pin 35 (r=17, inner)
    PinCoord::new(0.953571, 0.908121), // Pin 36 (r=17, outer)
    PinCoord::new(0.917286, 0.938909), // Pin 37 (r=18, inner)
    PinCoord::new(0.953571, 0.938909), // Pin 38 (r=18, outer)
];

/// Normalized (x, y) coordinates for CN6 (Left Arduino Power, pins 1..8).
pub const CN6_COORDS: &[PinCoord] = &[
    PinCoord::new(0.155286, 0.507879), // Pin 1
    PinCoord::new(0.155286, 0.538667), // Pin 2
    PinCoord::new(0.155286, 0.569455), // Pin 3
    PinCoord::new(0.155286, 0.600242), // Pin 4
    PinCoord::new(0.155286, 0.63103), // Pin 5
    PinCoord::new(0.155286, 0.661818), // Pin 6
    PinCoord::new(0.155286, 0.692606), // Pin 7
    PinCoord::new(0.155286, 0.723394), // Pin 8
];

/// Normalized (x, y) coordinates for CN8 (Left Arduino Analog In, pins 1..6).
pub const CN8_COORDS: &[PinCoord] = &[
    PinCoord::new(0.155286, 0.78497), // Pin 1
    PinCoord::new(0.155286, 0.815758), // Pin 2
    PinCoord::new(0.155286, 0.846545), // Pin 3
    PinCoord::new(0.155286, 0.877333), // Pin 4
    PinCoord::new(0.155286, 0.908121), // Pin 5
    PinCoord::new(0.155286, 0.938909), // Pin 6
];

/// Normalized (x, y) coordinates for CN5 (Right Arduino Digital High, pins 1..10).
pub const CN5_COORDS: &[PinCoord] = &[
    PinCoord::new(0.844714, 0.674182), // Pin 1
    PinCoord::new(0.844714, 0.643394), // Pin 2
    PinCoord::new(0.844714, 0.612606), // Pin 3
    PinCoord::new(0.844714, 0.581818), // Pin 4
    PinCoord::new(0.844714, 0.55103), // Pin 5
    PinCoord::new(0.844714, 0.520242), // Pin 6
    PinCoord::new(0.844714, 0.489455), // Pin 7
    PinCoord::new(0.844714, 0.458667), // Pin 8
    PinCoord::new(0.844714, 0.427879), // Pin 9
    PinCoord::new(0.844714, 0.397091), // Pin 10
];

/// Normalized (x, y) coordinates for CN9 (Right Arduino Digital Low, pins 1..8).
pub const CN9_COORDS: &[PinCoord] = &[
    PinCoord::new(0.844714, 0.938909), // Pin 1
    PinCoord::new(0.844714, 0.908121), // Pin 2
    PinCoord::new(0.844714, 0.877333), // Pin 3
    PinCoord::new(0.844714, 0.846545), // Pin 4
    PinCoord::new(0.844714, 0.815758), // Pin 5
    PinCoord::new(0.844714, 0.78497), // Pin 6
    PinCoord::new(0.844714, 0.754182), // Pin 7
    PinCoord::new(0.844714, 0.723394), // Pin 8
];


pub fn get_pin_coord(connector: &str, pin_num: u8) -> Option<PinCoord> {
    if pin_num == 0 {
        return None;
    }
    let idx = (pin_num - 1) as usize;
    match connector {
        "CN7" => CN7_COORDS.get(idx).copied(),
        "CN10" => CN10_COORDS.get(idx).copied(),
        "CN6" => CN6_COORDS.get(idx).copied(),
        "CN8" => CN8_COORDS.get(idx).copied(),
        "CN5" => CN5_COORDS.get(idx).copied(),
        "CN9" => CN9_COORDS.get(idx).copied(),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PhysicalPinMapping {
    pub mcu_pin: &'static str,
    pub morpho: (&'static str, u8, PinCoord),
    pub arduino: Option<(&'static str, u8, &'static str, PinCoord)>,
}

pub fn lookup_physical_pin_mapping(mcu_pin: &str) -> Option<PhysicalPinMapping> {
    let loc = stakhal_core::nucleo_pinout::lookup_pin(mcu_pin)?;
    let (m_conn, m_pin) = loc.morpho?;
    let m_coord = get_pin_coord(m_conn, m_pin)?;

    let arduino = if let Some((a_conn, a_pin, a_lbl)) = loc.arduino {
        let a_coord = get_pin_coord(a_conn, a_pin)?;
        Some((a_conn, a_pin, a_lbl, a_coord))
    } else {
        None
    };

    Some(PhysicalPinMapping {
        mcu_pin: loc.mcu_pin,
        morpho: (m_conn, m_pin, m_coord),
        arduino,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_connector_coordinates_in_bounds() {
        for coord in CN7_COORDS {
            assert!(coord.norm_x > 0.0 && coord.norm_x < 1.0);
            assert!(coord.norm_y > 0.0 && coord.norm_y < 1.0);
        }
        for coord in CN10_COORDS {
            assert!(coord.norm_x > 0.0 && coord.norm_x < 1.0);
            assert!(coord.norm_y > 0.0 && coord.norm_y < 1.0);
        }
        for coord in CN6_COORDS {
            assert!(coord.norm_x > 0.0 && coord.norm_x < 1.0);
            assert!(coord.norm_y > 0.0 && coord.norm_y < 1.0);
        }
        for coord in CN8_COORDS {
            assert!(coord.norm_x > 0.0 && coord.norm_x < 1.0);
            assert!(coord.norm_y > 0.0 && coord.norm_y < 1.0);
        }
        for coord in CN5_COORDS {
            assert!(coord.norm_x > 0.0 && coord.norm_x < 1.0);
            assert!(coord.norm_y > 0.0 && coord.norm_y < 1.0);
        }
        for coord in CN9_COORDS {
            assert!(coord.norm_x > 0.0 && coord.norm_x < 1.0);
            assert!(coord.norm_y > 0.0 && coord.norm_y < 1.0);
        }
    }

    #[test]
    fn test_known_shared_pin_pa5_d13() {
        let mapping = lookup_physical_pin_mapping("PA5").expect("PA5 mapping must exist");
        assert_eq!(mapping.morpho.0, "CN10");
        assert_eq!(mapping.morpho.1, 11);

        let (a_conn, a_pin, a_lbl, a_coord) = mapping.arduino.expect("PA5 must have Arduino pin");
        assert_eq!(a_conn, "CN5");
        assert_eq!(a_pin, 6);
        assert_eq!(a_lbl, "D13");

        // Verify that Morpho and Arduino have different coordinates, with vertical offset > 0.015 (1.52mm / 60 mil Arduino gap offset)
        assert_ne!(mapping.morpho.2.norm_x, a_coord.norm_x);
        assert!((mapping.morpho.2.norm_y - a_coord.norm_y).abs() > 0.015);
    }

    #[test]
    fn test_exact_morpho_row_spacing() {
        // Between adjacent rows, vertical distance should be exactly 2.54 / 82.5
        let expected_dy = 2.54 / BOARD_HEIGHT_MM;
        for i in 0..18 {
            let p_a = CN7_COORDS[i * 2];
            let p_b = CN7_COORDS[(i + 1) * 2];
            let dy = p_b.norm_y - p_a.norm_y;
            assert!((dy - expected_dy).abs() < 1e-5);
        }
    }
}
