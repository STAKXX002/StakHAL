//! Secret Stage skin unlock and ambient visual effects.
//!
//! Renders three non-interactive overlay layers using Cairo and GTK4 tick callbacks:
//! - Unlock ripple: expands from the top-right corner over 450ms (ease-out cubic),
//!   swaps the application skin at 50% progress, and fades out by 900ms.
//! - Ambient bands: three slow drifting azure and accent bands at alpha <= 0.06,
//!   capped near 30 fps by frame skipping, paused when window is inactive or hidden.
//! - Header shimmer: a 1px gold highlight sweeping along the header bottom edge
//!   every 8 seconds at alpha <= 0.25.
//!
//! All effects collapse immediately (instant swap, no ambient, no shimmer) when
//! reduced motion is active via `tokens::motion::is_animations_enabled()`.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

use gtk4::cairo;
use gtk4::glib;
use gtk4::prelude::*;
use libadwaita as adw;

use super::tokens;

pub const RIPPLE_EXPAND_MS: f64 = 450.0;
pub const RIPPLE_TOTAL_MS: f64 = 900.0;
pub const RIPPLE_SWAP_PROGRESS: f64 = 0.5;

pub const AMBIENT_MAX_ALPHA: f64 = 0.06;
pub const AMBIENT_FRAME_INTERVAL: Duration = Duration::from_millis(33);

pub const SHIMMER_PERIOD_SECS: f64 = 8.0;
pub const SHIMMER_SWEEP_SECS: f64 = 1.2;
pub const SHIMMER_MAX_ALPHA: f64 = 0.25;
const SHIMMER_FRAME_INTERVAL: Duration = Duration::from_millis(16);

/// Calculated frame parameters for the unlock ripple overlay.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RippleFrame {
    pub radius_frac: f64,
    pub ring_alpha: f64,
    pub wash_alpha: f64,
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

    let ring_alpha = (0.78 * fade_alpha).clamp(0.0, 1.0);
    let wash_alpha = if clamped_ms <= RIPPLE_EXPAND_MS {
        0.24 * radius_frac
    } else {
        0.24 * fade_alpha
    };

    RippleFrame {
        radius_frac,
        ring_alpha,
        wash_alpha,
        progress,
        should_swap_skin: progress >= RIPPLE_SWAP_PROGRESS,
        finished: clamped_ms >= RIPPLE_TOTAL_MS,
    }
}

/// Draw a single frame of the top-right unlock ripple.
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
    if frame.finished || (frame.ring_alpha <= 0.0 && frame.wash_alpha <= 0.0) {
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
    let (gold_r, gold_g, gold_b) = pal.gold.rgb();

    // Soft radial wash sweeping inward from top-right
    if frame.wash_alpha > 0.0 {
        let inner = (radius * 0.45).max(0.0);
        let wash =
            cairo::RadialGradient::new(origin_x, origin_y, inner, origin_x, origin_y, radius);
        wash.add_color_stop_rgba(0.0, acc_r, acc_g, acc_b, 0.0);
        wash.add_color_stop_rgba(0.75, acc_r, acc_g, acc_b, frame.wash_alpha * 0.55);
        wash.add_color_stop_rgba(1.0, acc_r, acc_g, acc_b, frame.wash_alpha);
        let _ = cr.set_source(&wash);
        cr.arc(origin_x, origin_y, radius, 0.0, std::f64::consts::TAU);
        let _ = cr.fill();
    }

    // Glowing halo + crisp leading ring
    if frame.ring_alpha > 0.0 {
        cr.set_source_rgba(acc_r, acc_g, acc_b, frame.ring_alpha * 0.28);
        cr.set_line_width(16.0);
        cr.new_path();
        cr.arc(origin_x, origin_y, radius, 0.0, std::f64::consts::TAU);
        let _ = cr.stroke();

        cr.set_source_rgba(acc_r, acc_g, acc_b, frame.ring_alpha * 0.85);
        cr.set_line_width(2.5);
        cr.new_path();
        cr.arc(origin_x, origin_y, radius, 0.0, std::f64::consts::TAU);
        let _ = cr.stroke();

        cr.set_source_rgba(gold_r, gold_g, gold_b, frame.ring_alpha * 0.45);
        cr.set_line_width(1.25);
        cr.new_path();
        cr.arc(
            origin_x,
            origin_y,
            (radius - 4.0).max(0.5),
            0.0,
            std::f64::consts::TAU,
        );
        let _ = cr.stroke();
    }
}

/// Pure specification of one drifting ambient light band.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AmbientBandSpec {
    pub color: (f64, f64, f64),
    pub alpha: f64,
    pub center_x: f64,
    pub center_y: f64,
    pub dx: f64,
    pub dy: f64,
}

/// Compute 3 slow drifting azure and accent bands for `phase_secs`, keeping alpha <= 0.06.
pub fn compute_ambient_bands(
    phase_secs: f64,
    width: f64,
    height: f64,
    pal: &tokens::Palette,
) -> [AmbientBandSpec; 3] {
    let w = width.max(1.0);
    let h = height.max(1.0);

    // Azure hue harmonized with the palette border and deep blue tones
    let azure = (40.0 / 255.0, 110.0 / 255.0, 232.0 / 255.0);
    let accent = pal.accent.rgb();
    let azure_accent_mix = (
        azure.0 * 0.5 + accent.0 * 0.5,
        azure.1 * 0.5 + accent.1 * 0.5,
        azure.2 * 0.5 + accent.2 * 0.5,
    );

    let cx0 = w * (0.30 + 0.22 * (phase_secs * 0.075).sin());
    let cy0 = h * (0.28 + 0.14 * (phase_secs * 0.055).cos());

    let cx1 = w * (0.68 + 0.20 * (phase_secs * 0.060 + 2.1).cos());
    let cy1 = h * (0.58 + 0.16 * (phase_secs * 0.048 + 1.0).sin());

    let cx2 = w * (0.48 + 0.25 * (phase_secs * 0.042 + 4.0).sin());
    let cy2 = h * (0.80 + 0.12 * (phase_secs * 0.052 + 2.8).cos());

    let span = w.max(h) * 0.28;

    [
        AmbientBandSpec {
            color: azure,
            alpha: 0.052_f64.min(AMBIENT_MAX_ALPHA),
            center_x: cx0,
            center_y: cy0,
            dx: span * 0.72,
            dy: span * 0.68,
        },
        AmbientBandSpec {
            color: accent,
            alpha: 0.042_f64.min(AMBIENT_MAX_ALPHA),
            center_x: cx1,
            center_y: cy1,
            dx: -span * 0.66,
            dy: span * 0.74,
        },
        AmbientBandSpec {
            color: azure_accent_mix,
            alpha: 0.036_f64.min(AMBIENT_MAX_ALPHA),
            center_x: cx2,
            center_y: cy2,
            dx: span * 0.52,
            dy: span * 0.85,
        },
    ]
}

/// Render the 3 slow drifting azure and accent bands onto `cr`.
pub fn draw_ambient_bands(
    cr: &cairo::Context,
    width: f64,
    height: f64,
    phase_secs: f64,
    pal: &tokens::Palette,
) {
    if width <= 0.0 || height <= 0.0 {
        return;
    }

    let bands = compute_ambient_bands(phase_secs, width, height, pal);
    for band in &bands {
        let alpha = band.alpha.clamp(0.0, AMBIENT_MAX_ALPHA);
        if alpha <= 0.0 {
            continue;
        }
        let (r, g, b) = band.color;
        let grad = cairo::LinearGradient::new(
            band.center_x - band.dx,
            band.center_y - band.dy,
            band.center_x + band.dx,
            band.center_y + band.dy,
        );
        grad.add_color_stop_rgba(0.0, r, g, b, 0.0);
        grad.add_color_stop_rgba(0.5, r, g, b, alpha);
        grad.add_color_stop_rgba(1.0, r, g, b, 0.0);
        let _ = cr.set_source(&grad);
        cr.rectangle(0.0, 0.0, width, height);
        let _ = cr.fill();
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

/// Render the 1px gold shimmer highlight along the bottom edge of the header overlay.
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

/// Manages Stage unlock ripple, ambient background bands, and header shimmer overlays.
#[derive(Clone)]
pub struct SkinOverlayController {
    skin_overlay: gtk4::Overlay,
    window: adw::ApplicationWindow,
    ambient_area: gtk4::DrawingArea,
    shimmer_area: gtk4::DrawingArea,
    unlocked: Rc<Cell<bool>>,
    effects_active: Rc<Cell<bool>>,
    ambient_clock: Rc<RefCell<AnimationClock>>,
    ambient_running: Rc<Cell<bool>>,
    ambient_gen: Rc<Cell<u64>>,
    shimmer_clock: Rc<RefCell<AnimationClock>>,
    shimmer_running: Rc<Cell<bool>>,
    shimmer_gen: Rc<Cell<u64>>,
    shimmer_was_drawing: Rc<Cell<bool>>,
    on_skin_swap: Rc<dyn Fn()>,
}

impl SkinOverlayController {
    /// Wire passive pass-through ambient and shimmer overlays onto `skin_overlay` and `header_overlay`.
    pub fn setup(
        window: &adw::ApplicationWindow,
        skin_overlay: &gtk4::Overlay,
        header_overlay: &gtk4::Overlay,
        on_skin_swap: Rc<dyn Fn()>,
    ) -> Self {
        let ambient_area = gtk4::DrawingArea::builder()
            .hexpand(true)
            .vexpand(true)
            .can_target(false)
            .can_focus(false)
            .build();
        skin_overlay.add_overlay(&ambient_area);

        let shimmer_area = gtk4::DrawingArea::builder()
            .hexpand(true)
            .vexpand(true)
            .can_target(false)
            .can_focus(false)
            .build();
        header_overlay.add_overlay(&shimmer_area);

        let effects_active = Rc::new(Cell::new(false));
        let ambient_clock = Rc::new(RefCell::new(AnimationClock::new(AMBIENT_FRAME_INTERVAL)));
        let shimmer_clock = Rc::new(RefCell::new(AnimationClock::new(SHIMMER_FRAME_INTERVAL)));

        {
            let active_draw = Rc::clone(&effects_active);
            let clk_draw = Rc::clone(&ambient_clock);
            ambient_area.set_draw_func(move |_, cr, w, h| {
                if !active_draw.get() || !tokens::motion::is_animations_enabled() {
                    return;
                }
                let phase = clk_draw.borrow().phase_secs();
                let pal = tokens::palette();
                draw_ambient_bands(cr, w as f64, h as f64, phase, &pal);
            });
        }

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
            ambient_area,
            shimmer_area,
            unlocked: Rc::new(Cell::new(false)),
            effects_active,
            ambient_clock,
            ambient_running: Rc::new(Cell::new(false)),
            ambient_gen: Rc::new(Cell::new(0)),
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
            ctrl_close.effects_active.set(false);
            ctrl_close.stop_loops(Instant::now());
            glib::Propagation::Proceed
        });

        controller
    }

    pub fn ambient_area(&self) -> &gtk4::DrawingArea {
        &self.ambient_area
    }

    pub fn shimmer_area(&self) -> &gtk4::DrawingArea {
        &self.shimmer_area
    }

    pub fn is_effects_active(&self) -> bool {
        self.effects_active.get()
    }

    /// Trigger the skin unlock sequence once. Subsequent calls are no-ops.
    pub fn trigger_unlock(&self) {
        if self.unlocked.replace(true) {
            return;
        }

        if !tokens::motion::is_animations_enabled() {
            // Reduced motion: instant skin swap, no ripple, no ambient, no shimmer.
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

            if frame.finished || overlay_weak.upgrade().is_none() {
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

    /// Synchronize ambient band and header shimmer tick loops with window active/visible state.
    pub fn sync_loop_state(&self) {
        let should_run = self.effects_active.get()
            && tokens::motion::is_animations_enabled()
            && self.window.is_active()
            && self.window.is_visible();

        let now = Instant::now();
        if should_run {
            self.start_ambient_loop(now);
            self.start_shimmer_loop(now);
        } else {
            self.stop_loops(now);
        }
    }

    fn stop_loops(&self, now: Instant) {
        self.ambient_clock.borrow_mut().pause(now);
        self.ambient_running.set(false);
        self.ambient_gen.set(self.ambient_gen.get().wrapping_add(1));

        self.shimmer_clock.borrow_mut().pause(now);
        self.shimmer_running.set(false);
        self.shimmer_gen.set(self.shimmer_gen.get().wrapping_add(1));
        if !tokens::motion::is_animations_enabled() && self.shimmer_was_drawing.replace(false) {
            self.shimmer_area.queue_draw();
            self.ambient_area.queue_draw();
        }
    }

    fn start_ambient_loop(&self, now: Instant) {
        self.ambient_clock.borrow_mut().resume(now);
        if self.ambient_running.replace(true) {
            return;
        }

        let my_gen = self.ambient_gen.get().wrapping_add(1);
        self.ambient_gen.set(my_gen);

        let win_weak = self.window.downgrade();
        let active_flag = Rc::clone(&self.effects_active);
        let running_flag = Rc::clone(&self.ambient_running);
        let gen_cell = Rc::clone(&self.ambient_gen);
        let clock = Rc::clone(&self.ambient_clock);

        self.ambient_area.add_tick_callback(move |area, _| {
            let now = Instant::now();
            let alive = my_gen == gen_cell.get()
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
                if !tokens::motion::is_animations_enabled() {
                    area.queue_draw();
                }
                return glib::ControlFlow::Break;
            }

            if clock.borrow_mut().tick(now) {
                area.queue_draw();
            }
            glib::ControlFlow::Continue
        });
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
        let running_flag = Rc::clone(&self.shimmer_running);
        let gen_cell = Rc::clone(&self.shimmer_gen);
        let clock = Rc::clone(&self.shimmer_clock);
        let was_drawing = Rc::clone(&self.shimmer_was_drawing);

        self.shimmer_area.add_tick_callback(move |area, _| {
            let now = Instant::now();
            let alive = my_gen == gen_cell.get()
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

        // At 225ms (half of 450ms expansion): ease_out_cubic(0.5) = 0.875
        let f_mid_expand = compute_ripple_frame(225.0);
        assert!((f_mid_expand.radius_frac - 0.875).abs() < 1e-6);
        assert!(!f_mid_expand.should_swap_skin);
        assert!(!f_mid_expand.finished);

        // At 450ms (50% progress of 900ms lifecycle): ring fully expanded, swap triggers
        let f_swap = compute_ripple_frame(450.0);
        assert!((f_swap.progress - 0.5).abs() < 1e-6);
        assert!((f_swap.radius_frac - 1.0).abs() < 1e-6);
        assert!(f_swap.should_swap_skin);
        assert!(!f_swap.finished);

        // At 675ms: overlay fading out cleanly
        let f_fade = compute_ripple_frame(675.0);
        assert!(f_fade.ring_alpha < f_swap.ring_alpha);
        assert!(f_fade.ring_alpha > 0.0);
        assert!(f_fade.should_swap_skin);
        assert!(!f_fade.finished);

        // At 900ms: fade complete and finished
        let f_end = compute_ripple_frame(900.0);
        assert_eq!(f_end.ring_alpha, 0.0);
        assert_eq!(f_end.wash_alpha, 0.0);
        assert!(f_end.finished);

        // Out-of-range times clamp cleanly without panics
        let f_neg = compute_ripple_frame(-20.0);
        assert_eq!(f_neg.progress, 0.0);
        assert!(!f_neg.finished);
        let f_over = compute_ripple_frame(1500.0);
        assert_eq!(f_over.progress, 1.0);
        assert_eq!(f_over.ring_alpha, 0.0);
        assert!(f_over.finished);
    }

    #[test]
    fn test_ambient_bands_count_and_alpha_cap() {
        let stage = tokens::Palette::STAGE;
        let bands_0 = compute_ambient_bands(0.0, 1280.0, 820.0, &stage);
        let bands_10 = compute_ambient_bands(10.0, 1280.0, 820.0, &stage);
        assert_ne!(
            bands_0[0].center_x, bands_10[0].center_x,
            "Ambient bands must drift slowly over time"
        );
        for t in [0.0, 2.5, 17.3, 60.0] {
            let bands = compute_ambient_bands(t, 1280.0, 820.0, &stage);
            assert_eq!(bands.len(), 3);
            for band in &bands {
                assert!(band.alpha > 0.0);
                assert!(
                    band.alpha <= 0.06,
                    "Ambient band alpha must be <= 0.06, got {}",
                    band.alpha
                );
            }
        }
    }

    #[test]
    fn test_animation_clock_frame_skip_and_pause_resume() {
        let mut clock = AnimationClock::new(AMBIENT_FRAME_INTERVAL);
        let t0 = Instant::now();

        // Starts paused until resume
        assert!(clock.is_paused());
        assert!(!clock.tick(t0));

        // First frame after resume draws immediately
        clock.resume(t0);
        assert!(!clock.is_paused());
        assert!(clock.tick(t0));

        // At 16ms (~60fps tick), frame is skipped to maintain ~30fps cap
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
    fn test_cairo_draws_ripple_ambient_and_shimmer_safely() {
        let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 320, 200).unwrap();
        let cr = cairo::Context::new(&surface).unwrap();
        let stage = tokens::Palette::STAGE;

        draw_ripple_frame(&cr, 320.0, 200.0, 200.0, &stage);
        draw_ripple_frame(&cr, 320.0, 200.0, 600.0, &stage);
        draw_ambient_bands(&cr, 320.0, 200.0, 3.5, &stage);
        draw_header_shimmer(&cr, 320.0, 48.0, 0.5, &stage);

        // Zero and negative dimensions must not panic
        draw_ripple_frame(&cr, 0.0, 0.0, 100.0, &stage);
        draw_ambient_bands(&cr, -10.0, 200.0, 1.0, &stage);
        draw_header_shimmer(&cr, 320.0, 0.0, 0.5, &stage);
        surface.flush();
    }
}
