use std::cell::RefCell;
use std::rc::Rc;
use gtk4::cairo;
use stakhal_core::graph::{compute_state_machine_layout, is_fault_state};
use crate::state::AppState;
use crate::ui::tokens;

// Strict Design Tokens Integration
const COLOR_CANVAS_BG: (f64, f64, f64) = tokens::color::BG_VOID;
const COLOR_DOT_GRID: (f64, f64, f64, f64) = (
    tokens::color::BORDER_HAIR.0,
    tokens::color::BORDER_HAIR.1,
    tokens::color::BORDER_HAIR.2,
    0.35,
);

// Fault tokens (strictly reserved for FAULT state and fault transitions)
const COLOR_FAULT_RED: (f64, f64, f64) = tokens::color::STATE_ERROR;
const COLOR_FAULT_FILL: (f64, f64, f64, f64) = (45.0 / 255.0, 16.0 / 255.0, 18.0 / 255.0, 0.95);

// Panel surface fills
const COLOR_NODE_FILL_DEFAULT: (f64, f64, f64, f64) = (
    tokens::color::BG_PANEL.0,
    tokens::color::BG_PANEL.1,
    tokens::color::BG_PANEL.2,
    0.95,
);
const COLOR_NODE_FILL_INITIAL: (f64, f64, f64, f64) = (24.0 / 255.0, 30.0 / 255.0, 36.0 / 255.0, 0.95);
const COLOR_NODE_FILL_HOVER: (f64, f64, f64, f64) = (28.0 / 255.0, 36.0 / 255.0, 44.0 / 255.0, 0.95);
const COLOR_NODE_FILL_SELECTED: (f64, f64, f64, f64) = (20.0 / 255.0, 32.0 / 255.0, 38.0 / 255.0, 0.95);

// 1px Hairline Borders
const COLOR_BORDER_DEFAULT: (f64, f64, f64) = tokens::color::BORDER_HAIR;
const COLOR_BORDER_INITIAL: (f64, f64, f64) = (50.0 / 255.0, 60.0 / 255.0, 70.0 / 255.0);
const COLOR_BORDER_HOVER: (f64, f64, f64) = (60.0 / 255.0, 75.0 / 255.0, 85.0 / 255.0);
const COLOR_BORDER_SELECTED: (f64, f64, f64) = tokens::color::ACCENT;

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
    // 1. Ensure state diagram layout is computed and view is fitted if needed
    {
        let mut st = state.borrow_mut();
        if st.state_diagram_layout.is_none() {
            let loaded_project = st.project.borrow().loaded_project.clone();
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
            if let Some((lw, lh)) = st.state_diagram_layout.as_ref().map(|l| (l.width, l.height)) {
                let avail_w = (width - 40.0).max(100.0);
                let avail_h = (height - 40.0).max(100.0);
                let fit_zoom = (avail_w / lw).min(avail_h / lh).min(1.0).max(0.2);
                st.diagram_zoom = fit_zoom;
                st.diagram_pan_x = ((width - lw * fit_zoom) * 0.5).max(20.0);
                st.diagram_pan_y = ((height - lh * fit_zoom) * 0.5).max(20.0);
                st.diagram_needs_fit = false;
            }
        }
    }

    let st = state.borrow();
    let layout = match &st.state_diagram_layout {
        Some(l) => l,
        None => {
            // Draw empty placeholder message
            cr.set_source_rgb(COLOR_CANVAS_BG.0, COLOR_CANVAS_BG.1, COLOR_CANVAS_BG.2);
            cr.rectangle(0.0, 0.0, width, height);
            let _ = cr.fill();

            cr.select_font_face(tokens::font::CAIRO_SANS, cairo::FontSlant::Normal, cairo::FontWeight::Bold);
            cr.set_font_size(14.0);
            cr.set_source_rgb(tokens::color::TEXT_MUTED.0, tokens::color::TEXT_MUTED.1, tokens::color::TEXT_MUTED.2);
            let _ = cr.move_to(width * 0.35, height * 0.5);
            let _ = cr.show_text("No state machines detected in current project.");
            return;
        }
    };

    let zoom = st.diagram_zoom;
    let pan_x = st.diagram_pan_x;
    let pan_y = st.diagram_pan_y;
    let selected_node = st.selected_state_node.as_deref();
    let hovered_node = st.hovered_state_node.as_deref();

    // Fill canvas background
    cr.set_source_rgb(COLOR_CANVAS_BG.0, COLOR_CANVAS_BG.1, COLOR_CANVAS_BG.2);
    cr.rectangle(0.0, 0.0, width.max(4000.0), height.max(4000.0));
    let _ = cr.fill();

    // Apply zoom and pan transforms
    let _ = cr.translate(pan_x, pan_y);
    let _ = cr.scale(zoom, zoom);

    // Draw technical dot grid
    draw_dot_grid(cr, layout.width.max(1600.0), layout.height.max(1200.0));

    // Draw Lane Header Banners (Interface talking -> Sans)
    for lane in &layout.lanes {
        if lane.name != "INITIAL" {
            cr.select_font_face(tokens::font::CAIRO_SANS, cairo::FontSlant::Normal, cairo::FontWeight::Bold);
            cr.set_font_size(8.5);
            if selected_node.is_some() {
                cr.set_source_rgba(tokens::color::TEXT_MUTED.0, tokens::color::TEXT_MUTED.1, tokens::color::TEXT_MUTED.2, 0.35);
            } else {
                cr.set_source_rgba(tokens::color::TEXT_MUTED.0, tokens::color::TEXT_MUTED.1, tokens::color::TEXT_MUTED.2, 0.75);
            }
            let banner_text = if lane.name == "FAULT" {
                "// GLOBAL FAULT HANDLER".to_string()
            } else {
                format!("// FLOW LANE: {}", lane.name)
            };
            let _ = cr.move_to(lane.x_start, lane.y - 12.0);
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
                (COLOR_FAULT_RED.0, COLOR_FAULT_RED.1, COLOR_FAULT_RED.2, 0.15)
            } else if is_connected_to_selection {
                (COLOR_FAULT_RED.0, COLOR_FAULT_RED.1, COLOR_FAULT_RED.2, 1.0)
            } else {
                (COLOR_FAULT_RED.0, COLOR_FAULT_RED.1, COLOR_FAULT_RED.2, 0.85)
            }
        } else if is_connected_to_selection {
            (tokens::color::ACCENT.0, tokens::color::ACCENT.1, tokens::color::ACCENT.2, 0.95)
        } else if is_dimmed {
            (COLOR_BORDER_DEFAULT.0, COLOR_BORDER_DEFAULT.1, COLOR_BORDER_DEFAULT.2, 0.25)
        } else {
            (COLOR_BORDER_DEFAULT.0, COLOR_BORDER_DEFAULT.1, COLOR_BORDER_DEFAULT.2, 0.85)
        };

        cr.set_source_rgba(edge_r, edge_g, edge_b, edge_a);
        cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
        let _ = cr.stroke();

        // Draw arrowhead at edge.end aligned with the final segment
        let (arrow_dx, arrow_dy) = if edge.waypoints.len() >= 2 {
            let n = edge.waypoints.len();
            (edge.waypoints[n - 1].0 - edge.waypoints[n - 2].0, edge.waypoints[n - 1].1 - edge.waypoints[n - 2].1)
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

    // 3. Draw Nodes
    for (id, node) in &layout.nodes {
        let is_selected = selected_node == Some(id.as_str());
        let is_hovered = hovered_node == Some(id.as_str());

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
        );

        // Draw node label (Data -> JetBrains Mono)
        cr.select_font_face(tokens::font::CAIRO_MONO, cairo::FontSlant::Normal, cairo::FontWeight::Bold);
        cr.set_font_size(12.0);

        if is_dimmed_node {
            cr.set_source_rgba(tokens::color::TEXT_MUTED.0, tokens::color::TEXT_MUTED.1, tokens::color::TEXT_MUTED.2, 0.4);
        } else if node.is_fault {
            cr.set_source_rgb(COLOR_FAULT_RED.0, COLOR_FAULT_RED.1, COLOR_FAULT_RED.2);
        } else if is_selected {
            cr.set_source_rgb(tokens::color::TEXT_PRIMARY.0, tokens::color::TEXT_PRIMARY.1, tokens::color::TEXT_PRIMARY.2);
        } else {
            cr.set_source_rgb(tokens::color::TEXT_PRIMARY.0, tokens::color::TEXT_PRIMARY.1, tokens::color::TEXT_PRIMARY.2);
        }

        let ext = cr.text_extents(&node.label);
        let text_w = ext.as_ref().map(|e| e.width()).unwrap_or(0.0);
        let text_h = ext.as_ref().map(|e| e.height()).unwrap_or(0.0);

        let text_x = node.x + (node.width - text_w) * 0.5;
        let text_y = node.y + (node.height + text_h) * 0.5 - 2.0;
        let _ = cr.move_to(text_x, text_y);
        let _ = cr.show_text(&node.label);

        // Top-left tag badge (INITIAL or FAULT) - Interface talking -> Sans
        if node.is_initial {
            cr.select_font_face(tokens::font::CAIRO_SANS, cairo::FontSlant::Normal, cairo::FontWeight::Bold);
            cr.set_font_size(8.0);
            if is_dimmed_node {
                cr.set_source_rgba(tokens::color::TEXT_MUTED.0, tokens::color::TEXT_MUTED.1, tokens::color::TEXT_MUTED.2, 0.3);
            } else {
                cr.set_source_rgb(tokens::color::TEXT_MUTED.0, tokens::color::TEXT_MUTED.1, tokens::color::TEXT_MUTED.2);
            }
            let _ = cr.move_to(node.x + 8.0, node.y + 12.0);
            let _ = cr.show_text("[INIT]");
        } else if node.is_fault {
            cr.select_font_face(tokens::font::CAIRO_SANS, cairo::FontSlant::Normal, cairo::FontWeight::Bold);
            cr.set_font_size(8.0);
            if is_dimmed_node {
                cr.set_source_rgba(COLOR_FAULT_RED.0, COLOR_FAULT_RED.1, COLOR_FAULT_RED.2, 0.3);
            } else {
                cr.set_source_rgb(COLOR_FAULT_RED.0, COLOR_FAULT_RED.1, COLOR_FAULT_RED.2);
            }
            let _ = cr.move_to(node.x + 8.0, node.y + 12.0);
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
            );
        }

        // On non-fault nodes: show collapsed indicator badges for distant high-fan-in targets (e.g. FAULT)
        if !node.is_fault {
            for (idx, target) in node.collapsed_out_badges.iter().enumerate() {
                let is_fault_target = is_fault_state(target);
                let badge_label = if is_fault_target { "[FAULT]" } else { target.as_str() };
                let bx = node.x + node.width - 56.0 - idx as f64 * 58.0;
                let by = node.y + node.height - 15.0;
                draw_node_badge(cr, bx, by, badge_label, is_fault_target, is_dimmed_node);
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

        let is_dimmed = selected_node.is_some() && !is_connected_to_selection;
        draw_guard_badge(
            cr,
            edge.label_pos.0,
            edge.label_pos.1,
            &edge.display_guard,
            edge.is_fault,
            is_dimmed,
        );
    }
}

fn draw_dot_grid(cr: &cairo::Context, max_w: f64, max_h: f64) {
    cr.set_source_rgba(COLOR_DOT_GRID.0, COLOR_DOT_GRID.1, COLOR_DOT_GRID.2, COLOR_DOT_GRID.3);
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
) {
    cr.new_path();
    cr.arc(x + w - r, y + r, r, -std::f64::consts::FRAC_PI_2, 0.0);
    cr.arc(x + w - r, y + h - r, r, 0.0, std::f64::consts::FRAC_PI_2);
    cr.arc(x + r, y + h - r, r, std::f64::consts::FRAC_PI_2, std::f64::consts::PI);
    cr.arc(x + r, y + r, r, std::f64::consts::PI, 3.0 * std::f64::consts::FRAC_PI_2);
    cr.close_path();

    // Node fill
    if is_dimmed {
        cr.set_source_rgba(tokens::color::BG_VOID.0, tokens::color::BG_VOID.1, tokens::color::BG_VOID.2, 0.5);
    } else if is_fault {
        cr.set_source_rgba(COLOR_FAULT_FILL.0, COLOR_FAULT_FILL.1, COLOR_FAULT_FILL.2, COLOR_FAULT_FILL.3);
    } else if is_selected {
        cr.set_source_rgba(COLOR_NODE_FILL_SELECTED.0, COLOR_NODE_FILL_SELECTED.1, COLOR_NODE_FILL_SELECTED.2, COLOR_NODE_FILL_SELECTED.3);
    } else if is_hovered {
        cr.set_source_rgba(COLOR_NODE_FILL_HOVER.0, COLOR_NODE_FILL_HOVER.1, COLOR_NODE_FILL_HOVER.2, COLOR_NODE_FILL_HOVER.3);
    } else if is_initial {
        cr.set_source_rgba(COLOR_NODE_FILL_INITIAL.0, COLOR_NODE_FILL_INITIAL.1, COLOR_NODE_FILL_INITIAL.2, COLOR_NODE_FILL_INITIAL.3);
    } else {
        cr.set_source_rgba(COLOR_NODE_FILL_DEFAULT.0, COLOR_NODE_FILL_DEFAULT.1, COLOR_NODE_FILL_DEFAULT.2, COLOR_NODE_FILL_DEFAULT.3);
    }
    let _ = cr.fill_preserve();

    // Node stroke (Strict 1px hairline)
    if is_dimmed {
        cr.set_source_rgba(tokens::color::BORDER_HAIR.0, tokens::color::BORDER_HAIR.1, tokens::color::BORDER_HAIR.2, 0.35);
    } else if is_fault {
        cr.set_source_rgb(COLOR_FAULT_RED.0, COLOR_FAULT_RED.1, COLOR_FAULT_RED.2);
    } else if is_selected {
        cr.set_source_rgb(COLOR_BORDER_SELECTED.0, COLOR_BORDER_SELECTED.1, COLOR_BORDER_SELECTED.2);
    } else if is_hovered {
        cr.set_source_rgb(COLOR_BORDER_HOVER.0, COLOR_BORDER_HOVER.1, COLOR_BORDER_HOVER.2);
    } else if is_initial {
        cr.set_source_rgb(COLOR_BORDER_INITIAL.0, COLOR_BORDER_INITIAL.1, COLOR_BORDER_INITIAL.2);
    } else {
        cr.set_source_rgb(COLOR_BORDER_DEFAULT.0, COLOR_BORDER_DEFAULT.1, COLOR_BORDER_DEFAULT.2);
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
) {
    cr.select_font_face(tokens::font::CAIRO_MONO, cairo::FontSlant::Normal, cairo::FontWeight::Bold);
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
    cr.arc(x + r, y + h - r, r, std::f64::consts::FRAC_PI_2, std::f64::consts::PI);
    cr.arc(x + r, y + r, r, std::f64::consts::PI, 3.0 * std::f64::consts::FRAC_PI_2);
    cr.close_path();

    if is_fault {
        if is_dimmed {
            cr.set_source_rgba(COLOR_FAULT_FILL.0, COLOR_FAULT_FILL.1, COLOR_FAULT_FILL.2, 0.3);
            let _ = cr.fill_preserve();
            cr.set_source_rgba(COLOR_FAULT_RED.0, COLOR_FAULT_RED.1, COLOR_FAULT_RED.2, 0.25);
        } else {
            cr.set_source_rgba(COLOR_FAULT_FILL.0, COLOR_FAULT_FILL.1, COLOR_FAULT_FILL.2, 0.95);
            let _ = cr.fill_preserve();
            cr.set_source_rgba(COLOR_FAULT_RED.0, COLOR_FAULT_RED.1, COLOR_FAULT_RED.2, 0.85);
        }
    } else {
        if is_dimmed {
            cr.set_source_rgba(tokens::color::BG_PANEL.0, tokens::color::BG_PANEL.1, tokens::color::BG_PANEL.2, 0.3);
            let _ = cr.fill_preserve();
            cr.set_source_rgba(tokens::color::BORDER_HAIR.0, tokens::color::BORDER_HAIR.1, tokens::color::BORDER_HAIR.2, 0.25);
        } else {
            cr.set_source_rgba(tokens::color::BG_PANEL.0, tokens::color::BG_PANEL.1, tokens::color::BG_PANEL.2, 0.95);
            let _ = cr.fill_preserve();
            cr.set_source_rgba(tokens::color::BORDER_HAIR.0, tokens::color::BORDER_HAIR.1, tokens::color::BORDER_HAIR.2, 0.85);
        }
    }
    cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
    let _ = cr.stroke();

    if is_fault {
        if is_dimmed {
            cr.set_source_rgba(COLOR_FAULT_RED.0, COLOR_FAULT_RED.1, COLOR_FAULT_RED.2, 0.3);
        } else {
            cr.set_source_rgb(COLOR_FAULT_RED.0, COLOR_FAULT_RED.1, COLOR_FAULT_RED.2);
        }
    } else {
        if is_dimmed {
            cr.set_source_rgba(tokens::color::TEXT_MUTED.0, tokens::color::TEXT_MUTED.1, tokens::color::TEXT_MUTED.2, 0.3);
        } else {
            cr.set_source_rgb(tokens::color::TEXT_PRIMARY.0, tokens::color::TEXT_PRIMARY.1, tokens::color::TEXT_PRIMARY.2);
        }
    }
    let _ = cr.move_to(x + pad_x - ext_xb, y + pad_y - ext_yb);
    let _ = cr.show_text(text);
}

fn draw_guard_badge(
    cr: &cairo::Context,
    center_x: f64,
    center_y: f64,
    text: &str,
    is_fault: bool,
    is_dimmed: bool,
) {
    cr.select_font_face(tokens::font::CAIRO_MONO, cairo::FontSlant::Normal, cairo::FontWeight::Normal);
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
    cr.arc(badge_x + badge_w - r, badge_y + r, r, -std::f64::consts::FRAC_PI_2, 0.0);
    cr.arc(badge_x + badge_w - r, badge_y + badge_h - r, r, 0.0, std::f64::consts::FRAC_PI_2);
    cr.arc(badge_x + r, badge_y + badge_h - r, r, std::f64::consts::FRAC_PI_2, std::f64::consts::PI);
    cr.arc(badge_x + r, badge_y + r, r, std::f64::consts::PI, 3.0 * std::f64::consts::FRAC_PI_2);
    cr.close_path();

    // Badge fill
    if is_dimmed {
        cr.set_source_rgba(tokens::color::BG_PANEL.0, tokens::color::BG_PANEL.1, tokens::color::BG_PANEL.2, 0.4);
    } else {
        cr.set_source_rgba(tokens::color::BG_PANEL.0, tokens::color::BG_PANEL.1, tokens::color::BG_PANEL.2, 0.95);
    }
    let _ = cr.fill_preserve();

    // Badge stroke (1px hairline)
    if is_fault {
        if is_dimmed {
            cr.set_source_rgba(COLOR_FAULT_RED.0, COLOR_FAULT_RED.1, COLOR_FAULT_RED.2, 0.3);
        } else {
            cr.set_source_rgba(COLOR_FAULT_RED.0, COLOR_FAULT_RED.1, COLOR_FAULT_RED.2, 0.85);
        }
    } else if is_dimmed {
        cr.set_source_rgba(tokens::color::BORDER_HAIR.0, tokens::color::BORDER_HAIR.1, tokens::color::BORDER_HAIR.2, 0.3);
    } else {
        cr.set_source_rgba(tokens::color::BORDER_HAIR.0, tokens::color::BORDER_HAIR.1, tokens::color::BORDER_HAIR.2, 0.85);
    }
    cr.set_line_width(tokens::shape::BORDER_WIDTH_HAIR);
    let _ = cr.stroke();

    // Badge text
    if is_fault {
        if is_dimmed {
            cr.set_source_rgba(COLOR_FAULT_RED.0, COLOR_FAULT_RED.1, COLOR_FAULT_RED.2, 0.3);
        } else {
            cr.set_source_rgb(COLOR_FAULT_RED.0, COLOR_FAULT_RED.1, COLOR_FAULT_RED.2);
        }
    } else if is_dimmed {
        cr.set_source_rgba(tokens::color::TEXT_MUTED.0, tokens::color::TEXT_MUTED.1, tokens::color::TEXT_MUTED.2, 0.3);
    } else {
        cr.set_source_rgb(tokens::color::TEXT_PRIMARY.0, tokens::color::TEXT_PRIMARY.1, tokens::color::TEXT_PRIMARY.2);
    }

    let _ = cr.move_to(badge_x + padding_x - ext_xb, badge_y + padding_y - ext_yb);
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
        assert!(state.borrow().state_diagram_layout.is_some());
        let layout = state.borrow().state_diagram_layout.clone().unwrap();
        assert_eq!(layout.nodes.len(), 14);
        assert_eq!(layout.edges.len(), 31);
        assert!(!layout.lanes.is_empty());
        assert!(layout.lanes.iter().any(|l| l.name == "CALIBRATION"));
        assert!(layout.lanes.iter().any(|l| l.name == "FAULT"));

        // 2. Select GOING node (expand outgoing edges including to FAULT)
        state.borrow_mut().selected_state_node = Some("GOING".to_string());
        draw_state_diagram(&cr, 1200.0, 800.0, &state);
        surface.flush();

        // 3. Select FAULT node (expand all 12 fault triggers)
        state.borrow_mut().selected_state_node = Some("FAULT".to_string());
        draw_state_diagram(&cr, 1200.0, 800.0, &state);
        surface.flush();

        // 4. Hover CALIBRATING node while FAULT is selected
        state.borrow_mut().hovered_state_node = Some("CALIBRATING".to_string());
        draw_state_diagram(&cr, 1200.0, 800.0, &state);
        surface.flush();

        // 5. Deselect (click background -> return to low clutter collapsed view)
        state.borrow_mut().selected_state_node = None;
        state.borrow_mut().hovered_state_node = None;
        draw_state_diagram(&cr, 1200.0, 800.0, &state);
        surface.flush();
    }
}
