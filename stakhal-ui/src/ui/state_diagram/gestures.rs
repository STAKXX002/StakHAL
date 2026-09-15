use std::cell::RefCell;
use std::rc::Rc;
use gtk4::gdk;
use gtk4::prelude::*;
use stakhal_core::graph::state_transition::compute_project_state_layout;
use crate::state::{AppState, AppWidgets};

const NODE_W: f64 = 140.0;
const NODE_H: f64 = 48.0;

pub fn setup_state_diagram_drawing_and_gestures(
    state: &Rc<RefCell<AppState>>,
    widgets: &Rc<AppWidgets>,
) {
    let state_draw = Rc::clone(state);
    widgets
        .diagram_drawing_area
        .set_draw_func(move |area, cr, width, height| {
            super::draw::draw_state_diagram_canvas(area, cr, width as f64, height as f64, &state_draw);
        });

    // Click Gesture
    let click_gesture = gtk4::GestureClick::new();
    let state_click = Rc::clone(state);
    let widgets_click = Rc::clone(widgets);

    click_gesture.connect_pressed(move |_g, _n_press, x, y| {
        let mut st = state_click.borrow_mut();
        let zoom = st.diagram_zoom;
        let pan_x = st.diagram_pan_x;
        let pan_y = st.diagram_pan_y;

        let canvas_x = (x - pan_x) / zoom;
        let canvas_y = (y - pan_y) / zoom;

        let mut clicked_node = None;
        for (id, &(nx, ny)) in &st.state_node_positions {
            if canvas_x >= nx && canvas_x <= nx + NODE_W && canvas_y >= ny && canvas_y <= ny + NODE_H {
                clicked_node = Some(id.clone());
                break;
            }
        }

        if let Some(ref node_id) = clicked_node {
            st.selected_state_node = Some(node_id.clone());
            // Look up description
            let mut desc_opt = None;
            if let Some(ref model) = st.state_model {
                for m in &model.machines {
                    for s in &m.states {
                        if s.id == *node_id {
                            desc_opt = Some((s.peripheral.clone(), s.name.clone(), s.description.clone()));
                            break;
                        }
                    }
                }
            }
            drop(st);

            if let Some((periph, name, desc)) = desc_opt {
                widgets_click
                    .lbl_selected_info
                    .set_text(&format!("SELECTED: [{}] State '{}' — {}", periph, name, desc));
            }
        } else {
            st.selected_state_node = None;
            drop(st);
            widgets_click
                .lbl_selected_info
                .set_text("Select a state node to view transition triggers and lifecycle details.");
        }

        widgets_click.diagram_drawing_area.queue_draw();
    });
    widgets.diagram_drawing_area.add_controller(click_gesture);

    // Drag Gesture (Node Dragging or Canvas Panning)
    let drag_gesture = gtk4::GestureDrag::new();
    let state_drag = Rc::clone(state);
    let _widgets_drag = Rc::clone(widgets);

    drag_gesture.connect_drag_begin(move |_, start_x, start_y| {
        let mut st = state_drag.borrow_mut();
        let zoom = st.diagram_zoom;
        let pan_x = st.diagram_pan_x;
        let pan_y = st.diagram_pan_y;

        let canvas_x = (start_x - pan_x) / zoom;
        let canvas_y = (start_y - pan_y) / zoom;

        let mut hit_node = None;
        for (id, &(nx, ny)) in &st.state_node_positions {
            if canvas_x >= nx && canvas_x <= nx + NODE_W && canvas_y >= ny && canvas_y <= ny + NODE_H {
                hit_node = Some((id.clone(), (nx, ny)));
                break;
            }
        }

        if let Some((node_id, pos)) = hit_node {
            st.dragged_state_node = Some(node_id);
            st.drag_start_node_pos = pos;
        } else {
            st.dragged_state_node = None;
            st.drag_start_pan_pos = (pan_x, pan_y);
        }
        st.drag_start_click_pos = (start_x, start_y);
    });

    let state_drag_update = Rc::clone(state);
    let widgets_drag_update = Rc::clone(widgets);

    drag_gesture.connect_drag_update(move |_, offset_x, offset_y| {
        let mut st = state_drag_update.borrow_mut();
        if let Some(ref node_id) = st.dragged_state_node.clone() {
            let zoom = st.diagram_zoom;
            let (orig_x, orig_y) = st.drag_start_node_pos;
            let new_x = orig_x + offset_x / zoom;
            let new_y = orig_y + offset_y / zoom;
            st.state_node_positions.insert(node_id.clone(), (new_x, new_y));
        } else {
            let (orig_pan_x, orig_pan_y) = st.drag_start_pan_pos;
            st.diagram_pan_x = orig_pan_x + offset_x;
            st.diagram_pan_y = orig_pan_y + offset_y;
        }
        drop(st);
        widgets_drag_update.diagram_drawing_area.queue_draw();
    });

    let state_drag_end = Rc::clone(state);
    drag_gesture.connect_drag_end(move |_, _, _| {
        let mut st = state_drag_end.borrow_mut();
        st.dragged_state_node = None;
    });

    widgets.diagram_drawing_area.add_controller(drag_gesture);

    // Motion Controller (Hover Detection)
    let motion = gtk4::EventControllerMotion::new();
    let state_motion = Rc::clone(state);
    let widgets_motion = Rc::clone(widgets);

    motion.connect_motion(move |_, x, y| {
        let mut st = state_motion.borrow_mut();
        let zoom = st.diagram_zoom;
        let pan_x = st.diagram_pan_x;
        let pan_y = st.diagram_pan_y;

        let canvas_x = (x - pan_x) / zoom;
        let canvas_y = (y - pan_y) / zoom;

        let mut hit = None;
        for (id, &(nx, ny)) in &st.state_node_positions {
            if canvas_x >= nx && canvas_x <= nx + NODE_W && canvas_y >= ny && canvas_y <= ny + NODE_H {
                hit = Some(id.clone());
                break;
            }
        }

        if st.hovered_state_node != hit {
            st.hovered_state_node = hit;
            drop(st);
            widgets_motion.diagram_drawing_area.queue_draw();
        }
    });
    widgets.diagram_drawing_area.add_controller(motion);

    // Scroll Controller (Zoom)
    let scroll = gtk4::EventControllerScroll::new(gtk4::EventControllerScrollFlags::VERTICAL);
    let state_scroll = Rc::clone(state);
    let widgets_scroll = Rc::clone(widgets);

    scroll.connect_scroll(move |_, _, dy| {
        let mut st = state_scroll.borrow_mut();
        let old_zoom = st.diagram_zoom;
        let factor = if dy < 0.0 { 1.15 } else { 0.85 };
        let new_zoom = (old_zoom * factor).clamp(0.25, 3.0);

        st.diagram_zoom = new_zoom;
        drop(st);

        widgets_scroll.diagram_drawing_area.queue_draw();
        gdk::glib::Propagation::Stop
    });
    widgets.diagram_drawing_area.add_controller(scroll);

    // Peripheral Combo Box filter
    let state_combo = Rc::clone(state);
    let widgets_combo = Rc::clone(widgets);

    widgets.combo_peripheral.connect_selected_notify(move |combo| {
        let selected_idx = combo.selected();
        let item = combo
            .model()
            .and_then(|m| m.item(selected_idx))
            .and_then(|obj| obj.downcast::<gtk4::StringObject>().ok())
            .map(|s| s.string().to_string());

        let periph_filter = match item.as_deref() {
            Some("ALL PERIPHERALS") | None => None,
            Some(name) => Some(name.to_string()),
        };

        let mut st = state_combo.borrow_mut();
        st.selected_peripheral = periph_filter.clone();
        if let Some(ref model) = st.state_model {
            let (pos, bounds) = compute_project_state_layout(model, periph_filter.as_deref());
            st.state_node_positions = pos;
            st.diagram_bounds = bounds;
            st.diagram_pan_x = 0.0;
            st.diagram_pan_y = 0.0;
            st.diagram_zoom = 1.0;

            widgets_combo
                .diagram_drawing_area
                .set_content_width(bounds.0);
            widgets_combo
                .diagram_drawing_area
                .set_content_height(bounds.1);
        }
        drop(st);
        widgets_combo.diagram_drawing_area.queue_draw();
    });

    // Fit to View Button
    let state_fit = Rc::clone(state);
    let widgets_fit = Rc::clone(widgets);

    widgets.btn_fit_to_view.connect_clicked(move |_| {
        let mut st = state_fit.borrow_mut();
        if st.state_node_positions.is_empty() {
            return;
        }

        let (bw, bh) = st.diagram_bounds;
        let vw = widgets_fit
            .diagram_scrolled
            .hadjustment()
            .page_size()
            .max(widgets_fit.diagram_scrolled.width() as f64);
        let vh = widgets_fit
            .diagram_scrolled
            .vadjustment()
            .page_size()
            .max(widgets_fit.diagram_scrolled.height() as f64);

        if bw > 0 && bh > 0 && vw > 0.0 && vh > 0.0 {
            let fit_zoom = (vw / bw as f64).min(vh / bh as f64).clamp(0.3, 2.0);
            if !fit_zoom.is_nan() && !fit_zoom.is_infinite() {
                st.diagram_zoom = fit_zoom;
                st.diagram_pan_x = 0.0;
                st.diagram_pan_y = 0.0;

                let zoomed_w = (bw as f64 * fit_zoom).ceil() as i32;
                let zoomed_h = (bh as f64 * fit_zoom).ceil() as i32;
                drop(st);

                widgets_fit.diagram_drawing_area.set_content_width(zoomed_w);
                widgets_fit.diagram_drawing_area.set_content_height(zoomed_h);

                widgets_fit.diagram_scrolled.hadjustment().set_value(0.0);
                widgets_fit.diagram_scrolled.vadjustment().set_value(0.0);

                widgets_fit.diagram_drawing_area.queue_draw();
            }
        }
    });
}
