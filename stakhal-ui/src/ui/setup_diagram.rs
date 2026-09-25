use std::cell::RefCell;
use std::rc::Rc;
use gtk4::prelude::*;

use crate::state::{AppState, AppWidgets};
use crate::ui::nucleo_pinout::setup_nucleo_pinout_drawing_and_gestures;
use crate::ui::state_diagram::setup_state_diagram_drawing_and_gestures;

pub fn setup_diagram_and_navigation(
    state: &Rc<RefCell<AppState>>,
    widgets: &Rc<AppWidgets>,
    btn_diagram_back: &gtk4::Button,
    btn_pinout_back: &gtk4::Button,
    btn_serial_back: &gtk4::Button,
) {
    setup_state_diagram_drawing_and_gestures(
        &widgets.diagram_drawing_area,
        &widgets.btn_fit_to_view,
        &widgets.lbl_selected_info,
        Rc::clone(state),
    );
    setup_nucleo_pinout_drawing_and_gestures(state, widgets);

    // Navigation callbacks
    let stack_back2 = widgets.stack.clone();
    btn_diagram_back.connect_clicked(move |_| {
        crate::navigate_stack(&stack_back2, "overview", gtk4::StackTransitionType::SlideRight);
    });

    let stack_back3 = widgets.stack.clone();
    btn_pinout_back.connect_clicked(move |_| {
        crate::navigate_stack(&stack_back3, "overview", gtk4::StackTransitionType::SlideRight);
    });

    let stack_back_serial = widgets.stack.clone();
    btn_serial_back.connect_clicked(move |_| {
        crate::navigate_stack(&stack_back_serial, "overview", gtk4::StackTransitionType::SlideRight);
    });

    let stack_diagram = widgets.stack.clone();
    let state_diagram_nav = Rc::clone(state);
    let area_diagram_nav = widgets.diagram_drawing_area.clone();
    widgets.btn_call_graph.connect_clicked(move |_| {
        state_diagram_nav.borrow().with_canvas_state_mut(|st| {
            let sel = st.selected_state_machine;
            if crate::ui::tokens::motion::is_animations_enabled() && !st.session_revealed_machines.contains(&sel) {
                st.session_revealed_machines.insert(sel);
                st.edge_reveal_animation = Some((sel, std::time::Instant::now()));
            }
        });
        area_diagram_nav.queue_draw();
        crate::navigate_stack(&stack_diagram, "state_diagram", gtk4::StackTransitionType::SlideLeft);
    });

    let stack_pinout = widgets.stack.clone();
    widgets.btn_nucleo_pinout.connect_clicked(move |_| {
        crate::navigate_stack(&stack_pinout, "nucleo_pinout", gtk4::StackTransitionType::SlideLeft);
    });

    let stack_serial = widgets.stack.clone();
    let state_serial = Rc::clone(state);
    let widgets_serial = Rc::clone(widgets);
    widgets.btn_serial_monitor.connect_clicked(move |_| {
        crate::ui::setup_serial::refresh_serial_ports(&state_serial, &widgets_serial);
        crate::navigate_stack(&stack_serial, "serial_monitor", gtk4::StackTransitionType::SlideLeft);
    });

    // State machine selector dropdown callback
    let state_combo = Rc::clone(state);
    let area_combo = widgets.diagram_drawing_area.clone();
    let lbl_info_combo = widgets.lbl_selected_info.clone();
    widgets.combo_state_machine.connect_selected_notify(move |cb| {
        let idx = cb.selected() as usize;
        state_combo.borrow().with_canvas_state_mut(|st| {
            st.selected_state_machine = idx;
            st.selected_state_node = None;
            st.state_diagram_layout = None; // trigger layout recompute for selected machine
            st.diagram_needs_fit = true;

            if crate::ui::tokens::motion::is_animations_enabled() && !st.session_revealed_machines.contains(&idx) {
                st.session_revealed_machines.insert(idx);
                st.edge_reveal_animation = Some((idx, std::time::Instant::now()));
            } else {
                st.edge_reveal_animation = None;
            }
        });
        {
            let proj = Rc::clone(&state_combo.borrow().project);
            let proj_guard = proj.borrow();
            if let Some(ref p) = proj_guard.loaded_project {
                if idx < p.state_machines.len() {
                    let sm = &p.state_machines[idx];
                    if !sm.ambiguous_transitions.is_empty() {
                        let notes: Vec<_> = sm
                            .ambiguous_transitions
                            .iter()
                            .map(|a| format!("(any state) -> {} [{}]", a.target, a.guard))
                            .collect();
                        lbl_info_combo.set_text(&format!(
                            "NOTE: {} Ambiguous Transition(s): {}",
                            sm.ambiguous_transitions.len(),
                            notes.join(", ")
                        ));
                    } else {
                        lbl_info_combo.set_text("Select a state node to inspect transitions and guard triggers.");
                    }
                }
            }
        }
        area_combo.queue_draw();
    });

    // Nucleo Pinout module filter dropdown callback
    let state_mod_combo = Rc::clone(state);
    let area_pinout_combo = widgets.pinout_drawing_area.clone();
    widgets.combo_pinout_module.connect_selected_notify(move |cb| {
        let idx = cb.selected();
        let sel_str = cb.model()
            .and_then(|m| m.downcast::<gtk4::StringList>().ok())
            .and_then(|sl| sl.string(idx))
            .map(|s| s.to_string());

        let mod_filter = match sel_str.as_deref() {
            Some("All Modules") | None => None,
            Some(s) => Some(s.to_string()),
        };

        state_mod_combo.borrow().with_canvas_state_mut(|c| {
            c.selected_pinout_module = mod_filter;
        });
        area_pinout_combo.queue_draw();
    });
}
