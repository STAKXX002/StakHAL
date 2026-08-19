use std::cell::RefCell;
use std::rc::Rc;
use gtk4::cairo;
use stakhal_core::graph::{compute_graph_bounds, compute_graph_layout};
use crate::state::AppState;


pub fn draw_call_graph_canvas(
    _area: &gtk4::DrawingArea,
    cr: &cairo::Context,
    _width: f64,
    _height: f64,
    state: &Rc<RefCell<AppState>>,
) {
    {


        let mut st = state.borrow_mut();
        if st.loaded_project.is_none() {
            return;
        }
        if st.graph_node_positions.is_empty() {
            let p = st.loaded_project.as_ref().unwrap();
            let (pos, headers) = compute_graph_layout(&p.call_graph_edges, &st.collapsed_chains);
            let bounds = compute_graph_bounds(&pos, &headers);
            let colors = compute_all_node_status_colors(&p.call_graph_edges, &pos);
            st.graph_node_positions = pos;
            st.node_status_colors = colors;
            st.chain_headers = headers;
            st.graph_bounds = bounds;
        }
    }

    let st = state.borrow();
    let project = match &st.loaded_project {
        Some(p) => p,
        None => return,
    };

    let edges = &project.call_graph_edges;
    if edges.is_empty() {
        return;
    }

    let zoom = st.graph_zoom;
    let pan_x = st.graph_pan_x;
    let pan_y = st.graph_pan_y;

    let selected_node = st.selected_graph_node.as_deref();
    let hovered_node = st.hovered_graph_node.as_deref();
    let positions = &st.graph_node_positions;
    let headers = &st.chain_headers;

    // Fill viewport background
    cr.set_source_rgb(0.04, 0.04, 0.04);
    cr.rectangle(0.0, 0.0, _width.max(3000.0), _height.max(3000.0));
    let _ = cr.fill();

    // Apply translation and zoom transforms
    let _ = cr.translate(pan_x, pan_y);
    let _ = cr.scale(zoom, zoom);


    // Canvas Title Header
    cr.select_font_face("monospace", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(13.0);
    cr.set_source_rgb(0.43, 0.43, 0.43);
    let _ = cr.move_to(20.0, 30.0);
    let _ = cr.show_text("[ STAKHAL CALL GRAPH DIAGRAM (DRAGGABLE) ]");

    // Draw Chain Headers (Collapsible bars)
    for h in headers {
        cr.set_source_rgb(0.08, 0.08, 0.08);
        draw_rounded_rectangle(cr, h.x, h.y, h.w, h.h, 5.0);
        let _ = cr.fill_preserve();
        cr.set_source_rgb(0.20, 0.20, 0.20);
        cr.set_line_width(1.0);
        let _ = cr.stroke();

        cr.select_font_face("monospace", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
        cr.set_font_size(11.5);
        cr.set_source_rgb(0.81, 0.57, 0.47);
        let _ = cr.move_to(h.x + 10.0, h.y + 18.0);
        let icon = if h.is_collapsed { "▸" } else { "▾" };
        let _ = cr.show_text(icon);

        cr.set_source_rgb(0.85, 0.85, 0.85);
        let _ = cr.move_to(h.x + 24.0, h.y + 18.0);
        let _ = cr.show_text(&h.label);
    }

    // Highlights computation
    let (highlighted_nodes, highlighted_edges) = if let Some(sel) = selected_node {
        let mut connected_n = std::collections::HashSet::new();
        let mut connected_e = std::collections::HashSet::new();
        connected_n.insert(sel);

        for (idx, e) in edges.iter().enumerate() {
            if e.from == sel || e.to == sel {
                connected_e.insert(idx);
                connected_n.insert(e.from.as_str());
                connected_n.insert(e.to.as_str());
            }
        }
        (Some(connected_n), Some(connected_e))
    } else {
        (None, None)
    };


    // Draw Edges & Socket Dots
    for (idx, e) in edges.iter().enumerate() {
        let from_pos = positions.get(&e.from);
        let to_pos = positions.get(&e.to);

        if let (Some(&(fx, fy)), Some(&(tx, ty))) = (from_pos, to_pos) {
            let fw = (e.from.len() as f64 * 8.5 + 28.0).max(110.0);
            let fh = 34.0;
            let tw = (e.to.len() as f64 * 8.5 + 28.0).max(110.0);
            let th = 34.0;

            let fc = (fx + fw / 2.0, fy + fh / 2.0);
            let tc = (tx + tw / 2.0, ty + th / 2.0);

            let (sx, sy) = get_rect_ray_intersection(fx, fy, fw, fh, tc.0, tc.1);
            let (ex, ey) = get_rect_ray_intersection(tx, ty, tw, th, fc.0, fc.1);

            let is_hl = highlighted_edges.as_ref().map_or(false, |hl| hl.contains(&idx));
            let is_dimmed = highlighted_edges.is_some() && !is_hl;

            if is_hl {
                cr.set_source_rgb(1.0, 1.0, 1.0);
                cr.set_line_width(2.0);
            } else if is_dimmed {
                cr.set_source_rgb(0.16, 0.16, 0.16);
                cr.set_line_width(1.0);
            } else {
                cr.set_source_rgb(0.43, 0.43, 0.43);
                cr.set_line_width(1.5);
            }

            let dx = ex - sx;
            let dy = ey - sy;

            let (cp1_x, cp1_y, cp2_x, cp2_y) = if dy.abs() >= dx.abs() {
                let offset_y = (dy.abs() * 0.55).max(40.0);
                let sign_y = if dy >= 0.0 { 1.0 } else { -1.0 };
                (sx, sy + offset_y * sign_y, ex, ey - offset_y * sign_y)
            } else {
                let offset_x = (dx.abs() * 0.55).max(40.0);
                let sign_x = if dx >= 0.0 { 1.0 } else { -1.0 };
                (sx + offset_x * sign_x, sy, ex - offset_x * sign_x, ey)
            };

            let _ = cr.move_to(sx, sy);
            let _ = cr.curve_to(cp1_x, cp1_y, cp2_x, cp2_y, ex, ey);
            let _ = cr.stroke();

            // Arrow Tip
            let angle = (ey - cp2_y).atan2(ex - cp2_x);
            let arrow_len = if is_hl { 10.0 } else { 8.0 };
            let arrow_angle = 0.45;

            let x1 = ex - arrow_len * (angle - arrow_angle).cos();
            let y1 = ey - arrow_len * (angle - arrow_angle).sin();
            let x2 = ex - arrow_len * (angle + arrow_angle).cos();
            let y2 = ey - arrow_len * (angle + arrow_angle).sin();

            let _ = cr.move_to(ex, ey);
            let _ = cr.line_to(x1, y1);
            let _ = cr.line_to(x2, y2);
            let _ = cr.close_path();
            let _ = cr.fill();

            // Socket Dots (Source & Target connection circles)
            if !is_dimmed {
                let socket_r = 3.5;
                if is_hl {
                    cr.set_source_rgb(1.0, 1.0, 1.0);
                } else {
                    cr.set_source_rgb(0.52, 0.52, 0.52);
                }
                // Source Socket
                cr.arc(sx, sy, socket_r, 0.0, 2.0 * std::f64::consts::PI);
                let _ = cr.fill();
                // Target Socket
                cr.arc(ex, ey, socket_r, 0.0, 2.0 * std::f64::consts::PI);
                let _ = cr.fill();
            }
        }
    }

    // Draw Nodes (Terminal/Monochrome Style with status-only colored header strip)
    for (n_id, &(n_x, n_y)) in positions {

        let n_w = (n_id.len() as f64 * 8.5 + 28.0).max(110.0);
        let n_h = 34.0;
        let radius = 7.0;

        let is_selected = selected_node == Some(n_id.as_str());
        let is_hovered = hovered_node == Some(n_id.as_str());
        let is_connected = highlighted_nodes.as_ref().map_or(false, |set| set.contains(n_id.as_str()));

        let is_dimmed = highlighted_nodes.is_some() && !is_connected;

        if is_selected {
            cr.set_source_rgb(0.14, 0.14, 0.16);
            draw_rounded_rectangle(cr, n_x, n_y, n_w, n_h, radius);
            let _ = cr.fill_preserve();
            cr.set_source_rgb(1.0, 1.0, 1.0);
            cr.set_line_width(2.0);
            let _ = cr.stroke();
        } else if is_hovered {
            cr.set_source_rgb(0.15, 0.15, 0.18);
            draw_rounded_rectangle(cr, n_x, n_y, n_w, n_h, radius);
            let _ = cr.fill_preserve();
            cr.set_source_rgb(0.90, 0.90, 0.90);
            cr.set_line_width(1.5);
            let _ = cr.stroke();
        } else if is_connected {
            cr.set_source_rgb(0.11, 0.11, 0.13);
            draw_rounded_rectangle(cr, n_x, n_y, n_w, n_h, radius);
            let _ = cr.fill_preserve();
            cr.set_source_rgb(0.67, 0.67, 0.67);
            cr.set_line_width(1.5);
            let _ = cr.stroke();
        } else if is_dimmed {
            cr.set_source_rgb(0.05, 0.05, 0.06);
            draw_rounded_rectangle(cr, n_x, n_y, n_w, n_h, radius);
            let _ = cr.fill_preserve();
            cr.set_source_rgb(0.14, 0.14, 0.14);
            cr.set_line_width(1.0);
            let _ = cr.stroke();
        } else {
            cr.set_source_rgb(0.09, 0.09, 0.11);
            draw_rounded_rectangle(cr, n_x, n_y, n_w, n_h, radius);
            let _ = cr.fill_preserve();
            cr.set_source_rgb(0.20, 0.20, 0.24);
            cr.set_line_width(1.0);
            let _ = cr.stroke();
        }

        // Header Strip (top ~7px of node box filled with status color or neutral monochrome)
        let (cat_r, cat_g, cat_b) = st.node_status_colors.get(n_id).copied().unwrap_or((0.75, 0.75, 0.75));
        let _ = cr.save();
        draw_rounded_rectangle(cr, n_x, n_y, n_w, n_h, radius);
        let _ = cr.clip();
        if is_dimmed {
            cr.set_source_rgb(cat_r * 0.4, cat_g * 0.4, cat_b * 0.4);
        } else {
            cr.set_source_rgb(cat_r, cat_g, cat_b);
        }
        cr.rectangle(n_x, n_y, n_w, 7.0);
        let _ = cr.fill();
        let _ = cr.restore();


        if is_selected || is_hovered || is_connected {
            cr.set_source_rgb(1.0, 1.0, 1.0);
        } else if is_dimmed {
            cr.set_source_rgb(0.33, 0.33, 0.33);
        } else {
            cr.set_source_rgb(0.88, 0.88, 0.88);
        }

        cr.select_font_face(
            "monospace",
            cairo::FontSlant::Normal,
            if is_selected || is_hovered {
                cairo::FontWeight::Bold
            } else {
                cairo::FontWeight::Normal
            },
        );
        cr.set_font_size(11.5);
        if let Ok(extents) = cr.text_extents(n_id) {
            let tx = n_x + (n_w - extents.width()) / 2.0;
            let ty = n_y + (n_h + extents.height()) / 2.0 + 2.0;
            let _ = cr.move_to(tx, ty);
            let _ = cr.show_text(n_id);
        }
    }
}



pub fn draw_rounded_rectangle(cr: &cairo::Context, x: f64, y: f64, w: f64, h: f64, r: f64) {

    let pi = std::f64::consts::PI;
    cr.new_sub_path();
    cr.arc(x + w - r, y + r, r, -pi / 2.0, 0.0);
    cr.arc(x + w - r, y + h - r, r, 0.0, pi / 2.0);
    cr.arc(x + r, y + h - r, r, pi / 2.0, pi);
    cr.arc(x + r, y + r, r, pi, 3.0 * pi / 2.0);
    cr.close_path();
}

pub fn compute_all_node_status_colors(
    edges: &[stakhal_core::graph::builder::GraphEdge],
    positions: &std::collections::HashMap<String, (f64, f64)>,
) -> std::collections::HashMap<String, (f64, f64, f64)> {
    let mut colors = std::collections::HashMap::new();
    for node_id in positions.keys() {
        colors.insert(node_id.clone(), get_node_status_color(node_id, edges));
    }
    colors
}

pub fn get_node_status_color(


    node_id: &str,
    edges: &[stakhal_core::graph::builder::GraphEdge],
) -> (f64, f64, f64) {
    let outgoing_count = edges.iter().filter(|e| e.from == node_id).count();
    let incoming_count = edges.iter().filter(|e| e.to == node_id).count();

    if node_id.ends_with("_IRQHandler") && !node_id.starts_with("HAL_") {
        if outgoing_count == 0 {
            // Error (Red): Unlinked IRQ handler
            return (0.94, 0.27, 0.27);
        } else if outgoing_count > 2 {
            // Warning (Yellow): Shared vector IRQ handler chain
            return (0.96, 0.62, 0.04);
        }
    }

    if node_id.contains("Callback") || node_id.starts_with("HAL_") {
        let has_user_override = edges.iter().any(|e| {
            e.from == node_id && !e.to.starts_with("HAL_") && !e.to.ends_with("_IRQHandler")
        });

        if has_user_override {
            // Ok (Green): User-implemented callback override
            return (0.13, 0.77, 0.37);
        } else if outgoing_count == 0 && incoming_count > 0 {
            // Warning (Yellow): Unhandled weak callback
            return (0.96, 0.62, 0.04);
        }
    }

    // Default neutral monochrome for normal nodes (main, Init, HAL dispatchers)
    (0.75, 0.75, 0.75)
}


pub fn get_rect_ray_intersection(
    rect_x: f64,
    rect_y: f64,
    rect_w: f64,
    rect_h: f64,
    target_x: f64,
    target_y: f64,
) -> (f64, f64) {
    let cx = rect_x + rect_w / 2.0;
    let cy = rect_y + rect_h / 2.0;

    let dx = target_x - cx;
    let dy = target_y - cy;

    if dx == 0.0 && dy == 0.0 {
        return (cx, cy);
    }

    let scale_x = if dx > 0.0 {
        (rect_w / 2.0) / dx
    } else if dx < 0.0 {
        (-rect_w / 2.0) / dx
    } else {
        f64::INFINITY
    };

    let scale_y = if dy > 0.0 {
        (rect_h / 2.0) / dy
    } else if dy < 0.0 {
        (-rect_h / 2.0) / dy
    } else {
        f64::INFINITY
    };

    let scale = scale_x.min(scale_y);

    (cx + dx * scale, cy + dy * scale)
}
