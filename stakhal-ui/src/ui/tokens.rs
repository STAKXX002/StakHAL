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
            assert!(mid > 0.5, "Ease-out must advance faster early and decelerate later");

            // Monotonicity and strict bounds (no overshoot / no bounce)
            let mut prev = 0.0;
            for step in 1..=100 {
                let t = step as f64 / 100.0;
                let val = ease_out_cubic(t);
                assert!(val >= prev, "Curve must be monotonically non-decreasing");
                assert!(val >= 0.0 && val <= 1.0, "Curve must strictly stay within [0.0, 1.0], no overshoot");
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

