use std::path::PathBuf;
use eframe::egui;

pub struct StakHalEguiApp {
    pub project_path: Option<PathBuf>,
    pub ioc_path: Option<PathBuf>,
    pub main_c_path: Option<PathBuf>,
    pub loaded_project: Option<stakhal_core::ioc::IocProject>,
    pub user_regions: Vec<stakhal_core::source::marker_scan::UserRegion>,
    pub pv_declarations: Vec<stakhal_core::source::pv_extract::PvDeclaration>,
    pub error_message: Option<String>,
}

impl Default for StakHalEguiApp {
    fn default() -> Self {
        let mut app = Self {
            project_path: None,
            ioc_path: None,
            main_c_path: None,
            loaded_project: None,
            user_regions: Vec::new(),
            pv_declarations: Vec::new(),
            error_message: None,
        };

        // Try loading last project from config if available
        if let Some(home) = std::env::var_os("HOME") {
            let config_path = PathBuf::from(home)
                .join(".config")
                .join("stakhal")
                .join("last_project.json");
            if let Ok(content) = std::fs::read_to_string(&config_path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(dir_str) = json.get("project_dir").and_then(|v| v.as_str()) {
                        let path = PathBuf::from(dir_str);
                        if path.exists() {
                            app.load_project_from_dir(path);
                        }
                    }
                }
            }
        }

        app
    }
}

impl StakHalEguiApp {
    pub fn load_project_from_dir(&mut self, dir: PathBuf) {
        self.project_path = Some(dir.clone());
        self.error_message = None;

        match stakhal_core::ioc::discover_project_files(&dir) {
            Ok((ioc_path, main_c_path)) => {
                self.ioc_path = Some(ioc_path.clone());
                self.main_c_path = Some(main_c_path.clone());

                match stakhal_core::ioc::parser::parse_ioc(&ioc_path) {
                    Ok(project) => {
                        self.loaded_project = Some(project);
                    }
                    Err(err) => {
                        self.error_message = Some(format!("IOC parse error: {}", err));
                        self.loaded_project = None;
                    }
                }

                match stakhal_core::source::marker_scan::scan_file(&main_c_path) {
                    Ok(mut regions) => {
                        if let Some(gap) = stakhal_core::source::marker_scan::find_loop_body_gap(&regions) {
                            regions.push(gap);
                        }
                        self.user_regions = regions;
                    }
                    Err(err) => {
                        self.user_regions.clear();
                        if self.error_message.is_none() {
                            self.error_message = Some(format!("User region scan error: {}", err));
                        }
                    }
                }

                match stakhal_core::source::pv_extract::extract_pv_declarations(&main_c_path) {
                    Ok(pvs) => {
                        self.pv_declarations = pvs;
                    }
                    Err(err) => {
                        self.pv_declarations.clear();
                        if self.error_message.is_none() {
                            self.error_message = Some(format!("PV extract error: {}", err));
                        }
                    }
                }
            }
            Err(err) => {
                self.error_message = Some(format!("Discovery error: {}", err));
                self.loaded_project = None;
                self.user_regions.clear();
                self.pv_declarations.clear();
            }
        }
    }

    fn open_folder_picker(&mut self) {
        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
            self.load_project_from_dir(folder);
        }
    }
}

impl eframe::App for StakHalEguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("📁 Open Folder").clicked() {
                    self.open_folder_picker();
                }

                ui.separator();

                if let Some(ref path) = self.project_path {
                    ui.label(format!("Project: {}", path.display()));
                } else {
                    ui.label("No folder selected");
                }
            });

            if self.ioc_path.is_some() || self.main_c_path.is_some() {
                ui.horizontal(|ui| {
                    if let Some(ref ioc) = self.ioc_path {
                        ui.label(format!("IOC: {}", ioc.display()));
                    }
                    ui.separator();
                    if let Some(ref main_c) = self.main_c_path {
                        ui.label(format!("Main C: {}", main_c.display()));
                    }
                });
            }

            if let Some(ref project) = self.loaded_project {
                ui.separator();
                ui.horizontal(|ui| {
                    let project_name = project
                        .raw
                        .get("ProjectManager.ProjectName")
                        .cloned()
                        .unwrap_or_else(|| "—".to_string());
                    ui.label(format!("NAME: {}", project_name));
                    ui.separator();
                    ui.label(format!("FAMILY: {}", project.mcu_family));
                    ui.separator();
                    ui.label(format!("MCU: {}", project.mcu_name));
                });
            }

            if let Some(ref err) = self.error_message {
                ui.separator();
                ui.colored_label(egui::Color32::RED, format!("Error: {}", err));
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.columns(3, |columns| {
                // Column 0: Peripherals
                columns[0].vertical(|ui| {
                    let count = self
                        .loaded_project
                        .as_ref()
                        .map_or(0, |p| p.peripherals.len());
                    ui.heading(format!("[ ▸ PERIPHERALS ({}) ]", count));
                    ui.separator();

                    egui::ScrollArea::vertical()
                        .id_salt("peripherals_scroll")
                        .show(ui, |ui| {
                            if let Some(ref project) = self.loaded_project {
                                if project.peripherals.is_empty() {
                                    ui.label("No peripherals declared");
                                } else {
                                    for periph in &project.peripherals {
                                        ui.group(|ui| {
                                            ui.horizontal(|ui| {
                                                ui.label(egui::RichText::new(&periph.name).strong());
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(egui::Align::Center),
                                                    |ui| {
                                                        ui.label(format!(
                                                            "{} params",
                                                            periph.parameters.len()
                                                        ));
                                                    },
                                                );
                                            });
                                            ui.label(periph.mode.as_deref().unwrap_or("—"));
                                        });
                                    }
                                }
                            } else {
                                ui.label("No project loaded");
                            }
                        });
                });

                // Column 1: User Regions
                columns[1].vertical(|ui| {
                    ui.heading(format!("[ ▸ USER REGIONS ({}) ]", self.user_regions.len()));
                    ui.separator();

                    egui::ScrollArea::vertical()
                        .id_salt("regions_scroll")
                        .show(ui, |ui| {
                            if !self.user_regions.is_empty() {
                                for region in &self.user_regions {
                                    ui.group(|ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new(&region.tag).strong());
                                            if region.tag == "__loop_body__" {
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(egui::Align::Center),
                                                    |ui| {
                                                        ui.label("[implicit]");
                                                    },
                                                );
                                            }
                                        });
                                        ui.label(format!(
                                            "L{}-L{} (bytes {}..{})",
                                            region.line_range.0,
                                            region.line_range.1,
                                            region.byte_range.0,
                                            region.byte_range.1
                                        ));
                                    });
                                }
                            } else {
                                ui.label("No regions found");
                            }
                        });
                });

                // Column 2: PV Variables
                columns[2].vertical(|ui| {
                    ui.heading(format!("[ ▸ PV VARIABLES ({}) ]", self.pv_declarations.len()));
                    ui.separator();

                    egui::ScrollArea::vertical()
                        .id_salt("pv_scroll")
                        .show(ui, |ui| {
                            if !self.pv_declarations.is_empty() {
                                for pv in &self.pv_declarations {
                                    ui.group(|ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new(&pv.name).strong());
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    ui.label(format!("Line {}", pv.line));
                                                },
                                            );
                                        });
                                        let subtitle = match &pv.initial_value {
                                            Some(val) => format!("{} = {}", pv.type_str, val),
                                            None => pv.type_str.clone(),
                                        };
                                        ui.label(subtitle);
                                    });
                                }
                            } else {
                                ui.label("No PV variables found");
                            }
                        });
                });
            });
        });
    }
}

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "StakHAL — Hardware Abstraction Inspector (egui)",
        native_options,
        Box::new(|_cc| Ok(Box::new(StakHalEguiApp::default()))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_egui_app_blink_fixture_loading() {
        let fixture_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../stakhal-core/tests/fixtures/stakhal_blink_f446re");
        assert!(fixture_dir.exists(), "Fixture directory must exist");

        let mut app = StakHalEguiApp::default();
        app.load_project_from_dir(fixture_dir);

        assert!(app.error_message.is_none(), "Expected no loading error, got: {:?}", app.error_message);
        assert!(app.loaded_project.is_some(), "Loaded project should be Some");
        
        let proj = app.loaded_project.as_ref().unwrap();
        assert_eq!(proj.mcu_family, "STM32F4");
        assert_eq!(proj.mcu_name, "STM32F446RETx");

        assert!(!app.user_regions.is_empty(), "User regions should not be empty");
        assert!(app.user_regions.iter().any(|r| r.tag == "Includes"));
        assert!(app.user_regions.iter().any(|r| r.tag == "__loop_body__"));
    }

    #[test]
    fn test_egui_app_timers_fixture_loading() {
        let fixture_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../stakhal-core/tests/fixtures/stm32_03_timers");
        assert!(fixture_dir.exists(), "Fixture directory must exist");

        let mut app = StakHalEguiApp::default();
        app.load_project_from_dir(fixture_dir);

        assert!(app.error_message.is_none(), "Expected no loading error, got: {:?}", app.error_message);
        assert!(app.loaded_project.is_some(), "Loaded project should be Some");

        let proj = app.loaded_project.as_ref().unwrap();
        assert_eq!(proj.peripherals.len(), 5);
        assert_eq!(app.pv_declarations.len(), 8);
        assert!(app.pv_declarations.iter().any(|pv| pv.name == "isrCount"));
    }
}
