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
        let (current_sel, selected_sm_idx) = state_click.borrow().with_canvas_state_mut(|st| {
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

            (st.selected_state_node.clone(), st.selected_state_machine)
        });

        // Update selected info label
        if let Some(ref sel) = current_sel {
            let proj = Rc::clone(&state_click.borrow().project);
            let proj_guard = proj.borrow();
            if let Some(ref p) = proj_guard.loaded_project {
                if let Some(sm) = p.state_machines.get(selected_sm_idx) {
                    let outgoing: Vec<_> = sm.transitions.iter().filter(|t| &t.from == sel).collect();
                    let incoming: Vec<_> = sm.transitions.iter().filter(|t| &t.to == sel).collect();

                    let mut info = format!("STATE: {} | Incoming: {} | Outgoing: {}", sel, incoming.len(), outgoing.len());
                    if !outgoing.is_empty() {
                        let out_summary: Vec<_> = outgoing
                            .iter()
                            .map(|t| {
                                if !t.label.is_empty() && !t.guard.is_empty() && t.label != t.guard {
                                    format!("-> {} [{} (raw: {})]", t.to, t.label, t.guard)
                                } else {
                                    format!("-> {} [{}]", t.to, t.display_guard(25))
                                }
                            })
                            .collect();
                        info.push_str(&format!(" | Exits: {}", out_summary.join(", ")));
                    }
                    info_click.set_text(&info);
                }
            }
        } else {
            info_click.set_text("Click a node to inspect full transition paths | Click background to collapse high-fan-in edges");
        }

        area_click.queue_draw();
    });

    drawing_area.add_controller(click_gesture);

    // 3. Motion Controller for Hover & Cursor Position Tracking
    let motion_controller = gtk4::EventControllerMotion::new();
    let state_motion = Rc::clone(&state);
    let area_motion = drawing_area.clone();

    motion_controller.connect_motion(move |_, x, y| {
        let mut needs_draw = false;
        state_motion.borrow().with_canvas_state_mut(|st| {
            st.diagram_mouse_pos = Some((x, y));
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
                needs_draw = true;
            }
        });

        if needs_draw {
            area_motion.queue_draw();
        }
    });

    let state_enter = Rc::clone(&state);
    motion_controller.connect_enter(move |_, x, y| {
        state_enter.borrow().with_canvas_state_mut(|st| {
            st.diagram_mouse_pos = Some((x, y));
        });
    });

    let state_leave = Rc::clone(&state);
    motion_controller.connect_leave(move |_| {
        state_leave.borrow().with_canvas_state_mut(|st| {
            st.diagram_mouse_pos = None;
        });
    });

    drawing_area.add_controller(motion_controller);

    // 4. Drag Controller for Panning
    let drag_gesture = gtk4::GestureDrag::new();
    drag_gesture.set_button(1);

    let state_drag_begin = Rc::clone(&state);
    drag_gesture.connect_drag_begin(move |_, x, y| {
        state_drag_begin.borrow().with_canvas_state_mut(|st| {
            st.drag_start_click_pos = (x, y);
            st.drag_start_pan_pos = (st.diagram_pan_x, st.diagram_pan_y);
        });
    });

    let state_drag_update = Rc::clone(&state);
    let area_drag = drawing_area.clone();
    drag_gesture.connect_drag_update(move |_, offset_x, offset_y| {
        if offset_x.abs() > 2.0 || offset_y.abs() > 2.0 {
            state_drag_update.borrow().with_canvas_state_mut(|st| {
                st.diagram_pan_x = st.drag_start_pan_pos.0 + offset_x;
                st.diagram_pan_y = st.drag_start_pan_pos.1 + offset_y;
            });
            area_drag.queue_draw();
        }
    });

    drag_gesture.connect_drag_end(move |_, _, _| {});

    drawing_area.add_controller(drag_gesture);

    // 5. Scroll Controller for Zooming Anchored to Mouse Cursor
    let scroll_controller = gtk4::EventControllerScroll::new(
        gtk4::EventControllerScrollFlags::VERTICAL,
    );

    let state_scroll = Rc::clone(&state);
    let area_scroll = drawing_area.clone();
    scroll_controller.connect_scroll(move |controller, _, dy| {
        state_scroll.borrow().with_canvas_state_mut(|st| {
            // 1. Get mouse position in canvas/widget coordinates
            let (cursor_x, cursor_y) = st
                .diagram_mouse_pos
                .or_else(|| controller.current_event().and_then(|e| e.position()))
                .unwrap_or_else(|| {
                    (area_scroll.width().max(800) as f64 * 0.5, area_scroll.height().max(600) as f64 * 0.5)
                });

            // 2. Apply new zoom scale and pan offset anchored to cursor
            let factor = if dy < 0.0 { 1.15 } else { 1.0 / 1.15 };
            let (new_zoom, new_pan_x, new_pan_y) = calculate_zoom_at_cursor(
                st.diagram_zoom,
                st.diagram_pan_x,
                st.diagram_pan_y,
                cursor_x,
                cursor_y,
                factor,
                0.2,
                3.5,
            );

            st.diagram_zoom = new_zoom;
            st.diagram_pan_x = new_pan_x;
            st.diagram_pan_y = new_pan_y;
        });

        area_scroll.queue_draw();
        glib::Propagation::Stop
    });

    drawing_area.add_controller(scroll_controller);

    // 6. Fit to View Button
    let state_fit = Rc::clone(&state);
    let area_fit = drawing_area.clone();
    btn_fit_to_view.connect_clicked(move |_| {
        state_fit.borrow().with_canvas_state_mut(|st| {
            st.diagram_needs_fit = true;
        });
        area_fit.queue_draw();
    });
}

/// Compute new zoom level and pan offsets such that the world coordinate under the cursor
/// remains invariant before and after the zoom operation.
pub fn calculate_zoom_at_cursor(
    old_zoom: f64,
    old_pan_x: f64,
    old_pan_y: f64,
    cursor_x: f64,
    cursor_y: f64,
    factor: f64,
    min_zoom: f64,
    max_zoom: f64,
) -> (f64, f64, f64) {
    // 1. Convert cursor screen position to canvas/world coordinates BEFORE zoom
    let world_x = (cursor_x - old_pan_x) / old_zoom;
    let world_y = (cursor_y - old_pan_y) / old_zoom;

    // 2. Apply new zoom scale clamped to limits
    let new_zoom = (old_zoom * factor).clamp(min_zoom, max_zoom);

    // 3. Recompute pan offsets so the same world point still lands under the cursor
    let new_pan_x = cursor_x - world_x * new_zoom;
    let new_pan_y = cursor_y - world_y * new_zoom;

    (new_zoom, new_pan_x, new_pan_y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zoom_in_anchored_to_cursor() {
        // Cursor away from top-left corner, e.g. at (600, 300)
        let cursor_x = 600.0;
        let cursor_y = 300.0;
        let old_zoom = 1.0;
        let old_pan_x = 40.0;
        let old_pan_y = 40.0;

        // World coordinates under cursor before zoom
        let world_x = (cursor_x - old_pan_x) / old_zoom; // 560.0
        let world_y = (cursor_y - old_pan_y) / old_zoom; // 260.0

        let factor = 1.15;
        let (new_zoom, new_pan_x, new_pan_y) = calculate_zoom_at_cursor(
            old_zoom, old_pan_x, old_pan_y, cursor_x, cursor_y, factor, 0.2, 3.5,
        );

        assert!((new_zoom - 1.15).abs() < 1e-9);

        // Screen position of world point after zoom
        let screen_x_after = world_x * new_zoom + new_pan_x;
        let screen_y_after = world_y * new_zoom + new_pan_y;

        assert!(
            (screen_x_after - cursor_x).abs() < 1e-9,
            "X drift detected: expected {}, got {}",
            cursor_x,
            screen_x_after
        );
        assert!(
            (screen_y_after - cursor_y).abs() < 1e-9,
            "Y drift detected: expected {}, got {}",
            cursor_y,
            screen_y_after
        );
    }

    #[test]
    fn test_repeated_zoom_drift_prevention() {
        // Point in middle-right of graph
        let cursor_x = 850.0;
        let cursor_y = 420.0;
        let mut zoom = 1.0;
        let mut pan_x = 50.0;
        let mut pan_y = 60.0;

        // Target world coordinate
        let target_world_x = (cursor_x - pan_x) / zoom;
        let target_world_y = (cursor_y - pan_y) / zoom;

        // Repeatedly zoom in 8 times
        for step in 1..=8 {
            let (next_zoom, next_pan_x, next_pan_y) = calculate_zoom_at_cursor(
                zoom, pan_x, pan_y, cursor_x, cursor_y, 1.15, 0.2, 5.0,
            );
            zoom = next_zoom;
            pan_x = next_pan_x;
            pan_y = next_pan_y;

            let rendered_x = target_world_x * zoom + pan_x;
            let rendered_y = target_world_y * zoom + pan_y;

            assert!(
                (rendered_x - cursor_x).abs() < 1e-9,
                "Step {}: point drifted in X from {} to {}",
                step,
                cursor_x,
                rendered_x
            );
            assert!(
                (rendered_y - cursor_y).abs() < 1e-9,
                "Step {}: point drifted in Y from {} to {}",
                step,
                cursor_y,
                rendered_y
            );
        }

        // Repeatedly zoom back out 8 times
        for step in 1..=8 {
            let (next_zoom, next_pan_x, next_pan_y) = calculate_zoom_at_cursor(
                zoom, pan_x, pan_y, cursor_x, cursor_y, 1.0 / 1.15, 0.2, 5.0,
            );
            zoom = next_zoom;
            pan_x = next_pan_x;
            pan_y = next_pan_y;

            let rendered_x = target_world_x * zoom + pan_x;
            let rendered_y = target_world_y * zoom + pan_y;

            assert!(
                (rendered_x - cursor_x).abs() < 1e-9,
                "Zoom-out step {}: point drifted in X from {} to {}",
                step,
                cursor_x,
                rendered_x
            );
            assert!(
                (rendered_y - cursor_y).abs() < 1e-9,
                "Zoom-out step {}: point drifted in Y from {} to {}",
                step,
                cursor_y,
                rendered_y
            );
        }
    }

    #[test]
    fn test_zoom_clamping_preserves_pan() {
        let cursor_x = 500.0;
        let cursor_y = 350.0;
        let max_zoom = 3.5;
        let pan_x = 100.0;
        let pan_y = 100.0;

        // Already at max zoom, zooming in further
        let (clamped_zoom, new_pan_x, new_pan_y) = calculate_zoom_at_cursor(
            max_zoom, pan_x, pan_y, cursor_x, cursor_y, 1.25, 0.2, max_zoom,
        );

        assert_eq!(clamped_zoom, max_zoom);
        assert!((new_pan_x - pan_x).abs() < 1e-9);
        assert!((new_pan_y - pan_y).abs() < 1e-9);
    }
}
