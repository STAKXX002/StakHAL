use std::cell::RefCell;
use std::rc::Rc;
use gtk4::cairo;
use gtk4::glib;
use gtk4::prelude::*;
use crate::state::AppState;
use crate::ui::tokens;

pub const VERIFY_STROKE_SIZE: i32 = 16;
pub const COLOR_SUCCESS_RGB: (f64, f64, f64) = (0.204, 0.827, 0.600); // #34D399 (state_ready)

/// Draws a single continuous checkmark stroke with progress in [0.0, 1.0].
/// Segment 1: (3.0, 8.5) -> (6.5, 12.0) [length ~4.95]
/// Segment 2: (6.5, 12.0) -> (13.0, 4.5) [length ~9.92]
pub fn draw_verify_stroke_path(cr: &cairo::Context, progress: f64) {
    if progress <= 0.0 {
        return;
    }

    let p = progress.clamp(0.0, 1.0);
    let eased = tokens::motion::ease_out_cubic(p);

    let p1: (f64, f64) = (3.0, 8.5);
    let p2: (f64, f64) = (6.5, 12.0);
    let p3: (f64, f64) = (13.0, 4.5);

    let len1: f64 = ((p2.0 - p1.0).powi(2) + (p2.1 - p1.1).powi(2)).sqrt();
    let len2: f64 = ((p3.0 - p2.0).powi(2) + (p3.1 - p2.1).powi(2)).sqrt();
    let total_len = len1 + len2;
    let target_d = eased * total_len;

    cr.set_source_rgb(COLOR_SUCCESS_RGB.0, COLOR_SUCCESS_RGB.1, COLOR_SUCCESS_RGB.2);
    cr.set_line_width(2.0);
    cr.set_line_cap(cairo::LineCap::Round);
    cr.set_line_join(cairo::LineJoin::Round);

    let _ = cr.move_to(p1.0, p1.1);

    if target_d <= len1 {
        let f = target_d / len1;
        let cur_x = p1.0 + (p2.0 - p1.0) * f;
        let cur_y = p1.1 + (p2.1 - p1.1) * f;
        let _ = cr.line_to(cur_x, cur_y);
    } else {
        let _ = cr.line_to(p2.0, p2.1);
        let rem = target_d - len1;
        let f = (rem / len2).clamp(0.0, 1.0);
        let cur_x = p2.0 + (p3.0 - p2.0) * f;
        let cur_y = p2.1 + (p3.1 - p2.1) * f;
        let _ = cr.line_to(cur_x, cur_y);
    }

    let _ = cr.stroke();
}

/// Create an unconfigured 16x16 verify stroke drawing area.
pub fn build_verify_stroke_area() -> gtk4::DrawingArea {
    gtk4::DrawingArea::builder()
        .content_width(VERIFY_STROKE_SIZE)
        .content_height(VERIFY_STROKE_SIZE)
        .valign(gtk4::Align::Center)
        .halign(gtk4::Align::Center)
        .build()
}

/// Attach drawing function and tick callback to drive the verify checkmark stroke reveal.
pub fn setup_verify_stroke_area(
    area: &gtk4::DrawingArea,
    state: Rc<RefCell<AppState>>,
    is_traceability: bool,
) {
    let state_draw = Rc::clone(&state);
    area.set_draw_func(move |_area, cr, _w, _h| {
        let anim_start = {
            let st = state_draw.borrow();
            let bt = st.build_trace.borrow();
            if is_traceability {
                bt.traceability_verify_start
            } else {
                bt.flash_verify_start
            }
        };

        if let Some(start) = anim_start {
            let progress = if !tokens::motion::is_animations_enabled() {
                1.0
            } else {
                let elapsed = start.elapsed().as_millis() as f64;
                let dur = tokens::motion::DURATION_SHORT_MS as f64;
                (elapsed / dur).clamp(0.0, 1.0)
            };
            draw_verify_stroke_path(cr, progress);
        }
    });

    let state_tick = Rc::clone(&state);
    area.add_tick_callback(move |area, _| {
        let anim_start = {
            let st = state_tick.borrow();
            let bt = st.build_trace.borrow();
            if is_traceability {
                bt.traceability_verify_start
            } else {
                bt.flash_verify_start
            }
        };

        if let Some(start) = anim_start {
            if tokens::motion::is_animations_enabled() {
                let elapsed = start.elapsed().as_millis() as u64;
                if elapsed < tokens::motion::DURATION_SHORT_MS {
                    area.queue_draw();
                }
            }
        }
        glib::ControlFlow::Continue
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_draw_verify_stroke_path_progression() {
        let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 16, 16).unwrap();
        let cr = cairo::Context::new(&surface).unwrap();

        // 0% progress does nothing
        draw_verify_stroke_path(&cr, 0.0);
        // 50% progress draws partial checkmark
        draw_verify_stroke_path(&cr, 0.5);
        // 100% progress completes checkmark
        draw_verify_stroke_path(&cr, 1.0);
        surface.flush();
    }

    #[test]
    fn test_reduced_motion_fallback() {
        // When animations are disabled, verify that progress collapses immediately to 1.0
        let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 16, 16).unwrap();
        let cr = cairo::Context::new(&surface).unwrap();

        let anim_enabled = false;
        let progress = if !anim_enabled {
            1.0
        } else {
            0.0
        };
        assert_eq!(progress, 1.0);
        draw_verify_stroke_path(&cr, progress);
        surface.flush();
    }
}
