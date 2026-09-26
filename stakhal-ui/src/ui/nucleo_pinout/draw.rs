use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use gtk4::cairo;
use gtk4::prelude::*;
use crate::state::{AppState, AppWidgets};
use crate::ui::tokens;
use super::coords;

thread_local! {
    static BOARD_SURFACE: std::cell::OnceCell<cairo::ImageSurface> = const { std::cell::OnceCell::new() };
}

pub fn with_board_surface<R>(f: impl FnOnce(&cairo::ImageSurface) -> R) -> R {
    BOARD_SURFACE.with(|cell| {
        let surface = cell.get_or_init(|| {
            let png_bytes = include_bytes!("../../../assets/nucleo_f446re_board.png");
            let mut cursor = std::io::Cursor::new(png_bytes);
            cairo::ImageSurface::create_from_png(&mut cursor)
                .expect("Failed to load nucleo_f446re_board.png")
        });
        f(surface)
    })
}

pub struct PinDef {
    pub pin_num: u8,
    pub mcu_pin: &'static str,
    pub default_label: Option<&'static str>,
}

pub struct ConnectorDef {
    pub name: &'static str,
    pub pins: &'static [PinDef],
}

pub const CN7_PINS: &[PinDef] = &[
    PinDef { pin_num: 1, mcu_pin: "PC10", default_label: None },
    PinDef { pin_num: 2, mcu_pin: "PC11", default_label: None },
    PinDef { pin_num: 3, mcu_pin: "PC12", default_label: None },
    PinDef { pin_num: 4, mcu_pin: "PD2", default_label: None },
    PinDef { pin_num: 5, mcu_pin: "VDD", default_label: Some("3V3") },
    PinDef { pin_num: 6, mcu_pin: "E5V", default_label: Some("5V") },
    PinDef { pin_num: 7, mcu_pin: "BOOT0", default_label: None },
    PinDef { pin_num: 8, mcu_pin: "GND", default_label: None },
    PinDef { pin_num: 9, mcu_pin: "NC", default_label: None },
    PinDef { pin_num: 10, mcu_pin: "NC", default_label: None },
    PinDef { pin_num: 11, mcu_pin: "NC", default_label: None },
    PinDef { pin_num: 12, mcu_pin: "IOREF", default_label: None },
    PinDef { pin_num: 13, mcu_pin: "PA13", default_label: Some("SWDIO") },
    PinDef { pin_num: 14, mcu_pin: "RESET", default_label: None },
    PinDef { pin_num: 15, mcu_pin: "PA14", default_label: Some("SWCLK") },
    PinDef { pin_num: 16, mcu_pin: "+3V3", default_label: None },
    PinDef { pin_num: 17, mcu_pin: "PA15", default_label: None },
    PinDef { pin_num: 18, mcu_pin: "+5V", default_label: None },
    PinDef { pin_num: 19, mcu_pin: "GND", default_label: None },
    PinDef { pin_num: 20, mcu_pin: "GND", default_label: None },
    PinDef { pin_num: 21, mcu_pin: "PB7", default_label: None },
    PinDef { pin_num: 22, mcu_pin: "GND", default_label: None },
    PinDef { pin_num: 23, mcu_pin: "PC13", default_label: Some("USER_BTN") },
    PinDef { pin_num: 24, mcu_pin: "VIN", default_label: None },
    PinDef { pin_num: 25, mcu_pin: "PC14", default_label: Some("OSC32_IN") },
    PinDef { pin_num: 26, mcu_pin: "NC", default_label: None },
    PinDef { pin_num: 27, mcu_pin: "PC15", default_label: Some("OSC32_OUT") },
    PinDef { pin_num: 28, mcu_pin: "PA0", default_label: Some("A0") },
    PinDef { pin_num: 29, mcu_pin: "PH0", default_label: Some("OSC_IN") },
    PinDef { pin_num: 30, mcu_pin: "PA1", default_label: Some("A1") },
    PinDef { pin_num: 31, mcu_pin: "PH1", default_label: Some("OSC_OUT") },
    PinDef { pin_num: 32, mcu_pin: "PA4", default_label: Some("A2") },
    PinDef { pin_num: 33, mcu_pin: "VBAT", default_label: None },
    PinDef { pin_num: 34, mcu_pin: "PB0", default_label: Some("A3") },
    PinDef { pin_num: 35, mcu_pin: "PC2", default_label: None },
    PinDef { pin_num: 36, mcu_pin: "PC1", default_label: Some("A4") },
    PinDef { pin_num: 37, mcu_pin: "PC3", default_label: None },
    PinDef { pin_num: 38, mcu_pin: "PC0", default_label: Some("A5") },
];

pub const CN6_PINS: &[PinDef] = &[
    PinDef { pin_num: 1, mcu_pin: "NC", default_label: None },
    PinDef { pin_num: 2, mcu_pin: "IOREF", default_label: None },
    PinDef { pin_num: 3, mcu_pin: "RESET", default_label: None },
    PinDef { pin_num: 4, mcu_pin: "+3V3", default_label: None },
    PinDef { pin_num: 5, mcu_pin: "+5V", default_label: None },
    PinDef { pin_num: 6, mcu_pin: "GND", default_label: None },
    PinDef { pin_num: 7, mcu_pin: "GND", default_label: None },
    PinDef { pin_num: 8, mcu_pin: "VIN", default_label: None },
];

pub const CN8_PINS: &[PinDef] = &[
    PinDef { pin_num: 1, mcu_pin: "PA0", default_label: Some("A0") },
    PinDef { pin_num: 2, mcu_pin: "PA1", default_label: Some("A1") },
    PinDef { pin_num: 3, mcu_pin: "PA4", default_label: Some("A2") },
    PinDef { pin_num: 4, mcu_pin: "PB0", default_label: Some("A3") },
    PinDef { pin_num: 5, mcu_pin: "PC1", default_label: Some("A4") },
    PinDef { pin_num: 6, mcu_pin: "PC0", default_label: Some("A5") },
];

pub const CN10_PINS: &[PinDef] = &[
    PinDef { pin_num: 1, mcu_pin: "PC9", default_label: None },
    PinDef { pin_num: 2, mcu_pin: "PC8", default_label: None },
    PinDef { pin_num: 3, mcu_pin: "PB8", default_label: Some("D15") },
    PinDef { pin_num: 4, mcu_pin: "PC6", default_label: None },
    PinDef { pin_num: 5, mcu_pin: "PB9", default_label: Some("D14") },
    PinDef { pin_num: 6, mcu_pin: "PC5", default_label: None },
    PinDef { pin_num: 7, mcu_pin: "AVDD", default_label: None },
    PinDef { pin_num: 8, mcu_pin: "U5V", default_label: None },
    PinDef { pin_num: 9, mcu_pin: "GND", default_label: None },
    PinDef { pin_num: 10, mcu_pin: "NC", default_label: None },
    PinDef { pin_num: 11, mcu_pin: "PA5", default_label: Some("D13") },
    PinDef { pin_num: 12, mcu_pin: "PA12", default_label: None },
    PinDef { pin_num: 13, mcu_pin: "PA6", default_label: Some("D12") },
    PinDef { pin_num: 14, mcu_pin: "PA11", default_label: None },
    PinDef { pin_num: 15, mcu_pin: "PA7", default_label: Some("D11") },
    PinDef { pin_num: 16, mcu_pin: "PB12", default_label: None },
    PinDef { pin_num: 17, mcu_pin: "PB6", default_label: Some("D10") },
    PinDef { pin_num: 18, mcu_pin: "NC", default_label: None },
    PinDef { pin_num: 19, mcu_pin: "PC7", default_label: Some("D9") },
    PinDef { pin_num: 20, mcu_pin: "GND", default_label: None },
    PinDef { pin_num: 21, mcu_pin: "PA9", default_label: Some("D8") },
    PinDef { pin_num: 22, mcu_pin: "PB2", default_label: None },
    PinDef { pin_num: 23, mcu_pin: "PA8", default_label: Some("D7") },
    PinDef { pin_num: 24, mcu_pin: "PB1", default_label: None },
    PinDef { pin_num: 25, mcu_pin: "PB10", default_label: Some("D6") },
    PinDef { pin_num: 26, mcu_pin: "PB15", default_label: None },
    PinDef { pin_num: 27, mcu_pin: "PB4", default_label: Some("D5") },
    PinDef { pin_num: 28, mcu_pin: "PB14", default_label: None },
    PinDef { pin_num: 29, mcu_pin: "PB5", default_label: Some("D4") },
    PinDef { pin_num: 30, mcu_pin: "PB13", default_label: None },
    PinDef { pin_num: 31, mcu_pin: "PB3", default_label: Some("D3") },
    PinDef { pin_num: 32, mcu_pin: "AGND", default_label: None },
    PinDef { pin_num: 33, mcu_pin: "PA10", default_label: Some("D2") },
    PinDef { pin_num: 34, mcu_pin: "PC4", default_label: None },
    PinDef { pin_num: 35, mcu_pin: "PA2", default_label: Some("D1") },
    PinDef { pin_num: 36, mcu_pin: "NC", default_label: None },
    PinDef { pin_num: 37, mcu_pin: "PA3", default_label: Some("D0") },
    PinDef { pin_num: 38, mcu_pin: "NC", default_label: None },
];

pub const CN5_PINS: &[PinDef] = &[
    PinDef { pin_num: 1, mcu_pin: "PA9", default_label: Some("D8") },
    PinDef { pin_num: 2, mcu_pin: "PC7", default_label: Some("D9") },
    PinDef { pin_num: 3, mcu_pin: "PB6", default_label: Some("D10") },
    PinDef { pin_num: 4, mcu_pin: "PA7", default_label: Some("D11") },
    PinDef { pin_num: 5, mcu_pin: "PA6", default_label: Some("D12") },
    PinDef { pin_num: 6, mcu_pin: "PA5", default_label: Some("D13") },
    PinDef { pin_num: 7, mcu_pin: "GND", default_label: None },
    PinDef { pin_num: 8, mcu_pin: "AREF", default_label: None },
    PinDef { pin_num: 9, mcu_pin: "PB9", default_label: Some("D14") },
    PinDef { pin_num: 10, mcu_pin: "PB8", default_label: Some("D15") },
];

pub const CN9_PINS: &[PinDef] = &[
    PinDef { pin_num: 1, mcu_pin: "PA3", default_label: Some("D0") },
    PinDef { pin_num: 2, mcu_pin: "PA2", default_label: Some("D1") },
    PinDef { pin_num: 3, mcu_pin: "PA10", default_label: Some("D2") },
    PinDef { pin_num: 4, mcu_pin: "PB3", default_label: Some("D3") },
    PinDef { pin_num: 5, mcu_pin: "PB5", default_label: Some("D4") },
    PinDef { pin_num: 6, mcu_pin: "PB4", default_label: Some("D5") },
    PinDef { pin_num: 7, mcu_pin: "PB10", default_label: Some("D6") },
    PinDef { pin_num: 8, mcu_pin: "PA8", default_label: Some("D7") },
];

pub const CONNECTORS: &[ConnectorDef] = &[
    ConnectorDef { name: "CN7", pins: CN7_PINS },
    ConnectorDef { name: "CN6", pins: CN6_PINS },
    ConnectorDef { name: "CN8", pins: CN8_PINS },
    ConnectorDef { name: "CN10", pins: CN10_PINS },
    ConnectorDef { name: "CN5", pins: CN5_PINS },
    ConnectorDef { name: "CN9", pins: CN9_PINS },
];

#[derive(Debug, Clone)]
pub struct PinHighlightInfo {
    pub signal: String,
    pub label: Option<String>,
    pub modules: Vec<String>,
    pub is_muted: bool,
}

pub fn get_active_pin_highlights(state: &AppState) -> HashMap<(&'static str, u8), PinHighlightInfo> {
    let mut map = HashMap::new();
    let project_guard = state.project.borrow();
    let project = match &project_guard.loaded_project {
        Some(p) => p,
        None => return map,
    };

    let selected_mod = state.with_canvas_state(|c| c.selected_pinout_module.clone());
    let selected_mod = selected_mod.as_deref();

    for pin_cfg in &project.pins {
        let mcu_pin = &pin_cfg.pin;
        let signal = &pin_cfg.signal;
        let label = &pin_cfg.label;
        let modules = &pin_cfg.modules;

        let is_muted = match selected_mod {
            None => false,
            Some(target) => !modules.iter().any(|m| m == target),
        };

        if let Some(loc) = stakhal_core::nucleo_pinout::lookup_pin(mcu_pin) {
            if let Some((conn, pin_num)) = loc.morpho {
                let static_conn = match conn {
                    "CN7" => "CN7",
                    "CN10" => "CN10",
                    _ => conn,
                };
                map.insert((static_conn, pin_num), PinHighlightInfo {
                    signal: signal.clone(),
                    label: label.clone(),
                    modules: modules.clone(),
                    is_muted,
                });
            }
            if let Some((conn, pin_num, _label)) = loc.arduino {
                let static_conn = match conn {
                    "CN5" => "CN5",
                    "CN6" => "CN6",
                    "CN8" => "CN8",
                    "CN9" => "CN9",
                    _ => conn,
                };
                map.insert((static_conn, pin_num), PinHighlightInfo {
                    signal: signal.clone(),
                    label: label.clone(),
                    modules: modules.clone(),
                    is_muted,
                });
            }
        }
    }

    map
}

pub fn lookup_mcu_pin(conn_name: &str, pin_num: u8) -> Option<&'static str> {
    CONNECTORS
        .iter()
        .find(|c| c.name == conn_name)?
        .pins
        .iter()
        .find(|p| p.pin_num == pin_num)
        .map(|p| p.mcu_pin)
}

pub fn is_pin_hovered(
    conn_name: &str,
    pin_num: u8,
    hovered_pin: Option<&(String, u8)>,
) -> bool {
    let Some((h_conn, h_pin)) = hovered_pin else {
        return false;
    };
    if h_conn == conn_name && *h_pin == pin_num {
        return true;
    }
    if let (Some(mcu1), Some(mcu2)) = (lookup_mcu_pin(conn_name, pin_num), lookup_mcu_pin(h_conn, *h_pin)) {
        if !mcu1.is_empty() && mcu1 != "NC" && mcu1 != "GND" && mcu1 != "+3V3" && mcu1 != "+5V" && mcu1 == mcu2 {
            return true;
        }
    }
    false
}

pub fn get_board_rect(canvas_w: f64, canvas_h: f64) -> (f64, f64, f64, f64) {
    let top_reserved = 82.0;
    let bottom_reserved = 46.0;
    let min_gutter_w = 160.0;
    let avail_w = (canvas_w - 2.0 * min_gutter_w).max(200.0);
    let avail_h = (canvas_h - top_reserved - bottom_reserved).max(200.0);

    let board_aspect = 70.0 / 82.5;

    let (board_w, board_h) = if avail_w / avail_h > board_aspect {
        let h = avail_h;
        let w = h * board_aspect;
        (w, h)
    } else {
        let w = avail_w;
        let h = w / board_aspect;
        (w, h)
    };

    let board_x = (canvas_w - board_w) / 2.0;
    let board_y = top_reserved + (avail_h - board_h) / 2.0;
    (board_x, board_y, board_w, board_h)
}

pub fn get_pin_marker_pos(conn_name: &str, pin_num: u8, canvas_w: f64, canvas_h: f64) -> Option<(f64, f64)> {
    let coord = coords::get_pin_coord(conn_name, pin_num)?;
    let (board_x, board_y, board_w, board_h) = get_board_rect(canvas_w, canvas_h);
    let px = board_x + coord.norm_x * board_w;
    let py = board_y + coord.norm_y * board_h;
    Some((px, py))
}

#[derive(Debug, Clone)]
pub struct CalloutBadge {
    pub conn_name: &'static str,
    pub pin_num: u8,
    pub mcu_pin: &'static str,
    pub arduino_label: Option<&'static str>,
    pub signal_text: String,
    pub is_muted: bool,
    pub color: (f64, f64, f64),
    pub is_left: bool,
    pub morpho_pos: (f64, f64),
    pub arduino_pos: Option<(f64, f64)>,
    pub route_pos: (f64, f64),
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

pub fn compute_callout_badges(
    canvas_w: f64,
    canvas_h: f64,
    state: &AppState,
) -> (Vec<CalloutBadge>, Vec<CalloutBadge>) {
    let (board_x, board_y, board_w, board_h) = get_board_rect(canvas_w, canvas_h);

    let project_guard = state.project.borrow();
    let project = match &project_guard.loaded_project {
        Some(p) => p,
        None => return (Vec::new(), Vec::new()),
    };

    let selected_mod = state.with_canvas_state(|c| c.selected_pinout_module.clone());
    let selected_mod = selected_mod.as_deref();

    let mut raw_callouts = Vec::new();
    let mut seen_mcu = std::collections::HashSet::new();

    for pin_cfg in &project.pins {
        let mcu_pin = pin_cfg.pin.as_str();
        if !seen_mcu.insert(mcu_pin.to_string()) {
            continue;
        }

        let Some(mapping) = coords::lookup_physical_pin_mapping(mcu_pin) else {
            continue;
        };

        let (m_conn, m_pin, m_coord) = mapping.morpho;
        let m_pos = (board_x + m_coord.norm_x * board_w, board_y + m_coord.norm_y * board_h);

        let a_pos = mapping.arduino.map(|(a_conn, a_pin, _lbl, a_coord)| {
            let _ = (a_conn, a_pin);
            (board_x + a_coord.norm_x * board_w, board_y + a_coord.norm_y * board_h)
        });

        let ard_label = mapping.arduino.map(|(_, _, lbl, _)| lbl);

        let is_muted = match selected_mod {
            None => false,
            Some(target) => !pin_cfg.modules.iter().any(|m| m == target),
        };

        let reserved = stakhal_core::nucleo_pinout::check_reserved(mcu_pin);
        let color = match reserved {
            Some(res) => match res.severity {
                stakhal_core::nucleo_pinout::ReservedSeverity::Critical => tokens::color::STATE_ERROR,
                stakhal_core::nucleo_pinout::ReservedSeverity::Caution => tokens::color::STATE_ACTIVE,
            },
            None => tokens::color::STATE_READY,
        };

        let sig_text = match &pin_cfg.label {
            Some(lbl) => {
                if lbl.ends_with("_Pin") {
                    lbl.clone()
                } else {
                    format!("{}_Pin", lbl)
                }
            }
            None => pin_cfg.signal.clone(),
        };

        let is_left = m_coord.norm_x < 0.5;
        let route_pos = m_pos;

        raw_callouts.push((
            m_conn,
            m_pin,
            mapping.mcu_pin,
            ard_label,
            sig_text,
            is_muted,
            color,
            is_left,
            m_pos,
            a_pos,
            route_pos,
        ));
    }

    let mut left_raw: Vec<_> = raw_callouts.iter().filter(|c| c.7).cloned().collect();
    let mut right_raw: Vec<_> = raw_callouts.iter().filter(|c| !c.7).cloned().collect();

    left_raw.sort_by(|a, b| a.10.1.partial_cmp(&b.10.1).unwrap_or(std::cmp::Ordering::Equal));
    right_raw.sort_by(|a, b| a.10.1.partial_cmp(&b.10.1).unwrap_or(std::cmp::Ordering::Equal));

    let badge_h = 22.0;
    let spacing = badge_h + 4.0;
    let min_y = board_y + 4.0;
    let max_y = (board_y + board_h - badge_h - 4.0).max(min_y);

    let layout_side = |items: &[(
        &'static str,
        u8,
        &'static str,
        Option<&'static str>,
        String,
        bool,
        (f64, f64, f64),
        bool,
        (f64, f64),
        Option<(f64, f64)>,
        (f64, f64),
    )], is_left: bool| -> Vec<CalloutBadge> {
        if items.is_empty() {
            return Vec::new();
        }

        let mut ys: Vec<f64> = items.iter().map(|item| (item.10.1 - badge_h / 2.0).clamp(min_y, max_y)).collect();

        for i in 1..ys.len() {
            if ys[i] < ys[i - 1] + spacing {
                ys[i] = ys[i - 1] + spacing;
            }
        }
        if let Some(last) = ys.last_mut() {
            if *last > max_y {
                *last = max_y;
            }
        }
        for i in (0..ys.len().saturating_sub(1)).rev() {
            if ys[i] > ys[i + 1] - spacing {
                ys[i] = ys[i + 1] - spacing;
            }
        }
        for y in &mut ys {
            *y = y.clamp(min_y, max_y);
        }

        let mut badges = Vec::with_capacity(items.len());
        for (idx, item) in items.iter().enumerate() {
            let pin_id_str = match item.3 {
                Some(ard) => format!("{} / {}", item.2, ard),
                None => item.2.to_string(),
            };
            let text_char_len = pin_id_str.len() + 3 + item.4.len();
            let est_w = ((text_char_len as f64) * 7.2 + 20.0).clamp(110.0, 240.0);

            let (bx, by, bw) = if is_left {
                let badge_right = board_x - 14.0;
                let badge_x = (badge_right - est_w).max(12.0);
                let actual_w = badge_right - badge_x;
                (badge_x, ys[idx], actual_w)
            } else {
                let badge_left = board_x + board_w + 14.0;
                let max_w = (canvas_w - 12.0 - badge_left).max(60.0);
                let actual_w = est_w.min(max_w);
                (badge_left, ys[idx], actual_w)
            };

            badges.push(CalloutBadge {
                conn_name: item.0,
                pin_num: item.1,
                mcu_pin: item.2,
                arduino_label: item.3,
                signal_text: item.4.clone(),
                is_muted: item.5,
                color: item.6,
                is_left: item.7,
                morpho_pos: item.8,
                arduino_pos: item.9,
                route_pos: item.10,
                x: bx,
                y: by,
                w: bw,
                h: badge_h,
            });
        }
        badges
    };

    let left_badges = layout_side(&left_raw, true);
    let right_badges = layout_side(&right_raw, false);

    (left_badges, right_badges)
}

pub fn find_hit_pin(
    x: f64,
    y: f64,
    canvas_w: f64,
    canvas_h: f64,
    state: &AppState,
) -> Option<(&'static str, u8)> {
    let (board_x, board_y, board_w, board_h) = get_board_rect(canvas_w, canvas_h);
    let hit_radius = 12.0;
    let hit_radius_sq = hit_radius * hit_radius;

    let mut closest_pin: Option<(&'static str, u8, f64)> = None;

    // 1. Check all physical pin markers
    for conn in CONNECTORS {
        for p in conn.pins {
            if let Some(coord) = coords::get_pin_coord(conn.name, p.pin_num) {
                let px = board_x + coord.norm_x * board_w;
                let py = board_y + coord.norm_y * board_h;
                let d2 = (x - px) * (x - px) + (y - py) * (y - py);
                if d2 <= hit_radius_sq {
                    match closest_pin {
                        Some((_, _, best_d2)) if d2 < best_d2 => {
                            closest_pin = Some((conn.name, p.pin_num, d2));
                        }
                        None => {
                            closest_pin = Some((conn.name, p.pin_num, d2));
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    if let Some((c, p, _)) = closest_pin {
        return Some((c, p));
    }

    // 2. Check gutter callout badges for active pins
    let (left_badges, right_badges) = compute_callout_badges(canvas_w, canvas_h, state);
    for badge in left_badges.iter().chain(right_badges.iter()) {
        if x >= badge.x && x <= badge.x + badge.w && y >= badge.y && y <= badge.y + badge.h {
            return Some((badge.conn_name, badge.pin_num));
        }
    }

    None
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
    let (hovered_pin, hovered_mouse, is_filtering_module) = state.borrow().with_canvas_state(|c| {
        (c.hovered_pinout_pin.clone(), c.hovered_pinout_mouse, c.selected_pinout_module.is_some())
    });
    let highlights = get_active_pin_highlights(&state.borrow());
    let hovered_pin = hovered_pin.as_ref();

    let active_conflicts: Vec<(&'static str, &stakhal_core::nucleo_pinout::ReservedPin)> = {
        let mut list = Vec::new();
        let proj = Rc::clone(&state.borrow().project);
        let proj_guard = proj.borrow();
        if let Some(project) = &proj_guard.loaded_project {
            for pin_cfg in &project.pins {
                if let Some(res) = stakhal_core::nucleo_pinout::check_reserved(&pin_cfg.pin) {
                    if !list.iter().any(|(p, _)| *p == res.mcu_pin) {
                        list.push((res.mcu_pin, res));
                    }
                }
            }
        }
        list
    };

    let canvas_w = width.max(800.0);
    let canvas_h = height.max(600.0);
    let (board_x, board_y, board_w, board_h) = get_board_rect(canvas_w, canvas_h);

    // 1. Canvas Background
    cr.set_source_rgb(tokens::color::BG_VOID.0, tokens::color::BG_VOID.1, tokens::color::BG_VOID.2);
    cr.rectangle(0.0, 0.0, canvas_w, canvas_h);
    let _ = cr.fill();

    // 2. Header Silkscreen Title & Subtitle
    cr.select_font_face(tokens::font::CAIRO_SANS, cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(15.0);
    cr.set_source_rgb(tokens::color::TEXT_PRIMARY.0, tokens::color::TEXT_PRIMARY.1, tokens::color::TEXT_PRIMARY.2);
    let title_str = "STMicroelectronics NUCLEO-F446RE";
    let title_y = 26.0;
    if let Ok(ext) = cr.text_extents(title_str) {
        let _ = cr.move_to((canvas_w - ext.width()) / 2.0, title_y);
        let _ = cr.show_text(title_str);
    }

    cr.select_font_face(tokens::font::CAIRO_MONO, cairo::FontSlant::Normal, cairo::FontWeight::Normal);
    cr.set_font_size(10.0);
    cr.set_source_rgb(tokens::color::TEXT_MUTED.0, tokens::color::TEXT_MUTED.1, tokens::color::TEXT_MUTED.2);
    let subtitle_str = "ARM® Cortex®-M4 MCU @ 180MHz • Physical Board Pinout Overlay";
    let subtitle_y = title_y + 17.0;
    if let Ok(ext) = cr.text_extents(subtitle_str) {
        let _ = cr.move_to((canvas_w - ext.width()) / 2.0, subtitle_y);
        let _ = cr.show_text(subtitle_str);
    }

    // Conflict Banner if detected
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
            format!("! {} pin conflicts detected • see highlighted pins below", count)
        };

        cr.select_font_face(tokens::font::CAIRO_SANS, cairo::FontSlant::Normal, cairo::FontWeight::Bold);
        cr.set_font_size(10.5);
        cr.set_source_rgb(cr_r, cr_g, cr_b);
        if let Ok(ext) = cr.text_extents(&banner_str) {
            let banner_y = subtitle_y + 17.0;
            let _ = cr.move_to((canvas_w - ext.width()) / 2.0, banner_y);
            let _ = cr.show_text(&banner_str);
        }
    }

    // 3. Render Cached Board Background Surface
    with_board_surface(|board_surface| {
        let img_w = board_surface.width() as f64;
        let img_h = board_surface.height() as f64;

        // Soft drop shadow behind board
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.35);
        cr.rectangle(board_x - 3.0, board_y - 2.0, board_w + 6.0, board_h + 6.0);
        let _ = cr.fill();

        if let Ok(()) = cr.save() {
            let _ = cr.translate(board_x, board_y);
            let _ = cr.scale(board_w / img_w, board_h / img_h);
            cr.set_source_surface(board_surface, 0.0, 0.0).ok();
            let _ = cr.paint();
            let _ = cr.restore();
        }
    });

    // Crisp framing hairline around board
    cr.set_source_rgba(tokens::color::BORDER_HAIR.0, tokens::color::BORDER_HAIR.1, tokens::color::BORDER_HAIR.2, 0.4);
    cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
    cr.rectangle(board_x, board_y, board_w, board_h);
    let _ = cr.stroke();

    // 4. Compute Gutter Callout Badges
    let (left_badges, right_badges) = compute_callout_badges(canvas_w, canvas_h, &state.borrow());

    // 5. Dual-Identity Linking Hairlines Pass
    for badge in left_badges.iter().chain(right_badges.iter()) {
        if let Some(a_pos) = badge.arduino_pos {
            let (mx, my) = badge.morpho_pos;
            let (ax, ay) = a_pos;

            let is_hovered = is_pin_hovered(badge.conn_name, badge.pin_num, hovered_pin);

            let (r, g, b, alpha, width) = if is_hovered {
                (tokens::color::ACCENT.0, tokens::color::ACCENT.1, tokens::color::ACCENT.2, 1.0, 1.5)
            } else if badge.is_muted {
                (badge.color.0, badge.color.1, badge.color.2, 0.35, 1.0)
            } else {
                (badge.color.0, badge.color.1, badge.color.2, 0.70, 1.0)
            };

            cr.set_source_rgba(r, g, b, alpha);
            cr.set_line_width(width);
            let _ = cr.move_to(mx, my);
            let _ = cr.line_to(ax, ay);
            let _ = cr.stroke();
        }
    }

    // 6. Active Pin Markers Pass (Morpho dot + Arduino dot)
    let marker_r = 4.2;
    for badge in left_badges.iter().chain(right_badges.iter()) {
        let is_hovered = is_pin_hovered(badge.conn_name, badge.pin_num, hovered_pin);

        let (fill_r, fill_g, fill_b, alpha) = if is_hovered {
            (tokens::color::ACCENT.0, tokens::color::ACCENT.1, tokens::color::ACCENT.2, 1.0)
        } else if badge.is_muted {
            (badge.color.0, badge.color.1, badge.color.2, 0.35)
        } else {
            (badge.color.0, badge.color.1, badge.color.2, 1.0)
        };

        let current_r = if is_hovered { marker_r + 1.2 } else { marker_r };

        // Morpho marker
        let (mx, my) = badge.morpho_pos;
        cr.set_source_rgba(fill_r, fill_g, fill_b, alpha);
        cr.arc(mx, my, current_r, 0.0, 2.0 * std::f64::consts::PI);
        let _ = cr.fill_preserve();
        cr.set_source_rgba(tokens::color::BG_VOID.0, tokens::color::BG_VOID.1, tokens::color::BG_VOID.2, alpha);
        cr.set_line_width(1.0);
        let _ = cr.stroke();

        // Arduino marker (if dual-identity)
        if let Some((ax, ay)) = badge.arduino_pos {
            cr.set_source_rgba(fill_r, fill_g, fill_b, alpha);
            cr.arc(ax, ay, current_r, 0.0, 2.0 * std::f64::consts::PI);
            let _ = cr.fill_preserve();
            cr.set_source_rgba(tokens::color::BG_VOID.0, tokens::color::BG_VOID.1, tokens::color::BG_VOID.2, alpha);
            cr.set_line_width(1.0);
            let _ = cr.stroke();
        }
    }

    // 7. Hover Marker for Passive / Unassigned Pins
    if let Some((h_conn, h_pin)) = hovered_pin {
        let is_already_active = highlights.contains_key(&(h_conn.as_str(), *h_pin));
        if !is_already_active {
            if let Some((hx, hy)) = get_pin_marker_pos(h_conn, *h_pin, canvas_w, canvas_h) {
                cr.set_source_rgb(tokens::color::ACCENT.0, tokens::color::ACCENT.1, tokens::color::ACCENT.2);
                cr.arc(hx, hy, 5.0, 0.0, 2.0 * std::f64::consts::PI);
                let _ = cr.fill_preserve();
                cr.set_source_rgb(tokens::color::TEXT_PRIMARY.0, tokens::color::TEXT_PRIMARY.1, tokens::color::TEXT_PRIMARY.2);
                cr.set_line_width(1.0);
                let _ = cr.stroke();
            }
        }
    }

    // 8. Gutter Callouts & Leader Lines Pass
    for badge in left_badges.iter().chain(right_badges.iter()) {
        let is_hovered = is_pin_hovered(badge.conn_name, badge.pin_num, hovered_pin);

        let (border_r, border_g, border_b, alpha, border_w) = if is_hovered {
            (tokens::color::ACCENT.0, tokens::color::ACCENT.1, tokens::color::ACCENT.2, 1.0, 1.5)
        } else if badge.is_muted {
            (badge.color.0, badge.color.1, badge.color.2, 0.35, 1.0)
        } else {
            (badge.color.0, badge.color.1, badge.color.2, 0.85, 1.0)
        };

        // Leader line
        let (px, py) = badge.route_pos;
        let badge_target_y = badge.y + badge.h / 2.0;

        cr.set_source_rgba(border_r, border_g, border_b, if is_hovered { 1.0 } else if badge.is_muted { 0.35 } else { 0.65 });
        cr.set_line_width(if is_hovered { 1.2 } else { 1.0 });

        if badge.is_left {
            let dogleg_x = (board_x - 6.0).max(badge.x + badge.w + 2.0);
            let _ = cr.move_to(px, py);
            let _ = cr.line_to(dogleg_x, py);
            let _ = cr.line_to(badge.x + badge.w, badge_target_y);
            let _ = cr.stroke();
        } else {
            let dogleg_x = (board_x + board_w + 6.0).min(badge.x - 2.0);
            let _ = cr.move_to(px, py);
            let _ = cr.line_to(dogleg_x, py);
            let _ = cr.line_to(badge.x, badge_target_y);
            let _ = cr.stroke();
        }

        // Small anchor dot at marker
        cr.set_source_rgba(border_r, border_g, border_b, alpha);
        cr.arc(px, py, 2.0, 0.0, 2.0 * std::f64::consts::PI);
        let _ = cr.fill();

        // Badge Box Background & Border (sharp 0px corners)
        cr.set_source_rgb(tokens::color::BG_PANEL.0, tokens::color::BG_PANEL.1, tokens::color::BG_PANEL.2);
        cr.rectangle(badge.x, badge.y, badge.w, badge.h);
        let _ = cr.fill_preserve();

        cr.set_source_rgba(border_r, border_g, border_b, alpha);
        cr.set_line_width(border_w);
        let _ = cr.stroke();

        // Badge Text: Pin ID • Signal/Label
        let pin_id_str = match badge.arduino_label {
            Some(ard) => format!("{} / {}", badge.mcu_pin, ard),
            None => badge.mcu_pin.to_string(),
        };

        cr.select_font_face(tokens::font::CAIRO_MONO, cairo::FontSlant::Normal, cairo::FontWeight::Bold);
        cr.set_font_size(9.5);

        let text_y = badge.y + badge.h * 0.68;
        let mut cur_x = badge.x + 8.0;

        // Pin ID
        if badge.is_muted {
            cr.set_source_rgb(tokens::color::TEXT_MUTED.0, tokens::color::TEXT_MUTED.1, tokens::color::TEXT_MUTED.2);
        } else {
            cr.set_source_rgb(tokens::color::TEXT_PRIMARY.0, tokens::color::TEXT_PRIMARY.1, tokens::color::TEXT_PRIMARY.2);
        }
        let _ = cr.move_to(cur_x, text_y);
        let _ = cr.show_text(&pin_id_str);
        if let Ok(ext) = cr.text_extents(&pin_id_str) {
            cur_x += ext.x_advance();
        }

        // Bullet
        cr.set_source_rgb(tokens::color::TEXT_MUTED.0, tokens::color::TEXT_MUTED.1, tokens::color::TEXT_MUTED.2);
        let bullet = " • ";
        let _ = cr.move_to(cur_x, text_y);
        let _ = cr.show_text(bullet);
        if let Ok(ext) = cr.text_extents(bullet) {
            cur_x += ext.x_advance();
        }

        // Signal / Label
        if is_hovered {
            cr.set_source_rgb(tokens::color::ACCENT.0, tokens::color::ACCENT.1, tokens::color::ACCENT.2);
        } else if badge.is_muted {
            cr.set_source_rgb(tokens::color::TEXT_MUTED.0, tokens::color::TEXT_MUTED.1, tokens::color::TEXT_MUTED.2);
        } else {
            cr.set_source_rgb(badge.color.0, badge.color.1, badge.color.2);
        }
        let _ = cr.move_to(cur_x, text_y);
        let _ = cr.show_text(&badge.signal_text);
    }

    // 9. Footer Legend Bar
    let legend_y = canvas_h - 24.0;
    cr.select_font_face(tokens::font::CAIRO_SANS, cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(10.0);

    let mut leg_x = board_x;

    // Active Signal in Loaded Project
    cr.set_source_rgb(tokens::color::STATE_READY.0, tokens::color::STATE_READY.1, tokens::color::STATE_READY.2);
    cr.rectangle(leg_x, legend_y - 9.0, 11.0, 11.0);
    let _ = cr.fill();

    cr.set_source_rgb(tokens::color::TEXT_PRIMARY.0, tokens::color::TEXT_PRIMARY.1, tokens::color::TEXT_PRIMARY.2);
    let active_label = if is_filtering_module {
        "Active in Module"
    } else {
        "Active Signal in Loaded Project"
    };
    let _ = cr.move_to(leg_x + 16.0, legend_y);
    let _ = cr.show_text(active_label);
    if let Ok(ext) = cr.text_extents(active_label) {
        leg_x += ext.width() + 32.0;
    }

    if is_filtering_module {
        cr.set_source_rgb(tokens::color::BG_PANEL.0, tokens::color::BG_PANEL.1, tokens::color::BG_PANEL.2);
        cr.rectangle(leg_x, legend_y - 9.0, 11.0, 11.0);
        let _ = cr.fill_preserve();
        cr.set_source_rgba(tokens::color::STATE_READY.0, tokens::color::STATE_READY.1, tokens::color::STATE_READY.2, 0.35);
        cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
        let _ = cr.stroke();

        cr.set_source_rgb(tokens::color::TEXT_MUTED.0, tokens::color::TEXT_MUTED.1, tokens::color::TEXT_MUTED.2);
        let _ = cr.move_to(leg_x + 16.0, legend_y);
        let _ = cr.show_text("Active Elsewhere in Project");
        if let Ok(ext) = cr.text_extents("Active Elsewhere in Project") {
            leg_x += ext.width() + 32.0;
        }
    }

    if !active_conflicts.is_empty() {
        let has_critical = active_conflicts
            .iter()
            .any(|(_, r)| r.severity == stakhal_core::nucleo_pinout::ReservedSeverity::Critical);
        let (cr_r, cr_g, cr_b) = if has_critical {
            tokens::color::STATE_ERROR
        } else {
            tokens::color::STATE_ACTIVE
        };
        cr.set_source_rgb(cr_r, cr_g, cr_b);
        cr.rectangle(leg_x, legend_y - 9.0, 11.0, 11.0);
        let _ = cr.fill();

        cr.set_source_rgb(tokens::color::TEXT_PRIMARY.0, tokens::color::TEXT_PRIMARY.1, tokens::color::TEXT_PRIMARY.2);
        let _ = cr.move_to(leg_x + 16.0, legend_y);
        let _ = cr.show_text("Pin Conflict");
        if let Ok(ext) = cr.text_extents("Pin Conflict") {
            leg_x += ext.width() + 32.0;
        }
    }
    cr.set_source_rgba(tokens::color::TEXT_PRIMARY.0, tokens::color::TEXT_PRIMARY.1, tokens::color::TEXT_PRIMARY.2, 0.7);
    cr.set_line_width(1.2);
    let _ = cr.move_to(leg_x, legend_y - 3.5);
    let _ = cr.line_to(leg_x + 16.0, legend_y - 3.5);
    let _ = cr.stroke();
    cr.arc(leg_x, legend_y - 3.5, 2.5, 0.0, 2.0 * std::f64::consts::PI);
    let _ = cr.fill();
    cr.arc(leg_x + 16.0, legend_y - 3.5, 2.5, 0.0, 2.0 * std::f64::consts::PI);
    let _ = cr.fill();

    cr.set_source_rgb(tokens::color::TEXT_MUTED.0, tokens::color::TEXT_MUTED.1, tokens::color::TEXT_MUTED.2);
    let _ = cr.move_to(leg_x + 22.0, legend_y);
    let _ = cr.show_text("Dual Identity (Morpho + Arduino)");

    // 10. FINAL PASS: Compact Floating Tooltip Card
    if let (Some((conn_name, pin_num)), Some((mx, my))) = (hovered_pin, hovered_mouse) {
        if let Some(conn) = CONNECTORS.iter().find(|c| c.name == conn_name) {
            if let Some(p) = conn.pins.iter().find(|p| p.pin_num == *pin_num) {
                let default_lbl_str = p.default_label.map(|l| format!(" ({})", l)).unwrap_or_default();
                let line1 = format!("{}-{} : {}{}", conn_name, pin_num, p.mcu_pin, default_lbl_str);

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

                let reserved_info = if is_hl.is_some() {
                    stakhal_core::nucleo_pinout::check_reserved(p.mcu_pin)
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

                cr.select_font_face(tokens::font::CAIRO_MONO, cairo::FontSlant::Normal, cairo::FontWeight::Bold);
                cr.set_font_size(10.0);
                let w1 = cr.text_extents(&line1).map(|e| e.width()).unwrap_or(120.0);

                cr.select_font_face(tokens::font::CAIRO_MONO, cairo::FontSlant::Normal, cairo::FontWeight::Bold);
                cr.set_font_size(9.5);
                let w2 = cr.text_extents(&line2).map(|e| e.width()).unwrap_or(120.0);

                let w3 = line3
                    .as_ref()
                    .map(|l3| cr.text_extents(l3).map(|e| e.width()).unwrap_or(120.0))
                    .unwrap_or(0.0);

                let tt_w = (w1.max(w2).max(w3) + 24.0).max(160.0);
                let tt_h = if line3.is_some() { 58.0 } else { 42.0 };

                let tt_x = if mx > canvas_w / 2.0 {
                    (mx - tt_w - 12.0).max(15.0)
                } else {
                    (mx + 12.0).min(canvas_w - tt_w - 15.0)
                };

                let mut tt_y = my - (tt_h + 6.0);
                if tt_y < 15.0 {
                    tt_y = my + 14.0;
                }
                if tt_y + tt_h > canvas_h - 15.0 {
                    tt_y = (my - tt_h - 8.0).max(15.0);
                }

                cr.set_source_rgb(tokens::color::BG_PANEL.0, tokens::color::BG_PANEL.1, tokens::color::BG_PANEL.2);
                cr.rectangle(tt_x, tt_y, tt_w, tt_h);
                let _ = cr.fill_preserve();

                let (border_r, border_g, border_b) = match reserved_info {
                    Some(res) => match res.severity {
                        stakhal_core::nucleo_pinout::ReservedSeverity::Critical => tokens::color::STATE_ERROR,
                        stakhal_core::nucleo_pinout::ReservedSeverity::Caution => tokens::color::STATE_ACTIVE,
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

                cr.select_font_face(tokens::font::CAIRO_MONO, cairo::FontSlant::Normal, cairo::FontWeight::Bold);
                cr.set_font_size(10.0);
                cr.set_source_rgb(tokens::color::TEXT_PRIMARY.0, tokens::color::TEXT_PRIMARY.1, tokens::color::TEXT_PRIMARY.2);
                let _ = cr.move_to(tt_x + 10.0, tt_y + 16.0);
                let _ = cr.show_text(&line1);

                cr.select_font_face(tokens::font::CAIRO_MONO, cairo::FontSlant::Normal, cairo::FontWeight::Bold);
                cr.set_font_size(9.5);
                if is_hl.is_some() {
                    cr.set_source_rgb(border_r, border_g, border_b);
                } else {
                    cr.set_source_rgb(tokens::color::TEXT_MUTED.0, tokens::color::TEXT_MUTED.1, tokens::color::TEXT_MUTED.2);
                }
                let _ = cr.move_to(tt_x + 10.0, tt_y + 32.0);
                let _ = cr.show_text(&line2);

                if let Some(ref l3) = line3 {
                    cr.select_font_face(tokens::font::CAIRO_MONO, cairo::FontSlant::Normal, cairo::FontWeight::Bold);
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
    widgets.pinout_drawing_area.set_draw_func(move |area, cr, w, h| {
        draw_nucleo_pinout_canvas(area, cr, w as f64, h as f64, &state_draw);
    });

    let motion = gtk4::EventControllerMotion::new();
    let state_motion = Rc::clone(state);
    let widgets_motion = Rc::clone(widgets);

    motion.connect_motion(move |_, x, y| {
        let area = &widgets_motion.pinout_drawing_area;
        let cw = area.width().max(800) as f64;
        let ch = area.height().max(600) as f64;

        let hit_pin = find_hit_pin(x, y, cw, ch, &state_motion.borrow());

        match hit_pin {
            Some((conn_name, pin_num)) => {
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
    fn test_pin_marker_bounds() {
        let (bx, by, bw, bh) = get_board_rect(1280.0, 820.0);
        assert!(bw > 300.0);
        assert!(bh > 300.0);

        for conn in CONNECTORS {
            for p in conn.pins {
                let pos = get_pin_marker_pos(conn.name, p.pin_num, 1280.0, 820.0);
                assert!(pos.is_some(), "Position must exist for {}-{}", conn.name, p.pin_num);
                let (px, py) = pos.unwrap();
                assert!(px >= bx && px <= bx + bw, "Marker X {px} out of board for {}-{}", conn.name, p.pin_num);
                assert!(py >= by && py <= by + bh, "Marker Y {py} out of board for {}-{}", conn.name, p.pin_num);
            }
        }
    }

    #[test]
    fn test_draw_nucleo_pinout_canvas_rendering() {
        let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 1280, 820).expect("Failed to create surface");
        let cr = cairo::Context::new(&surface).expect("Failed to create context");
        let state = Rc::new(RefCell::new(AppState::default()));

        let fixture_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../stakhal-core/tests/fixtures/stakhal_blink_f446re");
        let ioc_path = fixture_dir.join("stakhal_blink_f446re.ioc");
        let main_c_path = fixture_dir.join("Core/Src/main.c");
        if let Ok(project) = stakhal_core::ir::schema::load_project(&ioc_path, &main_c_path) {
            state.borrow().project.borrow_mut().loaded_project = Some(project);
        }

        draw_nucleo_pinout(&cr, 1280.0, 820.0, &state);
        surface.flush();

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

        draw_nucleo_pinout(&cr, 1280.0, 820.0, &state);
        surface.flush();
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
        hatch_state.project.borrow_mut().loaded_project = state.project.borrow().loaded_project.clone();
        let hatch_hl = get_active_pin_highlights(&hatch_state);

        // Find GRIP_IN1
        let grip = hatch_hl.values().find(|h| h.label.as_deref() == Some("GRIP_IN1")).expect("GRIP_IN1 missing");
        assert!(!grip.is_muted, "GRIP_IN1 should not be muted for hatch module");
        assert!(grip.modules.contains(&"hatch".to_string()));

        // Find Z1_LIMIT
        let z1 = hatch_hl.values().find(|h| h.label.as_deref() == Some("Z1_LIMIT")).expect("Z1_LIMIT missing");
        assert!(z1.is_muted, "Z1_LIMIT should be muted when hatch is selected");
        assert!(z1.modules.contains(&"alignment".to_string()));

        // Render to canvas to verify drawing with module filter doesn't panic
        let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 1280, 820).expect("Failed to create surface");
        let cr = cairo::Context::new(&surface).expect("Failed to create context");
        let rc_state = Rc::new(RefCell::new(hatch_state));
        draw_nucleo_pinout(&cr, 1280.0, 820.0, &rc_state);
        surface.flush();
    }

    #[test]
    fn test_hit_testing_radius_and_badges() {
        let fixture_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../stakhal-core/tests/fixtures/aa_ns_stm_port");
        let ioc_path = fixture_dir.join("aa_ns_stm_port.ioc");
        let main_c_path = fixture_dir.join("Core/Src/main.c");
        let project = stakhal_core::ir::schema::load_project(&ioc_path, &main_c_path)
            .expect("Failed to load aa_ns_stm_port");

        let state = AppState::default();
        state.project.borrow_mut().loaded_project = Some(project);

        // Hit testing at exact pin marker position
        let (px, py) = get_pin_marker_pos("CN10", 11, 1280.0, 820.0).unwrap();
        let hit = find_hit_pin(px, py, 1280.0, 820.0, &state);
        assert_eq!(hit, Some(("CN10", 11)));

        // Hit testing within 5px of marker
        let hit_near = find_hit_pin(px + 4.0, py - 3.0, 1280.0, 820.0, &state);
        assert_eq!(hit_near, Some(("CN10", 11)));

        // Hit testing far away from any pin
        let hit_far = find_hit_pin(10.0, 10.0, 1280.0, 820.0, &state);
        assert_eq!(hit_far, None);
    }

    #[test]
    fn test_render_board_overlay_verification_screenshot() {
        let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 1280, 820).expect("Failed to create surface");
        let cr = cairo::Context::new(&surface).expect("Failed to create context");

        let fixture_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../stakhal-core/tests/fixtures/aa_ns_stm_port");
        let ioc_path = fixture_dir.join("aa_ns_stm_port.ioc");
        let main_c_path = fixture_dir.join("Core/Src/main.c");
        let project = stakhal_core::ir::schema::load_project(&ioc_path, &main_c_path)
            .expect("Failed to load aa_ns_stm_port");

        let mut project = project;
        project.pins.push(stakhal_core::ioc::parser::PinConfig {
            pin: "PA13".to_string(),
            signal: "GPIO_Output".to_string(),
            label: Some("DBG_SWDIO".to_string()),
            modules: vec!["hatch".to_string()],
        });
        project.pins.push(stakhal_core::ioc::parser::PinConfig {
            pin: "PC14".to_string(),
            signal: "GPIO_Input".to_string(),
            label: Some("RTC_IN".to_string()),
            modules: Vec::new(),
        });

        let (hover_px, hover_py) = get_pin_marker_pos("CN10", 11, 1280.0, 820.0).unwrap();

        let state = AppState::default();
        state.project.borrow_mut().loaded_project = Some(project);
        state.with_canvas_state_mut(|c| {
            c.selected_pinout_module = Some("hatch".to_string());
            c.hovered_pinout_pin = Some(("CN10".to_string(), 11)); // PA5 (D13)
            c.hovered_pinout_mouse = Some((hover_px, hover_py));
        });

        let rc_state = Rc::new(RefCell::new(state));
        draw_nucleo_pinout(&cr, 1280.0, 820.0, &rc_state);
        surface.flush();

        let out_path = "/home/stakxx002/.gemini/antigravity-ide/brain/e21edbbd-844e-44ef-9dfa-1af3c8e3a19b/pinout_board_overlay_verification.png";
        let mut file = std::fs::File::create(out_path).expect("Failed to create output PNG");
        surface.write_to_png(&mut file).expect("Failed to write PNG");
    }
}
