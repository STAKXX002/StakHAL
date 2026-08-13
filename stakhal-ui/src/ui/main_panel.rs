use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
use crate::state::create_icon_button;

pub struct MainPanelWidgets {
    pub overview_box: gtk4::Box,
    pub btn_browse: gtk4::Button,
    pub btn_load: gtk4::Button,
    pub btn_call_graph: gtk4::Button,
    pub lbl_discovered_dir: gtk4::Label,
    pub lbl_ioc_path: gtk4::Label,
    pub lbl_main_c_path: gtk4::Label,
    pub lbl_project_name: gtk4::Label,
    pub lbl_mcu_family: gtk4::Label,
    pub lbl_mcu_name: gtk4::Label,
    pub lbl_periph_header: gtk4::Label,
    pub lbl_region_header: gtk4::Label,
    pub lbl_pv_header: gtk4::Label,
    pub list_peripherals: gtk4::ListBox,
    pub list_user_regions: gtk4::ListBox,
    pub list_pv_variables: gtk4::ListBox,
}

pub fn build_main_panel() -> MainPanelWidgets {
    let btn_browse = gtk4::Button::builder()
        .icon_name("folder-open-symbolic")
        .tooltip_text("Browse Project Folder")
        .css_classes(vec!["stakhal-btn".to_string(), "flat".to_string()])
        .build();
    btn_browse.set_cursor_from_name(Some("pointer"));

    let lbl_discovered_dir = gtk4::Label::builder()
        .label("No folder selected")
        .halign(gtk4::Align::Start)
        .hexpand(true)
        .ellipsize(gtk4::pango::EllipsizeMode::End)
        .css_classes(vec!["dim-label".to_string()])
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

    let btn_load = create_icon_button("Load Project", "system-run-symbolic", true);
    btn_load.set_sensitive(false);

    let btn_call_graph = gtk4::Button::builder()
        .label("[ Call Graph ]")
        .css_classes(vec!["stakhal-btn".to_string(), "flat".to_string()])
        .sensitive(false)
        .build();
    btn_call_graph.set_cursor_from_name(Some("pointer"));

    let toolbar_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(12)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(18)
        .margin_end(18)
        .build();

    let paths_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(2)
        .hexpand(true)
        .build();

    paths_box.append(&lbl_discovered_dir);
    let sub_paths_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(16)
        .build();
    sub_paths_box.append(&lbl_ioc_path);
    sub_paths_box.append(&lbl_main_c_path);
    paths_box.append(&sub_paths_box);

    toolbar_box.append(&btn_browse);
    toolbar_box.append(&paths_box);
    toolbar_box.append(&btn_load);
    toolbar_box.append(&btn_call_graph);

    let lbl_project_name = gtk4::Label::builder()
        .label("NAME: —")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["caption".to_string()])
        .build();

    let lbl_mcu_family = gtk4::Label::builder()
        .label("FAMILY: —")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["caption".to_string()])
        .build();

    let lbl_mcu_name = gtk4::Label::builder()
        .label("MCU: —")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["caption".to_string()])
        .build();

    let div_1 = gtk4::Label::builder().label("|").css_classes(vec!["dim-label".to_string()]).build();
    let div_2 = gtk4::Label::builder().label("|").css_classes(vec!["dim-label".to_string()]).build();

    let status_bar_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(12)
        .margin_bottom(12)
        .margin_start(18)
        .margin_end(18)
        .css_classes(vec!["card".to_string()])
        .build();

    status_bar_box.append(&lbl_project_name);
    status_bar_box.append(&div_1);
    status_bar_box.append(&lbl_mcu_family);
    status_bar_box.append(&div_2);
    status_bar_box.append(&lbl_mcu_name);

    let lbl_periph_header = gtk4::Label::builder()
        .label("[ ▸ PERIPHERALS ]")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["title-4".to_string()])
        .build();

    let lbl_region_header = gtk4::Label::builder()
        .label("[ ▸ USER REGIONS ]")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["title-4".to_string()])
        .build();

    let lbl_pv_header = gtk4::Label::builder()
        .label("[ ▸ PV VARIABLES ]")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["title-4".to_string()])
        .build();

    let list_peripherals = gtk4::ListBox::builder()
        .selection_mode(gtk4::SelectionMode::None)
        .build();

    let list_user_regions = gtk4::ListBox::builder()
        .selection_mode(gtk4::SelectionMode::None)
        .build();

    let list_pv_variables = gtk4::ListBox::builder()
        .selection_mode(gtk4::SelectionMode::None)
        .build();

    let col_peripherals = create_column_box(&lbl_periph_header, &list_peripherals);
    let col_regions = create_column_box(&lbl_region_header, &list_user_regions);
    let col_pv = create_column_box(&lbl_pv_header, &list_pv_variables);

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

    MainPanelWidgets {
        overview_box,
        btn_browse,
        btn_load,
        btn_call_graph,
        lbl_discovered_dir,
        lbl_ioc_path,
        lbl_main_c_path,
        lbl_project_name,
        lbl_mcu_family,
        lbl_mcu_name,
        lbl_periph_header,
        lbl_region_header,
        lbl_pv_header,
        list_peripherals,
        list_user_regions,
        list_pv_variables,
    }
}

pub fn create_column_box(header_label: &gtk4::Label, list_box: &gtk4::ListBox) -> gtk4::Box {
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

pub fn clear_list_box(list_box: &gtk4::ListBox) {
    while let Some(child) = list_box.first_child() {
        list_box.remove(&child);
    }
}

pub fn create_peripheral_row(name: &str, mode: Option<&str>, param_count: usize) -> adw::ActionRow {
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

pub fn create_region_row(
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

pub fn create_pv_row(
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

    row.set_cursor_from_name(Some("pointer"));

    let lbl_line = gtk4::Label::builder()
        .label(&format!("Line {}", line))
        .valign(gtk4::Align::Center)
        .css_classes(vec!["dim-label".to_string(), "caption".to_string()])
        .build();

    row.add_suffix(&lbl_line);
    row
}
