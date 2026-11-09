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

struct DiagramColors {
    canvas_bg: (f64, f64, f64),
    dot_grid: (f64, f64, f64, f64),
    fault_red: (f64, f64, f64),
    fault_fill: (f64, f64, f64, f64),
    node_fill_default: (f64, f64, f64, f64),
    node_fill_initial: (f64, f64, f64, f64),
    node_fill_hover: (f64, f64, f64, f64),
    node_fill_selected: (f64, f64, f64, f64),
    border_default: (f64, f64, f64),
    border_initial: (f64, f64, f64),
    border_hover: (f64, f64, f64),
    border_selected: (f64, f64, f64),
    bg_void: (f64, f64, f64),
    bg_panel: (f64, f64, f64),
    text_primary: (f64, f64, f64),
    text_muted: (f64, f64, f64),
    accent: (f64, f64, f64),
}

impl DiagramColors {
    fn from_palette(pal: tokens::Palette) -> Self {
        let is_default = pal == tokens::Palette::DEFAULT;

        let fault_fill = if is_default {
            // Preserve historical default FAULT node fill for pixel-identical snapshots
            (45.0 / 255.0, 16.0 / 255.0, 18.0 / 255.0, 0.95)
        } else {
            let m = mix_rgb(pal.bg_panel.rgb(), pal.state_error.rgb(), 0.18);
            (m.0, m.1, m.2, 0.95)
        };

        let node_fill_initial = if is_default {
            // Preserve historical default INITIAL node fill for pixel-identical snapshots
            (24.0 / 255.0, 30.0 / 255.0, 36.0 / 255.0, 0.95)
        } else {
            let m = mix_rgb(pal.bg_panel.rgb(), pal.bg_raised.rgb(), 0.67);
            (m.0, m.1, m.2, 0.95)
        };

        let node_fill_hover = if is_default {
            // Preserve historical default HOVER node fill for pixel-identical snapshots
            (28.0 / 255.0, 36.0 / 255.0, 44.0 / 255.0, 0.95)
        } else {
            let m = mix_rgb(pal.bg_raised.rgb(), pal.border_hair.rgb(), 0.25);
            (m.0, m.1, m.2, 0.95)
        };

        let node_fill_selected = if is_default {
            // Preserve historical default SELECTED node fill for pixel-identical snapshots
            (20.0 / 255.0, 32.0 / 255.0, 38.0 / 255.0, 0.95)
        } else {
            let m = mix_rgb(pal.bg_panel.rgb(), pal.accent.rgb(), 0.08);
            (m.0, m.1, m.2, 0.95)
        };

        let border_initial = if is_default {
            // Preserve historical default INITIAL border for pixel-identical snapshots
            (50.0 / 255.0, 60.0 / 255.0, 70.0 / 255.0)
        } else {
            mix_rgb(pal.border_hair.rgb(), pal.text_muted.rgb(), 0.20)
        };

        let border_hover = if is_default {
            // Preserve historical default HOVER border for pixel-identical snapshots
            (60.0 / 255.0, 75.0 / 255.0, 85.0 / 255.0)
        } else {
            mix_rgb(pal.border_hair.rgb(), pal.text_muted.rgb(), 0.35)
        };

        Self {
            canvas_bg: pal.bg_void.rgb(),
            dot_grid: (
                pal.border_hair.0,
                pal.border_hair.1,
                pal.border_hair.2,
                0.35,
            ),
            fault_red: pal.state_error.rgb(),
            fault_fill,
            node_fill_default: (pal.bg_panel.0, pal.bg_panel.1, pal.bg_panel.2, 0.95),
            node_fill_initial,
            node_fill_hover,
            node_fill_selected,
            border_default: pal.border_hair.rgb(),
            border_initial,
            border_hover,
            border_selected: pal.accent.rgb(),
            bg_void: pal.bg_void.rgb(),
            bg_panel: pal.bg_panel.rgb(),
            text_primary: pal.text_primary.rgb(),
            text_muted: pal.text_muted.rgb(),
            accent: pal.accent.rgb(),
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
                    0.35,
                );
            } else {
                cr.set_source_rgba(
                    colors.text_muted.0,
                    colors.text_muted.1,
                    colors.text_muted.2,
                    0.75,
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
                    0.15,
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
                    0.85,
                )
            }
        } else if is_connected_to_selection {
            (colors.accent.0, colors.accent.1, colors.accent.2, 0.95)
        } else if is_dimmed {
            (
                colors.border_default.0,
                colors.border_default.1,
                colors.border_default.2,
                0.25,
            )
        } else {
            (
                colors.border_default.0,
                colors.border_default.1,
                colors.border_default.2,
                0.85,
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
        cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
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
            cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
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
                0.4,
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
                    0.3,
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
                    0.3,
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

    // Node fill
    if is_dimmed {
        cr.set_source_rgba(colors.bg_void.0, colors.bg_void.1, colors.bg_void.2, 0.5);
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

    // Node stroke (Strict 1px hairline)
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
            0.35,
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
    cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
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

    if is_fault {
        if is_dimmed {
            cr.set_source_rgba(
                colors.fault_fill.0,
                colors.fault_fill.1,
                colors.fault_fill.2,
                0.3,
            );
            let _ = cr.fill_preserve();
            cr.set_source_rgba(
                colors.fault_red.0,
                colors.fault_red.1,
                colors.fault_red.2,
                0.25,
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
        if is_dimmed {
            cr.set_source_rgba(colors.bg_panel.0, colors.bg_panel.1, colors.bg_panel.2, 0.3);
            let _ = cr.fill_preserve();
            cr.set_source_rgba(
                colors.border_default.0,
                colors.border_default.1,
                colors.border_default.2,
                0.25,
            );
        } else {
            cr.set_source_rgba(
                colors.bg_panel.0,
                colors.bg_panel.1,
                colors.bg_panel.2,
                0.95,
            );
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
                0.3,
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
                0.3,
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
        cr.set_source_rgba(colors.bg_panel.0, colors.bg_panel.1, colors.bg_panel.2, 0.4);
    } else {
        cr.set_source_rgba(
            colors.bg_panel.0,
            colors.bg_panel.1,
            colors.bg_panel.2,
            0.95,
        );
    }
    let _ = cr.fill_preserve();

    // Badge stroke (1px hairline)
    if is_fault {
        if is_dimmed {
            cr.set_source_rgba(
                colors.fault_red.0,
                colors.fault_red.1,
                colors.fault_red.2,
                0.3,
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
            colors.border_default.0,
            colors.border_default.1,
            colors.border_default.2,
            0.3,
        );
    } else {
        cr.set_source_rgba(
            colors.border_default.0,
            colors.border_default.1,
            colors.border_default.2,
            0.85,
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
                0.3,
            );
        } else {
            cr.set_source_rgb(colors.fault_red.0, colors.fault_red.1, colors.fault_red.2);
        }
    } else if is_dimmed {
        cr.set_source_rgba(
            colors.text_muted.0,
            colors.text_muted.1,
            colors.text_muted.2,
            0.3,
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
}
