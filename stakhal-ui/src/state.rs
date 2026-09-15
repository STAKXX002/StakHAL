use std::path::PathBuf;
use gtk4::prelude::*;
use libadwaita as adw;
use stakhal_core::graph::state_transition::ProjectStateModel;
use stakhal_core::ir::schema::Project;

pub struct AppState {
    pub project_dir: Option<PathBuf>,
    pub discovered_ioc: Option<PathBuf>,
    pub discovered_main_c: Option<PathBuf>,
    pub loaded_project: Option<Project>,

    // State transition diagram state
    pub state_model: Option<ProjectStateModel>,
    pub selected_peripheral: Option<String>,
    pub state_node_positions: std::collections::HashMap<String, (f64, f64)>,
    pub selected_state_node: Option<String>,
    pub hovered_state_node: Option<String>,
    pub dragged_state_node: Option<String>,
    pub diagram_bounds: (i32, i32),
    pub diagram_zoom: f64,
    pub diagram_pan_x: f64,
    pub diagram_pan_y: f64,
    pub _last_mouse_pos: (f64, f64),

    pub drag_start_node_pos: (f64, f64),
    pub drag_start_click_pos: (f64, f64),
    pub drag_start_pan_pos: (f64, f64),

    // Nucleo pinout state
    pub hovered_pinout_pin: Option<(String, u8)>,
    pub hovered_pinout_mouse: Option<(f64, f64)>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            project_dir: None,
            discovered_ioc: None,
            discovered_main_c: None,
            loaded_project: None,
            state_model: None,
            selected_peripheral: None,
            state_node_positions: std::collections::HashMap::new(),
            selected_state_node: None,
            hovered_state_node: None,
            dragged_state_node: None,
            diagram_bounds: (800, 600),
            diagram_zoom: 1.0,
            diagram_pan_x: 0.0,
            diagram_pan_y: 0.0,
            _last_mouse_pos: (400.0, 300.0),
            drag_start_node_pos: (0.0, 0.0),
            drag_start_click_pos: (0.0, 0.0),
            drag_start_pan_pos: (0.0, 0.0),
            hovered_pinout_pin: None,
            hovered_pinout_mouse: None,
        }
    }
}




pub struct AppWidgets {
    pub window: adw::ApplicationWindow,
    pub _stack: gtk4::Stack,
    pub toast_overlay: adw::ToastOverlay,
    pub lbl_discovered_dir: gtk4::Label,
    pub lbl_ioc_path: gtk4::Label,
    pub lbl_main_c_path: gtk4::Label,
    pub btn_load: gtk4::Button,
    pub btn_state_diagram: gtk4::Button,
    pub btn_nucleo_pinout: gtk4::Button,
    pub lbl_project_name: gtk4::Label,
    pub lbl_mcu_family: gtk4::Label,
    pub lbl_mcu_name: gtk4::Label,
    pub lbl_periph_header: gtk4::Label,
    pub lbl_region_header: gtk4::Label,
    pub list_peripherals: gtk4::ListBox,
    pub list_user_regions: gtk4::ListBox,

    // State transition diagram widgets
    pub diagram_drawing_area: gtk4::DrawingArea,
    pub btn_fit_to_view: gtk4::Button,
    pub diagram_scrolled: gtk4::ScrolledWindow,
    pub combo_peripheral: gtk4::DropDown,
    pub lbl_selected_info: gtk4::Label,

    // Nucleo Pinout widgets
    pub pinout_drawing_area: gtk4::DrawingArea,
    pub _pinout_scrolled: gtk4::ScrolledWindow,
}


pub fn create_icon_button(label_text: &str, icon_name: &str, is_suggested: bool) -> gtk4::Button {
    let bx = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .halign(gtk4::Align::Center)
        .valign(gtk4::Align::Center)
        .build();

    let icon = gtk4::Image::from_icon_name(icon_name);
    bx.append(&icon);

    let btn = gtk4::Button::builder()
        .child(&bx)
        .tooltip_text(label_text)
        .css_classes(vec!["stakhal-btn".to_string()])
        .build();

    btn.set_cursor_from_name(Some("pointer"));

    if is_suggested {
        btn.add_css_class("suggested-action");
    } else {
        btn.add_css_class("flat");
    }

    btn
}
