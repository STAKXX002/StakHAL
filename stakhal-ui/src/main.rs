mod config;

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use gtk4::gdk;
use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
use sourceview5::prelude::*;

use gtk4::cairo;
use stakhal_core::graph::builder::{EdgeType, GraphEdge};
use stakhal_core::ioc::discovery::discover_project_files;
use stakhal_core::ir::schema::{load_project, Project};
use stakhal_core::source::marker_scan::scan_file;
use stakhal_core::source::pv_extract::PvDeclaration;
use stakhal_core::source::render_model::{build_source_render_model, LineTier};
use stakhal_core::source::usage_finder::find_variable_usages;
use stakhal_core::source::writeback::write_region;

use config::{load_app_config, save_app_config};

const APP_ID: &str = "org.stakhal.StakHAL";

#[derive(Clone, Debug)]
struct GeneratedRun {
    start_line: usize,
    end_line: usize,
    is_collapsed: bool,
}



#[derive(Default)]
struct AppState {
    project_dir: Option<PathBuf>,
    discovered_ioc: Option<PathBuf>,
    discovered_main_c: Option<PathBuf>,
    loaded_project: Option<Project>,
    active_pv_index: Option<usize>,
    active_decl: Option<PvDeclaration>,
    active_usage_lines: Vec<usize>,
    is_inline_editing: bool,
    generated_runs: Vec<GeneratedRun>,
    is_generated_hidden: bool,
    selected_graph_node: Option<String>,
    graph_node_positions: std::collections::HashMap<String, (f64, f64)>,
    dragged_graph_node: Option<String>,
    drag_start_node_pos: (f64, f64),
    drag_start_click_pos: (f64, f64),
}

struct AppWidgets {
    window: adw::ApplicationWindow,
    stack: gtk4::Stack,
    toast_overlay: adw::ToastOverlay,
    lbl_discovered_dir: gtk4::Label,
    lbl_ioc_path: gtk4::Label,
    lbl_main_c_path: gtk4::Label,
    btn_load: gtk4::Button,
    btn_call_graph: gtk4::Button,
    lbl_project_name: gtk4::Label,
    lbl_mcu_family: gtk4::Label,
    lbl_mcu_name: gtk4::Label,
    lbl_periph_header: gtk4::Label,
    lbl_region_header: gtk4::Label,
    lbl_pv_header: gtk4::Label,
    list_peripherals: gtk4::ListBox,
    list_user_regions: gtk4::ListBox,
    list_pv_variables: gtk4::ListBox,

    // Source view widgets
    source_view: sourceview5::View,
    source_buffer: sourceview5::Buffer,
    lbl_active_pv: gtk4::Label,
    btn_toggle_generated: gtk4::Button,
    tag_declaration: gtk4::TextTag,
    tag_usage: gtk4::TextTag,
    tag_generated: gtk4::TextTag,
    tag_readonly: gtk4::TextTag,
    tag_invisible: gtk4::TextTag,

    // Inline edit bar widgets
    inline_edit_bar: gtk4::Box,
    lbl_inline_error: gtk4::Label,
    btn_inline_save: gtk4::Button,
    btn_inline_cancel: gtk4::Button,

    // Call graph widgets
    graph_drawing_area: gtk4::DrawingArea,
}

fn main() {
    let app = adw::Application::builder()
        .application_id(APP_ID)
        .build();

    app.connect_activate(build_ui);
    app.run();
}

fn create_icon_button(label_text: &str, icon_name: &str, is_suggested: bool) -> gtk4::Button {
    let bx = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .halign(gtk4::Align::Center)
        .valign(gtk4::Align::Center)
        .build();

    let img = gtk4::Image::from_icon_name(icon_name);
    let lbl = gtk4::Label::new(Some(label_text));

    bx.append(&img);
    bx.append(&lbl);

    let btn = gtk4::Button::builder()
        .child(&bx)
        .build();

    if is_suggested {
        btn.add_css_class("suggested-action");
    } else {
        btn.add_css_class("flat");
    }

    btn
}

fn build_ui(app: &adw::Application) {
    // Force dark mode consistently across all Libadwaita / GTK4 widgets and dialogs
    adw::StyleManager::default().set_color_scheme(adw::ColorScheme::ForceDark);

    // btop-inspired Terminal/TUI Design System CSS Provider
    let css_provider = gtk4::CssProvider::new();
    css_provider.load_from_string(r#"
* {
    font-family: 'DejaVu Sans Mono', 'Liberation Mono', monospace;
    font-size: 13px;
    border-radius: 0px;
    box-shadow: none;
}
window, .background, .main-window {
    background-color: #0a0a0a;
    color: #e0e0e0;
}
headerbar, .topbar {
    background-color: #0a0a0a;
    border-bottom: 1px solid #2a2a2a;
    color: #e0e0e0;
}
@define-color accent_color #ffffff;
@define-color accent_bg_color #1a1a1a;
@define-color accent_fg_color #ffffff;
@define-color accent_fill_color #1a1a1a;
@define-color window_bg_color #0a0a0a;
@define-color window_fg_color #e0e0e0;
@define-color view_bg_color #0a0a0a;
@define-color view_fg_color #e0e0e0;
@define-color card_bg_color #111111;
@define-color card_fg_color #e0e0e0;
@define-color dialog_bg_color #111111;
@define-color popover_bg_color #111111;
.card, .boxed-list, list {
    background-color: #111111;
    border: 1px solid #2a2a2a;
    border-radius: 0px;
}
list > row, row.adw-action-row {
    border-bottom: 1px solid #2a2a2a;
    background-color: #111111;
    color: #e0e0e0;
    padding: 6px 12px;
}
list > row:last-child {
    border-bottom: none;
}
.clickable-row {
    transition: background-color 120ms ease-out, border-color 120ms ease-out, color 120ms ease-out;
}
.clickable-row:hover {
    background-color: #1a1a1a;
}
.clickable-row:active {
    background-color: #262626;
}
.clickable-row:selected {
    background-color: #222222;
    color: #ffffff;
}
button:not(.titlebutton) {
    background-color: #111111;
    color: #e0e0e0;
    border: 1px solid #2a2a2a;
    border-radius: 0px;
    padding: 6px 12px;
    transition: background-color 120ms ease-out, border-color 120ms ease-out, color 120ms ease-out;
}
button:not(.titlebutton):hover {
    background-color: #1a1a1a;
    color: #ffffff;
    border-color: #444444;
}
button:not(.titlebutton):active {
    background-color: #262626;
    border-color: #555555;
}
button.suggested-action:not(.titlebutton) {
    background-color: #ffffff;
    color: #000000;
    font-weight: bold;
    border: 1px solid #ffffff;
    border-radius: 0px;
    transition: background-color 120ms ease-out, border-color 120ms ease-out, color 120ms ease-out;
}
button.suggested-action:not(.titlebutton):hover {
    background-color: #e0e0e0;
    color: #000000;
}
button.suggested-action:not(.titlebutton):active {
    background-color: #cccccc;
    color: #000000;
}
button.suggested-action:not(.titlebutton):disabled {
    background-color: #222222;
    color: #6e6e6e;
    border-color: #2a2a2a;
}
*:focus, button:not(.titlebutton):focus, entry:focus {
    outline-color: #ffffff;
    outline-offset: -1px;
}
.dim-label, .caption, subtitle {
    color: #6e6e6e;
}
.implicit-badge {
    background-color: #facc15;
    color: #000000;
    font-size: 11px;
    font-weight: bold;
    padding: 2px 6px;
    border-radius: 0px;
}
toast {
    background-color: #111111;
    color: #e0e0e0;
    border: 1px solid #2a2a2a;
    border-radius: 0px;
}
.toast-success {
    color: #4ade80;
}
.toast-error {
    color: #f87171;
}
textview, textview text {
    background-color: #0a0a0a;
    color: #e0e0e0;
}
.status-bar-box {
    background-color: #111111;
    border: 1px solid #2a2a2a;
    padding: 8px 14px;
}
.status-divider {
    color: #2a2a2a;
    font-weight: bold;
    margin: 0 8px;
}
button.flat:not(.titlebutton) {
    background-color: transparent;
    border: 1px solid #2a2a2a;
    color: #e0e0e0;
}
button.flat:not(.titlebutton):hover {
    background-color: #1a1a1a;
    border-color: #444444;
    color: #ffffff;
}
button.flat:not(.titlebutton):active {
    background-color: #262626;
    border-color: #555555;
}
"#);
    if let Some(display) = gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &css_provider,
            gtk4::STYLE_PROVIDER_PRIORITY_USER,
        );
    }

    // PAGE 1: Overview Page
    let btn_browse = gtk4::Button::builder()
        .icon_name("folder-open-symbolic")
        .tooltip_text("Browse Project Folder")
        .css_classes(vec!["flat".to_string()])
        .build();

    let btn_load = gtk4::Button::builder()
        .icon_name("system-run-symbolic")
        .tooltip_text("Load Project")
        .sensitive(false)
        .css_classes(vec!["suggested-action".to_string()])
        .build();

    let lbl_discovered_dir = gtk4::Label::builder()
        .label("No folder selected")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["title-4".to_string()])
        .build();

    let lbl_ioc_path = gtk4::Label::builder()
        .label("IOC Path: —")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["dim-label".to_string(), "caption".to_string()])
        .build();

    let lbl_main_c_path = gtk4::Label::builder()
        .label("Main C Path: —")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["dim-label".to_string(), "caption".to_string()])
        .build();

    let path_info_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(6)
        .hexpand(true)
        .build();
    path_info_box.append(&lbl_discovered_dir);
    path_info_box.append(&lbl_ioc_path);
    path_info_box.append(&lbl_main_c_path);

    let btn_call_graph = gtk4::Button::builder()
        .icon_name("network-workgroup-symbolic")
        .tooltip_text("Call Graph")
        .sensitive(false)
        .css_classes(vec!["flat".to_string()])
        .build();

    let toolbar_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(12)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(18)
        .margin_end(18)
        .build();
    toolbar_box.append(&btn_browse);
    toolbar_box.append(&path_info_box);
    toolbar_box.append(&btn_load);
    toolbar_box.append(&btn_call_graph);

    // Compact Status Bar Row (btop info bar pattern)
    let lbl_project_name = gtk4::Label::builder()
        .label("NAME: —")
        .halign(gtk4::Align::Start)
        .build();

    let div_1 = gtk4::Label::builder().label("|").css_classes(vec!["status-divider".to_string()]).build();

    let lbl_mcu_family = gtk4::Label::builder()
        .label("FAMILY: —")
        .halign(gtk4::Align::Start)
        .build();

    let div_2 = gtk4::Label::builder().label("|").css_classes(vec!["status-divider".to_string()]).build();

    let lbl_mcu_name = gtk4::Label::builder()
        .label("PART: —")
        .halign(gtk4::Align::Start)
        .build();

    let status_bar_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(12)
        .margin_bottom(12)
        .margin_start(18)
        .margin_end(18)
        .css_classes(vec!["status-bar-box".to_string()])
        .build();
    status_bar_box.append(&lbl_project_name);
    status_bar_box.append(&div_1);
    status_bar_box.append(&lbl_mcu_family);
    status_bar_box.append(&div_2);
    status_bar_box.append(&lbl_mcu_name);

    // Three Column Setup: Peripherals, User Regions, PV Variables (Balanced weights)
    let lbl_periph_header = gtk4::Label::builder()
        .label("[ ▸ PERIPHERALS ]")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["heading".to_string()])
        .build();

    let list_peripherals = gtk4::ListBox::builder()
        .css_classes(vec!["boxed-list".to_string()])
        .selection_mode(gtk4::SelectionMode::None)
        .build();

    let col_peripherals = create_column_box(&lbl_periph_header, &list_peripherals);

    let lbl_region_header = gtk4::Label::builder()
        .label("[ ▸ USER REGIONS ]")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["heading".to_string()])
        .build();

    let list_user_regions = gtk4::ListBox::builder()
        .css_classes(vec!["boxed-list".to_string()])
        .selection_mode(gtk4::SelectionMode::None)
        .build();

    let col_regions = create_column_box(&lbl_region_header, &list_user_regions);

    let lbl_pv_header = gtk4::Label::builder()
        .label("[ ▸ PV VARIABLES ]")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["heading".to_string()])
        .build();

    let list_pv_variables = gtk4::ListBox::builder()
        .css_classes(vec!["boxed-list".to_string()])
        .selection_mode(gtk4::SelectionMode::Single)
        .build();

    let col_pv = create_column_box(&lbl_pv_header, &list_pv_variables);
    col_pv.set_hexpand(true);

    let columns_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .homogeneous(false)
        .spacing(12)
        .vexpand(true)
        .margin_start(18)
        .margin_end(18)
        .margin_bottom(18)
        .build();
    columns_box.append(&col_peripherals);
    columns_box.append(&col_regions);
    columns_box.append(&col_pv);

    let overview_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .build();
    overview_box.append(&toolbar_box);
    overview_box.append(&status_bar_box);
    overview_box.append(&columns_box);

    // PAGE 2: PV Source Panel
    let btn_back = create_icon_button("Back to Overview", "go-previous-symbolic", false);

    let lbl_active_pv = gtk4::Label::builder()
        .label("PV Variable Source View")
        .halign(gtk4::Align::Start)
        .hexpand(true)
        .css_classes(vec!["title-3".to_string()])
        .build();

    let btn_toggle_generated = gtk4::Button::builder()
        .label("[ Show Generated ]")
        .css_classes(vec!["flat".to_string()])
        .build();

    let source_header_bar = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(18)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(18)
        .margin_end(18)
        .build();
    source_header_bar.append(&btn_back);
    source_header_bar.append(&lbl_active_pv);
    source_header_bar.append(&btn_toggle_generated);

    let source_buffer = sourceview5::Buffer::new(None);
    let scheme_mgr = sourceview5::StyleSchemeManager::default();

    let res_path_1 = PathBuf::from("stakhal-ui/resources");
    let res_path_2 = PathBuf::from("resources");
    if res_path_1.exists() {
        scheme_mgr.append_search_path(&res_path_1.display().to_string());
    }
    if res_path_2.exists() {
        scheme_mgr.append_search_path(&res_path_2.display().to_string());
    }

    let dark_scheme = scheme_mgr
        .scheme("stakhal-dark")
        .or_else(|| {
            eprintln!("Warning: Could not load 'stakhal-dark' style scheme, falling back to system dark scheme.");
            scheme_mgr.scheme("Adwaita-dark")
        })
        .or_else(|| scheme_mgr.scheme("oblivion"))
        .or_else(|| scheme_mgr.scheme("solarized-dark"))
        .or_else(|| scheme_mgr.scheme("classic-dark"));
    if let Some(ref scheme) = dark_scheme {
        source_buffer.set_style_scheme(Some(scheme));
    }

    let source_view = sourceview5::View::with_buffer(&source_buffer);
    source_view.set_show_line_numbers(true);
    source_view.set_editable(false);
    source_view.set_cursor_visible(false);
    source_view.set_monospace(true);

    let tag_declaration = gtk4::TextTag::builder()
        .name("declaration")
        .paragraph_background("rgba(255, 255, 255, 0.14)")
        .build();

    let tag_usage = gtk4::TextTag::builder()
        .name("usage")
        .paragraph_background("rgba(255, 255, 255, 0.05)")
        .build();

    let tag_generated = gtk4::TextTag::builder()
        .name("generated")
        .foreground("#6e6e6e")
        .build();

    let tag_readonly = gtk4::TextTag::builder()
        .name("readonly")
        .editable(false)
        .build();

    let tag_invisible = gtk4::TextTag::builder()
        .name("invisible")
        .invisible(true)
        .build();

    let tag_table = source_buffer.tag_table();
    tag_table.add(&tag_declaration);
    tag_table.add(&tag_usage);
    tag_table.add(&tag_generated);
    tag_table.add(&tag_readonly);
    tag_table.add(&tag_invisible);

    let source_scrolled = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Automatic)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .kinetic_scrolling(true)
        .overlay_scrolling(true)
        .vexpand(true)
        .child(&source_view)
        .build();

    // Inline edit floating action bar
    let lbl_edit_status = gtk4::Label::builder()
        .label("Editing declaration (Enter to Save, Esc to Cancel)")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["body".to_string(), "dim-label".to_string()])
        .build();

    let lbl_inline_error = gtk4::Label::builder()
        .label("")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["error".to_string()])
        .visible(false)
        .build();

    let btn_inline_save = gtk4::Button::builder()
        .label("Save")
        .css_classes(vec!["suggested-action".to_string()])
        .build();

    let btn_inline_cancel = gtk4::Button::builder()
        .label("Cancel")
        .build();

    let bar_inner = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(12)
        .margin_top(6)
        .margin_bottom(6)
        .margin_start(12)
        .margin_end(12)
        .build();
    bar_inner.append(&lbl_edit_status);
    bar_inner.append(&lbl_inline_error);
    bar_inner.append(&btn_inline_cancel);
    bar_inner.append(&btn_inline_save);

    let inline_edit_bar = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .halign(gtk4::Align::End)
        .valign(gtk4::Align::End)
        .margin_bottom(18)
        .margin_end(24)
        .css_classes(vec!["card".to_string()])
        .visible(false)
        .build();
    inline_edit_bar.append(&bar_inner);

    let source_overlay = gtk4::Overlay::builder()
        .child(&source_scrolled)
        .vexpand(true)
        .build();
    source_overlay.add_overlay(&inline_edit_bar);

    let source_panel_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .build();
    source_panel_box.append(&source_header_bar);
    source_panel_box.append(&source_overlay);

    // PAGE 3: Call Graph Panel
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
        .content_width(2000)
        .content_height(1500)
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

    // View Stack with smooth panel-switch transitions (Overview ↔ Source View ↔ Call Graph)
    let stack = gtk4::Stack::builder()
        .transition_type(gtk4::StackTransitionType::SlideLeftRight)
        .transition_duration(220)
        .interpolate_size(true)
        .build();
    stack.add_named(&overview_box, Some("overview"));
    stack.add_named(&source_panel_box, Some("source_view"));
    stack.add_named(&graph_panel_box, Some("call_graph"));
    stack.set_visible_child_name("overview");

    // HeaderBar
    let header_bar = adw::HeaderBar::new();

    // ToastOverlay for transient feedback
    let toast_overlay = adw::ToastOverlay::new();

    // Root Content Layout
    let content_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .build();
    content_box.append(&header_bar);
    content_box.append(&stack);

    toast_overlay.set_child(Some(&content_box));

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("StakHAL - STM32 Project Viewer")
        .default_width(1200)
        .default_height(800)
        .content(&toast_overlay)
        .build();

    let state = Rc::new(RefCell::new(AppState::default()));
    let widgets = Rc::new(AppWidgets {
        window: window.clone(),
        stack,
        toast_overlay,
        lbl_discovered_dir,
        lbl_ioc_path,
        lbl_main_c_path,
        btn_load,
        btn_call_graph,
        lbl_project_name,
        lbl_mcu_family,
        lbl_mcu_name,
        lbl_periph_header,
        lbl_region_header,
        lbl_pv_header,
        list_peripherals,
        list_user_regions,
        list_pv_variables,
        source_view,
        source_buffer,
        lbl_active_pv,
        btn_toggle_generated,
        tag_declaration,
        tag_usage,
        tag_generated,
        tag_readonly,
        tag_invisible,
        inline_edit_bar,
        lbl_inline_error,
        btn_inline_save,
        btn_inline_cancel,
        graph_drawing_area: graph_drawing_area.clone(),
    });

    // Cairo Draw Callback for Unified Draggable Call Graph
    let state_draw = Rc::clone(&state);
    graph_drawing_area.set_draw_func(move |_area, cr, width, height| {
        let mut st = state_draw.borrow_mut();
        let edges = match &st.loaded_project {
            Some(p) => p.call_graph_edges.clone(),
            None => return,
        };

        if edges.is_empty() {
            return;
        }

        if st.graph_node_positions.is_empty() {
            st.graph_node_positions = compute_initial_graph_layout(&edges);
        }

        let selected_node = st.selected_graph_node.clone();
        let positions = st.graph_node_positions.clone();
        drop(st);

        let canvas_w = width as f64;
        let canvas_h = height as f64;

        // Background (#0a0a0a)
        cr.set_source_rgb(0.04, 0.04, 0.04);
        cr.rectangle(0.0, 0.0, canvas_w, canvas_h);
        let _ = cr.fill();

        // Canvas Header
        cr.select_font_face("monospace", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
        cr.set_font_size(13.0);
        cr.set_source_rgb(0.43, 0.43, 0.43);
        let _ = cr.move_to(20.0, 30.0);
        let _ = cr.show_text("[ STAKHAL CALL GRAPH CANVAS (DRAGGABLE) ]");

        // Highlights computation
        let (highlighted_nodes, highlighted_edges) = if let Some(ref sel) = selected_node {
            let mut connected_n = std::collections::HashSet::new();
            let mut connected_e = std::collections::HashSet::new();
            connected_n.insert(sel.clone());

            for (idx, e) in edges.iter().enumerate() {
                if e.from == *sel || e.to == *sel {
                    connected_e.insert(idx);
                    connected_n.insert(e.from.clone());
                    connected_n.insert(e.to.clone());
                }
            }
            (Some(connected_n), Some(connected_e))
        } else {
            (None, None)
        };

        // Draw Edges with Directional Arrowheads attached to node bounding boxes
        for (idx, e) in edges.iter().enumerate() {
            let from_pos = positions.get(&e.from);
            let to_pos = positions.get(&e.to);

            if let (Some(&(fx, fy)), Some(&(tx, ty))) = (from_pos, to_pos) {
                let fw = (e.from.len() as f64 * 8.5 + 28.0).max(110.0);
                let fh = 34.0;
                let tw = (e.to.len() as f64 * 8.5 + 28.0).max(110.0);
                let th = 34.0;

                let fc = (fx + fw / 2.0, fy + fh / 2.0);
                let tc = (tx + tw / 2.0, ty + th / 2.0);

                let (sx, sy) = get_rect_ray_intersection(fx, fy, fw, fh, tc.0, tc.1);
                let (ex, ey) = get_rect_ray_intersection(tx, ty, tw, th, fc.0, fc.1);

                let is_hl = match &highlighted_edges {
                    Some(hl_set) => hl_set.contains(&idx),
                    None => false,
                };
                let is_dimmed = highlighted_edges.is_some() && !is_hl;

                if is_hl {
                    cr.set_source_rgb(1.0, 1.0, 1.0);
                    cr.set_line_width(2.0);
                } else if is_dimmed {
                    cr.set_source_rgb(0.16, 0.16, 0.16);
                    cr.set_line_width(1.0);
                } else {
                    cr.set_source_rgb(0.43, 0.43, 0.43);
                    cr.set_line_width(1.5);
                }

                let _ = cr.move_to(sx, sy);
                let _ = cr.line_to(ex, ey);
                let _ = cr.stroke();

                let angle = (ey - sy).atan2(ex - sx);
                let arrow_len = if is_hl { 10.0 } else { 8.0 };
                let arrow_angle = 0.45;

                let x1 = ex - arrow_len * (angle - arrow_angle).cos();
                let y1 = ey - arrow_len * (angle - arrow_angle).sin();
                let x2 = ex - arrow_len * (angle + arrow_angle).cos();
                let y2 = ey - arrow_len * (angle + arrow_angle).sin();

                let _ = cr.move_to(ex, ey);
                let _ = cr.line_to(x1, y1);
                let _ = cr.line_to(x2, y2);
                let _ = cr.close_path();
                let _ = cr.fill();
            }
        }

        // Draw Nodes
        for (n_id, &(n_x, n_y)) in &positions {
            let n_w = (n_id.len() as f64 * 8.5 + 28.0).max(110.0);
            let n_h = 34.0;

            let is_selected = selected_node.as_deref() == Some(n_id.as_str());
            let is_connected = match &highlighted_nodes {
                Some(set) => set.contains(n_id),
                None => false,
            };
            let is_dimmed = highlighted_nodes.is_some() && !is_connected;

            if is_selected {
                cr.set_source_rgb(0.13, 0.13, 0.13);
                let _ = cr.rectangle(n_x, n_y, n_w, n_h);
                let _ = cr.fill_preserve();
                cr.set_source_rgb(1.0, 1.0, 1.0);
                cr.set_line_width(2.0);
                let _ = cr.stroke();

                cr.set_source_rgb(1.0, 1.0, 1.0);
            } else if is_connected {
                cr.set_source_rgb(0.10, 0.10, 0.10);
                let _ = cr.rectangle(n_x, n_y, n_w, n_h);
                let _ = cr.fill_preserve();
                cr.set_source_rgb(0.67, 0.67, 0.67);
                cr.set_line_width(1.5);
                let _ = cr.stroke();

                cr.set_source_rgb(1.0, 1.0, 1.0);
            } else if is_dimmed {
                cr.set_source_rgb(0.05, 0.05, 0.05);
                let _ = cr.rectangle(n_x, n_y, n_w, n_h);
                let _ = cr.fill_preserve();
                cr.set_source_rgb(0.12, 0.12, 0.12);
                cr.set_line_width(1.0);
                let _ = cr.stroke();

                cr.set_source_rgb(0.33, 0.33, 0.33);
            } else {
                cr.set_source_rgb(0.07, 0.07, 0.07);
                let _ = cr.rectangle(n_x, n_y, n_w, n_h);
                let _ = cr.fill_preserve();
                cr.set_source_rgb(0.16, 0.16, 0.16);
                cr.set_line_width(1.0);
                let _ = cr.stroke();

                cr.set_source_rgb(0.88, 0.88, 0.88);
            }

            cr.select_font_face(
                "monospace",
                cairo::FontSlant::Normal,
                if is_selected {
                    cairo::FontWeight::Bold
                } else {
                    cairo::FontWeight::Normal
                },
            );
            cr.set_font_size(11.5);
            if let Ok(extents) = cr.text_extents(n_id) {
                let tx = n_x + (n_w - extents.width()) / 2.0;
                let ty = n_y + (n_h + extents.height()) / 2.0 - 2.0;
                let _ = cr.move_to(tx, ty);
                let _ = cr.show_text(n_id);
            }
        }
    });

    // Gesture controller for node drag & click interaction
    let gesture_drag = gtk4::GestureDrag::new();
    gesture_drag.set_button(1);
    let state_drag_begin = Rc::clone(&state);

    gesture_drag.connect_drag_begin(move |_, start_x, start_y| {
        let mut st = state_drag_begin.borrow_mut();
        st.drag_start_click_pos = (start_x, start_y);
        st.dragged_graph_node = None;

        let mut clicked_id = None;
        let mut start_pos = (0.0, 0.0);

        for (id, &(nx, ny)) in &st.graph_node_positions {
            let nw = (id.len() as f64 * 8.5 + 28.0).max(110.0);
            let nh = 34.0;
            if start_x >= nx && start_x <= nx + nw && start_y >= ny && start_y <= ny + nh {
                clicked_id = Some(id.clone());
                start_pos = (nx, ny);
                break;
            }
        }

        if let Some(id) = clicked_id {
            st.dragged_graph_node = Some(id);
            st.drag_start_node_pos = start_pos;
        }
    });

    let state_drag_update = Rc::clone(&state);
    let area_drag_update = graph_drawing_area.clone();
    gesture_drag.connect_drag_update(move |_, offset_x, offset_y| {
        let mut st = state_drag_update.borrow_mut();
        if let Some(ref node_id) = st.dragged_graph_node.clone() {
            let (snx, sny) = st.drag_start_node_pos;
            let new_x = (snx + offset_x).max(0.0);
            let new_y = (sny + offset_y).max(0.0);
            st.graph_node_positions.insert(node_id.clone(), (new_x, new_y));
            drop(st);
            area_drag_update.queue_draw();
        }
    });

    let state_drag_end = Rc::clone(&state);
    let area_drag_end = graph_drawing_area.clone();
    gesture_drag.connect_drag_end(move |_, offset_x, offset_y| {
        let dist = offset_x.hypot(offset_y);
        let mut st = state_drag_end.borrow_mut();

        if dist < 5.0 {
            let (cx, cy) = st.drag_start_click_pos;
            let mut clicked_id = None;
            for (id, &(nx, ny)) in &st.graph_node_positions {
                let nw = (id.len() as f64 * 8.5 + 28.0).max(110.0);
                let nh = 34.0;
                if cx >= nx && cx <= nx + nw && cy >= ny && cy <= ny + nh {
                    clicked_id = Some(id.clone());
                    break;
                }
            }

            if let Some(id) = clicked_id {
                if st.selected_graph_node.as_deref() == Some(&id) {
                    st.selected_graph_node = None;
                } else {
                    st.selected_graph_node = Some(id);
                }
            } else {
                st.selected_graph_node = None;
            }
        }

        st.dragged_graph_node = None;
        drop(st);
        area_drag_end.queue_draw();
    });

    graph_drawing_area.add_controller(gesture_drag);

    // Check last_project.json on startup
    let config = load_app_config();
    if let Some(dir_str) = config.project_dir {
        let path = PathBuf::from(dir_str);
        if path.exists() {
            try_discover_folder(&path, &state, &widgets);
        }
    }

    // Connect Browse Button Callback
    let state_browse = Rc::clone(&state);
    let widgets_browse = Rc::clone(&widgets);
    btn_browse.connect_clicked(move |_| {
        let dialog = gtk4::FileDialog::builder()
            .title("Select STM32 Project Directory")
            .build();

        let state = Rc::clone(&state_browse);
        let widgets = Rc::clone(&widgets_browse);
        let parent_win = widgets.window.clone();
        dialog.select_folder(
            Some(&parent_win),
            None::<&gio::Cancellable>,
            move |result| {
                if let Ok(file) = result {
                    if let Some(path) = file.path() {
                        try_discover_folder(&path, &state, &widgets);
                    }
                }
            },
        );
    });

    // Connect Load Project Button Callback
    let state_load = Rc::clone(&state);
    let widgets_load = Rc::clone(&widgets);
    widgets.btn_load.connect_clicked(move |_| {
        do_load_project(&state_load, &widgets_load);
    });

    // Connect Call Graph navigation buttons
    let widgets_graph_open = Rc::clone(&widgets);
    widgets.btn_call_graph.connect_clicked(move |_| {
        widgets_graph_open.stack.set_visible_child_full("call_graph", gtk4::StackTransitionType::SlideLeft);
    });

    let widgets_graph_back = Rc::clone(&widgets);
    btn_graph_back.connect_clicked(move |_| {
        widgets_graph_back.stack.set_visible_child_full("overview", gtk4::StackTransitionType::SlideRight);
    });

    // Connect Back Button Callback
    let widgets_back = Rc::clone(&widgets);
    btn_back.connect_clicked(move |_| {
        widgets_back.stack.set_visible_child_full("overview", gtk4::StackTransitionType::SlideRight);
    });

    // Connect PV Row Click Callback
    let state_pv_click = Rc::clone(&state);
    let widgets_pv_click = Rc::clone(&widgets);
    widgets.list_pv_variables.connect_row_activated(move |_, row| {
        let idx = row.index() as usize;
        open_pv_source_view(idx, &state_pv_click, &widgets_pv_click);
    });

    // Connect Global Toggle Generated Code Button Callback
    let state_toggle = Rc::clone(&state);
    let widgets_toggle = Rc::clone(&widgets);
    widgets.btn_toggle_generated.connect_clicked(move |_| {
        toggle_all_generated_runs(&state_toggle, &widgets_toggle);
    });

    // Connect Inline Edit Action Buttons
    let state_save_btn = Rc::clone(&state);
    let widgets_save_btn = Rc::clone(&widgets);
    widgets.btn_inline_save.connect_clicked(move |_| {
        save_inline_declaration_edit(&state_save_btn, &widgets_save_btn);
    });

    let state_cancel_btn = Rc::clone(&state);
    let widgets_cancel_btn = Rc::clone(&widgets);
    widgets.btn_inline_cancel.connect_clicked(move |_| {
        cancel_inline_declaration_edit(&state_cancel_btn, &widgets_cancel_btn);
    });

    // Key controller for Enter/Escape during inline editing
    let key_controller = gtk4::EventControllerKey::new();
    let state_key = Rc::clone(&state);
    let widgets_key = Rc::clone(&widgets);
    key_controller.connect_key_pressed(move |_, key, _code, _state| {
        let is_editing = state_key.borrow().is_inline_editing;
        if !is_editing {
            return glib::Propagation::Proceed;
        }

        if key == gdk::Key::Return || key == gdk::Key::KP_Enter {
            save_inline_declaration_edit(&state_key, &widgets_key);
            glib::Propagation::Stop
        } else if key == gdk::Key::Escape {
            cancel_inline_declaration_edit(&state_key, &widgets_key);
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });
    widgets.source_view.add_controller(key_controller);

    // Connect Click Gesture on GtkSourceView
    let gesture = gtk4::GestureClick::new();
    let state_source_click = Rc::clone(&state);
    let widgets_source_click = Rc::clone(&widgets);
    gesture.connect_pressed(move |_g, _n_press, x, y| {
        let widgets = &widgets_source_click;
        let st = state_source_click.borrow();
        let decl = match &st.active_decl {
            Some(d) => d.clone(),
            None => return,
        };

        let (buffer_x, buffer_y) = widgets.source_view.window_to_buffer_coords(
            gtk4::TextWindowType::Text,
            x as i32,
            y as i32,
        );

        if let Some(iter) = widgets.source_view.iter_at_location(buffer_x, buffer_y) {
            let clicked_line_1based = (iter.line() + 1) as usize;

            if clicked_line_1based == decl.line {
                if !st.is_inline_editing {
                    drop(st);
                    enter_inline_edit_mode(&state_source_click, &widgets_source_click);
                }
            } else if st.active_usage_lines.contains(&clicked_line_1based) {
                let mut scroll_iter = iter;
                widgets.source_view.scroll_to_iter(&mut scroll_iter, 0.1, true, 0.0, 0.5);
            }
        }
    });
    widgets.source_view.add_controller(gesture);

    // Custom Right-Click Context Menu for PV Source Panel
    let right_click_gesture = gtk4::GestureClick::new();
    right_click_gesture.set_button(3);
    let state_right_click = Rc::clone(&state);
    let widgets_right_click = Rc::clone(&widgets);

    right_click_gesture.connect_pressed(move |g, _n_press, x, y| {
        g.set_state(gtk4::EventSequenceState::Claimed);
        let widgets = &widgets_right_click;
        let st = state_right_click.borrow();

        let (buffer_x, buffer_y) = widgets.source_view.window_to_buffer_coords(
            gtk4::TextWindowType::Text,
            x as i32,
            y as i32,
        );

        let is_decl_line = if let Some(iter) = widgets.source_view.iter_at_location(buffer_x, buffer_y) {
            let clicked_line_1based = (iter.line() + 1) as usize;
            if let Some(ref decl) = st.active_decl {
                clicked_line_1based == decl.line
            } else {
                false
            }
        } else {
            false
        };
        drop(st);

        let popover = gtk4::Popover::builder()
            .autohide(true)
            .build();

        let menu_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(4)
            .margin_top(4)
            .margin_bottom(4)
            .margin_start(4)
            .margin_end(4)
            .build();

        let btn_copy = gtk4::Button::builder()
            .label("Copy")
            .icon_name("edit-copy-symbolic")
            .halign(gtk4::Align::Fill)
            .css_classes(vec!["flat".to_string()])
            .build();

        let popover_clone = popover.clone();
        let widgets_copy = Rc::clone(&widgets);
        btn_copy.connect_clicked(move |_| {
            let clipboard = widgets_copy.source_view.display().clipboard();
            widgets_copy.source_buffer.copy_clipboard(&clipboard);
            popover_clone.popdown();
        });
        menu_box.append(&btn_copy);

        if is_decl_line {
            let btn_edit = gtk4::Button::builder()
                .label("Edit Declaration")
                .icon_name("document-edit-symbolic")
                .halign(gtk4::Align::Fill)
                .css_classes(vec!["flat".to_string()])
                .build();

            let popover_edit_clone = popover.clone();
            let state_edit = Rc::clone(&state_right_click);
            let widgets_edit = Rc::clone(&widgets);
            btn_edit.connect_clicked(move |_| {
                popover_edit_clone.popdown();
                enter_inline_edit_mode(&state_edit, &widgets_edit);
            });
            menu_box.append(&btn_edit);
        }

        popover.set_child(Some(&menu_box));
        let rect = gdk::Rectangle::new(x as i32, y as i32, 1, 1);
        popover.set_parent(&widgets.source_view);
        popover.set_pointing_to(Some(&rect));
        popover.popup();
    });
    widgets.source_view.add_controller(right_click_gesture);

    window.present();
}

fn create_column_box(header_label: &gtk4::Label, list_box: &gtk4::ListBox) -> gtk4::Box {
    let scrolled = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .vexpand(true)
        .child(list_box)
        .build();

    let col_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(6)
        .hexpand(true)
        .vexpand(true)
        .build();

    col_box.append(header_label);
    col_box.append(&scrolled);
    col_box
}

fn try_discover_folder(dir: &Path, state: &Rc<RefCell<AppState>>, widgets: &Rc<AppWidgets>) {
    let mut st = state.borrow_mut();
    st.project_dir = Some(dir.to_path_buf());
    widgets.lbl_discovered_dir.set_text(&dir.display().to_string());

    match discover_project_files(dir) {
        Ok((ioc_path, main_c_path)) => {
            widgets
                .lbl_ioc_path
                .set_text(&format!("IOC: {}", ioc_path.display()));
            widgets
                .lbl_main_c_path
                .set_text(&format!("Main C: {}", main_c_path.display()));

            st.discovered_ioc = Some(ioc_path);
            st.discovered_main_c = Some(main_c_path);
            widgets.btn_load.set_sensitive(true);
        }
        Err(err) => {
            widgets.toast_overlay.add_toast(adw::Toast::new(&format!("Discovery Error: {}", err)));
            widgets.lbl_ioc_path.set_text("IOC Path: —");
            widgets.lbl_main_c_path.set_text("Main C Path: —");
            st.discovered_ioc = None;
            st.discovered_main_c = None;
            widgets.btn_load.set_sensitive(false);
        }
    }
}

fn get_rect_ray_intersection(
    rect_x: f64,
    rect_y: f64,
    rect_w: f64,
    rect_h: f64,
    target_x: f64,
    target_y: f64,
) -> (f64, f64) {
    let cx = rect_x + rect_w / 2.0;
    let cy = rect_y + rect_h / 2.0;
    let dx = target_x - cx;
    let dy = target_y - cy;

    if dx == 0.0 && dy == 0.0 {
        return (cx, cy);
    }

    let scale_x = if dx != 0.0 {
        (rect_w / 2.0) / dx.abs()
    } else {
        f64::INFINITY
    };
    let scale_y = if dy != 0.0 {
        (rect_h / 2.0) / dy.abs()
    } else {
        f64::INFINITY
    };
    let scale = scale_x.min(scale_y);

    (cx + dx * scale, cy + dy * scale)
}

fn compute_initial_graph_layout(
    edges: &[GraphEdge],
) -> std::collections::HashMap<String, (f64, f64)> {
    let mut map = std::collections::HashMap::new();

    let max_row_w = 720.0;

    // 1. INITIALIZATION SECTION
    let init_edges: Vec<&GraphEdge> = edges
        .iter()
        .filter(|e| e.edge_type == EdgeType::Init)
        .collect();

    let mut init_bottom_y = 160.0;

    if !init_edges.is_empty() {
        let mut target_nodes: Vec<String> = init_edges.iter().map(|e| e.to.clone()).collect();
        target_nodes.sort();
        target_nodes.dedup();

        let spacing_x = 20.0;
        let row_height = 55.0;
        let mut rows: Vec<Vec<(String, f64)>> = Vec::new();

        let mut current_row: Vec<(String, f64)> = Vec::new();
        let mut current_row_w = 0.0;

        for target in &target_nodes {
            let w = (target.len() as f64 * 8.5 + 28.0).max(110.0);
            if !current_row.is_empty()
                && (current_row_w + spacing_x + w > max_row_w || current_row.len() >= 5)
            {
                rows.push(current_row);
                current_row = Vec::new();
                current_row_w = 0.0;
            }
            current_row_w += if current_row.is_empty() {
                w
            } else {
                spacing_x + w
            };
            current_row.push((target.clone(), w));
        }
        if !current_row.is_empty() {
            rows.push(current_row);
        }

        let first_row_w = if let Some(r0) = rows.first() {
            r0.iter().map(|(_, w)| w).sum::<f64>()
                + (r0.len().saturating_sub(1) as f64 * spacing_x)
        } else {
            110.0
        };

        let main_w = 110.0;
        let main_x = (40.0 + (first_row_w / 2.0) - (main_w / 2.0)).max(40.0);
        map.insert("main".to_string(), (main_x, 50.0));

        let mut row_start_y = 130.0;
        for row in rows {
            let mut curr_x = 40.0;
            for (id, w) in row {
                map.insert(id, (curr_x, row_start_y));
                curr_x += w + spacing_x;
            }
            row_start_y += row_height;
        }
        init_bottom_y = row_start_y;
    }

    // 2. INTERRUPT CHAINS SECTION (Stacked vertically, left-aligned)
    let irq_entry_edges: Vec<&GraphEdge> = edges
        .iter()
        .filter(|e| e.edge_type == EdgeType::IrqEntry)
        .collect();

    let mut current_chain_y = init_bottom_y + 45.0;

    for irq_edge in &irq_entry_edges {
        let handler_id = irq_edge.from.clone();
        let dispatch_id = irq_edge.to.clone();

        let override_targets: Vec<String> = edges
            .iter()
            .filter(|e| e.edge_type == EdgeType::WeakOverride && e.from == dispatch_id)
            .map(|e| e.to.clone())
            .collect();

        let handler_w = (handler_id.len() as f64 * 8.5 + 28.0).max(130.0);
        let dispatch_w = (dispatch_id.len() as f64 * 8.5 + 28.0).max(130.0);

        let chain_x = 40.0;
        let chain_y_l1 = current_chain_y;
        let chain_y_l2 = chain_y_l1 + 65.0;
        let chain_y_l3 = chain_y_l2 + 65.0;

        let mut override_w_sum = 0.0;
        let mut override_nodes_info = Vec::new();
        let mut curr_ov_x = chain_x;

        for ov in &override_targets {
            let w = (ov.len() as f64 * 8.5 + 28.0).max(130.0);
            override_nodes_info.push((ov.clone(), curr_ov_x, w));
            curr_ov_x += w + 20.0;
            override_w_sum += w + 20.0;
        }

        let chain_max_width = handler_w.max(dispatch_w).max(override_w_sum).max(160.0);
        let center_x = chain_x + (chain_max_width / 2.0);

        map.insert(handler_id, (center_x - (handler_w / 2.0), chain_y_l1));
        map.insert(dispatch_id, (center_x - (dispatch_w / 2.0), chain_y_l2));

        for (ov_id, ov_x, _) in override_nodes_info {
            map.insert(ov_id, (ov_x, chain_y_l3));
        }

        let chain_height = if override_targets.is_empty() {
            130.0
        } else {
            195.0
        };

        current_chain_y += chain_height + 45.0;
    }

    map
}

fn do_load_project(state: &Rc<RefCell<AppState>>, widgets: &Rc<AppWidgets>) {
    let (ioc_path, main_c_path, dir_opt) = {
        let st = state.borrow();
        (st.discovered_ioc.clone(), st.discovered_main_c.clone(), st.project_dir.clone())
    };

    let (ioc_path, main_c_path) = match (ioc_path, main_c_path) {
        (Some(i), Some(c)) => (i, c),
        _ => return,
    };

    match load_project(&ioc_path, &main_c_path) {
        Ok(project) => {
            if let Some(dir) = dir_opt {
                save_app_config(&dir.display().to_string());
            }

            widgets.lbl_project_name.set_text(&format!("NAME: {}", project.meta.name));
            widgets.lbl_mcu_family.set_text(&format!("FAMILY: {}", project.meta.mcu_family));
            widgets.lbl_mcu_name.set_text(&format!("PART: {}", project.meta.mcu_name));

            // Populate Peripherals List using adw::ActionRow
            widgets.lbl_periph_header.set_text(&format!("[ ▸ PERIPHERALS ({}) ]", project.peripherals.len()));
            clear_list_box(&widgets.list_peripherals);
            for p in &project.peripherals {
                let row = create_peripheral_row(&p.name, p.mode.as_deref(), p.parameters.len());
                widgets.list_peripherals.append(&row);
            }

            // Populate User Regions List using adw::ActionRow
            let mut total_regions = project.user_regions.len();
            if project.loop_body.is_some() {
                total_regions += 1;
            }

            widgets.lbl_region_header.set_text(&format!("[ ▸ USER REGIONS ({}) ]", total_regions));
            clear_list_box(&widgets.list_user_regions);

            for r in &project.user_regions {
                let row = create_region_row(
                    &r.tag,
                    r.byte_range.0,
                    r.byte_range.1,
                    r.line_range.0,
                    r.line_range.1,
                    false,
                );
                widgets.list_user_regions.append(&row);
            }

            if let Some(ref lb) = project.loop_body {
                let row = create_region_row(
                    &lb.tag,
                    lb.byte_range.0,
                    lb.byte_range.1,
                    lb.line_range.0,
                    lb.line_range.1,
                    true,
                );
                widgets.list_user_regions.append(&row);
            }

            // Populate PV Variables List using adw::ActionRow
            widgets.lbl_pv_header.set_text(&format!("[ ▸ PV VARIABLES ({}) ]", project.pv_declarations.len()));
            clear_list_box(&widgets.list_pv_variables);
            for pv in &project.pv_declarations {
                let row = create_pv_row(&pv.name, &pv.type_str, pv.initial_value.as_deref(), pv.line);
                widgets.list_pv_variables.append(&row);
            }

            let init_positions = compute_initial_graph_layout(&project.call_graph_edges);
            state.borrow_mut().graph_node_positions = init_positions;
            widgets.graph_drawing_area.set_content_width(2000);
            widgets.graph_drawing_area.set_content_height(1500);
            widgets.btn_call_graph.set_sensitive(true);
            widgets.graph_drawing_area.queue_draw();

            state.borrow_mut().loaded_project = Some(project);
            widgets.toast_overlay.add_toast(adw::Toast::new("✓ Project loaded successfully"));
        }
        Err(err) => {
            widgets.toast_overlay.add_toast(adw::Toast::new(&format!("✗ Load Error: {}", err)));
        }
    }
}

fn open_pv_source_view(pv_idx: usize, state: &Rc<RefCell<AppState>>, widgets: &Rc<AppWidgets>) {
    let (project, main_c_path) = {
        let st = state.borrow();
        match &st.loaded_project {
            Some(p) => (p.clone(), p.meta.main_c_path.clone()),
            None => return,
        }
    };

    if pv_idx >= project.pv_declarations.len() {
        return;
    }

    widgets.inline_edit_bar.set_visible(false);
    widgets.source_view.set_editable(false);
    widgets.source_view.set_cursor_visible(false);
    state.borrow_mut().is_inline_editing = false;

    let decl = project.pv_declarations[pv_idx].clone();
    let main_c_content = match std::fs::read_to_string(&main_c_path) {
        Ok(c) => c,
        Err(e) => {
            widgets.toast_overlay.add_toast(adw::Toast::new(&format!("✗ Error reading main.c: {}", e)));
            return;
        }
    };

    widgets.lbl_active_pv.set_text(&format!("[ PV VARIABLE: {} {} (Line {}) ]", decl.type_str, decl.name, decl.line));
    widgets.source_buffer.set_text(&main_c_content);

    // Apply C language syntax definition
    if let Some(lang) = sourceview5::LanguageManager::default().language("c") {
        widgets.source_buffer.set_language(Some(&lang));
    }

    let usages = find_variable_usages(&main_c_path, &decl.name, decl.byte_range).unwrap_or_default();
    let usage_byte_ranges: Vec<(usize, usize)> = usages.iter().map(|u| u.byte_range).collect();

    let rendered_lines = build_source_render_model(
        &main_c_path,
        &project.user_regions,
        decl.byte_range,
        &usage_byte_ranges,
    )
    .unwrap_or_default();

    // Clear existing line tags
    let start_iter = widgets.source_buffer.start_iter();
    let end_iter = widgets.source_buffer.end_iter();
    widgets.source_buffer.remove_all_tags(&start_iter, &end_iter);

    let mut usage_lines = Vec::new();

    for line in &rendered_lines {
        let line_idx = (line.line_number.saturating_sub(1)) as i32;
        if let Some(line_start) = widgets.source_buffer.iter_at_line(line_idx) {
            let mut line_end = line_start;
            if !line_end.ends_line() {
                line_end.forward_to_line_end();
            }

            match line.tier {
                LineTier::Declaration => {
                    widgets.source_buffer.apply_tag(&widgets.tag_declaration, &line_start, &line_end);
                }
                LineTier::Usage => {
                    widgets.source_buffer.apply_tag(&widgets.tag_usage, &line_start, &line_end);
                    usage_lines.push(line.line_number);
                }
                LineTier::Generated => {
                    widgets.source_buffer.apply_tag(&widgets.tag_generated, &line_start, &line_end);
                }
                LineTier::Normal => {}
            }
        }
    }

    // Group generated lines into runs
    let mut runs: Vec<GeneratedRun> = Vec::new();
    let mut current_run_start: Option<usize> = None;
    let mut current_run_end: usize = 0;

    for item in &rendered_lines {
        if item.tier == LineTier::Generated {
            if current_run_start.is_some() {
                current_run_end = item.line_number;
            } else {
                current_run_start = Some(item.line_number);
                current_run_end = item.line_number;
            }
        } else {
            if let Some(start) = current_run_start {
                if current_run_end >= start {
                    runs.push(GeneratedRun {
                        start_line: start,
                        end_line: current_run_end,
                        is_collapsed: true,
                    });
                }
                current_run_start = None;
            }
        }
    }
    if let Some(start) = current_run_start {
        if current_run_end >= start {
            runs.push(GeneratedRun {
                start_line: start,
                end_line: current_run_end,
                is_collapsed: true,
            });
        }
    }

    for run in &runs {
        apply_run_collapse(run, widgets);
    }
    widgets.btn_toggle_generated.set_label("[ Show Generated ]");

    {
        let mut st = state.borrow_mut();
        st.active_pv_index = Some(pv_idx);
        st.active_decl = Some(decl.clone());
        st.active_usage_lines = usage_lines;
        st.generated_runs = runs;
        st.is_generated_hidden = true;
    }

    widgets.stack.set_visible_child_full("source_view", gtk4::StackTransitionType::SlideLeft);

    // Sequence transition: wait for horizontal slide animation to complete fully (260ms), THEN scroll vertically to target declaration line
    let decl_line_idx = (decl.line.saturating_sub(1)) as i32;
    let source_view_clone = widgets.source_view.clone();
    let source_buffer_clone = widgets.source_buffer.clone();

    glib::timeout_add_local_once(std::time::Duration::from_millis(260), move || {
        if let Some(mut decl_iter) = source_buffer_clone.iter_at_line(decl_line_idx) {
            source_view_clone.scroll_to_iter(&mut decl_iter, 0.1, true, 0.0, 0.3);
        }
    });
}

fn apply_run_collapse(run: &GeneratedRun, widgets: &Rc<AppWidgets>) {
    let start_line_idx = (run.start_line.saturating_sub(1)) as i32;
    let end_line_idx = run.end_line as i32;

    let start_iter = widgets.source_buffer.iter_at_line(start_line_idx);
    let mut end_iter = widgets.source_buffer.iter_at_line(end_line_idx);
    if end_iter.is_none() {
        end_iter = Some(widgets.source_buffer.end_iter());
    }

    if let (Some(start_it), Some(end_it)) = (start_iter, end_iter) {
        if run.is_collapsed {
            widgets.source_buffer.apply_tag(&widgets.tag_invisible, &start_it, &end_it);
        } else {
            widgets.source_buffer.remove_tag(&widgets.tag_invisible, &start_it, &end_it);
        }
    }
}

fn toggle_all_generated_runs(state: &Rc<RefCell<AppState>>, widgets: &Rc<AppWidgets>) {
    let (is_hidden, runs) = {
        let mut st = state.borrow_mut();
        st.is_generated_hidden = !st.is_generated_hidden;
        (st.is_generated_hidden, st.generated_runs.clone())
    };

    for mut run in runs {
        run.is_collapsed = is_hidden;
        apply_run_collapse(&run, widgets);
    }

    if is_hidden {
        widgets.btn_toggle_generated.set_label("[ Show Generated ]");
    } else {
        widgets.btn_toggle_generated.set_label("[ Hide Generated ]");
    }
}

fn enter_inline_edit_mode(state: &Rc<RefCell<AppState>>, widgets: &Rc<AppWidgets>) {
    let (decl, line_count) = {
        let st = state.borrow();
        let decl = match &st.active_decl {
            Some(d) => d.clone(),
            None => return,
        };
        (decl, widgets.source_buffer.line_count())
    };

    widgets.source_view.set_editable(true);
    widgets.source_view.set_cursor_visible(true);

    let decl_line_idx = (decl.line.saturating_sub(1)) as i32;

    // Apply tag_readonly to range before decl line
    if decl_line_idx > 0 {
        let start = widgets.source_buffer.start_iter();
        if let Some(end) = widgets.source_buffer.iter_at_line(decl_line_idx) {
            widgets.source_buffer.apply_tag(&widgets.tag_readonly, &start, &end);
        }
    }

    // Apply tag_readonly to range after decl line
    let after_line_idx = decl_line_idx + 1;
    if after_line_idx < line_count {
        if let Some(start) = widgets.source_buffer.iter_at_line(after_line_idx) {
            let end = widgets.source_buffer.end_iter();
            widgets.source_buffer.apply_tag(&widgets.tag_readonly, &start, &end);
        }
    }

    // Place cursor on declaration line
    if let Some(mut iter) = widgets.source_buffer.iter_at_line(decl_line_idx) {
        widgets.source_buffer.place_cursor(&iter);
        widgets.source_view.scroll_to_iter(&mut iter, 0.1, true, 0.0, 0.5);
    }

    widgets.lbl_inline_error.set_visible(false);
    widgets.lbl_inline_error.set_text("");
    widgets.inline_edit_bar.set_visible(true);
    state.borrow_mut().is_inline_editing = true;
}

fn save_inline_declaration_edit(state: &Rc<RefCell<AppState>>, widgets: &Rc<AppWidgets>) {
    let (decl, main_c_path, active_pv_idx) = {
        let st = state.borrow();
        let decl = match &st.active_decl {
            Some(d) => d.clone(),
            None => return,
        };
        let main_c_path = match &st.loaded_project {
            Some(p) => p.meta.main_c_path.clone(),
            None => return,
        };
        (decl, main_c_path, st.active_pv_index)
    };

    let decl_line_idx = (decl.line.saturating_sub(1)) as i32;
    let new_text = match widgets.source_buffer.iter_at_line(decl_line_idx) {
        Some(start) => {
            let mut end = start;
            if !end.ends_line() {
                end.forward_to_line_end();
            }
            widgets.source_buffer.text(&start, &end, false).to_string()
        }
        None => return,
    };

    let trimmed_text = new_text.trim_end_matches(['\r', '\n']);

    match save_pv_declaration_edit(&main_c_path, &decl, trimmed_text) {
        Ok(_) => {
            widgets.inline_edit_bar.set_visible(false);
            widgets.source_view.set_editable(false);
            widgets.source_view.set_cursor_visible(false);
            state.borrow_mut().is_inline_editing = false;

            // Display toast notification for save success
            widgets.toast_overlay.add_toast(adw::Toast::new("Declaration saved successfully"));

            // Reload project and refresh view
            do_load_project(state, widgets);
            if let Some(idx) = active_pv_idx {
                open_pv_source_view(idx, state, widgets);
            }
        }
        Err(err_msg) => {
            widgets.lbl_inline_error.set_text(&format!("Save error: {}", err_msg));
            widgets.lbl_inline_error.set_visible(true);
            widgets.toast_overlay.add_toast(adw::Toast::new(&format!("Save Error: {}", err_msg)));
        }
    }
}

fn cancel_inline_declaration_edit(state: &Rc<RefCell<AppState>>, widgets: &Rc<AppWidgets>) {
    let active_pv_idx = state.borrow().active_pv_index;
    widgets.inline_edit_bar.set_visible(false);
    widgets.source_view.set_editable(false);
    widgets.source_view.set_cursor_visible(false);
    state.borrow_mut().is_inline_editing = false;

    if let Some(idx) = active_pv_idx {
        open_pv_source_view(idx, state, widgets);
    }
}

fn save_pv_declaration_edit(
    main_c_path: &Path,
    decl: &PvDeclaration,
    new_text: &str,
) -> Result<(), String> {
    let fresh_regions = scan_file(main_c_path).map_err(|e| e.to_string())?;
    let pv_region = fresh_regions
        .iter()
        .find(|r| r.tag == "PV")
        .ok_or_else(|| "PV region not found in main.c".to_string())?;

    let full_content = std::fs::read_to_string(main_c_path).map_err(|e| e.to_string())?;

    if decl.byte_range.0 < pv_region.byte_range.0 || decl.byte_range.1 > pv_region.byte_range.1 {
        return Err("Declaration byte range is outside current PV region".to_string());
    }

    let offset_start = decl.byte_range.0 - pv_region.byte_range.0;
    let offset_end = decl.byte_range.1 - pv_region.byte_range.0;

    let mut pv_content = full_content[pv_region.byte_range.0..pv_region.byte_range.1].to_string();
    if offset_start > pv_content.len() || offset_end > pv_content.len() {
        return Err("Invalid byte range offsets inside PV region".to_string());
    }

    pv_content.replace_range(offset_start..offset_end, new_text);

    write_region(main_c_path, pv_region, &pv_content).map_err(|e| e.to_string())
}

fn clear_list_box(list_box: &gtk4::ListBox) {
    while let Some(child) = list_box.first_child() {
        list_box.remove(&child);
    }
}

fn create_peripheral_row(name: &str, mode: Option<&str>, param_count: usize) -> adw::ActionRow {
    let row = adw::ActionRow::builder()
        .title(name)
        .subtitle(mode.unwrap_or("—"))
        .build();

    let badge = gtk4::Label::builder()
        .label(&format!("{} params", param_count))
        .valign(gtk4::Align::Center)
        .css_classes(vec!["dim-label".to_string(), "caption".to_string()])
        .build();

    row.add_suffix(&badge);
    row
}

fn create_region_row(
    tag: &str,
    byte_start: usize,
    byte_end: usize,
    line_start: usize,
    line_end: usize,
    is_implicit: bool,
) -> adw::ActionRow {
    let details = format!("L{}-L{} (bytes {}..{})", line_start, line_end, byte_start, byte_end);
    let row = adw::ActionRow::builder()
        .title(tag)
        .subtitle(&details)
        .build();

    if is_implicit {
        let badge = gtk4::Label::builder()
            .label("implicit")
            .valign(gtk4::Align::Center)
            .css_classes(vec!["implicit-badge".to_string()])
            .build();
        row.add_suffix(&badge);
    }

    row
}

fn create_pv_row(
    name: &str,
    type_str: &str,
    initial_value: Option<&str>,
    line: usize,
) -> adw::ActionRow {
    let subtitle = match initial_value {
        Some(val) => format!("{} = {}", type_str, val),
        None => type_str.to_string(),
    };

    let row = adw::ActionRow::builder()
        .title(name)
        .subtitle(&subtitle)
        .activatable(true)
        .css_classes(vec!["clickable-row".to_string()])
        .build();

    let lbl_line = gtk4::Label::builder()
        .label(&format!("Line {}", line))
        .valign(gtk4::Align::Center)
        .css_classes(vec!["dim-label".to_string(), "caption".to_string()])
        .build();

    row.add_suffix(&lbl_line);
    row
}
