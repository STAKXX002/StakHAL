use gtk4::prelude::*;
use crate::state::create_icon_button;
use crate::toolchain::serial::COMMON_BAUD_RATES;

pub struct SerialMonitorWidgets {
    pub serial_panel_box: gtk4::Box,
    pub btn_serial_back: gtk4::Button,
    pub combo_port: gtk4::DropDown,
    pub btn_refresh_ports: gtk4::Button,
    pub combo_baud: gtk4::DropDown,
    pub btn_connect: gtk4::Button,
    pub lbl_serial_status: gtk4::Label,
    pub btn_clear_log: gtk4::Button,
    pub serial_log_view: gtk4::TextView,
    pub serial_scrolled: gtk4::ScrolledWindow,
    pub entry_command: gtk4::Entry,
    pub btn_send: gtk4::Button,
    pub box_quick_commands: gtk4::Box,
}

pub fn build_serial_monitor_panel() -> SerialMonitorWidgets {
    let btn_serial_back = create_icon_button("Back to Overview", "go-previous-symbolic", false);

    let lbl_serial_title = gtk4::Label::builder()
        .label("[ SERIAL MONITOR ]")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["title-3".to_string()])
        .build();

    let lbl_port = gtk4::Label::builder()
        .label("PORT:")
        .css_classes(vec!["caption".to_string(), "dim-label".to_string()])
        .build();

    let combo_port = gtk4::DropDown::from_strings(&["No Ports Detected"]);
    combo_port.set_cursor_from_name(Some("pointer"));
    combo_port.set_tooltip_text(Some("Select target serial communication port"));
    combo_port.set_css_classes(&["stakhal-btn", "flat"]);

    let btn_refresh_ports = gtk4::Button::builder()
        .icon_name("view-refresh-symbolic")
        .tooltip_text("Rescan available serial communication ports")
        .css_classes(vec!["flat".to_string()])
        .build();
    btn_refresh_ports.set_cursor_from_name(Some("pointer"));

    let lbl_baud = gtk4::Label::builder()
        .label("BAUD:")
        .css_classes(vec!["caption".to_string(), "dim-label".to_string()])
        .build();

    let baud_strings: Vec<String> = COMMON_BAUD_RATES.iter().map(|b| b.to_string()).collect();
    let baud_strs: Vec<&str> = baud_strings.iter().map(|s| s.as_str()).collect();
    let combo_baud = gtk4::DropDown::from_strings(&baud_strs);
    combo_baud.set_cursor_from_name(Some("pointer"));
    combo_baud.set_tooltip_text(Some("Select UART baud rate (auto-detected from source)"));
    combo_baud.set_css_classes(&["stakhal-btn", "flat"]);

    // Default to 115200 if present
    if let Some(idx) = COMMON_BAUD_RATES.iter().position(|&b| b == 115200) {
        combo_baud.set_selected(idx as u32);
    }

    let btn_connect = gtk4::Button::builder()
        .label("Connect")
        .css_classes(vec!["stakhal-btn".to_string(), "suggested-action".to_string()])
        .tooltip_text("Open serial communication port")
        .build();
    btn_connect.set_cursor_from_name(Some("pointer"));

    let lbl_serial_status = gtk4::Label::builder()
        .label("DISCONNECTED")
        .valign(gtk4::Align::Center)
        .css_classes(vec![
            "card".to_string(),
            "caption".to_string(),
            "data-mono".to_string(),
            "status-idle".to_string(),
        ])
        .build();

    let btn_clear_log = gtk4::Button::builder()
        .label("Clear")
        .css_classes(vec!["flat".to_string(), "caption".to_string()])
        .tooltip_text("Clear serial monitor console logs")
        .build();
    btn_clear_log.set_cursor_from_name(Some("pointer"));

    let serial_header_bar = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(10)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(16)
        .margin_end(16)
        .build();

    serial_header_bar.append(&btn_serial_back);
    serial_header_bar.append(&lbl_serial_title);

    let header_spacer1 = gtk4::Box::builder().hexpand(true).build();
    serial_header_bar.append(&header_spacer1);

    serial_header_bar.append(&lbl_port);
    serial_header_bar.append(&combo_port);
    serial_header_bar.append(&btn_refresh_ports);

    serial_header_bar.append(&lbl_baud);
    serial_header_bar.append(&combo_baud);

    serial_header_bar.append(&btn_connect);
    serial_header_bar.append(&lbl_serial_status);
    serial_header_bar.append(&btn_clear_log);

    let serial_log_view = gtk4::TextView::builder()
        .editable(false)
        .cursor_visible(false)
        .monospace(true)
        .wrap_mode(gtk4::WrapMode::WordChar)
        .top_margin(8)
        .bottom_margin(8)
        .left_margin(8)
        .right_margin(8)
        .build();

    let serial_scrolled = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Automatic)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .hexpand(true)
        .vexpand(true)
        .child(&serial_log_view)
        .css_classes(vec!["card".to_string()])
        .margin_start(16)
        .margin_end(16)
        .build();

    // Command input section (for Phase 4)
    let entry_command = gtk4::Entry::builder()
        .placeholder_text("Enter command to send (e.g. GO, CAL, RET, STOP)...")
        .hexpand(true)
        .css_classes(vec!["data-mono".to_string()])
        .build();

    let btn_send = gtk4::Button::builder()
        .label("Send")
        .css_classes(vec!["stakhal-btn".to_string(), "suggested-action".to_string()])
        .tooltip_text("Send command followed by CRLF (Enter)")
        .build();
    btn_send.set_cursor_from_name(Some("pointer"));

    let input_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .margin_start(16)
        .margin_end(16)
        .margin_top(8)
        .build();
    input_box.append(&entry_command);
    input_box.append(&btn_send);

    let box_quick_commands = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(6)
        .margin_start(16)
        .margin_end(16)
        .margin_top(6)
        .margin_bottom(12)
        .build();

    let serial_panel_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .build();
    serial_panel_box.append(&serial_header_bar);
    serial_panel_box.append(&serial_scrolled);
    serial_panel_box.append(&input_box);
    serial_panel_box.append(&box_quick_commands);

    SerialMonitorWidgets {
        serial_panel_box,
        btn_serial_back,
        combo_port,
        btn_refresh_ports,
        combo_baud,
        btn_connect,
        lbl_serial_status,
        btn_clear_log,
        serial_log_view,
        serial_scrolled,
        entry_command,
        btn_send,
        box_quick_commands,
    }
}
