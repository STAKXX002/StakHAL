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
        st.drag_start_pan_pos = (st.graph_pan_x, st.graph_pan_y);
        st.dragged_graph_node = None;

        let zoom = st.graph_zoom;
        let pan_x = st.graph_pan_x;
        let pan_y = st.graph_pan_y;

        let world_x = (start_x - pan_x) / zoom;
        let world_y = (start_y - pan_y) / zoom;

        let mut clicked_id = None;
        let mut start_pos = (0.0, 0.0);

        for (id, &(nx, ny)) in &st.graph_node_positions {
            let nw = (id.len() as f64 * 8.5 + 28.0).max(110.0);
            let nh = 34.0;
            if world_x >= nx && world_x <= nx + nw && world_y >= ny && world_y <= ny + nh {
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
        } else {
            // Dragging background pans canvas
            let (spx, spy) = st.drag_start_pan_pos;
            st.graph_pan_x = spx + offset_x;
            st.graph_pan_y = spy + offset_y;
        }
        drop(st);
        area_drag_update.queue_draw();
    });

    let state_drag_end = Rc::clone(state);
    let area_drag_end = widgets.graph_drawing_area.clone();
    gesture_drag.connect_drag_end(move |_, offset_x, offset_y| {
        let dist = offset_x.hypot(offset_y);
        let mut st = state_drag_end.borrow_mut();

        if dist < 5.0 {
            let zoom = st.graph_zoom;
            let pan_x = st.graph_pan_x;
            let pan_y = st.graph_pan_y;

            let world_x = (st.drag_start_click_pos.0 - pan_x) / zoom;
            let world_y = (st.drag_start_click_pos.1 - pan_y) / zoom;

            // Check if clicked inside a chain header bar
            let mut clicked_header_id = None;
            for h in &st.chain_headers {
                if world_x >= h.x && world_x <= h.x + h.w && world_y >= h.y && world_y <= h.y + h.h {
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
                if world_x >= nx && world_x <= nx + nw && world_y >= ny && world_y <= ny + nh {
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

    // Zoom controller (Ctrl + Scroll / Trackpad) - Anchored to Cursor
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

            let old_zoom = st.graph_zoom;
            let old_pan_x = st.graph_pan_x;
            let old_pan_y = st.graph_pan_y;

            let (cx, cy) = st.last_mouse_pos;
            let world_x = (cx - old_pan_x) / old_zoom;
            let world_y = (cy - old_pan_y) / old_zoom;

            let new_zoom = (old_zoom + zoom_delta).clamp(0.25, 2.5);
            let new_pan_x = cx - world_x * new_zoom;
            let new_pan_y = cy - world_y * new_zoom;

            st.graph_zoom = new_zoom;
            st.graph_pan_x = new_pan_x;
            st.graph_pan_y = new_pan_y;

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

    // Touchpad Pinch-to-Zoom Controller - Anchored to Cursor
    let gesture_zoom = gtk4::GestureZoom::new();
    let state_zoom = Rc::clone(state);
    let area_zoom = widgets.graph_drawing_area.clone();
    let initial_zoom = Rc::new(std::cell::Cell::new(1.0f64));
    let initial_pan = Rc::new(std::cell::Cell::new((0.0f64, 0.0f64)));

    let initial_zoom_begin = Rc::clone(&initial_zoom);
    let initial_pan_begin = Rc::clone(&initial_pan);
    let state_zoom_begin = Rc::clone(state);
    gesture_zoom.connect_begin(move |_, _| {
        let st = state_zoom_begin.borrow();
        initial_zoom_begin.set(st.graph_zoom);
        initial_pan_begin.set((st.graph_pan_x, st.graph_pan_y));
    });

    gesture_zoom.connect_scale_changed(move |_, scale| {
        let mut st = state_zoom.borrow_mut();
        let base_zoom = initial_zoom.get();
        let (base_pan_x, base_pan_y) = initial_pan.get();

        let (cx, cy) = st.last_mouse_pos;
        let world_x = (cx - base_pan_x) / base_zoom;
        let world_y = (cy - base_pan_y) / base_zoom;

        let new_zoom = (base_zoom * scale).clamp(0.25, 2.5);
        let new_pan_x = cx - world_x * new_zoom;
        let new_pan_y = cy - world_y * new_zoom;

        st.graph_zoom = new_zoom;
        st.graph_pan_x = new_pan_x;
        st.graph_pan_y = new_pan_y;

        let (w, h) = st.graph_bounds;
        let zoomed_w = (w as f64 * new_zoom).ceil() as i32;
        let zoomed_h = (h as f64 * new_zoom).ceil() as i32;
        drop(st);
        area_zoom.set_content_width(zoomed_w);
        area_zoom.set_content_height(zoomed_h);
        area_zoom.queue_draw();
    });

    widgets.graph_drawing_area.add_controller(gesture_zoom);

    // Motion Controller (Hover & Cursor tracking)
    let motion_controller = gtk4::EventControllerMotion::new();
    let state_motion = Rc::clone(state);
    let area_motion = widgets.graph_drawing_area.clone();

    motion_controller.connect_motion(move |_, x, y| {
        let mut st = state_motion.borrow_mut();
        st.last_mouse_pos = (x, y);

        let zoom = st.graph_zoom;
        let pan_x = st.graph_pan_x;
        let pan_y = st.graph_pan_y;

        let world_x = (x - pan_x) / zoom;
        let world_y = (y - pan_y) / zoom;

        let mut newly_hovered = None;
        for (id, &(nx, ny)) in &st.graph_node_positions {
            let nw = (id.len() as f64 * 8.5 + 28.0).max(110.0);
            let nh = 34.0;
            if world_x >= nx && world_x <= nx + nw && world_y >= ny && world_y <= ny + nh {
                newly_hovered = Some(id.clone());
                break;
            }
        }

        if st.hovered_graph_node != newly_hovered {
            st.hovered_graph_node = newly_hovered;
            drop(st);
            area_motion.queue_draw();
        }
    });

    let state_leave = Rc::clone(state);
    let area_leave = widgets.graph_drawing_area.clone();
    motion_controller.connect_leave(move |_| {
        let mut st = state_leave.borrow_mut();
        if st.hovered_graph_node.is_some() {
            st.hovered_graph_node = None;
            drop(st);
            area_leave.queue_draw();
        }
    });

    widgets.graph_drawing_area.add_controller(motion_controller);
}





