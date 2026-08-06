use std::path::{Path, PathBuf};
use std::rc::Rc;

use serde::{Deserialize, Serialize};
use slint::{ModelRc, VecModel};
use stakhal_core::ioc::discover_project_files;
use stakhal_core::ir::load_project;

slint::include_modules!();

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LastProjectConfig {
    project_dir: String,
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

fn main() -> Result<(), slint::PlatformError> {
    let ui = MainWindow::new()?;

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

        match load_project(ioc_path, main_c_path) {
            Ok(project) => {
                // Clear error
                ui.set_has_error(false);
                ui.set_error_message("".into());

                // Set header info
                ui.set_project_name(project.meta.name.into());
                ui.set_mcu_family(project.meta.mcu_family.into());
                ui.set_mcu_name(project.meta.mcu_name.into());
                ui.set_project_loaded(true);

                // Populate Peripherals
                let periph_items: Vec<PeripheralItem> = project
                    .peripherals
                    .into_iter()
                    .map(|p| PeripheralItem {
                        name: p.name.into(),
                        mode: p.mode.unwrap_or_else(|| "—".to_string()).into(),
                        param_count: p.parameters.len().to_string().into(),
                    })
                    .collect();
                let periph_model: Rc<VecModel<PeripheralItem>> = Rc::new(VecModel::from(periph_items));
                ui.set_peripherals(ModelRc::from(periph_model));

                // Populate User Regions (including loop_body if Some)
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
                let region_model: Rc<VecModel<RegionItem>> = Rc::new(VecModel::from(region_items));
                ui.set_regions(ModelRc::from(region_model));
            }
            Err(err) => {
                // Show error banner, do NOT clear previously loaded data
                ui.set_has_error(true);
                ui.set_error_message(err.to_string().into());
            }
        }
    });

    ui.run()
}
