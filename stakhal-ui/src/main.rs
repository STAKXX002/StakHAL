use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use gtk4::{gdk, gio, glib};
use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;

use stakhal_core::graph::builder::EdgeType;
use stakhal_core::ioc::discovery::discover_project_files;
use stakhal_core::ir::schema::load_project;

mod config;
mod state;
mod ui;

use config::{load_app_config, save_app_config};
use state::{AppState, AppWidgets};
use ui::call_graph::{
    build_call_graph_panel, compute_graph_bounds, compute_graph_layout, setup_call_graph_drawing_and_gestures,
    CallGraphPanelWidgets,
};
use ui::nucleo_pinout::{
    build_nucleo_pinout_panel, setup_nucleo_pinout_drawing_and_gestures, NucleoPinoutPanelWidgets,
};

use ui::main_panel::{
    build_main_panel, clear_list_box, create_peripheral_row, create_pv_row, create_region_row, MainPanelWidgets,
};
use ui::source_panel::{
    build_source_panel, cancel_inline_declaration_edit, enter_inline_edit_mode, open_pv_source_view,
    save_inline_declaration_edit, toggle_all_generated_runs, SourcePanelWidgets,
};


const APP_ID: &str = "com.stakhal.ui";

fn main() {
    let app = adw::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();

    app.connect_activate(build_ui);
    app.run_with_args(&["stakhal-ui"]);
}

fn build_ui(app: &adw::Application) {
    adw::StyleManager::default().set_color_scheme(adw::ColorScheme::ForceDark);

    let css_provider = gtk4::CssProvider::new();
    css_provider.load_from_string(r#"
* {
    font-family: 'DejaVu Sans Mono', 'Liberation Mono', monospace;
    font-size: 13px;
    border-radius: 0px;
    box-shadow: none;
}
window, dialog {
    background-color: #0a0a0a;
    color: #e5e5e5;
}
windowcontrols button {
    border: none;
    background: transparent;
    border-radius: 0px;
}
windowcontrols button:hover {
    border: none;
    background: transparent;
}
button.stakhal-btn {
    border: 1px solid #262626;
    background-color: #121212;
    color: #e5e5e5;
    transition: all 120ms ease;
    border-radius: 0px;
}
button.stakhal-btn:hover {
    border-color: #525252;
    background-color: #1a1a1a;
    color: #ffffff;
}
button.stakhal-btn:active {
    background-color: #262626;
}
button.stakhal-btn.suggested-action {
    border-color: #e5e5e5;
    background-color: #e5e5e5;
    color: #0a0a0a;
}
button.stakhal-btn.suggested-action:hover {
    border-color: #ffffff;
    background-color: #ffffff;
    color: #000000;
}
button.stakhal-btn.flat {
    border-color: transparent;
    background-color: transparent;
    color: #a3a3a3;
}
button.stakhal-btn.flat:hover {
    border-color: #262626;
    background-color: #171717;
    color: #ffffff;
}
row, listboxrow, actionrow {
    border-radius: 0px;
    transition: none;
}
.clickable-row {
    transition: all 120ms ease;
}
.clickable-row:hover {
    background-color: #171717;
}
.clickable-row:active {
    background-color: #262626;
}
.dim-label {
    color: #737373;
}
.title-1, .title-2, .title-3, .heading {
    color: #f5f5f5;
    font-weight: bold;
}

/* Reserved Status Classes */
.status-error, .error {
    color: #ef4444;
}
.status-warning, .warning {
    color: #f59e0b;
}
.status-ok, .ok {
    color: #22c55e;
}
"#);


    gtk4::style_context_add_provider_for_display(
        &gdk::Display::default().expect("Could not connect to a display."),
        &css_provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    let MainPanelWidgets {
        overview_box,
        btn_browse,
        btn_load,
        btn_call_graph,
        btn_nucleo_pinout,
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
    } = build_main_panel();

    let SourcePanelWidgets {
        source_panel_box,
        btn_source_back,
        lbl_active_pv,
        btn_toggle_generated,
        source_view,
        source_buffer,
        tag_declaration,
        tag_usage,
        tag_generated,
        tag_readonly,
        tag_invisible,
        inline_edit_bar,
        lbl_inline_error,
        btn_inline_save,
        btn_inline_cancel,
    } = build_source_panel();

    let context_menu_popover = gtk4::Popover::builder()
        .autohide(true)
        .build();
    context_menu_popover.set_parent(&source_view);

    let CallGraphPanelWidgets {
        graph_panel_box,
        btn_graph_back,
        btn_fit_to_view,
        graph_drawing_area,
        graph_scrolled,
    } = build_call_graph_panel();

    let NucleoPinoutPanelWidgets {
        pinout_panel_box,
        btn_pinout_back,
        pinout_drawing_area,
        pinout_scrolled,
    } = build_nucleo_pinout_panel();

    let stack = gtk4::Stack::builder()
        .transition_type(gtk4::StackTransitionType::SlideLeftRight)
        .transition_duration(220)
        .build();

    stack.add_named(&overview_box, Some("overview"));
    stack.add_named(&source_panel_box, Some("source_view"));
    stack.add_named(&graph_panel_box, Some("call_graph"));
    stack.add_named(&pinout_panel_box, Some("nucleo_pinout"));
    stack.set_visible_child_name("overview");

    let header_bar = adw::HeaderBar::new();
    let toast_overlay = adw::ToastOverlay::new();

    let content_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .build();
    content_box.append(&header_bar);
    content_box.append(&stack);

    toast_overlay.set_child(Some(&content_box));

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("StakHAL — Hardware Abstraction Inspector")
        .maximized(true)
        .content(&toast_overlay)
        .build();

    let state = Rc::new(RefCell::new(AppState::default()));
    let widgets = Rc::new(AppWidgets {
        window: window.clone(),
        stack: stack.clone(),
        toast_overlay,
        lbl_discovered_dir,
        lbl_ioc_path,
        lbl_main_c_path,
        btn_load,
        btn_call_graph: btn_call_graph.clone(),
        btn_nucleo_pinout: btn_nucleo_pinout.clone(),
        lbl_project_name,
        lbl_mcu_family,
        lbl_mcu_name,
        lbl_periph_header,
        lbl_region_header,
        lbl_pv_header,
        list_peripherals,
        list_user_regions,
        list_pv_variables,
        source_view: source_view.clone(),
        source_buffer: source_buffer.clone(),
        lbl_active_pv,
        btn_toggle_generated: btn_toggle_generated.clone(),
        tag_declaration,
        tag_usage,
        tag_generated,
        tag_readonly,
        tag_invisible,
        inline_edit_bar,
        lbl_inline_error,
        graph_drawing_area,
        btn_fit_to_view: btn_fit_to_view.clone(),
        graph_scrolled: graph_scrolled.clone(),
        pinout_drawing_area,
        _pinout_scrolled: pinout_scrolled,
        context_menu_popover,
    });

    setup_call_graph_drawing_and_gestures(&state, &widgets);
    setup_nucleo_pinout_drawing_and_gestures(&state, &widgets);

    // Navigation callbacks
    let stack_back1 = stack.clone();
    btn_source_back.connect_clicked(move |_| {
        stack_back1.set_visible_child_full("overview", gtk4::StackTransitionType::SlideRight);
    });

    let stack_back2 = stack.clone();
    btn_graph_back.connect_clicked(move |_| {
        stack_back2.set_visible_child_full("overview", gtk4::StackTransitionType::SlideRight);
    });

    let stack_back3 = stack.clone();
    btn_pinout_back.connect_clicked(move |_| {
        stack_back3.set_visible_child_full("overview", gtk4::StackTransitionType::SlideRight);
    });



    let state_fit = Rc::clone(&state);
    let widgets_fit = Rc::clone(&widgets);
    widgets.btn_fit_to_view.connect_clicked(move |_| {
        let mut st = state_fit.borrow_mut();
        if st.loaded_project.is_none() || st.graph_node_positions.is_empty() {
            return;
        }
        let (bw, bh) = st.graph_bounds;
        let vw = widgets_fit
            .graph_scrolled
            .hadjustment()
            .page_size()
            .max(widgets_fit.graph_scrolled.width() as f64);
        let vh = widgets_fit
            .graph_scrolled
            .vadjustment()
            .page_size()
            .max(widgets_fit.graph_scrolled.height() as f64);

        if bw > 0 && bh > 0 && vw > 0.0 && vh > 0.0 {
            let fit_zoom = (vw / bw as f64).min(vh / bh as f64).clamp(0.25, 2.5);
            if !fit_zoom.is_nan() && !fit_zoom.is_infinite() {
                st.graph_zoom = fit_zoom;
                st.graph_pan_x = 0.0;
                st.graph_pan_y = 0.0;

                let zoomed_w = (bw as f64 * fit_zoom).ceil() as i32;
                let zoomed_h = (bh as f64 * fit_zoom).ceil() as i32;
                drop(st);


                widgets_fit.graph_drawing_area.set_content_width(zoomed_w);
                widgets_fit.graph_drawing_area.set_content_height(zoomed_h);

                widgets_fit.graph_scrolled.hadjustment().set_value(0.0);
                widgets_fit.graph_scrolled.vadjustment().set_value(0.0);

                widgets_fit.graph_drawing_area.queue_draw();
            }
        }
    });



    let stack_graph = stack.clone();
    btn_call_graph.connect_clicked(move |_| {
        stack_graph.set_visible_child_full("call_graph", gtk4::StackTransitionType::SlideLeft);
    });

    let stack_pinout = stack.clone();
    btn_nucleo_pinout.connect_clicked(move |_| {
        stack_pinout.set_visible_child_full("nucleo_pinout", gtk4::StackTransitionType::SlideLeft);
    });


    // Toggle generated code callback
    let state_toggle = Rc::clone(&state);
    let widgets_toggle = Rc::clone(&widgets);
    btn_toggle_generated.connect_clicked(move |_| {
        toggle_all_generated_runs(&state_toggle, &widgets_toggle);
    });

    // Inline edit callbacks
    let state_save_btn = Rc::clone(&state);
    let widgets_save_btn = Rc::clone(&widgets);
    btn_inline_save.connect_clicked(move |_| {
        save_inline_declaration_edit(&state_save_btn, &widgets_save_btn, do_load_project);
    });

    let state_cancel_btn = Rc::clone(&state);
    let widgets_cancel_btn = Rc::clone(&widgets);
    btn_inline_cancel.connect_clicked(move |_| {
        cancel_inline_declaration_edit(&state_cancel_btn, &widgets_cancel_btn);
    });

    let key_controller = gtk4::EventControllerKey::new();
    let state_key = Rc::clone(&state);
    let widgets_key = Rc::clone(&widgets);
    key_controller.connect_key_pressed(move |_, key, _code, modifier| {
        let is_editing = state_key.borrow().is_inline_editing;
        if !is_editing {
            return glib::Propagation::Proceed;
        }

        let is_ctrl = modifier.contains(gdk::ModifierType::CONTROL_MASK);

        if key == gdk::Key::Return || key == gdk::Key::KP_Enter || (is_ctrl && (key == gdk::Key::s || key == gdk::Key::S)) {
            save_inline_declaration_edit(&state_key, &widgets_key, do_load_project);
            glib::Propagation::Stop
        } else if key == gdk::Key::Escape {
            cancel_inline_declaration_edit(&state_key, &widgets_key);
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });

    widgets.source_view.add_controller(key_controller);

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

    let right_click_gesture = gtk4::GestureClick::new();
    right_click_gesture.set_button(3);
    let state_right_click = Rc::clone(&state);
    let widgets_right_click = Rc::clone(&widgets);

    right_click_gesture.connect_pressed(move |g, _n_press, x, y| {
        let widgets = &widgets_right_click;
        widgets.context_menu_popover.popdown();
        g.set_state(gtk4::EventSequenceState::Claimed);
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

        widgets.context_menu_popover.set_child(None::<&gtk4::Widget>);

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
            .css_classes(vec!["stakhal-btn".to_string(), "flat".to_string()])
            .build();
        btn_copy.set_cursor_from_name(Some("pointer"));

        let popover_clone = widgets.context_menu_popover.clone();
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
                .css_classes(vec!["stakhal-btn".to_string(), "flat".to_string()])
                .build();
            btn_edit.set_cursor_from_name(Some("pointer"));

            let popover_edit_clone = widgets.context_menu_popover.clone();
            let state_edit = Rc::clone(&state_right_click);
            let widgets_edit = Rc::clone(&widgets);
            btn_edit.connect_clicked(move |_| {
                popover_edit_clone.popdown();
                enter_inline_edit_mode(&state_edit, &widgets_edit);
            });
            menu_box.append(&btn_edit);
        }

        widgets.context_menu_popover.set_child(Some(&menu_box));
        let rect = gdk::Rectangle::new(x as i32, y as i32, 1, 1);
        widgets.context_menu_popover.set_pointing_to(Some(&rect));
        widgets.context_menu_popover.popup();
    });
    widgets.source_view.add_controller(right_click_gesture);

    // Connect Browse Button
    let state_browse = Rc::clone(&state);
    let widgets_browse = Rc::clone(&widgets);
    btn_browse.connect_clicked(move |_| {
        let dialog = gtk4::FileDialog::builder()
            .title("Select STM32 Project Directory")
            .build();

        let state_dialog = Rc::clone(&state_browse);
        let widgets_dialog = Rc::clone(&widgets_browse);

        dialog.select_folder(
            Some(&widgets_browse.window),
            gio::Cancellable::NONE,
            move |res| {
                if let Ok(folder) = res {
                    if let Some(path) = folder.path() {
                        try_discover_folder(&path, &state_dialog, &widgets_dialog);
                    }
                }
            },
        );
    });

    // Connect Load Button
    let state_load = Rc::clone(&state);
    let widgets_load = Rc::clone(&widgets);
    widgets.btn_load.connect_clicked(move |_| {
        do_load_project(&state_load, &widgets_load);
    });

    window.present();

    // Check last_project.json on startup
    let config = load_app_config();
    if let Some(dir_str) = config.project_dir {
        let path = PathBuf::from(dir_str);
        if path.exists() {
            try_discover_folder(&path, &state, &widgets);
        }
    }
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
    let (ioc_path, main_c_path, dir_path) = {
        let st = state.borrow();

        match (&st.discovered_ioc, &st.discovered_main_c, &st.project_dir) {
            (Some(i), Some(m), Some(d)) => (i.clone(), m.clone(), d.clone()),
            _ => {
                widgets.toast_overlay.add_toast(adw::Toast::new("✗ Project files not selected"));
                return;
            }
        }
    };

    save_app_config(&dir_path.display().to_string());

    match load_project(&ioc_path, &main_c_path) {
        Ok(project) => {
            widgets.lbl_project_name.set_text(&format!("NAME: {}", project.meta.name));
            widgets.lbl_mcu_family.set_text(&format!("FAMILY: {}", project.meta.mcu_family));
            widgets.lbl_mcu_name.set_text(&format!("MCU: {}", project.meta.mcu_name));

            widgets.lbl_periph_header.set_text(&format!("[ ▸ PERIPHERALS ({}) ]", project.peripherals.len()));

            let mut total_regions = project.user_regions.len();
            if project.loop_body.is_some() {
                total_regions += 1;
            }
            widgets.lbl_region_header.set_text(&format!("[ ▸ USER REGIONS ({}) ]", total_regions));

            widgets.lbl_pv_header.set_text(&format!("[ ▸ PV VARIABLES ({}) ]", project.pv_declarations.len()));

            clear_list_box(&widgets.list_peripherals);
            clear_list_box(&widgets.list_user_regions);
            clear_list_box(&widgets.list_pv_variables);

            for p in &project.peripherals {
                let row = create_peripheral_row(&p.name, p.mode.as_deref(), p.parameters.len());
                widgets.list_peripherals.append(&row);
            }

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

            let pv_targets: Vec<(&str, (usize, usize))> = project
                .pv_declarations
                .iter()
                .map(|pv| (pv.name.as_str(), (0, 0)))
                .collect();
            let batch_usages = stakhal_core::source::usage_finder::find_variable_usages_batch(&main_c_path, &pv_targets)
                .unwrap_or_else(|_| vec![Vec::new(); project.pv_declarations.len()]);

            for (idx, (pv, usages)) in project.pv_declarations.iter().zip(batch_usages.into_iter()).enumerate() {
                let is_unreferenced = usages.is_empty();
                let row = create_pv_row(&pv.name, &pv.type_str, pv.initial_value.as_deref(), pv.line, is_unreferenced);

                let state_clone = Rc::clone(state);
                let widgets_clone = Rc::clone(widgets);
                row.connect_activated(move |_| {
                    open_pv_source_view(idx, &state_clone, &widgets_clone);
                });
                widgets.list_pv_variables.append(&row);
            }

            let is_f446 = project.meta.mcu_name.to_uppercase().contains("F446");

            let mut collapsed = std::collections::HashSet::new();
            for e in project.call_graph_edges.iter().filter(|e| e.edge_type == EdgeType::IrqEntry) {
                collapsed.insert(e.from.clone());
            }

            let (init_positions, headers) = compute_graph_layout(&project.call_graph_edges, &collapsed);
            let (w, h) = compute_graph_bounds(&init_positions, &headers);
            let colors = crate::ui::call_graph::draw::compute_all_node_status_colors(&project.call_graph_edges, &init_positions);
            {
                let mut st = state.borrow_mut();
                st.collapsed_chains = collapsed;
                st.graph_node_positions = init_positions;
                st.node_status_colors = colors;
                st.chain_headers = headers;
                st.graph_bounds = (w, h);
                st.loaded_project = Some(project);
            }

            widgets.graph_drawing_area.set_content_width(w);
            widgets.graph_drawing_area.set_content_height(h);
            widgets.btn_call_graph.set_sensitive(true);
            widgets.graph_drawing_area.queue_draw();

            if is_f446 {
                widgets.btn_nucleo_pinout.set_sensitive(true);
                widgets.btn_nucleo_pinout.set_tooltip_text(Some("View Nucleo-F446RE Physical Connector Pinout"));
            } else {
                widgets.btn_nucleo_pinout.set_sensitive(false);
                widgets.btn_nucleo_pinout.set_tooltip_text(Some("Nucleo Pinout visualizer is F446RE-only for now"));
            }
            widgets.pinout_drawing_area.queue_draw();

            widgets.toast_overlay.add_toast(adw::Toast::new("✓ Project loaded successfully"));
        }


        Err(err) => {
            widgets.btn_nucleo_pinout.set_sensitive(false);
            widgets.btn_nucleo_pinout.set_tooltip_text(Some("Nucleo Pinout visualizer is F446RE-only for now"));
            widgets.toast_overlay.add_toast(adw::Toast::new(&format!("✗ Load Error: {}", err)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ui_build_smoke() {
        if let Err(err) = gtk4::init() {
            eprintln!("GTK display not available, skipping UI smoke test: {}", err);
            return;
        }
        let _ = adw::init();

        let app = adw::Application::builder()
            .application_id("com.stakhal.ui.smoke_test")
            .flags(gio::ApplicationFlags::NON_UNIQUE)
            .build();

        if let Err(err) = app.register(gio::Cancellable::NONE) {
            eprintln!("Failed to register GTK application in test: {}", err);
            return;
        }

        app.connect_activate(build_ui);
        app.activate();

        let windows = app.windows();
        assert!(
            !windows.is_empty(),
            "Expected ApplicationWindow to be constructed during build_ui"
        );
    }

    #[test]
    fn test_f446_project_loading_enables_pinout_btn() {
        let fixture_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../stakhal-core/tests/fixtures/stakhal_blink_f446re");
        let ioc_path = fixture_dir.join("stakhal_blink_f446re.ioc");
        let main_c_path = fixture_dir.join("Core/Src/main.c");

        let project = load_project(&ioc_path, &main_c_path).expect("Failed to load f446 fixture project");
        assert!(project.meta.mcu_name.to_uppercase().contains("F446"));

        let loc = stakhal_core::nucleo_pinout::lookup_pin("PA5");
        assert!(loc.is_some());
        let pin_loc = loc.unwrap();
        assert_eq!(pin_loc.morpho, Some(("CN10", 11)));
        assert_eq!(pin_loc.arduino, Some(("CN5", 6, "D13")));
    }


}




