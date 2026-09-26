use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use gtk4::cairo;
use gtk4::prelude::*;
use crate::state::{AppState, AppWidgets};
use crate::ui::tokens;

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

const CN6_PINS: &[PinDef] = &[
    PinDef { pin_num: 1, mcu_pin: "NC", default_label: None },
    PinDef { pin_num: 2, mcu_pin: "IOREF", default_label: None },
    PinDef { pin_num: 3, mcu_pin: "RESET", default_label: None },
    PinDef { pin_num: 4, mcu_pin: "+3V3", default_label: None },
    PinDef { pin_num: 5, mcu_pin: "+5V", default_label: None },
    PinDef { pin_num: 6, mcu_pin: "GND", default_label: None },
    PinDef { pin_num: 7, mcu_pin: "GND", default_label: None },
    PinDef { pin_num: 8, mcu_pin: "VIN", default_label: None },
];

const CN8_PINS: &[PinDef] = &[
    PinDef { pin_num: 1, mcu_pin: "PA0", default_label: Some("A0") },
    PinDef { pin_num: 2, mcu_pin: "PA1", default_label: Some("A1") },
    PinDef { pin_num: 3, mcu_pin: "PA4", default_label: Some("A2") },
    PinDef { pin_num: 4, mcu_pin: "PB0", default_label: Some("A3") },
    PinDef { pin_num: 5, mcu_pin: "PC1", default_label: Some("A4") },
    PinDef { pin_num: 6, mcu_pin: "PC0", default_label: Some("A5") },
];

const CN10_PINS: &[PinDef] = &[
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

const CN5_PINS: &[PinDef] = &[
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

const CN9_PINS: &[PinDef] = &[
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
    cr.set_source_rgb(tokens::color::BG_VOID.0, tokens::color::BG_VOID.1, tokens::color::BG_VOID.2);
    cr.rectangle(0.0, 0.0, canvas_w, canvas_h);
    let _ = cr.fill();

    // 1. Board Silhouette (PCB Outline) - Surface BG_PANEL, hairline border BORDER_HAIR, 0px sharp corners
    cr.set_source_rgb(tokens::color::BG_PANEL.0, tokens::color::BG_PANEL.1, tokens::color::BG_PANEL.2);
    cr.rectangle(board_x, board_y, board_w, board_h);
    let _ = cr.fill_preserve();
    cr.set_source_rgb(tokens::color::BORDER_HAIR.0, tokens::color::BORDER_HAIR.1, tokens::color::BORDER_HAIR.2);
    cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
    let _ = cr.stroke();

    // Subtle PCB Grid Accent Texture Lines
    cr.set_source_rgba(tokens::color::BORDER_HAIR.0, tokens::color::BORDER_HAIR.1, tokens::color::BORDER_HAIR.2, 0.25);
    cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
    let mut gy = board_y + 40.0;
    while gy < board_y + board_h {
        let _ = cr.move_to(board_x + 10.0, gy);
        let _ = cr.line_to(board_x + board_w - 10.0, gy);
        let _ = cr.stroke();
        gy += 40.0;
    }

    // 2. ST-LINK Debugger Top Section Notch - 0px sharp corners
    let notch_w = (board_w * 0.24).clamp(220.0, 380.0);
    let notch_x = board_x + (board_w - notch_w) / 2.0;
    let notch_y = board_y + 8.0;
    cr.set_source_rgb(tokens::color::BG_VOID.0, tokens::color::BG_VOID.1, tokens::color::BG_VOID.2);
    cr.rectangle(notch_x, notch_y, notch_w, 20.0);
    let _ = cr.fill_preserve();
    cr.set_source_rgb(tokens::color::BORDER_HAIR.0, tokens::color::BORDER_HAIR.1, tokens::color::BORDER_HAIR.2);
    cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
    let _ = cr.stroke();

    cr.select_font_face(tokens::font::CAIRO_SANS, cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(9.0);
    cr.set_source_rgb(tokens::color::TEXT_MUTED.0, tokens::color::TEXT_MUTED.1, tokens::color::TEXT_MUTED.2);
    if let Ok(ext) = cr.text_extents("ST-LINK V2-1 ON-BOARD DEBUGGER") {
        let _ = cr.move_to(notch_x + (notch_w - ext.width()) / 2.0, notch_y + 14.0);
        let _ = cr.show_text("ST-LINK V2-1 ON-BOARD DEBUGGER");
    }

    // 3. Board Header Banner Silkscreen - TEXT_PRIMARY title, TEXT_MUTED subtitle
    cr.select_font_face(tokens::font::CAIRO_SANS, cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(16.0);
    cr.set_source_rgb(tokens::color::TEXT_PRIMARY.0, tokens::color::TEXT_PRIMARY.1, tokens::color::TEXT_PRIMARY.2);
    let title_str = "STMicroelectronics NUCLEO-F446RE";
    let title_y = board_y + 46.0;
    if let Ok(ext) = cr.text_extents(title_str) {
        let _ = cr.move_to(board_x + (board_w - ext.width()) / 2.0, title_y);
        let _ = cr.show_text(title_str);
    }

    cr.select_font_face(tokens::font::CAIRO_MONO, cairo::FontSlant::Normal, cairo::FontWeight::Normal);
    cr.set_font_size(10.5);
    cr.set_source_rgb(tokens::color::TEXT_MUTED.0, tokens::color::TEXT_MUTED.1, tokens::color::TEXT_MUTED.2);
    let subtitle_str = "ARM® Cortex®-M4 MCU @ 180MHz • Physical 64-Pin Connector Pinout";
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
            format!("! {} pin conflicts detected • see highlighted pins below", count)
        };

        cr.select_font_face(tokens::font::CAIRO_SANS, cairo::FontSlant::Normal, cairo::FontWeight::Bold);
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
    cr.set_source_rgb(tokens::color::BG_VOID.0, tokens::color::BG_VOID.1, tokens::color::BG_VOID.2);
    cr.rectangle(mcu_x, mcu_y, mcu_w, mcu_h);
    let _ = cr.fill_preserve();
    cr.set_source_rgb(tokens::color::BORDER_HAIR.0, tokens::color::BORDER_HAIR.1, tokens::color::BORDER_HAIR.2);
    cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
    let _ = cr.stroke();

    // MCU Orientation Pin 1 Dot
    cr.set_source_rgb(tokens::color::TEXT_MUTED.0, tokens::color::TEXT_MUTED.1, tokens::color::TEXT_MUTED.2);
    cr.arc(mcu_x + 14.0, mcu_y + 14.0, 3.5, 0.0, 2.0 * std::f64::consts::PI);
    let _ = cr.fill();

    // MCU Text Labels (Data -> JetBrains Mono)
    cr.select_font_face(tokens::font::CAIRO_MONO, cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(11.0);
    cr.set_source_rgb(tokens::color::TEXT_PRIMARY.0, tokens::color::TEXT_PRIMARY.1, tokens::color::TEXT_PRIMARY.2);
    if let Ok(ext) = cr.text_extents("STM32F446") {
        let _ = cr.move_to(mcu_x + (mcu_w - ext.width()) / 2.0, mcu_y + mcu_h * 0.42);
        let _ = cr.show_text("STM32F446");
    }
    cr.set_font_size(10.0);
    cr.set_source_rgb(tokens::color::TEXT_PRIMARY.0, tokens::color::TEXT_PRIMARY.1, tokens::color::TEXT_PRIMARY.2);
    if let Ok(ext) = cr.text_extents("RET6") {
        let _ = cr.move_to(mcu_x + (mcu_w - ext.width()) / 2.0, mcu_y + mcu_h * 0.55);
        let _ = cr.show_text("RET6");
    }
    cr.set_font_size(9.0);
    cr.set_source_rgb(tokens::color::TEXT_MUTED.0, tokens::color::TEXT_MUTED.1, tokens::color::TEXT_MUTED.2);
    if let Ok(ext) = cr.text_extents("LQFP64") {
        let _ = cr.move_to(mcu_x + (mcu_w - ext.width()) / 2.0, mcu_y + mcu_h * 0.68);
        let _ = cr.show_text("LQFP64");
    }

    // User LED (LD2 - Green STATE_READY) Indicator Box - sharp 0px corners
    let is_pa5_active = highlights.contains_key(&("CN10", 11)) || highlights.contains_key(&("CN5", 6));
    let led_y = row_start_y + 2.0 * row_h;
    let led_h = (1.2 * row_h).clamp(30.0, 42.0);
    cr.set_source_rgb(tokens::color::BG_PANEL.0, tokens::color::BG_PANEL.1, tokens::color::BG_PANEL.2);
    cr.rectangle(mcu_x, led_y, mcu_w, led_h);
    let _ = cr.fill_preserve();

    if is_pa5_active {
        cr.set_source_rgb(tokens::color::STATE_READY.0, tokens::color::STATE_READY.1, tokens::color::STATE_READY.2);
        cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
        let _ = cr.stroke();

        cr.set_source_rgb(tokens::color::STATE_READY.0, tokens::color::STATE_READY.1, tokens::color::STATE_READY.2);
        cr.arc(mcu_x + 18.0, led_y + led_h / 2.0, 6.0, 0.0, 2.0 * std::f64::consts::PI);
        let _ = cr.fill();

        cr.select_font_face(tokens::font::CAIRO_MONO, cairo::FontSlant::Normal, cairo::FontWeight::Bold);
        cr.set_font_size(10.0);
        cr.set_source_rgb(tokens::color::STATE_READY.0, tokens::color::STATE_READY.1, tokens::color::STATE_READY.2);
        let _ = cr.move_to(mcu_x + 32.0, led_y + led_h / 2.0 + 4.0);
        let _ = cr.show_text("LD2 [ON]");
    } else {
        cr.set_source_rgb(tokens::color::BORDER_HAIR.0, tokens::color::BORDER_HAIR.1, tokens::color::BORDER_HAIR.2);
        cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
        let _ = cr.stroke();

        cr.set_source_rgb(tokens::color::BORDER_HAIR.0, tokens::color::BORDER_HAIR.1, tokens::color::BORDER_HAIR.2);
        cr.arc(mcu_x + 18.0, led_y + led_h / 2.0, 5.0, 0.0, 2.0 * std::f64::consts::PI);
        let _ = cr.fill();

        cr.select_font_face(tokens::font::CAIRO_MONO, cairo::FontSlant::Normal, cairo::FontWeight::Normal);
        cr.set_font_size(9.5);
        cr.set_source_rgb(tokens::color::TEXT_MUTED.0, tokens::color::TEXT_MUTED.1, tokens::color::TEXT_MUTED.2);
        let _ = cr.move_to(mcu_x + 32.0, led_y + led_h / 2.0 + 4.0);
        let _ = cr.show_text("LD2 (PA5)");
    }

    // User Button (B1 USER) & Reset Button (B2 RESET) - Sharp 0px corners, 1px border
    let btn_y = row_start_y + 13.0 * row_h;
    let btn_h = (1.0 * row_h).clamp(26.0, 34.0);
    let b1_w = (mcu_w - 6.0) / 2.0;

    // B1 Button (PC13)
    cr.set_source_rgb(tokens::color::BG_PANEL.0, tokens::color::BG_PANEL.1, tokens::color::BG_PANEL.2);
    cr.rectangle(mcu_x, btn_y, b1_w, btn_h);
    let _ = cr.fill_preserve();
    cr.set_source_rgb(tokens::color::BORDER_HAIR.0, tokens::color::BORDER_HAIR.1, tokens::color::BORDER_HAIR.2);
    cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
    let _ = cr.stroke();
    cr.select_font_face(tokens::font::CAIRO_SANS, cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(9.0);
    cr.set_source_rgb(tokens::color::TEXT_PRIMARY.0, tokens::color::TEXT_PRIMARY.1, tokens::color::TEXT_PRIMARY.2);
    let _ = cr.move_to(mcu_x + 8.0, btn_y + btn_h / 2.0 + 3.0);
    let _ = cr.show_text("B1 USER");

    // B2 Button (RESET)
    cr.set_source_rgb(tokens::color::BG_PANEL.0, tokens::color::BG_PANEL.1, tokens::color::BG_PANEL.2);
    cr.rectangle(mcu_x + b1_w + 6.0, btn_y, b1_w, btn_h);
    let _ = cr.fill_preserve();
    cr.set_source_rgb(tokens::color::BORDER_HAIR.0, tokens::color::BORDER_HAIR.1, tokens::color::BORDER_HAIR.2);
    cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
    let _ = cr.stroke();
    cr.select_font_face(tokens::font::CAIRO_SANS, cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(9.0);
    cr.set_source_rgb(tokens::color::TEXT_PRIMARY.0, tokens::color::TEXT_PRIMARY.1, tokens::color::TEXT_PRIMARY.2);
    let _ = cr.move_to(mcu_x + b1_w + 10.0, btn_y + btn_h / 2.0 + 3.0);
    let _ = cr.show_text("B2 RESET");

    // 5. Draw Connector Header Titles & Active Counts with Text Tags [MORPHO] / [ARDUINO]
    for conn in CONNECTORS {
        let is_morpho = conn.name == "CN7" || conn.name == "CN10";
        let top_pin_idx = match conn.name {
            "CN5" => 9,
            "CN9" => 7,
            _ => 0,
        };

        let (cell_x, cell_y, cell_w, _cell_h) = get_pin_cell_rect(conn.name, top_pin_idx, canvas_w, canvas_h);
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
        let header_text = format!("{} {} • {} ({} active)", conn.name, tag, type_title, active_count);

        cr.select_font_face(tokens::font::CAIRO_MONO, cairo::FontSlant::Normal, cairo::FontWeight::Bold);
        cr.set_font_size(10.5);
        cr.set_source_rgb(tokens::color::TEXT_PRIMARY.0, tokens::color::TEXT_PRIMARY.1, tokens::color::TEXT_PRIMARY.2);

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
    for conn in CONNECTORS {
        for (idx, p) in conn.pins.iter().enumerate() {
            let (cell_x, cell_y, cell_w, cell_h) = get_pin_cell_rect(conn.name, idx, canvas_w, canvas_h);

            let is_hl = highlights.get(&(conn.name, p.pin_num));
            let is_hovered = hovered_pin.map_or(false, |(c_name, p_num)| {
                c_name == conn.name && *p_num == p.pin_num
            });

            if let Some(hl_info) = is_hl {
                let reserved_info = stakhal_core::nucleo_pinout::check_reserved(p.mcu_pin);
                let (hl_r, hl_g, hl_b) = match reserved_info {
                    Some(res) => match res.severity {
                        stakhal_core::nucleo_pinout::ReservedSeverity::Critical => tokens::color::STATE_ERROR,
                        stakhal_core::nucleo_pinout::ReservedSeverity::Caution => tokens::color::STATE_ACTIVE,
                    },
                    None => tokens::color::STATE_READY,
                };

                // Highlighted Pin (Active in loaded project) - background BG_PANEL
                cr.set_source_rgb(tokens::color::BG_PANEL.0, tokens::color::BG_PANEL.1, tokens::color::BG_PANEL.2);
                cr.rectangle(cell_x, cell_y, cell_w, cell_h);
                let _ = cr.fill_preserve();

                // Interactive/Selection ONLY uses ACCENT
                if is_hovered {
                    cr.set_source_rgb(tokens::color::ACCENT.0, tokens::color::ACCENT.1, tokens::color::ACCENT.2);
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
                    cr.set_source_rgb(tokens::color::BG_VOID.0, tokens::color::BG_VOID.1, tokens::color::BG_VOID.2);
                    cr.rectangle(cell_x + 3.0, cell_y + 3.0, badge_w, cell_h - 6.0);
                    let _ = cr.fill_preserve();
                    cr.set_source_rgba(hl_r, hl_g, hl_b, 0.35);
                    cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
                    let _ = cr.stroke();

                    cr.select_font_face(tokens::font::CAIRO_MONO, cairo::FontSlant::Normal, cairo::FontWeight::Normal);
                    cr.set_font_size(9.5);
                    cr.set_source_rgb(tokens::color::TEXT_MUTED.0, tokens::color::TEXT_MUTED.1, tokens::color::TEXT_MUTED.2);
                } else {
                    cr.set_source_rgb(hl_r, hl_g, hl_b);
                    cr.rectangle(cell_x + 3.0, cell_y + 3.0, badge_w, cell_h - 6.0);
                    let _ = cr.fill();

                    cr.select_font_face(tokens::font::CAIRO_MONO, cairo::FontSlant::Normal, cairo::FontWeight::Bold);
                    cr.set_font_size(9.5);
                    cr.set_source_rgb(tokens::color::BG_VOID.0, tokens::color::BG_VOID.1, tokens::color::BG_VOID.2);
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
                    cr.set_source_rgb(tokens::color::TEXT_MUTED.0, tokens::color::TEXT_MUTED.1, tokens::color::TEXT_MUTED.2);
                } else {
                    cr.set_source_rgb(tokens::color::TEXT_PRIMARY.0, tokens::color::TEXT_PRIMARY.1, tokens::color::TEXT_PRIMARY.2);
                }
                let mcu_pin_x = cell_x + badge_w + 8.0;
                let _ = cr.move_to(mcu_pin_x, cell_y + cell_h * 0.64);
                let _ = cr.show_text(p.mcu_pin);

                let mcu_ext_w = cr.text_extents(p.mcu_pin).map(|e| e.width()).unwrap_or(24.0);

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
                    cr.select_font_face(tokens::font::CAIRO_MONO, cairo::FontSlant::Normal, cairo::FontWeight::Normal);
                    cr.set_font_size(9.0);
                    cr.set_source_rgb(tokens::color::TEXT_MUTED.0, tokens::color::TEXT_MUTED.1, tokens::color::TEXT_MUTED.2);
                } else {
                    cr.select_font_face(tokens::font::CAIRO_MONO, cairo::FontSlant::Normal, cairo::FontWeight::Bold);
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
                cr.set_source_rgb(tokens::color::BG_PANEL.0, tokens::color::BG_PANEL.1, tokens::color::BG_PANEL.2);
                cr.rectangle(cell_x, cell_y, cell_w, cell_h);
                let _ = cr.fill_preserve();

                // Interactive/Selection ONLY uses ACCENT
                if is_hovered {
                    cr.set_source_rgb(tokens::color::ACCENT.0, tokens::color::ACCENT.1, tokens::color::ACCENT.2);
                } else {
                    cr.set_source_rgb(tokens::color::BORDER_HAIR.0, tokens::color::BORDER_HAIR.1, tokens::color::BORDER_HAIR.2);
                }
                cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
                let _ = cr.stroke();

                // Pin Number Box for Neutral Cell - sharp 0px corners
                let badge_w = (cell_w * 0.16).clamp(26.0, 34.0);
                cr.set_source_rgb(tokens::color::BG_VOID.0, tokens::color::BG_VOID.1, tokens::color::BG_VOID.2);
                cr.rectangle(cell_x + 4.0, cell_y + 4.0, badge_w, cell_h - 8.0);
                let _ = cr.fill();

                cr.select_font_face(tokens::font::CAIRO_MONO, cairo::FontSlant::Normal, cairo::FontWeight::Normal);
                cr.set_font_size(9.5);
                if is_hovered {
                    cr.set_source_rgb(tokens::color::TEXT_PRIMARY.0, tokens::color::TEXT_PRIMARY.1, tokens::color::TEXT_PRIMARY.2);
                } else {
                    cr.set_source_rgb(tokens::color::TEXT_MUTED.0, tokens::color::TEXT_MUTED.1, tokens::color::TEXT_MUTED.2);
                }
                let pnum_str = format!("{}", p.pin_num);
                if let Ok(ext) = cr.text_extents(&pnum_str) {
                    let tx = cell_x + 4.0 + (badge_w - ext.width()) / 2.0;
                    let _ = cr.move_to(tx, cell_y + cell_h * 0.60);
                    let _ = cr.show_text(&pnum_str);
                }

                // MCU Pin & Label Text
                if is_hovered {
                    cr.set_source_rgb(tokens::color::TEXT_PRIMARY.0, tokens::color::TEXT_PRIMARY.1, tokens::color::TEXT_PRIMARY.2);
                } else {
                    cr.set_source_rgb(tokens::color::TEXT_PRIMARY.0, tokens::color::TEXT_PRIMARY.1, tokens::color::TEXT_PRIMARY.2);
                }
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
    cr.select_font_face(tokens::font::CAIRO_SANS, cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(10.0);

    let leg_x = board_x + (board_w * 0.02).max(16.0);

    // Active Signal Legend
    cr.set_source_rgb(tokens::color::STATE_READY.0, tokens::color::STATE_READY.1, tokens::color::STATE_READY.2);
    cr.rectangle(leg_x, legend_y, 14.0, 14.0);
    let _ = cr.fill();

    cr.set_source_rgb(tokens::color::TEXT_PRIMARY.0, tokens::color::TEXT_PRIMARY.1, tokens::color::TEXT_PRIMARY.2);
    let _ = cr.move_to(leg_x + 22.0, legend_y + 11.0);
    let active_label = if is_filtering_module {
        "Active in Module"
    } else {
        "Active Signal in Loaded Project"
    };
    let _ = cr.show_text(active_label);

    if is_filtering_module {
        let leg_x2 = leg_x + 150.0;
        cr.set_source_rgb(tokens::color::BG_PANEL.0, tokens::color::BG_PANEL.1, tokens::color::BG_PANEL.2);
        cr.rectangle(leg_x2, legend_y, 14.0, 14.0);
        let _ = cr.fill_preserve();
        cr.set_source_rgba(tokens::color::STATE_READY.0, tokens::color::STATE_READY.1, tokens::color::STATE_READY.2, 0.35);
        cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
        let _ = cr.stroke();

        cr.set_source_rgb(tokens::color::TEXT_MUTED.0, tokens::color::TEXT_MUTED.1, tokens::color::TEXT_MUTED.2);
        let _ = cr.move_to(leg_x2 + 22.0, legend_y + 11.0);
        let _ = cr.show_text("Active Elsewhere in Project");
    }

    // 8. FINAL PASS: Compact Floating Tooltip Card - sharp 0px corners, BG_PANEL, 1px hairline border
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

                // Compact Floating Position offset near cursor
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

                // Card Background BG_PANEL, sharp 0px corners
                cr.set_source_rgb(tokens::color::BG_PANEL.0, tokens::color::BG_PANEL.1, tokens::color::BG_PANEL.2);
                cr.rectangle(tt_x, tt_y, tt_w, tt_h);
                let _ = cr.fill_preserve();

                // Border color (1px hairline)
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

                // Line 1: Header Info (TEXT_PRIMARY, JetBrains Mono)
                cr.select_font_face(tokens::font::CAIRO_MONO, cairo::FontSlant::Normal, cairo::FontWeight::Bold);
                cr.set_font_size(10.0);
                cr.set_source_rgb(tokens::color::TEXT_PRIMARY.0, tokens::color::TEXT_PRIMARY.1, tokens::color::TEXT_PRIMARY.2);
                let _ = cr.move_to(tt_x + 10.0, tt_y + 16.0);
                let _ = cr.show_text(&line1);

                // Line 2: Signal / Status Info
                cr.select_font_face(tokens::font::CAIRO_MONO, cairo::FontSlant::Normal, cairo::FontWeight::Bold);
                cr.set_font_size(9.5);
                if is_hl.is_some() {
                    cr.set_source_rgb(border_r, border_g, border_b);
                } else {
                    cr.set_source_rgb(tokens::color::TEXT_MUTED.0, tokens::color::TEXT_MUTED.1, tokens::color::TEXT_MUTED.2);
                }
                let _ = cr.move_to(tt_x + 10.0, tt_y + 32.0);
                let _ = cr.show_text(&line2);

                // Line 3: Conflict Reason Warning
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

        let mut hit_pin: Option<(&'static str, u8, &'static str, Option<&'static str>)> = None;

        for conn in CONNECTORS {
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
                assert!(x >= 30.0, "Pin cell X {} out of bounds for {} pin {}", x, conn.name, p.pin_num);
                assert!(x + w <= 1570.0, "Pin cell X+W {} out of bounds for {} pin {}", x + w, conn.name, p.pin_num);
                assert!(y >= 30.0, "Pin cell Y {} out of bounds for {} pin {}", y, conn.name, p.pin_num);
                assert!(y + h <= 960.0, "Pin cell Y+H {} out of bounds for {} pin {}", y + h, conn.name, p.pin_num);
            }
        }
    }

    #[test]
    fn test_draw_nucleo_pinout_canvas_rendering() {
        let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 1200, 750).expect("Failed to create surface");
        let cr = cairo::Context::new(&surface).expect("Failed to create context");
        let state = Rc::new(RefCell::new(AppState::default()));

        let fixture_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../stakhal-core/tests/fixtures/stakhal_blink_f446re");
        let ioc_path = fixture_dir.join("stakhal_blink_f446re.ioc");
        let main_c_path = fixture_dir.join("Core/Src/main.c");
        if let Ok(project) = stakhal_core::ir::schema::load_project(&ioc_path, &main_c_path) {
            state.borrow().project.borrow_mut().loaded_project = Some(project);
        }

        draw_nucleo_pinout(&cr, 1200.0, 750.0, &state);
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

        draw_nucleo_pinout(&cr, 1200.0, 750.0, &state);
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
        let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 1200, 750).expect("Failed to create surface");
        let cr = cairo::Context::new(&surface).expect("Failed to create context");
        let rc_state = Rc::new(RefCell::new(hatch_state));
        draw_nucleo_pinout(&cr, 1200.0, 750.0, &rc_state);
        surface.flush();
    }
}
