use gtk4::prelude::*;

use crate::state::create_icon_button;

pub struct CallGraphPanelWidgets {
    pub graph_panel_box: gtk4::Box,
    pub btn_graph_back: gtk4::Button,
    pub graph_drawing_area: gtk4::DrawingArea,
}

pub fn build_call_graph_panel() -> CallGraphPanelWidgets {
    let btn_graph_back = create_icon_button("Back to Overview", "go-previous-symbolic", false);

    let lbl_graph_title = gtk4::Label::builder()
        .label("[ CALL GRAPH DIAGRAM ]")
        .halign(gtk4::Align::Start)
        .hexpand(true)
        .css_classes(vec!["title-3".to_string()])
        .build();

    let lbl_graph_hint = gtk4::Label::builder()
        .label("Click node to highlight connections, drag node to move")
        .halign(gtk4::Align::End)
        .css_classes(vec!["dim-label".to_string(), "caption".to_string()])
        .build();

    let graph_header_bar = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(18)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(18)
        .margin_end(18)
        .build();
    graph_header_bar.append(&btn_graph_back);
    graph_header_bar.append(&lbl_graph_title);
    graph_header_bar.append(&lbl_graph_hint);

    let graph_drawing_area = gtk4::DrawingArea::builder()
        .content_width(800)
        .content_height(600)
        .hexpand(true)
        .vexpand(true)
        .build();


    let graph_scrolled = gtk4::ScrolledWindow::builder()
        .child(&graph_drawing_area)
        .hexpand(true)
        .vexpand(true)
        .build();

    let graph_panel_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .build();
    graph_panel_box.append(&graph_header_bar);
    graph_panel_box.append(&graph_scrolled);

    CallGraphPanelWidgets {
        graph_panel_box,
        btn_graph_back,
        graph_drawing_area,
    }
}
