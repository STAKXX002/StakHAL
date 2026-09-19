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

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum StatusKind {
    Ready,
    Active,
    Error,
    Idle,
}

pub fn update_build_status(lbl: &gtk4::Label, text: &str, kind: StatusKind) {
    lbl.set_text(text);
    lbl.remove_css_class("status-ready");
    lbl.remove_css_class("status-active");
    lbl.remove_css_class("status-error");
    lbl.remove_css_class("status-idle");
    match kind {
        StatusKind::Ready => lbl.add_css_class("status-ready"),
        StatusKind::Active => lbl.add_css_class("status-active"),
        StatusKind::Error => lbl.add_css_class("status-error"),
        StatusKind::Idle => lbl.add_css_class("status-idle"),
    }
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
    // Default to OpenGL renderer for robust, lag-free hardware acceleration on WSLg & Wayland
    if std::env::var_os("GSK_RENDERER").is_none() {
        std::env::set_var("GSK_RENDERER", "gl");
    }

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
@define-color bg_void #0A0D10;
@define-color bg_panel #14181C;
@define-color border_hair #262C31;
@define-color text_primary #E4E7EA;
@define-color text_muted #6B7378;
@define-color state_ready #34D399;
@define-color state_active #F5A623;
@define-color state_error #E5484D;
@define-color accent #4FD1C5;

* {
    font-family: 'IBM Plex Sans', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;
    font-size: 13px;
    border-radius: 0px;
    box-shadow: none;
}

window, dialog {
    background-color: @bg_void;
    color: @text_primary;
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

/* Panel and Card Containers */
.card, frame, scrolledwindow.card, box.card {
    background-color: @bg_panel;
    border: 1px solid @border_hair;
    border-radius: 0px;
}

/* Buttons */
button.stakhal-btn {
    font-family: 'IBM Plex Sans', sans-serif;
    font-weight: 500;
    border: 1px solid @border_hair;
    background-color: @bg_panel;
    color: @text_primary;
    transition: all 120ms ease;
    border-radius: 2px;
}

button.stakhal-btn:hover {
    border-color: @accent;
    background-color: #1a2026;
    color: #ffffff;
}

button.stakhal-btn:active {
    background-color: #262C31;
}

button.stakhal-btn.suggested-action {
    border: 1px solid @accent;
    background-color: @accent;
    color: @bg_void;
    font-weight: 600;
    border-radius: 2px;
}

button.stakhal-btn.suggested-action:hover {
    border-color: #5fe3d7;
    background-color: #5fe3d7;
    color: @bg_void;
}

button.stakhal-btn.suggested-action:active {
    background-color: #3bb3a8;
}

button.stakhal-btn.flat {
    border: 1px solid transparent;
    background-color: transparent;
    color: @text_muted;
    border-radius: 2px;
}

button.stakhal-btn.flat:hover {
    border-color: @border_hair;
    background-color: @bg_panel;
    color: @text_primary;
}

/* Rows and ListBoxes */
row, listboxrow, actionrow {
    border-radius: 0px;
    transition: none;
    border-bottom: 1px solid @border_hair;
}

.clickable-row {
    transition: all 120ms ease;
}

.clickable-row:hover {
    background-color: #1a2026;
}

.clickable-row:active {
    background-color: #262C31;
}

/* Data / Monospace Typography Split */
textview, textview text, .data-mono, .data-mono * {
    font-family: 'JetBrains Mono', 'DejaVu Sans Mono', 'Liberation Mono', monospace;
    font-size: 12px;
}

textview text {
    background-color: @bg_panel;
    color: @text_primary;
}

.dim-label {
    color: @text_muted;
}

.title-1, .title-2, .title-3, .title-4, .heading {
    font-family: 'IBM Plex Sans', sans-serif;
    color: @text_primary;
    font-weight: 600;
}

/* Strictly Reserved Status Classes (signal only, never decoration) */
.status-ready, .status-ok {
    color: @state_ready;
    font-weight: 600;
}

.status-active, .status-busy, .status-warning {
    color: @state_active;
    font-weight: 600;
}

.status-error, .status-fault {
    color: @state_error;
    font-weight: 600;
}

.status-idle {
    color: @text_muted;
}

.implicit-badge {
    font-family: 'JetBrains Mono', monospace;
    color: @text_muted;
    background-color: #1a2026;
    border: 1px solid @border_hair;
    border-radius: 2px;
    padding: 2px 6px;
}

dropdown button {
    border: 1px solid @border_hair;
    background-color: @bg_panel;
    color: @text_primary;
    border-radius: 2px;
    font-family: 'IBM Plex Sans', sans-serif;
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
        btn_build,
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
        combo_pinout_module,
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
        .title(concat!("StakHAL v", env!("CARGO_PKG_VERSION")))
        .default_width(1280)
        .default_height(820)
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
        btn_build: btn_build.clone(),
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
        combo_pinout_module: combo_pinout_module.clone(),
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

    // Nucleo Pinout module filter dropdown callback
    let state_mod_combo = Rc::clone(&state);
    let area_pinout_combo = widgets.pinout_drawing_area.clone();
    combo_pinout_module.connect_selected_notify(move |cb| {
        let idx = cb.selected();
        let sel_str = cb.model()
            .and_then(|m| m.downcast::<gtk4::StringList>().ok())
            .and_then(|sl| sl.string(idx))
            .map(|s| s.to_string());

        let mod_filter = match sel_str.as_deref() {
            Some("All Modules") | None => None,
            Some(s) => Some(s.to_string()),
        };

        state_mod_combo.borrow_mut().selected_pinout_module = mod_filter;
        area_pinout_combo.queue_draw();
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
        update_build_status(&status_clear, "IDLE", StatusKind::Idle);
    });

    // Connect Build & Flash Button
    // Connect Build Button (compile only, no flash)
    let state_b = Rc::clone(&state);
    let widgets_b = Rc::clone(&widgets);
    widgets.btn_build.connect_clicked(move |_| {
        execute_build_pipeline(false, &state_b, &widgets_b);
    });

    // Connect Build & Flash Button
    let state_bf = Rc::clone(&state);
    let widgets_bf = Rc::clone(&widgets);
    widgets.btn_build_flash.connect_clicked(move |_| {
        execute_build_pipeline(true, &state_bf, &widgets_bf);
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



fn execute_build_pipeline(
    flash_after_build: bool,
    state: &Rc<RefCell<AppState>>,
    widgets: &Rc<AppWidgets>,
) {
    let (dir, build_sys) = {
        let mut st = state.borrow_mut();
        if st.build_in_progress {
            return;
        }
        let dir = match &st.project_dir {
            Some(d) => d.clone(),
            None => return,
        };
        let build_sys = match &st.detected_build_system {
            Some(bs) => bs.clone(),
            None => return,
        };
        st.build_in_progress = true;
        (dir, build_sys)
    };

    widgets.btn_build.set_sensitive(false);
    widgets.btn_build_flash.set_sensitive(false);
    update_build_status(&widgets.lbl_build_status, "BUILDING...", StatusKind::Active);

    let build_cmd_res = toolchain::builder::get_build_command(&build_sys, &dir);
    let (cmd, args, exec_dir) = match build_cmd_res {
        Ok(tuple) => tuple,
        Err(err) => {
            append_log_text(&widgets.build_log_view, &format!("[ERROR] {}", err));
            update_build_status(&widgets.lbl_build_status, "BUILD FAILED", StatusKind::Error);
            let mut st = state.borrow_mut();
            st.build_in_progress = false;
            widgets.btn_build.set_sensitive(st.has_build_system);
            widgets.btn_build_flash.set_sensitive(st.has_build_system);
            return;
        }
    };

    append_log_text(&widgets.build_log_view, "============================================================");
    append_log_text(&widgets.build_log_view, &format!("[BUILD] [{}] Running `{} {}` in {}", build_sys.display_name(), cmd, args.join(" "), exec_dir.display()));
    append_log_text(&widgets.build_log_view, "============================================================");

    let rx = toolchain::runner::spawn_streaming_process(
        cmd.clone(),
        args,
        exec_dir,
    );

    let state_timer = Rc::clone(state);
    let widgets_timer = Rc::clone(widgets);
    let dir_timer = dir.clone();
    let build_sys_timer = build_sys.clone();

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
            if success {
                append_log_text(&widgets_timer.build_log_view, &format!("\n[BUILD SUCCESS] `{}` finished successfully.", cmd));
                let res = toolchain::builder::resolve_artifact_for_build_system(&dir_timer, &build_sys_timer);
                match res {
                    toolchain::makefile::ArtifactResolution::Exact(bin_path) => {
                        append_log_text(&widgets_timer.build_log_view, &format!("[ARTIFACT] Resolved output binary: {}", bin_path.display()));
                        if flash_after_build {
                            update_build_status(&widgets_timer.lbl_build_status, "PROBING...", StatusKind::Active);
                            run_probe_detection_and_flash(bin_path, &state_timer, &widgets_timer, dir_timer.clone());
                        } else {
                            update_build_status(&widgets_timer.lbl_build_status, "SUCCESS", StatusKind::Ready);
                            widgets_timer.toast_overlay.add_toast(adw::Toast::new("[OK] Build succeeded"));
                            let mut st = state_timer.borrow_mut();
                            st.build_in_progress = false;
                            widgets_timer.btn_build.set_sensitive(st.has_build_system);
                            widgets_timer.btn_build_flash.set_sensitive(st.has_build_system);
                        }
                    }
                    toolchain::makefile::ArtifactResolution::MultipleCandidates(candidates) => {
                        append_log_text(&widgets_timer.build_log_view, &format!("[ARTIFACT] Found {} candidate .bin files in build directory:", candidates.len()));
                        for c in &candidates {
                            append_log_text(&widgets_timer.build_log_view, &format!("  - {}", c.display()));
                        }
                        if flash_after_build {
                            update_build_status(&widgets_timer.lbl_build_status, "SELECT ARTIFACT", StatusKind::Active);

                            let dialog = adw::MessageDialog::builder()
                                .heading("Multiple Build Artifacts Found")
                                .body("Please select which binary to flash:")
                                .transient_for(&widgets_timer.window)
                                .build();
                            for (idx, c) in candidates.iter().enumerate() {
                                let name = c.file_name().unwrap_or_default().to_string_lossy();
                                dialog.add_response(&idx.to_string(), &name);
                            }
                            dialog.add_response("cancel", "Cancel");
                            let state_dlg = Rc::clone(&state_timer);
                            let widgets_dlg = Rc::clone(&widgets_timer);
                            let cand_clone = candidates.clone();
                            let dir_clone = dir_timer.clone();
                            dialog.connect_response(None, move |_, resp| {
                                if resp != "cancel" {
                                    if let Ok(idx) = resp.parse::<usize>() {
                                        if let Some(chosen) = cand_clone.get(idx) {
                                            append_log_text(&widgets_dlg.build_log_view, &format!("[ARTIFACT] Selected candidate: {}", chosen.display()));
                                            run_probe_detection_and_flash(chosen.clone(), &state_dlg, &widgets_dlg, dir_clone.clone());
                                            return;
                                        }
                                    }
                                }
                                append_log_text(&widgets_dlg.build_log_view, "[ARTIFACT] Operation cancelled by user.");
                                update_build_status(&widgets_dlg.lbl_build_status, "CANCELLED", StatusKind::Idle);
                                let mut st = state_dlg.borrow_mut();
                                st.build_in_progress = false;
                                widgets_dlg.btn_build.set_sensitive(st.has_build_system);
                                widgets_dlg.btn_build_flash.set_sensitive(st.has_build_system);
                            });
                            dialog.present();
                        } else {
                            update_build_status(&widgets_timer.lbl_build_status, "SUCCESS", StatusKind::Ready);
                            widgets_timer.toast_overlay.add_toast(adw::Toast::new("[OK] Build succeeded"));
                            let mut st = state_timer.borrow_mut();
                            st.build_in_progress = false;
                            widgets_timer.btn_build.set_sensitive(st.has_build_system);
                            widgets_timer.btn_build_flash.set_sensitive(st.has_build_system);
                        }
                    }
                    toolchain::makefile::ArtifactResolution::NoneFound(expected) => {
                        append_log_text(&widgets_timer.build_log_view, &format!("[ERROR] Build succeeded but target .bin was not found. Expected: {}", expected.display()));
                        update_build_status(&widgets_timer.lbl_build_status, "ARTIFACT MISSING", StatusKind::Error);
                        let mut st = state_timer.borrow_mut();
                        st.build_in_progress = false;
                        widgets_timer.btn_build.set_sensitive(st.has_build_system);
                        widgets_timer.btn_build_flash.set_sensitive(st.has_build_system);
                    }
                }
            } else {
                let code_str = code.map(|c| c.to_string()).unwrap_or_else(|| "unknown".to_string());
                let action_type = if flash_after_build { "Flashing halted." } else { "Build failed." };
                append_log_text(&widgets_timer.build_log_view, &format!("\n[BUILD FAILED] {} exited with error code {}. {}", cmd, code_str, action_type));
                update_build_status(&widgets_timer.lbl_build_status, "BUILD FAILED", StatusKind::Error);
                let mut st = state_timer.borrow_mut();
                st.build_in_progress = false;
                widgets_timer.btn_build.set_sensitive(st.has_build_system);
                widgets_timer.btn_build_flash.set_sensitive(st.has_build_system);
            }

            return glib::ControlFlow::Break;
        }

        glib::ControlFlow::Continue
    });
}

fn try_discover_folder(dir: &Path, state: &Rc<RefCell<AppState>>, widgets: &Rc<AppWidgets>) {
    let mut st = state.borrow_mut();
    st.project_dir = Some(dir.to_path_buf());
    widgets.lbl_discovered_dir.set_text(&dir.display().to_string());

    let build_sys = toolchain::builder::detect_build_system(dir);
    let has_build_sys = build_sys.is_some();
    st.has_makefile = dir.join("Makefile").is_file() || dir.join("makefile").is_file();
    st.has_build_system = has_build_sys;
    st.detected_build_system = build_sys;
    widgets.btn_build.set_sensitive(has_build_sys);
    widgets.btn_build_flash.set_sensitive(has_build_sys);

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
            widgets.lbl_ioc_path.set_text("IOC Path: N/A");
            widgets.lbl_main_c_path.set_text("Main C Path: N/A");
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
                widgets.toast_overlay.add_toast(adw::Toast::new("[ERROR] Project files not selected"));
                return;
            }
        }
    };

    save_app_config(&dir_path.display().to_string());

    let build_sys = toolchain::builder::detect_build_system(&dir_path);
    let has_build_sys = build_sys.is_some();
    {
        let mut st = state.borrow_mut();
        st.has_makefile = dir_path.join("Makefile").is_file() || dir_path.join("makefile").is_file();
        st.has_build_system = has_build_sys;
        st.detected_build_system = build_sys;
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

                let mut mod_names = vec!["All Modules".to_string()];
                if project_modules.len() > 1 {
                    mod_names.extend(project_modules.clone());
                }
                let mod_str_refs: Vec<&str> = mod_names.iter().map(|s| s.as_str()).collect();
                let mod_string_list = gtk4::StringList::new(&mod_str_refs);
                widgets.combo_pinout_module.set_model(Some(&mod_string_list));
                widgets.combo_pinout_module.set_selected(0);
                widgets.combo_pinout_module.set_sensitive(project_modules.len() > 1);

                state.borrow_mut().selected_pinout_module = None;
            } else {
                widgets.btn_nucleo_pinout.set_sensitive(false);
                widgets
                    .btn_nucleo_pinout
                    .set_tooltip_text(Some("Nucleo Pinout visualizer is F446RE-only for now"));
                let mod_string_list = gtk4::StringList::new(&["All Modules"]);
                widgets.combo_pinout_module.set_model(Some(&mod_string_list));
                widgets.combo_pinout_module.set_selected(0);
                widgets.combo_pinout_module.set_sensitive(false);
                state.borrow_mut().selected_pinout_module = None;
            }
            widgets.pinout_drawing_area.queue_draw();

            widgets.toast_overlay.add_toast(adw::Toast::new("[OK] Project loaded successfully"));
        }

        Err(err) => {
            widgets.btn_nucleo_pinout.set_sensitive(false);
            widgets.btn_nucleo_pinout.set_tooltip_text(Some("Nucleo Pinout visualizer is F446RE-only for now"));
            let mod_string_list = gtk4::StringList::new(&["All Modules"]);
            widgets.combo_pinout_module.set_model(Some(&mod_string_list));
            widgets.combo_pinout_module.set_selected(0);
            widgets.combo_pinout_module.set_sensitive(false);
            state.borrow_mut().selected_pinout_module = None;
            widgets.toast_overlay.add_toast(adw::Toast::new(&format!("[ERROR] Load Error: {}", err)));
        }
    }
}

fn run_probe_detection_and_flash(
    artifact: PathBuf,
    state: &Rc<RefCell<AppState>>,
    widgets: &Rc<AppWidgets>,
    project_dir: PathBuf,
) {
    append_log_text(&widgets.build_log_view, "\n============================================================");
    append_log_text(&widgets.build_log_view, "[PROBE] Scanning for connected ST-Link programmers (`st-info --probe`)...");
    append_log_text(&widgets.build_log_view, "============================================================");

    match toolchain::probe::detect_stlink_probes() {
        Ok(probes) => {
            if probes.len() == 1 {
                let p = &probes[0];
                append_log_text(&widgets.build_log_view, &format!("[PROBE] Detected single target: {}", p.display_label()));
                run_flash_stage(artifact, Some(p.serial.clone()), state, widgets, project_dir);
            } else {
                append_log_text(&widgets.build_log_view, &format!("[PROBE] Detected {} ST-Link programmers:", probes.len()));
                for p in &probes {
                    append_log_text(&widgets.build_log_view, &format!("  - {}", p.display_label()));
                }
                update_build_status(&widgets.lbl_build_status, "SELECT PROBE", StatusKind::Active);

                let dialog = adw::MessageDialog::builder()
                    .heading("Multiple ST-Link Probes Detected")
                    .body("Please select which ST-Link probe to flash:")
                    .transient_for(&widgets.window)
                    .build();

                for (idx, p) in probes.iter().enumerate() {
                    dialog.add_response(&idx.to_string(), &p.display_label());
                }
                dialog.add_response("cancel", "Cancel");

                let state_dlg = Rc::clone(state);
                let widgets_dlg = Rc::clone(widgets);
                let probes_clone = probes.clone();
                let artifact_clone = artifact.clone();
                let dir_clone = project_dir.clone();

                dialog.connect_response(None, move |_, resp| {
                    if resp != "cancel" {
                        if let Ok(idx) = resp.parse::<usize>() {
                            if let Some(chosen_probe) = probes_clone.get(idx) {
                                append_log_text(&widgets_dlg.build_log_view, &format!("[PROBE] Selected target: {}", chosen_probe.display_label()));
                                run_flash_stage(artifact_clone.clone(), Some(chosen_probe.serial.clone()), &state_dlg, &widgets_dlg, dir_clone.clone());
                                return;
                            }
                        }
                    }
                    append_log_text(&widgets_dlg.build_log_view, "[PROBE] Flashing cancelled by user.");
                    update_build_status(&widgets_dlg.lbl_build_status, "CANCELLED", StatusKind::Idle);
                    let mut st = state_dlg.borrow_mut();
                    st.build_in_progress = false;
                    widgets_dlg.btn_build.set_sensitive(st.has_build_system);
                    widgets_dlg.btn_build_flash.set_sensitive(st.has_build_system);
                });
                dialog.present();
            }
        }
        Err(err) => {
            append_log_text(&widgets.build_log_view, &format!("[ERROR] {}", err));
            let (txt, kind) = match err {
                toolchain::probe::ProbeError::ToolNotFound => ("ST-INFO MISSING", StatusKind::Error),
                toolchain::probe::ProbeError::ZeroProbesFound => ("NO PROBE", StatusKind::Active),
                toolchain::probe::ProbeError::ExecutionFailed(_) => ("PROBE ERROR", StatusKind::Error),
            };
            update_build_status(&widgets.lbl_build_status, txt, kind);
            widgets.toast_overlay.add_toast(adw::Toast::new(&format!("[ERROR] {}", err)));
            let mut st = state.borrow_mut();
            st.build_in_progress = false;
            widgets.btn_build.set_sensitive(st.has_build_system);
            widgets.btn_build_flash.set_sensitive(st.has_build_system);
        }
    }
}

fn run_flash_stage(
    artifact: PathBuf,
    probe_serial: Option<String>,
    state: &Rc<RefCell<AppState>>,
    widgets: &Rc<AppWidgets>,
    project_dir: PathBuf,
) {
    if !toolchain::runner::is_executable_on_path("st-flash") {
        append_log_text(&widgets.build_log_view, "[ERROR] `st-flash` executable not found on PATH. Please install stlink-tools (e.g. `sudo apt install stlink-tools`).");
        update_build_status(&widgets.lbl_build_status, "ST-FLASH MISSING", StatusKind::Error);
        widgets.toast_overlay.add_toast(adw::Toast::new("[ERROR] `st-flash` not found on PATH"));
        let mut st = state.borrow_mut();
        st.build_in_progress = false;
        widgets.btn_build.set_sensitive(st.has_build_system);
        widgets.btn_build_flash.set_sensitive(st.has_build_system);
        return;
    }

    let (cmd, args) = toolchain::flasher::build_flash_command(probe_serial.as_deref(), &artifact);

    update_build_status(&widgets.lbl_build_status, "FLASHING...", StatusKind::Active);
    append_log_text(&widgets.build_log_view, "\n============================================================");
    append_log_text(&widgets.build_log_view, &format!("[FLASH] Running `{} {}`", cmd, args.join(" ")));
    append_log_text(&widgets.build_log_view, "============================================================");

    let rx = toolchain::runner::spawn_streaming_process(cmd, args, project_dir);

    let state_timer = Rc::clone(state);
    let widgets_timer = Rc::clone(widgets);

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
            widgets_timer.btn_build.set_sensitive(st.has_build_system);
            widgets_timer.btn_build_flash.set_sensitive(st.has_build_system);

            if success {
                append_log_text(&widgets_timer.build_log_view, "\n[FLASH SUCCESS] Firmware written to 0x08000000 and target MCU reset successfully!");
                update_build_status(&widgets_timer.lbl_build_status, "SUCCESS", StatusKind::Ready);
                widgets_timer.toast_overlay.add_toast(adw::Toast::new("[OK] Build and Flash Succeeded!"));
            } else {
                let code_str = code.map(|c| c.to_string()).unwrap_or_else(|| "unknown".to_string());
                append_log_text(&widgets_timer.build_log_view, &format!("\n[FLASH FAILED] st-flash exited with code {}.", code_str));
                update_build_status(&widgets_timer.lbl_build_status, "FLASH FAILED", StatusKind::Error);
                widgets_timer.toast_overlay.add_toast(adw::Toast::new("[ERROR] Flash failed (see console output)"));
            }

            return glib::ControlFlow::Break;
        }

        glib::ControlFlow::Continue
    });
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

    #[test]
    fn test_multi_machine_loading_and_selection_aa_ns_stm_port() {
        let fixture_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../stakhal-core/tests/fixtures/aa_ns_stm_port");
        let ioc_path = fixture_dir.join("aa_ns_stm_port.ioc");
        let main_c_path = fixture_dir.join("Core/Src/main.c");

        let project = load_project(&ioc_path, &main_c_path).expect("Failed to load aa_ns_stm_port project");
        assert_eq!(project.state_machines.len(), 2, "Expected exactly 2 state machines");

        // Verify machine 0: AlignState (state)
        let sm0 = &project.state_machines[0];
        assert_eq!(sm0.enum_def.name, "AlignState");
        assert_eq!(sm0.display_name, "AlignState (state)");
        assert_eq!(sm0.states.len(), 12); // 11 variants + synthetic SYSTEM FAULT
        let layout0 = stakhal_core::graph::compute_state_machine_layout(sm0);
        assert_eq!(layout0.nodes.len(), 12);
        assert!(layout0.nodes.contains_key("SYSTEM FAULT"));

        // Verify machine 1: HatchState (hatchState)
        let sm1 = &project.state_machines[1];
        assert_eq!(sm1.enum_def.name, "HatchState");
        assert_eq!(sm1.display_name, "HatchState (hatchState)");
        assert_eq!(sm1.states.len(), 3);
        let layout1 = stakhal_core::graph::compute_state_machine_layout(sm1);
        assert_eq!(layout1.nodes.len(), 3);
        assert!(!layout1.nodes.contains_key("SYSTEM FAULT"));
    }

    #[test]
    fn test_render_snapshots() {
        let _ = gtk4::init();

        // 1. State diagram rendering test
        let surface_sm = gtk4::cairo::ImageSurface::create(gtk4::cairo::Format::ARgb32, 1400, 900).expect("surface create");
        let cr_sm = gtk4::cairo::Context::new(&surface_sm).expect("cr create");
        let state_sm = Rc::new(RefCell::new(AppState::default()));
        let fixture_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../stakhal-core/tests/fixtures/docking_firmware_v2");
        let ioc_path = fixture_dir.join("docking_firmware_v2.ioc");
        let main_c_path = fixture_dir.join("Core/Src/main.c");
        if let Ok(project) = load_project(&ioc_path, &main_c_path) {
            state_sm.borrow_mut().loaded_project = Some(project);
            state_sm.borrow_mut().selected_state_node = Some("GOING".to_string());
            ui::state_diagram::draw::draw_state_diagram(&cr_sm, 1400.0, 900.0, &state_sm);
            surface_sm.flush();
        }

        // 2. Nucleo pinout rendering test
        let surface_pin = gtk4::cairo::ImageSurface::create(gtk4::cairo::Format::ARgb32, 1400, 850).expect("surface create");
        let cr_pin = gtk4::cairo::Context::new(&surface_pin).expect("cr create");
        let state_pin = Rc::new(RefCell::new(AppState::default()));
        let f446_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../stakhal-core/tests/fixtures/stakhal_blink_f446re");
        let f446_ioc = f446_dir.join("stakhal_blink_f446re.ioc");
        let f446_main = f446_dir.join("Core/Src/main.c");
        if let Ok(project) = load_project(&f446_ioc, &f446_main) {
            state_pin.borrow_mut().loaded_project = Some(project);
            state_pin.borrow_mut().hovered_pinout_pin = Some(("CN10".to_string(), 11));
            state_pin.borrow_mut().hovered_pinout_mouse = Some((600.0, 300.0));
            ui::nucleo_pinout::draw::draw_nucleo_pinout(&cr_pin, 1400.0, 850.0, &state_pin);
            surface_pin.flush();
        }

        // 3. Nucleo pinout module filter snapshots (aa_ns_stm_port)
        let aa_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../stakhal-core/tests/fixtures/aa_ns_stm_port");
        let aa_ioc = aa_dir.join("aa_ns_stm_port.ioc");
        let aa_main = aa_dir.join("Core/Src/main.c");
        if let Ok(project) = load_project(&aa_ioc, &aa_main) {
            let artifact_dir = std::path::Path::new("/home/stakxx002/.gemini/antigravity-ide/brain/e21edbbd-844e-44ef-9dfa-1af3c8e3a19b");

            // All modules
            let surf_all = gtk4::cairo::ImageSurface::create(gtk4::cairo::Format::ARgb32, 1400, 850).expect("surface");
            let cr_all = gtk4::cairo::Context::new(&surf_all).expect("cr");
            let st_all = Rc::new(RefCell::new(AppState {
                loaded_project: Some(project.clone()),
                selected_pinout_module: None,
                ..AppState::default()
            }));
            ui::nucleo_pinout::draw::draw_nucleo_pinout(&cr_all, 1400.0, 850.0, &st_all);
            surf_all.flush();
            if artifact_dir.exists() {
                if let Ok(mut f) = std::fs::File::create(artifact_dir.join("pinout_all_modules.png")) {
                    let _ = surf_all.write_to_png(&mut f);
                }
            }

            // Hatch module
            let surf_hatch = gtk4::cairo::ImageSurface::create(gtk4::cairo::Format::ARgb32, 1400, 850).expect("surface");
            let cr_hatch = gtk4::cairo::Context::new(&surf_hatch).expect("cr");
            let st_hatch = Rc::new(RefCell::new(AppState {
                loaded_project: Some(project.clone()),
                selected_pinout_module: Some("hatch".to_string()),
                hovered_pinout_pin: Some(("CN10".to_string(), 16)), // GRIP_IN1
                hovered_pinout_mouse: Some((850.0, 320.0)),
                ..AppState::default()
            }));
            ui::nucleo_pinout::draw::draw_nucleo_pinout(&cr_hatch, 1400.0, 850.0, &st_hatch);
            surf_hatch.flush();
            if artifact_dir.exists() {
                if let Ok(mut f) = std::fs::File::create(artifact_dir.join("pinout_module_hatch.png")) {
                    let _ = surf_hatch.write_to_png(&mut f);
                }
            }

            // 4. State diagram orthogonal snapshots: docking_firmware_v2
            if let Ok(dock_proj) = load_project(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../stakhal-core/tests/fixtures/docking_firmware_v2/docking_firmware_v2.ioc"), &Path::new(env!("CARGO_MANIFEST_DIR")).join("../stakhal-core/tests/fixtures/docking_firmware_v2/Core/Src/main.c")) {
                let sm = dock_proj.state_machines[0].clone();
                let layout = stakhal_core::graph::compute_state_machine_layout(&sm);
                let w = (layout.width as i32).max(1400);
                let h = (layout.height as i32).max(900);

                let surf_dock = gtk4::cairo::ImageSurface::create(gtk4::cairo::Format::ARgb32, w, h).expect("surf");
                let cr_dock = gtk4::cairo::Context::new(&surf_dock).expect("cr");
                let st_dock = Rc::new(RefCell::new(AppState {
                    loaded_project: Some(dock_proj.clone()),
                    state_diagram_layout: Some(layout.clone()),
                    diagram_bounds: (w, h),
                    diagram_zoom: 1.0,
                    diagram_pan_x: 40.0,
                    diagram_pan_y: 40.0,
                    selected_state_node: None,
                    ..AppState::default()
                }));
                ui::state_diagram::draw::draw_state_diagram(&cr_dock, w as f64, h as f64, &st_dock);
                surf_dock.flush();
                if artifact_dir.exists() {
                    if let Ok(mut f) = std::fs::File::create(artifact_dir.join("orthogonal_docking_v2.png")) {
                        let _ = surf_dock.write_to_png(&mut f);
                    }
                }

                // Docking with GOING selected
                let surf_dock_sel = gtk4::cairo::ImageSurface::create(gtk4::cairo::Format::ARgb32, w, h).expect("surf");
                let cr_dock_sel = gtk4::cairo::Context::new(&surf_dock_sel).expect("cr");
                let st_dock_sel = Rc::new(RefCell::new(AppState {
                    loaded_project: Some(dock_proj),
                    state_diagram_layout: Some(layout),
                    diagram_bounds: (w, h),
                    diagram_zoom: 1.0,
                    diagram_pan_x: 40.0,
                    diagram_pan_y: 40.0,
                    selected_state_node: Some("GOING".to_string()),
                    ..AppState::default()
                }));
                ui::state_diagram::draw::draw_state_diagram(&cr_dock_sel, w as f64, h as f64, &st_dock_sel);
                surf_dock_sel.flush();
                if artifact_dir.exists() {
                    if let Ok(mut f) = std::fs::File::create(artifact_dir.join("orthogonal_docking_v2_selected.png")) {
                        let _ = surf_dock_sel.write_to_png(&mut f);
                    }
                }
            }

            // 5. State diagram orthogonal snapshots: aa_ns_stm_port (AlignState)
            {
                let sm_align = project.state_machines.iter().find(|s| s.enum_def.name.contains("Align")).unwrap_or(&project.state_machines[0]).clone();
                let layout_align = stakhal_core::graph::compute_state_machine_layout(&sm_align);
                let w = (layout_align.width as i32).max(1400);
                let h = (layout_align.height as i32).max(900);

                let surf_align = gtk4::cairo::ImageSurface::create(gtk4::cairo::Format::ARgb32, w, h).expect("surf");
                let cr_align = gtk4::cairo::Context::new(&surf_align).expect("cr");
                let st_align = Rc::new(RefCell::new(AppState {
                    loaded_project: Some(project.clone()),
                    state_diagram_layout: Some(layout_align.clone()),
                    diagram_bounds: (w, h),
                    diagram_zoom: 1.0,
                    diagram_pan_x: 40.0,
                    diagram_pan_y: 40.0,
                    selected_state_node: None,
                    ..AppState::default()
                }));
                ui::state_diagram::draw::draw_state_diagram(&cr_align, w as f64, h as f64, &st_align);
                surf_align.flush();
                if artifact_dir.exists() {
                    if let Ok(mut f) = std::fs::File::create(artifact_dir.join("orthogonal_aa_ns_align.png")) {
                        let _ = surf_align.write_to_png(&mut f);
                    }
                }

                // AlignState with RETURNING selected (shows RETURNING -> RECOVERY cross-lane edge)
                let surf_align_ret = gtk4::cairo::ImageSurface::create(gtk4::cairo::Format::ARgb32, w, h).expect("surf");
                let cr_align_ret = gtk4::cairo::Context::new(&surf_align_ret).expect("cr");
                let st_align_ret = Rc::new(RefCell::new(AppState {
                    loaded_project: Some(project),
                    state_diagram_layout: Some(layout_align),
                    diagram_bounds: (w, h),
                    diagram_zoom: 1.0,
                    diagram_pan_x: 40.0,
                    diagram_pan_y: 40.0,
                    selected_state_node: Some("RETURNING".to_string()),
                    ..AppState::default()
                }));
                ui::state_diagram::draw::draw_state_diagram(&cr_align_ret, w as f64, h as f64, &st_align_ret);
                surf_align_ret.flush();
                if artifact_dir.exists() {
                    if let Ok(mut f) = std::fs::File::create(artifact_dir.join("orthogonal_aa_ns_align_selected.png")) {
                        let _ = surf_align_ret.write_to_png(&mut f);
                    }
                }
            }
        }
    }
}




