use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use serde::{Deserialize, Serialize};
use slint::{ModelRc, VecModel};
use stakhal_core::ioc::discover_project_files;
use stakhal_core::ir::load_project;
use stakhal_core::source::pv_extract::PvDeclaration;
use stakhal_core::source::render_model::{build_source_render_model, LineTier, RenderedLine};
use stakhal_core::source::usage_finder::{find_variable_usages, UsageSite};
use stakhal_core::source::{scan_file, write_region, UserRegion};

slint::include_modules!();

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppConfig {
    project_dir: Option<String>,
}

#[derive(Default)]
struct LoadedState {
    ioc_path: PathBuf,
    main_c_path: PathBuf,
    user_regions: Vec<UserRegion>,
    pv_declarations: Vec<PvDeclaration>,
    pv_region_byte_range: Option<(usize, usize)>,
    active_pv_index: Option<usize>,
    active_usages: Vec<UsageSite>,
    rendered_lines: Vec<RenderedLine>,
    editing_line_index: Option<usize>,
    inline_error: Option<String>,
}

fn get_config_file_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join(".config").join("stakhal").join("last_project.json"))
}

fn load_app_config() -> AppConfig {
    if let Some(config_path) = get_config_file_path() {
        if let Ok(content) = fs::read_to_string(config_path) {
            if let Ok(config) = serde_json::from_str::<AppConfig>(&content) {
                return config;
            }
        }
    }
    AppConfig { project_dir: None }
}

fn save_app_config(dir: &str) {
    if let Some(config_path) = get_config_file_path() {
        if let Some(parent) = config_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let config = AppConfig {
            project_dir: Some(dir.to_string()),
        };
        if let Ok(json) = serde_json::to_string_pretty(&config) {
            let _ = fs::write(config_path, json);
        }
    }
}

fn update_source_panel_ui(ui: &MainWindow, state: &LoadedState) {
    let source_line_items: Vec<SourceLineItem> = state
        .rendered_lines
        .iter()
        .enumerate()
        .map(|(idx, line)| {
            let is_editing = state.editing_line_index == Some(idx);
            let (has_error, error_message) = if is_editing {
                match &state.inline_error {
                    Some(err_msg) => (true, err_msg.clone()),
                    None => (false, String::new()),
                }
            } else {
                (false, String::new())
            };

            let tier_str = match line.tier {
                LineTier::Declaration => "declaration",
                LineTier::Usage => "usage",
                LineTier::Normal => "normal",
                LineTier::Generated => "generated",
            };

            SourceLineItem {
                line_number: line.line_number as i32,
                text: line.text.clone().into(),
                tier_str: tier_str.into(),
                is_editing,
                has_error,
                error_message: error_message.into(),
            }
        })
        .collect();

    ui.set_source_lines(ModelRc::from(Rc::new(VecModel::from(source_line_items))));
}

fn load_project_into_ui(
    ui: &MainWindow,
    ioc_path: &Path,
    main_c_path: &Path,
    state: &RefCell<LoadedState>,
) -> Result<(), String> {
    let project = load_project(ioc_path, main_c_path).map_err(|e| e.to_string())?;

    let pv_region_range = project
        .user_regions
        .iter()
        .find(|r| r.tag == "PV")
        .map(|r| r.byte_range);

    let mut st = state.borrow_mut();
    st.ioc_path = ioc_path.to_path_buf();
    st.main_c_path = main_c_path.to_path_buf();
    st.user_regions = project.user_regions.clone();
    st.pv_declarations = project.pv_declarations.clone();
    st.pv_region_byte_range = pv_region_range;
    st.active_pv_index = None;
    st.active_usages.clear();
    st.rendered_lines.clear();
    st.editing_line_index = None;
    st.inline_error = None;

    ui.set_showing_source_view(false);
    ui.set_has_error(false);
    ui.set_error_message("".into());

    ui.set_project_name(project.meta.name.into());
    ui.set_mcu_family(project.meta.mcu_family.into());
    ui.set_mcu_name(project.meta.mcu_name.into());
    ui.set_project_loaded(true);

    let periph_items: Vec<PeripheralItem> = project
        .peripherals
        .into_iter()
        .map(|p| PeripheralItem {
            name: p.name.into(),
            mode: p.mode.unwrap_or_else(|| "—".to_string()).into(),
            param_count: p.parameters.len().to_string().into(),
        })
        .collect();
    ui.set_peripherals(ModelRc::from(Rc::new(VecModel::from(periph_items))));

    let mut region_items: Vec<RegionItem> = Vec::new();
    for r in project.user_regions {
        region_items.push(RegionItem {
            tag: r.tag.into(),
            byte_range: format!("({}, {})", r.byte_range.0, r.byte_range.1).into(),
            line_range: format!("({}, {})", r.line_range.0, r.line_range.1).into(),
            is_implicit: false,
        });
    }
    if let Some(lb) = project.loop_body {
        region_items.push(RegionItem {
            tag: lb.tag.into(),
            byte_range: format!("({}, {})", lb.byte_range.0, lb.byte_range.1).into(),
            line_range: format!("({}, {})", lb.line_range.0, lb.line_range.1).into(),
            is_implicit: true,
        });
    }
    ui.set_regions(ModelRc::from(Rc::new(VecModel::from(region_items))));

    let pv_items: Vec<PvItem> = project
        .pv_declarations
        .iter()
        .enumerate()
        .map(|(idx, d)| PvItem {
            name: d.name.clone().into(),
            type_str: d.type_str.clone().into(),
            initial_value: d.initial_value.clone().unwrap_or_else(|| "—".to_string()).into(),
            raw_text: d.raw_text.clone().into(),
            line: d.line as i32,
            pv_index: idx as i32,
        })
        .collect();
    ui.set_pv_variables(ModelRc::from(Rc::new(VecModel::from(pv_items))));

    Ok(())
}

fn main() -> Result<(), slint::PlatformError> {
    let ui = MainWindow::new()?;
    let state = Rc::new(RefCell::new(LoadedState::default()));

    let app_config = load_app_config();

    if let Some(saved_dir) = app_config.project_dir {
        let path = PathBuf::from(&saved_dir);
        if path.is_dir() {
            ui.set_project_dir(saved_dir.into());
            if let Ok((ioc_path, main_c_path)) = discover_project_files(&path) {
                ui.set_discovered_ioc_path(ioc_path.to_string_lossy().to_string().into());
                ui.set_discovered_main_c_path(main_c_path.to_string_lossy().to_string().into());
                ui.set_has_discovered_paths(true);
            }
        }
    }

    let ui_weak_folder = ui.as_weak();
    ui.on_browse_folder_clicked(move || {
        let ui = match ui_weak_folder.upgrade() {
            Some(ui) => ui,
            None => return,
        };

        if let Some(folder_path) = rfd::FileDialog::new().pick_folder() {
            let dir_str = folder_path.to_string_lossy().to_string();
            ui.set_project_dir(dir_str.clone().into());

            match discover_project_files(&folder_path) {
                Ok((ioc_path, main_c_path)) => {
                    ui.set_discovered_ioc_path(ioc_path.to_string_lossy().to_string().into());
                    ui.set_discovered_main_c_path(main_c_path.to_string_lossy().to_string().into());
                    ui.set_has_discovered_paths(true);
                    ui.set_has_error(false);
                    ui.set_error_message("".into());

                    save_app_config(&dir_str);
                }
                Err(err) => {
                    ui.set_has_discovered_paths(false);
                    ui.set_has_error(true);
                    ui.set_error_message(err.to_string().into());
                }
            }
        }
    });

    let ui_weak_load = ui.as_weak();
    let state_load = Rc::clone(&state);
    ui.on_load_clicked(move || {
        let ui = match ui_weak_load.upgrade() {
            Some(ui) => ui,
            None => return,
        };

        if !ui.get_has_discovered_paths() {
            return;
        }

        let ioc_path_str = ui.get_discovered_ioc_path().to_string();
        let main_c_path_str = ui.get_discovered_main_c_path().to_string();

        let ioc_path = Path::new(&ioc_path_str);
        let main_c_path = Path::new(&main_c_path_str);

        if let Err(err_msg) = load_project_into_ui(&ui, ioc_path, main_c_path, &state_load) {
            ui.set_has_error(true);
            ui.set_error_message(err_msg.into());
        }
    });

    let ui_weak_open_src = ui.as_weak();
    let state_open_src = Rc::clone(&state);
    ui.on_open_pv_source_view(move |idx_i32| {
        let ui = match ui_weak_open_src.upgrade() {
            Some(u) => u,
            None => return,
        };
        let idx = idx_i32 as usize;
        let mut st = state_open_src.borrow_mut();
        if idx >= st.pv_declarations.len() {
            return;
        }

        let decl = st.pv_declarations[idx].clone();
        let main_c_path = st.main_c_path.clone();
        let user_regions = st.user_regions.clone();

        let usages = find_variable_usages(&main_c_path, &decl.name, decl.byte_range)
            .unwrap_or_default();
        let usage_byte_ranges: Vec<(usize, usize)> = usages.iter().map(|u| u.byte_range).collect();

        let rendered_lines = build_source_render_model(
            &main_c_path,
            &user_regions,
            decl.byte_range,
            &usage_byte_ranges,
        )
        .unwrap_or_default();

        st.active_pv_index = Some(idx);
        st.active_usages = usages;
        st.rendered_lines = rendered_lines;
        st.editing_line_index = None;
        st.inline_error = None;

        ui.set_active_pv_name(decl.name.into());
        ui.set_active_pv_type(decl.type_str.into());
        ui.set_showing_source_view(true);
        ui.set_source_scroll_y(0.0_f32.into());


        update_source_panel_ui(&ui, &st);
    });

    let ui_weak_close_src = ui.as_weak();
    let state_close_src = Rc::clone(&state);
    ui.on_close_pv_source_view(move || {
        let ui = match ui_weak_close_src.upgrade() {
            Some(u) => u,
            None => return,
        };
        let mut st = state_close_src.borrow_mut();
        st.active_pv_index = None;
        st.editing_line_index = None;
        st.inline_error = None;
        ui.set_showing_source_view(false);
    });

    let ui_weak_click_decl = ui.as_weak();
    let state_click_decl = Rc::clone(&state);
    ui.on_click_declaration_line(move |idx_i32| {
        let ui = match ui_weak_click_decl.upgrade() {
            Some(u) => u,
            None => return,
        };
        let idx = idx_i32 as usize;
        let mut st = state_click_decl.borrow_mut();
        st.editing_line_index = Some(idx);
        st.inline_error = None;
        update_source_panel_ui(&ui, &st);
    });

    let ui_weak_cancel_decl = ui.as_weak();
    let state_cancel_decl = Rc::clone(&state);
    ui.on_cancel_declaration_edit(move |_idx_i32| {
        let ui = match ui_weak_cancel_decl.upgrade() {
            Some(u) => u,
            None => return,
        };
        let mut st = state_cancel_decl.borrow_mut();
        st.editing_line_index = None;
        st.inline_error = None;
        update_source_panel_ui(&ui, &st);
    });

    let ui_weak_save_decl = ui.as_weak();
    let state_save_decl = Rc::clone(&state);
    ui.on_save_declaration_edit(move |_line_idx_i32, new_raw_text_slint| {
        let ui = match ui_weak_save_decl.upgrade() {
            Some(u) => u,
            None => return,
        };
        let new_raw_text = new_raw_text_slint.to_string();

        let (main_c_path, ioc_path, loaded_pv_range, active_pv_idx) = {
            let st = state_save_decl.borrow();
            (
                st.main_c_path.clone(),
                st.ioc_path.clone(),
                st.pv_region_byte_range,
                st.active_pv_index,
            )
        };

        let active_idx = match active_pv_idx {
            Some(i) => i,
            None => return,
        };

        let orig_decl = {
            let st = state_save_decl.borrow();
            if active_idx >= st.pv_declarations.len() {
                return;
            }
            st.pv_declarations[active_idx].clone()
        };

        let fresh_regions = match scan_file(&main_c_path) {
            Ok(r) => r,
            Err(err) => {
                let mut st = state_save_decl.borrow_mut();
                st.inline_error = Some(err.to_string());
                update_source_panel_ui(&ui, &st);
                return;
            }
        };

        let fresh_pv_region = match fresh_regions.into_iter().find(|r| r.tag == "PV") {
            Some(r) => r,
            None => {
                let mut st = state_save_decl.borrow_mut();
                st.inline_error = Some("No PV region found in fresh scan".to_string());
                update_source_panel_ui(&ui, &st);
                return;
            }
        };

        if let Some(loaded_range) = loaded_pv_range {
            if fresh_pv_region.byte_range != loaded_range {
                let mut st = state_save_decl.borrow_mut();
                st.inline_error = Some(
                    "File has changed since project was loaded — reload project and try again"
                        .to_string(),
                );
                update_source_panel_ui(&ui, &st);
                return;
            }
        } else {
            let mut st = state_save_decl.borrow_mut();
            st.inline_error = Some(
                "File has changed since project was loaded — reload project and try again"
                    .to_string(),
            );
            update_source_panel_ui(&ui, &st);
            return;
        }

        let file_content = match fs::read_to_string(&main_c_path) {
            Ok(c) => c,
            Err(e) => {
                let mut st = state_save_decl.borrow_mut();
                st.inline_error = Some(e.to_string());
                update_source_panel_ui(&ui, &st);
                return;
            }
        };

        let pv_start = fresh_pv_region.byte_range.0;
        let pv_end = fresh_pv_region.byte_range.1;

        if pv_end > file_content.as_bytes().len() {
            let mut st = state_save_decl.borrow_mut();
            st.inline_error = Some("PV region byte range out of file bounds".to_string());
            update_source_panel_ui(&ui, &st);
            return;
        }

        let pv_full_text = &file_content[pv_start..pv_end];

        let decl_start = orig_decl.byte_range.0;
        let decl_end = orig_decl.byte_range.1;

        if decl_start < pv_start || decl_end > pv_end {
            let mut st = state_save_decl.borrow_mut();
            st.inline_error = Some("Declaration byte range out of PV region bounds".to_string());
            update_source_panel_ui(&ui, &st);
            return;
        }

        let decl_start_rel = decl_start - pv_start;
        let decl_end_rel = decl_end - pv_start;

        let mut new_pv_full_text = String::new();
        new_pv_full_text.push_str(&pv_full_text[..decl_start_rel]);
        new_pv_full_text.push_str(&new_raw_text);
        new_pv_full_text.push_str(&pv_full_text[decl_end_rel..]);

        match write_region(&main_c_path, &fresh_pv_region, &new_pv_full_text) {
            Ok(()) => {
                ui.set_showing_source_view(false);
                if let Err(err_msg) = load_project_into_ui(&ui, &ioc_path, &main_c_path, &state_save_decl) {
                    ui.set_has_error(true);
                    ui.set_error_message(err_msg.into());
                }
            }
            Err(err) => {
                let mut st = state_save_decl.borrow_mut();
                st.inline_error = Some(err.to_string());
                update_source_panel_ui(&ui, &st);
            }
        }
    });

    ui.run()
}
