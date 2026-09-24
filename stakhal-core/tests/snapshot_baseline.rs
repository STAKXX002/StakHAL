use std::path::Path;
use stakhal_core::graph::compute_state_machine_layout;
use stakhal_core::ir::schema::load_project;

#[test]
fn generate_or_verify_snapshots() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let snapshot_dir = manifest_dir.join("tests/snapshots");
    let snapshot_file = snapshot_dir.join("baseline_state_machine_snapshot.json");

    // 1. docking_firmware
    let df_root = manifest_dir.join("tests/fixtures/docking_firmware");
    let df_ioc = df_root.join("docking_firmware.ioc");
    let df_main = df_root.join("Core/Src/main.c");
    let df_proj = load_project(&df_ioc, &df_main).expect("load docking_firmware");
    let df_layouts: Vec<_> = df_proj.state_machines.iter().map(compute_state_machine_layout).collect();

    // 2. docking_firmware_v2
    let df2_root = manifest_dir.join("tests/fixtures/docking_firmware_v2");
    let df2_ioc = df2_root.join("docking_firmware_v2.ioc");
    let df2_main = df2_root.join("Core/Src/main.c");
    let df2_proj = load_project(&df2_ioc, &df2_main).expect("load docking_firmware_v2");
    let df2_layouts: Vec<_> = df2_proj.state_machines.iter().map(compute_state_machine_layout).collect();

    // 3. aa_ns_stm_port
    let aa_root = manifest_dir.join("tests/fixtures/aa_ns_stm_port");
    let aa_ioc = aa_root.join("aa_ns_stm_port.ioc");
    let aa_main = aa_root.join("Core/Src/main.c");
    let aa_proj = load_project(&aa_ioc, &aa_main).expect("load aa_ns_stm_port");
    let aa_layouts: Vec<_> = aa_proj.state_machines.iter().map(compute_state_machine_layout).collect();

    fn normalize_sm(manifest_dir: &Path, sm: &stakhal_core::graph::AppStateMachine) -> stakhal_core::graph::AppStateMachine {
        let mut sm = sm.clone();
        let manifest_str = manifest_dir.to_string_lossy();
        if let Some(stripped) = sm.enum_def.file_path.strip_prefix(manifest_str.as_ref()) {
            sm.enum_def.file_path = stripped.trim_start_matches('/').to_string();
        }
        if let Some(stripped) = sm.var.file_path.strip_prefix(manifest_str.as_ref()) {
            sm.var.file_path = stripped.trim_start_matches('/').to_string();
        }
        sm
    }

    fn normalize_layout(l: &stakhal_core::graph::StateMachineLayout) -> DeterministicLayout {
        let mut nodes = std::collections::BTreeMap::new();
        for (k, v) in &l.nodes {
            let mut node = v.clone();
            node.x = (node.x * 1000.0).round() / 1000.0;
            node.y = (node.y * 1000.0).round() / 1000.0;
            node.width = (node.width * 1000.0).round() / 1000.0;
            node.height = (node.height * 1000.0).round() / 1000.0;
            node.collapsed_out_badges.sort();
            nodes.insert(k.clone(), node);
        }
        let round_pt = |(x, y): (f64, f64)| ((x * 1000.0).round() / 1000.0, (y * 1000.0).round() / 1000.0);
        let mut edges = Vec::new();
        for e in &l.edges {
            let mut edge = e.clone();
            edge.start = round_pt(edge.start);
            edge.end = round_pt(edge.end);
            edge.control1 = round_pt(edge.control1);
            edge.control2 = round_pt(edge.control2);
            edge.waypoints = edge.waypoints.iter().copied().map(round_pt).collect();
            edge.label_pos = round_pt(edge.label_pos);
            edges.push(edge);
        }
        DeterministicLayout {
            nodes,
            edges,
            lanes: l.lanes.clone(),
            width: (l.width * 1000.0).round() / 1000.0,
            height: (l.height * 1000.0).round() / 1000.0,
        }
    }

    #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
    struct DeterministicLayout {
        nodes: std::collections::BTreeMap<String, stakhal_core::graph::NodeLayout>,
        edges: Vec<stakhal_core::graph::EdgeLayout>,
        lanes: Vec<stakhal_core::graph::LaneLayout>,
        width: f64,
        height: f64,
    }

    #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
    struct SnapshotData {
        df_sms: Vec<stakhal_core::graph::AppStateMachine>,
        df_layouts: Vec<DeterministicLayout>,
        df2_sms: Vec<stakhal_core::graph::AppStateMachine>,
        df2_layouts: Vec<DeterministicLayout>,
        aa_sms: Vec<stakhal_core::graph::AppStateMachine>,
        aa_layouts: Vec<DeterministicLayout>,
    }

    let data = SnapshotData {
        df_sms: df_proj.state_machines.iter().map(|sm| normalize_sm(manifest_dir, sm)).collect(),
        df_layouts: df_layouts.iter().map(normalize_layout).collect(),
        df2_sms: df2_proj.state_machines.iter().map(|sm| normalize_sm(manifest_dir, sm)).collect(),
        df2_layouts: df2_layouts.iter().map(normalize_layout).collect(),
        aa_sms: aa_proj.state_machines.iter().map(|sm| normalize_sm(manifest_dir, sm)).collect(),
        aa_layouts: aa_layouts.iter().map(normalize_layout).collect(),
    };

    let mut serialized = serde_json::to_string_pretty(&data).unwrap();
    serialized.push('\n');

    let regenerate = std::env::var("STAKHAL_REGENERATE_SNAPSHOTS").as_deref() == Ok("1");
    if regenerate {
        std::fs::create_dir_all(&snapshot_dir).expect("failed to create snapshot dir");
        std::fs::write(&snapshot_file, &serialized).expect("failed to write snapshot file");
        println!("Regenerated baseline snapshot at {}", snapshot_file.display());
    } else {
        assert!(
            snapshot_file.exists(),
            "Snapshot file does not exist at {}. In CI, the baseline snapshot must already be committed. For local regeneration, run with STAKHAL_REGENERATE_SNAPSHOTS=1.",
            snapshot_file.display()
        );
        let expected_json = std::fs::read_to_string(&snapshot_file).expect("failed to read baseline snapshot");
        let expected_json = expected_json.replace("\r\n", "\n");
        assert_eq!(
            serialized,
            expected_json,
            "State machine graphs or layouts JSON differ from baseline snapshot! If this change was intentional, re-run with STAKHAL_REGENERATE_SNAPSHOTS=1 and commit the updated snapshot."
        );
    }
}
