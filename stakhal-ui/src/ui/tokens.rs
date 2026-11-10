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

/// Token representing a single color in both hex and normalized RGB formats.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorToken {
    pub hex: &'static str,
    pub rgb: (f64, f64, f64),
}

impl ColorToken {
    pub const fn new(hex: &'static str, rgb: (f64, f64, f64)) -> Self {
        Self { hex, rgb }
    }

    pub const fn hex(&self) -> &'static str {
        self.hex
    }

    pub const fn rgb(&self) -> (f64, f64, f64) {
        self.rgb
    }
}

impl std::ops::Deref for ColorToken {
    type Target = (f64, f64, f64);
    fn deref(&self) -> &Self::Target {
        &self.rgb
    }
}

/// Cohesive color palette for StakHAL UI styling.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    pub bg_void: ColorToken,
    pub bg_panel: ColorToken,
    pub bg_raised: ColorToken,
    pub border_hair: ColorToken,
    pub text_primary: ColorToken,
    pub text_muted: ColorToken,
    pub state_ready: ColorToken,
    pub state_active: ColorToken,
    pub state_error: ColorToken,
    pub accent: ColorToken,
    pub accent_hover: ColorToken,
    pub accent_pressed: ColorToken,
    pub gold: ColorToken,
    pub header: ColorToken,
    pub header_fg: ColorToken,
    pub line: ColorToken,
    pub node_border: ColorToken,
    pub text_strong: ColorToken,
    pub sky: ColorToken,
    pub azure: ColorToken,
    pub pin_active: ColorToken,
}

impl Palette {
    /// The default dark industrial palette.
    pub const DEFAULT: Self = Self {
        bg_void: ColorToken::new(color::BG_VOID_HEX, color::BG_VOID),
        bg_panel: ColorToken::new(color::BG_PANEL_HEX, color::BG_PANEL),
        bg_raised: ColorToken::new("#1A2026", (26.0 / 255.0, 32.0 / 255.0, 38.0 / 255.0)),
        border_hair: ColorToken::new(color::BORDER_HAIR_HEX, color::BORDER_HAIR),
        text_primary: ColorToken::new(color::TEXT_PRIMARY_HEX, color::TEXT_PRIMARY),
        text_muted: ColorToken::new(color::TEXT_MUTED_HEX, color::TEXT_MUTED),
        state_ready: ColorToken::new(color::STATE_READY_HEX, color::STATE_READY),
        state_active: ColorToken::new(color::STATE_ACTIVE_HEX, color::STATE_ACTIVE),
        state_error: ColorToken::new(color::STATE_ERROR_HEX, color::STATE_ERROR),
        accent: ColorToken::new(color::ACCENT_HEX, color::ACCENT),
        accent_hover: ColorToken::new("#5FE3D7", (95.0 / 255.0, 227.0 / 255.0, 215.0 / 255.0)),
        accent_pressed: ColorToken::new("#3BB3A8", (59.0 / 255.0, 179.0 / 255.0, 168.0 / 255.0)),
        gold: ColorToken::new(color::STATE_ACTIVE_HEX, color::STATE_ACTIVE),
        header: ColorToken::new(color::BG_PANEL_HEX, color::BG_PANEL),
        header_fg: ColorToken::new(color::TEXT_PRIMARY_HEX, color::TEXT_PRIMARY),
        line: ColorToken::new(color::BORDER_HAIR_HEX, color::BORDER_HAIR),
        node_border: ColorToken::new(color::BORDER_HAIR_HEX, color::BORDER_HAIR),
        text_strong: ColorToken::new("#ffffff", (1.0, 1.0, 1.0)),
        sky: ColorToken::new(color::BORDER_HAIR_HEX, color::BORDER_HAIR),
        azure: ColorToken::new(color::ACCENT_HEX, color::ACCENT),
        pin_active: ColorToken::new(color::STATE_READY_HEX, color::STATE_READY),
    };

    /// The light Stage secret theme palette.
    pub const STAGE: Self = Self {
        bg_void: ColorToken::new("#F4F9FD", (244.0 / 255.0, 249.0 / 255.0, 253.0 / 255.0)),
        bg_panel: ColorToken::new("#FFFFFF", (1.0, 1.0, 1.0)),
        bg_raised: ColorToken::new("#E8F2F9", (232.0 / 255.0, 242.0 / 255.0, 249.0 / 255.0)),
        border_hair: ColorToken::new("#C3D7E6", (195.0 / 255.0, 215.0 / 255.0, 230.0 / 255.0)),
        text_primary: ColorToken::new("#1B2559", (27.0 / 255.0, 37.0 / 255.0, 89.0 / 255.0)),
        text_muted: ColorToken::new("#4A5B86", (74.0 / 255.0, 91.0 / 255.0, 134.0 / 255.0)),
        state_ready: ColorToken::new("#0A7A66", (10.0 / 255.0, 122.0 / 255.0, 102.0 / 255.0)),
        state_active: ColorToken::new("#8F6700", (143.0 / 255.0, 103.0 / 255.0, 0.0)),
        state_error: ColorToken::new("#C62F4D", (198.0 / 255.0, 47.0 / 255.0, 77.0 / 255.0)),
        accent: ColorToken::new("#2E6DB4", (46.0 / 255.0, 109.0 / 255.0, 180.0 / 255.0)),
        accent_hover: ColorToken::new("#285FA0", (40.0 / 255.0, 95.0 / 255.0, 160.0 / 255.0)),
        accent_pressed: ColorToken::new("#1F4E86", (31.0 / 255.0, 78.0 / 255.0, 134.0 / 255.0)),
        gold: ColorToken::new("#C9A24F", (201.0 / 255.0, 162.0 / 255.0, 79.0 / 255.0)),
        header: ColorToken::new("#1F2960", (31.0 / 255.0, 41.0 / 255.0, 96.0 / 255.0)),
        header_fg: ColorToken::new("#F4F9FD", (244.0 / 255.0, 249.0 / 255.0, 253.0 / 255.0)),
        line: ColorToken::new("#4F76AE", (79.0 / 255.0, 118.0 / 255.0, 174.0 / 255.0)),
        node_border: ColorToken::new("#3F5FA0", (63.0 / 255.0, 95.0 / 255.0, 160.0 / 255.0)),
        text_strong: ColorToken::new("#1B2559", (27.0 / 255.0, 37.0 / 255.0, 89.0 / 255.0)),
        sky: ColorToken::new("#A0D3EA", (160.0 / 255.0, 211.0 / 255.0, 234.0 / 255.0)),
        azure: ColorToken::new("#59A0CF", (89.0 / 255.0, 160.0 / 255.0, 207.0 / 255.0)),
        pin_active: ColorToken::new("#2E6DB4", (46.0 / 255.0, 109.0 / 255.0, 180.0 / 255.0)),
    };
}

thread_local! {
    static CURRENT_PALETTE: std::cell::RefCell<Palette> = const { std::cell::RefCell::new(Palette::DEFAULT) };
}

/// Retrieve the active thread-local palette.
pub fn palette() -> Palette {
    CURRENT_PALETTE.with(|p| *p.borrow())
}

/// Set the active thread-local palette.
pub fn set_palette(palette: Palette) {
    CURRENT_PALETTE.with(|p| *p.borrow_mut() = palette);
}

pub mod font {
    pub const SANS_FAMILY: &str = "'IBM Plex Sans', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif";
    pub const MONO_FAMILY: &str =
        "'JetBrains Mono', 'DejaVu Sans Mono', 'Liberation Mono', monospace";

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

pub mod motion {
    use gtk4::gio;
    use gtk4::prelude::SettingsExt;

    /// Micro-interaction duration (~100ms) for button hover/press states
    pub const DURATION_MICRO_MS: u64 = 100;
    /// Short duration (~180ms) for view transitions, status chip changes, and node click flashes
    pub const DURATION_SHORT_MS: u64 = 180;
    /// Medium duration (~350ms) for diagram load-in effects and startup reveals
    pub const DURATION_MEDIUM_MS: u64 = 350;

    /// CSS easing string: strictly ease-out curve with no bounce or overshoot
    pub const EASING_CSS: &str = "cubic-bezier(0.0, 0.0, 0.2, 1.0)";

    /// Pure ease-out cubic curve: f(t) = 1 - (1 - t)^3 for t in [0.0, 1.0].
    /// Strictly no spring, bounce, or overshoot. Monotonically increasing,
    /// decelerating smoothly to 1.0 at t = 1.0.
    pub fn ease_out_cubic(t: f64) -> f64 {
        let clamped = t.clamp(0.0, 1.0);
        1.0 - (1.0 - clamped).powi(3)
    }

    /// Check whether system animation is enabled.
    /// Respects GTK settings (gtk-enable-animations) and GSettings (org.gnome.desktop.interface enable-animations).
    /// If animations are disabled, all animations collapse to instant (0ms).
    pub fn is_animations_enabled() -> bool {
        // 1. Check GTK4 settings if GTK is initialized and default settings exist
        if gtk4::is_initialized_main_thread() {
            if let Some(settings) = gtk4::Settings::default() {
                if !settings.is_gtk_enable_animations() {
                    return false;
                }
            }
        }

        // 2. Check GSettings schema org.gnome.desktop.interface if present
        if let Some(source) = gio::SettingsSchemaSource::default() {
            if source.lookup("org.gnome.desktop.interface", true).is_some() {
                let gsettings = gio::Settings::new("org.gnome.desktop.interface");
                if !gsettings.boolean("enable-animations") {
                    return false;
                }
            }
        }

        true
    }

    /// Return effective animation duration in milliseconds, collapsing to 0 if animations are disabled.
    pub fn effective_duration_ms(base_duration_ms: u64) -> u64 {
        if is_animations_enabled() {
            base_duration_ms
        } else {
            0
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_motion_constants() {
            assert_eq!(DURATION_MICRO_MS, 100);
            assert_eq!(DURATION_SHORT_MS, 180);
            assert_eq!(DURATION_MEDIUM_MS, 350);
            assert_eq!(EASING_CSS, "cubic-bezier(0.0, 0.0, 0.2, 1.0)");
        }

        #[test]
        fn test_ease_out_cubic_properties() {
            // Boundary values
            assert!((ease_out_cubic(0.0) - 0.0).abs() < 1e-6);
            assert!((ease_out_cubic(1.0) - 1.0).abs() < 1e-6);

            // Clamping
            assert!((ease_out_cubic(-0.5) - 0.0).abs() < 1e-6);
            assert!((ease_out_cubic(1.5) - 1.0).abs() < 1e-6);

            // Ease-out deceleration: at midpoint (t=0.5), progress should be well ahead of linear (1 - 0.5^3 = 0.875)
            let mid = ease_out_cubic(0.5);
            assert!((mid - 0.875).abs() < 1e-6);
            assert!(
                mid > 0.5,
                "Ease-out must advance faster early and decelerate later"
            );

            // Monotonicity and strict bounds (no overshoot / no bounce)
            let mut prev = 0.0;
            for step in 1..=100 {
                let t = step as f64 / 100.0;
                let val = ease_out_cubic(t);
                assert!(val >= prev, "Curve must be monotonically non-decreasing");
                assert!(
                    (0.0..=1.0).contains(&val),
                    "Curve must strictly stay within [0.0, 1.0], no overshoot"
                );
                prev = val;
            }
        }

        #[test]
        fn test_effective_duration_behavior() {
            let enabled = is_animations_enabled();
            if enabled {
                assert_eq!(effective_duration_ms(DURATION_MICRO_MS), 100);
                assert_eq!(effective_duration_ms(DURATION_SHORT_MS), 180);
                assert_eq!(effective_duration_ms(DURATION_MEDIUM_MS), 350);
            } else {
                assert_eq!(effective_duration_ms(DURATION_MICRO_MS), 0);
                assert_eq!(effective_duration_ms(DURATION_SHORT_MS), 0);
                assert_eq!(effective_duration_ms(DURATION_MEDIUM_MS), 0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_palette_default_equals_constants() {
        let def = Palette::DEFAULT;
        assert_eq!(def.bg_void.hex(), color::BG_VOID_HEX);
        assert_eq!(def.bg_void.rgb(), color::BG_VOID);
        assert_eq!(def.bg_panel.hex(), color::BG_PANEL_HEX);
        assert_eq!(def.bg_panel.rgb(), color::BG_PANEL);
        assert_eq!(def.border_hair.hex(), color::BORDER_HAIR_HEX);
        assert_eq!(def.border_hair.rgb(), color::BORDER_HAIR);
        assert_eq!(def.text_primary.hex(), color::TEXT_PRIMARY_HEX);
        assert_eq!(def.text_primary.rgb(), color::TEXT_PRIMARY);
        assert_eq!(def.text_muted.hex(), color::TEXT_MUTED_HEX);
        assert_eq!(def.text_muted.rgb(), color::TEXT_MUTED);
        assert_eq!(def.state_ready.hex(), color::STATE_READY_HEX);
        assert_eq!(def.state_ready.rgb(), color::STATE_READY);
        assert_eq!(def.state_active.hex(), color::STATE_ACTIVE_HEX);
        assert_eq!(def.state_active.rgb(), color::STATE_ACTIVE);
        assert_eq!(def.state_error.hex(), color::STATE_ERROR_HEX);
        assert_eq!(def.state_error.rgb(), color::STATE_ERROR);
        assert_eq!(def.accent.hex(), color::ACCENT_HEX);
        assert_eq!(def.accent.rgb(), color::ACCENT);

        assert_eq!(def.bg_raised.hex(), "#1A2026");
        assert_eq!(def.accent_hover.hex(), "#5FE3D7");
        assert_eq!(def.accent_pressed.hex(), "#3BB3A8");
        assert_eq!(def.gold.hex(), color::STATE_ACTIVE_HEX);
        assert_eq!(def.gold.rgb(), color::STATE_ACTIVE);
        assert_eq!(def.header.hex(), color::BG_PANEL_HEX);
        assert_eq!(def.header.rgb(), color::BG_PANEL);
        assert_eq!(def.header_fg.hex(), color::TEXT_PRIMARY_HEX);
        assert_eq!(def.header_fg.rgb(), color::TEXT_PRIMARY);
        assert_eq!(def.line.hex(), color::BORDER_HAIR_HEX);
        assert_eq!(def.line.rgb(), color::BORDER_HAIR);
        assert_eq!(def.node_border.hex(), color::BORDER_HAIR_HEX);
        assert_eq!(def.node_border.rgb(), color::BORDER_HAIR);
        assert_eq!(def.text_strong.hex(), "#ffffff");
        assert_eq!(def.text_strong.rgb(), (1.0, 1.0, 1.0));
        assert_eq!(def.sky.hex(), color::BORDER_HAIR_HEX);
        assert_eq!(def.sky.rgb(), color::BORDER_HAIR);
        assert_eq!(def.azure.hex(), color::ACCENT_HEX);
        assert_eq!(def.azure.rgb(), color::ACCENT);
        assert_eq!(def.pin_active.hex(), color::STATE_READY_HEX);
        assert_eq!(def.pin_active.rgb(), color::STATE_READY);

        let stage = Palette::STAGE;
        assert_eq!(stage.text_muted.hex(), "#4A5B86");
        assert_eq!(stage.sky.hex(), "#A0D3EA");
        assert_eq!(stage.azure.hex(), "#59A0CF");
        assert_eq!(stage.pin_active, stage.accent);

        // Thread local get and set
        assert_eq!(palette(), Palette::DEFAULT);
        set_palette(Palette::STAGE);
        assert_eq!(palette(), Palette::STAGE);
        set_palette(Palette::DEFAULT);
        assert_eq!(palette(), Palette::DEFAULT);
    }

    fn srgb_to_linear(c: f64) -> f64 {
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }

    fn relative_luminance(rgb: (f64, f64, f64)) -> f64 {
        let r = srgb_to_linear(rgb.0);
        let g = srgb_to_linear(rgb.1);
        let b = srgb_to_linear(rgb.2);
        0.2126 * r + 0.7152 * g + 0.0722 * b
    }

    fn contrast_ratio(c1: (f64, f64, f64), c2: (f64, f64, f64)) -> f64 {
        let l1 = relative_luminance(c1);
        let l2 = relative_luminance(c2);
        let lighter = l1.max(l2);
        let darker = l1.min(l2);
        (lighter + 0.05) / (darker + 0.05)
    }

    fn mix_rgb(a: (f64, f64, f64), b: (f64, f64, f64), t: f64) -> (f64, f64, f64) {
        (
            a.0 * (1.0 - t) + b.0 * t,
            a.1 * (1.0 - t) + b.1 * t,
            a.2 * (1.0 - t) + b.2 * t,
        )
    }

    #[test]
    fn test_stage_palette_wcag_contrast() {
        let s = Palette::STAGE;
        let win_end = mix_rgb(s.bg_void.rgb(), s.azure.rgb(), 0.28);
        let checks = [
            (
                "text_primary vs bg_void",
                s.text_primary.rgb(),
                s.bg_void.rgb(),
                4.5,
            ),
            (
                "text_primary vs bg_panel",
                s.text_primary.rgb(),
                s.bg_panel.rgb(),
                4.5,
            ),
            (
                "text_primary vs bg_raised",
                s.text_primary.rgb(),
                s.bg_raised.rgb(),
                4.5,
            ),
            (
                "text_primary vs window end",
                s.text_primary.rgb(),
                win_end,
                4.5,
            ),
            (
                "text_muted vs bg_void",
                s.text_muted.rgb(),
                s.bg_void.rgb(),
                4.5,
            ),
            (
                "text_muted vs bg_panel",
                s.text_muted.rgb(),
                s.bg_panel.rgb(),
                4.5,
            ),
            (
                "text_muted vs bg_raised",
                s.text_muted.rgb(),
                s.bg_raised.rgb(),
                4.5,
            ),
            ("text_muted vs window end", s.text_muted.rgb(), win_end, 4.5),
            ("accent vs bg_void", s.accent.rgb(), s.bg_void.rgb(), 4.5),
            ("accent vs bg_panel", s.accent.rgb(), s.bg_panel.rgb(), 4.5),
            ("bg_void vs accent", s.bg_void.rgb(), s.accent.rgb(), 4.5),
            (
                "bg_void vs accent_hover",
                s.bg_void.rgb(),
                s.accent_hover.rgb(),
                4.5,
            ),
            (
                "bg_void vs accent_pressed",
                s.bg_void.rgb(),
                s.accent_pressed.rgb(),
                4.5,
            ),
            (
                "state_ready vs bg_void",
                s.state_ready.rgb(),
                s.bg_void.rgb(),
                4.5,
            ),
            (
                "state_ready vs bg_panel",
                s.state_ready.rgb(),
                s.bg_panel.rgb(),
                4.5,
            ),
            (
                "state_active vs bg_void",
                s.state_active.rgb(),
                s.bg_void.rgb(),
                4.5,
            ),
            (
                "state_active vs bg_panel",
                s.state_active.rgb(),
                s.bg_panel.rgb(),
                4.5,
            ),
            (
                "state_error vs bg_void",
                s.state_error.rgb(),
                s.bg_void.rgb(),
                4.5,
            ),
            (
                "state_error vs bg_panel",
                s.state_error.rgb(),
                s.bg_panel.rgb(),
                4.5,
            ),
            ("line vs bg_void", s.line.rgb(), s.bg_void.rgb(), 3.0),
            ("line vs bg_panel", s.line.rgb(), s.bg_panel.rgb(), 3.0),
            (
                "node_border vs bg_void",
                s.node_border.rgb(),
                s.bg_void.rgb(),
                3.0,
            ),
            (
                "node_border vs bg_panel",
                s.node_border.rgb(),
                s.bg_panel.rgb(),
                3.0,
            ),
            (
                "header_fg vs header",
                s.header_fg.rgb(),
                s.header.rgb(),
                4.5,
            ),
            ("gold vs header", s.gold.rgb(), s.header.rgb(), 3.0),
        ];

        for (label, fg, bg, min_ratio) in checks {
            let ratio = contrast_ratio(fg, bg);
            assert!(
                ratio >= min_ratio,
                "{label} contrast must be >= {min_ratio}, got {ratio}"
            );
        }
    }
}
