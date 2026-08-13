use std::cell::RefCell;
use std::rc::Rc;
use gtk4::prelude::*;
use stakhal_core::graph::compute_graph_layout;
use crate::state::{AppState, AppWidgets};
use super::draw::draw_call_graph_canvas;

pub fn setup_call_graph_drawing_and_gestures(
    state: &Rc<RefCell<AppState>>,
    widgets: &Rc<AppWidgets>,
) {
    let state_draw = Rc::clone(state);
    widgets.graph_drawing_area.set_draw_func(move |_area, cr, width, height| {
        draw_call_graph_canvas(cr, width as f64, height as f64, &state_draw);
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
