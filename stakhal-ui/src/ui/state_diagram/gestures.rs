use std::cell::RefCell;
use std::rc::Rc;
use gtk4::glib;
use gtk4::prelude::*;
use crate::state::AppState;
use super::draw::draw_state_diagram_canvas;

pub fn setup_state_diagram_drawing_and_gestures(
    drawing_area: &gtk4::DrawingArea,
    btn_fit_to_view: &gtk4::Button,
    lbl_selected_info: &gtk4::Label,
    state: Rc<RefCell<AppState>>,
) {
    // 1. Drawing Area Function
    let state_draw = Rc::clone(&state);
    drawing_area.set_draw_func(move |area, cr, w, h| {
        draw_state_diagram_canvas(area, cr, w as f64, h as f64, &state_draw);
    });

    // 2. Click Gesture for Node Selection
    let click_gesture = gtk4::GestureClick::new();
    click_gesture.set_button(1); // Left click

    let state_click = Rc::clone(&state);
    let area_click = drawing_area.clone();
    let info_click = lbl_selected_info.clone();

    click_gesture.connect_pressed(move |_, _, x, y| {
        let mut st = state_click.borrow_mut();
        let zoom = st.diagram_zoom;
        let pan_x = st.diagram_pan_x;
        let pan_y = st.diagram_pan_y;

        let gx = (x - pan_x) / zoom;
        let gy = (y - pan_y) / zoom;

        let mut hit_node = None;
        if let Some(ref layout) = st.state_diagram_layout {
            for (id, node) in &layout.nodes {
                if gx >= node.x && gx <= node.x + node.width && gy >= node.y && gy <= node.y + node.height {
                    hit_node = Some(id.clone());
                    break;
                }
            }
        }

        // Toggle selection off if clicking the already selected node, or collapse if clicking background
        if hit_node.is_none() || st.selected_state_node == hit_node {
            st.selected_state_node = None;
        } else {
            st.selected_state_node = hit_node.clone();
        }

        let current_sel = st.selected_state_node.clone();

        // Update selected info label
        if let Some(ref sel) = current_sel {
            if let Some(ref p) = st.loaded_project {
                if !p.state_machines.is_empty() {
                    let sm = &p.state_machines[st.selected_state_machine];
                    let outgoing: Vec<_> = sm.transitions.iter().filter(|t| &t.from == sel).collect();
                    let incoming: Vec<_> = sm.transitions.iter().filter(|t| &t.to == sel).collect();

                    let mut info = format!("STATE: {} | Incoming: {} | Outgoing: {}", sel, incoming.len(), outgoing.len());
                    if !outgoing.is_empty() {
                        let out_summary: Vec<_> = outgoing.iter().map(|t| format!("-> {} [{}]", t.to, t.display_guard(25))).collect();
                        info.push_str(&format!(" | Exits: {}", out_summary.join(", ")));
                    }
                    info_click.set_text(&info);
                }
            }
        } else {
            info_click.set_text("Click a node to inspect full transition paths • Click background to collapse high-fan-in edges");
        }

        area_click.queue_draw();
    });

    drawing_area.add_controller(click_gesture);

    // 3. Motion Controller for Hover
    let motion_controller = gtk4::EventControllerMotion::new();
    let state_motion = Rc::clone(&state);
    let area_motion = drawing_area.clone();

    motion_controller.connect_motion(move |_, x, y| {
        let mut st = state_motion.borrow_mut();
        let zoom = st.diagram_zoom;
        let pan_x = st.diagram_pan_x;
        let pan_y = st.diagram_pan_y;

        let gx = (x - pan_x) / zoom;
        let gy = (y - pan_y) / zoom;

        let mut hovered = None;
        if let Some(ref layout) = st.state_diagram_layout {
            for (id, node) in &layout.nodes {
                if gx >= node.x && gx <= node.x + node.width && gy >= node.y && gy <= node.y + node.height {
                    hovered = Some(id.clone());
                    break;
                }
            }
        }

        if st.hovered_state_node != hovered {
            st.hovered_state_node = hovered;
            area_motion.queue_draw();
        }
    });

    drawing_area.add_controller(motion_controller);

    // 4. Drag Controller for Panning
    let drag_gesture = gtk4::GestureDrag::new();
    drag_gesture.set_button(1);

    let state_drag_begin = Rc::clone(&state);
    drag_gesture.connect_drag_begin(move |_, x, y| {
        let mut st = state_drag_begin.borrow_mut();
        st.drag_start_click_pos = (x, y);
        st.drag_start_pan_pos = (st.diagram_pan_x, st.diagram_pan_y);
    });

    let state_drag_update = Rc::clone(&state);
    let area_drag = drawing_area.clone();
    drag_gesture.connect_drag_update(move |_, offset_x, offset_y| {
        let mut st = state_drag_update.borrow_mut();
        st.diagram_pan_x = st.drag_start_pan_pos.0 + offset_x;
        st.diagram_pan_y = st.drag_start_pan_pos.1 + offset_y;
        area_drag.queue_draw();
    });

    drawing_area.add_controller(drag_gesture);

    // 5. Scroll Controller for Zooming
    let scroll_controller = gtk4::EventControllerScroll::new(
        gtk4::EventControllerScrollFlags::VERTICAL,
    );

    let state_scroll = Rc::clone(&state);
    let area_scroll = drawing_area.clone();
    scroll_controller.connect_scroll(move |_, _, dy| {
        let mut st = state_scroll.borrow_mut();
        let factor = if dy < 0.0 { 1.15 } else { 1.0 / 1.15 };
        let new_zoom = (st.diagram_zoom * factor).clamp(0.2, 3.5);
        st.diagram_zoom = new_zoom;
        area_scroll.queue_draw();
        glib::Propagation::Stop
    });

    drawing_area.add_controller(scroll_controller);

    // 6. Fit to View Button
    let state_fit = Rc::clone(&state);
    let area_fit = drawing_area.clone();
    btn_fit_to_view.connect_clicked(move |_| {
        let mut st = state_fit.borrow_mut();
        st.diagram_needs_fit = true;
        area_fit.queue_draw();
    });
}
