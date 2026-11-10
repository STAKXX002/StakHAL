use crate::state::AppState;
use crate::ui::tokens;
use gtk4::cairo;
use stakhal_core::graph::{compute_state_machine_layout, is_fault_state};
use std::cell::RefCell;
use std::rc::Rc;

fn mix_rgb(a: (f64, f64, f64), b: (f64, f64, f64), t: f64) -> (f64, f64, f64) {
    (
        (a.0 + (b.0 - a.0) * t).clamp(0.0, 1.0),
        (a.1 + (b.1 - a.1) * t).clamp(0.0, 1.0),
        (a.2 + (b.2 - a.2) * t).clamp(0.0, 1.0),
    )
}

const STAGE_EDGE_WIDTH: f64 = 1.5;
const STAGE_NODE_BORDER_WIDTH: f64 = 1.5;
const STAGE_SELECTED_BORDER_WIDTH: f64 = 2.0;
const STAGE_SELECTED_GLOW_WIDTH: f64 = 6.0;
const STAGE_SELECTED_GLOW_ALPHA: f64 = 0.16;
const STAGE_RIBBON_WIDTH: f64 = 3.0;

fn trace_rounded_rect(cr: &cairo::Context, x: f64, y: f64, w: f64, h: f64, radius: f64) {
    let r = radius.clamp(0.0, (w * 0.5).min(h * 0.5).max(0.0));
    cr.new_path();
    cr.arc(x + w - r, y + r, r, -std::f64::consts::FRAC_PI_2, 0.0);
    cr.arc(x + w - r, y + h - r, r, 0.0, std::f64::consts::FRAC_PI_2);
    cr.arc(
        x + r,
        y + h - r,
        r,
        std::f64::consts::FRAC_PI_2,
        std::f64::consts::PI,
    );
    cr.arc(
        x + r,
        y + r,
        r,
        std::f64::consts::PI,
        3.0 * std::f64::consts::FRAC_PI_2,
    );
    cr.close_path();
}

struct DiagramColors {
    is_default: bool,
    canvas_bg: (f64, f64, f64),
    dot_grid: (f64, f64, f64, f64),
    fault_red: (f64, f64, f64),
    fault_fill: (f64, f64, f64, f64),
    node_fill_default: (f64, f64, f64, f64),
    node_fill_initial: (f64, f64, f64, f64),
    node_fill_hover: (f64, f64, f64, f64),
    node_fill_selected: (f64, f64, f64, f64),
    node_fill_dimmed: (f64, f64, f64, f64),
    edge_default: (f64, f64, f64),
    border_default: (f64, f64, f64),
    border_initial: (f64, f64, f64),
    border_hover: (f64, f64, f64),
    border_selected: (f64, f64, f64),
    pill_fill: (f64, f64, f64, f64),
    pill_border: (f64, f64, f64),
    bg_panel: (f64, f64, f64),
    bg_raised: (f64, f64, f64),
    text_primary: (f64, f64, f64),
    text_muted: (f64, f64, f64),
    accent: (f64, f64, f64),
    azure: (f64, f64, f64),
    gold: (f64, f64, f64),
    edge_width: f64,
    node_border_width: f64,
    lane_alpha_selected: f64,
    lane_alpha_normal: f64,
    edge_alpha_default: f64,
    edge_alpha_dimmed: f64,
    fault_edge_alpha_normal: f64,
    fault_edge_alpha_dimmed: f64,
    dimmed_border_alpha: f64,
    dimmed_text_alpha: f64,
    dimmed_tag_alpha: f64,
    pill_fill_dimmed_alpha: f64,
    pill_border_alpha: f64,
}

impl DiagramColors {
    fn from_palette(pal: tokens::Palette) -> Self {
        let is_default = pal == tokens::Palette::DEFAULT;

        let dot_grid = if is_default {
            (
                pal.border_hair.0,
                pal.border_hair.1,
                pal.border_hair.2,
                0.35,
            )
        } else {
            (pal.line.0, pal.line.1, pal.line.2, 0.35)
        };

        let fault_fill = if is_default {
            // Preserve historical default FAULT node fill for pixel-identical snapshots
            (45.0 / 255.0, 16.0 / 255.0, 18.0 / 255.0, 0.95)
        } else {
            let m = mix_rgb(pal.bg_panel.rgb(), pal.state_error.rgb(), 0.08);
            (m.0, m.1, m.2, 1.0)
        };

        let node_fill_default = if is_default {
            (pal.bg_panel.0, pal.bg_panel.1, pal.bg_panel.2, 0.95)
        } else {
            (pal.bg_panel.0, pal.bg_panel.1, pal.bg_panel.2, 1.0)
        };

        let node_fill_initial = if is_default {
            // Preserve historical default INITIAL node fill for pixel-identical snapshots
            (24.0 / 255.0, 30.0 / 255.0, 36.0 / 255.0, 0.95)
        } else {
            (pal.bg_raised.0, pal.bg_raised.1, pal.bg_raised.2, 1.0)
        };

        let node_fill_hover = if is_default {
            // Preserve historical default HOVER node fill for pixel-identical snapshots
            (28.0 / 255.0, 36.0 / 255.0, 44.0 / 255.0, 0.95)
        } else {
            (pal.bg_raised.0, pal.bg_raised.1, pal.bg_raised.2, 1.0)
        };

        let node_fill_selected = if is_default {
            // Preserve historical default SELECTED node fill for pixel-identical snapshots
            (20.0 / 255.0, 32.0 / 255.0, 38.0 / 255.0, 0.95)
        } else {
            (pal.bg_raised.0, pal.bg_raised.1, pal.bg_raised.2, 1.0)
        };

        let node_fill_dimmed = if is_default {
            (pal.bg_void.0, pal.bg_void.1, pal.bg_void.2, 0.5)
        } else {
            (pal.bg_void.0, pal.bg_void.1, pal.bg_void.2, 1.0)
        };

        let border_initial = if is_default {
            // Preserve historical default INITIAL border for pixel-identical snapshots
            (50.0 / 255.0, 60.0 / 255.0, 70.0 / 255.0)
        } else {
            pal.node_border.rgb()
        };

        let border_hover = if is_default {
            // Preserve historical default HOVER border for pixel-identical snapshots
            (60.0 / 255.0, 75.0 / 255.0, 85.0 / 255.0)
        } else {
            pal.accent.rgb()
        };

        let pill_fill = if is_default {
            (pal.bg_panel.0, pal.bg_panel.1, pal.bg_panel.2, 0.95)
        } else {
            (pal.bg_panel.0, pal.bg_panel.1, pal.bg_panel.2, 1.0)
        };

        let pill_border = if is_default {
            pal.border_hair.rgb()
        } else {
            pal.line.rgb()
        };

        Self {
            is_default,
            canvas_bg: pal.bg_void.rgb(),
            dot_grid,
            fault_red: pal.state_error.rgb(),
            fault_fill,
            node_fill_default,
            node_fill_initial,
            node_fill_hover,
            node_fill_selected,
            node_fill_dimmed,
            edge_default: pal.line.rgb(),
            border_default: pal.node_border.rgb(),
            border_initial,
            border_hover,
            border_selected: pal.accent.rgb(),
            pill_fill,
            pill_border,
            bg_panel: pal.bg_panel.rgb(),
            bg_raised: pal.bg_raised.rgb(),
            text_primary: pal.text_primary.rgb(),
            text_muted: pal.text_muted.rgb(),
            accent: pal.accent.rgb(),
            azure: pal.azure.rgb(),
            gold: pal.gold.rgb(),
            edge_width: if is_default {
                tokens::shape::BORDER_WIDTH_HAIR
            } else {
                STAGE_EDGE_WIDTH
            },
            node_border_width: if is_default {
                tokens::shape::BORDER_WIDTH_HAIR
            } else {
                STAGE_NODE_BORDER_WIDTH
            },
            lane_alpha_selected: if is_default { 0.35 } else { 1.0 },
            lane_alpha_normal: if is_default { 0.75 } else { 1.0 },
            edge_alpha_default: if is_default { 0.85 } else { 0.95 },
            edge_alpha_dimmed: if is_default { 0.25 } else { 0.55 },
            fault_edge_alpha_normal: if is_default { 0.85 } else { 0.95 },
            fault_edge_alpha_dimmed: if is_default { 0.15 } else { 0.50 },
            dimmed_border_alpha: if is_default { 0.35 } else { 0.60 },
            dimmed_text_alpha: if is_default { 0.4 } else { 1.0 },
            dimmed_tag_alpha: if is_default { 0.3 } else { 1.0 },
            pill_fill_dimmed_alpha: if is_default { 0.4 } else { 0.95 },
            pill_border_alpha: if is_default { 0.85 } else { 1.0 },
        }
    }
}

pub fn draw_state_diagram_canvas(
    _area: &gtk4::DrawingArea,
    cr: &cairo::Context,
    width: f64,
    height: f64,
    state: &Rc<RefCell<AppState>>,
) {
    draw_state_diagram(cr, width, height, state);
}

pub fn draw_state_diagram(
    cr: &cairo::Context,
    width: f64,
    height: f64,
    state: &Rc<RefCell<AppState>>,
) {
    let colors = DiagramColors::from_palette(tokens::palette());
    // 1. Ensure state diagram layout is computed and view is fitted if needed
    {
        let loaded_project = state.borrow().project.borrow().loaded_project.clone();
        state.borrow().with_canvas_state_mut(|st| {
            if st.state_diagram_layout.is_none() {
                if let Some(ref p) = loaded_project {
                    if !p.state_machines.is_empty() {
                        let idx = st.selected_state_machine.min(p.state_machines.len() - 1);
                        let sm = p.state_machines[idx].clone();
                        let layout = compute_state_machine_layout(&sm);
                        st.selected_state_machine = idx;
                        st.diagram_bounds = (layout.width as i32, layout.height as i32);
                        let mut pos = std::collections::HashMap::new();
                        for (id, n) in &layout.nodes {
                            pos.insert(id.clone(), (n.x, n.y));
                        }
                        st.state_node_positions = pos;
                        st.state_diagram_layout = Some(layout);
                        st.diagram_needs_fit = true;
                    }
                }
            }

            if st.diagram_needs_fit && width > 100.0 && height > 100.0 {
                if let Some((lw, lh)) = st
                    .state_diagram_layout
                    .as_ref()
                    .map(|l| (l.width, l.height))
                {
                    let avail_w = (width - 40.0).max(100.0);
                    let avail_h = (height - 40.0).max(100.0);
                    let fit_zoom = (avail_w / lw).min(avail_h / lh).clamp(0.2, 1.0);
                    st.diagram_zoom = fit_zoom;
                    st.diagram_pan_x = ((width - lw * fit_zoom) * 0.5).max(20.0);
                    st.diagram_pan_y = ((height - lh * fit_zoom) * 0.5).max(20.0);
                    st.diagram_needs_fit = false;
                }
            }
        });
    }

    let (
        layout_opt,
        zoom,
        pan_x,
        pan_y,
        selected_node,
        hovered_node,
        node_flash,
        edge_reveal,
        edge_pulse,
        selected_sm_idx,
    ) = state.borrow().with_canvas_state(|st| {
        (
            st.state_diagram_layout.clone(),
            st.diagram_zoom,
            st.diagram_pan_x,
            st.diagram_pan_y,
            st.selected_state_node.clone(),
            st.hovered_state_node.clone(),
            st.node_flash_animation.clone(),
            st.edge_reveal_animation,
            st.edge_pulse_animation.clone(),
            st.selected_state_machine,
        )
    });
    let layout = match &layout_opt {
        Some(l) => l,
        None => {
            // Draw empty placeholder message
            cr.set_source_rgb(colors.canvas_bg.0, colors.canvas_bg.1, colors.canvas_bg.2);
            cr.rectangle(0.0, 0.0, width, height);
            let _ = cr.fill();

            cr.select_font_face(
                tokens::font::CAIRO_SANS,
                cairo::FontSlant::Normal,
                cairo::FontWeight::Bold,
            );
            cr.set_font_size(14.0);
            cr.set_source_rgb(
                colors.text_muted.0,
                colors.text_muted.1,
                colors.text_muted.2,
            );
            cr.move_to(width * 0.35, height * 0.5);
            let _ = cr.show_text("No state machines detected in current project.");
            return;
        }
    };

    let selected_node = selected_node.as_deref();
    let hovered_node = hovered_node.as_deref();

    let (flash_node, flash_progress) = match node_flash {
        Some((ref node, start)) if tokens::motion::is_animations_enabled() => {
            let elapsed = start.elapsed().as_millis() as f64;
            let dur = tokens::motion::DURATION_SHORT_MS as f64;
            let t = (elapsed / dur).clamp(0.0, 1.0);
            (Some(node.clone()), tokens::motion::ease_out_cubic(t))
        }
        _ => (None, 1.0),
    };

    let edge_reveal_progress = match edge_reveal {
        Some((idx, start)) if idx == selected_sm_idx && tokens::motion::is_animations_enabled() => {
            let elapsed = start.elapsed().as_millis() as f64;
            let dur = tokens::motion::DURATION_MEDIUM_MS as f64;
            let t = (elapsed / dur).clamp(0.0, 1.0);
            tokens::motion::ease_out_cubic(t)
        }
        _ => 1.0,
    };

    let (pulse_edge, pulse_t) = match edge_pulse {
        Some((ref from, ref to, start)) if tokens::motion::is_animations_enabled() => {
            let elapsed = start.elapsed().as_millis() as f64;
            let dur = tokens::motion::DURATION_MEDIUM_MS as f64;
            let t = (elapsed / dur).clamp(0.0, 1.0);
            (Some((from.clone(), to.clone())), t)
        }
        _ => (None, 0.0),
    };

    // Fill canvas background
    cr.set_source_rgb(colors.canvas_bg.0, colors.canvas_bg.1, colors.canvas_bg.2);
    cr.rectangle(0.0, 0.0, width.max(4000.0), height.max(4000.0));
    let _ = cr.fill();

    // Apply zoom and pan transforms
    cr.translate(pan_x, pan_y);
    cr.scale(zoom, zoom);

    // Draw technical dot grid
    draw_dot_grid(
        cr,
        layout.width.max(1600.0),
        layout.height.max(1200.0),
        &colors,
    );

    // Draw Lane Header Banners (Interface talking -> Sans)
    for lane in &layout.lanes {
        if lane.name != "INITIAL" {
            cr.select_font_face(
                tokens::font::CAIRO_SANS,
                cairo::FontSlant::Normal,
                cairo::FontWeight::Bold,
            );
            cr.set_font_size(8.5);
            if selected_node.is_some() {
                cr.set_source_rgba(
                    colors.text_muted.0,
                    colors.text_muted.1,
                    colors.text_muted.2,
                    colors.lane_alpha_selected,
                );
            } else {
                cr.set_source_rgba(
                    colors.text_muted.0,
                    colors.text_muted.1,
                    colors.text_muted.2,
                    colors.lane_alpha_normal,
                );
            }
            let banner_text = if lane.name == "FAULT" {
                "// GLOBAL FAULT HANDLER".to_string()
            } else {
                format!("// FLOW LANE: {}", lane.name)
            };
            cr.move_to(lane.x_start, lane.y - 12.0);
            let _ = cr.show_text(&banner_text);
        }
    }

    // 2. Draw Edges
    for edge in &layout.edges {
        let is_connected_to_selection = match selected_node {
            Some(sel) => edge.from == sel || edge.to == sel,
            None => false,
        };

        // DECLUTTERING RULE:
        // When NO node is selected, collapsed high-fan-in edges (like the 12 transitions to FAULT)
        // are NOT drawn as full lines. They are represented by badges on the source nodes.
        if selected_node.is_none() && edge.is_high_fan_in {
            continue;
        }

        // When a node IS selected, only draw edges connected to that node.
        // Non-connected high-fan-in edges stay collapsed to keep clutter low.
        if selected_node.is_some() && !is_connected_to_selection && edge.is_high_fan_in {
            continue;
        }

        let is_dimmed = selected_node.is_some() && !is_connected_to_selection;

        // Draw orthogonal connector
        cr.new_path();
        if !edge.waypoints.is_empty() {
            cr.move_to(edge.waypoints[0].0, edge.waypoints[0].1);
            for pt in &edge.waypoints[1..] {
                cr.line_to(pt.0, pt.1);
            }
        } else {
            cr.move_to(edge.start.0, edge.start.1);
            cr.curve_to(
                edge.control1.0,
                edge.control1.1,
                edge.control2.0,
                edge.control2.1,
                edge.end.0,
                edge.end.1,
            );
        }

        let (edge_r, edge_g, edge_b, edge_a) = if edge.is_fault {
            if is_dimmed {
                (
                    colors.fault_red.0,
                    colors.fault_red.1,
                    colors.fault_red.2,
                    colors.fault_edge_alpha_dimmed,
                )
            } else if is_connected_to_selection {
                (
                    colors.fault_red.0,
                    colors.fault_red.1,
                    colors.fault_red.2,
                    1.0,
                )
            } else {
                (
                    colors.fault_red.0,
                    colors.fault_red.1,
                    colors.fault_red.2,
                    colors.fault_edge_alpha_normal,
                )
            }
        } else if is_connected_to_selection {
            (colors.accent.0, colors.accent.1, colors.accent.2, 0.95)
        } else if is_dimmed {
            (
                colors.edge_default.0,
                colors.edge_default.1,
                colors.edge_default.2,
                colors.edge_alpha_dimmed,
            )
        } else {
            (
                colors.edge_default.0,
                colors.edge_default.1,
                colors.edge_default.2,
                colors.edge_alpha_default,
            )
        };

        let is_pulse_edge = match &pulse_edge {
            Some((ref from, ref to)) => edge.from == *from && edge.to == *to && pulse_t < 1.0,
            None => false,
        };

        let total_len = if edge.waypoints.len() >= 2 {
            let mut l = 0.0;
            for i in 0..(edge.waypoints.len() - 1) {
                let dx = edge.waypoints[i + 1].0 - edge.waypoints[i].0;
                let dy = edge.waypoints[i + 1].1 - edge.waypoints[i].1;
                l += (dx * dx + dy * dy).sqrt();
            }
            l
        } else {
            let dx = edge.end.0 - edge.start.0;
            let dy = edge.end.1 - edge.start.1;
            (dx * dx + dy * dy).sqrt().max(10.0)
        };

        cr.set_source_rgba(edge_r, edge_g, edge_b, edge_a);
        cr.set_line_width(colors.edge_width);
        if edge_reveal_progress < 1.0 {
            let visible_len = (total_len * edge_reveal_progress).max(0.1);
            cr.set_dash(&[visible_len, total_len * 2.0], 0.0);
            if is_pulse_edge {
                let _ = cr.stroke_preserve();
            } else {
                let _ = cr.stroke();
            }
            cr.set_dash(&[], 0.0);
        } else {
            if is_pulse_edge {
                let _ = cr.stroke_preserve();
            } else {
                let _ = cr.stroke();
            }
        }

        // Overlay pass: traveling signal pulse along active state transition edge
        if is_pulse_edge {
            cr.set_source_rgba(colors.accent.0, colors.accent.1, colors.accent.2, 1.0);
            cr.set_line_width(2.0);
            let pulse_len = 24.0_f64.min(total_len * 0.4).max(8.0);
            cr.set_dash(&[pulse_len, total_len * 2.0], -(pulse_t * total_len));
            let _ = cr.stroke();
            cr.set_dash(&[], 0.0);
            cr.set_line_width(colors.edge_width);
        }

        // Draw arrowhead at edge.end aligned with the final segment only if reveal is nearly complete
        if edge_reveal_progress >= 0.95 {
            let (arrow_dx, arrow_dy) = if edge.waypoints.len() >= 2 {
                let n = edge.waypoints.len();
                (
                    edge.waypoints[n - 1].0 - edge.waypoints[n - 2].0,
                    edge.waypoints[n - 1].1 - edge.waypoints[n - 2].1,
                )
            } else {
                (edge.end.0 - edge.control2.0, edge.end.1 - edge.control2.1)
            };
            let angle = arrow_dy.atan2(arrow_dx);
            let arrow_len = 8.0;

            cr.new_path();
            cr.move_to(edge.end.0, edge.end.1);
            cr.line_to(
                edge.end.0 - arrow_len * (angle - 0.4).cos(),
                edge.end.1 - arrow_len * (angle - 0.4).sin(),
            );
            cr.line_to(
                edge.end.0 - arrow_len * (angle + 0.4).cos(),
                edge.end.1 - arrow_len * (angle + 0.4).sin(),
            );
            cr.close_path();
            cr.set_source_rgba(edge_r, edge_g, edge_b, edge_a);
            let _ = cr.fill();
        }
    }

    // 3. Draw Nodes
    for (id, node) in &layout.nodes {
        let is_selected = selected_node == Some(id.as_str());
        let is_hovered = hovered_node == Some(id.as_str());
        let is_flashing = flash_node.as_deref() == Some(id.as_str()) && flash_progress < 1.0;
        let flash_intensity = if is_flashing {
            1.0 - flash_progress
        } else {
            0.0
        };

        let is_connected_to_selection = match selected_node {
            Some(sel) => {
                node.id == sel
                    || layout.edges.iter().any(|e| {
                        (e.from == sel && e.to == node.id) || (e.to == sel && e.from == node.id)
                    })
            }
            None => true,
        };
        let is_dimmed_node = selected_node.is_some() && !is_connected_to_selection;

        draw_rounded_node(
            cr,
            node.x,
            node.y,
            node.width,
            node.height,
            tokens::shape::BORDER_RADIUS_ELEMENT as f64,
            node.is_fault,
            node.is_initial,
            is_selected,
            is_hovered,
            is_dimmed_node,
            flash_intensity,
            &colors,
        );

        // Draw node label (Data -> JetBrains Mono)
        cr.select_font_face(
            tokens::font::CAIRO_MONO,
            cairo::FontSlant::Normal,
            cairo::FontWeight::Bold,
        );
        cr.set_font_size(12.0);

        if is_dimmed_node {
            cr.set_source_rgba(
                colors.text_muted.0,
                colors.text_muted.1,
                colors.text_muted.2,
                colors.dimmed_text_alpha,
            );
        } else if node.is_fault {
            cr.set_source_rgb(colors.fault_red.0, colors.fault_red.1, colors.fault_red.2);
        } else {
            cr.set_source_rgb(
                colors.text_primary.0,
                colors.text_primary.1,
                colors.text_primary.2,
            );
        }

        let ext = cr.text_extents(&node.label);
        let text_w = ext.as_ref().map(|e| e.width()).unwrap_or(0.0);
        let text_h = ext.as_ref().map(|e| e.height()).unwrap_or(0.0);

        let text_x = node.x + (node.width - text_w) * 0.5;
        let text_y = node.y + (node.height + text_h) * 0.5 - 2.0;
        cr.move_to(text_x, text_y);
        let _ = cr.show_text(&node.label);

        // Top-left tag badge (INITIAL or FAULT) - Interface talking -> Sans
        if node.is_initial {
            cr.select_font_face(
                tokens::font::CAIRO_SANS,
                cairo::FontSlant::Normal,
                cairo::FontWeight::Bold,
            );
            cr.set_font_size(8.0);
            if is_dimmed_node {
                cr.set_source_rgba(
                    colors.text_muted.0,
                    colors.text_muted.1,
                    colors.text_muted.2,
                    colors.dimmed_tag_alpha,
                );
            } else {
                cr.set_source_rgb(
                    colors.text_muted.0,
                    colors.text_muted.1,
                    colors.text_muted.2,
                );
            }
            cr.move_to(node.x + 8.0, node.y + 12.0);
            let _ = cr.show_text("[INIT]");
        } else if node.is_fault {
            cr.select_font_face(
                tokens::font::CAIRO_SANS,
                cairo::FontSlant::Normal,
                cairo::FontWeight::Bold,
            );
            cr.set_font_size(8.0);
            if is_dimmed_node {
                cr.set_source_rgba(
                    colors.fault_red.0,
                    colors.fault_red.1,
                    colors.fault_red.2,
                    colors.dimmed_tag_alpha,
                );
            } else {
                cr.set_source_rgb(colors.fault_red.0, colors.fault_red.1, colors.fault_red.2);
            }
            cr.move_to(node.x + 8.0, node.y + 12.0);
            let _ = cr.show_text("[FAULT]");
        }

        // Footer badges:
        // On FAULT node: show incoming trigger count
        if node.is_fault && node.incoming_count > 0 {
            let badge_text = format!("{} Triggers", node.incoming_count);
            draw_node_badge(
                cr,
                node.x + node.width - 70.0,
                node.y + node.height - 15.0,
                &badge_text,
                true,
                is_dimmed_node,
                &colors,
            );
        }

        // On non-fault nodes: show collapsed indicator badges for distant high-fan-in targets (e.g. FAULT)
        if !node.is_fault {
            for (idx, target) in node.collapsed_out_badges.iter().enumerate() {
                let is_fault_target = is_fault_state(target);
                let badge_label = if is_fault_target {
                    "[FAULT]"
                } else {
                    target.as_ref()
                };
                let bx = node.x + node.width - 56.0 - idx as f64 * 58.0;
                let by = node.y + node.height - 15.0;
                draw_node_badge(
                    cr,
                    bx,
                    by,
                    badge_label,
                    is_fault_target,
                    is_dimmed_node,
                    &colors,
                );
            }
        }
    }

    // 4. Draw Edge Guard Badges (rendered on top of edges and nodes for clean legibility)
    for edge in &layout.edges {
        if edge.display_guard.is_empty() {
            continue;
        }

        let is_connected_to_selection = match selected_node {
            Some(sel) => edge.from == sel || edge.to == sel,
            None => false,
        };

        if selected_node.is_none() && edge.is_high_fan_in {
            continue;
        }

        if selected_node.is_some() && !is_connected_to_selection && edge.is_high_fan_in {
            continue;
        }

        if edge_reveal_progress >= 0.85 {
            let is_dimmed = selected_node.is_some() && !is_connected_to_selection;
            draw_guard_badge(
                cr,
                edge.label_pos.0,
                edge.label_pos.1,
                &edge.display_guard,
                edge.is_fault,
                is_dimmed,
                &colors,
            );
        }
    }
}

fn draw_dot_grid(cr: &cairo::Context, max_w: f64, max_h: f64, colors: &DiagramColors) {
    cr.set_source_rgba(
        colors.dot_grid.0,
        colors.dot_grid.1,
        colors.dot_grid.2,
        colors.dot_grid.3,
    );
    let step = 32.0;
    let mut x = 20.0;
    cr.new_path();
    while x < max_w {
        let mut y = 20.0;
        while y < max_h {
            cr.rectangle(x, y, 1.0, 1.0);
            y += step;
        }
        x += step;
    }
    let _ = cr.fill();
}

#[allow(clippy::too_many_arguments)]
fn draw_rounded_node(
    cr: &cairo::Context,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    r: f64,
    is_fault: bool,
    is_initial: bool,
    is_selected: bool,
    is_hovered: bool,
    is_dimmed: bool,
    flash_intensity: f64,
    colors: &DiagramColors,
) {
    if !colors.is_default {
        // Two offset translucent rounded rectangles for soft elevation without blur filters
        let (a_far, a_near) = if is_dimmed {
            (0.04, 0.06)
        } else {
            (0.07, 0.11)
        };
        trace_rounded_rect(cr, x + 1.0, y + 4.0, w, h, r);
        cr.set_source_rgba(
            colors.text_primary.0,
            colors.text_primary.1,
            colors.text_primary.2,
            a_far,
        );
        let _ = cr.fill();

        trace_rounded_rect(cr, x + 0.5, y + 2.0, w, h, r);
        cr.set_source_rgba(
            colors.text_primary.0,
            colors.text_primary.1,
            colors.text_primary.2,
            a_near,
        );
        let _ = cr.fill();

        // Selected node: 6px soft accent glow around outer edge (alpha <= 0.18)
        if is_selected {
            trace_rounded_rect(
                cr,
                x - STAGE_SELECTED_GLOW_WIDTH * 0.5,
                y - STAGE_SELECTED_GLOW_WIDTH * 0.5,
                w + STAGE_SELECTED_GLOW_WIDTH,
                h + STAGE_SELECTED_GLOW_WIDTH,
                r + STAGE_SELECTED_GLOW_WIDTH * 0.5,
            );
            cr.set_source_rgba(
                colors.accent.0,
                colors.accent.1,
                colors.accent.2,
                STAGE_SELECTED_GLOW_ALPHA,
            );
            cr.set_line_width(STAGE_SELECTED_GLOW_WIDTH);
            let _ = cr.stroke();
        }
    }

    trace_rounded_rect(cr, x, y, w, h, r);

    // Node fill
    if is_dimmed {
        cr.set_source_rgba(
            colors.node_fill_dimmed.0,
            colors.node_fill_dimmed.1,
            colors.node_fill_dimmed.2,
            colors.node_fill_dimmed.3,
        );
    } else if is_fault {
        cr.set_source_rgba(
            colors.fault_fill.0,
            colors.fault_fill.1,
            colors.fault_fill.2,
            colors.fault_fill.3,
        );
    } else if is_selected {
        cr.set_source_rgba(
            colors.node_fill_selected.0,
            colors.node_fill_selected.1,
            colors.node_fill_selected.2,
            colors.node_fill_selected.3,
        );
    } else if is_hovered {
        cr.set_source_rgba(
            colors.node_fill_hover.0,
            colors.node_fill_hover.1,
            colors.node_fill_hover.2,
            colors.node_fill_hover.3,
        );
    } else if is_initial {
        cr.set_source_rgba(
            colors.node_fill_initial.0,
            colors.node_fill_initial.1,
            colors.node_fill_initial.2,
            colors.node_fill_initial.3,
        );
    } else {
        cr.set_source_rgba(
            colors.node_fill_default.0,
            colors.node_fill_default.1,
            colors.node_fill_default.2,
            colors.node_fill_default.3,
        );
    }
    let _ = cr.fill_preserve();

    if flash_intensity > 0.0 {
        cr.set_source_rgba(
            colors.accent.0,
            colors.accent.1,
            colors.accent.2,
            0.40 * flash_intensity,
        );
        let _ = cr.fill_preserve();
    }

    // Stage: 3px left ribbon bar clipped cleanly inside the rounded node rectangle
    if !colors.is_default {
        if cr.save().is_ok() {
            trace_rounded_rect(cr, x, y, w, h, r);
            cr.clip();
            let ribbon = if is_initial {
                colors.gold
            } else {
                colors.azure
            };
            let ribbon_alpha = if is_dimmed { 0.55 } else { 1.0 };
            cr.set_source_rgba(ribbon.0, ribbon.1, ribbon.2, ribbon_alpha);
            cr.rectangle(x, y, STAGE_RIBBON_WIDTH, h);
            let _ = cr.fill();
            let _ = cr.restore();
        }
        trace_rounded_rect(cr, x, y, w, h, r);
    }

    // Node stroke
    if flash_intensity > 0.0 {
        cr.set_source_rgba(
            colors.accent.0,
            colors.accent.1,
            colors.accent.2,
            (0.6 + 0.4 * flash_intensity).min(1.0),
        );
    } else if is_dimmed {
        cr.set_source_rgba(
            colors.border_default.0,
            colors.border_default.1,
            colors.border_default.2,
            colors.dimmed_border_alpha,
        );
    } else if !colors.is_default && is_selected {
        cr.set_source_rgb(
            colors.border_selected.0,
            colors.border_selected.1,
            colors.border_selected.2,
        );
    } else if is_fault {
        cr.set_source_rgb(colors.fault_red.0, colors.fault_red.1, colors.fault_red.2);
    } else if is_selected {
        cr.set_source_rgb(
            colors.border_selected.0,
            colors.border_selected.1,
            colors.border_selected.2,
        );
    } else if is_hovered {
        cr.set_source_rgb(
            colors.border_hover.0,
            colors.border_hover.1,
            colors.border_hover.2,
        );
    } else if is_initial {
        cr.set_source_rgb(
            colors.border_initial.0,
            colors.border_initial.1,
            colors.border_initial.2,
        );
    } else {
        cr.set_source_rgb(
            colors.border_default.0,
            colors.border_default.1,
            colors.border_default.2,
        );
    }
    let stroke_width = if !colors.is_default && is_selected {
        STAGE_SELECTED_BORDER_WIDTH
    } else {
        colors.node_border_width
    };
    cr.set_line_width(stroke_width);
    let _ = cr.stroke();
}

fn draw_node_badge(
    cr: &cairo::Context,
    x: f64,
    y: f64,
    text: &str,
    is_fault: bool,
    is_dimmed: bool,
    colors: &DiagramColors,
) {
    cr.select_font_face(
        tokens::font::CAIRO_MONO,
        cairo::FontSlant::Normal,
        cairo::FontWeight::Bold,
    );
    cr.set_font_size(7.5);

    let (ext_w, ext_h, ext_xb, ext_yb) = if let Ok(e) = cr.text_extents(text) {
        (e.width(), e.height(), e.x_bearing(), e.y_bearing())
    } else {
        (0.0, 0.0, 0.0, 0.0)
    };

    let pad_x = 4.0;
    let pad_y = 2.0;
    let w = ext_w + pad_x * 2.0;
    let h = ext_h + pad_y * 2.0;
    let r = tokens::shape::BORDER_RADIUS_ELEMENT as f64;

    cr.new_path();
    cr.arc(x + w - r, y + r, r, -std::f64::consts::FRAC_PI_2, 0.0);
    cr.arc(x + w - r, y + h - r, r, 0.0, std::f64::consts::FRAC_PI_2);
    cr.arc(
        x + r,
        y + h - r,
        r,
        std::f64::consts::FRAC_PI_2,
        std::f64::consts::PI,
    );
    cr.arc(
        x + r,
        y + r,
        r,
        std::f64::consts::PI,
        3.0 * std::f64::consts::FRAC_PI_2,
    );
    cr.close_path();

    let dimmed_fill_alpha = if colors.is_default { 0.3 } else { 0.65 };
    let dimmed_border_alpha = if colors.is_default {
        0.25
    } else {
        colors.dimmed_border_alpha
    };

    if is_fault {
        if is_dimmed {
            cr.set_source_rgba(
                colors.fault_fill.0,
                colors.fault_fill.1,
                colors.fault_fill.2,
                dimmed_fill_alpha,
            );
            let _ = cr.fill_preserve();
            cr.set_source_rgba(
                colors.fault_red.0,
                colors.fault_red.1,
                colors.fault_red.2,
                dimmed_border_alpha,
            );
        } else {
            cr.set_source_rgba(
                colors.fault_fill.0,
                colors.fault_fill.1,
                colors.fault_fill.2,
                0.95,
            );
            let _ = cr.fill_preserve();
            cr.set_source_rgba(
                colors.fault_red.0,
                colors.fault_red.1,
                colors.fault_red.2,
                0.85,
            );
        }
    } else {
        let badge_fill = if colors.is_default {
            colors.bg_panel
        } else {
            colors.bg_raised
        };
        if is_dimmed {
            cr.set_source_rgba(badge_fill.0, badge_fill.1, badge_fill.2, dimmed_fill_alpha);
            let _ = cr.fill_preserve();
            cr.set_source_rgba(
                colors.border_default.0,
                colors.border_default.1,
                colors.border_default.2,
                dimmed_border_alpha,
            );
        } else {
            cr.set_source_rgba(badge_fill.0, badge_fill.1, badge_fill.2, 0.95);
            let _ = cr.fill_preserve();
            cr.set_source_rgba(
                colors.border_default.0,
                colors.border_default.1,
                colors.border_default.2,
                0.85,
            );
        }
    }
    cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
    let _ = cr.stroke();

    if is_fault {
        if is_dimmed {
            cr.set_source_rgba(
                colors.fault_red.0,
                colors.fault_red.1,
                colors.fault_red.2,
                colors.dimmed_tag_alpha,
            );
        } else {
            cr.set_source_rgb(colors.fault_red.0, colors.fault_red.1, colors.fault_red.2);
        }
    } else {
        if is_dimmed {
            cr.set_source_rgba(
                colors.text_muted.0,
                colors.text_muted.1,
                colors.text_muted.2,
                colors.dimmed_tag_alpha,
            );
        } else {
            cr.set_source_rgb(
                colors.text_primary.0,
                colors.text_primary.1,
                colors.text_primary.2,
            );
        }
    }
    cr.move_to(x + pad_x - ext_xb, y + pad_y - ext_yb);
    let _ = cr.show_text(text);
}

fn draw_guard_badge(
    cr: &cairo::Context,
    center_x: f64,
    center_y: f64,
    text: &str,
    is_fault: bool,
    is_dimmed: bool,
    colors: &DiagramColors,
) {
    cr.select_font_face(
        tokens::font::CAIRO_MONO,
        cairo::FontSlant::Normal,
        cairo::FontWeight::Normal,
    );
    cr.set_font_size(9.5);

    let (ext_w, ext_h, ext_xb, ext_yb) = if let Ok(e) = cr.text_extents(text) {
        (e.width(), e.height(), e.x_bearing(), e.y_bearing())
    } else {
        (0.0, 0.0, 0.0, 0.0)
    };

    let padding_x = 6.0;
    let padding_y = 3.0;
    let badge_w = ext_w + padding_x * 2.0;
    let badge_h = ext_h + padding_y * 2.0;
    let badge_x = center_x - badge_w * 0.5;
    let badge_y = center_y - badge_h * 0.5;
    let r = tokens::shape::BORDER_RADIUS_ELEMENT as f64;

    cr.new_path();
    cr.arc(
        badge_x + badge_w - r,
        badge_y + r,
        r,
        -std::f64::consts::FRAC_PI_2,
        0.0,
    );
    cr.arc(
        badge_x + badge_w - r,
        badge_y + badge_h - r,
        r,
        0.0,
        std::f64::consts::FRAC_PI_2,
    );
    cr.arc(
        badge_x + r,
        badge_y + badge_h - r,
        r,
        std::f64::consts::FRAC_PI_2,
        std::f64::consts::PI,
    );
    cr.arc(
        badge_x + r,
        badge_y + r,
        r,
        std::f64::consts::PI,
        3.0 * std::f64::consts::FRAC_PI_2,
    );
    cr.close_path();

    // Badge fill
    if is_dimmed {
        cr.set_source_rgba(
            colors.pill_fill.0,
            colors.pill_fill.1,
            colors.pill_fill.2,
            colors.pill_fill_dimmed_alpha,
        );
    } else {
        cr.set_source_rgba(
            colors.pill_fill.0,
            colors.pill_fill.1,
            colors.pill_fill.2,
            colors.pill_fill.3,
        );
    }
    let _ = cr.fill_preserve();

    // Badge stroke (1px hairline)
    let dimmed_pill_border_alpha = if colors.is_default {
        0.3
    } else {
        colors.dimmed_border_alpha
    };
    if is_fault {
        if is_dimmed {
            cr.set_source_rgba(
                colors.fault_red.0,
                colors.fault_red.1,
                colors.fault_red.2,
                dimmed_pill_border_alpha,
            );
        } else {
            cr.set_source_rgba(
                colors.fault_red.0,
                colors.fault_red.1,
                colors.fault_red.2,
                0.85,
            );
        }
    } else if is_dimmed {
        cr.set_source_rgba(
            colors.pill_border.0,
            colors.pill_border.1,
            colors.pill_border.2,
            dimmed_pill_border_alpha,
        );
    } else {
        cr.set_source_rgba(
            colors.pill_border.0,
            colors.pill_border.1,
            colors.pill_border.2,
            colors.pill_border_alpha,
        );
    }
    cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
    let _ = cr.stroke();

    // Badge text
    if is_fault {
        if is_dimmed {
            cr.set_source_rgba(
                colors.fault_red.0,
                colors.fault_red.1,
                colors.fault_red.2,
                colors.dimmed_tag_alpha,
            );
        } else {
            cr.set_source_rgb(colors.fault_red.0, colors.fault_red.1, colors.fault_red.2);
        }
    } else if is_dimmed {
        cr.set_source_rgba(
            colors.text_muted.0,
            colors.text_muted.1,
            colors.text_muted.2,
            colors.dimmed_tag_alpha,
        );
    } else {
        cr.set_source_rgb(
            colors.text_primary.0,
            colors.text_primary.1,
            colors.text_primary.2,
        );
    }

    cr.move_to(badge_x + padding_x - ext_xb, badge_y + padding_y - ext_yb);
    let _ = cr.show_text(text);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_draw_empty_state_diagram_canvas() {
        let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 1200, 800)
            .expect("Failed to create surface");
        let cr = cairo::Context::new(&surface).expect("Failed to create context");
        let state = Rc::new(RefCell::new(AppState::default()));

        draw_state_diagram(&cr, 1200.0, 800.0, &state);
        surface.flush();
    }

    #[test]
    fn test_draw_docking_firmware_state_diagram_canvas() {
        let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 1200, 800)
            .expect("Failed to create surface");
        let cr = cairo::Context::new(&surface).expect("Failed to create context");
        let state = Rc::new(RefCell::new(AppState::default()));

        let fixture_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../stakhal-core/tests/fixtures/docking_firmware");
        let ioc_path = fixture_dir.join("docking_firmware.ioc");
        let main_c_path = fixture_dir.join("Core/Src/main.c");

        let project = stakhal_core::ir::schema::load_project(&ioc_path, &main_c_path)
            .expect("Failed to load docking firmware fixture");
        assert_eq!(project.state_machines.len(), 1);

        state.borrow().project.borrow_mut().loaded_project = Some(project);

        // 1. Initial draw (unselected default view - low clutter with collapsed badges)
        draw_state_diagram(&cr, 1200.0, 800.0, &state);
        surface.flush();

        // Verify layout was computed
        assert!(state
            .borrow()
            .with_canvas_state(|c| c.state_diagram_layout.is_some()));
        let layout = state
            .borrow()
            .with_canvas_state(|c| c.state_diagram_layout.clone())
            .unwrap();
        assert_eq!(layout.nodes.len(), 14);
        assert_eq!(layout.edges.len(), 31);
        assert!(!layout.lanes.is_empty());
        assert!(layout.lanes.iter().any(|l| l.name == "CALIBRATION"));
        assert!(layout.lanes.iter().any(|l| l.name == "FAULT"));

        // 2. Select GOING node (expand outgoing edges including to FAULT)
        state
            .borrow()
            .with_canvas_state_mut(|c| c.selected_state_node = Some("GOING".to_string()));
        draw_state_diagram(&cr, 1200.0, 800.0, &state);
        surface.flush();

        // 3. Select FAULT node (expand all 12 fault triggers)
        state
            .borrow()
            .with_canvas_state_mut(|c| c.selected_state_node = Some("FAULT".to_string()));
        draw_state_diagram(&cr, 1200.0, 800.0, &state);
        surface.flush();

        // 4. Hover CALIBRATING node while FAULT is selected
        state
            .borrow()
            .with_canvas_state_mut(|c| c.hovered_state_node = Some("CALIBRATING".to_string()));
        draw_state_diagram(&cr, 1200.0, 800.0, &state);
        surface.flush();

        // 5. Deselect (click background -> return to low clutter collapsed view)
        state.borrow().with_canvas_state_mut(|c| {
            c.selected_state_node = None;
            c.hovered_state_node = None;
        });
        draw_state_diagram(&cr, 1200.0, 800.0, &state);
        surface.flush();
    }

    #[test]
    fn test_state_diagram_animations() {
        let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 1200, 800)
            .expect("Failed to create surface");
        let cr = cairo::Context::new(&surface).expect("Failed to create context");
        let state = Rc::new(RefCell::new(AppState::default()));

        let fixture_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../stakhal-core/tests/fixtures/docking_firmware");
        let ioc_path = fixture_dir.join("docking_firmware.ioc");
        let main_c_path = fixture_dir.join("Core/Src/main.c");

        let project = stakhal_core::ir::schema::load_project(&ioc_path, &main_c_path)
            .expect("Failed to load docking firmware fixture");
        state.borrow().project.borrow_mut().loaded_project = Some(project);

        // 1. Test Node Click Flash Animation
        state.borrow().with_canvas_state_mut(|c| {
            c.node_flash_animation = Some(("GOING".to_string(), std::time::Instant::now()));
        });
        draw_state_diagram(&cr, 1200.0, 800.0, &state);
        surface.flush();

        // 2. Test Edge Reveal Animation in progress (t=0)
        state.borrow().with_canvas_state_mut(|c| {
            c.node_flash_animation = None;
            c.edge_reveal_animation = Some((0, std::time::Instant::now()));
        });
        draw_state_diagram(&cr, 1200.0, 800.0, &state);
        surface.flush();

        // 3. Test Edge Reveal Animation finished
        state.borrow().with_canvas_state_mut(|c| {
            c.edge_reveal_animation = None;
        });
        draw_state_diagram(&cr, 1200.0, 800.0, &state);
        surface.flush();

        // 4. Test Session Revealed Machine Tracking
        let is_revealed = state
            .borrow()
            .with_canvas_state(|c| c.session_revealed_machines.contains(&0));
        assert!(!is_revealed);
        state.borrow().with_canvas_state_mut(|c| {
            c.session_revealed_machines.insert(0);
        });
        let is_revealed2 = state
            .borrow()
            .with_canvas_state(|c| c.session_revealed_machines.contains(&0));
        assert!(is_revealed2);
    }

    #[test]
    fn test_state_diagram_edge_pulse_animation() {
        let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 1200, 800)
            .expect("Failed to create surface");
        let cr = cairo::Context::new(&surface).expect("Failed to create context");
        let state = Rc::new(RefCell::new(AppState::default()));

        let fixture_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../stakhal-core/tests/fixtures/docking_firmware");
        let ioc_path = fixture_dir.join("docking_firmware.ioc");
        let main_c_path = fixture_dir.join("Core/Src/main.c");

        let project = stakhal_core::ir::schema::load_project(&ioc_path, &main_c_path)
            .expect("Failed to load docking firmware fixture");
        state.borrow().project.borrow_mut().loaded_project = Some(project);

        // Select RETURNED state to highlight connecting edges
        state.borrow().with_canvas_state_mut(|c| {
            c.selected_state_node = Some("RETURNED".to_string());
        });

        // 1. Milestone t = 0ms
        let start_0 = std::time::Instant::now();
        state.borrow().with_canvas_state_mut(|c| {
            c.edge_pulse_animation =
                Some(("RETURNING".to_string(), "RETURNED".to_string(), start_0));
        });
        let elapsed_0 = start_0.elapsed().as_millis() as f64;
        let t_0 = (elapsed_0 / tokens::motion::DURATION_MEDIUM_MS as f64).clamp(0.0, 1.0);
        assert!(t_0 < 0.05, "Progress at t=0ms must be ~0.0, got {}", t_0);
        draw_state_diagram(&cr, 1200.0, 800.0, &state);
        surface.flush();

        // 2. Milestone mid (t = 175ms)
        let start_mid = std::time::Instant::now() - std::time::Duration::from_millis(175);
        state.borrow().with_canvas_state_mut(|c| {
            c.edge_pulse_animation =
                Some(("RETURNING".to_string(), "RETURNED".to_string(), start_mid));
        });
        let elapsed_mid = start_mid.elapsed().as_millis() as f64;
        let t_mid = (elapsed_mid / tokens::motion::DURATION_MEDIUM_MS as f64).clamp(0.0, 1.0);
        assert!(
            (t_mid - 0.5).abs() < 0.05,
            "Progress at mid must be ~0.5, got {}",
            t_mid
        );
        draw_state_diagram(&cr, 1200.0, 800.0, &state);
        surface.flush();

        // Save visual artifact at midpoint
        let artifact_dir = std::env::temp_dir().join(format!(
            "stakhal_test_state_diagram_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::create_dir_all(&artifact_dir);
        if artifact_dir.exists() {
            if let Ok(mut f) =
                std::fs::File::create(artifact_dir.join("state_diagram_edge_pulse_mid.png"))
            {
                let _ = surface.write_to_png(&mut f);
            }
        }

        // 3. Milestone at exactly t = 350ms
        let start_350 = std::time::Instant::now() - std::time::Duration::from_millis(350);
        state.borrow().with_canvas_state_mut(|c| {
            c.edge_pulse_animation =
                Some(("RETURNING".to_string(), "RETURNED".to_string(), start_350));
        });
        let elapsed_350 = start_350.elapsed().as_millis() as f64;
        let t_350 = (elapsed_350 / tokens::motion::DURATION_MEDIUM_MS as f64).clamp(0.0, 1.0);
        assert_eq!(t_350, 1.0, "Progress at exactly 350ms must equal 1.0");
        draw_state_diagram(&cr, 1200.0, 800.0, &state);
        surface.flush();

        // 4. Milestone at t = 380ms (+30ms past duration)
        let start_380 = std::time::Instant::now() - std::time::Duration::from_millis(380);
        state.borrow().with_canvas_state_mut(|c| {
            c.edge_pulse_animation =
                Some(("RETURNING".to_string(), "RETURNED".to_string(), start_380));
        });
        let elapsed_380 = start_380.elapsed().as_millis() as f64;
        let t_380 = (elapsed_380 / tokens::motion::DURATION_MEDIUM_MS as f64).clamp(0.0, 1.0);
        assert_eq!(
            t_380, 1.0,
            "Progress at +30ms (380ms) must be clamped to 1.0"
        );
        draw_state_diagram(&cr, 1200.0, 800.0, &state);
        surface.flush();

        // 5. Milestone at t = 430ms (+80ms past duration)
        let start_430 = std::time::Instant::now() - std::time::Duration::from_millis(430);
        state.borrow().with_canvas_state_mut(|c| {
            c.edge_pulse_animation =
                Some(("RETURNING".to_string(), "RETURNED".to_string(), start_430));
        });
        let elapsed_430 = start_430.elapsed().as_millis() as f64;
        let t_430 = (elapsed_430 / tokens::motion::DURATION_MEDIUM_MS as f64).clamp(0.0, 1.0);
        assert_eq!(
            t_430, 1.0,
            "Progress at +80ms (430ms) must be clamped to 1.0"
        );
        draw_state_diagram(&cr, 1200.0, 800.0, &state);
        surface.flush();

        // 6. Reduced-motion path: zero pulse, no rendering artifact
        state.borrow().with_canvas_state_mut(|c| {
            c.edge_pulse_animation = None;
        });
        draw_state_diagram(&cr, 1200.0, 800.0, &state);
        surface.flush();
    }

    struct PaletteResetGuard(tokens::Palette);
    impl Drop for PaletteResetGuard {
        fn drop(&mut self) {
            tokens::set_palette(self.0);
        }
    }

    fn rel_luminance(rgb: (f64, f64, f64)) -> f64 {
        fn lin(c: f64) -> f64 {
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        }
        0.2126 * lin(rgb.0) + 0.7152 * lin(rgb.1) + 0.0722 * lin(rgb.2)
    }

    fn contrast_ratio(a: (f64, f64, f64), b: (f64, f64, f64)) -> f64 {
        let la = rel_luminance(a);
        let lb = rel_luminance(b);
        let (hi, lo) = if la >= lb { (la, lb) } else { (lb, la) };
        (hi + 0.05) / (lo + 0.05)
    }

    #[test]
    fn test_diagram_colors_default_and_stage_light_theme() {
        let def = DiagramColors::from_palette(tokens::Palette::DEFAULT);
        assert!(def.is_default);
        assert_eq!(def.edge_default, tokens::Palette::DEFAULT.line.rgb());
        assert_eq!(
            def.border_default,
            tokens::Palette::DEFAULT.node_border.rgb()
        );
        assert_eq!(def.border_selected, tokens::Palette::DEFAULT.accent.rgb());
        assert_eq!(def.pill_border, tokens::Palette::DEFAULT.border_hair.rgb());
        assert_eq!(
            def.pill_fill,
            (
                tokens::Palette::DEFAULT.bg_panel.0,
                tokens::Palette::DEFAULT.bg_panel.1,
                tokens::Palette::DEFAULT.bg_panel.2,
                0.95
            )
        );
        assert_eq!(def.edge_width, tokens::shape::BORDER_WIDTH_HAIR);
        assert_eq!(def.node_border_width, tokens::shape::BORDER_WIDTH_HAIR);
        assert_eq!(def.edge_alpha_default, 0.85);
        assert_eq!(def.edge_alpha_dimmed, 0.25);

        let stg = DiagramColors::from_palette(tokens::Palette::STAGE);
        assert!(!stg.is_default);
        assert_eq!(stg.edge_default, tokens::Palette::STAGE.line.rgb());
        assert_eq!(stg.border_default, tokens::Palette::STAGE.node_border.rgb());
        assert_eq!(stg.border_selected, tokens::Palette::STAGE.accent.rgb());
        assert_eq!(stg.border_hover, tokens::Palette::STAGE.accent.rgb());
        assert_eq!(stg.pill_border, tokens::Palette::STAGE.line.rgb());
        assert_eq!(stg.pill_border_alpha, 1.0);
        assert_eq!(stg.text_primary, tokens::Palette::STAGE.text_primary.rgb());
        assert_eq!(stg.azure, tokens::Palette::STAGE.azure.rgb());
        assert_eq!(stg.gold, tokens::Palette::STAGE.gold.rgb());
        assert_eq!(stg.edge_width, 1.5);
        assert_eq!(stg.node_border_width, 1.5);
        assert_eq!(STAGE_SELECTED_BORDER_WIDTH, 2.0);
        assert_eq!(STAGE_SELECTED_GLOW_WIDTH, 6.0);
        const {
            assert!(STAGE_SELECTED_GLOW_ALPHA <= 0.18);
        }
        assert_eq!(STAGE_RIBBON_WIDTH, 3.0);
        assert_eq!(
            stg.pill_fill,
            (
                tokens::Palette::STAGE.bg_panel.0,
                tokens::Palette::STAGE.bg_panel.1,
                tokens::Palette::STAGE.bg_panel.2,
                1.0
            )
        );
        assert_eq!(
            stg.node_fill_default,
            (
                tokens::Palette::STAGE.bg_panel.0,
                tokens::Palette::STAGE.bg_panel.1,
                tokens::Palette::STAGE.bg_panel.2,
                1.0
            )
        );
        assert_eq!(
            stg.node_fill_selected,
            (
                tokens::Palette::STAGE.bg_raised.0,
                tokens::Palette::STAGE.bg_raised.1,
                tokens::Palette::STAGE.bg_raised.2,
                1.0
            )
        );
    }

    #[test]
    fn test_stage_state_diagram_all_text_and_fill_contrast() {
        let stg = DiagramColors::from_palette(tokens::Palette::STAGE);
        let composite = |fg: (f64, f64, f64), alpha: f64, bg: (f64, f64, f64)| {
            mix_rgb(bg, fg, alpha.clamp(0.0, 1.0))
        };

        let dim_pill_bg = composite(
            (stg.pill_fill.0, stg.pill_fill.1, stg.pill_fill.2),
            stg.pill_fill_dimmed_alpha,
            stg.canvas_bg,
        );
        let dim_badge_bg = composite(stg.bg_raised, 0.65, stg.canvas_bg);
        let fault_fill_rgb = (stg.fault_fill.0, stg.fault_fill.1, stg.fault_fill.2);
        let dim_fault_badge_bg = composite(fault_fill_rgb, 0.65, stg.canvas_bg);

        let pairs = [
            (
                "empty placeholder vs canvas_bg",
                stg.text_muted,
                stg.canvas_bg,
            ),
            (
                "lane banner (normal) vs canvas_bg",
                composite(stg.text_muted, stg.lane_alpha_normal, stg.canvas_bg),
                stg.canvas_bg,
            ),
            (
                "lane banner (selected node active) vs canvas_bg",
                composite(stg.text_muted, stg.lane_alpha_selected, stg.canvas_bg),
                stg.canvas_bg,
            ),
            (
                "normal node label vs node_fill_default",
                stg.text_primary,
                (
                    stg.node_fill_default.0,
                    stg.node_fill_default.1,
                    stg.node_fill_default.2,
                ),
            ),
            (
                "initial node label vs node_fill_initial",
                stg.text_primary,
                (
                    stg.node_fill_initial.0,
                    stg.node_fill_initial.1,
                    stg.node_fill_initial.2,
                ),
            ),
            (
                "[INIT] tag vs node_fill_initial",
                stg.text_muted,
                (
                    stg.node_fill_initial.0,
                    stg.node_fill_initial.1,
                    stg.node_fill_initial.2,
                ),
            ),
            (
                "selected node label vs node_fill_selected",
                stg.text_primary,
                (
                    stg.node_fill_selected.0,
                    stg.node_fill_selected.1,
                    stg.node_fill_selected.2,
                ),
            ),
            (
                "hovered node label vs node_fill_hover",
                stg.text_primary,
                (
                    stg.node_fill_hover.0,
                    stg.node_fill_hover.1,
                    stg.node_fill_hover.2,
                ),
            ),
            (
                "fault node label & [FAULT] tag vs fault_fill",
                stg.fault_red,
                fault_fill_rgb,
            ),
            (
                "dimmed node label vs node_fill_dimmed",
                composite(
                    stg.text_muted,
                    stg.dimmed_text_alpha,
                    (
                        stg.node_fill_dimmed.0,
                        stg.node_fill_dimmed.1,
                        stg.node_fill_dimmed.2,
                    ),
                ),
                (
                    stg.node_fill_dimmed.0,
                    stg.node_fill_dimmed.1,
                    stg.node_fill_dimmed.2,
                ),
            ),
            (
                "dimmed [INIT] tag vs node_fill_dimmed",
                composite(
                    stg.text_muted,
                    stg.dimmed_tag_alpha,
                    (
                        stg.node_fill_dimmed.0,
                        stg.node_fill_dimmed.1,
                        stg.node_fill_dimmed.2,
                    ),
                ),
                (
                    stg.node_fill_dimmed.0,
                    stg.node_fill_dimmed.1,
                    stg.node_fill_dimmed.2,
                ),
            ),
            (
                "dimmed [FAULT] tag vs node_fill_dimmed",
                composite(
                    stg.fault_red,
                    stg.dimmed_tag_alpha,
                    (
                        stg.node_fill_dimmed.0,
                        stg.node_fill_dimmed.1,
                        stg.node_fill_dimmed.2,
                    ),
                ),
                (
                    stg.node_fill_dimmed.0,
                    stg.node_fill_dimmed.1,
                    stg.node_fill_dimmed.2,
                ),
            ),
            (
                "edge guard pill (normal) vs pill_fill",
                stg.text_primary,
                (stg.pill_fill.0, stg.pill_fill.1, stg.pill_fill.2),
            ),
            (
                "edge guard pill (fault) vs pill_fill",
                stg.fault_red,
                (stg.pill_fill.0, stg.pill_fill.1, stg.pill_fill.2),
            ),
            (
                "edge guard pill (dimmed) vs dimmed pill_fill",
                composite(stg.text_muted, stg.dimmed_tag_alpha, dim_pill_bg),
                dim_pill_bg,
            ),
            (
                "edge guard pill (dimmed fault) vs dimmed pill_fill",
                composite(stg.fault_red, stg.dimmed_tag_alpha, dim_pill_bg),
                dim_pill_bg,
            ),
            (
                "node badge (normal) vs bg_raised",
                stg.text_primary,
                stg.bg_raised,
            ),
            (
                "node badge (dimmed) vs dimmed badge bg",
                composite(stg.text_muted, stg.dimmed_tag_alpha, dim_badge_bg),
                dim_badge_bg,
            ),
            (
                "fault node badge (dimmed) vs dimmed fault badge bg",
                composite(stg.fault_red, stg.dimmed_tag_alpha, dim_fault_badge_bg),
                dim_fault_badge_bg,
            ),
        ];

        for (name, fg, bg) in pairs {
            let ratio = contrast_ratio(fg, bg);
            assert!(
                ratio >= 4.5,
                "Stage state diagram contrast for '{}' is {:.3}:1 (expected >= 4.5)",
                name,
                ratio
            );
        }
    }

    #[test]
    fn test_stage_palette_state_diagram_renders_light_surfaces_and_edges() {
        let _guard = PaletteResetGuard(tokens::palette());
        tokens::set_palette(tokens::Palette::STAGE);

        let mut surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 1200, 800)
            .expect("Failed to create surface");
        {
            let cr = cairo::Context::new(&surface).expect("Failed to create context");
            let state = Rc::new(RefCell::new(AppState::default()));

            let fixture_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../stakhal-core/tests/fixtures/docking_firmware");
            let ioc_path = fixture_dir.join("docking_firmware.ioc");
            let main_c_path = fixture_dir.join("Core/Src/main.c");
            let project = stakhal_core::ir::schema::load_project(&ioc_path, &main_c_path)
                .expect("Failed to load docking firmware fixture");
            state.borrow().project.borrow_mut().loaded_project = Some(project);
            state.borrow().with_canvas_state_mut(|c| {
                c.selected_state_node = Some("GOING".to_string());
                c.hovered_state_node = Some("CALIBRATING".to_string());
            });

            draw_state_diagram(&cr, 1200.0, 800.0, &state);
            surface.flush();
        }

        let data = surface.data().expect("surface pixel data");
        let mut saw_bg_void = false;
        let mut saw_bg_panel = false;
        let mut saw_node_border_or_line = false;
        let mut saw_azure_ribbon = false;
        let mut saw_gold_ribbon = false;
        // ARGB32 on little-endian stores bytes B, G, R, A
        for px in data.chunks_exact(4) {
            let (b, g, r) = (px[0], px[1], px[2]);
            if r == 0xF4 && g == 0xF9 && b == 0xFD {
                saw_bg_void = true;
            }
            if r > 245 && g > 245 && b > 245 {
                saw_bg_panel = true;
            }
            // Match non-background blue-navy ink from node_border (#3F5FA0), line (#4F76AE), or accent (#2E6DB4)
            if r < 110 && g < 150 && b > 140 {
                saw_node_border_or_line = true;
            }
            // Match Stage azure (#59A0CF) left ribbon bar on non-initial nodes
            if (r as i32 - 0x59).abs() <= 4
                && (g as i32 - 0xA0).abs() <= 4
                && (b as i32 - 0xCF).abs() <= 4
            {
                saw_azure_ribbon = true;
            }
            // Match Stage gold (#C9A24F) left ribbon bar on initial node
            if (r as i32 - 0xC9).abs() <= 4
                && (g as i32 - 0xA2).abs() <= 4
                && (b as i32 - 0x4F).abs() <= 4
            {
                saw_gold_ribbon = true;
            }
        }
        assert!(
            saw_bg_void,
            "Expected Stage light bg_void canvas background"
        );
        assert!(saw_bg_panel, "Expected Stage white node panel surface");
        assert!(
            saw_node_border_or_line,
            "Expected Stage blue/navy borders, edges, or accent strokes"
        );
        assert!(
            saw_azure_ribbon,
            "Expected Stage azure 3px left ribbon on nodes"
        );
        assert!(
            saw_gold_ribbon,
            "Expected Stage gold 3px left ribbon on initial node"
        );
    }

    #[test]
    fn test_stage_node_shadow_ribbon_and_selection_glow() {
        let stage = DiagramColors::from_palette(tokens::Palette::STAGE);
        let default = DiagramColors::from_palette(tokens::Palette::DEFAULT);

        let pixel_at = |surface: &mut cairo::ImageSurface, px_x: i32, px_y: i32| -> (u8, u8, u8) {
            surface.flush();
            let stride = surface.stride() as usize;
            let data = surface.data().expect("surface data");
            let off = px_y as usize * stride + px_x as usize * 4;
            (data[off + 2], data[off + 1], data[off])
        };

        // Stage selected initial node at (40, 40, 175, 54) on bg_void canvas
        let mut s_stg = cairo::ImageSurface::create(cairo::Format::ARgb32, 300, 160).unwrap();
        {
            let cr = cairo::Context::new(&s_stg).unwrap();
            cr.set_source_rgb(stage.canvas_bg.0, stage.canvas_bg.1, stage.canvas_bg.2);
            cr.rectangle(0.0, 0.0, 300.0, 160.0);
            let _ = cr.fill();
            draw_rounded_node(
                &cr, 40.0, 40.0, 175.0, 54.0, 2.0, false, true, true, false, false, 0.0, &stage,
            );
        }
        // Background outside node+glow+shadow stays exact bg_void
        assert_eq!(pixel_at(&mut s_stg, 10, 10), (0xF4, 0xF9, 0xFD));
        // Soft shadow below node (40..215, y=97 inside +2..+4 offset) is darker than bg_void
        let shadow_px = pixel_at(&mut s_stg, 120, 97);
        assert!(
            shadow_px.0 < 0xF4 && shadow_px.1 < 0xF9,
            "Expected soft shadow pixel below Stage node, got {:?}",
            shadow_px
        );
        // 6px selection glow outside left border (x=36 is 4px outside x=40) is tinted toward accent
        let glow_px = pixel_at(&mut s_stg, 36, 67);
        assert!(
            glow_px.0 < 0xF0 && glow_px.2 > glow_px.0,
            "Expected 6px accent glow outside Stage selected node, got {:?}",
            glow_px
        );
        // 3px left ribbon on initial node (x=42, y=67) is Stage gold (#C9A24F)
        let ribbon_gold = pixel_at(&mut s_stg, 42, 67);
        assert!(
            (ribbon_gold.0 as i32 - 0xC9).abs() <= 2
                && (ribbon_gold.1 as i32 - 0xA2).abs() <= 2
                && (ribbon_gold.2 as i32 - 0x4F).abs() <= 2,
            "Expected gold ribbon on Stage initial node, got {:?}",
            ribbon_gold
        );

        // Stage non-initial unselected node shows azure ribbon at (42, 67)
        let mut s_norm = cairo::ImageSurface::create(cairo::Format::ARgb32, 300, 160).unwrap();
        {
            let cr = cairo::Context::new(&s_norm).unwrap();
            cr.set_source_rgb(stage.canvas_bg.0, stage.canvas_bg.1, stage.canvas_bg.2);
            cr.rectangle(0.0, 0.0, 300.0, 160.0);
            let _ = cr.fill();
            draw_rounded_node(
                &cr, 40.0, 40.0, 175.0, 54.0, 2.0, false, false, false, false, false, 0.0, &stage,
            );
        }
        let ribbon_azure = pixel_at(&mut s_norm, 42, 67);
        assert!(
            (ribbon_azure.0 as i32 - 0x59).abs() <= 2
                && (ribbon_azure.1 as i32 - 0xA0).abs() <= 2
                && (ribbon_azure.2 as i32 - 0xCF).abs() <= 2,
            "Expected azure ribbon on Stage normal node, got {:?}",
            ribbon_azure
        );

        // Default skin: no shadow below node and no glow outside left border
        let mut s_def = cairo::ImageSurface::create(cairo::Format::ARgb32, 300, 160).unwrap();
        {
            let cr = cairo::Context::new(&s_def).unwrap();
            cr.set_source_rgb(
                default.canvas_bg.0,
                default.canvas_bg.1,
                default.canvas_bg.2,
            );
            cr.rectangle(0.0, 0.0, 300.0, 160.0);
            let _ = cr.fill();
            draw_rounded_node(
                &cr, 40.0, 40.0, 175.0, 54.0, 2.0, false, true, true, false, false, 0.0, &default,
            );
        }
        assert_eq!(pixel_at(&mut s_def, 120, 97), (0x0A, 0x0D, 0x10));
        assert_eq!(pixel_at(&mut s_def, 36, 67), (0x0A, 0x0D, 0x10));
    }
}
