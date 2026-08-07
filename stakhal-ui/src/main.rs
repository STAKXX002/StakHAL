use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use serde::{Deserialize, Serialize};
use slint::{ModelRc, VecModel};
use stakhal_core::ioc::discover_project_files;
use stakhal_core::ir::load_project;
use stakhal_core::source::pv_extract::PvDeclaration;

slint::include_modules!();

const DEFAULT_EDITOR_TEMPLATE: &str = "code --goto {path}:{line}:{col}";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppConfig {
    project_dir: Option<String>,
    editor_cmd_template: Option<String>,
}

#[derive(Default)]
struct LoadedState {
    ioc_path: PathBuf,
    main_c_path: PathBuf,
    pv_declarations: Vec<PvDeclaration>,
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
    AppConfig {
        project_dir: None,
        editor_cmd_template: None,
    }
}

fn save_app_config(dir: Option<&str>, template: Option<&str>) {
    if let Some(config_path) = get_config_file_path() {
        if let Some(parent) = config_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let mut current = load_app_config();
        if let Some(d) = dir {
            current.project_dir = Some(d.to_string());
        }
        if let Some(t) = template {
            current.editor_cmd_template = Some(t.to_string());
        }
        if let Ok(json) = serde_json::to_string_pretty(&current) {
            let _ = fs::write(config_path, json);
        }
    }
}

fn parse_cmd_line(cmd_line: &str) -> Option<(String, Vec<String>)> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut quote_char = ' ';

    for ch in cmd_line.chars() {
        match ch {
            '"' | '\'' if !in_quotes => {
                in_quotes = true;
                quote_char = ch;
            }
            q if in_quotes && q == quote_char => {
                in_quotes = false;
            }
            ' ' | '\t' if !in_quotes => {
                if !current.is_empty() {
                    args.push(current.clone());
                    current.clear();
                }
            }
            _ => {
                current.push(ch);
            }
        }
    }
    if !current.is_empty() {
        args.push(current);
    }

    if args.is_empty() {
        return None;
    }
    let program = args.remove(0);
    Some((program, args))
}

fn launch_editor(
    template: &str,
    main_c_path: &Path,
    line: usize,
) -> Result<(), String> {
    let path_str = main_c_path.to_string_lossy();
    let line_str = line.to_string();
    let col_str = "1";

    let expanded = template
        .replace("{path}", &path_str)
        .replace("{line}", &line_str)
        .replace("{col}", col_str);

    let (program, args) = parse_cmd_line(&expanded)
        .ok_or_else(|| "Editor command template is empty".to_string())?;

    std::process::Command::new(&program)
        .args(&args)
        .spawn()
        .map_err(|e| format!("Failed to launch editor: {} — check your editor command in settings.", e))?;

    Ok(())
}

fn load_project_into_ui(
    ui: &MainWindow,
    ioc_path: &Path,
    main_c_path: &Path,
    state: &RefCell<LoadedState>,
) -> Result<(), String> {
    let project = load_project(ioc_path, main_c_path).map_err(|e| e.to_string())?;

    let mut st = state.borrow_mut();
    st.ioc_path = ioc_path.to_path_buf();
    st.main_c_path = main_c_path.to_path_buf();
    st.pv_declarations = project.pv_declarations.clone();

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

    let editor_template = app_config
        .editor_cmd_template
        .unwrap_or_else(|| DEFAULT_EDITOR_TEMPLATE.to_string());
    ui.set_editor_cmd_template(editor_template.into());

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

    ui.on_editor_cmd_changed(move |new_template| {
        save_app_config(None, Some(&new_template.to_string()));
    });

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

                    save_app_config(Some(&dir_str), None);
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

    let ui_weak_open = ui.as_weak();
    let state_open = Rc::clone(&state);
    ui.on_open_pv_clicked(move |idx_i32| {
        let ui = match ui_weak_open.upgrade() {
            Some(u) => u,
            None => return,
        };
        let idx = idx_i32 as usize;
        let (main_c_path, line) = {
            let st = state_open.borrow();
            if idx >= st.pv_declarations.len() {
                return;
            }
            (st.main_c_path.clone(), st.pv_declarations[idx].line)
        };

        let template = ui.get_editor_cmd_template().to_string();

        match launch_editor(&template, &main_c_path, line) {
            Ok(()) => {
                ui.set_has_error(false);
                ui.set_error_message("".into());
            }
            Err(err_msg) => {
                ui.set_has_error(true);
                ui.set_error_message(err_msg.into());
            }
        }
    });

    ui.run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cmd_line_simple() {
        let (cmd, args) = parse_cmd_line("code --goto {path}:{line}:{col}").unwrap();
        assert_eq!(cmd, "code");
        assert_eq!(args, vec!["--goto", "{path}:{line}:{col}"]);
    }

    #[test]
    fn test_parse_cmd_line_quoted() {
        let (cmd, args) = parse_cmd_line("gedit +52 \"/path with spaces/main.c\"").unwrap();
        assert_eq!(cmd, "gedit");
        assert_eq!(args, vec!["+52", "/path with spaces/main.c"]);
    }

    #[test]
    fn test_launch_editor_invalid_command() {
        let res = launch_editor(
            "nonexistent_binary_xyz {path}:{line}",
            Path::new("/tmp/test.c"),
            42,
        );
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert!(err.contains("Failed to launch editor:"));
        assert!(err.contains("check your editor command in settings."));
    }
}


