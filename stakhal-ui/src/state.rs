use std::path::PathBuf;
use gtk4::prelude::*;
use libadwaita as adw;
use stakhal_core::graph::ChainHeaderLayout;
use stakhal_core::ir::schema::Project;
use stakhal_core::source::pv_extract::PvDeclaration;

#[derive(Clone, Debug)]
pub struct GeneratedRun {
    pub start_line: usize,
    pub end_line: usize,
    pub is_collapsed: bool,
}


pub struct AppState {
    pub project_dir: Option<PathBuf>,
    pub discovered_ioc: Option<PathBuf>,
    pub discovered_main_c: Option<PathBuf>,
    pub loaded_project: Option<Project>,
    pub active_pv_index: Option<usize>,
    pub active_decl: Option<PvDeclaration>,
    pub active_usage_lines: Vec<usize>,
    pub is_inline_editing: bool,
    pub generated_runs: Vec<GeneratedRun>,
    pub is_generated_hidden: bool,
    pub selected_graph_node: Option<String>,
    pub graph_node_positions: std::collections::HashMap<String, (f64, f64)>,
    pub node_status_colors: std::collections::HashMap<String, (f64, f64, f64)>,
    pub collapsed_chains: std::collections::HashSet<String>,
    pub chain_headers: Vec<ChainHeaderLayout>,
    pub graph_bounds: (i32, i32),
    pub graph_zoom: f64,
    pub graph_pan_x: f64,
    pub graph_pan_y: f64,
    pub last_mouse_pos: (f64, f64),
    pub dragged_graph_node: Option<String>,
    pub hovered_graph_node: Option<String>,

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
            active_pv_index: None,
            active_decl: None,
            active_usage_lines: Vec::new(),
            is_inline_editing: false,
            generated_runs: Vec::new(),
            is_generated_hidden: false,
            selected_graph_node: None,
            graph_node_positions: std::collections::HashMap::new(),
            node_status_colors: std::collections::HashMap::new(),
            collapsed_chains: std::collections::HashSet::new(),
            chain_headers: Vec::new(),
            graph_bounds: (800, 600),
            graph_zoom: 1.0,
            graph_pan_x: 0.0,
            graph_pan_y: 0.0,
            last_mouse_pos: (400.0, 300.0),
            dragged_graph_node: None,
            hovered_graph_node: None,
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
    pub stack: gtk4::Stack,
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
    pub lbl_pv_header: gtk4::Label,
    pub list_peripherals: gtk4::ListBox,
    pub list_user_regions: gtk4::ListBox,
    pub list_pv_variables: gtk4::ListBox,

    // Source view widgets
    pub source_view: sourceview5::View,
    pub source_buffer: sourceview5::Buffer,
    pub lbl_active_pv: gtk4::Label,
    pub btn_toggle_generated: gtk4::Button,
    pub tag_declaration: gtk4::TextTag,
    pub tag_usage: gtk4::TextTag,
    pub tag_generated: gtk4::TextTag,
    pub tag_readonly: gtk4::TextTag,
    pub tag_invisible: gtk4::TextTag,

    // Inline edit bar widgets
    pub inline_edit_bar: gtk4::Box,
    pub lbl_inline_error: gtk4::Label,

    // Call graph widgets
    pub graph_drawing_area: gtk4::DrawingArea,
    pub btn_fit_to_view: gtk4::Button,
    pub graph_scrolled: gtk4::ScrolledWindow,

    // Nucleo Pinout widgets
    pub pinout_drawing_area: gtk4::DrawingArea,
    pub _pinout_scrolled: gtk4::ScrolledWindow,

    // Popover widget
    pub context_menu_popover: gtk4::Popover,
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
