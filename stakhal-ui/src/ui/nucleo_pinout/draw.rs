use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use gtk4::cairo;
use gtk4::prelude::*;
use crate::state::{AppState, AppWidgets};

pub struct PinDef {
    pub pin_num: u8,
    pub mcu_pin: &'static str,
    pub default_label: Option<&'static str>,
}

pub struct ConnectorDef {
    pub name: &'static str,
    #[allow(dead_code)]
    pub title: &'static str,
    pub pins: &'static [PinDef],
    #[allow(dead_code)]
    pub rows: u8,
    #[allow(dead_code)]
    pub cols: u8,
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
        title: "CN7 — Morpho Left",
        pins: CN7_PINS,
        rows: 19,
        cols: 2,
    },
    ConnectorDef {
        name: "CN6",
        title: "CN6 — Power",
        pins: CN6_PINS,
        rows: 8,
        cols: 1,
    },
    ConnectorDef {
        name: "CN8",
        title: "CN8 — Analog In",
        pins: CN8_PINS,
        rows: 6,
        cols: 1,
    },
    ConnectorDef {
        name: "CN10",
        title: "CN10 — Morpho Right",
        pins: CN10_PINS,
        rows: 19,
        cols: 2,
    },
    ConnectorDef {
        name: "CN5",
        title: "CN5 — Digital High",
        pins: CN5_PINS,
        rows: 10,
        cols: 1,
    },
    ConnectorDef {
        name: "CN9",
        title: "CN9 — Digital Low",
        pins: CN9_PINS,
        rows: 8,
        cols: 1,
    },
];

struct PinHighlightInfo {
    signal: String,
}

fn get_active_pin_highlights(state: &AppState) -> HashMap<(&'static str, u8), PinHighlightInfo> {
    let mut map = HashMap::new();
    let project = match &state.loaded_project {
        Some(p) => p,
        None => return map,
    };

    for pin_cfg in &project.pins {
        let mcu_pin = &pin_cfg.pin;
        let signal = &pin_cfg.signal;

        if let Some(loc) = stakhal_core::nucleo_pinout::lookup_pin(mcu_pin) {
            if let Some((conn, pin_num)) = loc.morpho {
                let static_conn = match conn {
                    "CN7" => "CN7",
                    "CN10" => "CN10",
                    _ => conn,
                };
                map.insert((static_conn, pin_num), PinHighlightInfo { signal: signal.clone() });
            }
            if let Some((conn, pin_num, _label)) = loc.arduino {
                let static_conn = match conn {
                    "CN5" => "CN5",
                    "CN6" => "CN6",
                    "CN8" => "CN8",
                    "CN9" => "CN9",
                    _ => conn,
                };
                map.insert((static_conn, pin_num), PinHighlightInfo { signal: signal.clone() });
            }
        }
    }

    map
}

pub fn get_pin_cell_rect(conn_name: &str, pin_idx: usize) -> (f64, f64, f64, f64) {
    let board_x = 40.0;
    let board_y = 50.0;
    let board_w = 1400.0;
    let cell_w = 195.0;
    let cell_h = 32.0;
    let row_h = 36.0;
    let row_start_y = board_y + 110.0; // 160.0

    match conn_name {
        "CN7" => {
            let r = pin_idx / 2;
            let c = pin_idx % 2;
            let cell_x = board_x + 24.0 + (c as f64) * (cell_w + 4.0);
            let cell_y = row_start_y + (r as f64) * row_h;
            (cell_x, cell_y, cell_w, cell_h)
        }
        "CN6" => {
            let r = 4 + pin_idx;
            let cell_x = board_x + 24.0 + 2.0 * (cell_w + 4.0) + 16.0;
            let cell_y = row_start_y + (r as f64) * row_h;
            (cell_x, cell_y, cell_w, cell_h)
        }
        "CN8" => {
            let r = 13 + pin_idx;
            let cell_x = board_x + 24.0 + 2.0 * (cell_w + 4.0) + 16.0;
            let cell_y = row_start_y + (r as f64) * row_h;
            (cell_x, cell_y, cell_w, cell_h)
        }
        "CN10" => {
            let r = pin_idx / 2;
            let c = pin_idx % 2;
            let col1_x = board_x + board_w - 24.0 - cell_w;
            let col0_x = col1_x - 4.0 - cell_w;
            let cell_x = if c == 0 { col0_x } else { col1_x };
            let cell_y = row_start_y + (r as f64) * row_h;
            (cell_x, cell_y, cell_w, cell_h)
        }
        "CN5" => {
            // Physical Nucleo: Pin 10 at top (Row 0), Pin 1 at bottom (Row 9)
            let r = 9 - pin_idx;
            let col0_x = board_x + board_w - 24.0 - cell_w - 4.0 - cell_w;
            let cell_x = col0_x - 16.0 - cell_w;
            let cell_y = row_start_y + (r as f64) * row_h;
            (cell_x, cell_y, cell_w, cell_h)
        }
        "CN9" => {
            // Physical Nucleo: Pin 8 at top (Row 11), Pin 1 at bottom (Row 18)
            let r = 18 - pin_idx;
            let col0_x = board_x + board_w - 24.0 - cell_w - 4.0 - cell_w;
            let cell_x = col0_x - 16.0 - cell_w;
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
    let st = state.borrow();
    let highlights = get_active_pin_highlights(&st);
    let hovered_pin = st.hovered_pinout_pin.as_ref();

    // Canvas Background
    cr.set_source_rgb(0.04, 0.04, 0.05);
    cr.rectangle(0.0, 0.0, width.max(1480.0), height.max(980.0));
    let _ = cr.fill();

    // Board Dimensions
    let board_x = 40.0;
    let board_y = 50.0;
    let board_w = 1400.0;
    let board_h = 890.0;

    // 1. Board Silhouette (PCB Outline)
    cr.set_source_rgb(0.07, 0.09, 0.11);
    draw_rounded_rectangle(cr, board_x, board_y, board_w, board_h, 16.0);
    let _ = cr.fill_preserve();
    cr.set_source_rgb(0.16, 0.22, 0.28);
    cr.set_line_width(2.0);
    let _ = cr.stroke();

    // Subtle PCB Grid Accent Texture Lines
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.02);
    cr.set_line_width(1.0);
    let mut gy = board_y + 40.0;
    while gy < board_y + board_h {
        let _ = cr.move_to(board_x + 10.0, gy);
        let _ = cr.line_to(board_x + board_w - 10.0, gy);
        let _ = cr.stroke();
        gy += 40.0;
    }

    // 2. ST-LINK Debugger Top Section Notch
    cr.set_source_rgb(0.11, 0.14, 0.17);
    draw_rounded_rectangle(cr, board_x + 500.0, board_y + 12.0, 400.0, 24.0, 6.0);
    let _ = cr.fill_preserve();
    cr.set_source_rgb(0.24, 0.30, 0.36);
    cr.set_line_width(1.0);
    let _ = cr.stroke();

    cr.select_font_face("monospace", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(10.0);
    cr.set_source_rgb(0.50, 0.60, 0.70);
    let _ = cr.move_to(board_x + 630.0, board_y + 28.0);
    let _ = cr.show_text("ST-LINK V2-1 ON-BOARD DEBUGGER");

    // 3. Board Header Banner Silkscreen
    cr.select_font_face("sans-serif", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(18.0);
    cr.set_source_rgb(0.92, 0.95, 0.98);
    let title_str = "STMicroelectronics NUCLEO-F446RE";
    if let Ok(ext) = cr.text_extents(title_str) {
        let _ = cr.move_to(board_x + (board_w - ext.width()) / 2.0, board_y + 65.0);
        let _ = cr.show_text(title_str);
    }

    cr.select_font_face("monospace", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
    cr.set_font_size(11.0);
    cr.set_source_rgb(0.55, 0.65, 0.75);
    let subtitle_str = "ARM® Cortex®-M4 MCU @ 180MHz — Physical 64-Pin Connector Pinout";
    if let Ok(ext) = cr.text_extents(subtitle_str) {
        let _ = cr.move_to(board_x + (board_w - ext.width()) / 2.0, board_y + 85.0);
        let _ = cr.show_text(subtitle_str);
    }

    // 4. Center Component Graphics (MCU IC Chip, Buttons & LEDs)
    let left_group_right = board_x + 24.0 + 2.0 * 199.0 + 16.0 + 195.0; // 673.0
    let right_group_left = board_x + board_w - 24.0 - 2.0 * 199.0 - 16.0 - 195.0; // 807.0

    let mcu_x = left_group_right + 10.0;
    let mcu_w = right_group_left - 10.0 - mcu_x; // 114.0
    let mcu_y = board_y + 390.0;
    let mcu_h = 160.0;

    // MCU LQFP64 Chip Frame
    cr.set_source_rgb(0.12, 0.15, 0.18);
    draw_rounded_rectangle(cr, mcu_x, mcu_y, mcu_w, mcu_h, 6.0);
    let _ = cr.fill_preserve();
    cr.set_source_rgb(0.28, 0.36, 0.44);
    cr.set_line_width(1.5);
    let _ = cr.stroke();

    // MCU Orientation Pin 1 Dot
    cr.set_source_rgb(0.60, 0.70, 0.80);
    cr.arc(mcu_x + 12.0, mcu_y + 12.0, 3.5, 0.0, 2.0 * std::f64::consts::PI);
    let _ = cr.fill();

    // MCU Text Label
    cr.select_font_face("sans-serif", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(11.0);
    cr.set_source_rgb(0.90, 0.94, 0.98);
    if let Ok(ext) = cr.text_extents("STM32F446") {
        let _ = cr.move_to(mcu_x + (mcu_w - ext.width()) / 2.0, mcu_y + 70.0);
        let _ = cr.show_text("STM32F446");
    }
    cr.set_font_size(10.0);
    cr.set_source_rgb(0.60, 0.72, 0.84);
    if let Ok(ext) = cr.text_extents("RET6") {
        let _ = cr.move_to(mcu_x + (mcu_w - ext.width()) / 2.0, mcu_y + 88.0);
        let _ = cr.show_text("RET6");
    }
    cr.set_font_size(9.0);
    cr.set_source_rgb(0.45, 0.55, 0.65);
    if let Ok(ext) = cr.text_extents("LQFP64") {
        let _ = cr.move_to(mcu_x + (mcu_w - ext.width()) / 2.0, mcu_y + 105.0);
        let _ = cr.show_text("LQFP64");
    }

    // User LED (LD2 - Green) Indicator Box
    let is_pa5_active = highlights.contains_key(&("CN10", 11)) || highlights.contains_key(&("CN5", 6));
    let led_y = board_y + 260.0;
    cr.set_source_rgb(0.12, 0.15, 0.18);
    draw_rounded_rectangle(cr, mcu_x, led_y, mcu_w, 40.0, 4.0);
    let _ = cr.fill_preserve();

    if is_pa5_active {
        cr.set_source_rgb(0.13, 0.77, 0.36); // Green active glow
        cr.set_line_width(1.5);
        let _ = cr.stroke();

        cr.set_source_rgb(0.13, 0.77, 0.36);
        cr.arc(mcu_x + 16.0, led_y + 20.0, 6.0, 0.0, 2.0 * std::f64::consts::PI);
        let _ = cr.fill();

        cr.select_font_face("monospace", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
        cr.set_font_size(10.0);
        cr.set_source_rgb(0.20, 0.90, 0.45);
        let _ = cr.move_to(mcu_x + 28.0, led_y + 24.0);
        let _ = cr.show_text("LD2 [ON]");
    } else {
        cr.set_source_rgb(0.25, 0.30, 0.35);
        cr.set_line_width(1.0);
        let _ = cr.stroke();

        cr.set_source_rgb(0.25, 0.30, 0.35);
        cr.arc(mcu_x + 16.0, led_y + 20.0, 5.0, 0.0, 2.0 * std::f64::consts::PI);
        let _ = cr.fill();

        cr.select_font_face("monospace", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
        cr.set_font_size(9.5);
        cr.set_source_rgb(0.50, 0.55, 0.60);
        let _ = cr.move_to(mcu_x + 28.0, led_y + 24.0);
        let _ = cr.show_text("LD2 (PA5)");
    }

    // User Button (B1 Blue) & Reset Button (B2 Black)
    let btn_y = board_y + 610.0;
    let b1_w = (mcu_w - 6.0) / 2.0;

    // B1 Button (Blue - PC13)
    cr.set_source_rgb(0.01, 0.45, 0.75);
    draw_rounded_rectangle(cr, mcu_x, btn_y, b1_w, 32.0, 4.0);
    let _ = cr.fill();
    cr.select_font_face("monospace", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(9.0);
    cr.set_source_rgb(1.0, 1.0, 1.0);
    let _ = cr.move_to(mcu_x + 8.0, btn_y + 20.0);
    let _ = cr.show_text("B1 USER");

    // B2 Button (Black - RESET)
    cr.set_source_rgb(0.20, 0.20, 0.22);
    draw_rounded_rectangle(cr, mcu_x + b1_w + 6.0, btn_y, b1_w, 32.0, 4.0);
    let _ = cr.fill_preserve();
    cr.set_source_rgb(0.40, 0.40, 0.45);
    cr.set_line_width(1.0);
    let _ = cr.stroke();
    cr.select_font_face("monospace", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(9.0);
    cr.set_source_rgb(0.85, 0.85, 0.85);
    let _ = cr.move_to(mcu_x + b1_w + 10.0, btn_y + 20.0);
    let _ = cr.show_text("B2 RESET");

    // 5. Draw Connector Header Titles & Active Counts
    let header_y_coords = [
        ("CN7", board_y + 132.0, board_x + 24.0, "Morpho Left (38P)", true),
        ("CN6", board_y + 276.0, board_x + 24.0 + 2.0 * 199.0 + 16.0, "Arduino Power (8P)", false),
        ("CN8", board_y + 600.0, board_x + 24.0 + 2.0 * 199.0 + 16.0, "Arduino Analog (6P)", false),
        ("CN10", board_y + 132.0, board_x + board_w - 24.0 - 2.0 * 199.0 + 4.0, "Morpho Right (38P)", true),
        ("CN5", board_y + 132.0, board_x + board_w - 24.0 - 2.0 * 199.0 - 16.0 - 195.0, "Arduino High (10P)", false),
        ("CN9", board_y + 528.0, board_x + board_w - 24.0 - 2.0 * 199.0 - 16.0 - 195.0, "Arduino Low (8P)", false),
    ];

    for conn in CONNECTORS {
        if let Some((_, hy, hx, type_title, is_morpho)) = header_y_coords.iter().find(|(name, _, _, _, _)| *name == conn.name) {
            let active_count = conn
                .pins
                .iter()
                .filter(|p| highlights.contains_key(&(conn.name, p.pin_num)))
                .count();

            cr.select_font_face("monospace", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
            cr.set_font_size(11.0);

            if *is_morpho {
                cr.set_source_rgb(0.22, 0.74, 0.97); // Morpho Cyan/Blue
            } else {
                cr.set_source_rgb(0.85, 0.40, 0.95); // Arduino Magenta
            }
            let _ = cr.move_to(*hx, *hy);
            let header_text = format!("{} — {}", conn.name, type_title);
            let _ = cr.show_text(&header_text);

            if active_count > 0 {
                let badge_str = format!("{} active", active_count);
                cr.set_font_size(10.0);
                cr.set_source_rgb(0.01, 0.75, 0.98);
                let _ = cr.move_to(*hx + 130.0, *hy);
                let _ = cr.show_text(&badge_str);
            }
        }
    }

    // 6. Draw Pin Cells for All Connectors
    for conn in CONNECTORS {
        let is_morpho = conn.name == "CN7" || conn.name == "CN10";

        for (idx, p) in conn.pins.iter().enumerate() {
            let (cell_x, cell_y, cell_w, cell_h) = get_pin_cell_rect(conn.name, idx);

            let is_hl = highlights.get(&(conn.name, p.pin_num));
            let is_hovered = hovered_pin.map_or(false, |(c_name, p_num)| {
                c_name == conn.name && *p_num == p.pin_num
            });

            if let Some(hl_info) = is_hl {
                // Highlighted Pin (Active in loaded project)
                if is_morpho {
                    // Morpho Pin Active Fill (Sky Blue #0284c7)
                    cr.set_source_rgb(0.01, 0.52, 0.78);
                } else {
                    // Arduino Pin Active Fill (Magenta #c026d3)
                    cr.set_source_rgb(0.70, 0.12, 0.75);
                }
                draw_rounded_rectangle(cr, cell_x, cell_y, cell_w, cell_h, 4.0);
                let _ = cr.fill_preserve();

                if is_hovered {
                    cr.set_source_rgb(1.0, 1.0, 1.0);
                    cr.set_line_width(1.8);
                } else if is_morpho {
                    cr.set_source_rgb(0.22, 0.74, 0.97);
                    cr.set_line_width(1.0);
                } else {
                    cr.set_source_rgb(0.94, 0.52, 0.98);
                    cr.set_line_width(1.0);
                }
                let _ = cr.stroke();

                // Pin Number Badge Box inside Active Cell
                if is_morpho {
                    cr.set_source_rgb(0.01, 0.38, 0.60);
                } else {
                    cr.set_source_rgb(0.50, 0.08, 0.55);
                }
                draw_rounded_rectangle(cr, cell_x + 3.0, cell_y + 3.0, 26.0, cell_h - 6.0, 3.0);
                let _ = cr.fill();

                cr.select_font_face("monospace", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
                cr.set_font_size(10.0);
                cr.set_source_rgb(1.0, 1.0, 1.0);
                let pnum_str = format!("{:2}", p.pin_num);
                let _ = cr.move_to(cell_x + 6.0, cell_y + 20.0);
                let _ = cr.show_text(&pnum_str);

                // MCU Pin & Label Text
                cr.set_font_size(10.5);
                let label_part = match p.default_label {
                    Some(lbl) => format!(" ({}", lbl),
                    None => "".to_string(),
                };
                let left_str = format!("{}{}", p.mcu_pin, label_part);
                let _ = cr.move_to(cell_x + 34.0, cell_y + 20.0);
                let _ = cr.show_text(&left_str);

                // Signal Name Text (Right Aligned inside Cell)
                let sig_str = &hl_info.signal;
                if let Ok(ext) = cr.text_extents(sig_str) {
                    let sig_x = (cell_x + cell_w - ext.width() - 6.0).max(cell_x + 95.0);
                    let _ = cr.move_to(sig_x, cell_y + 20.0);
                    let _ = cr.show_text(sig_str);
                }
            } else {
                // Neutral / Unused Pin
                cr.set_source_rgb(0.09, 0.11, 0.13);
                draw_rounded_rectangle(cr, cell_x, cell_y, cell_w, cell_h, 4.0);
                let _ = cr.fill_preserve();

                if is_hovered {
                    cr.set_source_rgb(0.65, 0.65, 0.65);
                    cr.set_line_width(1.4);
                } else {
                    cr.set_source_rgb(0.16, 0.18, 0.22);
                    cr.set_line_width(1.0);
                }
                let _ = cr.stroke();

                // Pin Number Box for Neutral Cell
                if is_hovered {
                    cr.set_source_rgb(0.20, 0.22, 0.26);
                } else {
                    cr.set_source_rgb(0.12, 0.14, 0.17);
                }
                draw_rounded_rectangle(cr, cell_x + 3.0, cell_y + 3.0, 26.0, cell_h - 6.0, 3.0);
                let _ = cr.fill();

                cr.select_font_face("monospace", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
                cr.set_font_size(9.5);
                if is_hovered {
                    cr.set_source_rgb(0.90, 0.90, 0.90);
                } else {
                    cr.set_source_rgb(0.48, 0.50, 0.54);
                }
                let pnum_str = format!("{:2}", p.pin_num);
                let _ = cr.move_to(cell_x + 6.0, cell_y + 19.5);
                let _ = cr.show_text(&pnum_str);

                // MCU Pin & Label Text
                let label_part = match p.default_label {
                    Some(lbl) => format!(" ({})", lbl),
                    None => "".to_string(),
                };
                let left_str = format!("{}{}", p.mcu_pin, label_part);
                let _ = cr.move_to(cell_x + 34.0, cell_y + 19.5);
                let _ = cr.show_text(&left_str);
            }
        }
    }

    // 7. Footer Legend Bar inside Board Outline
    let legend_y = board_y + board_h - 32.0;
    cr.select_font_face("monospace", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(10.0);

    // Morpho Legend
    cr.set_source_rgb(0.01, 0.52, 0.78);
    draw_rounded_rectangle(cr, board_x + 40.0, legend_y, 14.0, 14.0, 3.0);
    let _ = cr.fill();
    cr.set_source_rgb(0.70, 0.80, 0.90);
    let _ = cr.move_to(board_x + 60.0, legend_y + 11.0);
    let _ = cr.show_text("Morpho Header (CN7 / CN10)");

    // Arduino Legend
    cr.set_source_rgb(0.70, 0.12, 0.75);
    draw_rounded_rectangle(cr, board_x + 320.0, legend_y, 14.0, 14.0, 3.0);
    let _ = cr.fill();
    cr.set_source_rgb(0.70, 0.80, 0.90);
    let _ = cr.move_to(board_x + 340.0, legend_y + 11.0);
    let _ = cr.show_text("Arduino Header (CN5 / CN6 / CN8 / CN9)");

    // Active Signal Legend
    cr.set_source_rgb(1.0, 1.0, 1.0);
    draw_rounded_rectangle(cr, board_x + 660.0, legend_y, 14.0, 14.0, 3.0);
    let _ = cr.fill_preserve();
    cr.set_source_rgb(0.22, 0.74, 0.97);
    cr.set_line_width(1.5);
    let _ = cr.stroke();
    cr.set_source_rgb(0.95, 0.95, 0.95);
    let _ = cr.move_to(board_x + 682.0, legend_y + 11.0);
    let _ = cr.show_text("Active Signal in Loaded Project");
}

pub fn draw_rounded_rectangle(cr: &cairo::Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    let pi = std::f64::consts::PI;
    cr.new_sub_path();
    cr.arc(x + w - r, y + r, r, -pi / 2.0, 0.0);
    cr.arc(x + w - r, y + h - r, r, 0.0, pi / 2.0);
    cr.arc(x + r, y + h - r, r, pi / 2.0, pi);
    cr.arc(x + r, y + r, r, pi, 3.0 * pi / 2.0);
    cr.close_path();
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
        let st = state_motion.borrow();
        let highlights = get_active_pin_highlights(&st);

        let mut hit_pin: Option<(&'static str, u8, &'static str, Option<&'static str>)> = None;

        for conn in CONNECTORS {
            for (idx, p) in conn.pins.iter().enumerate() {
                let (cell_x, cell_y, cell_w, cell_h) = get_pin_cell_rect(conn.name, idx);

                if x >= cell_x && x <= cell_x + cell_w && y >= cell_y && y <= cell_y + cell_h {
                    hit_pin = Some((conn.name, p.pin_num, p.mcu_pin, p.default_label));
                    break;
                }
            }
            if hit_pin.is_some() {
                break;
            }
        }

        drop(st);

        let area = &widgets_motion.pinout_drawing_area;

        match hit_pin {
            Some((conn_name, pin_num, mcu_pin, default_label)) => {
                let current_hovered = state_motion.borrow().hovered_pinout_pin.clone();
                let new_hovered = Some((conn_name.to_string(), pin_num));

                if current_hovered != new_hovered {
                    state_motion.borrow_mut().hovered_pinout_pin = new_hovered;

                    let label_str = default_label.map(|l| format!(" ({})", l)).unwrap_or_default();

                    let is_hl = highlights.get(&(conn_name, pin_num));
                    let tooltip = match is_hl {
                        Some(hl) => format!(
                            "Connector {} Pin {}: {}{} — Signal: {}",
                            conn_name, pin_num, mcu_pin, label_str, hl.signal
                        ),
                        None => format!(
                            "Connector {} Pin {}: {}{} — Unused in Project",
                            conn_name, pin_num, mcu_pin, label_str
                        ),
                    };

                    area.set_tooltip_text(Some(&tooltip));
                    area.queue_draw();
                }
            }
            None => {
                let current_hovered = state_motion.borrow().hovered_pinout_pin.clone();
                if current_hovered.is_some() {
                    state_motion.borrow_mut().hovered_pinout_pin = None;
                    area.set_tooltip_text(None);
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
                let (x, y, w, h) = get_pin_cell_rect(conn.name, idx);
                assert!(x >= 40.0, "Pin cell X {} out of bounds for {} pin {}", x, conn.name, p.pin_num);
                assert!(x + w <= 1440.0, "Pin cell X+W {} out of bounds for {} pin {}", x + w, conn.name, p.pin_num);
                assert!(y >= 50.0, "Pin cell Y {} out of bounds for {} pin {}", y, conn.name, p.pin_num);
                assert!(y + h <= 940.0, "Pin cell Y+H {} out of bounds for {} pin {}", y + h, conn.name, p.pin_num);
            }
        }
    }
}
