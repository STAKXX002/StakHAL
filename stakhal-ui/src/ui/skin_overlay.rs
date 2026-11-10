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

pub const REVEAL_CORE_MAX_ALPHA: f64 = 0.85;
pub const REVEAL_GLOW_MAX_ALPHA: f64 = 0.35;
pub const REVEAL_AZURE_TRAIL_MAX_ALPHA: f64 = 0.30;
pub const REVEAL_RIPPLE1_MAX_ALPHA: f64 = 0.20;
pub const REVEAL_RIPPLE2_MAX_ALPHA: f64 = 0.12;

pub const SPARKLE_COUNT: usize = 36;
pub const SPARKLE_LIFETIME_MS: f64 = 420.0;
pub const SPARKLE_MAX_ALPHA: f64 = 0.9;
pub const SPARKLE_DRIFT_PX: f64 = 12.0;
pub const SPARKLE_GOLDEN_ANGLE_DEG: f64 = 137.5078;

pub const FINAL_GLOW_DURATION_MS: f64 = 250.0;
pub const FINAL_GLOW_MAX_ALPHA: f64 = 0.10;

pub const SETTLE_STEP_MS: u64 = 25;
pub const SETTLE_MAX_TOTAL_MS: u64 = 500;
pub const SHINE_DURATION_MS: u64 = 700;

pub const HEADER_FLASH_DURATION_MS: f64 = 600.0;
pub const HEADER_FLASH_MAX_ALPHA: f64 = 0.6;

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

/// Opacity values for the Stage reveal's multi-band golden rim and trailing ripples.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GoldenRingAlphas {
    pub core: f64,
    pub glow: f64,
    pub azure_trail: f64,
    pub ripple1: f64,
    pub ripple2: f64,
}

/// Deterministic four-point star sparkle state along the expanding reveal wavefront.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sparkle {
    pub x: f64,
    pub y: f64,
    pub size: f64,
    pub alpha: f64,
    pub rotation: f64,
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

/// Compute bounded golden rim, outer glow, behind-edge azure trail, and trailing ripple alphas.
pub fn golden_ring_alphas(t: f64) -> GoldenRingAlphas {
    let tc = if t.is_nan() { 0.0 } else { t.clamp(0.0, 1.0) };
    GoldenRingAlphas {
        core: (REVEAL_CORE_MAX_ALPHA * reveal_ring_envelope(tc, 0.08, 0.58))
            .clamp(0.0, REVEAL_CORE_MAX_ALPHA),
        glow: (REVEAL_GLOW_MAX_ALPHA * reveal_ring_envelope(tc, 0.08, 0.58))
            .clamp(0.0, REVEAL_GLOW_MAX_ALPHA),
        azure_trail: (REVEAL_AZURE_TRAIL_MAX_ALPHA * reveal_ring_envelope(tc, 0.10, 0.55))
            .clamp(0.0, REVEAL_AZURE_TRAIL_MAX_ALPHA),
        ripple1: (REVEAL_RIPPLE1_MAX_ALPHA * reveal_ring_envelope(tc, 0.14, 0.52))
            .clamp(0.0, REVEAL_RIPPLE1_MAX_ALPHA),
        ripple2: (REVEAL_RIPPLE2_MAX_ALPHA * reveal_ring_envelope(tc, 0.20, 0.48))
            .clamp(0.0, REVEAL_RIPPLE2_MAX_ALPHA),
    }
}

/// Deterministic sparkle along the expanding circular reveal wavefront (no RNG).
///
/// Spawned when the reveal radius reaches the sparkle's base radius, lives `SPARKLE_LIFETIME_MS`
/// (420 ms), twinkles from scale `0.3 -> 1.0 -> 0.6`, drifts `12 px` radially outward,
/// and rotates within `0.0..=45.0` degrees.
pub fn sparkle(i: usize, t: f64, width: f64, height: f64) -> Option<Sparkle> {
    if i >= SPARKLE_COUNT
        || !width.is_finite()
        || !height.is_finite()
        || width <= 0.0
        || height <= 0.0
        || !t.is_finite()
        || t <= 0.0
    {
        return None;
    }

    let frac_i = (i as f64 + 0.5) / SPARKLE_COUNT as f64;
    let r_frac = 0.15 + frac_i * (0.95 - 0.15);
    let t_birth = 1.0 - (1.0 - r_frac).cbrt();
    let birth_ms = t_birth * REVEAL_DURATION_MS;
    let elapsed_ms = t * REVEAL_DURATION_MS;
    if elapsed_ms < birth_ms {
        return None;
    }

    let age_ms = elapsed_ms - birth_ms;
    if age_ms >= SPARKLE_LIFETIME_MS {
        return None;
    }

    let life = (age_ms / SPARKLE_LIFETIME_MS).clamp(0.0, 1.0);
    let max_radius = (0.5 * width).hypot(0.5 * height) + REVEAL_FEATHER_PX;
    let base_r = r_frac * max_radius;
    let r = base_r + SPARKLE_DRIFT_PX * tokens::motion::ease_out_cubic(life);
    let theta = ((i as f64) * SPARKLE_GOLDEN_ANGLE_DEG).to_radians();

    let x = 0.5 * width + r * theta.cos();
    let y = 0.5 * height + r * theta.sin();

    let size_bucket = ((i * 7) % 5) as f64;
    let base_size = 5.0 + size_bucket * 1.5;
    let scale = if life <= 0.5 {
        0.3 + 0.7 * tokens::motion::ease_out_cubic(life / 0.5)
    } else {
        1.0 - 0.4 * tokens::motion::ease_out_cubic((life - 0.5) / 0.5)
    };
    let size = (base_size * scale).clamp(0.0, 11.0);

    let fade_in = if life < 0.15 {
        tokens::motion::ease_out_cubic(life / 0.15)
    } else {
        1.0
    };
    let fade_out = if life > 0.5 {
        (1.0 - tokens::motion::ease_out_cubic((life - 0.5) / 0.5)).max(0.0)
    } else {
        1.0
    };
    let alpha = (SPARKLE_MAX_ALPHA * fade_in * fade_out).clamp(0.0, SPARKLE_MAX_ALPHA);

    let rot_offset = ((i * 11) % 21) as f64;
    let rotation = (rot_offset + 24.0 * life).clamp(0.0, 45.0);

    Some(Sparkle {
        x,
        y,
        size,
        alpha,
        rotation,
    })
}

/// Center golden radial glow fading out during the final `FINAL_GLOW_DURATION_MS` (250 ms).
pub fn final_glow_alpha(t: f64) -> f64 {
    if !t.is_finite() {
        return 0.0;
    }
    let start_t = (REVEAL_DURATION_MS - FINAL_GLOW_DURATION_MS) / REVEAL_DURATION_MS;
    if t < start_t || t >= 1.0 {
        return 0.0;
    }
    let u = ((t - start_t) / (1.0 - start_t)).clamp(0.0, 1.0);
    (FINAL_GLOW_MAX_ALPHA * (1.0 - u)).clamp(0.0, FINAL_GLOW_MAX_ALPHA)
}

/// Per-item stagger delay (ms) for Stage settle removal, stepping by 25 ms and capped at 500 ms.
pub fn calculate_settle_delay_ms(index: usize, count: usize) -> u64 {
    if count <= 1 || index == 0 {
        return 0;
    }
    let natural_last = (count - 1) as u64 * SETTLE_STEP_MS;
    if natural_last <= SETTLE_MAX_TOTAL_MS {
        (index as u64) * SETTLE_STEP_MS
    } else {
        let step = SETTLE_MAX_TOTAL_MS as f64 / (count - 1) as f64;
        ((index as f64) * step)
            .round()
            .clamp(0.0, SETTLE_MAX_TOTAL_MS as f64) as u64
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

fn append_radial_band_node(
    snapshot: &gtk4::Snapshot,
    bounds: &graphene::Rect,
    center: &graphene::Point,
    radii: (f64, f64, f64),
    alpha: f64,
    rgb: (f64, f64, f64),
) {
    let (inner_r, peak_r, outer_r) = radii;
    if outer_r <= 1.0 || alpha <= 0.001 || outer_r <= peak_r {
        return;
    }
    let clamped_in = inner_r.max(0.0).min(peak_r * 0.99);
    let peak_off = (peak_r / outer_r).clamp(0.001, 0.999) as f32;
    let in_off = ((clamped_in / outer_r) as f32).clamp(0.0, peak_off - 0.0005);

    let stops = [
        gsk::ColorStop::new(in_off, rgba_with_alpha(rgb, 0.0)),
        gsk::ColorStop::new(peak_off, rgba_with_alpha(rgb, alpha)),
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

fn push_rotated_rect_to_path(
    pb: &gsk::PathBuilder,
    cx: f32,
    cy: f32,
    hx: f32,
    hy: f32,
    cos_a: f32,
    sin_a: f32,
) {
    let corners = [(-hx, -hy), (hx, -hy), (hx, hy), (-hx, hy)];
    for (idx, (lx, ly)) in corners.iter().enumerate() {
        let px = cx + lx * cos_a - ly * sin_a;
        let py = cy + lx * sin_a + ly * cos_a;
        if idx == 0 {
            pb.move_to(px, py);
        } else {
            pb.line_to(px, py);
        }
    }
    pb.close();
}

pub fn append_sparkle_nodes(
    snapshot: &gtk4::Snapshot,
    sp: Sparkle,
    end_fade: f64,
    gold_light: (f64, f64, f64),
) {
    let alpha = (sp.alpha * end_fade).clamp(0.0, SPARKLE_MAX_ALPHA);
    if alpha <= 0.002 || sp.size <= 0.2 {
        return;
    }

    let cx = sp.x as f32;
    let cy = sp.y as f32;
    let glow_r = (sp.size * 0.9).max(2.5) as f32;
    let glow_bounds = graphene::Rect::new(cx - glow_r, cy - glow_r, 2.0 * glow_r, 2.0 * glow_r);
    let glow_center = graphene::Point::new(cx, cy);
    let glow_stops = [
        gsk::ColorStop::new(0.0, rgba_with_alpha(gold_light, alpha * 0.55)),
        gsk::ColorStop::new(1.0, rgba_with_alpha(gold_light, 0.0)),
    ];
    snapshot.append_radial_gradient(
        &glow_bounds,
        &glow_center,
        glow_r,
        glow_r,
        0.0,
        1.0,
        &glow_stops,
    );

    let hl = (sp.size * 0.5).max(1.0) as f32;
    let ht = (sp.size * 0.11).clamp(0.6, 1.1) as f32;
    let rad = (sp.rotation as f32).to_radians();
    let cos_a = rad.cos();
    let sin_a = rad.sin();

    let pb = gsk::PathBuilder::new();
    push_rotated_rect_to_path(&pb, cx, cy, hl, ht, cos_a, sin_a);
    push_rotated_rect_to_path(&pb, cx, cy, ht, hl, cos_a, sin_a);
    let cross_path = pb.to_path();
    snapshot.append_fill(
        &cross_path,
        gsk::FillRule::Winding,
        &rgba_with_alpha(gold_light, alpha),
    );
}

/// Record the GSK circular reveal mask and golden wavefront nodes into `snapshot`.
///
/// Uses `MaskMode::InvertedAlpha` (`source * (1 - mask.alpha)`), cutting an expanding
/// circular hole through `texture`, then draws the multi-band rim, trailing ripples,
/// sparkles, and end-of-reveal center glow above the mask overlay clipped to the window.
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

    snapshot.push_clip(&bounds);

    let alphas = golden_ring_alphas(progress);
    if radius > 0.0 {
        // Outer gold glow band (~24 px outside edge)
        append_radial_band_node(
            snapshot,
            &bounds,
            &center,
            ((radius - 2.0).max(0.0), radius + 4.0, radius + 24.0),
            alphas.glow,
            pal.gold.rgb(),
        );
        // Behind-edge azure trail (~18 px behind edge)
        if radius > 2.0 {
            let trail_in = (radius - 18.0).max(0.0);
            let trail_peak = (radius - 5.0).max(trail_in + 0.5);
            append_radial_band_node(
                snapshot,
                &bounds,
                &center,
                (trail_in, trail_peak, radius + 1.0),
                alphas.azure_trail,
                pal.azure.rgb(),
            );
        }
        // Leading-edge gold_light core (~3 px)
        append_radial_band_node(
            snapshot,
            &bounds,
            &center,
            ((radius - 1.5).max(0.0), radius, radius + 1.5),
            alphas.core,
            pal.gold_light.rgb(),
        );
        // Trailing ripples: gold then azure
        append_trail_ring_node(
            snapshot,
            &bounds,
            &center,
            radius * 0.62,
            3.0,
            alphas.ripple1,
            pal.gold.rgb(),
        );
        append_trail_ring_node(
            snapshot,
            &bounds,
            &center,
            radius * 0.36,
            2.5,
            alphas.ripple2,
            pal.azure.rgb(),
        );
    }

    let end_fade = if progress > 0.88 {
        ((1.0 - progress) / 0.12).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let gold_light = pal.gold_light.rgb();
    for i in 0..SPARKLE_COUNT {
        if let Some(sp) = sparkle(i, progress, width, height) {
            append_sparkle_nodes(snapshot, sp, end_fade, gold_light);
        }
    }

    let glow_a = final_glow_alpha(progress);
    if glow_a > 0.001 {
        let center_r = (0.45 * width).hypot(0.45 * height).max(40.0) as f32;
        let gold = pal.gold.rgb();
        let center_stops = [
            gsk::ColorStop::new(0.0, rgba_with_alpha(gold, glow_a)),
            gsk::ColorStop::new(0.55, rgba_with_alpha(gold, glow_a * 0.35)),
            gsk::ColorStop::new(1.0, rgba_with_alpha(gold, 0.0)),
        ];
        snapshot.append_radial_gradient(
            &bounds,
            &center,
            center_r,
            center_r,
            0.0,
            1.0,
            &center_stops,
        );
    }

    snapshot.pop();
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

/// Compute one-shot golden header flash sweep state for `elapsed_secs` within 600 ms.
pub fn compute_header_flash_frame(elapsed_secs: f64) -> Option<ShimmerFrame> {
    let duration_secs = HEADER_FLASH_DURATION_MS / 1000.0;
    if !elapsed_secs.is_finite() || elapsed_secs < 0.0 || elapsed_secs > duration_secs {
        return None;
    }

    let u = (elapsed_secs / duration_secs).clamp(0.0, 1.0);
    let eased = tokens::motion::ease_out_cubic(u);
    let center_frac = -0.15 + 1.30 * eased;
    let half_width_frac = 0.18;
    let envelope = (std::f64::consts::PI * u).sin().max(0.0);
    let peak_alpha = (HEADER_FLASH_MAX_ALPHA * envelope).clamp(0.0, HEADER_FLASH_MAX_ALPHA);

    if peak_alpha <= 0.001 {
        return None;
    }

    Some(ShimmerFrame {
        center_frac,
        half_width_frac,
        peak_alpha,
    })
}

fn draw_gold_sweep_strip(
    cr: &cairo::Context,
    width: f64,
    height: f64,
    frame: ShimmerFrame,
    alpha_cap: f64,
    rgb: (f64, f64, f64),
) {
    let (r, g, b) = rgb;
    let cx = frame.center_frac * width;
    let hw = (frame.half_width_frac * width).max(16.0);
    let alpha = frame.peak_alpha.clamp(0.0, alpha_cap);

    let grad = cairo::LinearGradient::new(cx - hw, 0.0, cx + hw, 0.0);
    grad.add_color_stop_rgba(0.0, r, g, b, 0.0);
    grad.add_color_stop_rgba(0.5, r, g, b, alpha);
    grad.add_color_stop_rgba(1.0, r, g, b, 0.0);

    let _ = cr.set_source(&grad);
    cr.rectangle(0.0, (height - 1.0).max(0.0), width, 1.0);
    let _ = cr.fill();
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

    draw_gold_sweep_strip(cr, width, height, frame, SHIMMER_MAX_ALPHA, pal.gold.rgb());
}

/// Render the 1px one-shot golden flash along the bottom edge of the header strip.
pub fn draw_header_flash(
    cr: &cairo::Context,
    width: f64,
    height: f64,
    elapsed_secs: f64,
    pal: &tokens::Palette,
) {
    if width <= 0.0 || height <= 0.0 {
        return;
    }

    let Some(frame) = compute_header_flash_frame(elapsed_secs) else {
        return;
    };

    draw_gold_sweep_strip(
        cr,
        width,
        height,
        frame,
        HEADER_FLASH_MAX_ALPHA,
        pal.gold_light.rgb(),
    );
}

fn collect_settle_targets(root: Option<&gtk4::Widget>, out: &mut Vec<gtk4::Widget>) {
    let Some(widget) = root else {
        return;
    };
    if !widget.is_visible() || !widget.is_mapped() {
        return;
    }

    let is_row = widget.is::<gtk4::ListBoxRow>();
    let is_card_container = !widget.is::<gtk4::Label>()
        && (widget.has_css_class("card")
            || widget.has_css_class("boxed-list")
            || widget.is::<gtk4::Frame>());

    if is_row || is_card_container {
        out.push(widget.clone());
    }

    let mut child = widget.first_child();
    while let Some(c) = child {
        collect_settle_targets(Some(&c), out);
        child = c.next_sibling();
    }
}

struct SkinOverlayInner {
    skin_overlay: gtk4::Overlay,
    window: adw::ApplicationWindow,
    shimmer_area: gtk4::DrawingArea,
    unlocked: Cell<bool>,
    closed: Cell<bool>,
    effects_active: Cell<bool>,
    shimmer_clock: RefCell<AnimationClock>,
    shimmer_running: Cell<bool>,
    shimmer_tick_running: Cell<bool>,
    shimmer_gen: Cell<u64>,
    sleep_gen: Cell<u64>,
    shimmer_was_drawing: Cell<bool>,
    flash_start: Cell<Option<Instant>>,
    shine_target: RefCell<Option<glib::WeakRef<gtk4::Button>>>,
    on_skin_swap: Rc<dyn Fn()>,
}

/// Manages the Stage circular reveal transition and header bottom-edge shimmer overlays.
#[derive(Clone)]
pub struct SkinOverlayController(Rc<SkinOverlayInner>);

/// Weak reference to a `SkinOverlayController` for cycle-free stack navigation hooks.
#[derive(Clone)]
pub struct WeakSkinOverlayController(std::rc::Weak<SkinOverlayInner>);

impl WeakSkinOverlayController {
    pub fn upgrade(&self) -> Option<SkinOverlayController> {
        self.0.upgrade().map(SkinOverlayController)
    }
}

type StackFlashHook = (glib::WeakRef<gtk4::Stack>, WeakSkinOverlayController);

thread_local! {
    static STACK_FLASH_HOOKS: RefCell<Vec<StackFlashHook>> = const { RefCell::new(Vec::new()) };
}

/// Associate `stack` with `ctrl` so every `navigate_stack` call while unlocked
/// triggers the header gold flash without altering navigation signatures.
pub fn register_stack_header_flash(stack: &gtk4::Stack, ctrl: &SkinOverlayController) {
    let weak_stack = stack.downgrade();
    let weak_ctrl = ctrl.downgrade();
    STACK_FLASH_HOOKS.with(|hooks| {
        let mut hooks = hooks.borrow_mut();
        hooks.retain(|(s, c)| s.upgrade().is_some() && c.upgrade().is_some());
        hooks.retain(|(s, _)| s.upgrade().as_ref() != Some(stack));
        hooks.push((weak_stack, weak_ctrl));
    });
}

/// Trigger the one-shot golden header flash for the controller bound to `stack`, if any.
pub fn trigger_stack_header_flash(stack: &gtk4::Stack) {
    let target: Option<SkinOverlayController> = STACK_FLASH_HOOKS.with(|hooks| {
        let mut hooks = hooks.borrow_mut();
        hooks.retain(|(s, c)| s.upgrade().is_some() && c.upgrade().is_some());
        hooks.iter().find_map(|(s, c)| {
            (s.upgrade().as_ref() == Some(stack))
                .then(|| c.upgrade())
                .flatten()
        })
    });
    if let Some(ctrl) = target {
        ctrl.trigger_header_flash();
    }
}

/// Return whether the controller registered for `stack` currently has an active header flash.
pub fn is_stack_header_flash_active(stack: &gtk4::Stack) -> bool {
    let target: Option<SkinOverlayController> = STACK_FLASH_HOOKS.with(|hooks| {
        let mut hooks = hooks.borrow_mut();
        hooks.retain(|(s, c)| s.upgrade().is_some() && c.upgrade().is_some());
        hooks.iter().find_map(|(s, c)| {
            (s.upgrade().as_ref() == Some(stack))
                .then(|| c.upgrade())
                .flatten()
        })
    });
    target.is_some_and(|ctrl| ctrl.is_header_flash_active())
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

        let inner = Rc::new(SkinOverlayInner {
            skin_overlay: skin_overlay.clone(),
            window: window.clone(),
            shimmer_area: shimmer_area.clone(),
            unlocked: Cell::new(false),
            closed: Cell::new(false),
            effects_active: Cell::new(false),
            shimmer_clock: RefCell::new(AnimationClock::new(SHIMMER_FRAME_INTERVAL)),
            shimmer_running: Cell::new(false),
            shimmer_tick_running: Cell::new(false),
            shimmer_gen: Cell::new(0),
            sleep_gen: Cell::new(0),
            shimmer_was_drawing: Cell::new(false),
            flash_start: Cell::new(None),
            shine_target: RefCell::new(None),
            on_skin_swap,
        });

        {
            let weak_draw = Rc::downgrade(&inner);
            shimmer_area.set_draw_func(move |_, cr, w, h| {
                let Some(inner) = weak_draw.upgrade() else {
                    return;
                };
                if !inner.effects_active.get() || !tokens::motion::is_animations_enabled() {
                    return;
                }
                let phase = inner.shimmer_clock.borrow().phase_secs();
                let pal = tokens::palette();
                draw_header_shimmer(cr, w as f64, h as f64, phase, &pal);
                if let Some(start) = inner.flash_start.get() {
                    let elapsed = start.elapsed().as_secs_f64();
                    if elapsed * 1000.0 <= HEADER_FLASH_DURATION_MS {
                        draw_header_flash(cr, w as f64, h as f64, elapsed, &pal);
                    } else {
                        inner.flash_start.set(None);
                    }
                }
            });
        }

        let controller = Self(inner);

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
            ctrl_close.0.closed.set(true);
            ctrl_close.0.effects_active.set(false);
            ctrl_close.stop_loops(Instant::now());
            glib::Propagation::Proceed
        });

        controller
    }

    pub fn downgrade(&self) -> WeakSkinOverlayController {
        WeakSkinOverlayController(Rc::downgrade(&self.0))
    }

    /// Register the primary button (e.g. Build & Flash) for the one-time unlock shine.
    pub fn set_shine_target(&self, button: &gtk4::Button) {
        self.0.shine_target.replace(Some(button.downgrade()));
    }

    pub fn shimmer_area(&self) -> &gtk4::DrawingArea {
        &self.0.shimmer_area
    }

    pub fn is_effects_active(&self) -> bool {
        self.0.effects_active.get()
    }

    pub fn is_shimmer_running(&self) -> bool {
        self.0.shimmer_running.get()
    }

    pub fn is_shimmer_tick_active(&self) -> bool {
        self.0.shimmer_tick_running.get()
    }

    pub fn is_header_flash_active(&self) -> bool {
        self.0
            .flash_start
            .get()
            .is_some_and(|s| s.elapsed().as_secs_f64() * 1000.0 <= HEADER_FLASH_DURATION_MS)
    }

    /// Trigger a one-shot 600 ms golden header sweep when unlocked and animations are enabled.
    pub fn trigger_header_flash(&self) {
        if self.0.closed.get()
            || !self.0.effects_active.get()
            || !tokens::motion::is_animations_enabled()
        {
            return;
        }
        self.0.flash_start.set(Some(Instant::now()));
        self.0.sleep_gen.set(self.0.sleep_gen.get().wrapping_add(1));
        self.sync_loop_state();
        self.0.shimmer_area.queue_draw();
    }

    fn apply_settle_and_shine(&self) {
        if !tokens::motion::is_animations_enabled() {
            return;
        }

        let mut targets = Vec::new();
        collect_settle_targets(self.0.skin_overlay.child().as_ref(), &mut targets);
        let count = targets.len();
        for (i, widget) in targets.into_iter().enumerate() {
            widget.add_css_class("stage-pre");
            let delay_ms = calculate_settle_delay_ms(i, count);
            let weak = widget.downgrade();
            glib::timeout_add_local_once(Duration::from_millis(delay_ms), move || {
                if let Some(w) = weak.upgrade() {
                    w.remove_css_class("stage-pre");
                }
            });
        }

        if let Some(btn) = self
            .0
            .shine_target
            .borrow()
            .as_ref()
            .and_then(|w| w.upgrade())
        {
            btn.add_css_class("stage-shine");
            let btn_weak = btn.downgrade();
            glib::timeout_add_local_once(Duration::from_millis(SHINE_DURATION_MS), move || {
                if let Some(b) = btn_weak.upgrade() {
                    b.remove_css_class("stage-shine");
                }
            });
        }
    }

    /// Trigger the skin unlock sequence once. Subsequent calls are no-ops.
    pub fn trigger_unlock(&self) {
        if self.0.unlocked.replace(true) {
            return;
        }

        if !tokens::motion::is_animations_enabled() {
            // Reduced motion: no capture, no overlay, instant swap, no shimmer/settle/shine.
            (self.0.on_skin_swap)();
            return;
        }

        let Some(old_texture) = capture_window_texture(&self.0.window, &self.0.skin_overlay) else {
            // Fallback when capture cannot run (e.g. unrealized or zero-size window):
            // swap skin immediately without an overlay, never panic.
            (self.0.on_skin_swap)();
            self.apply_settle_and_shine();
            self.0.effects_active.set(true);
            self.0.flash_start.set(Some(Instant::now()));
            self.sync_loop_state();
            return;
        };

        let reveal_overlay = RevealOverlay::new(old_texture);
        self.0.skin_overlay.add_overlay(&reveal_overlay);
        reveal_overlay.set_progress(0.0);

        // Apply the new skin immediately beneath the old-texture reveal overlay.
        (self.0.on_skin_swap)();
        self.apply_settle_and_shine();
        self.0.effects_active.set(true);
        self.0.flash_start.set(Some(Instant::now()));
        self.sync_loop_state();

        let start_time: Rc<Cell<Option<Instant>>> = Rc::new(Cell::new(None));
        let overlay_weak = self.0.skin_overlay.downgrade();
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

            if ctrl_tick.0.closed.get() || !tokens::motion::is_animations_enabled() {
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
        let should_run = !self.0.closed.get()
            && self.0.effects_active.get()
            && tokens::motion::is_animations_enabled()
            && self.0.window.is_active()
            && self.0.window.is_visible();

        let now = Instant::now();
        if should_run {
            self.start_shimmer_loop(now);
        } else {
            self.stop_loops(now);
        }
    }

    fn stop_loops(&self, now: Instant) {
        self.0.shimmer_clock.borrow_mut().pause(now);
        self.0.shimmer_running.set(false);
        self.0.shimmer_tick_running.set(false);
        self.0
            .shimmer_gen
            .set(self.0.shimmer_gen.get().wrapping_add(1));
        self.0.sleep_gen.set(self.0.sleep_gen.get().wrapping_add(1));
        if self.0.closed.get()
            || !self.0.window.is_visible()
            || !tokens::motion::is_animations_enabled()
        {
            self.0.flash_start.set(None);
        }
        if !tokens::motion::is_animations_enabled() && self.0.shimmer_was_drawing.replace(false) {
            self.0.shimmer_area.queue_draw();
        }
    }

    fn start_shimmer_loop(&self, now: Instant) {
        self.0.shimmer_clock.borrow_mut().resume(now);
        self.0.shimmer_running.set(true);
        if self.0.shimmer_tick_running.replace(true) {
            return;
        }

        let my_gen = self.0.shimmer_gen.get().wrapping_add(1);
        self.0.shimmer_gen.set(my_gen);
        self.0.sleep_gen.set(self.0.sleep_gen.get().wrapping_add(1));

        let ctrl_weak = self.downgrade();

        self.0.shimmer_area.add_tick_callback(move |area, _| {
            let now = Instant::now();
            let Some(ctrl) = ctrl_weak.upgrade() else {
                return glib::ControlFlow::Break;
            };
            let inner = &ctrl.0;
            let alive = my_gen == inner.shimmer_gen.get()
                && !inner.closed.get()
                && inner.effects_active.get()
                && tokens::motion::is_animations_enabled()
                && inner.window.is_active()
                && inner.window.is_visible();

            if !alive {
                inner.shimmer_clock.borrow_mut().pause(now);
                if inner.closed.get()
                    || !tokens::motion::is_animations_enabled()
                    || !inner.window.is_visible()
                {
                    inner.flash_start.set(None);
                }
                if my_gen == inner.shimmer_gen.get() {
                    inner.shimmer_running.set(false);
                    inner.shimmer_tick_running.set(false);
                }
                if !tokens::motion::is_animations_enabled()
                    && inner.shimmer_was_drawing.replace(false)
                {
                    area.queue_draw();
                }
                return glib::ControlFlow::Break;
            }

            let should_step = inner.shimmer_clock.borrow_mut().tick(now);
            let mut in_idle_gap = false;
            if should_step {
                let phase = inner.shimmer_clock.borrow().phase_secs();
                let periodic_active = compute_shimmer_frame(phase).is_some();
                let flash_active = match inner.flash_start.get() {
                    Some(start) => {
                        if now.saturating_duration_since(start).as_secs_f64() * 1000.0
                            <= HEADER_FLASH_DURATION_MS
                        {
                            true
                        } else {
                            inner.flash_start.set(None);
                            true
                        }
                    }
                    None => false,
                };
                if periodic_active || flash_active {
                    inner.shimmer_was_drawing.set(true);
                    area.queue_draw();
                } else {
                    if inner.shimmer_was_drawing.replace(false) {
                        area.queue_draw();
                    }
                    let cycle_pos = phase.max(0.0).rem_euclid(SHIMMER_PERIOD_SECS);
                    let rem_secs = SHIMMER_PERIOD_SECS - cycle_pos;
                    if rem_secs > 0.05 {
                        in_idle_gap = true;
                        inner.shimmer_tick_running.set(false);
                        let sleep_gen = inner.sleep_gen.get().wrapping_add(1);
                        inner.sleep_gen.set(sleep_gen);
                        let sleep_ms = ((rem_secs * 1000.0) - 10.0).max(10.0) as u64;
                        let wake_weak = ctrl.downgrade();
                        glib::timeout_add_local_once(Duration::from_millis(sleep_ms), move || {
                            if let Some(c) = wake_weak.upgrade() {
                                if c.0.shimmer_gen.get() == my_gen
                                    && c.0.sleep_gen.get() == sleep_gen
                                    && c.0.shimmer_running.get()
                                    && !c.0.closed.get()
                                    && c.0.effects_active.get()
                                    && tokens::motion::is_animations_enabled()
                                    && c.0.window.is_active()
                                    && c.0.window.is_visible()
                                {
                                    c.start_shimmer_loop(Instant::now());
                                }
                            }
                        });
                    }
                }
            }
            if in_idle_gap {
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
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
        draw_header_flash(&cr, 320.0, 2.0, 0.3, &stage);
        draw_header_flash(&cr, 0.0, 0.0, 0.3, &stage);
        surface.flush();
    }

    #[test]
    fn test_golden_ring_alphas_bounds() {
        let a0 = golden_ring_alphas(0.0);
        assert_eq!(a0.core, 0.0);
        assert_eq!(a0.glow, 0.0);
        assert_eq!(a0.azure_trail, 0.0);
        assert_eq!(a0.ripple1, 0.0);
        assert_eq!(a0.ripple2, 0.0);

        let a1 = golden_ring_alphas(1.0);
        assert_eq!(a1.core, 0.0);
        assert_eq!(a1.glow, 0.0);
        assert_eq!(a1.azure_trail, 0.0);
        assert_eq!(a1.ripple1, 0.0);
        assert_eq!(a1.ripple2, 0.0);

        let mut max_core = 0.0_f64;
        let mut max_glow = 0.0_f64;
        let mut max_trail = 0.0_f64;
        let mut max_r1 = 0.0_f64;
        let mut max_r2 = 0.0_f64;
        for step in 0..=200 {
            let t = step as f64 / 200.0;
            let a = golden_ring_alphas(t);
            assert!(a.core.is_finite() && (0.0..=0.85).contains(&a.core));
            assert!(a.glow.is_finite() && (0.0..=0.35).contains(&a.glow));
            assert!(a.azure_trail.is_finite() && (0.0..=0.30).contains(&a.azure_trail));
            assert!(a.ripple1.is_finite() && (0.0..=0.20).contains(&a.ripple1));
            assert!(a.ripple2.is_finite() && (0.0..=0.12).contains(&a.ripple2));
            max_core = max_core.max(a.core);
            max_glow = max_glow.max(a.glow);
            max_trail = max_trail.max(a.azure_trail);
            max_r1 = max_r1.max(a.ripple1);
            max_r2 = max_r2.max(a.ripple2);
        }
        assert!(max_core > 0.7 && max_core <= 0.85);
        assert!(max_glow > 0.3 && max_glow <= 0.35);
        assert!(max_trail > 0.25 && max_trail <= 0.30);
        assert!(max_r1 > 0.15 && max_r1 <= 0.20);
        assert!(max_r2 > 0.09 && max_r2 <= 0.12);
    }

    #[test]
    fn test_sparkle_deterministic_bounds_and_lifecycle() {
        let w = 1280.0;
        let h = 820.0;

        for i in 0..SPARKLE_COUNT {
            assert!(
                sparkle(i, 0.0, w, h).is_none(),
                "Sparkle {i} must be None at t = 0"
            );
            assert!(
                sparkle(i, 1.6, w, h).is_none(),
                "Sparkle {i} must be None after full lifetime"
            );
            assert!(
                sparkle(i, 2.5, w, h).is_none(),
                "Sparkle {i} must be None well after full lifetime"
            );
        }
        assert!(sparkle(SPARKLE_COUNT, 0.4, w, h).is_none());

        let mut saw_active = [false; SPARKLE_COUNT];
        for step in 1..=160 {
            let t = step as f64 / 100.0;
            for (i, seen) in saw_active.iter_mut().enumerate() {
                let s1 = sparkle(i, t, w, h);
                let s2 = sparkle(i, t, w, h);
                assert_eq!(s1, s2, "sparkle must be strictly deterministic");
                if let Some(s) = s1 {
                    *seen = true;
                    assert!(s.x.is_finite() && s.y.is_finite());
                    assert!(
                        s.alpha.is_finite() && (0.0..=0.9).contains(&s.alpha),
                        "Sparkle {i} alpha {} exceeds 0.9",
                        s.alpha
                    );
                    assert!(
                        s.size.is_finite() && (1.0..=11.0).contains(&s.size),
                        "Sparkle {i} size {} out of bounds",
                        s.size
                    );
                    assert!(
                        s.rotation.is_finite() && (0.0..=45.0).contains(&s.rotation),
                        "Sparkle {i} rotation {} out of 0..=45",
                        s.rotation
                    );
                }
            }
        }
        assert!(
            saw_active.iter().all(|&s| s),
            "All 36 sparkles must activate during reveal lifetime"
        );

        // Zero-size, negative, and non-finite windows never yield non-finite values
        for (tw, th) in [
            (0.0, 0.0),
            (0.0, 400.0),
            (600.0, 0.0),
            (-20.0, 300.0),
            (f64::NAN, 300.0),
            (f64::INFINITY, 300.0),
        ] {
            for i in 0..SPARKLE_COUNT {
                if let Some(s) = sparkle(i, 0.5, tw, th) {
                    assert!(
                        s.x.is_finite()
                            && s.y.is_finite()
                            && s.size.is_finite()
                            && s.alpha.is_finite()
                            && s.rotation.is_finite()
                    );
                }
            }
        }
    }

    #[test]
    fn test_final_glow_header_flash_and_settle_delays() {
        assert_eq!(final_glow_alpha(0.0), 0.0);
        assert_eq!(final_glow_alpha(0.65), 0.0);
        let at_start = final_glow_alpha(500.0 / 750.0);
        assert!((at_start - 0.10).abs() < 1e-6);
        let at_mid = final_glow_alpha(625.0 / 750.0);
        assert!(at_mid > 0.04 && at_mid < 0.06);
        assert_eq!(final_glow_alpha(1.0), 0.0);

        // Header flash bounds: 600 ms, alpha <= 0.6
        assert!(compute_header_flash_frame(-0.1).is_none());
        let flash_mid =
            compute_header_flash_frame(0.3).expect("Header flash must be active at 300 ms");
        assert!(flash_mid.peak_alpha > 0.5 && flash_mid.peak_alpha <= 0.6);
        assert!(compute_header_flash_frame(0.65).is_none());

        // Settle delays step at 25 ms and cap total at 500 ms
        assert_eq!(calculate_settle_delay_ms(0, 5), 0);
        assert_eq!(calculate_settle_delay_ms(4, 5), 100);
        assert_eq!(calculate_settle_delay_ms(19, 20), 475);
        assert_eq!(calculate_settle_delay_ms(49, 50), 500);
        for i in 0..50 {
            assert!(calculate_settle_delay_ms(i, 50) <= SETTLE_MAX_TOTAL_MS);
        }
    }

    #[test]
    fn test_reveal_node_budget_pure_under_150() {
        let w = 1280.0;
        let h = 820.0;
        let mut max_nodes = 0usize;
        let mut max_active_sparkles = 0usize;

        for step in 0..=100 {
            let t = step as f64 / 100.0;
            let radius = reveal_radius(t, w, h);
            let alphas = golden_ring_alphas(t);

            // Base GSK structure: container + mask + mask radial-gradient + texture + clip
            let mut nodes = 5usize;

            if radius > 0.0 {
                if alphas.glow > 0.001 && radius + 24.0 > 1.0 {
                    nodes += 1;
                }
                if radius > 2.0 && alphas.azure_trail > 0.001 {
                    nodes += 1;
                }
                if alphas.core > 0.001 && radius + 1.5 > 1.0 {
                    nodes += 1;
                }
                if radius * 0.62 > 1.0 && alphas.ripple1 > 0.001 {
                    nodes += 1;
                }
                if radius * 0.36 > 1.0 && alphas.ripple2 > 0.001 {
                    nodes += 1;
                }
            }

            let end_fade = if t > 0.88 {
                ((1.0 - t) / 0.12).clamp(0.0, 1.0)
            } else {
                1.0
            };
            let mut active_sparkles = 0usize;
            for i in 0..SPARKLE_COUNT {
                if let Some(sp) = sparkle(i, t, w, h) {
                    if (sp.alpha * end_fade).clamp(0.0, SPARKLE_MAX_ALPHA) > 0.002 && sp.size > 0.2
                    {
                        active_sparkles += 1;
                        // 1 RadialGradientNode + 1 FillNode (with 1 child ColorNode) = 3 recursive
                        nodes += 3;
                    }
                }
            }
            if final_glow_alpha(t) > 0.001 {
                nodes += 1;
            }

            max_active_sparkles = max_active_sparkles.max(active_sparkles);
            max_nodes = max_nodes.max(nodes);
            assert!(
                nodes < 150,
                "Expected reveal GSK node budget < 150 at t={t}, got {nodes}"
            );
        }

        assert!(
            max_active_sparkles > 10 && max_active_sparkles <= SPARKLE_COUNT,
            "Expected meaningful sparkle activation, peak={max_active_sparkles}"
        );
        assert!(
            max_nodes < 150,
            "Peak reveal node budget must remain < 150, got {max_nodes}"
        );

        let (r0, stops0) = build_reveal_mask_stops(0.0);
        assert!(r0.is_finite() && stops0.len() == 2);
        let (r_small, stops_small) = build_reveal_mask_stops(REVEAL_FEATHER_PX * 0.5);
        assert!(r_small.is_finite() && stops_small.len() == 2);
        let (r_big, stops_big) = build_reveal_mask_stops(120.0);
        assert!(r_big.is_finite() && stops_big.len() == 3);
    }
}
