use std::cell::RefCell;
use std::rc::Rc;
use gtk4::{gdk, gio};
use gtk4::prelude::*;
use libadwaita as adw;
pub mod config;
pub mod state;
pub mod toolchain;
pub mod ui;
pub use ui::setup_build_flash::{execute_build_pipeline, update_traceability_ui};
pub use ui::setup_project::{do_load_project, try_discover_folder};
pub use ui::setup_serial::{
    auto_reconnect_serial_after_flash, refresh_serial_ports, send_serial_command,
    update_quick_send_buttons,
};
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

pub fn navigate_stack(stack: &gtk4::Stack, child_name: &str, transition: gtk4::StackTransitionType) {
    let duration = ui::tokens::motion::effective_duration_ms(ui::tokens::motion::DURATION_SHORT_MS) as u32;
    stack.set_transition_duration(duration);
    if duration == 0 {
        stack.set_visible_child_name(child_name);
    } else {
        stack.set_visible_child_full(child_name, transition);
    }
}

use state::{AppState, AppWidgets};
use ui::nucleo_pinout::{build_nucleo_pinout_panel, NucleoPinoutPanelWidgets};
use ui::state_diagram::{build_state_diagram_panel, StateDiagramPanelWidgets};

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

    let initial_transition_duration =
        ui::tokens::motion::effective_duration_ms(ui::tokens::motion::DURATION_SHORT_MS) as u32;
    let stack = gtk4::Stack::builder()
        .transition_type(gtk4::StackTransitionType::SlideLeftRight)
        .transition_duration(initial_transition_duration)
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

    let launch_overlay = gtk4::Overlay::new();
    launch_overlay.set_child(Some(&content_box));
    ui::launch_overlay::setup_launch_overlay(&launch_overlay);

    toast_overlay.set_child(Some(&launch_overlay));

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

    // Connect Diagram and Navigation
    ui::setup_diagram::setup_diagram_and_navigation(
        &state,
        &widgets,
        &btn_diagram_back,
        &btn_pinout_back,
        &btn_serial_back,
    );

    // Connect Project Setup (Browse & Load)
    ui::setup_project::setup_project_handlers(&btn_browse, &state, &widgets);

    // Connect Build & Flash and Traceability
    ui::setup_build_flash::setup_build_flash_handlers(&state, &widgets);

    // Connect Serial Monitor Controls
    ui::setup_serial::setup_serial_handlers(&state, &widgets);

    window.present();

    // Check last_project.json on startup
    ui::setup_project::check_last_project(&state, &widgets);
}






pub(crate) fn clear_box_children(bx: &gtk4::Box) {
    while let Some(child) = bx.first_child() {
        bx.remove(&child);
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
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
            let artifact_dir = std::env::var_os("CARGO_TARGET_DIR")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../target"))
                .join("test-artifacts");
            std::fs::create_dir_all(&artifact_dir).expect("failed to create test-artifacts directory");

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
            let mut f = std::fs::File::create(artifact_dir.join("pinout_all_modules.png")).expect("create pinout_all_modules.png");
            surf_all.write_to_png(&mut f).expect("write pinout_all_modules.png");

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
            let mut f = std::fs::File::create(artifact_dir.join("pinout_module_hatch.png")).expect("create pinout_module_hatch.png");
            surf_hatch.write_to_png(&mut f).expect("write pinout_module_hatch.png");

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
                let mut f = std::fs::File::create(artifact_dir.join("orthogonal_docking_v2.png")).expect("create orthogonal_docking_v2.png");
                surf_dock.write_to_png(&mut f).expect("write orthogonal_docking_v2.png");

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
                let mut f = std::fs::File::create(artifact_dir.join("orthogonal_docking_v2_selected.png")).expect("create orthogonal_docking_v2_selected.png");
                surf_dock_sel.write_to_png(&mut f).expect("write orthogonal_docking_v2_selected.png");
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
                let mut f = std::fs::File::create(artifact_dir.join("orthogonal_aa_ns_align.png")).expect("create orthogonal_aa_ns_align.png");
                surf_align.write_to_png(&mut f).expect("write orthogonal_aa_ns_align.png");

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
                let mut f = std::fs::File::create(artifact_dir.join("orthogonal_aa_ns_align_selected.png")).expect("create orthogonal_aa_ns_align_selected.png");
                surf_align_ret.write_to_png(&mut f).expect("write orthogonal_aa_ns_align_selected.png");
            }

            // Verify all 6 snapshot artifacts were written and are non-empty
            for name in &[
                "pinout_all_modules.png",
                "pinout_module_hatch.png",
                "orthogonal_docking_v2.png",
                "orthogonal_docking_v2_selected.png",
                "orthogonal_aa_ns_align.png",
                "orthogonal_aa_ns_align_selected.png",
            ] {
                let p = artifact_dir.join(name);
                assert!(p.exists(), "Expected render snapshot {} to exist", p.display());
                assert!(
                    std::fs::metadata(&p).map(|m| m.len() > 0).unwrap_or(false),
                    "Expected render snapshot {} to be non-empty",
                    p.display()
                );
            }
        }

        #[test]
        fn test_navigate_stack_duration_tuning() {
            if gtk4::init().is_err() && !gtk4::is_initialized() {
                return;
            }
            let stack = gtk4::Stack::new();
            let label_a = gtk4::Label::new(Some("A"));
            let label_b = gtk4::Label::new(Some("B"));
            stack.add_named(&label_a, Some("a"));
            stack.add_named(&label_b, Some("b"));

            navigate_stack(&stack, "b", gtk4::StackTransitionType::SlideLeft);

            let expected_duration =
                ui::tokens::motion::effective_duration_ms(ui::tokens::motion::DURATION_SHORT_MS) as u32;
            assert_eq!(stack.transition_duration(), expected_duration);
        }
    }
}




