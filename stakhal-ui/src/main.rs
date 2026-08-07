use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use serde::{Deserialize, Serialize};
use slint::{ModelRc, VecModel};
use stakhal_core::ioc::discover_project_files;
use stakhal_core::ir::load_project;
use stakhal_core::source::pv_extract::PvDeclaration;
use stakhal_core::source::scan_file;
use stakhal_core::source::write_region;

slint::include_modules!();

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppConfig {
    project_dir: Option<String>,
}

#[derive(Default)]
struct LoadedState {
    ioc_path: PathBuf,
    main_c_path: PathBuf,
    pv_declarations: Vec<PvDeclaration>,
    pv_region_byte_range: Option<(usize, usize)>,
    expanded_pv_index: Option<usize>,
    inline_error: Option<(usize, String)>,
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

fn update_pv_ui_model(ui: &MainWindow, state: &LoadedState) {
    let pv_items: Vec<PvItem> = state
        .pv_declarations
        .iter()
        .enumerate()
        .map(|(idx, d)| {
            let is_expanded = state.expanded_pv_index == Some(idx);
            let (has_error, error_message) = match &state.inline_error {
                Some((err_idx, msg)) if *err_idx == idx => (true, msg.clone()),
                _ => (false, String::new()),
            };

            PvItem {
                name: d.name.clone().into(),
                type_str: d.type_str.clone().into(),
                initial_value: d.initial_value.clone().unwrap_or_else(|| "—".to_string()).into(),
                raw_text: d.raw_text.clone().into(),
                line: d.line as i32,
                pv_index: idx as i32,
                is_expanded,
                has_error,
                error_message: error_message.into(),
            }
        })
        .collect();

    ui.set_pv_variables(ModelRc::from(Rc::new(VecModel::from(pv_items))));
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
    st.pv_declarations = project.pv_declarations.clone();
    st.pv_region_byte_range = pv_region_range;
    st.expanded_pv_index = None;
    st.inline_error = None;

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

    update_pv_ui_model(ui, &st);

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

    let ui_weak_toggle = ui.as_weak();
    let state_toggle = Rc::clone(&state);
    ui.on_toggle_pv_expanded(move |idx_i32| {
        let ui = match ui_weak_toggle.upgrade() {
            Some(u) => u,
            None => return,
        };
        let idx = idx_i32 as usize;
        let mut st = state_toggle.borrow_mut();
        if idx >= st.pv_declarations.len() {
            return;
        }

        if st.expanded_pv_index == Some(idx) {
            st.expanded_pv_index = None;
            st.inline_error = None;
        } else {
            st.expanded_pv_index = Some(idx);
            st.inline_error = None;
        }
        update_pv_ui_model(&ui, &st);
    });

    let ui_weak_cancel = ui.as_weak();
    let state_cancel = Rc::clone(&state);
    ui.on_cancel_pv_inline_edit(move |_idx_i32| {
        let ui = match ui_weak_cancel.upgrade() {
            Some(u) => u,
            None => return,
        };
        let mut st = state_cancel.borrow_mut();
        st.expanded_pv_index = None;
        st.inline_error = None;
        update_pv_ui_model(&ui, &st);
    });

    let ui_weak_save = ui.as_weak();
    let state_save = Rc::clone(&state);
    ui.on_save_pv_inline_edit(move |idx_i32, new_raw_text_slint| {
        let ui = match ui_weak_save.upgrade() {
            Some(u) => u,
            None => return,
        };
        let idx = idx_i32 as usize;
        let new_raw_text = new_raw_text_slint.to_string();

        let (main_c_path, ioc_path, loaded_pv_range, orig_decl) = {
            let st = state_save.borrow();
            if idx >= st.pv_declarations.len() {
                return;
            }
            (
                st.main_c_path.clone(),
                st.ioc_path.clone(),
                st.pv_region_byte_range,
                st.pv_declarations[idx].clone(),
            )
        };

        let fresh_regions = match scan_file(&main_c_path) {
            Ok(r) => r,
            Err(err) => {
                let mut st = state_save.borrow_mut();
                st.inline_error = Some((idx, err.to_string()));
                update_pv_ui_model(&ui, &st);
                return;
            }
        };

        let fresh_pv_region = match fresh_regions.into_iter().find(|r| r.tag == "PV") {
            Some(r) => r,
            None => {
                let mut st = state_save.borrow_mut();
                st.inline_error = Some((idx, "No PV region found in fresh scan".to_string()));
                update_pv_ui_model(&ui, &st);
                return;
            }
        };

        if let Some(loaded_range) = loaded_pv_range {
            if fresh_pv_region.byte_range != loaded_range {
                let mut st = state_save.borrow_mut();
                st.inline_error = Some((
                    idx,
                    "File has changed since project was loaded — reload project and try again"
                        .to_string(),
                ));
                update_pv_ui_model(&ui, &st);
                return;
            }
        } else {
            let mut st = state_save.borrow_mut();
            st.inline_error = Some((
                idx,
                "File has changed since project was loaded — reload project and try again"
                    .to_string(),
            ));
            update_pv_ui_model(&ui, &st);
            return;
        }

        let file_content = match fs::read_to_string(&main_c_path) {
            Ok(c) => c,
            Err(e) => {
                let mut st = state_save.borrow_mut();
                st.inline_error = Some((idx, e.to_string()));
                update_pv_ui_model(&ui, &st);
                return;
            }
        };

        let pv_start = fresh_pv_region.byte_range.0;
        let pv_end = fresh_pv_region.byte_range.1;

        if pv_end > file_content.as_bytes().len() {
            let mut st = state_save.borrow_mut();
            st.inline_error = Some((idx, "PV region byte range out of file bounds".to_string()));
            update_pv_ui_model(&ui, &st);
            return;
        }

        let pv_full_text = &file_content[pv_start..pv_end];

        let decl_start = orig_decl.byte_range.0;
        let decl_end = orig_decl.byte_range.1;

        if decl_start < pv_start || decl_end > pv_end {
            let mut st = state_save.borrow_mut();
            st.inline_error = Some((
                idx,
                "Declaration byte range out of PV region bounds".to_string(),
            ));
            update_pv_ui_model(&ui, &st);
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
                if let Err(err_msg) = load_project_into_ui(&ui, &ioc_path, &main_c_path, &state_save) {
                    ui.set_has_error(true);
                    ui.set_error_message(err_msg.into());
                }
            }
            Err(err) => {
                let mut st = state_save.borrow_mut();
                st.inline_error = Some((idx, err.to_string()));
                update_pv_ui_model(&ui, &st);
            }
        }
    });

    ui.run()
}
