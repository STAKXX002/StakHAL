use std::cell::RefCell;
use std::rc::Rc;
use gtk4::prelude::*;
use stakhal_core::graph::{compute_graph_bounds, compute_graph_layout};
use crate::state::{AppState, AppWidgets};
use super::draw::draw_call_graph_canvas;

pub fn setup_call_graph_drawing_and_gestures(
    state: &Rc<RefCell<AppState>>,
    widgets: &Rc<AppWidgets>,
) {
    let state_draw = Rc::clone(state);
    widgets.graph_drawing_area.set_draw_func(move |area, cr, width, height| {
        draw_call_graph_canvas(area, cr, width as f64, height as f64, &state_draw);
    });

    let gesture_drag = gtk4::GestureDrag::new();
    gesture_drag.set_button(1);
    let state_drag_begin = Rc::clone(state);

    gesture_drag.connect_drag_begin(move |_, start_x, start_y| {
        let mut st = state_drag_begin.borrow_mut();
        st.drag_start_click_pos = (start_x, start_y);
        st.dragged_graph_node = None;

        let zoom = st.graph_zoom;
        let unscaled_x = start_x / zoom;
        let unscaled_y = start_y / zoom;

        let mut clicked_id = None;
        let mut start_pos = (0.0, 0.0);

        for (id, &(nx, ny)) in &st.graph_node_positions {
            let nw = (id.len() as f64 * 8.5 + 28.0).max(110.0);
            let nh = 34.0;
            if unscaled_x >= nx && unscaled_x <= nx + nw && unscaled_y >= ny && unscaled_y <= ny + nh {
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
            let zoom = st.graph_zoom;
            let new_x = (snx + offset_x / zoom).max(0.0);
            let new_y = (sny + offset_y / zoom).max(0.0);
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
            let zoom = st.graph_zoom;
            let cx = st.drag_start_click_pos.0 / zoom;
            let cy = st.drag_start_click_pos.1 / zoom;

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
                    let (w, h) = compute_graph_bounds(&pos, &headers);
                    st.graph_node_positions = pos;
                    st.chain_headers = headers;
                    st.graph_bounds = (w, h);
                    let zoomed_w = (w as f64 * zoom).ceil() as i32;
                    let zoomed_h = (h as f64 * zoom).ceil() as i32;
                    area_drag_end.set_content_width(zoomed_w);
                    area_drag_end.set_content_height(zoomed_h);
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

        let zoom = st.graph_zoom;
        let (w, h) = compute_graph_bounds(&st.graph_node_positions, &st.chain_headers);
        st.graph_bounds = (w, h);
        let zoomed_w = (w as f64 * zoom).ceil() as i32;
        let zoomed_h = (h as f64 * zoom).ceil() as i32;
        area_drag_end.set_content_width(zoomed_w);
        area_drag_end.set_content_height(zoomed_h);

        st.dragged_graph_node = None;
        drop(st);
        area_drag_end.queue_draw();
    });

    widgets.graph_drawing_area.add_controller(gesture_drag);

    // Zoom controller (Ctrl + Scroll / Trackpad)
    let scroll_controller = gtk4::EventControllerScroll::new(
        gtk4::EventControllerScrollFlags::VERTICAL
            | gtk4::EventControllerScrollFlags::HORIZONTAL
            | gtk4::EventControllerScrollFlags::KINETIC,
    );
    let state_scroll = Rc::clone(state);
    let area_scroll = widgets.graph_drawing_area.clone();

    scroll_controller.set_propagation_phase(gtk4::PropagationPhase::Capture);
    scroll_controller.connect_scroll(move |controller, _dx, dy| {
        let modifiers = controller.current_event_state();
        let is_ctrl = modifiers.contains(gtk4::gdk::ModifierType::CONTROL_MASK);

        if is_ctrl && dy != 0.0 {
            let zoom_delta = if dy < 0.0 { 0.08 } else { -0.08 };
            let mut st = state_scroll.borrow_mut();
            let new_zoom = (st.graph_zoom + zoom_delta).clamp(0.25, 2.5);
            st.graph_zoom = new_zoom;
            let (w, h) = st.graph_bounds;
            let zoomed_w = (w as f64 * new_zoom).ceil() as i32;
            let zoomed_h = (h as f64 * new_zoom).ceil() as i32;
            drop(st);
            area_scroll.set_content_width(zoomed_w);
            area_scroll.set_content_height(zoomed_h);
            area_scroll.queue_draw();
            gtk4::glib::Propagation::Stop
        } else {
            gtk4::glib::Propagation::Proceed
        }
    });

    widgets.graph_drawing_area.add_controller(scroll_controller);

    // Touchpad Pinch-to-Zoom Controller
    let gesture_zoom = gtk4::GestureZoom::new();
    let state_zoom = Rc::clone(state);
    let area_zoom = widgets.graph_drawing_area.clone();
    let initial_zoom = Rc::new(std::cell::Cell::new(1.0f64));

    let initial_zoom_begin = Rc::clone(&initial_zoom);
    let state_zoom_begin = Rc::clone(state);
    gesture_zoom.connect_begin(move |_, _| {
        initial_zoom_begin.set(state_zoom_begin.borrow().graph_zoom);
    });

    gesture_zoom.connect_scale_changed(move |_, scale| {
        let mut st = state_zoom.borrow_mut();
        let new_zoom = (initial_zoom.get() * scale).clamp(0.25, 2.5);
        st.graph_zoom = new_zoom;
        let (w, h) = st.graph_bounds;
        let zoomed_w = (w as f64 * new_zoom).ceil() as i32;
        let zoomed_h = (h as f64 * new_zoom).ceil() as i32;
        drop(st);
        area_zoom.set_content_width(zoomed_w);
        area_zoom.set_content_height(zoomed_h);
        area_zoom.queue_draw();
    });

    widgets.graph_drawing_area.add_controller(gesture_zoom);
}



