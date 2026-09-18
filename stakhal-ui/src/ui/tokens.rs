//! Design token system for StakHAL UI
//!
//! Strict color semantics, typography split (IBM Plex Sans vs JetBrains Mono),
//! 1px hairline borders, and 4/8/12/16/24/32px spacing scale.

#![allow(dead_code)]

pub mod color {
    // Hex string constants for GTK4 CSS and markup
    pub const BG_VOID_HEX: &str = "#0A0D10";
    pub const BG_PANEL_HEX: &str = "#14181C";
    pub const BORDER_HAIR_HEX: &str = "#262C31";
    pub const TEXT_PRIMARY_HEX: &str = "#E4E7EA";
    pub const TEXT_MUTED_HEX: &str = "#6B7378";
    pub const STATE_READY_HEX: &str = "#34D399";
    pub const STATE_ACTIVE_HEX: &str = "#F5A623";
    pub const STATE_ERROR_HEX: &str = "#E5484D";
    pub const ACCENT_HEX: &str = "#4FD1C5";

    // Cairo normalized RGB floating point tuples (0.0 .. 1.0)
    pub const BG_VOID: (f64, f64, f64) = (10.0 / 255.0, 13.0 / 255.0, 16.0 / 255.0);
    pub const BG_PANEL: (f64, f64, f64) = (20.0 / 255.0, 24.0 / 255.0, 28.0 / 255.0);
    pub const BORDER_HAIR: (f64, f64, f64) = (38.0 / 255.0, 44.0 / 255.0, 49.0 / 255.0);
    pub const TEXT_PRIMARY: (f64, f64, f64) = (228.0 / 255.0, 231.0 / 255.0, 234.0 / 255.0);
    pub const TEXT_MUTED: (f64, f64, f64) = (107.0 / 255.0, 115.0 / 255.0, 120.0 / 255.0);
    pub const STATE_READY: (f64, f64, f64) = (52.0 / 255.0, 211.0 / 255.0, 153.0 / 255.0);
    pub const STATE_ACTIVE: (f64, f64, f64) = (245.0 / 255.0, 166.0 / 255.0, 35.0 / 255.0);
    pub const STATE_ERROR: (f64, f64, f64) = (229.0 / 255.0, 72.0 / 255.0, 77.0 / 255.0);
    pub const ACCENT: (f64, f64, f64) = (79.0 / 255.0, 209.0 / 255.0, 197.0 / 255.0);
}

pub mod font {
    pub const SANS_FAMILY: &str = "'IBM Plex Sans', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif";
    pub const MONO_FAMILY: &str = "'JetBrains Mono', 'DejaVu Sans Mono', 'Liberation Mono', monospace";

    pub const CAIRO_SANS: &str = "IBM Plex Sans";
    pub const CAIRO_MONO: &str = "JetBrains Mono";
}

pub mod shape {
    pub const BORDER_RADIUS_ELEMENT: i32 = 2;
    pub const BORDER_RADIUS_CONTAINER: i32 = 0;
    pub const BORDER_WIDTH_HAIR: f64 = 1.0;
}

pub mod spacing {
    pub const SCALE_4: i32 = 4;
    pub const SCALE_8: i32 = 8;
    pub const SCALE_12: i32 = 12;
    pub const SCALE_16: i32 = 16;
    pub const SCALE_24: i32 = 24;
    pub const SCALE_32: i32 = 32;
}
