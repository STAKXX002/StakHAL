mod config;

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use gtk4::gio;
use gtk4::prelude::*;
use libadwaita as adw;

use stakhal_core::ioc::discovery::discover_project_files;
use stakhal_core::ir::schema::{load_project, Project};

use config::{load_app_config, save_app_config};

const APP_ID: &str = "org.stakhal.StakHAL";

#[derive(Default)]
struct AppState {
    project_dir: Option<PathBuf>,
    discovered_ioc: Option<PathBuf>,
    discovered_main_c: Option<PathBuf>,
    loaded_project: Option<Project>,
}

struct AppWidgets {
    window: adw::ApplicationWindow,
    banner: adw::Banner,
    lbl_discovered_dir: gtk4::Label,
    lbl_ioc_path: gtk4::Label,
    lbl_main_c_path: gtk4::Label,
    btn_load: gtk4::Button,
    lbl_project_name: gtk4::Label,
    lbl_mcu_family: gtk4::Label,
    lbl_mcu_name: gtk4::Label,
    lbl_periph_header: gtk4::Label,
    lbl_region_header: gtk4::Label,
    lbl_pv_header: gtk4::Label,
    list_peripherals: gtk4::ListBox,
    list_user_regions: gtk4::ListBox,
    list_pv_variables: gtk4::ListBox,
}

fn main() {
    let app = adw::Application::builder()
        .application_id(APP_ID)
        .build();

    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &adw::Application) {
    // Top Banner for Inline Errors
    let banner = adw::Banner::builder()
        .revealed(false)
        .build();

    // Top Action Controls: Browse Folder & Load Project
    let btn_browse = gtk4::Button::builder()
        .label("Browse Project Folder…")
        .icon_name("folder-open-symbolic")
        .build();

    let btn_load = gtk4::Button::builder()
        .label("Load Project")
        .icon_name("system-run-symbolic")
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
        .spacing(2)
        .hexpand(true)
        .build();
    path_info_box.append(&lbl_discovered_dir);
    path_info_box.append(&lbl_ioc_path);
    path_info_box.append(&lbl_main_c_path);

    let toolbar_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(12)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(16)
        .margin_end(16)
        .build();
    toolbar_box.append(&btn_browse);
    toolbar_box.append(&path_info_box);
    toolbar_box.append(&btn_load);

    // Project Header Summary Cards (Project Name, MCU Family, MCU Name)
    let lbl_project_name = gtk4::Label::builder()
        .label("—")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["title-3".to_string()])
        .build();

    let card_project = create_summary_card("PROJECT NAME", &lbl_project_name);

    let lbl_mcu_family = gtk4::Label::builder()
        .label("—")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["title-3".to_string()])
        .build();

    let card_family = create_summary_card("MCU FAMILY", &lbl_mcu_family);

    let lbl_mcu_name = gtk4::Label::builder()
        .label("—")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["title-3".to_string()])
        .build();

    let card_mcu = create_summary_card("MCU PART", &lbl_mcu_name);

    let header_cards_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .homogeneous(true)
        .spacing(12)
        .margin_bottom(12)
        .margin_start(16)
        .margin_end(16)
        .build();
    header_cards_box.append(&card_project);
    header_cards_box.append(&card_family);
    header_cards_box.append(&card_mcu);

    // Three Column Setup: Peripherals, User Regions, PV Variables
    let lbl_periph_header = gtk4::Label::builder()
        .label("Peripherals")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["heading".to_string()])
        .build();

    let list_peripherals = gtk4::ListBox::builder()
        .css_classes(vec!["boxed-list".to_string()])
        .selection_mode(gtk4::SelectionMode::None)
        .build();

    let col_peripherals = create_column_box(&lbl_periph_header, &list_peripherals);

    let lbl_region_header = gtk4::Label::builder()
        .label("User Regions")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["heading".to_string()])
        .build();

    let list_user_regions = gtk4::ListBox::builder()
        .css_classes(vec!["boxed-list".to_string()])
        .selection_mode(gtk4::SelectionMode::None)
        .build();

    let col_regions = create_column_box(&lbl_region_header, &list_user_regions);

    let lbl_pv_header = gtk4::Label::builder()
        .label("PV Variables")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["heading".to_string()])
        .build();

    let list_pv_variables = gtk4::ListBox::builder()
        .css_classes(vec!["boxed-list".to_string()])
        .selection_mode(gtk4::SelectionMode::None)
        .build();

    let col_pv = create_column_box(&lbl_pv_header, &list_pv_variables);

    let columns_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .homogeneous(true)
        .spacing(12)
        .vexpand(true)
        .margin_start(16)
        .margin_end(16)
        .margin_bottom(16)
        .build();
    columns_box.append(&col_peripherals);
    columns_box.append(&col_regions);
    columns_box.append(&col_pv);

    // HeaderBar
    let header_bar = adw::HeaderBar::new();

    // Content Vertical Layout
    let content_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .build();
    content_box.append(&header_bar);
    content_box.append(&banner);
    content_box.append(&toolbar_box);
    content_box.append(&header_cards_box);
    content_box.append(&columns_box);

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("StakHAL - STM32 Project Viewer")
        .default_width(1200)
        .default_height(800)
        .content(&content_box)
        .build();

    let state = Rc::new(RefCell::new(AppState::default()));
    let widgets = Rc::new(AppWidgets {
        window: window.clone(),
        banner,
        lbl_discovered_dir,
        lbl_ioc_path,
        lbl_main_c_path,
        btn_load,
        lbl_project_name,
        lbl_mcu_family,
        lbl_mcu_name,
        lbl_periph_header,
        lbl_region_header,
        lbl_pv_header,
        list_peripherals,
        list_user_regions,
        list_pv_variables,
    });

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

    window.present();
}

fn create_summary_card(title: &str, value_label: &gtk4::Label) -> gtk4::Box {
    let title_lbl = gtk4::Label::builder()
        .label(title)
        .halign(gtk4::Align::Start)
        .css_classes(vec!["dim-label".to_string(), "caption-heading".to_string()])
        .build();

    let box_card = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(4)
        .css_classes(vec!["card".to_string()])
        .margin_top(4)
        .margin_bottom(4)
        .build();

    let inner_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(4)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(16)
        .margin_end(16)
        .build();
    inner_box.append(&title_lbl);
    inner_box.append(value_label);

    box_card.append(&inner_box);
    box_card
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
        .spacing(8)
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
            widgets.banner.set_revealed(false);
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
            widgets.banner.set_title(&format!("Discovery Error: {}", err));
            widgets.banner.set_revealed(true);
            widgets.lbl_ioc_path.set_text("IOC Path: —");
            widgets.lbl_main_c_path.set_text("Main C Path: —");
            st.discovered_ioc = None;
            st.discovered_main_c = None;
            widgets.btn_load.set_sensitive(false);
        }
    }
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
            widgets.banner.set_revealed(false);

            if let Some(dir) = dir_opt {
                save_app_config(&dir.display().to_string());
            }

            widgets.lbl_project_name.set_text(&project.meta.name);
            widgets.lbl_mcu_family.set_text(&project.meta.mcu_family);
            widgets.lbl_mcu_name.set_text(&project.meta.mcu_name);

            // Populate Peripherals List
            widgets.lbl_periph_header.set_text(&format!("Peripherals ({})", project.peripherals.len()));
            clear_list_box(&widgets.list_peripherals);
            for p in &project.peripherals {
                let row = create_peripheral_row(&p.name, p.mode.as_deref(), p.parameters.len());
                widgets.list_peripherals.append(&row);
            }

            // Populate User Regions List (including loop_body if present)
            let mut total_regions = project.user_regions.len();
            if project.loop_body.is_some() {
                total_regions += 1;
            }

            widgets.lbl_region_header.set_text(&format!("User Regions ({})", total_regions));
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

            // Populate PV Variables List
            widgets.lbl_pv_header.set_text(&format!("PV Variables ({})", project.pv_declarations.len()));
            clear_list_box(&widgets.list_pv_variables);
            for pv in &project.pv_declarations {
                let row = create_pv_row(&pv.name, &pv.type_str, pv.initial_value.as_deref(), pv.line);
                widgets.list_pv_variables.append(&row);
            }

            state.borrow_mut().loaded_project = Some(project);
        }
        Err(err) => {
            widgets.banner.set_title(&format!("Load Error: {}", err));
            widgets.banner.set_revealed(true);
        }
    }
}

fn clear_list_box(list_box: &gtk4::ListBox) {
    while let Some(child) = list_box.first_child() {
        list_box.remove(&child);
    }
}

fn create_peripheral_row(name: &str, mode: Option<&str>, param_count: usize) -> gtk4::ListBoxRow {
    let lbl_name = gtk4::Label::builder()
        .label(name)
        .halign(gtk4::Align::Start)
        .css_classes(vec!["body".to_string()])
        .build();

    let mode_str = mode.unwrap_or("—");
    let lbl_mode = gtk4::Label::builder()
        .label(mode_str)
        .halign(gtk4::Align::Start)
        .css_classes(vec!["dim-label".to_string(), "caption".to_string()])
        .build();

    let name_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(2)
        .hexpand(true)
        .build();
    name_box.append(&lbl_name);
    name_box.append(&lbl_mode);

    let lbl_count = gtk4::Label::builder()
        .label(&format!("{} params", param_count))
        .halign(gtk4::Align::End)
        .valign(gtk4::Align::Center)
        .css_classes(vec!["dim-label".to_string(), "caption".to_string()])
        .build();

    let row_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .margin_top(8)
        .margin_bottom(8)
        .margin_start(12)
        .margin_end(12)
        .build();
    row_box.append(&name_box);
    row_box.append(&lbl_count);

    gtk4::ListBoxRow::builder()
        .child(&row_box)
        .build()
}

fn create_region_row(
    tag: &str,
    byte_start: usize,
    byte_end: usize,
    line_start: usize,
    line_end: usize,
    is_implicit: bool,
) -> gtk4::ListBoxRow {
    let lbl_tag = gtk4::Label::builder()
        .label(tag)
        .halign(gtk4::Align::Start)
        .css_classes(vec!["body".to_string()])
        .build();

    let lbl_details = gtk4::Label::builder()
        .label(&format!("L{}-L{} (bytes {}..{})", line_start, line_end, byte_start, byte_end))
        .halign(gtk4::Align::Start)
        .css_classes(vec!["dim-label".to_string(), "caption".to_string()])
        .build();

    let tag_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(2)
        .hexpand(true)
        .build();
    tag_box.append(&lbl_tag);
    tag_box.append(&lbl_details);

    let row_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .margin_top(8)
        .margin_bottom(8)
        .margin_start(12)
        .margin_end(12)
        .build();
    row_box.append(&tag_box);

    if is_implicit {
        let badge = gtk4::Label::builder()
            .label("implicit")
            .valign(gtk4::Align::Center)
            .css_classes(vec!["badge".to_string(), "accent".to_string()])
            .build();
        row_box.append(&badge);
    }

    gtk4::ListBoxRow::builder()
        .child(&row_box)
        .build()
}

fn create_pv_row(
    name: &str,
    type_str: &str,
    initial_value: Option<&str>,
    line: usize,
) -> gtk4::ListBoxRow {
    let lbl_name = gtk4::Label::builder()
        .label(name)
        .halign(gtk4::Align::Start)
        .css_classes(vec!["body".to_string()])
        .build();

    let val_info = match initial_value {
        Some(val) => format!("{} = {}", type_str, val),
        None => type_str.to_string(),
    };

    let lbl_type = gtk4::Label::builder()
        .label(&val_info)
        .halign(gtk4::Align::Start)
        .css_classes(vec!["dim-label".to_string(), "caption".to_string()])
        .build();

    let name_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(2)
        .hexpand(true)
        .build();
    name_box.append(&lbl_name);
    name_box.append(&lbl_type);

    let lbl_line = gtk4::Label::builder()
        .label(&format!("Line {}", line))
        .halign(gtk4::Align::End)
        .valign(gtk4::Align::Center)
        .css_classes(vec!["dim-label".to_string(), "caption".to_string()])
        .build();

    let row_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .margin_top(8)
        .margin_bottom(8)
        .margin_start(12)
        .margin_end(12)
        .build();
    row_box.append(&name_box);
    row_box.append(&lbl_line);

    gtk4::ListBoxRow::builder()
        .child(&row_box)
        .build()
}
