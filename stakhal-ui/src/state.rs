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


#[derive(Default)]
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
    pub collapsed_chains: std::collections::HashSet<String>,
    pub chain_headers: Vec<ChainHeaderLayout>,
    pub graph_bounds: (i32, i32),
    pub dragged_graph_node: Option<String>,

    pub drag_start_node_pos: (f64, f64),
    pub drag_start_click_pos: (f64, f64),
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
