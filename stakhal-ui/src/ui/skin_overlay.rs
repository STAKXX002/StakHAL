//! Secret Stage skin unlock and header shimmer visual effects.
//!
//! Renders two non-interactive overlay layers driven by GTK4 tick callbacks:
//! - Unlock reveal: captures the prior window appearance into a `gdk::Texture`, applies
//!   the new skin immediately beneath a pass-through `RevealOverlay` widget subclass, and
//!   expands a circular cutout from the window center to the farthest corner plus an 8px
//!   feather width over 750ms (`ease_out_cubic`) via `Snapshot::push_mask(MaskMode::InvertedAlpha)`,
//!   using GSK radial gradient nodes for the rim and trailing rings (no Cairo).
//! - Header shimmer: a thin strip along the header bottom edge (height_request <= 3,
//!   valign Start positioned at the bottom edge) sweeping a gold highlight every 8 seconds
//!   at alpha <= 0.25, stopping its tick callback when inactive or hidden and resuming
//!   on activation.
//!
//! All effects collapse immediately (instant swap, no capture, no overlay, no shimmer)
//! when reduced motion is active via `tokens::motion::is_animations_enabled()`.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

use gtk4::cairo;
use gtk4::gdk;
use gtk4::glib;
use gtk4::graphene;
use gtk4::gsk;
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;
use libadwaita as adw;

use super::tokens;

pub const REVEAL_DURATION_MS: f64 = 750.0;
pub const REVEAL_FEATHER_PX: f64 = 8.0;
pub const REVEAL_RIM_MAX_ALPHA: f64 = 0.6;
pub const REVEAL_OUTER_TRAIL_MAX_ALPHA: f64 = 0.25;
pub const REVEAL_INNER_TRAIL_MAX_ALPHA: f64 = 0.15;

pub const SHIMMER_PERIOD_SECS: f64 = 8.0;
pub const SHIMMER_SWEEP_SECS: f64 = 1.2;
pub const SHIMMER_MAX_ALPHA: f64 = 0.25;
pub const SHIMMER_STRIP_HEIGHT: i32 = 2;
const SHIMMER_FRAME_INTERVAL: Duration = Duration::from_millis(16);

/// Expanding circular hole radius (in logical pixels) at normalized progress `t in [0.0, 1.0]`.
///
/// Uses `ease_out_cubic` from the window center out to the farthest corner plus `REVEAL_FEATHER_PX`.
/// Returns `0.0` for `t <= 0.0`, non-finite inputs, or zero/negative dimensions.
pub fn reveal_radius(t: f64, width: f64, height: f64) -> f64 {
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return 0.0;
    }
    let tc = if t.is_nan() { 0.0 } else { t.clamp(0.0, 1.0) };
    let eased = tokens::motion::ease_out_cubic(tc);
    let max_radius = (0.5 * width).hypot(0.5 * height) + REVEAL_FEATHER_PX;
    eased * max_radius
}

/// Opacity values for the circular reveal's rim and two trailing rings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RingAlphas {
    pub rim: f64,
    pub outer_trail: f64,
    pub inner_trail: f64,
}

fn reveal_ring_envelope(tc: f64, fade_in_end: f64, fade_out_start: f64) -> f64 {
    if tc <= 0.0 || tc >= 1.0 {
        return 0.0;
    }
    let fade_in = if tc < fade_in_end && fade_in_end > 0.0 {
        tokens::motion::ease_out_cubic(tc / fade_in_end)
    } else {
        1.0
    };
    let fade_out = if tc > fade_out_start && fade_out_start < 1.0 {
        let u = (tc - fade_out_start) / (1.0 - fade_out_start);
        (1.0 - tokens::motion::ease_out_cubic(u)).max(0.0)
    } else {
        1.0
    };
    (fade_in * fade_out).clamp(0.0, 1.0)
}

/// Compute bounded ring alphas at normalized progress `t in [0.0, 1.0]`.
pub fn ring_alphas(t: f64) -> RingAlphas {
    let tc = if t.is_nan() { 0.0 } else { t.clamp(0.0, 1.0) };
    RingAlphas {
        rim: (REVEAL_RIM_MAX_ALPHA * reveal_ring_envelope(tc, 0.08, 0.55))
            .clamp(0.0, REVEAL_RIM_MAX_ALPHA),
        outer_trail: (REVEAL_OUTER_TRAIL_MAX_ALPHA * reveal_ring_envelope(tc, 0.14, 0.50))
            .clamp(0.0, REVEAL_OUTER_TRAIL_MAX_ALPHA),
        inner_trail: (REVEAL_INNER_TRAIL_MAX_ALPHA * reveal_ring_envelope(tc, 0.20, 0.45))
            .clamp(0.0, REVEAL_INNER_TRAIL_MAX_ALPHA),
    }
}

/// Capture the current window contents (including window background styling) into a
/// `gdk::Texture` aligned to `target_overlay`, accounting for the display scale factor.
///
/// Returns `None` cleanly if the window or renderer is unrealized or zero-sized.
pub fn capture_window_texture(
    window: &impl IsA<gtk4::Widget>,
    target_overlay: &gtk4::Overlay,
) -> Option<gdk::Texture> {
    let win_widget = window.as_ref();
    if !win_widget.is_realized() || !win_widget.is_visible() {
        return None;
    }
    let win_w = win_widget.width();
    let win_h = win_widget.height();
    if win_w <= 0 || win_h <= 0 {
        return None;
    }

    let native = win_widget.native()?;
    let renderer = native.renderer()?;
    if !renderer.is_realized() {
        return None;
    }

    let scale = native
        .surface()
        .map(|s| s.scale())
        .filter(|&s| s.is_finite() && s > 0.0)
        .unwrap_or_else(|| (win_widget.scale_factor().max(1)) as f64);

    let paintable = gtk4::WidgetPaintable::new(Some(win_widget));
    let snapshot = gtk4::Snapshot::new();
    snapshot.scale(scale as f32, scale as f32);
    paintable.snapshot(&snapshot, win_w as f64, win_h as f64);
    let node = snapshot.to_node()?;

    let bounds = target_overlay
        .compute_bounds(win_widget)
        .filter(|b| b.width() > 0.0 && b.height() > 0.0)?;

    let viewport = graphene::Rect::new(
        bounds.x() * scale as f32,
        bounds.y() * scale as f32,
        bounds.width() * scale as f32,
        bounds.height() * scale as f32,
    );

    let texture = renderer.render_texture(&node, Some(&viewport));
    if texture.width() <= 0 || texture.height() <= 0 {
        return None;
    }
    Some(texture)
}

fn rgba_with_alpha(rgb: (f64, f64, f64), alpha: f64) -> gdk::RGBA {
    gdk::RGBA::new(
        rgb.0 as f32,
        rgb.1 as f32,
        rgb.2 as f32,
        alpha.clamp(0.0, 1.0) as f32,
    )
}

fn build_reveal_mask_stops(radius: f64) -> (f32, Vec<gsk::ColorStop>) {
    if radius <= 0.0 {
        let zero = gdk::RGBA::new(0.0, 0.0, 0.0, 0.0);
        return (
            1.0,
            vec![
                gsk::ColorStop::new(0.0, zero),
                gsk::ColorStop::new(1.0, zero),
            ],
        );
    }

    if radius <= REVEAL_FEATHER_PX {
        let center_a = (radius / REVEAL_FEATHER_PX).clamp(0.0, 1.0) as f32;
        return (
            radius.max(0.5) as f32,
            vec![
                gsk::ColorStop::new(0.0, gdk::RGBA::new(0.0, 0.0, 0.0, center_a)),
                gsk::ColorStop::new(1.0, gdk::RGBA::new(0.0, 0.0, 0.0, 0.0)),
            ],
        );
    }

    let inner_off = ((radius - REVEAL_FEATHER_PX) / radius).clamp(0.0001, 0.9999) as f32;
    let solid = gdk::RGBA::new(0.0, 0.0, 0.0, 1.0);
    let clear = gdk::RGBA::new(0.0, 0.0, 0.0, 0.0);
    (
        radius as f32,
        vec![
            gsk::ColorStop::new(0.0, solid),
            gsk::ColorStop::new(inner_off, solid),
            gsk::ColorStop::new(1.0, clear),
        ],
    )
}

fn append_rim_ring_node(
    snapshot: &gtk4::Snapshot,
    bounds: &graphene::Rect,
    center: &graphene::Point,
    radius: f64,
    alpha: f64,
    pal: &tokens::Palette,
) {
    let alpha = alpha.clamp(0.0, REVEAL_RIM_MAX_ALPHA);
    if radius <= 1.0 || alpha <= 0.001 {
        return;
    }

    let inner_r = (radius - 3.0).max(0.0);
    let core_r = radius;
    let gold_r = radius + 1.5;
    let outer_r = radius + 4.0;

    let azure = pal.azure.rgb();
    let gold = pal.gold.rgb();

    let stops = [
        gsk::ColorStop::new((inner_r / outer_r) as f32, rgba_with_alpha(azure, 0.0)),
        gsk::ColorStop::new((core_r / outer_r) as f32, rgba_with_alpha(azure, alpha)),
        gsk::ColorStop::new((gold_r / outer_r) as f32, rgba_with_alpha(gold, alpha)),
        gsk::ColorStop::new(1.0, rgba_with_alpha(gold, 0.0)),
    ];
    snapshot.append_radial_gradient(
        bounds,
        center,
        outer_r as f32,
        outer_r as f32,
        0.0,
        1.0,
        &stops,
    );
}

fn append_trail_ring_node(
    snapshot: &gtk4::Snapshot,
    bounds: &graphene::Rect,
    center: &graphene::Point,
    radius: f64,
    half_width: f64,
    alpha: f64,
    rgb: (f64, f64, f64),
) {
    if radius <= 1.0 || half_width <= 0.0 || alpha <= 0.001 {
        return;
    }

    let outer_r = radius + half_width;
    let inner_r = (radius - half_width).max(0.0);
    if outer_r <= radius || inner_r >= radius {
        return;
    }

    let stops = [
        gsk::ColorStop::new((inner_r / outer_r) as f32, rgba_with_alpha(rgb, 0.0)),
        gsk::ColorStop::new((radius / outer_r) as f32, rgba_with_alpha(rgb, alpha)),
        gsk::ColorStop::new(1.0, rgba_with_alpha(rgb, 0.0)),
    ];
    snapshot.append_radial_gradient(
        bounds,
        center,
        outer_r as f32,
        outer_r as f32,
        0.0,
        1.0,
        &stops,
    );
}

/// Record the GSK circular reveal mask and gradient ring nodes into `snapshot`.
///
/// Uses `MaskMode::InvertedAlpha` (`source * (1 - mask.alpha)`), so the mask gradient
/// is opaque (`alpha = 1.0`) inside `radius - 8 px` and feathered to transparent outside
/// `radius`, cutting an expanding circular hole through `texture`.
pub fn snapshot_reveal_nodes(
    snapshot: &gtk4::Snapshot,
    texture: &gdk::Texture,
    width: f64,
    height: f64,
    progress: f64,
    pal: &tokens::Palette,
) {
    if width <= 0.0 || height <= 0.0 {
        return;
    }

    let w = width as f32;
    let h = height as f32;
    let bounds = graphene::Rect::new(0.0, 0.0, w, h);
    let center = graphene::Point::new(0.5 * w, 0.5 * h);
    let radius = reveal_radius(progress, width, height);

    snapshot.push_mask(gsk::MaskMode::InvertedAlpha);
    let (grad_r, mask_stops) = build_reveal_mask_stops(radius);
    snapshot.append_radial_gradient(&bounds, &center, grad_r, grad_r, 0.0, 1.0, &mask_stops);
    snapshot.pop();

    snapshot.append_texture(texture, &bounds);
    snapshot.pop();

    let alphas = ring_alphas(progress);
    if radius > 0.0 {
        append_rim_ring_node(snapshot, &bounds, &center, radius, alphas.rim, pal);
        append_trail_ring_node(
            snapshot,
            &bounds,
            &center,
            radius * 0.60,
            2.5,
            alphas.outer_trail,
            pal.azure.rgb(),
        );
        append_trail_ring_node(
            snapshot,
            &bounds,
            &center,
            radius * 0.35,
            2.0,
            alphas.inner_trail,
            pal.sky.rgb(),
        );
    }
}

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct RevealOverlay {
        pub texture: RefCell<Option<gdk::Texture>>,
        pub progress: Cell<f64>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RevealOverlay {
        const NAME: &'static str = "StakhalRevealOverlay";
        type Type = super::RevealOverlay;
        type ParentType = gtk4::Widget;
    }

    impl ObjectImpl for RevealOverlay {}

    impl WidgetImpl for RevealOverlay {
        fn snapshot(&self, snapshot: &gtk4::Snapshot) {
            let widget = self.obj();
            let w = widget.width() as f64;
            let h = widget.height() as f64;
            if w <= 0.0 || h <= 0.0 {
                return;
            }
            let tex_borrow = self.texture.borrow();
            let Some(texture) = tex_borrow.as_ref() else {
                return;
            };
            snapshot_reveal_nodes(
                snapshot,
                texture,
                w,
                h,
                self.progress.get(),
                &tokens::Palette::STAGE,
            );
        }
    }
}

glib::wrapper! {
    /// Pass-through widget displaying the pre-unlock texture while a circular reveal expands.
    pub struct RevealOverlay(ObjectSubclass<imp::RevealOverlay>)
        @extends gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl RevealOverlay {
    pub fn new(texture: gdk::Texture) -> Self {
        let obj: Self = glib::Object::builder().build();
        obj.imp().texture.replace(Some(texture));
        obj.set_hexpand(true);
        obj.set_vexpand(true);
        obj.set_can_target(false);
        obj.set_can_focus(false);
        obj
    }

    pub fn set_progress(&self, progress: f64) {
        self.imp().progress.set(progress);
        self.queue_draw();
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

/// Manages the Stage circular reveal transition and header bottom-edge shimmer overlays.
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
    /// retaining `skin_overlay` for the one-shot circular unlock reveal.
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
            // Reduced motion: no capture, no overlay, instant swap, no shimmer.
            (self.on_skin_swap)();
            return;
        }

        let Some(old_texture) = capture_window_texture(&self.window, &self.skin_overlay) else {
            // Fallback when capture cannot run (e.g. unrealized or zero-size window):
            // swap skin immediately without an overlay, never panic.
            (self.on_skin_swap)();
            self.effects_active.set(true);
            self.sync_loop_state();
            return;
        };

        let reveal_overlay = RevealOverlay::new(old_texture);
        self.skin_overlay.add_overlay(&reveal_overlay);
        reveal_overlay.set_progress(0.0);

        // Apply the new skin immediately beneath the old-texture reveal overlay.
        (self.on_skin_swap)();
        self.effects_active.set(true);
        self.sync_loop_state();

        let start_time: Rc<Cell<Option<Instant>>> = Rc::new(Cell::new(None));
        let overlay_weak = self.skin_overlay.downgrade();
        let ctrl_tick = self.clone();

        reveal_overlay.add_tick_callback(move |overlay, _| {
            let now = Instant::now();
            let start = match start_time.get() {
                Some(s) => s,
                None => {
                    start_time.set(Some(now));
                    now
                }
            };

            let Some(ov) = overlay_weak.upgrade() else {
                return glib::ControlFlow::Break;
            };

            if ctrl_tick.closed.get() || !tokens::motion::is_animations_enabled() {
                ov.remove_overlay(overlay);
                return glib::ControlFlow::Break;
            }

            let elapsed_ms = now.saturating_duration_since(start).as_secs_f64() * 1000.0;
            let progress = (elapsed_ms / REVEAL_DURATION_MS).clamp(0.0, 1.0);
            overlay.set_progress(progress);

            if elapsed_ms >= REVEAL_DURATION_MS {
                ov.remove_overlay(overlay);
                return glib::ControlFlow::Break;
            }

            glib::ControlFlow::Continue
        });
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
    fn test_reveal_radius_progression_and_edge_cases() {
        let w = 1280.0;
        let h = 820.0;
        assert_eq!(reveal_radius(0.0, w, h), 0.0);
        assert_eq!(reveal_radius(-0.25, w, h), 0.0);

        // Full coverage at t=1.0: reaches farthest corner from center plus feather width
        let farthest_corner = (0.5 * w).hypot(0.5 * h);
        let r_end = reveal_radius(1.0, w, h);
        assert!(
            r_end >= farthest_corner + REVEAL_FEATHER_PX - 1e-6,
            "reveal_radius at t=1 must cover farthest corner ({farthest_corner}) + feather ({REVEAL_FEATHER_PX}), got {r_end}"
        );
        assert!((r_end - (farthest_corner + REVEAL_FEATHER_PX)).abs() < 1e-6);

        // Ease-out cubic property at midpoint: ease_out_cubic(0.5) == 0.875
        let r_mid = reveal_radius(0.5, w, h);
        assert!((r_mid - 0.875 * r_end).abs() < 1e-6);

        // Monotonically non-decreasing over [0.0, 1.0]
        let mut prev = 0.0;
        for step in 0..=100 {
            let t = step as f64 / 100.0;
            let r = reveal_radius(t, w, h);
            assert!(r.is_finite());
            assert!(
                r >= prev,
                "reveal_radius must be monotonic at t={t}: {r} < {prev}"
            );
            prev = r;
        }

        // Clamps above 1.0 cleanly
        assert!((reveal_radius(2.5, w, h) - r_end).abs() < 1e-6);

        // Zero-size, negative, and non-finite inputs remain finite (0.0) without panicking
        for (tw, th) in [
            (0.0, 0.0),
            (0.0, 300.0),
            (400.0, 0.0),
            (-10.0, 200.0),
            (f64::NAN, 100.0),
            (f64::INFINITY, 100.0),
        ] {
            let r = reveal_radius(0.5, tw, th);
            assert!(r.is_finite(), "radius for ({tw}, {th}) must be finite");
            assert_eq!(r, 0.0);
        }
        assert_eq!(reveal_radius(f64::NAN, w, h), 0.0);
        assert!((reveal_radius(f64::INFINITY, w, h) - r_end).abs() < 1e-6);
    }

    #[test]
    fn test_ring_alphas_bounds_and_finite_inputs() {
        let a0 = ring_alphas(0.0);
        assert_eq!(a0.rim, 0.0);
        assert_eq!(a0.outer_trail, 0.0);
        assert_eq!(a0.inner_trail, 0.0);

        let a1 = ring_alphas(1.0);
        assert_eq!(a1.rim, 0.0);
        assert_eq!(a1.outer_trail, 0.0);
        assert_eq!(a1.inner_trail, 0.0);

        let mut max_rim = 0.0_f64;
        let mut max_outer = 0.0_f64;
        let mut max_inner = 0.0_f64;
        for step in 0..=200 {
            let t = step as f64 / 200.0;
            let a = ring_alphas(t);
            assert!(a.rim.is_finite() && (0.0..=0.6).contains(&a.rim));
            assert!(a.outer_trail.is_finite() && (0.0..=0.25).contains(&a.outer_trail));
            assert!(a.inner_trail.is_finite() && (0.0..=0.15).contains(&a.inner_trail));
            max_rim = max_rim.max(a.rim);
            max_outer = max_outer.max(a.outer_trail);
            max_inner = max_inner.max(a.inner_trail);
        }
        assert!(max_rim > 0.5 && max_rim <= 0.6);
        assert!(max_outer > 0.2 && max_outer <= 0.25);
        assert!(max_inner > 0.1 && max_inner <= 0.15);

        for extreme in [-1.0, 2.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let a = ring_alphas(extreme);
            assert!(a.rim.is_finite() && (0.0..=0.6).contains(&a.rim));
            assert!(a.outer_trail.is_finite() && (0.0..=0.25).contains(&a.outer_trail));
            assert!(a.inner_trail.is_finite() && (0.0..=0.15).contains(&a.inner_trail));
        }
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
    fn test_cairo_draws_shimmer_safely() {
        let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 320, 200).unwrap();
        let cr = cairo::Context::new(&surface).unwrap();
        let stage = tokens::Palette::STAGE;

        draw_header_shimmer(&cr, 320.0, 2.0, 0.5, &stage);

        // Zero dimensions must not panic
        draw_header_shimmer(&cr, 0.0, 0.0, 0.5, &stage);
        draw_header_shimmer(&cr, 320.0, 0.0, 0.5, &stage);
        surface.flush();
    }
}
