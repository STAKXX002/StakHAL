use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use gtk4::{gio, prelude::*};
use libadwaita as adw;

use stakhal_core::ioc::discovery::discover_project_files;
use stakhal_core::ir::schema::load_project;

use crate::append_log_text;
use crate::config::{load_app_config, save_app_config};
use crate::state::{AppState, AppWidgets};
use crate::toolchain;
use crate::ui::main_panel::{clear_list_box, create_peripheral_row, create_region_row};
use crate::update_quick_send_buttons;
use crate::update_traceability_ui;

pub fn try_discover_folder(dir: &Path, state: &Rc<RefCell<AppState>>, widgets: &Rc<AppWidgets>) {
    let build_sys = toolchain::builder::detect_build_system(dir);
    let has_build_sys = build_sys.is_some();
    let has_makefile = dir.join("Makefile").is_file() || dir.join("makefile").is_file();
    let discovery_res = discover_project_files(dir);

    {
        let st = state.borrow();
        {
            let mut proj = st.project.borrow_mut();
            proj.project_dir = Some(dir.to_path_buf());
            match &discovery_res {
                Ok((ioc_path, main_c_path)) => {
                    proj.discovered_ioc = Some(ioc_path.clone());
                    proj.discovered_main_c = Some(main_c_path.clone());
                }
                Err(_) => {
                    proj.discovered_ioc = None;
                    proj.discovered_main_c = None;
                }
            }
        }
        {
            let mut bt = st.build_trace.borrow_mut();
            bt.has_makefile = has_makefile;
            bt.has_build_system = has_build_sys;
            bt.detected_build_system = build_sys;
        }
    }

    widgets.lbl_discovered_dir.set_text(&dir.display().to_string());
    widgets.btn_build.set_sensitive(has_build_sys);
    widgets.btn_build_flash.set_sensitive(has_build_sys);

    match discovery_res {
        Ok((ioc_path, main_c_path)) => {
            widgets
                .lbl_ioc_path
                .set_text(&format!("IOC: {}", ioc_path.display()));
            widgets
                .lbl_main_c_path
                .set_text(&format!("Main C: {}", main_c_path.display()));
            widgets.btn_load.set_sensitive(true);
        }
        Err(err) => {
            widgets.toast_overlay.add_toast(adw::Toast::new(&format!("Discovery Error: {}", err)));
            widgets.lbl_ioc_path.set_text("IOC Path: N/A");
            widgets.lbl_main_c_path.set_text("Main C Path: N/A");
            widgets.btn_load.set_sensitive(false);
        }
    }

    update_traceability_ui(state, widgets);
}

pub fn do_load_project(state: &Rc<RefCell<AppState>>, widgets: &Rc<AppWidgets>) {
    let (ioc_path, main_c_path, dir_path) = {
        let st = state.borrow();
        let proj = st.project.borrow();

        match (&proj.discovered_ioc, &proj.discovered_main_c, &proj.project_dir) {
            (Some(i), Some(m), Some(d)) => (i.clone(), m.clone(), d.clone()),
            _ => {
                widgets.toast_overlay.add_toast(adw::Toast::new("[ERROR] Project files not selected"));
                return;
            }
        }
    };

    save_app_config(&dir_path.display().to_string());

    let build_sys = toolchain::builder::detect_build_system(&dir_path);
    let has_build_sys = build_sys.is_some();
    {
        let st = state.borrow();
        let mut bt = st.build_trace.borrow_mut();
        bt.has_makefile = dir_path.join("Makefile").is_file() || dir_path.join("makefile").is_file();
        bt.has_build_system = has_build_sys;
        bt.detected_build_system = build_sys;
    }
    widgets.btn_build.set_sensitive(has_build_sys);
    widgets.btn_build_flash.set_sensitive(has_build_sys);

    match load_project(&ioc_path, &main_c_path) {
        Ok(project) => {
            widgets.lbl_project_name.set_text(&format!("NAME: {}", project.meta.name));
            widgets.lbl_mcu_family.set_text(&format!("FAMILY: {}", project.meta.mcu_family));
            widgets.lbl_mcu_name.set_text(&format!("MCU: {}", project.meta.mcu_name));

            widgets.lbl_periph_header.set_text(&format!("[ PERIPHERALS ({}) ]", project.peripherals.len()));

            let mut total_regions = project.user_regions.len();
            if project.loop_body.is_some() {
                total_regions += 1;
            }
            widgets.lbl_region_header.set_text(&format!("[ USER REGIONS ({}) ]", total_regions));

            clear_list_box(&widgets.list_peripherals);
            clear_list_box(&widgets.list_user_regions);

            for p in &project.peripherals {
                let row = create_peripheral_row(&p.name, p.mode.as_deref(), p.parameters.len());
                widgets.list_peripherals.append(&row);
            }

            for r in &project.user_regions {
                let row = create_region_row(
                    &r.tag,
                    r.byte_range.0,
                    r.byte_range.1,
                    r.line_range.0,
                    r.line_range.1,
                    false,
                );
                widgets.list_user_regions.append(&row);
            }

            if let Some(ref lb) = project.loop_body {
                let row = create_region_row(
                    &lb.tag,
                    lb.byte_range.0,
                    lb.byte_range.1,
                    lb.line_range.0,
                    lb.line_range.1,
                    true,
                );
                widgets.list_user_regions.append(&row);
            }

            let is_f446 = project.meta.mcu_name.to_uppercase().contains("F446");
            let project_modules = project.modules.clone();

            // Setup state machines
            let sm_names: Vec<String> = if project.state_machines.is_empty() {
                vec!["NO STATE MACHINES DETECTED".to_string()]
            } else {
                project
                    .state_machines
                    .iter()
                    .map(|sm| sm.display_name.clone())
                    .collect()
            };
            let sm_str_refs: Vec<&str> = sm_names.iter().map(|s| s.as_str()).collect();
            let string_list = gtk4::StringList::new(&sm_str_refs);
            widgets.combo_state_machine.set_model(Some(&string_list));
            widgets.combo_state_machine.set_selected(0);

            if !project.state_machines.is_empty() {
                let sm = project.state_machines[0].clone();
                let layout = stakhal_core::graph::compute_state_machine_layout(&sm);
                let w = layout.width as i32;
                let h = layout.height as i32;

                {
                    state.borrow().with_canvas_state_mut(|st| {
                        st.selected_state_machine = 0;
                        st.selected_state_node = None;
                        st.diagram_bounds = (w, h);
                        let mut pos = std::collections::HashMap::new();
                        for (id, n) in &layout.nodes {
                            pos.insert(id.clone(), (n.x, n.y));
                        }
                        st.state_node_positions = pos;
                        st.state_diagram_layout = Some(layout);
                        st.diagram_needs_fit = true;
                    });
                    state.borrow().project.borrow_mut().loaded_project = Some(project);
                }

                widgets.diagram_drawing_area.set_content_width(w);
                widgets.diagram_drawing_area.set_content_height(h);
                widgets.btn_call_graph.set_sensitive(true);

                if !sm.ambiguous_transitions.is_empty() {
                    let notes: Vec<_> = sm
                        .ambiguous_transitions
                        .iter()
                        .map(|a| format!("(any state) -> {} [{}]", a.target, a.guard))
                        .collect();
                    widgets.lbl_selected_info.set_text(&format!(
                        "NOTE: {} Ambiguous Transition(s): {}",
                        sm.ambiguous_transitions.len(),
                        notes.join(", ")
                    ));
                } else {
                    widgets
                        .lbl_selected_info
                        .set_text("Select a state node to inspect transitions and guard triggers.");
                }
                widgets.diagram_drawing_area.queue_draw();
            } else {
                {
                    state.borrow().with_canvas_state_mut(|st| {
                        st.state_diagram_layout = None;
                    });
                    state.borrow().project.borrow_mut().loaded_project = Some(project);
                }
                widgets.btn_call_graph.set_sensitive(false);
                widgets
                    .lbl_selected_info
                    .set_text("No application state machines detected in current project.");
            }

            if is_f446 {
                widgets.btn_nucleo_pinout.set_sensitive(true);
                widgets
                    .btn_nucleo_pinout
                    .set_tooltip_text(Some("View Nucleo-F446RE Physical Connector Pinout"));

                let mut mod_names = vec!["All Modules".to_string()];
                if project_modules.len() > 1 {
                    mod_names.extend(project_modules.clone());
                }
                let mod_str_refs: Vec<&str> = mod_names.iter().map(|s| s.as_str()).collect();
                let mod_string_list = gtk4::StringList::new(&mod_str_refs);
                widgets.combo_pinout_module.set_model(Some(&mod_string_list));
                widgets.combo_pinout_module.set_selected(0);
                widgets.combo_pinout_module.set_sensitive(project_modules.len() > 1);

                state.borrow().with_canvas_state_mut(|c| {
                    c.selected_pinout_module = None;
                });
            } else {
                widgets.btn_nucleo_pinout.set_sensitive(false);
                widgets
                    .btn_nucleo_pinout
                    .set_tooltip_text(Some("Nucleo Pinout visualizer is F446RE-only for now"));
                let mod_string_list = gtk4::StringList::new(&["All Modules"]);
                widgets.combo_pinout_module.set_model(Some(&mod_string_list));
                widgets.combo_pinout_module.set_selected(0);
                widgets.combo_pinout_module.set_sensitive(false);
                state.borrow().with_canvas_state_mut(|c| {
                    c.selected_pinout_module = None;
                });
            }
            widgets.pinout_drawing_area.queue_draw();

            // Auto-detect console UART and baud rate
            if let Some(uart_info) = stakhal_core::source::detect_console_uart(&main_c_path) {
                append_log_text(
                    &widgets.build_log_view,
                    &format!(
                        "[SERIAL] Auto-detected console UART: {} @ {} baud",
                        uart_info.uart_instance, uart_info.baud_rate
                    ),
                );
                if let Some(pos) = crate::toolchain::serial::COMMON_BAUD_RATES
                    .iter()
                    .position(|&b| b == uart_info.baud_rate)
                {
                    widgets.combo_baud.set_selected(pos as u32);
                }
                {
                    let st = state.borrow();
                    let mut ser = st.serial.borrow_mut();
                    ser.selected_serial_baud = uart_info.baud_rate;
                    ser.detected_console_uart = Some(uart_info);
                }
            }
            update_quick_send_buttons(&main_c_path, state, widgets);

            widgets.toast_overlay.add_toast(adw::Toast::new("[OK] Project loaded successfully"));
        }

        Err(err) => {
            widgets.btn_nucleo_pinout.set_sensitive(false);
            widgets.btn_nucleo_pinout.set_tooltip_text(Some("Nucleo Pinout visualizer is F446RE-only for now"));
            let mod_string_list = gtk4::StringList::new(&["All Modules"]);
            widgets.combo_pinout_module.set_model(Some(&mod_string_list));
            widgets.combo_pinout_module.set_selected(0);
            widgets.combo_pinout_module.set_sensitive(false);
            state.borrow().with_canvas_state_mut(|c| {
                c.selected_pinout_module = None;
            });
            widgets.toast_overlay.add_toast(adw::Toast::new(&format!("[ERROR] Load Error: {}", err)));
        }
    }

    update_traceability_ui(state, widgets);
}

pub fn setup_project_handlers(
    btn_browse: &gtk4::Button,
    state: &Rc<RefCell<AppState>>,
    widgets: &Rc<AppWidgets>,
) {
    let state_browse = Rc::clone(state);
    let widgets_browse = Rc::clone(widgets);
    btn_browse.connect_clicked(move |_| {
        let dialog = gtk4::FileDialog::builder()
            .title("Select STM32 Project Directory")
            .build();

        let state_dialog = Rc::clone(&state_browse);
        let widgets_dialog = Rc::clone(&widgets_browse);

        dialog.select_folder(
            Some(&widgets_browse.window),
            gio::Cancellable::NONE,
            move |res| {
                if let Ok(folder) = res {
                    if let Some(path) = folder.path() {
                        try_discover_folder(&path, &state_dialog, &widgets_dialog);
                    }
                }
            },
        );
    });

    let state_load = Rc::clone(state);
    let widgets_load = Rc::clone(widgets);
    widgets.btn_load.connect_clicked(move |_| {
        do_load_project(&state_load, &widgets_load);
    });
}

pub fn check_last_project(state: &Rc<RefCell<AppState>>, widgets: &Rc<AppWidgets>) {
    let config = load_app_config();
    if let Some(dir_str) = config.project_dir {
        let path = PathBuf::from(dir_str);
        if path.exists() {
            try_discover_folder(&path, state, widgets);
        }
    }
}
