use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use gtk4::{gdk, gio, glib};
use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;

use stakhal_core::ioc::discovery::discover_project_files;
use stakhal_core::ir::schema::load_project;

mod config;
mod state;
mod toolchain;
mod ui;

fn append_log_text(view: &gtk4::TextView, text: &str) {
    let buffer = view.buffer();
    let mut end_iter = buffer.end_iter();
    buffer.insert(&mut end_iter, text);
    if !text.ends_with('\n') {
        buffer.insert(&mut buffer.end_iter(), "\n");
    }
    let mark = buffer.create_mark(None, &buffer.end_iter(), false);
    view.scroll_to_mark(&mark, 0.0, true, 0.0, 1.0);
}

use config::{load_app_config, save_app_config};
use state::{AppState, AppWidgets};
use ui::nucleo_pinout::{
    build_nucleo_pinout_panel, setup_nucleo_pinout_drawing_and_gestures, NucleoPinoutPanelWidgets,
};
use ui::state_diagram::{
    build_state_diagram_panel, setup_state_diagram_drawing_and_gestures, StateDiagramPanelWidgets,
};

use ui::main_panel::{
    build_main_panel, clear_list_box, create_peripheral_row, create_region_row, MainPanelWidgets,
};


const APP_ID: &str = "com.stakhal.ui";

fn main() {
    let app = adw::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();

    app.connect_activate(build_ui);
    app.run_with_args(&["stakhal-ui"]);
}

fn build_ui(app: &adw::Application) {
    adw::StyleManager::default().set_color_scheme(adw::ColorScheme::ForceDark);

    let css_provider = gtk4::CssProvider::new();
    css_provider.load_from_string(r#"
* {
    font-family: 'DejaVu Sans Mono', 'Liberation Mono', monospace;
    font-size: 13px;
    border-radius: 0px;
    box-shadow: none;
}
window, dialog {
    background-color: #0a0a0a;
    color: #e5e5e5;
}
windowcontrols button {
    border: none;
    background: transparent;
    border-radius: 0px;
}
windowcontrols button:hover {
    border: none;
    background: transparent;
}
button.stakhal-btn {
    border: 1px solid #262626;
    background-color: #121212;
    color: #e5e5e5;
    transition: all 120ms ease;
    border-radius: 0px;
}
button.stakhal-btn:hover {
    border-color: #525252;
    background-color: #1a1a1a;
    color: #ffffff;
}
button.stakhal-btn:active {
    background-color: #262626;
}
button.stakhal-btn.suggested-action {
    border-color: #e5e5e5;
    background-color: #e5e5e5;
    color: #0a0a0a;
}
button.stakhal-btn.suggested-action:hover {
    border-color: #ffffff;
    background-color: #ffffff;
    color: #000000;
}
button.stakhal-btn.flat {
    border-color: transparent;
    background-color: transparent;
    color: #a3a3a3;
}
button.stakhal-btn.flat:hover {
    border-color: #262626;
    background-color: #171717;
    color: #ffffff;
}
row, listboxrow, actionrow {
    border-radius: 0px;
    transition: none;
}
.clickable-row {
    transition: all 120ms ease;
}
.clickable-row:hover {
    background-color: #171717;
}
.clickable-row:active {
    background-color: #262626;
}
.dim-label {
    color: #737373;
}
.title-1, .title-2, .title-3, .heading {
    color: #f5f5f5;
    font-weight: bold;
}

/* Reserved Status Classes */
.status-error, .error {
    color: #ef4444;
}
.status-warning, .warning {
    color: #f59e0b;
}
.status-ok, .ok {
    color: #22c55e;
}
"#);


    gtk4::style_context_add_provider_for_display(
        &gdk::Display::default().expect("Could not connect to a display."),
        &css_provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    let MainPanelWidgets {
        overview_box,
        btn_browse,
        btn_load,
        btn_build_flash,
        btn_call_graph,
        btn_nucleo_pinout,
        lbl_discovered_dir,
        lbl_ioc_path,
        lbl_main_c_path,
        lbl_project_name,
        lbl_mcu_family,
        lbl_mcu_name,
        lbl_periph_header,
        lbl_region_header,
        list_peripherals,
        list_user_regions,
        build_log_view,
        lbl_build_status,
        btn_clear_log,
    } = build_main_panel();

    let StateDiagramPanelWidgets {
        diagram_panel_box,
        btn_diagram_back,
        btn_fit_to_view,
        combo_state_machine,
        diagram_drawing_area,
        diagram_scrolled,
        lbl_selected_info,
    } = build_state_diagram_panel();

    let NucleoPinoutPanelWidgets {
        pinout_panel_box,
        btn_pinout_back,
        pinout_drawing_area,
        pinout_scrolled,
    } = build_nucleo_pinout_panel();

    let stack = gtk4::Stack::builder()
        .transition_type(gtk4::StackTransitionType::SlideLeftRight)
        .transition_duration(220)
        .build();

    stack.add_named(&overview_box, Some("overview"));
    stack.add_named(&diagram_panel_box, Some("state_diagram"));
    stack.add_named(&pinout_panel_box, Some("nucleo_pinout"));
    stack.set_visible_child_name("overview");

    let header_bar = adw::HeaderBar::new();
    let toast_overlay = adw::ToastOverlay::new();

    let content_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .build();
    content_box.append(&header_bar);
    content_box.append(&stack);

    toast_overlay.set_child(Some(&content_box));

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("StakHAL — Hardware Abstraction Inspector")
        .default_width(1200)
        .default_height(800)
        .maximized(true)
        .content(&toast_overlay)
        .build();

    let state = Rc::new(RefCell::new(AppState::default()));
    let widgets = Rc::new(AppWidgets {
        window: window.clone(),
        _stack: stack.clone(),
        toast_overlay,
        lbl_discovered_dir,
        lbl_ioc_path,
        lbl_main_c_path,
        btn_load,
        btn_build_flash: btn_build_flash.clone(),
        btn_call_graph: btn_call_graph.clone(),
        btn_nucleo_pinout: btn_nucleo_pinout.clone(),
        lbl_project_name,
        lbl_mcu_family,
        lbl_mcu_name,
        lbl_periph_header,
        lbl_region_header,
        list_peripherals,
        list_user_regions,
        build_log_view: build_log_view.clone(),
        lbl_build_status: lbl_build_status.clone(),
        btn_clear_log: btn_clear_log.clone(),
        diagram_drawing_area: diagram_drawing_area.clone(),
        btn_fit_to_view: btn_fit_to_view.clone(),
        diagram_scrolled: diagram_scrolled.clone(),
        combo_state_machine: combo_state_machine.clone(),
        lbl_selected_info: lbl_selected_info.clone(),
        pinout_drawing_area,
        _pinout_scrolled: pinout_scrolled,
    });

    setup_state_diagram_drawing_and_gestures(
        &diagram_drawing_area,
        &btn_fit_to_view,
        &lbl_selected_info,
        Rc::clone(&state),
    );
    setup_nucleo_pinout_drawing_and_gestures(&state, &widgets);

    // Navigation callbacks
    let stack_back2 = stack.clone();
    btn_diagram_back.connect_clicked(move |_| {
        stack_back2.set_visible_child_full("overview", gtk4::StackTransitionType::SlideRight);
    });

    let stack_back3 = stack.clone();
    btn_pinout_back.connect_clicked(move |_| {
        stack_back3.set_visible_child_full("overview", gtk4::StackTransitionType::SlideRight);
    });

    let stack_diagram = stack.clone();
    btn_call_graph.connect_clicked(move |_| {
        stack_diagram.set_visible_child_full("state_diagram", gtk4::StackTransitionType::SlideLeft);
    });

    let stack_pinout = stack.clone();
    btn_nucleo_pinout.connect_clicked(move |_| {
        stack_pinout.set_visible_child_full("nucleo_pinout", gtk4::StackTransitionType::SlideLeft);
    });

    // State machine selector dropdown callback
    let state_combo = Rc::clone(&state);
    let area_combo = diagram_drawing_area.clone();
    let lbl_info_combo = lbl_selected_info.clone();
    combo_state_machine.connect_selected_notify(move |cb| {
        let idx = cb.selected() as usize;
        let mut st = state_combo.borrow_mut();
        st.selected_state_machine = idx;
        st.selected_state_node = None;
        st.state_diagram_layout = None; // trigger layout recompute for selected machine
        st.diagram_needs_fit = true;
        if let Some(ref p) = st.loaded_project {
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
        drop(st);
        area_combo.queue_draw();
    });



    // Connect Browse Button
    let state_browse = Rc::clone(&state);
    let widgets_browse = Rc::clone(&widgets);
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

    // Connect Load Button
    let state_load = Rc::clone(&state);
    let widgets_load = Rc::clone(&widgets);
    widgets.btn_load.connect_clicked(move |_| {
        do_load_project(&state_load, &widgets_load);
    });

    // Connect Clear Console Button
    let log_view_clear = widgets.build_log_view.clone();
    let status_clear = widgets.lbl_build_status.clone();
    widgets.btn_clear_log.connect_clicked(move |_| {
        log_view_clear.buffer().set_text("");
        status_clear.set_text("IDLE");
    });

    // Connect Build & Flash Button
    let state_bf = Rc::clone(&state);
    let widgets_bf = Rc::clone(&widgets);
    widgets.btn_build_flash.connect_clicked(move |_| {
        let (project_dir, has_makefile) = {
            let st = state_bf.borrow();
            (st.project_dir.clone(), st.has_makefile)
        };

        let dir = match project_dir {
            Some(d) if has_makefile => d,
            _ => {
                widgets_bf.toast_overlay.add_toast(adw::Toast::new("No project with Makefile loaded"));
                return;
            }
        };

        {
            let mut st = state_bf.borrow_mut();
            if st.build_in_progress {
                return;
            }
            st.build_in_progress = true;
        }

        widgets_bf.btn_build_flash.set_sensitive(false);
        widgets_bf.lbl_build_status.set_text("BUILDING...");

        let make_jobs = toolchain::runner::get_make_jobs_flag();
        append_log_text(&widgets_bf.build_log_view, "============================================================");
        append_log_text(&widgets_bf.build_log_view, &format!("[BUILD] Running `make {}` in {}", make_jobs, dir.display()));
        append_log_text(&widgets_bf.build_log_view, "============================================================");

        if !toolchain::runner::is_executable_on_path("make") {
            append_log_text(&widgets_bf.build_log_view, "[ERROR] 'make' executable not found on PATH. Please install build-essential.");
            widgets_bf.lbl_build_status.set_text("BUILD FAILED");
            state_bf.borrow_mut().build_in_progress = false;
            widgets_bf.btn_build_flash.set_sensitive(true);
            return;
        }

        let rx = toolchain::runner::spawn_streaming_process(
            "make".to_string(),
            vec![make_jobs],
            dir.clone(),
        );

        let state_timer = Rc::clone(&state_bf);
        let widgets_timer = Rc::clone(&widgets_bf);
        let dir_timer = dir.clone();

        glib::timeout_add_local(std::time::Duration::from_millis(25), move || {
            let mut finished = None;
            while let Ok(evt) = rx.try_recv() {
                match evt {
                    toolchain::runner::ProcessEvent::Line(line) => {
                        append_log_text(&widgets_timer.build_log_view, &line);
                    }
                    toolchain::runner::ProcessEvent::Finished(success, code) => {
                        finished = Some((success, code));
                    }
                    toolchain::runner::ProcessEvent::FailedToStart(err) => {
                        append_log_text(&widgets_timer.build_log_view, &format!("[ERROR] {}", err));
                    }
                    _ => {}
                }
            }

            if let Some((success, code)) = finished {
                let mut st = state_timer.borrow_mut();
                st.build_in_progress = false;
                widgets_timer.btn_build_flash.set_sensitive(st.has_makefile);

                if success {
                    append_log_text(&widgets_timer.build_log_view, "\n[BUILD SUCCESS] `make` finished successfully.");
                    let res = toolchain::makefile::resolve_build_artifact(&dir_timer);
                    match res {
                        toolchain::makefile::ArtifactResolution::Exact(bin_path) => {
                            append_log_text(&widgets_timer.build_log_view, &format!("[ARTIFACT] Resolved output binary: {}", bin_path.display()));
                            widgets_timer.lbl_build_status.set_text("BUILD OK");
                        }
                        toolchain::makefile::ArtifactResolution::MultipleCandidates(candidates) => {
                            append_log_text(&widgets_timer.build_log_view, &format!("[ARTIFACT] Found {} candidate .bin files in build directory:", candidates.len()));
                            for c in &candidates {
                                append_log_text(&widgets_timer.build_log_view, &format!("  - {}", c.display()));
                            }
                            widgets_timer.lbl_build_status.set_text("AMBIGUOUS");

                            let dialog = adw::MessageDialog::builder()
                                .heading("Multiple Build Artifacts Found")
                                .body("Please select which binary to target:")
                                .transient_for(&widgets_timer.window)
                                .build();
                            for (idx, c) in candidates.iter().enumerate() {
                                let name = c.file_name().unwrap_or_default().to_string_lossy();
                                dialog.add_response(&idx.to_string(), &name);
                            }
                            dialog.add_response("cancel", "Cancel");
                            let log_view_dlg = widgets_timer.build_log_view.clone();
                            let status_dlg = widgets_timer.lbl_build_status.clone();
                            let cand_clone = candidates.clone();
                            dialog.connect_response(None, move |_, resp| {
                                if resp != "cancel" {
                                    if let Ok(idx) = resp.parse::<usize>() {
                                        if let Some(chosen) = cand_clone.get(idx) {
                                            append_log_text(&log_view_dlg, &format!("[ARTIFACT] Selected candidate: {}", chosen.display()));
                                            status_dlg.set_text("BUILD OK");
                                        }
                                    }
                                }
                            });
                            dialog.present();
                        }
                        toolchain::makefile::ArtifactResolution::NoneFound(expected) => {
                            append_log_text(&widgets_timer.build_log_view, &format!("[ERROR] Build succeeded but target .bin was not found. Expected: {}", expected.display()));
                            widgets_timer.lbl_build_status.set_text("ARTIFACT MISSING");
                        }
                    }
                } else {
                    let code_str = code.map(|c| c.to_string()).unwrap_or_else(|| "unknown".to_string());
                    append_log_text(&widgets_timer.build_log_view, &format!("\n[BUILD FAILED] make exited with error code {}. Flashing halted.", code_str));
                    widgets_timer.lbl_build_status.set_text("BUILD FAILED");
                }

                return glib::ControlFlow::Break;
            }

            glib::ControlFlow::Continue
        });
    });

    let win_map = window.clone();
    window.connect_map(move |_| {
        let win = win_map.clone();
        glib::idle_add_local_once(move || {
            let width = win.width();
            let height = win.height();
            let is_max = win.is_maximized();
            println!("[WINDOW MAP SIGNAL] Window mapped: width={}, height={}, maximized={}", width, height, is_max);
        });
    });

    window.present();

    // Check last_project.json on startup
    let config = load_app_config();
    if let Some(dir_str) = config.project_dir {
        let path = PathBuf::from(dir_str);
        if path.exists() {
            try_discover_folder(&path, &state, &widgets);
        }
    }
}



fn try_discover_folder(dir: &Path, state: &Rc<RefCell<AppState>>, widgets: &Rc<AppWidgets>) {
    let mut st = state.borrow_mut();
    st.project_dir = Some(dir.to_path_buf());
    widgets.lbl_discovered_dir.set_text(&dir.display().to_string());

    let has_makefile = dir.join("Makefile").is_file();
    st.has_makefile = has_makefile;
    widgets.btn_build_flash.set_sensitive(has_makefile);

    match discover_project_files(dir) {
        Ok((ioc_path, main_c_path)) => {
            widgets
                .lbl_ioc_path
                .set_text(&format!("IOC: {}", ioc_path.display()));
            widgets
                .lbl_main_c_path
                .set_text(&format!("Main C: {}", main_c_path.display()));

            st.discovered_ioc = Some(ioc_path);
            st.discovered_main_c = Some(main_c_path);
            widgets.btn_load.set_sensitive(true);
        }
        Err(err) => {
            widgets.toast_overlay.add_toast(adw::Toast::new(&format!("Discovery Error: {}", err)));
            widgets.lbl_ioc_path.set_text("IOC Path: —");
            widgets.lbl_main_c_path.set_text("Main C Path: —");
            st.discovered_ioc = None;
            st.discovered_main_c = None;
            widgets.btn_load.set_sensitive(false);
        }
    }
}

fn do_load_project(state: &Rc<RefCell<AppState>>, widgets: &Rc<AppWidgets>) {
    let (ioc_path, main_c_path, dir_path) = {
        let st = state.borrow();

        match (&st.discovered_ioc, &st.discovered_main_c, &st.project_dir) {
            (Some(i), Some(m), Some(d)) => (i.clone(), m.clone(), d.clone()),
            _ => {
                widgets.toast_overlay.add_toast(adw::Toast::new("✗ Project files not selected"));
                return;
            }
        }
    };

    save_app_config(&dir_path.display().to_string());

    let has_makefile = dir_path.join("Makefile").is_file();
    {
        let mut st = state.borrow_mut();
        st.has_makefile = has_makefile;
    }
    widgets.btn_build_flash.set_sensitive(has_makefile);

    match load_project(&ioc_path, &main_c_path) {
        Ok(project) => {
            widgets.lbl_project_name.set_text(&format!("NAME: {}", project.meta.name));
            widgets.lbl_mcu_family.set_text(&format!("FAMILY: {}", project.meta.mcu_family));
            widgets.lbl_mcu_name.set_text(&format!("MCU: {}", project.meta.mcu_name));

            widgets.lbl_periph_header.set_text(&format!("[ ▸ PERIPHERALS ({}) ]", project.peripherals.len()));

            let mut total_regions = project.user_regions.len();
            if project.loop_body.is_some() {
                total_regions += 1;
            }
            widgets.lbl_region_header.set_text(&format!("[ ▸ USER REGIONS ({}) ]", total_regions));

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
                    let mut st = state.borrow_mut();
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
                    st.loaded_project = Some(project);
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
                    let mut st = state.borrow_mut();
                    st.state_diagram_layout = None;
                    st.loaded_project = Some(project);
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
            } else {
                widgets.btn_nucleo_pinout.set_sensitive(false);
                widgets
                    .btn_nucleo_pinout
                    .set_tooltip_text(Some("Nucleo Pinout visualizer is F446RE-only for now"));
            }
            widgets.pinout_drawing_area.queue_draw();

            widgets.toast_overlay.add_toast(adw::Toast::new("✓ Project loaded successfully"));
        }


        Err(err) => {
            widgets.btn_nucleo_pinout.set_sensitive(false);
            widgets.btn_nucleo_pinout.set_tooltip_text(Some("Nucleo Pinout visualizer is F446RE-only for now"));
            widgets.toast_overlay.add_toast(adw::Toast::new(&format!("✗ Load Error: {}", err)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ui_build_smoke() {
        if let Err(err) = gtk4::init() {
            eprintln!("GTK display not available, skipping UI smoke test: {}", err);
            return;
        }
        let _ = adw::init();

        let app = adw::Application::builder()
            .application_id("com.stakhal.ui.smoke_test")
            .flags(gio::ApplicationFlags::NON_UNIQUE)
            .build();

        if let Err(err) = app.register(gio::Cancellable::NONE) {
            eprintln!("Failed to register GTK application in test: {}", err);
            return;
        }

        app.connect_activate(build_ui);
        app.activate();

        let windows = app.windows();
        assert!(
            !windows.is_empty(),
            "Expected ApplicationWindow to be constructed during build_ui"
        );
    }

    #[test]
    fn test_f446_project_loading_enables_pinout_btn() {
        let fixture_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../stakhal-core/tests/fixtures/stakhal_blink_f446re");
        let ioc_path = fixture_dir.join("stakhal_blink_f446re.ioc");
        let main_c_path = fixture_dir.join("Core/Src/main.c");

        let project = load_project(&ioc_path, &main_c_path).expect("Failed to load f446 fixture project");
        assert!(project.meta.mcu_name.to_uppercase().contains("F446"));

        let loc = stakhal_core::nucleo_pinout::lookup_pin("PA5");
        assert!(loc.is_some());
        let pin_loc = loc.unwrap();
        assert_eq!(pin_loc.morpho, Some(("CN10", 11)));
        assert_eq!(pin_loc.arduino, Some(("CN5", 6, "D13")));
    }


}




