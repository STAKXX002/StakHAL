use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use gtk4::glib;
use gtk4::prelude::*;
use libadwaita as adw;
use sourceview5::prelude::*;
use stakhal_core::source::marker_scan::scan_file;
use stakhal_core::source::pv_extract::PvDeclaration;
use stakhal_core::source::render_model::{build_source_render_model, LineTier};
use stakhal_core::source::usage_finder::find_variable_usages;
use stakhal_core::source::writeback::write_region;
use crate::state::{create_icon_button, AppState, AppWidgets, GeneratedRun};

pub struct SourcePanelWidgets {
    pub source_panel_box: gtk4::Box,
    pub btn_source_back: gtk4::Button,
    pub lbl_active_pv: gtk4::Label,
    pub btn_toggle_generated: gtk4::Button,
    pub source_view: sourceview5::View,
    pub source_buffer: sourceview5::Buffer,
    pub tag_declaration: gtk4::TextTag,
    pub tag_usage: gtk4::TextTag,
    pub tag_generated: gtk4::TextTag,
    pub tag_readonly: gtk4::TextTag,
    pub tag_invisible: gtk4::TextTag,
    pub inline_edit_bar: gtk4::Box,
    pub lbl_inline_error: gtk4::Label,
    pub btn_inline_save: gtk4::Button,
    pub btn_inline_cancel: gtk4::Button,
}

pub fn build_source_panel() -> SourcePanelWidgets {
    let btn_source_back = create_icon_button("Back to Overview", "go-previous-symbolic", false);

    let lbl_active_pv = gtk4::Label::builder()
        .label("PV Variable Source View")
        .halign(gtk4::Align::Start)
        .hexpand(true)
        .css_classes(vec!["title-3".to_string()])
        .build();

    let btn_toggle_generated = gtk4::Button::builder()
        .label("[ Show Generated ]")
        .css_classes(vec!["stakhal-btn".to_string(), "flat".to_string()])
        .build();
    btn_toggle_generated.set_cursor_from_name(Some("pointer"));

    let source_header_bar = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(18)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(18)
        .margin_end(18)
        .build();
    source_header_bar.append(&btn_source_back);
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
        .or_else(|| scheme_mgr.scheme("Adwaita-dark"))
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
        .css_classes(vec!["stakhal-btn".to_string(), "suggested-action".to_string()])
        .build();
    btn_inline_save.set_cursor_from_name(Some("pointer"));

    let btn_inline_cancel = gtk4::Button::builder()
        .label("Cancel")
        .css_classes(vec!["stakhal-btn".to_string()])
        .build();
    btn_inline_cancel.set_cursor_from_name(Some("pointer"));

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

    SourcePanelWidgets {
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
    }
}

pub fn open_pv_source_view(pv_idx: usize, state: &Rc<RefCell<AppState>>, widgets: &Rc<AppWidgets>) {
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

    let decl_line_idx = (decl.line.saturating_sub(1)) as i32;
    let source_view_clone = widgets.source_view.clone();
    let source_buffer_clone = widgets.source_buffer.clone();

    glib::timeout_add_local_once(std::time::Duration::from_millis(260), move || {
        if let Some(mut decl_iter) = source_buffer_clone.iter_at_line(decl_line_idx) {
            source_view_clone.scroll_to_iter(&mut decl_iter, 0.1, true, 0.0, 0.3);
        }
    });
}

pub fn apply_run_collapse(run: &GeneratedRun, widgets: &Rc<AppWidgets>) {
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

pub fn toggle_all_generated_runs(state: &Rc<RefCell<AppState>>, widgets: &Rc<AppWidgets>) {
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

pub fn enter_inline_edit_mode(state: &Rc<RefCell<AppState>>, widgets: &Rc<AppWidgets>) {
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

    if decl_line_idx > 0 {
        let start = widgets.source_buffer.start_iter();
        if let Some(end) = widgets.source_buffer.iter_at_line(decl_line_idx) {
            widgets.source_buffer.apply_tag(&widgets.tag_readonly, &start, &end);
        }
    }

    let after_line_idx = decl_line_idx + 1;
    if after_line_idx < line_count {
        if let Some(start) = widgets.source_buffer.iter_at_line(after_line_idx) {
            let end = widgets.source_buffer.end_iter();
            widgets.source_buffer.apply_tag(&widgets.tag_readonly, &start, &end);
        }
    }

    if let Some(mut iter) = widgets.source_buffer.iter_at_line(decl_line_idx) {
        widgets.source_buffer.place_cursor(&iter);
        widgets.source_view.scroll_to_iter(&mut iter, 0.1, true, 0.0, 0.5);
    }

    widgets.lbl_inline_error.set_visible(false);
    widgets.lbl_inline_error.set_text("");
    widgets.inline_edit_bar.set_visible(true);
    state.borrow_mut().is_inline_editing = true;
}

pub fn save_inline_declaration_edit<F>(
    state: &Rc<RefCell<AppState>>,
    widgets: &Rc<AppWidgets>,
    reload_project_fn: F,
) where
    F: Fn(&Rc<RefCell<AppState>>, &Rc<AppWidgets>),
{
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

            widgets.toast_overlay.add_toast(adw::Toast::new("Declaration saved successfully"));

            reload_project_fn(state, widgets);
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

pub fn cancel_inline_declaration_edit(state: &Rc<RefCell<AppState>>, widgets: &Rc<AppWidgets>) {
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
