use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use gtk4::cairo;
use stakhal_core::graph::state_transition::{
    compute_project_state_layout, StateKind, StateMachine,
};
use crate::state::AppState;

pub fn draw_state_diagram_canvas(
    _area: &gtk4::DrawingArea,
    cr: &cairo::Context,
    width: f64,
    height: f64,
    state: &Rc<RefCell<AppState>>,
) {
    // Check initialization of layout
    {
        let mut st = state.borrow_mut();
        if st.loaded_project.is_none() {
            return;
        }
        if st.state_model.is_none() {
            if let Some(ref p) = st.loaded_project {
                let model = stakhal_core::graph::state_transition::build_project_state_model(p);
                let (pos, bounds) = compute_project_state_layout(&model, st.selected_peripheral.as_deref());
                st.state_node_positions = pos;
                st.diagram_bounds = bounds;
                st.state_model = Some(model);
            }
        }
    }

    let st = state.borrow();
    let model = match &st.state_model {
        Some(m) => m,
        None => return,
    };

    if model.machines.is_empty() {
        return;
    }

    let zoom = st.diagram_zoom;
    let pan_x = st.diagram_pan_x;
    let pan_y = st.diagram_pan_y;

    let selected_node_id = st.selected_state_node.as_deref();
    let hovered_node_id = st.hovered_state_node.as_deref();
    let positions = &st.state_node_positions;
    let selected_periph = st.selected_peripheral.as_deref();

    // Fill canvas background
    cr.set_source_rgb(0.04, 0.04, 0.04);
    cr.rectangle(0.0, 0.0, width.max(4000.0), height.max(4000.0));
    let _ = cr.fill();

    // Apply translation and zoom transforms
    let _ = cr.translate(pan_x, pan_y);
    let _ = cr.scale(zoom, zoom);

    let active_machines: Vec<&StateMachine> = match selected_periph {
        Some(p) if !p.is_empty() && p != "ALL" => {
            model.machines.iter().filter(|m| m.peripheral == p).collect()
        }
        _ => model.machines.iter().collect(),
    };

    // Calculate highlighted subgraphs
    let (highlighted_nodes, highlighted_transitions) = if let Some(sel) = selected_node_id {
        let mut nodes = HashSet::new();
        let mut transitions = HashSet::new();
        nodes.insert(sel);

        for m in &active_machines {
            for (t_idx, t) in m.transitions.iter().enumerate() {
                if t.from == sel || t.to == sel {
                    transitions.insert(format!("{}_{}", m.peripheral, t_idx));
                    nodes.insert(t.from.as_str());
                    nodes.insert(t.to.as_str());
                }
            }
        }
        (Some(nodes), Some(transitions))
    } else {
        (None, None)
    };

    const NODE_W: f64 = 140.0;
    const NODE_H: f64 = 48.0;
    const RADIUS: f64 = 6.0;

    // Draw Machine Headers and Sections
    for m in &active_machines {
        if let Some(first_state) = m.states.first() {
            if let Some(&(fx, fy)) = positions.get(&first_state.id) {
                cr.select_font_face("monospace", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
                cr.set_font_size(12.0);
                cr.set_source_rgb(0.55, 0.55, 0.55);
                let _ = cr.move_to(fx - 20.0, fy - 40.0);
                let _ = cr.show_text(&format!("[ PERIPHERAL STATE MACHINE: {} ]", m.peripheral));
            }
        }
    }

    // Draw Transitions
    for m in &active_machines {
        for (t_idx, t) in m.transitions.iter().enumerate() {
            let from_pos = positions.get(&t.from);
            let to_pos = positions.get(&t.to);

            if let (Some(&(fx, fy)), Some(&(tx, ty))) = (from_pos, to_pos) {
                let fc = (fx + NODE_W / 2.0, fy + NODE_H / 2.0);
                let tc = (tx + NODE_W / 2.0, ty + NODE_H / 2.0);

                let is_return = tx <= fx;
                let t_key = format!("{}_{}", m.peripheral, t_idx);
                let is_hl = highlighted_transitions.as_ref().map_or(false, |set| set.contains(&t_key));
                let is_dim = highlighted_transitions.is_some() && !is_hl;

                // Compute edge connection anchor points
                let (sx, sy, ex, ey, cp1_x, cp1_y, cp2_x, cp2_y) = if is_return {
                    // Curved backward return arc
                    let sx = fx + NODE_W / 2.0;
                    let sy = fy + NODE_H;
                    let ex = tx + NODE_W / 2.0;
                    let ey = ty + NODE_H;
                    let arc_depth = (fx - tx).abs() * 0.25 + 60.0;
                    (sx, sy, ex, ey, sx, sy + arc_depth, ex, ey + arc_depth)
                } else {
                    let (sx, sy) = get_rect_ray_intersection(fx, fy, NODE_W, NODE_H, tc.0, tc.1);
                    let (ex, ey) = get_rect_ray_intersection(tx, ty, NODE_W, NODE_H, fc.0, fc.1);
                    let dx = ex - sx;
                    let _dy = ey - sy;
                    let offset_x = (dx.abs() * 0.45).max(30.0);
                    (sx, sy, ex, ey, sx + offset_x, sy, ex - offset_x, ey)
                };

                // Transition Stroke Style
                if is_hl {
                    cr.set_source_rgb(1.0, 1.0, 1.0);
                    cr.set_line_width(2.0);
                } else if is_dim {
                    cr.set_source_rgb(0.14, 0.14, 0.14);
                    cr.set_line_width(1.0);
                } else {
                    cr.set_source_rgb(0.38, 0.38, 0.38);
                    cr.set_line_width(1.4);
                }

                let _ = cr.move_to(sx, sy);
                let _ = cr.curve_to(cp1_x, cp1_y, cp2_x, cp2_y, ex, ey);
                let _ = cr.stroke();

                // Arrow Head
                let angle = (ey - cp2_y).atan2(ex - cp2_x);
                let arrow_len = if is_hl { 9.0 } else { 7.5 };
                let arrow_angle = 0.42;

                let x1 = ex - arrow_len * (angle - arrow_angle).cos();
                let y1 = ey - arrow_len * (angle - arrow_angle).sin();
                let x2 = ex - arrow_len * (angle + arrow_angle).cos();
                let y2 = ey - arrow_len * (angle + arrow_angle).sin();

                let _ = cr.move_to(ex, ey);
                let _ = cr.line_to(x1, y1);
                let _ = cr.line_to(x2, y2);
                let _ = cr.close_path();
                let _ = cr.fill();

                // Draw Trigger Pill on Transition Midpoint
                let mid_x = 0.125 * sx + 0.375 * cp1_x + 0.375 * cp2_x + 0.125 * ex;
                let mid_y = 0.125 * sy + 0.375 * cp1_y + 0.375 * cp2_y + 0.125 * ey;

                draw_trigger_pill(cr, mid_x, mid_y, &t.trigger, is_hl, is_dim);
            }
        }
    }

    // Draw State Nodes
    for m in &active_machines {
        for s in &m.states {
            if let Some(&(nx, ny)) = positions.get(&s.id) {
                let is_sel = selected_node_id == Some(s.id.as_str());
                let is_hov = hovered_node_id == Some(s.id.as_str());
                let is_conn = highlighted_nodes.as_ref().map_or(false, |set| set.contains(s.id.as_str()));
                let is_dim = highlighted_nodes.is_some() && !is_conn;

                // Node Body Background
                if is_sel {
                    cr.set_source_rgb(0.16, 0.16, 0.20);
                    draw_rounded_rectangle(cr, nx, ny, NODE_W, NODE_H, RADIUS);
                    let _ = cr.fill_preserve();
                    cr.set_source_rgb(1.0, 1.0, 1.0);
                    cr.set_line_width(2.2);
                    let _ = cr.stroke();
                } else if is_hov {
                    cr.set_source_rgb(0.14, 0.14, 0.16);
                    draw_rounded_rectangle(cr, nx, ny, NODE_W, NODE_H, RADIUS);
                    let _ = cr.fill_preserve();
                    cr.set_source_rgb(0.85, 0.85, 0.85);
                    cr.set_line_width(1.8);
                    let _ = cr.stroke();
                } else if is_conn {
                    cr.set_source_rgb(0.11, 0.11, 0.13);
                    draw_rounded_rectangle(cr, nx, ny, NODE_W, NODE_H, RADIUS);
                    let _ = cr.fill_preserve();
                    cr.set_source_rgb(0.65, 0.65, 0.65);
                    cr.set_line_width(1.4);
                    let _ = cr.stroke();
                } else if is_dim {
                    cr.set_source_rgb(0.06, 0.06, 0.07);
                    draw_rounded_rectangle(cr, nx, ny, NODE_W, NODE_H, RADIUS);
                    let _ = cr.fill_preserve();
                    cr.set_source_rgb(0.16, 0.16, 0.18);
                    cr.set_line_width(1.0);
                    let _ = cr.stroke();
                } else {
                    cr.set_source_rgb(0.09, 0.09, 0.11);
                    draw_rounded_rectangle(cr, nx, ny, NODE_W, NODE_H, RADIUS);
                    let _ = cr.fill_preserve();
                    cr.set_source_rgb(0.24, 0.24, 0.28);
                    cr.set_line_width(1.2);
                    let _ = cr.stroke();
                }

                // Node Status Strip (Top accent bar)
                let (status_r, status_g, status_b) = match s.kind {
                    StateKind::Reset => (0.45, 0.45, 0.45),
                    StateKind::Ready => (0.13, 0.77, 0.36),     // Green: status-ok
                    StateKind::Active => (0.96, 0.62, 0.04),    // Yellow: status-warning
                    StateKind::Handling => (0.92, 0.92, 0.92),  // Bright monochrome
                    StateKind::Callback => (0.80, 0.80, 0.85),  // Neutral monochrome
                    StateKind::Error => (0.93, 0.27, 0.27),     // Red: status-error
                };

                let _ = cr.save();
                draw_rounded_rectangle(cr, nx, ny, NODE_W, NODE_H, RADIUS);
                let _ = cr.clip();
                if is_dim {
                    cr.set_source_rgb(status_r * 0.35, status_g * 0.35, status_b * 0.35);
                } else {
                    cr.set_source_rgb(status_r, status_g, status_b);
                }
                cr.rectangle(nx, ny, NODE_W, 5.0);
                let _ = cr.fill();
                let _ = cr.restore();

                // State Name Label
                cr.select_font_face("monospace", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
                cr.set_font_size(11.5);
                if is_dim {
                    cr.set_source_rgb(0.40, 0.40, 0.40);
                } else if is_sel {
                    cr.set_source_rgb(1.0, 1.0, 1.0);
                } else {
                    cr.set_source_rgb(0.90, 0.90, 0.90);
                }

                let text_width = cr.text_extents(&s.name).map(|e| e.width()).unwrap_or(0.0);
                let text_x = nx + (NODE_W - text_width) / 2.0;
                let text_y = ny + 24.0;
                let _ = cr.move_to(text_x, text_y);
                let _ = cr.show_text(&s.name);

                // State Kind Sub-label
                let kind_str = match s.kind {
                    StateKind::Reset => "RESET",
                    StateKind::Ready => "READY",
                    StateKind::Active => "ACTIVE",
                    StateKind::Handling => "ISR",
                    StateKind::Callback => "CALLBACK",
                    StateKind::Error => "ERROR",
                };
                cr.set_font_size(8.5);
                if is_dim {
                    cr.set_source_rgb(0.28, 0.28, 0.28);
                } else {
                    cr.set_source_rgb(0.55, 0.55, 0.55);
                }
                let k_width = cr.text_extents(kind_str).map(|e| e.width()).unwrap_or(0.0);
                let k_x = nx + (NODE_W - k_width) / 2.0;
                let k_y = ny + 38.0;
                let _ = cr.move_to(k_x, k_y);
                let _ = cr.show_text(kind_str);
            }
        }
    }
}

fn draw_trigger_pill(cr: &cairo::Context, cx: f64, cy: f64, label: &str, is_hl: bool, is_dim: bool) {
    let display_text = if label.len() > 28 {
        format!("{}...", &label[..25])
    } else {
        label.to_string()
    };

    cr.select_font_face("monospace", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
    cr.set_font_size(9.0);

    let text_width = cr.text_extents(&display_text).map(|e| e.width()).unwrap_or(0.0);
    let pill_w = text_width + 12.0;
    let pill_h = 16.0;
    let pill_x = cx - pill_w / 2.0;
    let pill_y = cy - pill_h / 2.0;

    if is_hl {
        cr.set_source_rgb(0.18, 0.18, 0.22);
        draw_rounded_rectangle(cr, pill_x, pill_y, pill_w, pill_h, 4.0);
        let _ = cr.fill_preserve();
        cr.set_source_rgb(0.85, 0.85, 0.85);
        cr.set_line_width(1.2);
        let _ = cr.stroke();
        cr.set_source_rgb(1.0, 1.0, 1.0);
    } else if is_dim {
        cr.set_source_rgb(0.06, 0.06, 0.07);
        draw_rounded_rectangle(cr, pill_x, pill_y, pill_w, pill_h, 4.0);
        let _ = cr.fill_preserve();
        cr.set_source_rgb(0.14, 0.14, 0.15);
        cr.set_line_width(0.8);
        let _ = cr.stroke();
        cr.set_source_rgb(0.28, 0.28, 0.28);
    } else {
        cr.set_source_rgb(0.08, 0.08, 0.10);
        draw_rounded_rectangle(cr, pill_x, pill_y, pill_w, pill_h, 4.0);
        let _ = cr.fill_preserve();
        cr.set_source_rgb(0.24, 0.24, 0.28);
        cr.set_line_width(0.9);
        let _ = cr.stroke();
        cr.set_source_rgb(0.68, 0.68, 0.68);
    }

    let tx = pill_x + 6.0;
    let ty = pill_y + 11.5;
    let _ = cr.move_to(tx, ty);
    let _ = cr.show_text(&display_text);
}

pub fn draw_rounded_rectangle(cr: &cairo::Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    let degrees = std::f64::consts::PI / 180.0;
    cr.new_sub_path();
    cr.arc(x + w - r, y + r, r, -90.0 * degrees, 0.0 * degrees);
    cr.arc(x + w - r, y + h - r, r, 0.0 * degrees, 90.0 * degrees);
    cr.arc(x + r, y + h - r, r, 90.0 * degrees, 180.0 * degrees);
    cr.arc(x + r, y + r, r, 180.0 * degrees, 270.0 * degrees);
    cr.close_path();
}

pub fn get_rect_ray_intersection(rx: f64, ry: f64, rw: f64, rh: f64, px: f64, py: f64) -> (f64, f64) {
    let cx = rx + rw / 2.0;
    let cy = ry + rh / 2.0;
    let dx = px - cx;
    let dy = py - cy;

    if dx.abs() < 1e-6 && dy.abs() < 1e-6 {
        return (cx, cy);
    }

    let hw = rw / 2.0;
    let hh = rh / 2.0;

    let tan_theta = dy / dx;
    let rect_tan = hh / hw;

    if dx.abs() * rect_tan >= dy.abs() {
        if dx > 0.0 {
            (cx + hw, cy + hw * tan_theta)
        } else {
            (cx - hw, cy - hw * tan_theta)
        }
    } else if dy > 0.0 {
        (cx + hh / tan_theta, cy + hh)
    } else {
        (cx - hh / tan_theta, cy - hh)
    }
}
