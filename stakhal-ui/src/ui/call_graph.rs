use std::cell::RefCell;
use std::rc::Rc;
use gtk4::cairo;
use gtk4::prelude::*;
use stakhal_core::graph::builder::{EdgeType, GraphEdge};
use crate::state::{create_icon_button, AppState, AppWidgets, ChainHeaderLayout};

pub fn build_call_graph_panel() -> (gtk4::Box, gtk4::Button, gtk4::DrawingArea) {
    let btn_graph_back = create_icon_button("Back to Overview", "go-previous-symbolic", false);

    let lbl_graph_title = gtk4::Label::builder()
        .label("[ CALL GRAPH DIAGRAM ]")
        .halign(gtk4::Align::Start)
        .hexpand(true)
        .css_classes(vec!["title-3".to_string()])
        .build();

    let lbl_graph_hint = gtk4::Label::builder()
        .label("Click node to highlight connections, drag node to move")
        .halign(gtk4::Align::End)
        .css_classes(vec!["dim-label".to_string(), "caption".to_string()])
        .build();

    let graph_header_bar = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(18)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(18)
        .margin_end(18)
        .build();
    graph_header_bar.append(&btn_graph_back);
    graph_header_bar.append(&lbl_graph_title);
    graph_header_bar.append(&lbl_graph_hint);

    let graph_drawing_area = gtk4::DrawingArea::builder()
        .content_width(2000)
        .content_height(1500)
        .hexpand(true)
        .vexpand(true)
        .build();

    let graph_scrolled = gtk4::ScrolledWindow::builder()
        .child(&graph_drawing_area)
        .hexpand(true)
        .vexpand(true)
        .build();

    let graph_panel_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .build();
    graph_panel_box.append(&graph_header_bar);
    graph_panel_box.append(&graph_scrolled);

    (graph_panel_box, btn_graph_back, graph_drawing_area)
}

pub fn setup_call_graph_drawing_and_gestures(
    state: &Rc<RefCell<AppState>>,
    widgets: &Rc<AppWidgets>,
) {
    let state_draw = Rc::clone(state);
    widgets.graph_drawing_area.set_draw_func(move |_area, cr, width, height| {
        let mut st = state_draw.borrow_mut();
        let edges = match &st.loaded_project {
            Some(p) => p.call_graph_edges.clone(),
            None => return,
        };

        if edges.is_empty() { return; }

        if st.graph_node_positions.is_empty() {
            let (pos, headers) = compute_graph_layout(&edges, &st.collapsed_chains);
            st.graph_node_positions = pos;
            st.chain_headers = headers;
        }

        let selected_node = st.selected_graph_node.clone();
        let positions = st.graph_node_positions.clone();
        let headers = st.chain_headers.clone();
        drop(st);

        let canvas_w = width as f64;
        let canvas_h = height as f64;

        cr.set_source_rgb(0.04, 0.04, 0.04);
        cr.rectangle(0.0, 0.0, canvas_w, canvas_h);
        let _ = cr.fill();

        // Canvas Title Header
        cr.select_font_face("monospace", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
        cr.set_font_size(13.0);
        cr.set_source_rgb(0.43, 0.43, 0.43);
        let _ = cr.move_to(20.0, 30.0);
        let _ = cr.show_text("[ STAKHAL CALL GRAPH CANVAS (DRAGGABLE) ]");

        // Draw Chain Headers (Collapsible bars)
        for h in &headers {
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
        let (highlighted_nodes, highlighted_edges) = if let Some(ref sel) = selected_node {
            let mut connected_n = std::collections::HashSet::new();
            let mut connected_e = std::collections::HashSet::new();
            connected_n.insert(sel.clone());

            for (idx, e) in edges.iter().enumerate() {
                if e.from == *sel || e.to == *sel {
                    connected_e.insert(idx);
                    connected_n.insert(e.from.clone());
                    connected_n.insert(e.to.clone());
                }
            }
            (Some(connected_n), Some(connected_e))
        } else {
            (None, None)
        };

        // Draw Edges
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
                    let offset_y = (dy.abs() * 0.5).max(35.0);
                    let sign_y = if dy >= 0.0 { 1.0 } else { -1.0 };
                    (sx, sy + offset_y * sign_y, ex, ey - offset_y * sign_y)
                } else {
                    let offset_x = (dx.abs() * 0.5).max(35.0);
                    let sign_x = if dx >= 0.0 { 1.0 } else { -1.0 };
                    (sx + offset_x * sign_x, sy, ex - offset_x * sign_x, ey)
                };

                let _ = cr.move_to(sx, sy);
                let _ = cr.curve_to(cp1_x, cp1_y, cp2_x, cp2_y, ex, ey);
                let _ = cr.stroke();

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
            }
        }

        // Draw Nodes
        for (n_id, &(n_x, n_y)) in &positions {
            let n_w = (n_id.len() as f64 * 8.5 + 28.0).max(110.0);
            let n_h = 34.0;
            let radius = 7.0;

            let is_selected = selected_node.as_deref() == Some(n_id.as_str());
            let is_connected = highlighted_nodes.as_ref().map_or(false, |set| set.contains(n_id));
            let is_dimmed = highlighted_nodes.is_some() && !is_connected;

            if is_selected {
                cr.set_source_rgb(0.13, 0.13, 0.13);
                draw_rounded_rectangle(cr, n_x, n_y, n_w, n_h, radius);
                let _ = cr.fill_preserve();
                cr.set_source_rgb(1.0, 1.0, 1.0);
                cr.set_line_width(2.0);
                let _ = cr.stroke();
            } else if is_connected {
                cr.set_source_rgb(0.10, 0.10, 0.10);
                draw_rounded_rectangle(cr, n_x, n_y, n_w, n_h, radius);
                let _ = cr.fill_preserve();
                cr.set_source_rgb(0.67, 0.67, 0.67);
                cr.set_line_width(1.5);
                let _ = cr.stroke();
            } else if is_dimmed {
                cr.set_source_rgb(0.05, 0.05, 0.05);
                draw_rounded_rectangle(cr, n_x, n_y, n_w, n_h, radius);
                let _ = cr.fill_preserve();
                cr.set_source_rgb(0.12, 0.12, 0.12);
                cr.set_line_width(1.0);
                let _ = cr.stroke();
            } else {
                cr.set_source_rgb(0.07, 0.07, 0.07);
                draw_rounded_rectangle(cr, n_x, n_y, n_w, n_h, radius);
                let _ = cr.fill_preserve();
                cr.set_source_rgb(0.16, 0.16, 0.16);
                cr.set_line_width(1.0);
                let _ = cr.stroke();
            }

            let (cat_r, cat_g, cat_b) = get_node_category_color(n_id);
            let _ = cr.save();
            draw_rounded_rectangle(cr, n_x, n_y, n_w, n_h, radius);
            let _ = cr.clip();
            if is_dimmed {
                cr.set_source_rgb(cat_r * 0.4, cat_g * 0.4, cat_b * 0.4);
            } else {
                cr.set_source_rgb(cat_r, cat_g, cat_b);
            }
            cr.rectangle(n_x, n_y, n_w, 3.0);
            let _ = cr.fill();
            let _ = cr.restore();

            if is_selected || is_connected {
                cr.set_source_rgb(1.0, 1.0, 1.0);
            } else if is_dimmed {
                cr.set_source_rgb(0.33, 0.33, 0.33);
            } else {
                cr.set_source_rgb(0.88, 0.88, 0.88);
            }

            cr.select_font_face("monospace", cairo::FontSlant::Normal, if is_selected { cairo::FontWeight::Bold } else { cairo::FontWeight::Normal });
            cr.set_font_size(11.5);
            if let Ok(extents) = cr.text_extents(n_id) {
                let tx = n_x + (n_w - extents.width()) / 2.0;
                let ty = n_y + (n_h + extents.height()) / 2.0 + 1.0;
                let _ = cr.move_to(tx, ty);
                let _ = cr.show_text(n_id);
            }
        }
    });

    let gesture_drag = gtk4::GestureDrag::new();
    gesture_drag.set_button(1);
    let state_drag_begin = Rc::clone(state);

    gesture_drag.connect_drag_begin(move |_, start_x, start_y| {
        let mut st = state_drag_begin.borrow_mut();
        st.drag_start_click_pos = (start_x, start_y);
        st.dragged_graph_node = None;

        let mut clicked_id = None;
        let mut start_pos = (0.0, 0.0);

        for (id, &(nx, ny)) in &st.graph_node_positions {
            let nw = (id.len() as f64 * 8.5 + 28.0).max(110.0);
            let nh = 34.0;
            if start_x >= nx && start_x <= nx + nw && start_y >= ny && start_y <= ny + nh {
                clicked_id = Some(id.clone());
                start_pos = (nx, ny);
                break;
            }
        }

        if let Some(id) = clicked_id {
            st.dragged_graph_node = Some(id);
            st.drag_start_node_pos = start_pos;
        }
    });

    let state_drag_update = Rc::clone(state);
    let area_drag_update = widgets.graph_drawing_area.clone();
    gesture_drag.connect_drag_update(move |_, offset_x, offset_y| {
        let mut st = state_drag_update.borrow_mut();
        if let Some(ref node_id) = st.dragged_graph_node.clone() {
            let (snx, sny) = st.drag_start_node_pos;
            let new_x = (snx + offset_x).max(0.0);
            let new_y = (sny + offset_y).max(0.0);
            st.graph_node_positions.insert(node_id.clone(), (new_x, new_y));
            drop(st);
            area_drag_update.queue_draw();
        }
    });

    let state_drag_end = Rc::clone(state);
    let area_drag_end = widgets.graph_drawing_area.clone();
    gesture_drag.connect_drag_end(move |_, offset_x, offset_y| {
        let dist = offset_x.hypot(offset_y);
        let mut st = state_drag_end.borrow_mut();

        if dist < 5.0 {
            let (cx, cy) = st.drag_start_click_pos;

            // Check if clicked inside a chain header bar
            let mut clicked_header_id = None;
            for h in &st.chain_headers {
                if cx >= h.x && cx <= h.x + h.w && cy >= h.y && cy <= h.y + h.h {
                    clicked_header_id = Some(h.handler_id.clone());
                    break;
                }
            }

            if let Some(handler_id) = clicked_header_id {
                if st.collapsed_chains.contains(&handler_id) {
                    st.collapsed_chains.remove(&handler_id);
                } else {
                    st.collapsed_chains.insert(handler_id);
                }

                if let Some(ref proj) = st.loaded_project {
                    let (pos, headers) = compute_graph_layout(&proj.call_graph_edges, &st.collapsed_chains);
                    st.graph_node_positions = pos;
                    st.chain_headers = headers;
                }
                st.dragged_graph_node = None;
                drop(st);
                area_drag_end.queue_draw();
                return;
            }

            // Otherwise, check node click selection
            let mut clicked_id = None;
            for (id, &(nx, ny)) in &st.graph_node_positions {
                let nw = (id.len() as f64 * 8.5 + 28.0).max(110.0);
                let nh = 34.0;
                if cx >= nx && cx <= nx + nw && cy >= ny && cy <= ny + nh {
                    clicked_id = Some(id.clone());
                    break;
                }
            }

            if let Some(id) = clicked_id {
                if st.selected_graph_node.as_deref() == Some(&id) {
                    st.selected_graph_node = None;
                } else {
                    st.selected_graph_node = Some(id);
                }
            } else {
                st.selected_graph_node = None;
            }
        }

        st.dragged_graph_node = None;
        drop(st);
        area_drag_end.queue_draw();
    });

    widgets.graph_drawing_area.add_controller(gesture_drag);
}

pub fn compute_graph_layout(
    edges: &[GraphEdge],
    collapsed_chains: &std::collections::HashSet<String>,
) -> (
    std::collections::HashMap<String, (f64, f64)>,
    Vec<ChainHeaderLayout>,
) {
    let mut map = std::collections::HashMap::new();
    let mut headers = Vec::new();

    let max_row_w = 720.0;

    // 1. INITIALIZATION SECTION
    let init_edges: Vec<&GraphEdge> = edges
        .iter()
        .filter(|e| e.edge_type == EdgeType::Init)
        .collect();

    let mut init_bottom_y = 160.0;

    if !init_edges.is_empty() {
        let mut target_nodes: Vec<String> = init_edges.iter().map(|e| e.to.clone()).collect();
        target_nodes.sort();
        target_nodes.dedup();

        let spacing_x = 20.0;
        let row_height = 55.0;
        let mut rows: Vec<Vec<(String, f64)>> = Vec::new();

        let mut current_row: Vec<(String, f64)> = Vec::new();
        let mut current_row_w = 0.0;

        for target in &target_nodes {
            let w = (target.len() as f64 * 8.5 + 28.0).max(110.0);
            if !current_row.is_empty()
                && (current_row_w + spacing_x + w > max_row_w || current_row.len() >= 5)
            {
                rows.push(current_row);
                current_row = Vec::new();
                current_row_w = 0.0;
            }
            current_row_w += if current_row.is_empty() {
                w
            } else {
                spacing_x + w
            };
            current_row.push((target.clone(), w));
        }
        if !current_row.is_empty() {
            rows.push(current_row);
        }

        let first_row_w = if let Some(r0) = rows.first() {
            r0.iter().map(|(_, w)| w).sum::<f64>()
                + (r0.len().saturating_sub(1) as f64 * spacing_x)
        } else {
            110.0
        };

        let main_w = 110.0;
        let main_x = (40.0 + (first_row_w / 2.0) - (main_w / 2.0)).max(40.0);
        map.insert("main".to_string(), (main_x, 50.0));

        let mut row_start_y = 130.0;
        for row in rows {
            let mut curr_x = 40.0;
            for (id, w) in row {
                map.insert(id, (curr_x, row_start_y));
                curr_x += w + spacing_x;
            }
            row_start_y += row_height;
        }
        init_bottom_y = row_start_y;
    }

    // 2. INTERRUPT CHAINS SECTION (Stacked vertically, left-aligned with collapsible header bars)
    let irq_entry_edges: Vec<&GraphEdge> = edges
        .iter()
        .filter(|e| e.edge_type == EdgeType::IrqEntry)
        .collect();

    let mut current_chain_y = init_bottom_y + 35.0;

    for irq_edge in &irq_entry_edges {
        let handler_id = irq_edge.from.clone();
        let dispatch_id = irq_edge.to.clone();

        let override_targets: Vec<String> = edges
            .iter()
            .filter(|e| e.edge_type == EdgeType::WeakOverride && e.from == dispatch_id)
            .map(|e| e.to.clone())
            .collect();

        let total_nodes = 1 + 1 + override_targets.len();
        let is_collapsed = collapsed_chains.contains(&handler_id);

        let header_label = format!("{} chain ({} nodes)", handler_id, total_nodes);
        let header_w = (header_label.len() as f64 * 8.0 + 36.0).max(220.0);
        let header_h = 28.0;
        let chain_x = 40.0;

        headers.push(ChainHeaderLayout {
            handler_id: handler_id.clone(),
            label: header_label,
            x: chain_x,
            y: current_chain_y,
            w: header_w,
            h: header_h,
            is_collapsed,
        });

        if is_collapsed {
            current_chain_y += header_h + 16.0;
        } else {
            let handler_w = (handler_id.len() as f64 * 8.5 + 28.0).max(130.0);
            let dispatch_w = (dispatch_id.len() as f64 * 8.5 + 28.0).max(130.0);

            let chain_y_l1 = current_chain_y + 38.0;
            let chain_y_l2 = chain_y_l1 + 65.0;
            let chain_y_l3 = chain_y_l2 + 65.0;

            let mut override_w_sum = 0.0;
            let mut override_nodes_info = Vec::new();
            let mut curr_ov_x = chain_x;

            for ov in &override_targets {
                let w = (ov.len() as f64 * 8.5 + 28.0).max(130.0);
                override_nodes_info.push((ov.clone(), curr_ov_x, w));
                curr_ov_x += w + 20.0;
                override_w_sum += w + 20.0;
            }

            let chain_max_width = handler_w.max(dispatch_w).max(override_w_sum).max(160.0);
            let center_x = chain_x + (chain_max_width / 2.0);

            map.insert(handler_id, (center_x - (handler_w / 2.0), chain_y_l1));
            map.insert(dispatch_id, (center_x - (dispatch_w / 2.0), chain_y_l2));

            for (ov_id, ov_x, _) in override_nodes_info {
                map.insert(ov_id, (ov_x, chain_y_l3));
            }

            let chain_height = if override_targets.is_empty() {
                130.0
            } else {
                195.0
            };

            current_chain_y += 38.0 + chain_height + 25.0;
        }
    }

    (map, headers)
}

fn draw_rounded_rectangle(cr: &cairo::Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    let pi = std::f64::consts::PI;
    cr.new_sub_path();
    cr.arc(x + w - r, y + r, r, -pi / 2.0, 0.0);
    cr.arc(x + w - r, y + h - r, r, 0.0, pi / 2.0);
    cr.arc(x + r, y + h - r, r, pi / 2.0, pi);
    cr.arc(x + r, y + r, r, pi, 3.0 * pi / 2.0);
    cr.close_path();
}

fn get_node_category_color(node_id: &str) -> (f64, f64, f64) {
    if node_id == "main" {
        (0.50, 0.50, 0.50) // Neutral gray
    } else if node_id.starts_with("MX_") || node_id.ends_with("_Init") {
        (0.31, 0.79, 0.69) // Teal / Cyan (#4ec9b0)
    } else if node_id.starts_with("HAL_") && node_id.ends_with("_IRQHandler") {
        (0.77, 0.53, 0.75) // Purple / Magenta (#c586c0) - HAL dispatch
    } else if node_id.ends_with("_IRQHandler") {
        (0.81, 0.57, 0.47) // Orange / Amber (#ce9178) - Vector IRQHandler
    } else if node_id.contains("Callback") || node_id.starts_with("HAL_") {
        (0.34, 0.61, 0.84) // Blue (#569cd6) - Callback nodes
    } else {
        (0.50, 0.50, 0.50) // Fallback neutral gray
    }
}

fn get_rect_ray_intersection(
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
