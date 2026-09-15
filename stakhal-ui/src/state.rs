use std::path::PathBuf;
use gtk4::prelude::*;
use libadwaita as adw;
use stakhal_core::ir::schema::Project;

pub struct AppState {
    pub project_dir: Option<PathBuf>,
    pub discovered_ioc: Option<PathBuf>,
    pub discovered_main_c: Option<PathBuf>,
    pub loaded_project: Option<Project>,

    // State machine diagram state
    pub selected_state_machine: usize,
    pub state_diagram_layout: Option<stakhal_core::graph::StateMachineLayout>,
    pub diagram_zoom: f64,
    pub diagram_pan_x: f64,
    pub diagram_pan_y: f64,
    pub diagram_needs_fit: bool,
    pub selected_state_node: Option<String>,
    pub hovered_state_node: Option<String>,
    pub state_node_positions: std::collections::HashMap<String, (f64, f64)>,
    pub diagram_bounds: (i32, i32),

    pub drag_start_click_pos: (f64, f64),
    pub drag_start_pan_pos: (f64, f64),

    // Nucleo pinout state
    pub hovered_pinout_pin: Option<(String, u8)>,
    pub hovered_pinout_mouse: Option<(f64, f64)>,

    // Build & flash state
    pub build_in_progress: bool,
    pub has_makefile: bool,
    pub has_build_system: bool,
    pub detected_build_system: Option<crate::toolchain::builder::BuildSystem>,
    #[allow(dead_code)]
    pub selected_probe: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            project_dir: None,
            discovered_ioc: None,
            discovered_main_c: None,
            loaded_project: None,
            selected_state_machine: 0,
            state_diagram_layout: None,
            diagram_zoom: 0.95,
            diagram_pan_x: 40.0,
            diagram_pan_y: 40.0,
            diagram_needs_fit: true,
            selected_state_node: None,
            hovered_state_node: None,
            state_node_positions: std::collections::HashMap::new(),
            diagram_bounds: (1200, 800),
            drag_start_click_pos: (0.0, 0.0),
            drag_start_pan_pos: (40.0, 40.0),
            hovered_pinout_pin: None,
            hovered_pinout_mouse: None,
            build_in_progress: false,
            has_makefile: false,
            has_build_system: false,
            detected_build_system: None,
            selected_probe: None,
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
    pub btn_call_graph: gtk4::Button,
    pub btn_nucleo_pinout: gtk4::Button,
    pub lbl_project_name: gtk4::Label,
    pub lbl_mcu_family: gtk4::Label,
    pub lbl_mcu_name: gtk4::Label,
    pub lbl_periph_header: gtk4::Label,
    pub lbl_region_header: gtk4::Label,
    pub list_peripherals: gtk4::ListBox,
    pub list_user_regions: gtk4::ListBox,

    // Build & flash widgets
    pub btn_build: gtk4::Button,
    pub btn_build_flash: gtk4::Button,
    pub build_log_view: gtk4::TextView,
    pub lbl_build_status: gtk4::Label,
    pub btn_clear_log: gtk4::Button,

    // State diagram widgets
    pub diagram_drawing_area: gtk4::DrawingArea,
    #[allow(dead_code)]
    pub btn_fit_to_view: gtk4::Button,
    #[allow(dead_code)]
    pub diagram_scrolled: gtk4::ScrolledWindow,
    pub combo_state_machine: gtk4::DropDown,
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
