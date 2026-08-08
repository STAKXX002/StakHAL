mod config;

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use gtk4::gdk;
use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use libadwaita as adw;
use sourceview5::prelude::*;

use stakhal_core::ioc::discovery::discover_project_files;
use stakhal_core::ir::schema::{load_project, Project};
use stakhal_core::source::marker_scan::scan_file;
use stakhal_core::source::pv_extract::PvDeclaration;
use stakhal_core::source::render_model::{build_source_render_model, LineTier};
use stakhal_core::source::usage_finder::find_variable_usages;
use stakhal_core::source::writeback::write_region;

use config::{load_app_config, save_app_config};

const APP_ID: &str = "org.stakhal.StakHAL";

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
}

struct AppWidgets {
    window: adw::ApplicationWindow,
    stack: gtk4::Stack,
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

    // Source view widgets
    source_view: sourceview5::View,
    source_buffer: sourceview5::Buffer,
    lbl_active_pv: gtk4::Label,
    tag_declaration: gtk4::TextTag,
    tag_usage: gtk4::TextTag,
    tag_generated: gtk4::TextTag,
    tag_readonly: gtk4::TextTag,

    // Inline edit bar widgets
    inline_edit_bar: gtk4::Box,
    lbl_inline_error: gtk4::Label,
    btn_inline_save: gtk4::Button,
    btn_inline_cancel: gtk4::Button,
}

fn main() {
    let app = adw::Application::builder()
        .application_id(APP_ID)
        .build();

    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &adw::Application) {
    // Force dark mode consistently across all Libadwaita / GTK4 widgets and dialogs
    adw::StyleManager::default().set_color_scheme(adw::ColorScheme::ForceDark);

    // Monospace font styling for GtkSourceView
    let css_provider = gtk4::CssProvider::new();
    css_provider.load_from_string(
        "textview { font-family: 'DejaVu Sans Mono', monospace; font-size: 13px; }",
    );
    if let Some(display) = gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &css_provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    // Top Banner for Inline Errors
    let banner = adw::Banner::builder()
        .revealed(false)
        .build();

    // PAGE 1: Overview Page
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
        .label("PV Variables (click variable to inspect code)")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["heading".to_string()])
        .build();

    let list_pv_variables = gtk4::ListBox::builder()
        .css_classes(vec!["boxed-list".to_string()])
        .selection_mode(gtk4::SelectionMode::Single)
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

    let overview_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .build();
    overview_box.append(&toolbar_box);
    overview_box.append(&header_cards_box);
    overview_box.append(&columns_box);

    // PAGE 2: PV Source Panel
    let btn_back = gtk4::Button::builder()
        .label("← Back to Overview")
        .icon_name("go-previous-symbolic")
        .build();

    let lbl_active_pv = gtk4::Label::builder()
        .label("PV Variable Source View")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["title-3".to_string()])
        .build();

    let source_header_bar = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(16)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(16)
        .margin_end(16)
        .build();
    source_header_bar.append(&btn_back);
    source_header_bar.append(&lbl_active_pv);

    let source_buffer = sourceview5::Buffer::new(None);
    let source_view = sourceview5::View::with_buffer(&source_buffer);
    source_view.set_show_line_numbers(true);
    source_view.set_editable(false);
    source_view.set_cursor_visible(false);
    source_view.set_monospace(true);

    let tag_declaration = gtk4::TextTag::builder()
        .name("declaration")
        .paragraph_background("rgba(16, 185, 129, 0.25)")
        .build();

    let tag_usage = gtk4::TextTag::builder()
        .name("usage")
        .paragraph_background("rgba(59, 130, 246, 0.18)")
        .build();

    let tag_generated = gtk4::TextTag::builder()
        .name("generated")
        .foreground("#6e6e6e")
        .build();

    let tag_readonly = gtk4::TextTag::builder()
        .name("readonly")
        .editable(false)
        .build();

    let tag_table = source_buffer.tag_table();
    tag_table.add(&tag_declaration);
    tag_table.add(&tag_usage);
    tag_table.add(&tag_generated);
    tag_table.add(&tag_readonly);

    let source_scrolled = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Automatic)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
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
        .margin_top(8)
        .margin_bottom(8)
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
        .margin_bottom(16)
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

    // View Stack with smooth transition
    let stack = gtk4::Stack::builder()
        .transition_type(gtk4::StackTransitionType::SlideLeftRight)
        .transition_duration(300)
        .build();
    stack.add_named(&overview_box, Some("overview"));
    stack.add_named(&source_panel_box, Some("source_view"));
    stack.set_visible_child_name("overview");

    // HeaderBar
    let header_bar = adw::HeaderBar::new();

    // Root Content Layout
    let content_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .build();
    content_box.append(&header_bar);
    content_box.append(&banner);
    content_box.append(&stack);

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
        stack,
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
        source_view,
        source_buffer,
        lbl_active_pv,
        tag_declaration,
        tag_usage,
        tag_generated,
        tag_readonly,
        inline_edit_bar,
        lbl_inline_error,
        btn_inline_save,
        btn_inline_cancel,
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

    // Connect Back Button Callback
    let widgets_back = Rc::clone(&widgets);
    btn_back.connect_clicked(move |_| {
        widgets_back.stack.set_visible_child_name("overview");
    });

    // Connect PV Row Click Callback
    let state_pv_click = Rc::clone(&state);
    let widgets_pv_click = Rc::clone(&widgets);
    widgets.list_pv_variables.connect_row_activated(move |_, row| {
        let idx = row.index() as usize;
        open_pv_source_view(idx, &state_pv_click, &widgets_pv_click);
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

            // Populate User Regions List
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
            widgets.banner.set_title(&format!("Error reading main.c: {}", e));
            widgets.banner.set_revealed(true);
            return;
        }
    };

    widgets.lbl_active_pv.set_text(&format!("PV Variable: {} {} (Line {})", decl.type_str, decl.name, decl.line));
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

    {
        let mut st = state.borrow_mut();
        st.active_pv_index = Some(pv_idx);
        st.active_decl = Some(decl.clone());
        st.active_usage_lines = usage_lines;
    }

    // Scroll view to declaration line
    let decl_line_idx = (decl.line.saturating_sub(1)) as i32;
    if let Some(mut decl_iter) = widgets.source_buffer.iter_at_line(decl_line_idx) {
        widgets.source_view.scroll_to_iter(&mut decl_iter, 0.1, true, 0.0, 0.3);
    }

    widgets.stack.set_visible_child_name("source_view");
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

            // Reload project and refresh view
            do_load_project(state, widgets);
            if let Some(idx) = active_pv_idx {
                open_pv_source_view(idx, state, widgets);
            }
        }
        Err(err_msg) => {
            widgets.lbl_inline_error.set_text(&format!("Save error: {}", err_msg));
            widgets.lbl_inline_error.set_visible(true);
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
