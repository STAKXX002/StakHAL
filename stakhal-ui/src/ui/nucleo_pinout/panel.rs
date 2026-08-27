use gtk4::prelude::*;
use crate::state::create_icon_button;

pub struct NucleoPinoutPanelWidgets {
    pub pinout_panel_box: gtk4::Box,
    pub btn_pinout_back: gtk4::Button,
    pub pinout_drawing_area: gtk4::DrawingArea,
    pub pinout_scrolled: gtk4::ScrolledWindow,
}

pub fn build_nucleo_pinout_panel() -> NucleoPinoutPanelWidgets {
    let btn_pinout_back = create_icon_button("Back to Overview", "go-previous-symbolic", false);

    let lbl_pinout_title = gtk4::Label::builder()
        .label("[ NUCLEO-F446RE PHYSICAL CONNECTOR PINOUT ]")
        .halign(gtk4::Align::Start)
        .hexpand(true)
        .css_classes(vec!["title-3".to_string()])
        .build();

    let lbl_pinout_hint = gtk4::Label::builder()
        .label("Highlighted pins indicate active signals in loaded project")
        .halign(gtk4::Align::End)
        .css_classes(vec!["dim-label".to_string(), "caption".to_string()])
        .build();

    let pinout_header_bar = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(12)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(18)
        .margin_end(18)
        .build();
    pinout_header_bar.append(&btn_pinout_back);
    pinout_header_bar.append(&lbl_pinout_title);
    pinout_header_bar.append(&lbl_pinout_hint);

    let pinout_drawing_area = gtk4::DrawingArea::builder()
        .content_width(1200)
        .content_height(750)
        .hexpand(true)
        .vexpand(true)
        .build();

    let pinout_scrolled = gtk4::ScrolledWindow::builder()
        .child(&pinout_drawing_area)
        .hexpand(true)
        .vexpand(true)
        .build();

    let pinout_panel_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .build();
    pinout_panel_box.append(&pinout_header_bar);
    pinout_panel_box.append(&pinout_scrolled);

    NucleoPinoutPanelWidgets {
        pinout_panel_box,
        btn_pinout_back,
        pinout_drawing_area,
        pinout_scrolled,
    }
}
