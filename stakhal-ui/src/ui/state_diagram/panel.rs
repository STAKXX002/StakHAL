use gtk4::prelude::*;
use crate::state::create_icon_button;

pub struct StateDiagramPanelWidgets {
    pub diagram_panel_box: gtk4::Box,
    pub btn_diagram_back: gtk4::Button,
    pub btn_fit_to_view: gtk4::Button,
    pub combo_state_machine: gtk4::DropDown,
    pub diagram_drawing_area: gtk4::DrawingArea,
    pub diagram_scrolled: gtk4::ScrolledWindow,
    pub lbl_selected_info: gtk4::Label,
}

pub fn build_state_diagram_panel() -> StateDiagramPanelWidgets {
    let btn_diagram_back = create_icon_button("Back to Overview", "go-previous-symbolic", false);
    let btn_fit_to_view = create_icon_button("Fit to View", "zoom-fit-best-symbolic", false);

    let lbl_diagram_title = gtk4::Label::builder()
        .label("[ STATE MACHINE GRAPH ]")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["title-3".to_string()])
        .build();

    let combo_state_machine = gtk4::DropDown::from_strings(&["NO STATE MACHINES DETECTED"]);
    combo_state_machine.set_cursor_from_name(Some("pointer"));
    combo_state_machine.set_tooltip_text(Some("Select application state machine"));

    let lbl_diagram_hint = gtk4::Label::builder()
        .label("Scroll to zoom | Drag to pan | Click node to inspect | Click canvas to collapse")
        .halign(gtk4::Align::End)
        .hexpand(true)
        .css_classes(vec!["dim-label".to_string(), "caption".to_string()])
        .build();

    let diagram_header_bar = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(12)
        .margin_top(12)
        .margin_bottom(8)
        .margin_start(18)
        .margin_end(18)
        .build();
    diagram_header_bar.append(&btn_diagram_back);
    diagram_header_bar.append(&btn_fit_to_view);
    diagram_header_bar.append(&lbl_diagram_title);
    diagram_header_bar.append(&combo_state_machine);
    diagram_header_bar.append(&lbl_diagram_hint);

    let lbl_selected_info = gtk4::Label::builder()
        .label("Click a state node to inspect full transition paths | Click background to collapse high-fan-in edges.")
        .halign(gtk4::Align::Start)
        .hexpand(true)
        .wrap(true)
        .wrap_mode(gtk4::pango::WrapMode::WordChar)
        .lines(2)
        .ellipsize(gtk4::pango::EllipsizeMode::End)
        .margin_start(18)
        .margin_end(18)
        .margin_bottom(8)
        .css_classes(vec!["dim-label".to_string(), "caption".to_string()])
        .build();

    let diagram_drawing_area = gtk4::DrawingArea::builder()
        .content_width(1200)
        .content_height(800)
        .hexpand(true)
        .vexpand(true)
        .build();

    let diagram_scrolled = gtk4::ScrolledWindow::builder()
        .child(&diagram_drawing_area)
        .hexpand(true)
        .vexpand(true)
        .build();

    let diagram_panel_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .build();
    diagram_panel_box.append(&diagram_header_bar);
    diagram_panel_box.append(&lbl_selected_info);
    diagram_panel_box.append(&diagram_scrolled);

    StateDiagramPanelWidgets {
        diagram_panel_box,
        btn_diagram_back,
        btn_fit_to_view,
        combo_state_machine,
        diagram_drawing_area,
        diagram_scrolled,
        lbl_selected_info,
    }
}
