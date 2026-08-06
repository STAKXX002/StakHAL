use std::path::Path;
use std::rc::Rc;

use slint::{ModelRc, VecModel};
use stakhal_core::ir::load_project;

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let ui = MainWindow::new()?;

    let ui_weak = ui.as_weak();

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
