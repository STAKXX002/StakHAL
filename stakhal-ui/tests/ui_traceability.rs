use std::cell::RefCell;
use std::rc::Rc;
use gtk4::gdk;
use gtk4::prelude::*;
use libadwaita as adw;
use stakhal_ui::state::{AppState, AppWidgets};
use stakhal_ui::toolchain;
use stakhal_ui::update_traceability_ui;

#[test]
fn test_traceability_ui_state_and_button_sensitivity() {
    if let Err(err) = gtk4::init() {
        eprintln!("GTK display not available, skipping test: {}", err);
        return;
    }
    if gdk::Display::default().is_none() {
        eprintln!("GDK default display not available, skipping test");
        return;
    }
    let _ = adw::init();

    let temp_dir = std::env::temp_dir().join(format!("stakhal_test_ui_trace_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(temp_dir.join("Core/Src")).unwrap();
    std::fs::create_dir_all(temp_dir.join("Core/Inc")).unwrap();

    let main_c_path = temp_dir.join("Core/Src/main.c");
    let initial_c = r#"/* USER CODE BEGIN Includes */
#include <stdio.h>
/* USER CODE END Includes */

/* USER CODE BEGIN 2 */
printf("BOOT\r\n");
/* USER CODE END 2 */
"#;
    std::fs::write(&main_c_path, initial_c).unwrap();

    let btn_enable = gtk4::Button::new();
    let state = Rc::new(RefCell::new(AppState::default()));
    state.borrow().project.borrow_mut().project_dir = Some(temp_dir.clone());
    state.borrow().project.borrow_mut().discovered_main_c = Some(main_c_path.clone());

    let window = adw::ApplicationWindow::builder().build();
    let stack = gtk4::Stack::new();
    let toast_overlay = adw::ToastOverlay::new();

    let widgets = Rc::new(AppWidgets {
        window,
        stack,
        toast_overlay,
        lbl_discovered_dir: gtk4::Label::new(None),
        lbl_ioc_path: gtk4::Label::new(None),
        lbl_main_c_path: gtk4::Label::new(None),
        btn_load: gtk4::Button::new(),
        btn_build: gtk4::Button::new(),
        btn_build_flash: gtk4::Button::new(),
        btn_enable_traceability: btn_enable.clone(),
        btn_call_graph: gtk4::Button::new(),
        btn_nucleo_pinout: gtk4::Button::new(),
        lbl_project_name: gtk4::Label::new(None),
        lbl_mcu_family: gtk4::Label::new(None),
        lbl_mcu_name: gtk4::Label::new(None),
        lbl_build_traceability: gtk4::Label::new(None),
        area_traceability_verify: gtk4::DrawingArea::new(),
        lbl_periph_header: gtk4::Label::new(None),
        lbl_region_header: gtk4::Label::new(None),
        list_peripherals: gtk4::ListBox::new(),
        list_user_regions: gtk4::ListBox::new(),
        build_log_view: gtk4::TextView::new(),
        lbl_build_status: gtk4::Label::new(None),
        area_flash_verify: gtk4::DrawingArea::new(),
        btn_clear_log: gtk4::Button::new(),
        diagram_drawing_area: gtk4::DrawingArea::new(),
        btn_fit_to_view: gtk4::Button::new(),
        diagram_scrolled: gtk4::ScrolledWindow::new(),
        combo_state_machine: gtk4::DropDown::builder().build(),
        lbl_selected_info: gtk4::Label::new(None),
        pinout_drawing_area: gtk4::DrawingArea::new(),
        _pinout_scrolled: gtk4::ScrolledWindow::new(),
        combo_pinout_module: gtk4::DropDown::builder().build(),
        btn_serial_monitor: gtk4::Button::new(),
        combo_port: gtk4::DropDown::builder().build(),
        btn_refresh_ports: gtk4::Button::new(),
        combo_baud: gtk4::DropDown::builder().build(),
        btn_connect_serial: gtk4::Button::new(),
        lbl_serial_status: gtk4::Label::new(None),
        btn_clear_serial: gtk4::Button::new(),
        serial_log_view: gtk4::TextView::new(),
        serial_scrolled: gtk4::ScrolledWindow::new(),
        entry_command: gtk4::Entry::new(),
        btn_send_command: gtk4::Button::new(),
        box_quick_commands: gtk4::Box::new(gtk4::Orientation::Horizontal, 0),
    });

    // 1. Non-git repo: should be disabled and Unknown status
    update_traceability_ui(&state, &widgets);
    assert!(!widgets.btn_enable_traceability.is_sensitive());
    assert_eq!(widgets.lbl_build_traceability.text(), "BUILD: Unknown");
    assert!(widgets.lbl_build_traceability.has_css_class("dim-label"));

    // 2. Initialize git repository and create a commit
    std::process::Command::new("git")
        .arg("-C")
        .arg(&temp_dir)
        .arg("init")
        .output()
        .unwrap();
    std::process::Command::new("git")
        .arg("-C")
        .arg(&temp_dir)
        .arg("config")
        .arg("user.name")
        .arg("Test")
        .output()
        .unwrap();
    std::process::Command::new("git")
        .arg("-C")
        .arg(&temp_dir)
        .arg("config")
        .arg("user.email")
        .arg("t@example.com")
        .output()
        .unwrap();
    std::process::Command::new("git")
        .arg("-C")
        .arg(&temp_dir)
        .arg("add")
        .arg(".")
        .output()
        .unwrap();
    std::process::Command::new("git")
        .arg("-C")
        .arg(&temp_dir)
        .arg("commit")
        .arg("-m")
        .arg("init")
        .output()
        .unwrap();

    let head_out = std::process::Command::new("git")
        .arg("-C")
        .arg(&temp_dir)
        .arg("rev-parse")
        .arg("--short")
        .arg("HEAD")
        .output()
        .unwrap();
    let head = String::from_utf8_lossy(&head_out.stdout).trim().to_string();

    // Now inside git repo, not yet enabled: should be sensitive
    update_traceability_ui(&state, &widgets);
    assert!(widgets.btn_enable_traceability.is_sensitive());

    // Test matched hash status chip
    state.borrow().build_trace.borrow_mut().captured_build_hash = Some(head.clone());
    update_traceability_ui(&state, &widgets);
    assert_eq!(
        widgets.lbl_build_traceability.text(),
        format!("BUILD: {} (Matches working tree)", head)
    );
    assert!(widgets.lbl_build_traceability.has_css_class("status-ready"));
    assert!(state.borrow().build_trace.borrow().traceability_was_matched);
    assert!(state.borrow().build_trace.borrow().traceability_verify_start.is_some());
    let initial_anim_start = state.borrow().build_trace.borrow().traceability_verify_start.unwrap();

    // Redraw while still matched: verify animation does NOT replay
    update_traceability_ui(&state, &widgets);
    assert_eq!(state.borrow().build_trace.borrow().traceability_verify_start, Some(initial_anim_start));

    // Test dirty hash status chip
    state.borrow().build_trace.borrow_mut().captured_build_hash = Some(format!("{}-dirty", head));
    update_traceability_ui(&state, &widgets);
    assert_eq!(
        widgets.lbl_build_traceability.text(),
        format!("BUILD: {}-dirty", head)
    );
    assert!(widgets.lbl_build_traceability.has_css_class("status-active"));
    assert!(!state.borrow().build_trace.borrow().traceability_was_matched);
    assert!(state.borrow().build_trace.borrow().traceability_verify_start.is_none());

    // Test diverged hash status chip
    state.borrow().build_trace.borrow_mut().captured_build_hash = Some("deadbeef".to_string());
    update_traceability_ui(&state, &widgets);
    assert_eq!(widgets.lbl_build_traceability.text(), "BUILD: deadbeef (Diverged)");
    assert!(widgets.lbl_build_traceability.has_css_class("status-error"));

    // Reset captured hash
    state.borrow().build_trace.borrow_mut().captured_build_hash = None;

    // 3. Insert traceability into source
    let res = toolchain::traceability::insert_traceability_into_source(&main_c_path, &temp_dir);
    assert!(res.is_ok());

    // Once enabled: should be insensitive
    update_traceability_ui(&state, &widgets);
    assert!(!widgets.btn_enable_traceability.is_sensitive());

    // 4. User manually removes line: should re-enable
    std::fs::write(&main_c_path, initial_c).unwrap();
    update_traceability_ui(&state, &widgets);
    assert!(widgets.btn_enable_traceability.is_sensitive());

    // 5. Staggered row reveal helper verification
    let row0 = stakhal_ui::ui::main_panel::create_peripheral_row("USART1", Some("Asynchronous"), 2);
    let row1 = stakhal_ui::ui::main_panel::create_peripheral_row("SPI1", Some("Full-Duplex"), 4);
    stakhal_ui::ui::main_panel::append_staggered_row(&widgets.list_peripherals, &row0, 0, 2);
    stakhal_ui::ui::main_panel::append_staggered_row(&widgets.list_peripherals, &row1, 1, 2);
    assert!(row0.has_css_class("stagger-reveal-row"));
    assert!(row1.has_css_class("stagger-reveal-row"));
    assert!(widgets.list_peripherals.first_child().is_some());

    let _ = std::fs::remove_dir_all(&temp_dir);
}
