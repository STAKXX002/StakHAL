use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use gtk4::{gdk, glib, prelude::*};
use libadwaita as adw;

use crate::state::{AppState, AppWidgets};
use crate::toolchain;
use crate::{append_log_text, append_serial_text, clear_box_children, update_build_status, StatusKind};

pub fn refresh_serial_ports(state: &Rc<RefCell<AppState>>, widgets: &Rc<AppWidgets>) {
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

pub fn attach_serial_rx_pump(
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
                        crate::ui::setup_build_flash::update_traceability_ui(&state_timer, &widgets_timer);
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

pub fn auto_reconnect_serial_after_flash(state: &Rc<RefCell<AppState>>, widgets: &Rc<AppWidgets>) {
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

pub fn toggle_serial_connection(state: &Rc<RefCell<AppState>>, widgets: &Rc<AppWidgets>) {
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

pub fn send_serial_command(text: &str, state: &Rc<RefCell<AppState>>, widgets: &Rc<AppWidgets>) {
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

pub fn update_quick_send_buttons(main_c_path: &Path, state: &Rc<RefCell<AppState>>, widgets: &Rc<AppWidgets>) {
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

pub fn setup_serial_handlers(state: &Rc<RefCell<AppState>>, widgets: &Rc<AppWidgets>) {
    let state_ref_ports = Rc::clone(state);
    let widgets_ref_ports = Rc::clone(widgets);
    widgets.btn_refresh_ports.connect_clicked(move |_| {
        refresh_serial_ports(&state_ref_ports, &widgets_ref_ports);
        crate::ui::setup_build_flash::update_traceability_ui(&state_ref_ports, &widgets_ref_ports);
    });

    let state_cp = Rc::clone(state);
    widgets.combo_port.connect_selected_notify(move |cb| {
        let idx = cb.selected() as usize;
        let st = state_cp.borrow();
        let mut ser = st.serial.borrow_mut();
        if idx < ser.available_serial_ports.len() {
            ser.selected_serial_port = Some(ser.available_serial_ports[idx].port_name.clone());
        }
    });

    let state_cb = Rc::clone(state);
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

    let state_conn = Rc::clone(state);
    let widgets_conn = Rc::clone(widgets);
    widgets.btn_connect_serial.connect_clicked(move |_| {
        toggle_serial_connection(&state_conn, &widgets_conn);
    });

    let state_send = Rc::clone(state);
    let widgets_send = Rc::clone(widgets);
    let entry_cmd_clone = widgets.entry_command.clone();
    widgets.btn_send_command.connect_clicked(move |_| {
        let text = entry_cmd_clone.text().to_string();
        send_serial_command(&text, &state_send, &widgets_send);
        entry_cmd_clone.set_text("");
    });

    let state_entry = Rc::clone(state);
    let widgets_entry = Rc::clone(widgets);
    widgets.entry_command.connect_activate(move |entry| {
        let text = entry.text().to_string();
        send_serial_command(&text, &state_entry, &widgets_entry);
        entry.set_text("");
    });

    let state_keys = Rc::clone(state);
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

    refresh_serial_ports(state, widgets);
}
