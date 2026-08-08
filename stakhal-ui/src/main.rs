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
}

struct AppWidgets {
    window: adw::ApplicationWindow,
    stack: gtk4::Stack,
    toast_overlay: adw::ToastOverlay,
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
    tag_invisible: gtk4::TextTag,

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

    // btop-inspired Terminal/TUI Design System CSS Provider
    let css_provider = gtk4::CssProvider::new();
    css_provider.load_from_string(
        "* { font-family: 'DejaVu Sans Mono', 'Liberation Mono', monospace; font-size: 13px; border-radius: 0px !important; box-shadow: none !important; }\
         window, .background, .main-window { background-color: #0a0a0a; color: #e0e0e0; }\
         headerbar, .topbar { background-color: #0a0a0a; border-bottom: 1px solid #2a2a2a; color: #e0e0e0; }\
         @define-color accent_color #ffffff;\
         @define-color accent_bg_color #1a1a1a;\
         @define-color accent_fg_color #ffffff;\
         @define-color accent_fill_color #1a1a1a;\
         @define-color window_bg_color #0a0a0a;\
         @define-color window_fg_color #e0e0e0;\
         @define-color view_bg_color #0a0a0a;\
         @define-color view_fg_color #e0e0e0;\
         @define-color card_bg_color #111111;\
         @define-color card_fg_color #e0e0e0;\
         @define-color dialog_bg_color #111111;\
         @define-color popover_bg_color #111111;\
         .card, .boxed-list, list { background-color: #111111; border: 1px solid #2a2a2a; border-radius: 0px !important; }\
         list > row, row.adw-action-row { border-bottom: 1px solid #2a2a2a; background-color: #111111; color: #e0e0e0; padding: 6px 12px; transition: background-color 120ms ease-out, border-color 120ms ease-out, color 120ms ease-out; }\
         list > row:last-child { border-bottom: none; }\
         list > row:hover, row.adw-action-row:hover { background-color: #1a1a1a; }\
         list > row:active, row.adw-action-row:active { background-color: #262626; }\
         list > row:selected { background-color: #222222; color: #ffffff; }\
         button { background-color: #111111; color: #e0e0e0; border: 1px solid #2a2a2a; border-radius: 0px !important; padding: 6px 12px; transition: background-color 120ms ease-out, border-color 120ms ease-out, color 120ms ease-out; }\
         button:hover { background-color: #1a1a1a; color: #ffffff; border-color: #444444; }\
         button:active { background-color: #262626; border-color: #555555; }\
         button.suggested-action { background-color: #ffffff !important; color: #000000 !important; font-weight: bold; border: 1px solid #ffffff !important; border-radius: 0px !important; transition: background-color 120ms ease-out, border-color 120ms ease-out, color 120ms ease-out; }\
         button.suggested-action:hover { background-color: #e0e0e0 !important; color: #000000 !important; }\
         button.suggested-action:active { background-color: #cccccc !important; color: #000000 !important; }\
         button.suggested-action:disabled { background-color: #222222 !important; color: #6e6e6e !important; border-color: #2a2a2a !important; }\
         *:focus, button:focus, entry:focus { outline: 1px solid #ffffff !important; outline-offset: -1px; }\
         .dim-label, .caption, subtitle { color: #6e6e6e; }\
         .implicit-badge { background-color: #facc15; color: #000000; font-size: 11px; font-weight: bold; padding: 2px 6px; border-radius: 0px !important; }\
         toast { background-color: #111111; color: #e0e0e0; border: 1px solid #2a2a2a; border-radius: 0px !important; }\
         .toast-success { color: #4ade80 !important; }\
         .toast-error { color: #f87171 !important; }\
         textview, textview text { background-color: #0a0a0a; color: #e0e0e0; }",
    );
    if let Some(display) = gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &css_provider,
            gtk4::STYLE_PROVIDER_PRIORITY_USER,
        );
    }

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
        .spacing(6)
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
        .margin_start(18)
        .margin_end(18)
        .build();
    toolbar_box.append(&btn_browse);
    toolbar_box.append(&path_info_box);
    toolbar_box.append(&btn_load);

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
        .spacing(18)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(18)
        .margin_end(18)
        .build();
    source_header_bar.append(&btn_back);
    source_header_bar.append(&lbl_active_pv);

    let source_buffer = sourceview5::Buffer::new(None);
    let scheme_mgr = sourceview5::StyleSchemeManager::default();
    let dark_scheme = scheme_mgr
        .scheme("Adwaita-dark")
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

    // View Stack with smooth panel-switch transitions (Overview ↔ Source View)
    let stack = gtk4::Stack::builder()
        .transition_type(gtk4::StackTransitionType::SlideLeftRight)
        .transition_duration(220)
        .interpolate_size(true)
        .build();
    stack.add_named(&overview_box, Some("overview"));
    stack.add_named(&source_panel_box, Some("source_view"));
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
        tag_invisible,
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
        widgets_back.stack.set_visible_child_full("overview", gtk4::StackTransitionType::SlideRight);
    });

    // Connect PV Row Click Callback
    let state_pv_click = Rc::clone(&state);
    let widgets_pv_click = Rc::clone(&widgets);
    widgets.list_pv_variables.connect_row_activated(move |_, row| {
        let idx = row.index() as usize;
        open_pv_source_view(idx, &state_pv_click, &widgets_pv_click);
    });

    // Attach gesture controller to source_view for gutter fold toggling
    let gesture_gutter = gtk4::GestureClick::new();
    gesture_gutter.set_button(1);
    let state_gutter = Rc::clone(&state);
    let widgets_gutter = Rc::clone(&widgets);
    gesture_gutter.connect_pressed(move |gesture, _n_press, _x, y| {
        let (_, window_y) = widgets_gutter.source_view.window_to_buffer_coords(
            gtk4::TextWindowType::Widget,
            0,
            y as i32,
        );

        let (iter, _) = widgets_gutter.source_view.line_at_y(window_y);
        let line_num = (iter.line() + 1) as usize;
        let st = state_gutter.borrow();
        let matching_run_idx = st.generated_runs.iter().position(|r| line_num >= r.start_line && line_num <= r.end_line);
        drop(st);

        if let Some(idx) = matching_run_idx {
            toggle_generated_run(idx, &state_gutter, &widgets_gutter);
            gesture.set_state(gtk4::EventSequenceState::Claimed);
        }
    });
    widgets.source_view.add_controller(gesture_gutter);

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
                        is_collapsed: false,
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
                is_collapsed: false,
            });
        }
    }

    {
        let mut st = state.borrow_mut();
        st.active_pv_index = Some(pv_idx);
        st.active_decl = Some(decl.clone());
        st.active_usage_lines = usage_lines;
        st.generated_runs = runs;
    }

    // Scroll view to declaration line
    let decl_line_idx = (decl.line.saturating_sub(1)) as i32;
    if let Some(mut decl_iter) = widgets.source_buffer.iter_at_line(decl_line_idx) {
        widgets.source_view.scroll_to_iter(&mut decl_iter, 0.1, true, 0.0, 0.3);
    }

    widgets.stack.set_visible_child_full("source_view", gtk4::StackTransitionType::SlideLeft);
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

fn toggle_generated_run(run_idx: usize, state: &Rc<RefCell<AppState>>, widgets: &Rc<AppWidgets>) {
    let run_opt = {
        let mut st = state.borrow_mut();
        if run_idx < st.generated_runs.len() {
            st.generated_runs[run_idx].is_collapsed = !st.generated_runs[run_idx].is_collapsed;
            Some(st.generated_runs[run_idx].clone())
        } else {
            None
        }
    };

    if let Some(run) = run_opt {
        apply_run_collapse(&run, widgets);
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
        .build();

    let lbl_line = gtk4::Label::builder()
        .label(&format!("Line {}", line))
        .valign(gtk4::Align::Center)
        .css_classes(vec!["dim-label".to_string(), "caption".to_string()])
        .build();

    row.add_suffix(&lbl_line);
    row
}
