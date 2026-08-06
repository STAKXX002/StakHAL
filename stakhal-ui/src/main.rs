use std::path::{Path, PathBuf};
use std::rc::Rc;

use serde::{Deserialize, Serialize};
use slint::{ModelRc, VecModel};
use stakhal_core::ir::load_project;

slint::include_modules!();

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LastProjectConfig {
    ioc_path: String,
    main_c_path: String,
}

fn get_config_file_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join(".config").join("stakhal").join("last_project.json"))
}

fn load_last_paths() -> Option<(String, String)> {
    let config_path = get_config_file_path()?;
    let content = std::fs::read_to_string(config_path).ok()?;
    let config: LastProjectConfig = serde_json::from_str(&content).ok()?;
    Some((config.ioc_path, config.main_c_path))
}

fn save_last_paths(ioc_path: &str, main_c_path: &str) {
    if let Some(config_path) = get_config_file_path() {
        if let Some(parent) = config_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let config = LastProjectConfig {
            ioc_path: ioc_path.to_string(),
            main_c_path: main_c_path.to_string(),
        };
        if let Ok(json) = serde_json::to_string_pretty(&config) {
            let _ = std::fs::write(config_path, json);
        }
    }
}

fn main() -> Result<(), slint::PlatformError> {
    let ui = MainWindow::new()?;

    // On startup, attempt to restore last used paths (do NOT auto-load)
    if let Some((saved_ioc, saved_main_c)) = load_last_paths() {
        ui.set_ioc_path(saved_ioc.into());
        ui.set_main_c_path(saved_main_c.into());
    }

    let ui_weak = ui.as_weak();

    // Browse .ioc button handler
    let ui_weak_ioc = ui.as_weak();
    ui.on_browse_ioc_clicked(move || {
        let ui = match ui_weak_ioc.upgrade() {
            Some(ui) => ui,
            None => return,
        };

        if let Some(path) = rfd::FileDialog::new()
            .add_filter("CubeMX Project", &["ioc"])
            .pick_file()
        {
            ui.set_ioc_path(path.to_string_lossy().to_string().into());
        }
    });

    // Browse main.c button handler
    let ui_weak_c = ui.as_weak();
    ui.on_browse_main_c_clicked(move || {
        let ui = match ui_weak_c.upgrade() {
            Some(ui) => ui,
            None => return,
        };

        if let Some(path) = rfd::FileDialog::new()
            .add_filter("C Source File", &["c"])
            .add_filter("All Files", &["*"])
            .pick_file()
        {
            ui.set_main_c_path(path.to_string_lossy().to_string().into());
        }
    });

    // Load Project button handler
    ui.on_load_clicked(move || {
        let ui = match ui_weak.upgrade() {
            Some(ui) => ui,
            None => return,
        };

        let ioc_path_str = ui.get_ioc_path().to_string();
        let main_c_path_str = ui.get_main_c_path().to_string();

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

                // On successful load, persist paths to ~/.config/stakhal/last_project.json
                save_last_paths(&ioc_path_str, &main_c_path_str);
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
