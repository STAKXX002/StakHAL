//! One-time startup launch animation for StakHAL UI
//!
//! Renders the corner bracket and wordmark motif from packaging/stakhal.svg
//! with an ease-out bracket draw-in and subtle wordmark scale/fade (<700ms total).
//! Plays once per process and collapses to instant when reduced motion is requested.

use std::cell::Cell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use gtk4::cairo;
use gtk4::glib;
use gtk4::prelude::*;

use super::tokens;

static LAUNCH_ANIMATION_PLAYED: AtomicBool = AtomicBool::new(false);

const REVEAL_DURATION_MS: f64 = 350.0;
const HOLD_DURATION_MS: f64 = 70.0;
const FADE_OUT_DURATION_MS: f64 = 140.0;
const TOTAL_DURATION_MS: f64 = REVEAL_DURATION_MS + HOLD_DURATION_MS + FADE_OUT_DURATION_MS; // 560ms (< 700ms)

/// Check whether the launch animation has already run in this process.
pub fn has_launch_animation_played() -> bool {
    LAUNCH_ANIMATION_PLAYED.load(Ordering::SeqCst)
}

/// Reset launch animation state (primarily for automated testing).
#[cfg(test)]
pub fn reset_launch_animation_for_testing() {
    LAUNCH_ANIMATION_PLAYED.store(false, Ordering::SeqCst);
}

/// Sets up the one-time launch animation atop the given overlay container.
/// If reduced motion is requested or the animation has already played,
/// this function is a no-op and the normal window content is displayed immediately.
pub fn setup_launch_overlay(overlay: &gtk4::Overlay) {
    if !tokens::motion::is_animations_enabled() {
        LAUNCH_ANIMATION_PLAYED.store(true, Ordering::SeqCst);
        return;
    }

    if LAUNCH_ANIMATION_PLAYED.swap(true, Ordering::SeqCst) {
        return;
    }

    let drawing_area = gtk4::DrawingArea::builder()
        .hexpand(true)
        .vexpand(true)
        .can_target(false)
        .build();

    let start_time: Rc<Cell<Option<Instant>>> = Rc::new(Cell::new(None));
    let start_time_draw = Rc::clone(&start_time);

    drawing_area.set_draw_func(move |_, cr, width, height| {
        let elapsed_ms = match start_time_draw.get() {
            Some(start) => Instant::now().duration_since(start).as_secs_f64() * 1000.0,
            None => 0.0,
        };

        draw_launch_frame(cr, width as f64, height as f64, elapsed_ms);
    });

    let overlay_weak = overlay.downgrade();
    let area_weak = drawing_area.downgrade();
    let start_time_tick = Rc::clone(&start_time);

    drawing_area.add_tick_callback(move |area, _| {
        let now = Instant::now();
        let start = match start_time_tick.get() {
            Some(s) => s,
            None => {
                start_time_tick.set(Some(now));
                now
            }
        };

        let elapsed_ms = now.duration_since(start).as_secs_f64() * 1000.0;
        if elapsed_ms >= TOTAL_DURATION_MS {
            if let (Some(ov), Some(ar)) = (overlay_weak.upgrade(), area_weak.upgrade()) {
                ov.remove_overlay(&ar);
            }
            return glib::ControlFlow::Break;
        }

        area.queue_draw();
        glib::ControlFlow::Continue
    });

    overlay.add_overlay(&drawing_area);
}

/// Render a single frame of the launch animation given elapsed milliseconds.
pub fn draw_launch_frame(cr: &cairo::Context, width: f64, height: f64, elapsed_ms: f64) {
    if width <= 0.0 || height <= 0.0 {
        return;
    }

    // 1. Overall fade out factor after hold period
    let global_alpha = if elapsed_ms <= REVEAL_DURATION_MS + HOLD_DURATION_MS {
        1.0
    } else {
        let fade_elapsed = elapsed_ms - (REVEAL_DURATION_MS + HOLD_DURATION_MS);
        let t = (fade_elapsed / FADE_OUT_DURATION_MS).clamp(0.0, 1.0);
        1.0 - tokens::motion::ease_out_cubic(t)
    };

    if global_alpha <= 0.0 {
        return;
    }

    // 2. Background void fill
    let (bg_r, bg_g, bg_b) = tokens::color::BG_VOID;
    cr.set_source_rgba(bg_r, bg_g, bg_b, global_alpha);
    cr.rectangle(0.0, 0.0, width, height);
    let _ = cr.fill();

    // 3. Normalized progress for reveal phase
    let reveal_t = (elapsed_ms / REVEAL_DURATION_MS).clamp(0.0, 1.0);
    let reveal_p = tokens::motion::ease_out_cubic(reveal_t);

    let cx = width / 2.0;
    let cy = height / 2.0;

    // Motif dimensions
    let half_size = 84.0;
    let max_arm = 42.0;
    let current_arm = max_arm * reveal_p;
    let offset_slide = 16.0 * (1.0 - reveal_p);

    let (acc_r, acc_g, acc_b) = tokens::color::ACCENT;
    let (text_r, text_g, text_b) = tokens::color::TEXT_PRIMARY;

    // 4. Draw Corner Brackets (stroke width 4.0, ease-out draw in)
    cr.set_line_width(4.0);
    cr.set_line_cap(cairo::LineCap::Square);
    cr.set_line_join(cairo::LineJoin::Miter);
    cr.set_source_rgba(acc_r, acc_g, acc_b, global_alpha);

    // Top-left bracket
    let tl_x = cx - half_size - offset_slide;
    let tl_y = cy - half_size - offset_slide;
    cr.new_path();
    cr.move_to(tl_x, tl_y + current_arm);
    cr.line_to(tl_x, tl_y);
    cr.line_to(tl_x + current_arm, tl_y);
    let _ = cr.stroke();

    // Bottom-right bracket
    let br_x = cx + half_size + offset_slide;
    let br_y = cy + half_size + offset_slide;
    cr.new_path();
    cr.move_to(br_x, br_y - current_arm);
    cr.line_to(br_x, br_y);
    cr.line_to(br_x - current_arm, br_y);
    let _ = cr.stroke();

    // 5. Draw "Stak" and "HAL" wordmark
    let scale = 0.92 + 0.08 * reveal_p;
    let text_alpha = reveal_p * global_alpha;

    cr.save().expect("cairo save");
    let _ = cr.translate(cx, cy);
    let _ = cr.scale(scale, scale);

    cr.select_font_face(
        tokens::font::CAIRO_SANS,
        cairo::FontSlant::Normal,
        cairo::FontWeight::Bold,
    );
    cr.set_font_size(32.0);

    let (stak_advance, hal_advance) = match (cr.text_extents("Stak"), cr.text_extents("HAL")) {
        (Ok(s), Ok(h)) => (s.x_advance(), h.x_advance()),
        _ => (72.0, 62.0),
    };

    let spacing = 6.0;
    let total_w = stak_advance + spacing + hal_advance;
    let text_start_x = -total_w / 2.0;
    let text_baseline_y = 10.0;

    // "Stak" in primary text color
    cr.set_source_rgba(text_r, text_g, text_b, text_alpha);
    let _ = cr.move_to(text_start_x, text_baseline_y);
    let _ = cr.show_text("Stak");

    // "HAL" in accent color
    cr.set_source_rgba(acc_r, acc_g, acc_b, text_alpha);
    let _ = cr.move_to(text_start_x + stak_advance + spacing, text_baseline_y);
    let _ = cr.show_text("HAL");

    cr.restore().expect("cairo restore");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_launch_animation_timing_constants() {
        assert!(TOTAL_DURATION_MS < 700.0, "Total duration must be strictly under 700ms");
        assert_eq!(REVEAL_DURATION_MS, 350.0);
        assert_eq!(HOLD_DURATION_MS, 70.0);
        assert_eq!(FADE_OUT_DURATION_MS, 140.0);
    }

    #[test]
    fn test_draw_launch_frame_zero_and_negative_dimensions() {
        let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 10, 10).unwrap();
        let cr = cairo::Context::new(&surface).unwrap();
        // Should not panic on zero or negative bounds
        draw_launch_frame(&cr, 0.0, 0.0, 100.0);
        draw_launch_frame(&cr, -100.0, -100.0, 100.0);
    }

    #[test]
    fn test_draw_launch_frame_stages() {
        let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 400, 300).unwrap();
        let cr = cairo::Context::new(&surface).unwrap();

        // 1. Initial frame (t=0)
        draw_launch_frame(&cr, 400.0, 300.0, 0.0);

        // 2. Mid reveal (t=175)
        draw_launch_frame(&cr, 400.0, 300.0, 175.0);

        // 3. Full reveal / hold (t=350)
        draw_launch_frame(&cr, 400.0, 300.0, 350.0);

        // 4. Fade out (t=450)
        draw_launch_frame(&cr, 400.0, 300.0, 450.0);

        // 5. Post completion (t=600)
        draw_launch_frame(&cr, 400.0, 300.0, 600.0);
    }

    #[test]
    fn test_one_time_launch_flag_behavior() {
        reset_launch_animation_for_testing();
        assert!(!has_launch_animation_played());

        // First invocation sets flag
        assert!(!LAUNCH_ANIMATION_PLAYED.swap(true, Ordering::SeqCst));
        assert!(has_launch_animation_played());

        // Subsequent check returns true
        assert!(LAUNCH_ANIMATION_PLAYED.swap(true, Ordering::SeqCst));
        assert!(has_launch_animation_played());
    }
}
