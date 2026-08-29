use std::path::{Path, PathBuf};
use eframe::egui;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum View {
    Inspector,
    Source { pv_index: usize },
}

pub struct StakHalEguiApp {
    pub project_path: Option<PathBuf>,
    pub ioc_path: Option<PathBuf>,
    pub main_c_path: Option<PathBuf>,
    pub loaded_project: Option<stakhal_core::ioc::IocProject>,
    pub user_regions: Vec<stakhal_core::source::marker_scan::UserRegion>,
    pub pv_declarations: Vec<stakhal_core::source::pv_extract::PvDeclaration>,
    pub error_message: Option<String>,

    pub current_view: View,
    pub main_c_code: String,
    pub inline_editing: bool,
    pub edit_buffer: String,
    pub edit_error: Option<String>,
    pub should_scroll_to_decl: bool,

    pub syntax_set: SyntaxSet,
    pub theme_set: ThemeSet,
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

            current_view: View::Inspector,
            main_c_code: String::new(),
            inline_editing: false,
            edit_buffer: String::new(),
            edit_error: None,
            should_scroll_to_decl: false,

            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
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

                if let Ok(code) = std::fs::read_to_string(&main_c_path) {
                    self.main_c_code = code;
                } else {
                    self.main_c_code.clear();
                }

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
                self.main_c_code.clear();
            }
        }
    }

    fn open_folder_picker(&mut self) {
        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
            self.load_project_from_dir(folder);
        }
    }

    fn highlight_c(&self, code: &str, font_id: egui::FontId) -> egui::text::LayoutJob {
        let mut job = egui::text::LayoutJob::default();
        let syntax = self
            .syntax_set
            .find_syntax_by_extension("c")
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());
        let theme = self
            .theme_set
            .themes
            .get("base16-ocean.dark")
            .or_else(|| self.theme_set.themes.values().next())
            .expect("Theme required");
        let mut highlighter = HighlightLines::new(syntax, theme);

        for line in code.lines() {
            if let Ok(ranges) = highlighter.highlight_line(line, &self.syntax_set) {
                for (style, text) in ranges {
                    let color = egui::Color32::from_rgb(
                        style.foreground.r,
                        style.foreground.g,
                        style.foreground.b,
                    );
                    job.append(
                        text,
                        0.0,
                        egui::TextFormat {
                            font_id: font_id.clone(),
                            color,
                            ..Default::default()
                        },
                    );
                }
            } else {
                job.append(
                    line,
                    0.0,
                    egui::TextFormat {
                        font_id: font_id.clone(),
                        color: egui::Color32::LIGHT_GRAY,
                        ..Default::default()
                    },
                );
            }
            job.append(
                "\n",
                0.0,
                egui::TextFormat {
                    font_id: font_id.clone(),
                    color: egui::Color32::TRANSPARENT,
                    ..Default::default()
                },
            );
        }

        job
    }
}

impl eframe::App for StakHalEguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            match self.current_view {
                View::Inspector => {
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
                }
                View::Source { pv_index } => {
                    ui.horizontal(|ui| {
                        if ui.button("← Back").clicked() {
                            self.current_view = View::Inspector;
                            self.inline_editing = false;
                            self.edit_error = None;
                        }

                        ui.separator();

                        if let Some(pv) = self.pv_declarations.get(pv_index) {
                            ui.label(
                                egui::RichText::new(format!(
                                    "[ PV VARIABLE: {} {} (Line {}) ]",
                                    pv.type_str, pv.name, pv.line
                                ))
                                .strong(),
                            );
                        }

                        if let Some(ref main_c) = self.main_c_path {
                            ui.separator();
                            ui.label(format!("Source: {}", main_c.display()));
                        }
                    });

                    if let Some(ref err) = self.edit_error {
                        ui.separator();
                        ui.colored_label(egui::Color32::RED, format!("Save Error: {}", err));
                    }
                }
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            match self.current_view {
                View::Inspector => {
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

                            let mut clicked_pv_index = None;
                            egui::ScrollArea::vertical()
                                .id_salt("pv_scroll")
                                .show(ui, |ui| {
                                    if !self.pv_declarations.is_empty() {
                                        for (idx, pv) in self.pv_declarations.iter().enumerate() {
                                            let card_response = ui.group(|ui| {
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

                                            let interact_resp = ui.interact(
                                                card_response.response.rect,
                                                card_response.response.id,
                                                egui::Sense::click(),
                                            );
                                            if interact_resp.clicked() {
                                                clicked_pv_index = Some(idx);
                                            }
                                        }
                                    } else {
                                        ui.label("No PV variables found");
                                    }
                                });

                            if let Some(idx) = clicked_pv_index {
                                self.current_view = View::Source { pv_index: idx };
                                self.inline_editing = false;
                                self.should_scroll_to_decl = true;
                                self.edit_error = None;
                            }
                        });
                    });
                }
                View::Source { pv_index } => {
                    let active_pv = self.pv_declarations.get(pv_index).cloned();

                    if self.inline_editing {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Edit Declaration:").strong());
                                let text_edit = egui::TextEdit::singleline(&mut self.edit_buffer)
                                    .font(egui::FontId::monospace(13.0))
                                    .desired_width(450.0);
                                ui.add(text_edit);

                                if ui.button("💾 Save").clicked() {
                                    if let (Some(pv), Some(ref main_c_path)) = (&active_pv, &self.main_c_path) {
                                        match save_pv_declaration_edit(
                                            main_c_path,
                                            pv,
                                            self.edit_buffer.trim(),
                                        ) {
                                            Ok(_) => {
                                                println!("[WRITEBACK SUCCESS] Declaration saved to disk");
                                                if let Some(ref dir) = self.project_path.clone() {
                                                    self.load_project_from_dir(dir.clone());
                                                }
                                                self.inline_editing = false;
                                                self.edit_error = None;
                                            }
                                            Err(err) => {
                                                println!("[WRITEBACK ERROR] Failed: {}", err);
                                                self.edit_error = Some(err);
                                            }
                                        }
                                    }
                                }

                                if ui.button("❌ Cancel").clicked() {
                                    self.inline_editing = false;
                                    self.edit_error = None;
                                }
                            });
                        });
                        ui.separator();
                    }

                    let font_id = egui::FontId::monospace(13.0);
                    let row_height = ui.fonts(|f| f.row_height(&font_id));

                    let target_line = active_pv.as_ref().map(|pv| pv.line).unwrap_or(1);
                    let mut scroll_area = egui::ScrollArea::vertical().id_salt("source_code_scroll");

                    if self.should_scroll_to_decl {
                        let target_scroll_y =
                            ((target_line.saturating_sub(1) as f32) * row_height - 150.0).max(0.0);
                        scroll_area = scroll_area.scroll_offset(egui::Vec2::new(0.0, target_scroll_y));
                        self.should_scroll_to_decl = false;
                    }

                    scroll_area.show(ui, |ui| {
                        let mut code = self.main_c_code.clone();
                        let mut layouter = |ui: &egui::Ui, string: &str, _: f32| {
                            let job = self.highlight_c(string, font_id.clone());
                            ui.fonts(|f| f.layout_job(job))
                        };

                        let text_edit = egui::TextEdit::multiline(&mut code)
                            .font(font_id.clone())
                            .code_editor()
                            .interactive(false)
                            .desired_width(f32::INFINITY)
                            .layouter(&mut layouter);

                        let response = ui.add(text_edit);

                        response.context_menu(|ui| {
                            println!("[CONTEXT MENU] Context menu requested on source view");

                            if ui.button("📋 Copy").clicked() {
                                ui.ctx().copy_text(self.main_c_code.clone());
                                println!("[CONTEXT MENU] Copy action executed");
                                ui.close_menu();
                            }

                            let mut clicked_line_opt = None;
                            if let Some(pos) = ui.ctx().pointer_latest_pos() {
                                let relative_y = pos.y - response.rect.min.y;
                                if relative_y >= 0.0 && row_height > 0.0 {
                                    let line = (relative_y / row_height).floor() as usize + 1;
                                    clicked_line_opt = Some(line);
                                }
                            }

                            println!("[CONTEXT MENU] Right-click detected on line {:?}", clicked_line_opt);

                            if let Some(pv) = active_pv.as_ref() {
                                if clicked_line_opt == Some(pv.line) || clicked_line_opt.is_none() {
                                    ui.separator();
                                    if ui.button("✏ Edit Declaration").clicked() {
                                        self.inline_editing = true;
                                        self.edit_buffer = get_line_content(&self.main_c_code, pv.line)
                                            .unwrap_or_else(|| pv.raw_text.clone());
                                        println!("[CONTEXT MENU] Edit Declaration selected for PV '{}' at line {}", pv.name, pv.line);
                                        ui.close_menu();
                                    }
                                }
                            }
                        });
                    });
                }
            }
        });
    }
}

fn save_pv_declaration_edit(
    main_c_path: &Path,
    decl: &stakhal_core::source::pv_extract::PvDeclaration,
    new_text: &str,
) -> Result<(), String> {
    let fresh_regions = stakhal_core::source::marker_scan::scan_file(main_c_path)
        .map_err(|e| e.to_string())?;
    let pv_region = fresh_regions
        .iter()
        .find(|r| r.tag == "PV")
        .ok_or_else(|| "PV region not found in main.c".to_string())?;

    let full_content = std::fs::read_to_string(main_c_path).map_err(|e| e.to_string())?;

    let (line_start, line_end) = find_line_byte_range(&full_content, decl.line)
        .ok_or_else(|| format!("Line {} not found in file", decl.line))?;

    if line_start < pv_region.byte_range.0 || line_end > pv_region.byte_range.1 {
        return Err("Line byte range is outside current PV region".to_string());
    }

    let offset_start = line_start - pv_region.byte_range.0;
    let offset_end = line_end - pv_region.byte_range.0;

    let mut pv_content = full_content[pv_region.byte_range.0..pv_region.byte_range.1].to_string();
    if offset_start > pv_content.len() || offset_end > pv_content.len() {
        return Err("Invalid byte range offsets inside PV region".to_string());
    }

    pv_content.replace_range(offset_start..offset_end, new_text);

    stakhal_core::source::writeback::write_region(main_c_path, pv_region, &pv_content)
        .map_err(|e| e.to_string())
}

fn find_line_byte_range(content: &str, line_1based: usize) -> Option<(usize, usize)> {
    if line_1based == 0 {
        return None;
    }
    let mut current_line = 1;
    let mut line_start = 0;

    for (idx, ch) in content.char_indices() {
        if current_line == line_1based {
            let mut line_end = idx;
            for (end_idx, end_ch) in content[idx..].char_indices() {
                let absolute_idx = idx + end_idx;
                if end_ch == '\n' || end_ch == '\r' {
                    line_end = absolute_idx;
                    break;
                }
                line_end = absolute_idx + end_ch.len_utf8();
            }
            return Some((line_start, line_end));
        }

        if ch == '\n' {
            current_line += 1;
            line_start = idx + 1;
        }
    }

    if current_line == line_1based {
        return Some((line_start, content.len()));
    }

    None
}

fn get_line_content(content: &str, line_1based: usize) -> Option<String> {
    let (start, end) = find_line_byte_range(content, line_1based)?;
    Some(content[start..end].trim_end_matches(&['\r', '\n'][..]).to_string())
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

    #[test]
    fn test_find_line_byte_range_and_get_content() {
        let content = "line1\nline2\nline3";
        assert_eq!(get_line_content(content, 1), Some("line1".to_string()));
        assert_eq!(get_line_content(content, 2), Some("line2".to_string()));
        assert_eq!(get_line_content(content, 3), Some("line3".to_string()));
        assert_eq!(get_line_content(content, 4), None);
    }

    #[test]
    fn test_source_view_and_context_menu_simulation() {
        let fixture_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../stakhal-core/tests/fixtures/stm32_03_timers");
        assert!(fixture_dir.exists(), "Fixture directory must exist");

        let mut app = StakHalEguiApp::default();
        app.load_project_from_dir(fixture_dir);

        assert!(!app.pv_declarations.is_empty(), "PV declarations should exist");
        
        // 1. Simulate clicking a PV variable row (e.g. pv_index = 0)
        app.current_view = View::Source { pv_index: 0 };
        app.should_scroll_to_decl = true;

        let active_pv = &app.pv_declarations[0];
        assert_eq!(active_pv.name, "isrCount");

        // 2. Perform 15 right-click position checks simulating context menu evaluation
        for line in 1..=15 {
            let line_content = get_line_content(&app.main_c_code, line);
            println!("[TEST SIMULATION] Right-click #{} evaluated at line {}: {:?}", line, line, line_content);
            if line == active_pv.line {
                println!("[TEST SIMULATION] Right-click landed on PV declaration line {} ('{}') - Edit option enabled", line, active_pv.name);
            } else {
                println!("[TEST SIMULATION] Right-click landed on line {} - Copy option enabled", line);
            }
        }

        // 3. Test inline edit and writeback simulation on temp copy
        let dir = tempfile::tempdir().unwrap();
        let main_c_path = dir.path().join("main.c");
        std::fs::write(&main_c_path, &app.main_c_code).unwrap();
        let ioc_path = dir.path().join("03_timers.ioc");
        std::fs::write(&ioc_path, "PCC.Checker=true\n").unwrap();

        let res = save_pv_declaration_edit(&main_c_path, active_pv, "uint32_t isrCount = 100;");
        assert!(res.is_ok(), "Expected writeback success, got: {:?}", res);

        let updated = std::fs::read_to_string(&main_c_path).unwrap();
        assert!(updated.contains("uint32_t isrCount = 100;"));
    }
}
