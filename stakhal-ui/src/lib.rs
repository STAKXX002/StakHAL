use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use gtk4::{gdk, gio, glib};
use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
pub mod config;
pub mod state;
pub mod toolchain;
pub mod ui;
pub use ui::setup_project::{do_load_project, try_discover_folder};
use crate::ui::serial_monitor::{build_serial_monitor_panel, SerialMonitorWidgets};

pub(crate) fn append_log_text(view: &gtk4::TextView, text: &str) {
    let buffer = view.buffer();
    let mut end_iter = buffer.end_iter();
    buffer.insert(&mut end_iter, text);
    if !text.ends_with('\n') {
        buffer.insert(&mut buffer.end_iter(), "\n");
    }
    let mark = buffer.create_mark(None, &buffer.end_iter(), false);
    view.scroll_to_mark(&mark, 0.0, true, 0.0, 1.0);
}

fn append_serial_text(view: &gtk4::TextView, scrolled: &gtk4::ScrolledWindow, text: &str) {
    let vadj = scrolled.vadjustment();
    let is_at_bottom = (vadj.value() + vadj.page_size()) >= (vadj.upper() - 25.0);

    let buffer = view.buffer();
    let mut end_iter = buffer.end_iter();
    buffer.insert(&mut end_iter, text);

    if is_at_bottom {
        let mark = buffer.create_mark(None, &buffer.end_iter(), false);
        view.scroll_to_mark(&mark, 0.0, true, 0.0, 1.0);
    }
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

use state::{AppState, AppWidgets};
use ui::nucleo_pinout::{
    build_nucleo_pinout_panel, setup_nucleo_pinout_drawing_and_gestures, NucleoPinoutPanelWidgets,
};
use ui::state_diagram::{
    build_state_diagram_panel, setup_state_diagram_drawing_and_gestures, StateDiagramPanelWidgets,
};

use ui::main_panel::{build_main_panel, MainPanelWidgets};


pub const APP_ID: &str = "com.stakhal.ui";

pub fn run() {
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

pub fn build_ui(app: &adw::Application) {
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


    if let Some(display) = gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &css_provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    let MainPanelWidgets {
        overview_box,
        btn_browse,
        btn_load,
        btn_build,
        btn_build_flash,
        btn_enable_traceability,
        btn_call_graph,
        btn_nucleo_pinout,
        lbl_discovered_dir,
        lbl_ioc_path,
        lbl_main_c_path,
        lbl_project_name,
        lbl_mcu_family,
        lbl_mcu_name,
        lbl_build_traceability,
        lbl_periph_header,
        lbl_region_header,
        list_peripherals,
        list_user_regions,
        build_log_view,
        lbl_build_status,
        btn_clear_log,
        btn_serial_monitor,
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

    let SerialMonitorWidgets {
        serial_panel_box,
        btn_serial_back,
        combo_port,
        btn_refresh_ports,
        combo_baud,
        btn_connect: btn_connect_serial,
        lbl_serial_status,
        btn_clear_log: btn_clear_serial,
        serial_log_view,
        serial_scrolled,
        entry_command,
        btn_send: btn_send_command,
        box_quick_commands,
    } = build_serial_monitor_panel();

    let stack = gtk4::Stack::builder()
        .transition_type(gtk4::StackTransitionType::SlideLeftRight)
        .transition_duration(220)
        .build();

    stack.add_named(&overview_box, Some("overview"));
    stack.add_named(&diagram_panel_box, Some("state_diagram"));
    stack.add_named(&pinout_panel_box, Some("nucleo_pinout"));
    stack.add_named(&serial_panel_box, Some("serial_monitor"));
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
        stack: stack.clone(),
        toast_overlay,
        lbl_discovered_dir,
        lbl_ioc_path,
        lbl_main_c_path,
        btn_load,
        btn_build: btn_build.clone(),
        btn_build_flash: btn_build_flash.clone(),
        btn_enable_traceability: btn_enable_traceability.clone(),
        btn_call_graph: btn_call_graph.clone(),
        btn_nucleo_pinout: btn_nucleo_pinout.clone(),
        lbl_project_name,
        lbl_mcu_family,
        lbl_mcu_name,
        lbl_build_traceability,
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
        btn_serial_monitor: btn_serial_monitor.clone(),
        combo_port: combo_port.clone(),
        btn_refresh_ports: btn_refresh_ports.clone(),
        combo_baud: combo_baud.clone(),
        btn_connect_serial: btn_connect_serial.clone(),
        lbl_serial_status: lbl_serial_status.clone(),
        btn_clear_serial: btn_clear_serial.clone(),
        serial_log_view: serial_log_view.clone(),
        serial_scrolled: serial_scrolled.clone(),
        entry_command: entry_command.clone(),
        btn_send_command: btn_send_command.clone(),
        box_quick_commands: box_quick_commands.clone(),
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

    let stack_back_serial = stack.clone();
    btn_serial_back.connect_clicked(move |_| {
        stack_back_serial.set_visible_child_full("overview", gtk4::StackTransitionType::SlideRight);
    });

    let stack_diagram = stack.clone();
    btn_call_graph.connect_clicked(move |_| {
        stack_diagram.set_visible_child_full("state_diagram", gtk4::StackTransitionType::SlideLeft);
    });

    let stack_pinout = stack.clone();
    btn_nucleo_pinout.connect_clicked(move |_| {
        stack_pinout.set_visible_child_full("nucleo_pinout", gtk4::StackTransitionType::SlideLeft);
    });

    let stack_serial = stack.clone();
    let state_serial = Rc::clone(&state);
    let widgets_serial = Rc::clone(&widgets);
    btn_serial_monitor.connect_clicked(move |_| {
        refresh_serial_ports(&state_serial, &widgets_serial);
        stack_serial.set_visible_child_full("serial_monitor", gtk4::StackTransitionType::SlideLeft);
    });

    // State machine selector dropdown callback
    let state_combo = Rc::clone(&state);
    let area_combo = diagram_drawing_area.clone();
    let lbl_info_combo = lbl_selected_info.clone();
    combo_state_machine.connect_selected_notify(move |cb| {
        let idx = cb.selected() as usize;
        state_combo.borrow().with_canvas_state_mut(|st| {
            st.selected_state_machine = idx;
            st.selected_state_node = None;
            st.state_diagram_layout = None; // trigger layout recompute for selected machine
            st.diagram_needs_fit = true;
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

        state_mod_combo.borrow().with_canvas_state_mut(|c| {
            c.selected_pinout_module = mod_filter;
        });
        area_pinout_combo.queue_draw();
    });

    // Connect Project Setup (Browse & Load)
    ui::setup_project::setup_project_handlers(&btn_browse, &state, &widgets);

    // Connect Clear Console Button

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

    // Connect Enable Build Traceability Button
    let state_trace = Rc::clone(&state);
    let widgets_trace = Rc::clone(&widgets);
    widgets.btn_enable_traceability.connect_clicked(move |_| {
        let (project_dir, main_c_path) = {
            let st = state_trace.borrow();
            let proj = st.project.borrow();
            match (&proj.project_dir, &proj.discovered_main_c) {
                (Some(d), Some(m)) => (d.clone(), m.clone()),
                _ => return,
            }
        };

        if !toolchain::traceability::is_git_repository(&project_dir) {
            widgets_trace
                .toast_overlay
                .add_toast(adw::Toast::new("Cannot enable traceability: project is not inside a Git repository"));
            return;
        }

        if toolchain::traceability::is_traceability_enabled_in_source(&main_c_path) {
            widgets_trace
                .toast_overlay
                .add_toast(adw::Toast::new("Build traceability is already enabled in main.c"));
            return;
        }

        let diff_preview = toolchain::traceability::generate_traceability_diff_preview(&main_c_path);

        let dialog = adw::MessageDialog::builder()
            .heading("Enable Build Traceability")
            .body("StakHAL can insert the build-info header include and boot banner printf into CubeMX user code regions in main.c.\n\nReview the exact changes below before applying:")
            .transient_for(&widgets_trace.window)
            .build();

        let preview_view = gtk4::TextView::builder()
            .editable(false)
            .cursor_visible(false)
            .monospace(true)
            .top_margin(8)
            .bottom_margin(8)
            .left_margin(12)
            .right_margin(12)
            .build();
        preview_view.buffer().set_text(&diff_preview);

        let preview_scrolled = gtk4::ScrolledWindow::builder()
            .min_content_height(140)
            .max_content_height(240)
            .child(&preview_view)
            .css_classes(vec!["card".to_string()])
            .build();

        dialog.set_extra_child(Some(&preview_scrolled));

        dialog.add_response("cancel", "Cancel");
        dialog.add_response("confirm", "Enable & Insert");
        dialog.set_response_appearance("confirm", adw::ResponseAppearance::Suggested);

        let state_dlg = Rc::clone(&state_trace);
        let widgets_dlg = Rc::clone(&widgets_trace);
        let dir_clone = project_dir.clone();
        let main_c_clone = main_c_path.clone();

        dialog.connect_response(None, move |_, resp| {
            if resp == "confirm" {
                match toolchain::traceability::insert_traceability_into_source(&main_c_clone, &dir_clone) {
                    Ok(()) => {
                        append_log_text(
                            &widgets_dlg.build_log_view,
                            &format!("[TRACEABILITY] Successfully inserted build info into {}", main_c_clone.display()),
                        );
                        widgets_dlg.toast_overlay.add_toast(
                            adw::Toast::new("Build traceability enabled in main.c")
                        );
                        update_traceability_ui(&state_dlg, &widgets_dlg);
                    }
                    Err(err) => {
                        append_log_text(
                            &widgets_dlg.build_log_view,
                            &format!("[TRACEABILITY ERROR] Failed to insert build info: {}", err),
                        );
                        widgets_dlg.toast_overlay.add_toast(
                            adw::Toast::new(&format!("Error: {}", err))
                        );
                    }
                }
            }
        });

        dialog.present();
    });

    // Connect Serial Monitor Controls
    let state_ref_ports = Rc::clone(&state);
    let widgets_ref_ports = Rc::clone(&widgets);
    widgets.btn_refresh_ports.connect_clicked(move |_| {
        refresh_serial_ports(&state_ref_ports, &widgets_ref_ports);
        update_traceability_ui(&state_ref_ports, &widgets_ref_ports);
    });

    let state_cp = Rc::clone(&state);
    widgets.combo_port.connect_selected_notify(move |cb| {
        let idx = cb.selected() as usize;
        let st = state_cp.borrow();
        let mut ser = st.serial.borrow_mut();
        if idx < ser.available_serial_ports.len() {
            ser.selected_serial_port = Some(ser.available_serial_ports[idx].port_name.clone());
        }
    });

    let state_cb = Rc::clone(&state);
    widgets.combo_baud.connect_selected_notify(move |cb| {
        let idx = cb.selected() as usize;
        if idx < crate::toolchain::serial::COMMON_BAUD_RATES.len() {
            let baud = crate::toolchain::serial::COMMON_BAUD_RATES[idx];
            state_cb.borrow().serial.borrow_mut().selected_serial_baud = baud;
        }
    });

    let s_view_clear = widgets.serial_log_view.clone();
    widgets.btn_clear_serial.connect_clicked(move |_| {
        s_view_clear.buffer().set_text("");
    });

    let state_conn = Rc::clone(&state);
    let widgets_conn = Rc::clone(&widgets);
    widgets.btn_connect_serial.connect_clicked(move |_| {
        toggle_serial_connection(&state_conn, &widgets_conn);
    });

    let state_send = Rc::clone(&state);
    let widgets_send = Rc::clone(&widgets);
    let entry_cmd_clone = widgets.entry_command.clone();
    widgets.btn_send_command.connect_clicked(move |_| {
        let text = entry_cmd_clone.text().to_string();
        send_serial_command(&text, &state_send, &widgets_send);
        entry_cmd_clone.set_text("");
    });

    let state_entry = Rc::clone(&state);
    let widgets_entry = Rc::clone(&widgets);
    widgets.entry_command.connect_activate(move |entry| {
        let text = entry.text().to_string();
        send_serial_command(&text, &state_entry, &widgets_entry);
        entry.set_text("");
    });

    let state_keys = Rc::clone(&state);
    let entry_keys = widgets.entry_command.clone();
    let key_controller = gtk4::EventControllerKey::new();
    key_controller.connect_key_pressed(move |_, key, _, _| {
        let st = state_keys.borrow();
        let mut ser = st.serial.borrow_mut();
        if ser.serial_command_history.is_empty() {
            return glib::Propagation::Proceed;
        }

        match key {
            gdk::Key::Up => {
                let new_idx = match ser.serial_history_index {
                    None => ser.serial_command_history.len().saturating_sub(1),
                    Some(idx) => idx.saturating_sub(1),
                };
                ser.serial_history_index = Some(new_idx);
                if let Some(cmd) = ser.serial_command_history.get(new_idx) {
                    entry_keys.set_text(cmd);
                    entry_keys.set_position(-1);
                }
                glib::Propagation::Stop
            }
            gdk::Key::Down => {
                if let Some(idx) = ser.serial_history_index {
                    if idx + 1 < ser.serial_command_history.len() {
                        let new_idx = idx + 1;
                        ser.serial_history_index = Some(new_idx);
                        if let Some(cmd) = ser.serial_command_history.get(new_idx) {
                            entry_keys.set_text(cmd);
                            entry_keys.set_position(-1);
                        }
                    } else {
                        ser.serial_history_index = None;
                        entry_keys.set_text("");
                    }
                    glib::Propagation::Stop
                } else {
                    glib::Propagation::Proceed
                }
            }
            _ => glib::Propagation::Proceed,
        }
    });
    widgets.entry_command.add_controller(key_controller);

    refresh_serial_ports(&state, &widgets);

    window.present();

    // Check last_project.json on startup
    ui::setup_project::check_last_project(&state, &widgets);
}



fn execute_build_pipeline(
    flash_after_build: bool,
    state: &Rc<RefCell<AppState>>,
    widgets: &Rc<AppWidgets>,
) {
    let (dir, build_sys) = {
        let st = state.borrow();
        let mut bt = st.build_trace.borrow_mut();
        if bt.build_in_progress {
            return;
        }

        let dir = match &st.project.borrow().project_dir {
            Some(d) => d.clone(),
            None => return,
        };
        let build_sys = match &bt.detected_build_system {
            Some(bs) => bs.clone(),
            None => return,
        };
        bt.build_in_progress = true;
        (dir, build_sys)
    };

    widgets.btn_build.set_sensitive(false);
    widgets.btn_build_flash.set_sensitive(false);
    widgets.btn_enable_traceability.set_sensitive(false);
    update_build_status(&widgets.lbl_build_status, "BUILDING...", StatusKind::Active);

    let build_cmd_res = toolchain::builder::get_build_command(&build_sys, &dir);
    let (cmd, args, exec_dir) = match build_cmd_res {
        Ok(tuple) => tuple,
        Err(err) => {
            append_log_text(&widgets.build_log_view, &format!("[ERROR] {}", err));
            update_build_status(&widgets.lbl_build_status, "BUILD FAILED", StatusKind::Error);
            let has_build_system = {
                let st = state.borrow();
                let mut bt = st.build_trace.borrow_mut();
                bt.build_in_progress = false;
                bt.has_build_system
            };
            widgets.btn_build.set_sensitive(has_build_system);
            widgets.btn_build_flash.set_sensitive(has_build_system);
            update_traceability_ui(state, widgets);
            return;
        }
    };

    // Generate build-info header before invoking compiler
    match toolchain::traceability::generate_build_info_header(&dir) {
        Ok(hash) => {
            append_log_text(
                &widgets.build_log_view,
                &format!("[TRACE] Generated Core/Inc/stakhal_build_info.h (STAKHAL_BUILD_HASH: \"{}\")", hash),
            );
        }
        Err(err) => {
            append_log_text(
                &widgets.build_log_view,
                &format!("[TRACE WARNING] Failed to generate build info header: {}", err),
            );
        }
    }

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
                            let has_build_system = {
                                let st = state_timer.borrow();
                                let mut bt = st.build_trace.borrow_mut();
                                bt.build_in_progress = false;
                                bt.has_build_system
                            };
                            widgets_timer.btn_build.set_sensitive(has_build_system);
                            widgets_timer.btn_build_flash.set_sensitive(has_build_system);
                            update_traceability_ui(&state_timer, &widgets_timer);
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
                                let has_build_system = {
                                    let st = state_dlg.borrow();
                                    let mut bt = st.build_trace.borrow_mut();
                                    bt.build_in_progress = false;
                                    bt.has_build_system
                                };
                                widgets_dlg.btn_build.set_sensitive(has_build_system);
                                widgets_dlg.btn_build_flash.set_sensitive(has_build_system);
                                update_traceability_ui(&state_dlg, &widgets_dlg);
                            });
                            dialog.present();
                        } else {
                            update_build_status(&widgets_timer.lbl_build_status, "SUCCESS", StatusKind::Ready);
                            widgets_timer.toast_overlay.add_toast(adw::Toast::new("[OK] Build succeeded"));
                            let has_build_system = {
                                let st = state_timer.borrow();
                                let mut bt = st.build_trace.borrow_mut();
                                bt.build_in_progress = false;
                                bt.has_build_system
                            };
                            widgets_timer.btn_build.set_sensitive(has_build_system);
                            widgets_timer.btn_build_flash.set_sensitive(has_build_system);
                            update_traceability_ui(&state_timer, &widgets_timer);
                        }
                    }
                    toolchain::makefile::ArtifactResolution::NoneFound(expected) => {
                        append_log_text(&widgets_timer.build_log_view, &format!("[ERROR] Build succeeded but target .bin was not found. Expected: {}", expected.display()));
                        update_build_status(&widgets_timer.lbl_build_status, "ARTIFACT MISSING", StatusKind::Error);
                        let has_build_system = {
                            let st = state_timer.borrow();
                            let mut bt = st.build_trace.borrow_mut();
                            bt.build_in_progress = false;
                            bt.has_build_system
                        };
                        widgets_timer.btn_build.set_sensitive(has_build_system);
                        widgets_timer.btn_build_flash.set_sensitive(has_build_system);
                        update_traceability_ui(&state_timer, &widgets_timer);
                    }
                }
            } else {
                let code_str = code.map(|c| c.to_string()).unwrap_or_else(|| "unknown".to_string());
                let action_type = if flash_after_build { "Flashing halted." } else { "Build failed." };
                append_log_text(&widgets_timer.build_log_view, &format!("\n[BUILD FAILED] {} exited with error code {}. {}", cmd, code_str, action_type));
                update_build_status(&widgets_timer.lbl_build_status, "BUILD FAILED", StatusKind::Error);
                let has_build_system = {
                    let st = state_timer.borrow();
                    let mut bt = st.build_trace.borrow_mut();
                    bt.build_in_progress = false;
                    bt.has_build_system
                };
                widgets_timer.btn_build.set_sensitive(has_build_system);
                widgets_timer.btn_build_flash.set_sensitive(has_build_system);
                update_traceability_ui(&state_timer, &widgets_timer);
            }

            return glib::ControlFlow::Break;
        }


        glib::ControlFlow::Continue
    });
}

fn apply_traceability_status_to_label(
    label: &gtk4::Label,
    status: &toolchain::traceability::TraceabilityStatus,
) {
    label.remove_css_class("status-ready");
    label.remove_css_class("status-active");
    label.remove_css_class("status-error");
    label.remove_css_class("status-idle");
    label.remove_css_class("dim-label");

    match status {
        toolchain::traceability::TraceabilityStatus::Unknown => {
            label.set_text("BUILD: Unknown");
            label.add_css_class("dim-label");
            label.set_tooltip_text(Some(
                "Firmware build unknown (not flashed with traceability enabled, or haven't reconnected since)",
            ));
        }
        toolchain::traceability::TraceabilityStatus::Dirty { base_hash } => {
            label.set_text(&format!("BUILD: {}-dirty", base_hash));
            label.add_css_class("status-active");
            label.set_tooltip_text(Some(&format!(
                "Board was flashed from an uncommitted working tree near {} - exact source unknown",
                base_hash
            )));
        }
        toolchain::traceability::TraceabilityStatus::MatchesWorkingTree { hash } => {
            label.set_text(&format!("BUILD: {} (Matches working tree)", hash));
            label.add_css_class("status-ready");
            label.set_tooltip_text(Some("Board matches working tree"));
        }
        toolchain::traceability::TraceabilityStatus::BehindWorkingTree { hash, count } => {
            let commit_str = if *count == 1 {
                "1 commit".to_string()
            } else {
                format!("{} commits", count)
            };
            label.set_text(&format!("BUILD: {} ({} behind)", hash, commit_str));
            label.add_css_class("status-active");
            label.set_tooltip_text(Some(&format!(
                "Board is {} behind working tree",
                commit_str
            )));
        }
        toolchain::traceability::TraceabilityStatus::Diverged { hash } => {
            label.set_text(&format!("BUILD: {} (Diverged)", hash));
            label.add_css_class("status-error");
            label.set_tooltip_text(Some(
                "Board's build doesn't match this branch's history",
            ));
        }
    }
}

pub fn update_traceability_ui(state: &Rc<RefCell<AppState>>, widgets: &Rc<AppWidgets>) {
    let (project_dir, main_c_path, build_in_progress, captured_hash) = {
        let st = state.borrow();
        let proj = st.project.borrow();
        let bt = st.build_trace.borrow();
        (
            proj.project_dir.clone(),
            proj.discovered_main_c.clone(),
            bt.build_in_progress,
            bt.captured_build_hash.clone(),
        )
    };

    let status = match (&project_dir, &captured_hash) {
        (Some(dir), Some(hash)) => {
            toolchain::traceability::compare_build_hash_to_head(dir, hash)
        }
        _ => toolchain::traceability::TraceabilityStatus::Unknown,
    };
    apply_traceability_status_to_label(&widgets.lbl_build_traceability, &status);

    match (project_dir, main_c_path) {
        (Some(dir), Some(main_c)) => {
            let is_git = toolchain::traceability::is_git_repository(&dir);
            let is_enabled = toolchain::traceability::is_traceability_enabled_in_source(&main_c);

            state.borrow().build_trace.borrow_mut().is_traceability_enabled = is_enabled;

            if !is_git {
                widgets.btn_enable_traceability.set_sensitive(false);
                widgets
                    .btn_enable_traceability
                    .set_tooltip_text(Some("Build traceability requires a Git repository"));
            } else if is_enabled {
                widgets.btn_enable_traceability.set_sensitive(false);
                widgets
                    .btn_enable_traceability
                    .set_tooltip_text(Some("Build traceability already enabled in main.c"));
            } else if build_in_progress {
                widgets.btn_enable_traceability.set_sensitive(false);
                widgets
                    .btn_enable_traceability
                    .set_tooltip_text(Some("Build in progress..."));
            } else {
                widgets.btn_enable_traceability.set_sensitive(true);
                widgets.btn_enable_traceability.set_tooltip_text(Some(
                    "Enable build traceability by inserting version banner into main.c",
                ));
            }
        }
        _ => {
            widgets.btn_enable_traceability.set_sensitive(false);
            widgets
                .btn_enable_traceability
                .set_tooltip_text(Some("Load a project to enable build traceability"));
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
                            if let Some(chosen) = probes_clone.get(idx) {
                                append_log_text(&widgets_dlg.build_log_view, &format!("[PROBE] Selected target: {}", chosen.display_label()));
                                run_flash_stage(artifact_clone.clone(), Some(chosen.serial.clone()), &state_dlg, &widgets_dlg, dir_clone.clone());
                                return;
                            }
                        }
                    }
                    append_log_text(&widgets_dlg.build_log_view, "[PROBE] Flashing cancelled by user.");
                    update_build_status(&widgets_dlg.lbl_build_status, "CANCELLED", StatusKind::Idle);
                    let has_build_system = {
                        let st = state_dlg.borrow();
                        let mut bt = st.build_trace.borrow_mut();
                        bt.build_in_progress = false;
                        bt.has_build_system
                    };
                    widgets_dlg.btn_build.set_sensitive(has_build_system);
                    widgets_dlg.btn_build_flash.set_sensitive(has_build_system);
                    update_traceability_ui(&state_dlg, &widgets_dlg);
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
            let has_build_system = {
                let st = state.borrow();
                let mut bt = st.build_trace.borrow_mut();
                bt.build_in_progress = false;
                bt.has_build_system
            };
            widgets.btn_build.set_sensitive(has_build_system);
            widgets.btn_build_flash.set_sensitive(has_build_system);
            update_traceability_ui(state, widgets);
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
        let has_build_system = {
            let st = state.borrow();
            let mut bt = st.build_trace.borrow_mut();
            bt.build_in_progress = false;
            bt.has_build_system
        };
        widgets.btn_build.set_sensitive(has_build_system);
        widgets.btn_build_flash.set_sensitive(has_build_system);
        update_traceability_ui(state, widgets);
        return;
    }

    // Phase 5: Disconnect any active serial session prior to flashing to avoid USB port contention
    let was_serial_connected = state.borrow().serial.borrow().is_serial_connected;
    if was_serial_connected {
        let session = {
            let st = state.borrow();
            let mut ser = st.serial.borrow_mut();
            ser.is_serial_connected = false;
            ser.serial_session.take()
        };
        if let Some(session) = session {
            session
                .tx_cmd
                .send(crate::toolchain::serial::SerialTxCommand::Disconnect)
                .ok();
        }
        widgets.btn_connect_serial.set_label("Connect");
        widgets.btn_connect_serial.remove_css_class("destructive-action");
        widgets.btn_connect_serial.add_css_class("suggested-action");
        update_build_status(&widgets.lbl_serial_status, "DISCONNECTED", StatusKind::Idle);
        widgets.combo_port.set_sensitive(true);
        widgets.combo_baud.set_sensitive(true);
        widgets.btn_refresh_ports.set_sensitive(true);
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
            let has_build_system = {
                let st = state_timer.borrow();
                let mut bt = st.build_trace.borrow_mut();
                bt.build_in_progress = false;
                bt.has_build_system
            };
            widgets_timer.btn_build.set_sensitive(has_build_system);
            widgets_timer.btn_build_flash.set_sensitive(has_build_system);
            update_traceability_ui(&state_timer, &widgets_timer);

            if success {
                append_log_text(&widgets_timer.build_log_view, "\n[FLASH SUCCESS] Firmware written to 0x08000000 and target MCU reset successfully!");
                update_build_status(&widgets_timer.lbl_build_status, "SUCCESS", StatusKind::Ready);
                widgets_timer.toast_overlay.add_toast(adw::Toast::new("[OK] Build and Flash Succeeded!"));

                // Phase 5: Auto-switch to Serial Monitor tab and auto-reconnect
                widgets_timer.stack.set_visible_child_full("serial_monitor", gtk4::StackTransitionType::SlideLeft);
                auto_reconnect_serial_after_flash(&state_timer, &widgets_timer);
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


fn refresh_serial_ports(state: &Rc<RefCell<AppState>>, widgets: &Rc<AppWidgets>) {
    let ports = crate::toolchain::serial::enumerate_serial_ports();
    let is_connected = state.borrow().serial.borrow().is_serial_connected;

    if ports.is_empty() {
        let empty_list = gtk4::StringList::new(&["No Ports Detected"]);
        widgets.combo_port.set_model(Some(&empty_list));
        widgets.combo_port.set_selected(0);
        widgets.combo_port.set_sensitive(false);
        if !is_connected {
            widgets.btn_connect_serial.set_sensitive(false);
        }
        {
            let st = state.borrow();
            let mut ser = st.serial.borrow_mut();
            ser.available_serial_ports = Vec::new();
            ser.selected_serial_port = None;
        }
    } else if ports.len() == 1 {
        let p = &ports[0];
        let single_list = gtk4::StringList::new(&[&p.display_name]);
        widgets.combo_port.set_model(Some(&single_list));
        widgets.combo_port.set_selected(0);
        widgets.combo_port.set_sensitive(!is_connected);
        if !is_connected {
            widgets.btn_connect_serial.set_sensitive(true);
        }
        {
            let st = state.borrow();
            let mut ser = st.serial.borrow_mut();
            ser.selected_serial_port = Some(p.port_name.clone());
            ser.available_serial_ports = ports;
        }
    } else {
        let display_names: Vec<String> = ports.iter().map(|p| p.display_name.clone()).collect();
        let display_refs: Vec<&str> = display_names.iter().map(|s| s.as_str()).collect();
        let list = gtk4::StringList::new(&display_refs);
        widgets.combo_port.set_model(Some(&list));
        widgets.combo_port.set_sensitive(!is_connected);
        if !is_connected {
            widgets.btn_connect_serial.set_sensitive(true);
        }

        let current_sel = state.borrow().serial.borrow().selected_serial_port.clone();
        let mut select_idx = 0;
        if let Some(ref cur) = current_sel {
            if let Some(pos) = ports.iter().position(|p| &p.port_name == cur) {
                select_idx = pos;
            }
        }
        {
            let st = state.borrow();
            let mut ser = st.serial.borrow_mut();
            ser.selected_serial_port = Some(ports[select_idx].port_name.clone());
            ser.available_serial_ports = ports;
        }
        widgets.combo_port.set_selected(select_idx as u32);
    }
}


fn attach_serial_rx_pump(
    event_rx: std::sync::mpsc::Receiver<crate::toolchain::serial::SerialRxEvent>,
    state: &Rc<RefCell<AppState>>,
    widgets: &Rc<AppWidgets>,
) {
    let state_timer = Rc::clone(state);
    let widgets_timer = Rc::clone(widgets);

    glib::timeout_add_local(std::time::Duration::from_millis(20), move || {
        let mut should_continue = true;

        while let Ok(evt) = event_rx.try_recv() {
            match evt {
                crate::toolchain::serial::SerialRxEvent::Connected { port_name, baud_rate } => {
                    append_serial_text(
                        &widgets_timer.serial_log_view,
                        &widgets_timer.serial_scrolled,
                        &format!("\n[SERIAL] Connected to {} at {} baud (8N1).\n", port_name, baud_rate),
                    );
                }
                crate::toolchain::serial::SerialRxEvent::Data(data) => {
                    append_serial_text(
                        &widgets_timer.serial_log_view,
                        &widgets_timer.serial_scrolled,
                        &data,
                    );

                    if let Some((hash, is_dirty)) = toolchain::traceability::parse_build_banner_line(&data) {
                        {
                            let st = state_timer.borrow();
                            let mut bt = st.build_trace.borrow_mut();
                            bt.captured_build_hash = Some(hash.clone());
                            bt.is_captured_hash_dirty = is_dirty;
                        }
                        append_log_text(
                            &widgets_timer.build_log_view,
                            &format!("[TRACE] Captured running build hash from serial: {}", hash),
                        );
                        update_traceability_ui(&state_timer, &widgets_timer);
                    }
                }
                crate::toolchain::serial::SerialRxEvent::Error(err) => {
                    append_serial_text(
                        &widgets_timer.serial_log_view,
                        &widgets_timer.serial_scrolled,
                        &format!("\n[SERIAL ERROR] {}\n", err),
                    );
                }
                crate::toolchain::serial::SerialRxEvent::Disconnected => {
                    append_serial_text(
                        &widgets_timer.serial_log_view,
                        &widgets_timer.serial_scrolled,
                        "\n[SERIAL] Port disconnected.\n",
                    );
                    let has_ports = {
                        let st = state_timer.borrow();
                        let mut ser = st.serial.borrow_mut();
                        ser.is_serial_connected = false;
                        ser.serial_session = None;
                        !ser.available_serial_ports.is_empty()
                    };
                    widgets_timer.btn_connect_serial.set_label("Connect");
                    widgets_timer.btn_connect_serial.remove_css_class("destructive-action");
                    widgets_timer.btn_connect_serial.add_css_class("suggested-action");
                    widgets_timer
                        .btn_connect_serial
                        .set_sensitive(has_ports);

                    update_build_status(&widgets_timer.lbl_serial_status, "DISCONNECTED", StatusKind::Idle);
                    widgets_timer.combo_port.set_sensitive(true);
                    widgets_timer.combo_baud.set_sensitive(true);
                    widgets_timer.btn_refresh_ports.set_sensitive(true);
                    should_continue = false;
                    break;
                }
            }
        }

        if should_continue {
            glib::ControlFlow::Continue
        } else {
            glib::ControlFlow::Break
        }
    });
}

fn auto_reconnect_serial_after_flash(state: &Rc<RefCell<AppState>>, widgets: &Rc<AppWidgets>) {
    let session = {
        let st = state.borrow();
        let mut ser = st.serial.borrow_mut();
        ser.is_serial_connected = false;
        ser.serial_session.take()
    };
    if let Some(session) = session {
        session
            .tx_cmd
            .send(crate::toolchain::serial::SerialTxCommand::Disconnect)
            .ok();
    }


    update_build_status(&widgets.lbl_serial_status, "RECONNECTING...", StatusKind::Active);
    widgets.btn_connect_serial.set_label("Connecting...");
    widgets.btn_connect_serial.set_sensitive(false);
    widgets.combo_port.set_sensitive(false);
    widgets.combo_baud.set_sensitive(false);
    widgets.btn_refresh_ports.set_sensitive(false);

    append_serial_text(
        &widgets.serial_log_view,
        &widgets.serial_scrolled,
        "\n[SERIAL] Flash completed. Waiting for target USB re-enumeration...\n",
    );

    let state_retry = Rc::clone(state);
    let widgets_retry = Rc::clone(widgets);
    let mut attempt = 0;
    const MAX_ATTEMPTS: u32 = 8; // ~2.4s total at 300ms intervals

    glib::timeout_add_local(std::time::Duration::from_millis(300), move || {
        if state_retry.borrow().serial.borrow().is_serial_connected {
            return glib::ControlFlow::Break;
        }

        attempt += 1;

        // Refresh ports on each attempt as device re-enumerates
        refresh_serial_ports(&state_retry, &widgets_retry);

        let (target_port, baud_rate) = {
            let st = state_retry.borrow();
            let ser = st.serial.borrow();
            let port = ser.selected_serial_port.clone();
            let baud = ser.selected_serial_baud;
            (port, baud)
        };

        if let Some(port_name) = target_port {
            match crate::toolchain::serial::spawn_serial_connection(port_name.clone(), baud_rate) {
                Ok((session, event_rx)) => {
                    {
                        let st = state_retry.borrow();
                        let mut ser = st.serial.borrow_mut();
                        ser.is_serial_connected = true;
                        ser.serial_session = Some(session);
                    }

                    widgets_retry.btn_connect_serial.set_label("Disconnect");
                    widgets_retry.btn_connect_serial.remove_css_class("suggested-action");
                    widgets_retry.btn_connect_serial.add_css_class("destructive-action");
                    widgets_retry.btn_connect_serial.set_sensitive(true);
                    update_build_status(&widgets_retry.lbl_serial_status, "CONNECTED", StatusKind::Ready);
                    widgets_retry.combo_port.set_sensitive(false);
                    widgets_retry.combo_baud.set_sensitive(false);
                    widgets_retry.btn_refresh_ports.set_sensitive(false);

                    attach_serial_rx_pump(event_rx, &state_retry, &widgets_retry);

                    append_serial_text(
                        &widgets_retry.serial_log_view,
                        &widgets_retry.serial_scrolled,
                        &format!("[SERIAL] Connected to {} at {} baud (8N1).\n", port_name, baud_rate),
                    );
                    widgets_retry
                        .toast_overlay
                        .add_toast(adw::Toast::new(&format!("Serial connected: {}", port_name)));

                    return glib::ControlFlow::Break;
                }
                Err(_err) => {
                    // Port might still be re-initializing or resetting, continue retrying
                }
            }
        }

        if attempt >= MAX_ATTEMPTS {
            append_serial_text(
                &widgets_retry.serial_log_view,
                &widgets_retry.serial_scrolled,
                "[SERIAL] Auto-reconnect timed out. Click Connect to manually establish connection.\n",
            );
            update_build_status(&widgets_retry.lbl_serial_status, "DISCONNECTED", StatusKind::Idle);
            widgets_retry.btn_connect_serial.set_label("Connect");
            widgets_retry.btn_connect_serial.remove_css_class("destructive-action");
            widgets_retry.btn_connect_serial.add_css_class("suggested-action");
            widgets_retry
                .btn_connect_serial
                .set_sensitive(!state_retry.borrow().serial.borrow().available_serial_ports.is_empty());
            widgets_retry.combo_port.set_sensitive(true);
            widgets_retry.combo_baud.set_sensitive(true);
            widgets_retry.btn_refresh_ports.set_sensitive(true);
            return glib::ControlFlow::Break;
        }

        glib::ControlFlow::Continue
    });
}

fn toggle_serial_connection(state: &Rc<RefCell<AppState>>, widgets: &Rc<AppWidgets>) {
    let is_connected = state.borrow().serial.borrow().is_serial_connected;

    if is_connected {
        let st = state.borrow();
        let ser = st.serial.borrow();
        if let Some(ref session) = ser.serial_session {
            session
                .tx_cmd
                .send(crate::toolchain::serial::SerialTxCommand::Disconnect)
                .ok();
        }
    } else {
        let (port_name, baud_rate) = {
            let st = state.borrow();
            let ser = st.serial.borrow();
            let port = match &ser.selected_serial_port {
                Some(p) => p.clone(),
                None => {
                    widgets
                        .toast_overlay
                        .add_toast(adw::Toast::new("No serial communication port selected"));
                    return;
                }
            };
            let baud = ser.selected_serial_baud;
            (port, baud)
        };

        update_build_status(&widgets.lbl_serial_status, "CONNECTING...", StatusKind::Active);

        match crate::toolchain::serial::spawn_serial_connection(port_name.clone(), baud_rate) {
            Ok((session, event_rx)) => {
                {
                    let st = state.borrow();
                    let mut ser = st.serial.borrow_mut();
                    ser.is_serial_connected = true;
                    ser.serial_session = Some(session);
                }

                widgets.btn_connect_serial.set_label("Disconnect");
                widgets.btn_connect_serial.remove_css_class("suggested-action");
                widgets.btn_connect_serial.add_css_class("destructive-action");
                update_build_status(&widgets.lbl_serial_status, "CONNECTED", StatusKind::Ready);
                widgets.combo_port.set_sensitive(false);
                widgets.combo_baud.set_sensitive(false);
                widgets.btn_refresh_ports.set_sensitive(false);

                attach_serial_rx_pump(event_rx, state, widgets);
            }
            Err(err) => {
                append_serial_text(
                    &widgets.serial_log_view,
                    &widgets.serial_scrolled,
                    &format!("\n[SERIAL ERROR] {}\n", err),
                );
                update_build_status(&widgets.lbl_serial_status, "ERROR", StatusKind::Error);
                widgets
                    .toast_overlay
                    .add_toast(adw::Toast::new(&format!("Serial connection error: {}", err)));
            }
        }
    }
}

fn send_serial_command(text: &str, state: &Rc<RefCell<AppState>>, widgets: &Rc<AppWidgets>) {
    let text = text.trim();
    if text.is_empty() {
        return;
    }

    let tx_sender = {
        let st = state.borrow();
        let mut ser = st.serial.borrow_mut();
        if !ser.is_serial_connected {
            None
        } else {
            if ser.serial_command_history.last().map(|s| s.as_str()) != Some(text) {
                ser.serial_command_history.push(text.to_string());
            }
            ser.serial_history_index = None;
            ser.serial_session.as_ref().map(|s| s.tx_cmd.clone())
        }
    };

    let tx_cmd = match tx_sender {
        Some(tx) => tx,
        None => {
            widgets
                .toast_overlay
                .add_toast(adw::Toast::new("Serial port is not currently connected"));
            return;
        }
    };

    // Echo sent command to serial console
    append_serial_text(
        &widgets.serial_log_view,
        &widgets.serial_scrolled,
        &format!("> {}\n", text),
    );

    let payload = format!("{}\r\n", text);
    tx_cmd
        .send(crate::toolchain::serial::SerialTxCommand::Send(payload))
        .ok();
}


fn clear_box_children(bx: &gtk4::Box) {
    while let Some(child) = bx.first_child() {
        bx.remove(&child);
    }
}

pub(crate) fn update_quick_send_buttons(main_c_path: &Path, state: &Rc<RefCell<AppState>>, widgets: &Rc<AppWidgets>) {
    clear_box_children(&widgets.box_quick_commands);

    let commands = stakhal_core::source::extract_project_command_names(main_c_path);
    if commands.is_empty() {
        return;
    }

    let lbl_quick = gtk4::Label::builder()
        .label("QUICK:")
        .css_classes(vec!["caption".to_string(), "dim-label".to_string(), "data-mono".to_string()])
        .valign(gtk4::Align::Center)
        .build();
    widgets.box_quick_commands.append(&lbl_quick);

    for cmd in commands {
        let btn = gtk4::Button::builder()
            .label(&cmd)
            .css_classes(vec!["stakhal-btn".to_string(), "flat".to_string(), "caption".to_string()])
            .tooltip_text(&format!("Send command: {}", cmd))
            .build();
        btn.set_cursor_from_name(Some("pointer"));

        let state_btn = Rc::clone(state);
        let widgets_btn = Rc::clone(widgets);
        let cmd_clone = cmd.clone();
        btn.connect_clicked(move |_| {
            send_serial_command(&cmd_clone, &state_btn, &widgets_btn);
        });

        widgets.box_quick_commands.append(&btn);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stakhal_core::ir::schema::load_project;

    #[test]
    fn test_serial_monitor_baud_auto_detection() {
        let fixture_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../stakhal-core/tests/fixtures/aa_ns_stm_port");
        let main_c_path = fixture_dir.join("Core/Src/main.c");

        let uart_info = stakhal_core::source::detect_console_uart(&main_c_path)
            .expect("should detect console UART in aa_ns_stm_port");
        assert_eq!(uart_info.uart_instance, "huart2");
        assert_eq!(uart_info.baud_rate, 115200);
    }

    #[test]
    fn test_serial_rx_captures_build_hash() {
        let state = Rc::new(RefCell::new(AppState::default()));
        let bt_cell = Rc::clone(&state.borrow().build_trace);
        assert_eq!(bt_cell.borrow().captured_build_hash, None);
        assert!(!bt_cell.borrow().is_captured_hash_dirty);

        let test_line = "STAKHAL_BUILD: a41f5f7-dirty\r\n";
        if let Some((hash, is_dirty)) = toolchain::traceability::parse_build_banner_line(test_line) {
            let mut bt = bt_cell.borrow_mut();
            bt.captured_build_hash = Some(hash);
            bt.is_captured_hash_dirty = is_dirty;
        }

        assert_eq!(
            bt_cell.borrow().captured_build_hash.as_deref(),
            Some("a41f5f7-dirty")
        );
        assert!(bt_cell.borrow().is_captured_hash_dirty);

        // Test clean hash
        let clean_line = "STAKHAL_BUILD: a41f5f7\r\n";
        if let Some((hash, is_dirty)) = toolchain::traceability::parse_build_banner_line(clean_line) {
            let mut bt = bt_cell.borrow_mut();
            bt.captured_build_hash = Some(hash);
            bt.is_captured_hash_dirty = is_dirty;
        }

        assert_eq!(
            bt_cell.borrow().captured_build_hash.as_deref(),
            Some("a41f5f7")
        );
        assert!(!bt_cell.borrow().is_captured_hash_dirty);
    }

    #[test]
    fn test_reproduce_run_flash_stage_double_borrow() {
        let state = Rc::new(RefCell::new(AppState::default()));
        let finished = Some((true, Some(0)));
        let bt_cell = Rc::clone(&state.borrow().build_trace);

        let panic_res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if let Some((success, _code)) = finished {
                let mut bt = bt_cell.borrow_mut();
                bt.build_in_progress = false;
                let _has_bs = bt.has_build_system;

                if success {
                    // Simulates re-borrowing the same cell while bt is still held alive:
                    let _reborrow = bt_cell.borrow_mut();
                }
            }
        }));

        assert!(
            panic_res.is_err(),
            "Expected double-borrow panic when bt is not dropped before re-borrowing build_trace cell"
        );
    }

    #[test]
    fn test_run_flash_stage_borrow_released_before_reconnect() {
        let state = Rc::new(RefCell::new(AppState::default()));
        let finished = Some((true, Some(0)));
        let bt_cell = Rc::clone(&state.borrow().build_trace);

        let panic_res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if let Some((success, _code)) = finished {
                let has_build_system = {
                    let mut bt = bt_cell.borrow_mut();
                    bt.build_in_progress = false;
                    bt.has_build_system
                };
                let _ = has_build_system;

                if success {
                    // Simulates auto_reconnect_serial_after_flash(&state, ...):
                    let _session = state.borrow().serial.borrow_mut().serial_session.take();
                }
            }
        }));

        assert!(
            panic_res.is_ok(),
            "Expected borrow to be dropped cleanly so auto-reconnect can borrow state mutably"
        );
    }

    #[test]
    fn test_rapid_project_reload_during_active_flash_is_structurally_immune() {
        let state = Rc::new(RefCell::new(AppState::default()));

        // Simulate active background flash holding or updating BuildTraceState
        let bt_cell = Rc::clone(&state.borrow().build_trace);
        let mut bt_guard = bt_cell.borrow_mut();
        bt_guard.build_in_progress = true;

        // While flash has build_trace actively borrowed, user triggers rapid project reload:
        let reload_res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            // 1. Mutate ProjectState
            {
                let proj_cell = Rc::clone(&state.borrow().project);
                let mut proj = proj_cell.borrow_mut();
                proj.project_dir = Some(PathBuf::from("/fake/path"));
            }

            // 2. Mutate DiagramCanvasState via helper
            state.borrow().with_canvas_state_mut(|canvas| {
                canvas.selected_state_machine = 1;
                canvas.diagram_zoom = 1.5;
                canvas.diagram_needs_fit = true;
            });

            // 3. Read back ProjectState
            let loaded_dir = state.borrow().project.borrow().project_dir.clone();
            assert_eq!(loaded_dir, Some(PathBuf::from("/fake/path")));
        }));

        assert!(
            reload_res.is_ok(),
            "Rapid project reload during active flash must succeed without RefCell collision"
        );

        // Flash concludes and releases build_trace
        bt_guard.build_in_progress = false;
        drop(bt_guard);
        assert!(!state.borrow().build_trace.borrow().build_in_progress);
    }

    #[test]
    fn test_successful_flash_serial_auto_reconnect_structural_independence() {
        let state = Rc::new(RefCell::new(AppState::default()));
        let finished = Some((true, Some(0)));
        let bt_cell = Rc::clone(&state.borrow().build_trace);

        // Pre-configure serial state
        state.borrow().serial.borrow_mut().selected_serial_baud = 115200;
        state.borrow().serial.borrow_mut().is_serial_connected = true;

        let panic_res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if let Some((success, _code)) = finished {
                // Simulate holding bt alive across the reconnect boundary (the exact prior bug)
                let mut bt = bt_cell.borrow_mut();
                bt.build_in_progress = false;
                let _has_bs = bt.has_build_system;

                if success {
                    // Under monolithic AppState, this crashed because bt was held alive.
                    // Under the domain split, serial is a separate cell, so this is structurally immune.
                    let ser_cell = Rc::clone(&state.borrow().serial);
                    let mut ser = ser_cell.borrow_mut();
                    ser.is_serial_connected = false;
                    let _session = ser.serial_session.take();
                    ser.selected_serial_port = Some("/dev/ttyACM0".to_string());
                }
            }
        }));

        assert!(
            panic_res.is_ok(),
            "Flash stage triggering serial auto-reconnect must never collide because cells are split"
        );
        assert!(!state.borrow().serial.borrow().is_serial_connected);
        assert_eq!(
            state.borrow().serial.borrow().selected_serial_port.as_deref(),
            Some("/dev/ttyACM0")
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
        // 1. State diagram rendering test
        let surface_sm = gtk4::cairo::ImageSurface::create(gtk4::cairo::Format::ARgb32, 1400, 900).expect("surface create");
        let cr_sm = gtk4::cairo::Context::new(&surface_sm).expect("cr create");
        let state_sm = Rc::new(RefCell::new(AppState::default()));
        let fixture_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../stakhal-core/tests/fixtures/docking_firmware_v2");
        let ioc_path = fixture_dir.join("docking_firmware_v2.ioc");
        let main_c_path = fixture_dir.join("Core/Src/main.c");
        if let Ok(project) = load_project(&ioc_path, &main_c_path) {
            state_sm.borrow().project.borrow_mut().loaded_project = Some(project);
            state_sm.borrow().with_canvas_state_mut(|c| {
                c.selected_state_node = Some("GOING".to_string());
            });
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
            state_pin.borrow().project.borrow_mut().loaded_project = Some(project);
            state_pin.borrow().with_canvas_state_mut(|c| {
                c.hovered_pinout_pin = Some(("CN10".to_string(), 11));
                c.hovered_pinout_mouse = Some((600.0, 300.0));
            });
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
            let st_all = Rc::new(RefCell::new(AppState::default()));
            st_all.borrow().with_canvas_state_mut(|c| {
                c.selected_pinout_module = None;
            });
            st_all.borrow().project.borrow_mut().loaded_project = Some(project.clone());
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
            let st_hatch = Rc::new(RefCell::new(AppState::default()));
            st_hatch.borrow().with_canvas_state_mut(|c| {
                c.selected_pinout_module = Some("hatch".to_string());
                c.hovered_pinout_pin = Some(("CN10".to_string(), 16)); // GRIP_IN1
                c.hovered_pinout_mouse = Some((850.0, 320.0));
            });
            st_hatch.borrow().project.borrow_mut().loaded_project = Some(project.clone());
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
                let st_dock = Rc::new(RefCell::new(AppState::default()));
                st_dock.borrow().with_canvas_state_mut(|c| {
                    c.state_diagram_layout = Some(layout.clone());
                    c.diagram_bounds = (w, h);
                    c.diagram_zoom = 1.0;
                    c.diagram_pan_x = 40.0;
                    c.diagram_pan_y = 40.0;
                    c.selected_state_node = None;
                });
                st_dock.borrow().project.borrow_mut().loaded_project = Some(dock_proj.clone());
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
                let st_dock_sel = Rc::new(RefCell::new(AppState::default()));
                st_dock_sel.borrow().with_canvas_state_mut(|c| {
                    c.state_diagram_layout = Some(layout);
                    c.diagram_bounds = (w, h);
                    c.diagram_zoom = 1.0;
                    c.diagram_pan_x = 40.0;
                    c.diagram_pan_y = 40.0;
                    c.selected_state_node = Some("GOING".to_string());
                });
                st_dock_sel.borrow().project.borrow_mut().loaded_project = Some(dock_proj);
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
                let st_align = Rc::new(RefCell::new(AppState::default()));
                st_align.borrow().with_canvas_state_mut(|c| {
                    c.state_diagram_layout = Some(layout_align.clone());
                    c.diagram_bounds = (w, h);
                    c.diagram_zoom = 1.0;
                    c.diagram_pan_x = 40.0;
                    c.diagram_pan_y = 40.0;
                    c.selected_state_node = None;
                });
                st_align.borrow().project.borrow_mut().loaded_project = Some(project.clone());
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
                let st_align_ret = Rc::new(RefCell::new(AppState::default()));
                st_align_ret.borrow().with_canvas_state_mut(|c| {
                    c.state_diagram_layout = Some(layout_align);
                    c.diagram_bounds = (w, h);
                    c.diagram_zoom = 1.0;
                    c.diagram_pan_x = 40.0;
                    c.diagram_pan_y = 40.0;
                    c.selected_state_node = Some("RETURNING".to_string());
                });
                st_align_ret.borrow().project.borrow_mut().loaded_project = Some(project);
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




