use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

use crate::graph::builder::{build_call_graph, GraphEdge};
use crate::ioc::parser::{parse_ioc, IocParseError, PeripheralConfig, PinConfig};
use crate::source::marker_scan::{find_loop_body_gap, scan_file, ScanError, UserRegion};
use crate::source::pv_extract::{extract_pv_declarations, PvDeclaration, PvExtractError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMeta {
    pub name: String, // derived from the .ioc filename stem, no extension
    pub mcu_family: String,
    pub mcu_name: String,
    pub ioc_path: PathBuf,
    pub main_c_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub meta: ProjectMeta,
    pub pins: Vec<PinConfig>,
    pub peripherals: Vec<PeripheralConfig>,
    pub user_regions: Vec<UserRegion>,
    pub loop_body: Option<UserRegion>, // from find_loop_body_gap, may be None
    pub call_graph_edges: Vec<GraphEdge>,
    pub pv_declarations: Vec<PvDeclaration>,
    #[serde(default)]
    pub state_machines: Vec<crate::graph::state_machine::AppStateMachine>,
    #[serde(default)]
    pub modules: Vec<String>,
}

#[derive(thiserror::Error, Debug)]
pub enum ProjectLoadError {
    #[error(transparent)]
    IocError(#[from] IocParseError),
    #[error(transparent)]
    ScanError(#[from] ScanError),
    #[error(transparent)]
    PvExtractError(#[from] PvExtractError),
}

pub fn load_project(ioc_path: &Path, main_c_path: &Path) -> Result<Project, ProjectLoadError> {
    let ioc = parse_ioc(ioc_path)?;
    let user_regions = scan_file(main_c_path)?;
    let loop_body = find_loop_body_gap(&user_regions);
    let call_graph_edges = build_call_graph(&ioc);

    let pv_declarations = match extract_pv_declarations(main_c_path) {
        Ok(decls) => decls,
        Err(PvExtractError::NoPvRegion) => vec![],
        Err(e) => return Err(ProjectLoadError::PvExtractError(e)),
    };

    let state_machines = match crate::graph::state_machine::discover_state_machines_in_project(main_c_path) {
        Ok(candidates) => {
            let mut list = Vec::new();
            for c in candidates {
                let file_path = Path::new(&c.var.file_path);
                if let Ok(sm) = crate::graph::state_machine::extract_state_machine_transitions(&c, file_path) {
                    list.push(sm);
                }
            }
            list
        }
        Err(_) => vec![],
    };

    let name = ioc_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("project")
        .to_string();

    let meta = ProjectMeta {
        name,
        mcu_family: ioc.mcu_family,
        mcu_name: ioc.mcu_name,
        ioc_path: ioc_path.to_path_buf(),
        main_c_path: main_c_path.to_path_buf(),
    };

    let (pins, modules) = crate::source::pin_modules::scan_pin_modules(&ioc.pins, main_c_path);

    Ok(Project {
        meta,
        pins,
        peripherals: ioc.peripherals,
        user_regions,
        loop_body,
        call_graph_edges,
        pv_declarations,
        state_machines,
        modules,
    })
}

pub fn project_to_json(project: &Project) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(project)
}

pub fn project_from_json(json: &str) -> Result<Project, serde_json::Error> {
    serde_json::from_str(json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::builder::EdgeType;

    fn blink_fixture_paths() -> (PathBuf, PathBuf) {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let fixture_dir = manifest_dir.join("tests/fixtures/stakhal_blink_f446re");
        (
            fixture_dir.join("stakhal_blink_f446re.ioc"),
            fixture_dir.join("Core/Src/main.c"),
        )
    }

    fn timers_fixture_paths() -> (PathBuf, PathBuf) {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let fixture_dir = manifest_dir.join("tests/fixtures/stm32_03_timers");
        (
            fixture_dir.join("03_timers.ioc"),
            fixture_dir.join("Core/Src/main.c"),
        )
    }

    #[test]
    fn test_load_project_stakhal_blink_f446re() {
        let (ioc_path, main_c_path) = blink_fixture_paths();
        let project = load_project(&ioc_path, &main_c_path).expect("failed to load blink project");

        assert_eq!(project.meta.name, "stakhal_blink_f446re");
        assert_eq!(project.meta.mcu_family, "STM32F4");
        assert_eq!(project.meta.mcu_name, "STM32F446RETx");
        assert_eq!(project.pins.len(), 7);
        assert_eq!(project.peripherals.len(), 0);
        assert!(project.loop_body.is_some());
        assert!(project.pv_declarations.is_empty());
    }

    #[test]
    fn test_load_project_stm32_03_timers() {
        let (ioc_path, main_c_path) = timers_fixture_paths();
        let project = load_project(&ioc_path, &main_c_path).expect("failed to load timers project");

        assert_eq!(project.meta.name, "03_timers");
        assert_eq!(project.peripherals.len(), 5);
        assert!(
            project
                .call_graph_edges
                .iter()
                .any(|e| e.edge_type == EdgeType::IrqEntry),
            "Expected at least one IrqEntry edge in timers project"
        );
        assert_eq!(project.pv_declarations.len(), 8);
        assert!(
            project.pv_declarations.iter().any(|d| d.name == "isrCount"),
            "Expected isrCount in pv_declarations"
        );
    }

    #[test]
    fn test_project_json_roundtrip() {
        let (ioc_path, main_c_path) = timers_fixture_paths();
        let orig = load_project(&ioc_path, &main_c_path).expect("failed to load project");

        let json_str = project_to_json(&orig).expect("failed to serialize project to JSON");
        let roundtrip: Project =
            project_from_json(&json_str).expect("failed to deserialize project from JSON");

        assert_eq!(roundtrip.meta.name, orig.meta.name);
        assert_eq!(roundtrip.meta.mcu_family, orig.meta.mcu_family);
        assert_eq!(roundtrip.meta.mcu_name, orig.meta.mcu_name);
        assert_eq!(roundtrip.pins.len(), orig.pins.len());
        assert_eq!(roundtrip.peripherals.len(), orig.peripherals.len());
        assert_eq!(roundtrip.user_regions.len(), orig.user_regions.len());
        assert_eq!(roundtrip.call_graph_edges.len(), orig.call_graph_edges.len());
        assert_eq!(roundtrip.pv_declarations.len(), orig.pv_declarations.len());
    }

    #[test]
    fn test_load_project_docking_firmware_v2() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/docking_firmware_v2");
        let ioc = root.join("docking_firmware_v2.ioc");
        let main_c = root.join("Core/Src/main.c");
        let project = load_project(&ioc, &main_c).expect("failed to load docking_firmware_v2");

        assert_eq!(project.state_machines.len(), 1);
        assert_eq!(project.state_machines[0].enum_def.name, "SystemState");
        assert_eq!(project.state_machines[0].states.len(), 14);
    }

    #[test]
    fn test_load_project_aa_ns_stm_port() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/aa_ns_stm_port");
        let ioc = root.join("aa_ns_stm_port.ioc");
        let main_c = root.join("Core/Src/main.c");
        let project = load_project(&ioc, &main_c).expect("failed to load aa_ns_stm_port");

        assert_eq!(project.state_machines.len(), 2);
        let names: Vec<&str> = project.state_machines.iter().map(|sm| sm.enum_def.name.as_str()).collect();
        assert!(names.contains(&"AlignState"));
        assert!(names.contains(&"HatchState"));
    }

    #[test]
    fn test_pin_to_module_mapping_aa_ns_stm_port() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/aa_ns_stm_port");
        let ioc = root.join("aa_ns_stm_port.ioc");
        let main_c = root.join("Core/Src/main.c");
        let project = load_project(&ioc, &main_c).expect("failed to load aa_ns_stm_port");

        // Verify discovered module list
        assert_eq!(
            project.modules,
            vec!["alignment", "commands", "hatch", "relay", "system"]
        );

        // Helper to find owning modules for a pin by label, signal, or pin id
        let find_pin_modules = |name: &str| -> Vec<String> {
            project
                .pins
                .iter()
                .find(|p| p.label.as_deref() == Some(name) || p.signal == name || p.pin == name)
                .map(|p| p.modules.clone())
                .unwrap_or_default()
        };

        // Assert known mappings per Phase 2 requirements
        assert_eq!(find_pin_modules("Z1_LIMIT"), vec!["alignment"]);
        assert_eq!(find_pin_modules("GRIP_IN1"), vec!["hatch"]);
        assert_eq!(find_pin_modules("RELAY_FAN"), vec!["relay"]);
        assert_eq!(find_pin_modules("RELAY_LIGHT"), vec!["relay"]);
        assert_eq!(find_pin_modules("Z1_STEP"), vec!["alignment"]);
        assert_eq!(find_pin_modules("USART2_TX"), vec!["commands"]);
    }
}
