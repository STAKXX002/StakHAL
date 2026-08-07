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
struct LastProjectConfig {
    project_dir: String,
}

#[derive(Default)]
struct LoadedState {
    ioc_path: PathBuf,
    main_c_path: PathBuf,
    pv_declarations: Vec<PvDeclaration>,
    pv_region_byte_range: Option<(usize, usize)>,
    active_edit_index: Option<usize>,
}

fn get_config_file_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join(".config").join("stakhal").join("last_project.json"))
}

fn load_last_folder() -> Option<String> {
    let config_path = get_config_file_path()?;
    let content = std::fs::read_to_string(config_path).ok()?;
    let config: LastProjectConfig = serde_json::from_str(&content).ok()?;
    Some(config.project_dir)
}

fn save_last_folder(dir: &str) {
    if let Some(config_path) = get_config_file_path() {
        if let Some(parent) = config_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let config = LastProjectConfig {
            project_dir: dir.to_string(),
        };
        if let Ok(json) = serde_json::to_string_pretty(&config) {
            let _ = std::fs::write(config_path, json);
        }
    }
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
    st.active_edit_index = None;

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
            byte_start: d.byte_range.0 as i32,
            byte_end: d.byte_range.1 as i32,
            pv_index: idx as i32,
        })
        .collect();
    ui.set_pv_variables(ModelRc::from(Rc::new(VecModel::from(pv_items))));

    Ok(())
}

fn main() -> Result<(), slint::PlatformError> {
    let ui = MainWindow::new()?;
    let state = Rc::new(RefCell::new(LoadedState::default()));

    // On startup, attempt to restore last used project folder and re-run discovery
    if let Some(saved_dir) = load_last_folder() {
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

                    save_last_folder(&dir_str);
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

    let ui_weak_edit = ui.as_weak();
    let state_edit = Rc::clone(&state);
    ui.on_edit_pv_clicked(move |idx_i32| {
        let ui = match ui_weak_edit.upgrade() {
            Some(u) => u,
            None => return,
        };
        let idx = idx_i32 as usize;
        let mut st = state_edit.borrow_mut();
        if idx < st.pv_declarations.len() {
            st.active_edit_index = Some(idx);
            let decl = &st.pv_declarations[idx];
            ui.set_edit_pv_name(decl.name.clone().into());
            ui.set_edit_pv_raw_text(decl.raw_text.clone().into());
            ui.set_edit_dialog_has_error(false);
            ui.set_edit_dialog_error_message("".into());
            ui.set_edit_dialog_visible(true);
        }
    });

    let ui_weak_cancel = ui.as_weak();
    let state_cancel = Rc::clone(&state);
    ui.on_cancel_pv_edit_clicked(move || {
        let ui = match ui_weak_cancel.upgrade() {
            Some(u) => u,
            None => return,
        };
        let mut st = state_cancel.borrow_mut();
        st.active_edit_index = None;
        ui.set_edit_dialog_visible(false);
    });

    let ui_weak_save = ui.as_weak();
    let state_save = Rc::clone(&state);
    ui.on_save_pv_edit_clicked(move || {
        let ui = match ui_weak_save.upgrade() {
            Some(u) => u,
            None => return,
        };

        let (main_c_path, ioc_path, loaded_pv_range, orig_decl) = {
            let st = state_save.borrow();
            let idx = match st.active_edit_index {
                Some(i) => i,
                None => return,
            };
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

        let new_raw_text = ui.get_edit_pv_raw_text().to_string();

        let fresh_regions = match scan_file(&main_c_path) {
            Ok(r) => r,
            Err(err) => {
                ui.set_edit_dialog_has_error(true);
                ui.set_edit_dialog_error_message(err.to_string().into());
                return;
            }
        };

        let fresh_pv_region = match fresh_regions.into_iter().find(|r| r.tag == "PV") {
            Some(r) => r,
            None => {
                ui.set_edit_dialog_has_error(true);
                ui.set_edit_dialog_error_message("No PV region found in fresh scan".into());
                return;
            }
        };

        if let Some(loaded_range) = loaded_pv_range {
            if fresh_pv_region.byte_range != loaded_range {
                ui.set_edit_dialog_has_error(true);
                ui.set_edit_dialog_error_message(
                    "File has changed since this was loaded — reload the project and try again".into(),
                );
                return;
            }
        } else {
            ui.set_edit_dialog_has_error(true);
            ui.set_edit_dialog_error_message(
                "File has changed since this was loaded — reload the project and try again".into(),
            );
            return;
        }

        let file_content = match fs::read_to_string(&main_c_path) {
            Ok(c) => c,
            Err(e) => {
                ui.set_edit_dialog_has_error(true);
                ui.set_edit_dialog_error_message(e.to_string().into());
                return;
            }
        };

        let pv_start = fresh_pv_region.byte_range.0;
        let pv_end = fresh_pv_region.byte_range.1;

        if pv_end > file_content.as_bytes().len() {
            ui.set_edit_dialog_has_error(true);
            ui.set_edit_dialog_error_message("PV region byte range out of file bounds".into());
            return;
        }

        let pv_full_text = &file_content[pv_start..pv_end];

        let decl_start = orig_decl.byte_range.0;
        let decl_end = orig_decl.byte_range.1;

        if decl_start < pv_start || decl_end > pv_end {
            ui.set_edit_dialog_has_error(true);
            ui.set_edit_dialog_error_message("Declaration byte range out of PV region bounds".into());
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
                ui.set_edit_dialog_visible(false);
                if let Err(err_msg) = load_project_into_ui(&ui, &ioc_path, &main_c_path, &state_save) {
                    ui.set_has_error(true);
                    ui.set_error_message(err_msg.into());
                }
            }
            Err(err) => {
                ui.set_edit_dialog_has_error(true);
                ui.set_edit_dialog_error_message(err.to_string().into());
            }
        }
    });

    ui.run()
}
