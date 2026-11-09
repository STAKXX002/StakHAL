//! Secret Stage skin unlock and header shimmer visual effects.
//!
//! Renders two non-interactive overlay layers using Cairo and GTK4 tick callbacks:
//! - Unlock ripple: a stroked azure ring with a soft accent inner glow expanding from
//!   the top-right corner over 450ms (ease-out cubic), swapping the application skin at
//!   50% progress, and fading out/removing itself by 900ms with no full-window fill.
//! - Header shimmer: a thin strip along the header bottom edge (height_request <= 3,
//!   valign Start positioned at the bottom edge) sweeping a gold highlight every 8 seconds
//!   at alpha <= 0.25, stopping its tick callback when inactive or hidden and resuming
//!   on activation.
//!
//! All effects collapse immediately (instant swap, no ripple, no shimmer) when
//! reduced motion is active via `tokens::motion::is_animations_enabled()`.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

use gtk4::cairo;
use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;
use libadwaita as adw;

use super::tokens;

pub const RIPPLE_EXPAND_MS: f64 = 450.0;
pub const RIPPLE_TOTAL_MS: f64 = 900.0;
pub const RIPPLE_SWAP_PROGRESS: f64 = 0.5;
pub const RIPPLE_MAX_RING_ALPHA: f64 = 0.35;
pub const RIPPLE_MAX_GLOW_ALPHA: f64 = 0.10;

pub const SHIMMER_PERIOD_SECS: f64 = 8.0;
pub const SHIMMER_SWEEP_SECS: f64 = 1.2;
pub const SHIMMER_MAX_ALPHA: f64 = 0.25;
pub const SHIMMER_STRIP_HEIGHT: i32 = 2;
const SHIMMER_FRAME_INTERVAL: Duration = Duration::from_millis(16);

/// Calculated frame parameters for the unlock ripple overlay.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RippleFrame {
    pub radius_frac: f64,
    pub ring_alpha: f64,
    pub inner_glow_alpha: f64,
    pub progress: f64,
    pub should_swap_skin: bool,
    pub finished: bool,
}

/// Compute ripple state at `elapsed_ms` within `[0.0, RIPPLE_TOTAL_MS]`.
pub fn compute_ripple_frame(elapsed_ms: f64) -> RippleFrame {
    let clamped_ms = elapsed_ms.max(0.0);
    let progress = (clamped_ms / RIPPLE_TOTAL_MS).clamp(0.0, 1.0);
    let expand_t = (clamped_ms / RIPPLE_EXPAND_MS).clamp(0.0, 1.0);
    let radius_frac = tokens::motion::ease_out_cubic(expand_t);

    let fade_duration = (RIPPLE_TOTAL_MS - RIPPLE_EXPAND_MS).max(1.0);
    let fade_alpha = if clamped_ms <= RIPPLE_EXPAND_MS {
        1.0
    } else {
        let fade_t = ((clamped_ms - RIPPLE_EXPAND_MS) / fade_duration).clamp(0.0, 1.0);
        (1.0 - tokens::motion::ease_out_cubic(fade_t)).clamp(0.0, 1.0)
    };

    let ring_alpha = (RIPPLE_MAX_RING_ALPHA * fade_alpha).clamp(0.0, RIPPLE_MAX_RING_ALPHA);
    let inner_glow_alpha = if clamped_ms <= RIPPLE_EXPAND_MS {
        RIPPLE_MAX_GLOW_ALPHA * radius_frac
    } else {
        RIPPLE_MAX_GLOW_ALPHA * fade_alpha
    }
    .clamp(0.0, RIPPLE_MAX_GLOW_ALPHA);

    RippleFrame {
        radius_frac,
        ring_alpha,
        inner_glow_alpha,
        progress,
        should_swap_skin: progress >= RIPPLE_SWAP_PROGRESS,
        finished: clamped_ms >= RIPPLE_TOTAL_MS,
    }
}

/// Draw a single frame of the top-right unlock ripple (stroked ring + soft inner glow only).
pub fn draw_ripple_frame(
    cr: &cairo::Context,
    width: f64,
    height: f64,
    elapsed_ms: f64,
    pal: &tokens::Palette,
) {
    if width <= 0.0 || height <= 0.0 {
        return;
    }

    let frame = compute_ripple_frame(elapsed_ms);
    if frame.finished || (frame.ring_alpha <= 0.0 && frame.inner_glow_alpha <= 0.0) {
        return;
    }

    let origin_x = width;
    let origin_y = 0.0;
    let max_radius = (width * width + height * height).sqrt() * 1.04;
    let radius = max_radius * frame.radius_frac;
    if radius <= 0.5 {
        return;
    }

    let (acc_r, acc_g, acc_b) = pal.accent.rgb();
    // Quadrant inside (0..width, 0..height) relative to top-right origin (width, 0)
    let angle_start = std::f64::consts::FRAC_PI_2 - 0.05;
    let angle_end = std::f64::consts::PI + 0.05;

    // Soft accent inner glow stroke (no full-window fill)
    if frame.inner_glow_alpha > 0.0 {
        let glow_r = (radius - 6.0).max(0.5);
        cr.set_source_rgba(acc_r, acc_g, acc_b, frame.inner_glow_alpha);
        cr.set_line_width(12.0);
        cr.new_path();
        cr.arc(origin_x, origin_y, glow_r, angle_start, angle_end);
        let _ = cr.stroke();
    }

    // Stroked azure leading ring
    if frame.ring_alpha > 0.0 {
        cr.set_source_rgba(acc_r, acc_g, acc_b, frame.ring_alpha);
        cr.set_line_width(2.5);
        cr.new_path();
        cr.arc(origin_x, origin_y, radius, angle_start, angle_end);
        let _ = cr.stroke();
    }
}

/// Pure clock managing frame-rate capping and pause/resume for tick-driven effects.
#[derive(Debug, Clone)]
pub struct AnimationClock {
    accumulated_secs: f64,
    last_step_at: Option<Instant>,
    last_draw_at: Option<Instant>,
    paused: bool,
    min_frame_interval: Duration,
}

impl AnimationClock {
    pub fn new(min_frame_interval: Duration) -> Self {
        Self {
            accumulated_secs: 0.0,
            last_step_at: None,
            last_draw_at: None,
            paused: true,
            min_frame_interval,
        }
    }

    /// Pause clock progression at `now` (accumulating any active delta first).
    pub fn pause(&mut self, now: Instant) {
        if !self.paused {
            if let Some(prev) = self.last_step_at {
                self.accumulated_secs += now.saturating_duration_since(prev).as_secs_f64();
            }
            self.last_step_at = None;
            self.paused = true;
        }
    }

    /// Resume clock progression from `now`, ensuring the next tick draws promptly.
    pub fn resume(&mut self, now: Instant) {
        if self.paused {
            self.paused = false;
            self.last_step_at = Some(now);
            self.last_draw_at = None;
        }
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn phase_secs(&self) -> f64 {
        self.accumulated_secs
    }

    /// Advance phase time and return `true` if a frame should be drawn (not paused and interval met).
    pub fn tick(&mut self, now: Instant) -> bool {
        if self.paused {
            return false;
        }

        if let Some(prev) = self.last_step_at {
            self.accumulated_secs += now.saturating_duration_since(prev).as_secs_f64();
        }
        self.last_step_at = Some(now);

        let should_draw = match self.last_draw_at {
            None => true,
            Some(prev_draw) => now.saturating_duration_since(prev_draw) >= self.min_frame_interval,
        };
        if should_draw {
            self.last_draw_at = Some(now);
        }
        should_draw
    }
}

/// Active header bottom-edge shimmer parameters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShimmerFrame {
    pub center_frac: f64,
    pub half_width_frac: f64,
    pub peak_alpha: f64,
}

/// Compute shimmer sweep state for `elapsed_secs` within an 8-second cycle.
/// Returns `None` outside the active sweep window.
pub fn compute_shimmer_frame(elapsed_secs: f64) -> Option<ShimmerFrame> {
    let t = elapsed_secs.max(0.0).rem_euclid(SHIMMER_PERIOD_SECS);
    if t > SHIMMER_SWEEP_SECS {
        return None;
    }

    let u = (t / SHIMMER_SWEEP_SECS).clamp(0.0, 1.0);
    let eased = tokens::motion::ease_out_cubic(u);
    let center_frac = -0.15 + 1.30 * eased;
    let half_width_frac = 0.14;
    let envelope = (std::f64::consts::PI * u).sin().max(0.0);
    let peak_alpha = (0.22 * envelope).min(SHIMMER_MAX_ALPHA);

    if peak_alpha <= 0.001 {
        return None;
    }

    Some(ShimmerFrame {
        center_frac,
        half_width_frac,
        peak_alpha,
    })
}

/// Render the 1px gold shimmer highlight along the bottom edge of the header strip.
pub fn draw_header_shimmer(
    cr: &cairo::Context,
    width: f64,
    height: f64,
    elapsed_secs: f64,
    pal: &tokens::Palette,
) {
    if width <= 0.0 || height <= 0.0 {
        return;
    }

    let Some(frame) = compute_shimmer_frame(elapsed_secs) else {
        return;
    };

    let (r, g, b) = pal.gold.rgb();
    let cx = frame.center_frac * width;
    let hw = (frame.half_width_frac * width).max(16.0);
    let alpha = frame.peak_alpha.clamp(0.0, SHIMMER_MAX_ALPHA);

    let grad = cairo::LinearGradient::new(cx - hw, 0.0, cx + hw, 0.0);
    grad.add_color_stop_rgba(0.0, r, g, b, 0.0);
    grad.add_color_stop_rgba(0.5, r, g, b, alpha);
    grad.add_color_stop_rgba(1.0, r, g, b, 0.0);

    let _ = cr.set_source(&grad);
    cr.rectangle(0.0, (height - 1.0).max(0.0), width, 1.0);
    let _ = cr.fill();
}

/// Manages the Stage unlock ripple and header bottom-edge shimmer overlays.
#[derive(Clone)]
pub struct SkinOverlayController {
    skin_overlay: gtk4::Overlay,
    window: adw::ApplicationWindow,
    shimmer_area: gtk4::DrawingArea,
    unlocked: Rc<Cell<bool>>,
    closed: Rc<Cell<bool>>,
    effects_active: Rc<Cell<bool>>,
    shimmer_clock: Rc<RefCell<AnimationClock>>,
    shimmer_running: Rc<Cell<bool>>,
    shimmer_gen: Rc<Cell<u64>>,
    shimmer_was_drawing: Rc<Cell<bool>>,
    on_skin_swap: Rc<dyn Fn()>,
}

impl SkinOverlayController {
    /// Wire the thin pass-through header shimmer overlay onto `header_overlay`,
    /// retaining `skin_overlay` for the one-shot unlock ripple.
    pub fn setup(
        window: &adw::ApplicationWindow,
        skin_overlay: &gtk4::Overlay,
        header_overlay: &gtk4::Overlay,
        on_skin_swap: Rc<dyn Fn()>,
    ) -> Self {
        let shimmer_area = gtk4::DrawingArea::builder()
            .height_request(SHIMMER_STRIP_HEIGHT)
            .valign(gtk4::Align::Start)
            .vexpand(false)
            .hexpand(true)
            .can_target(false)
            .can_focus(false)
            .build();
        header_overlay.add_overlay(&shimmer_area);
        header_overlay.set_measure_overlay(&shimmer_area, false);

        let shimmer_for_pos = shimmer_area.clone();
        header_overlay.connect_get_child_position(move |ov, child| {
            if child == shimmer_for_pos.upcast_ref::<gtk4::Widget>() {
                let w = ov.width().max(1);
                let h = ov.height().max(SHIMMER_STRIP_HEIGHT);
                let y = (h - SHIMMER_STRIP_HEIGHT).max(0);
                return Some(gdk::Rectangle::new(0, y, w, SHIMMER_STRIP_HEIGHT));
            }
            None
        });

        let effects_active = Rc::new(Cell::new(false));
        let shimmer_clock = Rc::new(RefCell::new(AnimationClock::new(SHIMMER_FRAME_INTERVAL)));

        {
            let active_draw = Rc::clone(&effects_active);
            let clk_draw = Rc::clone(&shimmer_clock);
            shimmer_area.set_draw_func(move |_, cr, w, h| {
                if !active_draw.get() || !tokens::motion::is_animations_enabled() {
                    return;
                }
                let phase = clk_draw.borrow().phase_secs();
                let pal = tokens::palette();
                draw_header_shimmer(cr, w as f64, h as f64, phase, &pal);
            });
        }

        let controller = Self {
            skin_overlay: skin_overlay.clone(),
            window: window.clone(),
            shimmer_area,
            unlocked: Rc::new(Cell::new(false)),
            closed: Rc::new(Cell::new(false)),
            effects_active,
            shimmer_clock,
            shimmer_running: Rc::new(Cell::new(false)),
            shimmer_gen: Rc::new(Cell::new(0)),
            shimmer_was_drawing: Rc::new(Cell::new(false)),
            on_skin_swap,
        };

        let ctrl_active = controller.clone();
        window.connect_is_active_notify(move |_| {
            ctrl_active.sync_loop_state();
        });

        let ctrl_visible = controller.clone();
        window.connect_visible_notify(move |_| {
            ctrl_visible.sync_loop_state();
        });

        let ctrl_close = controller.clone();
        window.connect_close_request(move |_| {
            ctrl_close.closed.set(true);
            ctrl_close.effects_active.set(false);
            ctrl_close.stop_loops(Instant::now());
            glib::Propagation::Proceed
        });

        controller
    }

    pub fn shimmer_area(&self) -> &gtk4::DrawingArea {
        &self.shimmer_area
    }

    pub fn is_effects_active(&self) -> bool {
        self.effects_active.get()
    }

    pub fn is_shimmer_running(&self) -> bool {
        self.shimmer_running.get()
    }

    /// Trigger the skin unlock sequence once. Subsequent calls are no-ops.
    pub fn trigger_unlock(&self) {
        if self.unlocked.replace(true) {
            return;
        }

        if !tokens::motion::is_animations_enabled() {
            // Reduced motion: instant skin swap, no ripple, no shimmer.
            (self.on_skin_swap)();
            return;
        }

        let ripple_area = gtk4::DrawingArea::builder()
            .hexpand(true)
            .vexpand(true)
            .can_target(false)
            .can_focus(false)
            .build();

        let start_time: Rc<Cell<Option<Instant>>> = Rc::new(Cell::new(None));
        let swapped = Rc::new(Cell::new(false));

        {
            let start_draw = Rc::clone(&start_time);
            ripple_area.set_draw_func(move |_, cr, w, h| {
                let elapsed_ms = match start_draw.get() {
                    Some(start) => {
                        Instant::now()
                            .saturating_duration_since(start)
                            .as_secs_f64()
                            * 1000.0
                    }
                    None => 0.0,
                };
                draw_ripple_frame(cr, w as f64, h as f64, elapsed_ms, &tokens::Palette::STAGE);
            });
        }

        let overlay_weak = self.skin_overlay.downgrade();
        let area_weak = ripple_area.downgrade();
        let start_tick = Rc::clone(&start_time);
        let swapped_tick = Rc::clone(&swapped);
        let on_swap = Rc::clone(&self.on_skin_swap);
        let ctrl_tick = self.clone();

        ripple_area.add_tick_callback(move |area, _| {
            let now = Instant::now();
            let start = match start_tick.get() {
                Some(s) => s,
                None => {
                    start_tick.set(Some(now));
                    now
                }
            };

            if ctrl_tick.closed.get() || overlay_weak.upgrade().is_none() {
                if let (Some(ov), Some(ar)) = (overlay_weak.upgrade(), area_weak.upgrade()) {
                    ov.remove_overlay(&ar);
                }
                return glib::ControlFlow::Break;
            }

            if !tokens::motion::is_animations_enabled() {
                if !swapped_tick.replace(true) {
                    (on_swap)();
                }
                if let (Some(ov), Some(ar)) = (overlay_weak.upgrade(), area_weak.upgrade()) {
                    ov.remove_overlay(&ar);
                }
                return glib::ControlFlow::Break;
            }

            let elapsed_ms = now.saturating_duration_since(start).as_secs_f64() * 1000.0;
            let frame = compute_ripple_frame(elapsed_ms);

            if frame.should_swap_skin && !swapped_tick.replace(true) {
                (on_swap)();
                ctrl_tick.effects_active.set(true);
                ctrl_tick.sync_loop_state();
            }

            if frame.finished {
                if let (Some(ov), Some(ar)) = (overlay_weak.upgrade(), area_weak.upgrade()) {
                    ov.remove_overlay(&ar);
                }
                return glib::ControlFlow::Break;
            }

            area.queue_draw();
            glib::ControlFlow::Continue
        });

        self.skin_overlay.add_overlay(&ripple_area);
    }

    /// Synchronize the header shimmer tick loop with window active/visible state.
    pub fn sync_loop_state(&self) {
        let should_run = !self.closed.get()
            && self.effects_active.get()
            && tokens::motion::is_animations_enabled()
            && self.window.is_active()
            && self.window.is_visible();

        let now = Instant::now();
        if should_run {
            self.start_shimmer_loop(now);
        } else {
            self.stop_loops(now);
        }
    }

    fn stop_loops(&self, now: Instant) {
        self.shimmer_clock.borrow_mut().pause(now);
        self.shimmer_running.set(false);
        self.shimmer_gen.set(self.shimmer_gen.get().wrapping_add(1));
        if !tokens::motion::is_animations_enabled() && self.shimmer_was_drawing.replace(false) {
            self.shimmer_area.queue_draw();
        }
    }

    fn start_shimmer_loop(&self, now: Instant) {
        self.shimmer_clock.borrow_mut().resume(now);
        if self.shimmer_running.replace(true) {
            return;
        }

        let my_gen = self.shimmer_gen.get().wrapping_add(1);
        self.shimmer_gen.set(my_gen);

        let win_weak = self.window.downgrade();
        let active_flag = Rc::clone(&self.effects_active);
        let closed_flag = Rc::clone(&self.closed);
        let running_flag = Rc::clone(&self.shimmer_running);
        let gen_cell = Rc::clone(&self.shimmer_gen);
        let clock = Rc::clone(&self.shimmer_clock);
        let was_drawing = Rc::clone(&self.shimmer_was_drawing);

        self.shimmer_area.add_tick_callback(move |area, _| {
            let now = Instant::now();
            let alive = my_gen == gen_cell.get()
                && !closed_flag.get()
                && active_flag.get()
                && tokens::motion::is_animations_enabled()
                && win_weak
                    .upgrade()
                    .is_some_and(|w| w.is_active() && w.is_visible());

            if !alive {
                clock.borrow_mut().pause(now);
                if my_gen == gen_cell.get() {
                    running_flag.set(false);
                }
                if !tokens::motion::is_animations_enabled() && was_drawing.replace(false) {
                    area.queue_draw();
                }
                return glib::ControlFlow::Break;
            }

            let should_step = clock.borrow_mut().tick(now);
            if should_step {
                let phase = clock.borrow().phase_secs();
                if compute_shimmer_frame(phase).is_some() {
                    was_drawing.set(true);
                    area.queue_draw();
                } else if was_drawing.replace(false) {
                    area.queue_draw();
                }
            }
            glib::ControlFlow::Continue
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ripple_frame_timeline_and_swap_midpoint() {
        let f0 = compute_ripple_frame(0.0);
        assert_eq!(f0.radius_frac, 0.0);
        assert!(!f0.should_swap_skin);
        assert!(!f0.finished);
        assert!(f0.ring_alpha <= RIPPLE_MAX_RING_ALPHA);
        assert!(f0.inner_glow_alpha <= RIPPLE_MAX_GLOW_ALPHA);

        // At 225ms (half of 450ms expansion): ease_out_cubic(0.5) = 0.875
        let f_mid_expand = compute_ripple_frame(225.0);
        assert!((f_mid_expand.radius_frac - 0.875).abs() < 1e-6);
        assert!(!f_mid_expand.should_swap_skin);
        assert!(!f_mid_expand.finished);
        assert!(f_mid_expand.ring_alpha <= 0.35);
        assert!(f_mid_expand.inner_glow_alpha <= 0.10);

        // At 450ms (50% progress of 900ms lifecycle): ring fully expanded, swap triggers
        let f_swap = compute_ripple_frame(450.0);
        assert!((f_swap.progress - 0.5).abs() < 1e-6);
        assert!((f_swap.radius_frac - 1.0).abs() < 1e-6);
        assert!(f_swap.should_swap_skin);
        assert!(!f_swap.finished);
        assert!(f_swap.ring_alpha <= 0.35);
        assert!(f_swap.inner_glow_alpha <= 0.10);

        // At 675ms: overlay fading out cleanly
        let f_fade = compute_ripple_frame(675.0);
        assert!(f_fade.ring_alpha < f_swap.ring_alpha);
        assert!(f_fade.ring_alpha > 0.0);
        assert!(f_fade.inner_glow_alpha < f_swap.inner_glow_alpha);
        assert!(f_fade.should_swap_skin);
        assert!(!f_fade.finished);

        // At 900ms: fade complete and finished
        let f_end = compute_ripple_frame(900.0);
        assert_eq!(f_end.ring_alpha, 0.0);
        assert_eq!(f_end.inner_glow_alpha, 0.0);
        assert!(f_end.finished);

        // Out-of-range times clamp cleanly without panics
        let f_neg = compute_ripple_frame(-20.0);
        assert_eq!(f_neg.progress, 0.0);
        assert!(!f_neg.finished);
        let f_over = compute_ripple_frame(1500.0);
        assert_eq!(f_over.progress, 1.0);
        assert_eq!(f_over.ring_alpha, 0.0);
        assert_eq!(f_over.inner_glow_alpha, 0.0);
        assert!(f_over.finished);
    }

    #[test]
    fn test_draw_ripple_frame_no_full_window_fill() {
        let mut surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 400, 300).unwrap();
        {
            let cr = cairo::Context::new(&surface).unwrap();
            // Render near t=225ms where radius ~ 455px covers almost the entire surface
            draw_ripple_frame(&cr, 400.0, 300.0, 225.0, &tokens::Palette::STAGE);
            surface.flush();
        }
        let stride = surface.stride() as usize;
        let data = surface.data().unwrap();
        // Top-right corner region (x=380, y=20) is well inside the ring radius and must stay transparent (no fill wash)
        let off = 20 * stride + 380 * 4;
        assert_eq!(
            &data[off..off + 4],
            &[0, 0, 0, 0],
            "Ripple must stroke the ring only with no full-window interior fill"
        );
    }

    #[test]
    fn test_animation_clock_frame_skip_and_pause_resume() {
        let mut clock = AnimationClock::new(Duration::from_millis(33));
        let t0 = Instant::now();

        // Starts paused until resume
        assert!(clock.is_paused());
        assert!(!clock.tick(t0));

        // First frame after resume draws immediately
        clock.resume(t0);
        assert!(!clock.is_paused());
        assert!(clock.tick(t0));

        // At 16ms, frame is skipped to maintain ~30fps cap
        let t_16 = t0 + Duration::from_millis(16);
        assert!(!clock.tick(t_16));

        // At 34ms (>= 33ms), frame draws
        let t_34 = t0 + Duration::from_millis(34);
        assert!(clock.tick(t_34));
        let phase_before_pause = clock.phase_secs();
        assert!((phase_before_pause - 0.034).abs() < 1e-6);

        // Pausing for 5 seconds must not advance phase
        clock.pause(t_34);
        assert!(clock.is_paused());
        let t_later = t_34 + Duration::from_secs(5);
        assert!(!clock.tick(t_later));
        assert!((clock.phase_secs() - phase_before_pause).abs() < 1e-6);

        // Resuming draws immediately and starts delta timing from resume instant
        clock.resume(t_later);
        assert!(clock.tick(t_later));
        assert!((clock.phase_secs() - phase_before_pause).abs() < 1e-6);

        let t_after_resume = t_later + Duration::from_millis(40);
        assert!(clock.tick(t_after_resume));
        assert!((clock.phase_secs() - (phase_before_pause + 0.040)).abs() < 1e-6);
    }

    #[test]
    fn test_shimmer_period_and_alpha_cap() {
        // Active sweep during start of 8s cycle
        let active = compute_shimmer_frame(0.6).expect("Shimmer must be active at 0.6s");
        assert!(active.peak_alpha > 0.0);
        assert!(
            active.peak_alpha <= 0.25,
            "Shimmer peak alpha must be <= 0.25, got {}",
            active.peak_alpha
        );

        // Idle gap outside sweep window
        assert!(compute_shimmer_frame(2.0).is_none());
        assert!(compute_shimmer_frame(7.5).is_none());

        // Repeats on 8 second period
        let repeated = compute_shimmer_frame(8.6).expect("Shimmer must repeat at 8.6s");
        assert!((repeated.center_frac - active.center_frac).abs() < 1e-6);
        assert!((repeated.peak_alpha - active.peak_alpha).abs() < 1e-6);
    }

    #[test]
    fn test_cairo_draws_ripple_and_shimmer_safely() {
        let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 320, 200).unwrap();
        let cr = cairo::Context::new(&surface).unwrap();
        let stage = tokens::Palette::STAGE;

        draw_ripple_frame(&cr, 320.0, 200.0, 200.0, &stage);
        draw_ripple_frame(&cr, 320.0, 200.0, 600.0, &stage);
        draw_header_shimmer(&cr, 320.0, 2.0, 0.5, &stage);

        // Zero and negative dimensions must not panic
        draw_ripple_frame(&cr, 0.0, 0.0, 100.0, &stage);
        draw_header_shimmer(&cr, 320.0, 0.0, 0.5, &stage);
        surface.flush();
    }
}
