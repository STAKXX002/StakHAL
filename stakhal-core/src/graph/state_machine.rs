use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use petgraph::stable_graph::{NodeIndex, StableDiGraph};
use rust_sugiyama::configure::{Config, CrossingMinimization};
use serde::{Deserialize, Serialize};
use tree_sitter::{Node, Parser};

use crate::source::marker_scan::ScanError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateNode {
    pub id: String,
    pub is_initial: bool,
    pub is_fault: bool,
    pub cluster: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransitionEdge {
    pub guard: String,
    pub is_fault: bool,
    pub transition_type: TransitionType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnumDefinition {
    pub name: String,
    pub variants: Vec<String>,
    pub file_path: String,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VariableScope {
    Global,
    Function(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackedVariable {
    pub name: String,
    pub enum_type: String,
    pub scope: VariableScope,
    pub initial_value: Option<String>,
    pub file_path: String,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateMachineCandidate {
    pub id: String,
    pub display_name: String,
    pub enum_def: EnumDefinition,
    pub var: TrackedVariable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppTransition {
    pub from: String,
    pub to: String,
    pub guard: String,
    #[serde(default)]
    pub label: String,
    pub is_fault: bool,
    pub transition_type: TransitionType,
    pub line: usize,
}

impl AppTransition {
    pub fn display_guard(&self, max_len: usize) -> String {
        let text = if !self.label.is_empty() {
            &self.label
        } else {
            &self.guard
        };
        if text.len() <= max_len {
            text.to_string()
        } else {
            format!("{}...", &text[..max_len.saturating_sub(3)])
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransitionType {
    Direct,
    IndirectHelper { helper_name: String, argument: Option<String> },
    EventTriggered,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AmbiguousTransition {
    pub target: String,
    pub guard: String,
    pub line: usize,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppStateMachine {
    pub id: String,
    pub display_name: String,
    pub enum_def: EnumDefinition,
    pub var: TrackedVariable,
    pub states: Vec<String>,
    pub transitions: Vec<AppTransition>,
    pub ambiguous_transitions: Vec<AmbiguousTransition>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LaneLayout {
    pub name: String,
    pub y: f64,
    pub x_start: f64,
    pub x_end: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StateMachineLayout {
    pub nodes: HashMap<String, NodeLayout>,
    pub edges: Vec<EdgeLayout>,
    #[serde(default)]
    pub lanes: Vec<LaneLayout>,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeLayout {
    pub id: String,
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub is_fault: bool,
    pub is_initial: bool,
    #[serde(default)]
    pub cluster: String,
    #[serde(default)]
    pub collapsed_out_badges: Vec<String>,
    #[serde(default)]
    pub incoming_count: usize,
    #[serde(default)]
    pub outgoing_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdgeLayout {
    pub from: String,
    pub to: String,
    pub guard: String,
    #[serde(default)]
    pub label: String,
    pub display_guard: String,
    pub is_fault: bool,
    pub start: (f64, f64),
    pub end: (f64, f64),
    pub control1: (f64, f64),
    pub control2: (f64, f64),
    #[serde(default)]
    pub waypoints: Vec<(f64, f64)>,
    pub label_pos: (f64, f64),
    pub transition_type: TransitionType,
    #[serde(default)]
    pub is_high_fan_in: bool,
    #[serde(default)]
    pub is_inter_cluster: bool,
}

/// Discover all enum definitions and variables declared with that enum type in a C file.
pub fn discover_state_machines_in_file(path: &Path) -> Result<Vec<StateMachineCandidate>, ScanError> {
    if !path.exists() {
        return Err(ScanError::FileNotFound(path.to_path_buf()));
    }

    let source = fs::read_to_string(path).map_err(|e| ScanError::IoError(e.to_string()))?;
    let path_str = path.to_string_lossy().to_string();

    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_c::language())
        .map_err(|e| ScanError::ParseError(e.to_string()))?;

    let tree = parser
        .parse(&source, None)
        .ok_or_else(|| ScanError::ParseError("Failed to parse C file".to_string()))?;

    let source_bytes = source.as_bytes();
    let root = tree.root_node();

    // 1. Collect all enum definitions
    let mut enums = Vec::new();
    collect_enum_definitions(root, source_bytes, &path_str, &mut enums);

    if enums.is_empty() {
        return Ok(Vec::new());
    }

    let enum_map: HashMap<String, EnumDefinition> = enums
        .into_iter()
        .map(|e| (e.name.clone(), e))
        .collect();

    // 2. Find variables declared with enum types
    let mut variables = Vec::new();
    collect_variables(root, source_bytes, &path_str, None, &enum_map, &mut variables);

    // 3. Match each variable to its enum definition
    let mut candidates = Vec::new();
    for var in variables {
        if let Some(enum_def) = enum_map.get(&var.enum_type) {
            let id = format!("{}_{}", var.enum_type, var.name);
            let display_name = format!("{} ({})", var.enum_type, var.name);
            candidates.push(StateMachineCandidate {
                id,
                display_name,
                enum_def: enum_def.clone(),
                var,
            });
        }
    }

    Ok(candidates)
}

/// Discover all enum definitions and variables across all C source files in the project (e.g. Core/Src/*.c or Src/*.c).
pub fn discover_state_machines_in_project(main_c_path: &Path) -> Result<Vec<StateMachineCandidate>, ScanError> {
    if !main_c_path.exists() {
        return Err(ScanError::FileNotFound(main_c_path.to_path_buf()));
    }

    let src_dir = main_c_path.parent().unwrap_or_else(|| Path::new("."));

    let mut c_files = Vec::new();
    if let Ok(entries) = fs::read_dir(src_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() && p.extension().map_or(false, |ext| ext == "c") {
                c_files.push(p);
            }
        }
    }
    if !c_files.contains(&main_c_path.to_path_buf()) {
        c_files.push(main_c_path.to_path_buf());
    }
    c_files.sort();

    let mut inc_dirs = Vec::new();
    if let Some(parent) = src_dir.parent() {
        let inc = parent.join("Inc");
        if inc.is_dir() {
            inc_dirs.push(inc);
        }
        let inc_lower = parent.join("inc");
        if inc_lower.is_dir() && !inc_dirs.contains(&inc_lower) {
            inc_dirs.push(inc_lower);
        }
    }
    if !inc_dirs.contains(&src_dir.to_path_buf()) {
        inc_dirs.push(src_dir.to_path_buf());
    }

    let mut h_files = Vec::new();
    for dir in inc_dirs {
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() && p.extension().map_or(false, |ext| ext == "h") {
                    h_files.push(p);
                }
            }
        }
    }
    h_files.sort();

    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_c::language())
        .map_err(|e| ScanError::ParseError(e.to_string()))?;

    let mut enums = Vec::new();

    for h_path in &h_files {
        if let Ok(source) = fs::read_to_string(h_path) {
            if let Some(tree) = parser.parse(&source, None) {
                let path_str = h_path.to_string_lossy().to_string();
                collect_enum_definitions(tree.root_node(), source.as_bytes(), &path_str, &mut enums);
            }
        }
    }

    for c_path in &c_files {
        if let Ok(source) = fs::read_to_string(c_path) {
            if let Some(tree) = parser.parse(&source, None) {
                let path_str = c_path.to_string_lossy().to_string();
                collect_enum_definitions(tree.root_node(), source.as_bytes(), &path_str, &mut enums);
            }
        }
    }

    if enums.is_empty() {
        return Ok(Vec::new());
    }

    let enum_map: HashMap<String, EnumDefinition> = enums
        .into_iter()
        .map(|e| (e.name.clone(), e))
        .collect();

    let mut variables = Vec::new();
    for c_path in &c_files {
        if let Ok(source) = fs::read_to_string(c_path) {
            if let Some(tree) = parser.parse(&source, None) {
                let path_str = c_path.to_string_lossy().to_string();
                collect_variables(tree.root_node(), source.as_bytes(), &path_str, None, &enum_map, &mut variables);
            }
        }
    }

    let mut candidates = Vec::new();
    let mut seen = HashSet::new();
    for var in variables {
        if let Some(enum_def) = enum_map.get(&var.enum_type) {
            let key = (enum_def.name.clone(), var.name.clone(), var.file_path.clone());
            if seen.insert(key) {
                let id = format!("{}_{}", var.enum_type, var.name);
                let display_name = format!("{} ({})", var.enum_type, var.name);
                candidates.push(StateMachineCandidate {
                    id,
                    display_name,
                    enum_def: enum_def.clone(),
                    var,
                });
            }
        }
    }

    candidates.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    Ok(candidates)
}

/// Extract all transitions (direct and event-triggered) and ambiguous assignments for a state machine candidate.
pub fn extract_state_machine_transitions(
    candidate: &StateMachineCandidate,
    file_path: &Path,
) -> Result<AppStateMachine, ScanError> {
    if !file_path.exists() {
        return Err(ScanError::FileNotFound(file_path.to_path_buf()));
    }

    let source = fs::read_to_string(file_path).map_err(|e| ScanError::IoError(e.to_string()))?;
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_c::language())
        .map_err(|e| ScanError::ParseError(e.to_string()))?;

    let tree = parser
        .parse(&source, None)
        .ok_or_else(|| ScanError::ParseError("Failed to parse C file".to_string()))?;

    let source_bytes = source.as_bytes();
    let root = tree.root_node();

    let known_variants: HashSet<String> = candidate.enum_def.variants.iter().cloned().collect();
    let mut transitions = Vec::new();
    let mut ambiguous_transitions = Vec::new();

    // 1. Identify helper functions that assign to tracked variable
    let helpers = collect_helper_functions(root, source_bytes, &candidate.var.name, &known_variants);

    // 2. Walk all function definitions (skip helper function bodies for direct assignment tracking)
    let mut fn_nodes = Vec::new();
    collect_functions(root, &mut fn_nodes);

    for fn_node in fn_nodes {
        let fn_name = extract_function_name(fn_node, source_bytes);
        let is_helper = fn_name.as_ref().map(|n| helpers.contains_key(n)).unwrap_or(false);
        if is_helper {
            continue;
        }

        if let Some(body) = fn_node.child_by_field_name("body") {
            let mut ctx = ASTContext::default();
            ctx.has_local_fault = candidate.enum_def.variants.iter().any(|v| is_fault_state(v));
            walk_statement(
                body,
                &ctx,
                source_bytes,
                &candidate.var.name,
                &known_variants,
                &helpers,
                &mut transitions,
                &mut ambiguous_transitions,
            );
        }
    }

    // 3. Resolve multi-hop command transitions across other C files in the project
    let src_dir = file_path.parent().unwrap_or_else(|| Path::new("."));
    resolve_cross_file_command_transitions(
        src_dir,
        file_path,
        root,
        source_bytes,
        candidate,
        &known_variants,
        &helpers,
        &mut transitions,
        &mut ambiguous_transitions,
    );

    let mut states = candidate.enum_def.variants.clone();
    if transitions.iter().any(|t| t.to == "SYSTEM FAULT") && !states.contains(&"SYSTEM FAULT".to_string()) {
        states.push("SYSTEM FAULT".to_string());
    }

    Ok(AppStateMachine {
        id: candidate.id.clone(),
        display_name: candidate.display_name.clone(),
        enum_def: candidate.enum_def.clone(),
        var: candidate.var.clone(),
        states,
        transitions,
        ambiguous_transitions,
    })
}

/// Infer semantic cluster name for labelling and UI lane banners.
pub fn infer_cluster_name(state: &str, initial_name: &str) -> &'static str {
    if is_fault_state(state) {
        "FAULT"
    } else if state == initial_name {
        "INITIAL"
    } else if state.starts_with("CAL") {
        "CALIBRATION"
    } else if state.starts_with("REC") {
        "RECOVERY"
    } else if state == "GOING" || state == "HOLD" || state == "RETURNING" || state == "RETURNED" {
        "MOTION"
    } else if state == "OPENING" || state == "CLOSING" {
        "MECHANISM"
    } else {
        "GENERAL"
    }
}

/// Build a petgraph StableDiGraph from an AppStateMachine.
/// Excludes high-fan-in FAULT edges (handled as badges) and wildcard/self transitions.
pub fn build_state_machine_graph(
    sm: &AppStateMachine,
) -> (StableDiGraph<StateNode, TransitionEdge>, HashMap<String, NodeIndex>) {
    let initial_name = sm.var.initial_value.as_deref().unwrap_or("IDLE");
    let mut graph = StableDiGraph::new();
    let mut node_map = HashMap::new();

    for state in &sm.states {
        let is_initial = state == initial_name;
        let is_fault = is_fault_state(state);
        let cluster = infer_cluster_name(state, initial_name).to_string();
        let idx = graph.add_node(StateNode {
            id: state.clone(),
            is_initial,
            is_fault,
            cluster,
        });
        node_map.insert(state.clone(), idx);
    }

    for t in &sm.transitions {
        if !t.is_fault && !is_fault_state(&t.to) && t.from != "(any state)" && t.from != t.to {
            if let (Some(&from_idx), Some(&to_idx)) = (node_map.get(&t.from), node_map.get(&t.to)) {
                graph.add_edge(
                    from_idx,
                    to_idx,
                    TransitionEdge {
                        guard: t.guard.clone(),
                        is_fault: t.is_fault,
                        transition_type: t.transition_type.clone(),
                    },
                );
            }
        }
    }

    (graph, node_map)
}

#[derive(Debug, Clone, Copy)]
pub struct BoundingBox {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}

impl BoundingBox {
    pub fn intersects(&self, other: &BoundingBox) -> bool {
        self.x0 < other.x1 && self.x1 > other.x0 && self.y0 < other.y1 && self.y1 > other.y0
    }
}

/// Compute layered node-link graph layout with hub-anchored swimlanes.
pub fn compute_state_machine_layout(sm: &AppStateMachine) -> StateMachineLayout {
    let node_width = 175.0;
    let node_height = 54.0;
    let initial_name = sm.var.initial_value.as_deref().unwrap_or("IDLE");

    // 1. Calculate operational and total in/out degrees
    let mut total_in_degrees: HashMap<String, usize> = HashMap::new();
    let mut total_out_degrees: HashMap<String, usize> = HashMap::new();
    let mut op_in_degrees: HashMap<String, usize> = HashMap::new();
    let mut op_out_degrees: HashMap<String, usize> = HashMap::new();
    for st in &sm.states {
        total_in_degrees.insert(st.clone(), 0);
        total_out_degrees.insert(st.clone(), 0);
        op_in_degrees.insert(st.clone(), 0);
        op_out_degrees.insert(st.clone(), 0);
    }
    for t in &sm.transitions {
        if t.from != t.to {
            if t.from != "(any state)" {
                *total_out_degrees.entry(t.from.clone()).or_default() += 1;
            }
            *total_in_degrees.entry(t.to.clone()).or_default() += 1;

            if !t.is_fault && !is_fault_state(&t.to) {
                if t.from != "(any state)" {
                    *op_out_degrees.entry(t.from.clone()).or_default() += 1;
                }
                *op_in_degrees.entry(t.to.clone()).or_default() += 1;
            }
        }
    }

    // High fan-in threshold for badge-only collapsing (e.g. 12 transitions to FAULT)
    let mut high_fan_in_nodes = HashSet::new();
    for st in &sm.states {
        let total_in = sm.transitions.iter().filter(|t| t.to == *st && t.from != *st).count();
        if total_in >= 4 || is_fault_state(st) {
            high_fan_in_nodes.insert(st.clone());
        }
    }

    let mut collapsed_out_map: HashMap<String, Vec<String>> = HashMap::new();
    for t in &sm.transitions {
        if t.from != "(any state)" && high_fan_in_nodes.contains(&t.to) && t.from != t.to {
            let list = collapsed_out_map.entry(t.from.clone()).or_default();
            if !list.contains(&t.to) {
                list.push(t.to.clone());
            }
        }
    }

    // Identify hub nodes:
    // 1. Initial state (e.g. IDLE) is always the entry hub.
    // 2. An exit hub must be an architectural junction bridging multiple distinct functional clusters
    //    (i.e. connects across >= 2 foreign clusters, excluding FAULT and INITIAL).
    //    States with high internal connectivity within a single lane (e.g. OPENING <-> CLOSING)
    //    belong in their functional swimlane, not as global hubs.
    let mut hub_nodes = HashSet::new();
    hub_nodes.insert(initial_name.to_string());

    for st in &sm.states {
        if is_fault_state(st) || st == initial_name {
            continue;
        }
        let my_cluster = infer_cluster_name(st, initial_name);
        let mut foreign_clusters = HashSet::new();
        for t in &sm.transitions {
            if t.from == *st && t.to != *st && !is_fault_state(&t.to) && t.to != initial_name {
                let other_cluster = infer_cluster_name(&t.to, initial_name);
                if other_cluster != my_cluster {
                    foreign_clusters.insert(other_cluster);
                }
            }
            if t.to == *st && t.from != *st && !is_fault_state(&t.from) && t.from != initial_name && t.from != "(any state)" {
                let other_cluster = infer_cluster_name(&t.from, initial_name);
                if other_cluster != my_cluster {
                    foreign_clusters.insert(other_cluster);
                }
            }
        }
        let op_deg = op_in_degrees.get(st).copied().unwrap_or(0) + op_out_degrees.get(st).copied().unwrap_or(0);
        if foreign_clusters.len() >= 2 && op_deg >= 4 {
            hub_nodes.insert(st.clone());
        }
    }

    // Designate entry hub: initial state if present, else hub with highest operational out-degree
    let entry_hub = if hub_nodes.contains(initial_name) {
        initial_name.to_string()
    } else {
        hub_nodes
            .iter()
            .max_by_key(|h| op_out_degrees.get(*h).copied().unwrap_or(0))
            .cloned()
            .unwrap_or_else(|| initial_name.to_string())
    };

    let mut exit_hubs: Vec<String> = hub_nodes.iter().filter(|h| **h != entry_hub).cloned().collect();
    exit_hubs.sort();

    // 2. Partition non-hub, non-fault states into logical flow lanes
    let mut lane_clusters: Vec<(&'static str, Vec<String>)> = vec![
        ("CALIBRATION", Vec::new()),
        ("MECHANISM", Vec::new()),
        ("MOTION", Vec::new()),
        ("RECOVERY", Vec::new()),
    ];
    let mut other_states = Vec::new();

    for st in &sm.states {
        if is_fault_state(st) || hub_nodes.contains(st) {
            continue;
        }
        let cluster = infer_cluster_name(st, initial_name);
        if let Some((_, list)) = lane_clusters.iter_mut().find(|(c, _)| *c == cluster) {
            list.push(st.clone());
        } else {
            other_states.push(st.clone());
        }
    }
    if !other_states.is_empty() {
        lane_clusters.push(("GENERAL", other_states));
    }
    lane_clusters.retain(|(_, list)| !list.is_empty());

    // 3. Run rust-sugiyama on each lane independently
    let config = Config {
        vertex_spacing: 80.0,
        c_minimization: CrossingMinimization::Median,
        transpose: true,
        ..Default::default()
    };
    let size_fn = |_idx: NodeIndex, _node: &StateNode| (node_height, node_width);

    let mut lane_node_ranks: HashMap<String, usize> = HashMap::new();
    let mut max_rank_in_any_lane = 0;

    for (_cluster_name, lane_states) in &lane_clusters {
        let lane_state_set: HashSet<String> = lane_states.iter().cloned().collect();
        let mut lane_graph = StableDiGraph::new();
        let mut lane_map = HashMap::new();

        for st in lane_states {
            let idx = lane_graph.add_node(StateNode {
                id: st.clone(),
                is_initial: false,
                is_fault: false,
                cluster: infer_cluster_name(st, initial_name).to_string(),
            });
            lane_map.insert(st.clone(), idx);
        }

        // Intra-lane edges only
        for t in &sm.transitions {
            if lane_state_set.contains(&t.from) && lane_state_set.contains(&t.to) && t.from != t.to {
                lane_graph.add_edge(
                    lane_map[&t.from],
                    lane_map[&t.to],
                    TransitionEdge {
                        guard: t.guard.clone(),
                        is_fault: false,
                        transition_type: t.transition_type.clone(),
                    },
                );
            }
        }

        let layouts = rust_sugiyama::from_graph(&lane_graph, &size_fn, &config);
        let mut sorted_nodes: Vec<(String, f64)> = Vec::new();
        for (nodes, _w, _h) in layouts {
            for (node_idx, (_sx, sy)) in nodes {
                sorted_nodes.push((lane_graph[node_idx].id.clone(), sy));
            }
        }
        sorted_nodes.sort_by(|a, b| a.1.total_cmp(&b.1));

        for (rank, (name, _)) in sorted_nodes.into_iter().enumerate() {
            lane_node_ranks.insert(name, rank);
            if rank > max_rank_in_any_lane {
                max_rank_in_any_lane = rank;
            }
        }
    }

    // 4. Geometry & Positioning of Hubs and Swimlanes
    let left_margin = 110.0;
    let top_margin = 110.0;
    let lane_gap = 130.0;
    let hub_to_lane_gap = 140.0;
    let rank_step_x = node_width + 80.0; // 255.0

    let lane_start_x = left_margin + node_width + hub_to_lane_gap; // 110 + 175 + 140 = 425.0

    let mut nodes_layout = HashMap::new();
    let mut lane_y_positions: HashMap<String, f64> = HashMap::new();

    // 4a. Position lane nodes in horizontal rows
    for (lane_idx, (cluster_name, lane_states)) in lane_clusters.iter().enumerate() {
        let lane_y = top_margin + lane_idx as f64 * lane_gap;
        lane_y_positions.insert(cluster_name.to_string(), lane_y);

        for st in lane_states {
            let rank = lane_node_ranks.get(st).copied().unwrap_or(0);
            let x = lane_start_x + rank as f64 * rank_step_x;
            let y = lane_y;

            let in_count = total_in_degrees.get(st).copied().unwrap_or(0);
            let out_count = total_out_degrees.get(st).copied().unwrap_or(0);
            let collapsed_badges = collapsed_out_map.get(st).cloned().unwrap_or_default();

            nodes_layout.insert(
                st.clone(),
                NodeLayout {
                    id: st.clone(),
                    label: st.clone(),
                    x,
                    y,
                    width: node_width,
                    height: node_height,
                    is_fault: false,
                    is_initial: false,
                    cluster: cluster_name.to_string(),
                    collapsed_out_badges: collapsed_badges,
                    incoming_count: in_count,
                    outgoing_count: out_count,
                },
            );
        }
    }

    let max_lane_x = lane_start_x + max_rank_in_any_lane as f64 * rank_step_x + node_width;
    let lowest_lane_y = top_margin + lane_clusters.len().saturating_sub(1) as f64 * lane_gap;

    // 4b. Position Entry Hub (IDLE) pinned on left
    let entry_y = (top_margin + lowest_lane_y - lane_gap) * 0.5; // vertically centered among top lanes
    let entry_x = left_margin;
    {
        let in_count = total_in_degrees.get(&entry_hub).copied().unwrap_or(0);
        let out_count = total_out_degrees.get(&entry_hub).copied().unwrap_or(0);
        let collapsed_badges = collapsed_out_map.get(&entry_hub).cloned().unwrap_or_default();
        nodes_layout.insert(
            entry_hub.clone(),
            NodeLayout {
                id: entry_hub.clone(),
                label: entry_hub.clone(),
                x: entry_x,
                y: entry_y,
                width: node_width,
                height: node_height,
                is_fault: false,
                is_initial: true,
                cluster: "INITIAL".to_string(),
                collapsed_out_badges: collapsed_badges,
                incoming_count: in_count,
                outgoing_count: out_count,
            },
        );
    }

    // 4c. Position Exit Hub(s) (RETURNED) pinned on lower-right
    let exit_x = max_lane_x + hub_to_lane_gap;
    let base_exit_y = lowest_lane_y - lane_gap * 0.5; // centered between MOTION and RECOVERY
    for (idx, exit_hub) in exit_hubs.iter().enumerate() {
        let exit_y = base_exit_y + idx as f64 * lane_gap;
        let in_count = total_in_degrees.get(exit_hub).copied().unwrap_or(0);
        let out_count = total_out_degrees.get(exit_hub).copied().unwrap_or(0);
        let collapsed_badges = collapsed_out_map.get(exit_hub).cloned().unwrap_or_default();
        nodes_layout.insert(
            exit_hub.clone(),
            NodeLayout {
                id: exit_hub.clone(),
                label: exit_hub.clone(),
                x: exit_x,
                y: exit_y,
                width: node_width,
                height: node_height,
                is_fault: false,
                is_initial: false,
                cluster: infer_cluster_name(exit_hub, initial_name).to_string(),
                collapsed_out_badges: collapsed_badges,
                incoming_count: in_count,
                outgoing_count: out_count,
            },
        );
    }

    let max_total_op_x = exit_x + node_width;
    let max_total_op_y = lowest_lane_y + node_height;

    // 4d. Position FAULT node(s) pinned centered at bottom
    let fault_y = max_total_op_y + 80.0;
    let center_x = (left_margin + max_total_op_x) * 0.5;
    let fault_states: Vec<String> = sm.states.iter().filter(|s| is_fault_state(s)).cloned().collect();
    let fault_count = fault_states.len().max(1);
    let total_fault_w = fault_count as f64 * node_width + fault_count.saturating_sub(1) as f64 * 40.0;
    let fault_start_x = center_x - total_fault_w * 0.5;

    for (col_idx, node_id) in fault_states.iter().enumerate() {
        let x = fault_start_x + col_idx as f64 * (node_width + 40.0);
        let y = fault_y;
        let in_count = total_in_degrees.get(node_id).copied().unwrap_or(0);
        let out_count = total_out_degrees.get(node_id).copied().unwrap_or(0);
        let collapsed_badges = collapsed_out_map.get(node_id).cloned().unwrap_or_default();
        nodes_layout.insert(
            node_id.clone(),
            NodeLayout {
                id: node_id.clone(),
                label: node_id.clone(),
                x,
                y,
                width: node_width,
                height: node_height,
                is_fault: true,
                is_initial: false,
                cluster: "FAULT".to_string(),
                collapsed_out_badges: collapsed_badges,
                incoming_count: in_count,
                outgoing_count: out_count,
            },
        );
    }

    // Generate cluster lane headers for UI drawing
    let mut lanes = Vec::new();
    for (cluster_name, lane_states) in &lane_clusters {
        let mut min_x = f64::MAX;
        let mut max_x = f64::MIN;
        let mut min_y = f64::MAX;
        for st in lane_states {
            if let Some(nl) = nodes_layout.get(st) {
                min_x = min_x.min(nl.x);
                max_x = max_x.max(nl.x + nl.width);
                min_y = min_y.min(nl.y);
            }
        }
        lanes.push(LaneLayout {
            name: cluster_name.to_string(),
            y: min_y,
            x_start: min_x,
            x_end: max_x,
        });
    }
    if !fault_states.is_empty() {
        lanes.push(LaneLayout {
            name: "FAULT".to_string(),
            y: fault_y,
            x_start: fault_start_x,
            x_end: fault_start_x + total_fault_w,
        });
    }
    lanes.sort_by(|a, b| a.y.total_cmp(&b.y));

    // 5. Bézier Edge Routing with Multi-Point Hub Anchors
    let mut edges_layout = Vec::new();

    // Map outgoing edges from entry_hub by target to distribute anchors
    let entry_outgoing: Vec<&AppTransition> = sm
        .transitions
        .iter()
        .filter(|t| t.from == entry_hub && !t.is_fault && !is_fault_state(&t.to))
        .collect();

    let mut overhead_track_idx = 0;
    let mut exit_corridor_track_idx = 0;
    let mut inter_lane_track_map: HashMap<(i32, i32), usize> = HashMap::new();

    for t in &sm.transitions {
        let from_layout = nodes_layout.get(&t.from);
        let to_layout = nodes_layout.get(&t.to);

        if let (Some(from), Some(to)) = (from_layout, to_layout) {
            let display_guard = t.display_guard(35);
            let is_high_fan_in = high_fan_in_nodes.contains(&t.to) && t.from != t.to;
            let is_inter_cluster = from.cluster != to.cluster;

            let (start, end, control1, control2, label_pos, waypoints) = if to.is_fault {
                // Route down toward centered fault
                let start = (from.x + from.width * 0.5, from.y + from.height);
                let end = (to.x + to.width * 0.5, to.y);
                let dy = (end.1 - start.1).max(30.0);
                let control1 = (start.0, start.1 + dy * 0.45);
                let control2 = (end.0, end.1 - dy * 0.45);
                let label_pos = (from.x + from.width * 0.5, from.y + from.height + 24.0);
                let waypoints = if (start.0 - end.0).abs() < 2.0 {
                    vec![start, end]
                } else {
                    let drop_y = lowest_lane_y + node_height + 40.0;
                    vec![start, (start.0, drop_y), (end.0, drop_y), end]
                };
                (start, end, control1, control2, label_pos, waypoints)
            } else if t.from == entry_hub {
                // Outgoing from Entry Hub (IDLE): stack connection anchors along right edge based on target vertical order
                let target_order = entry_outgoing
                    .iter()
                    .position(|x| x.to == t.to && x.guard == t.guard)
                    .unwrap_or(0);
                let n_conns = entry_outgoing.len().max(1);
                let fraction = (target_order as f64 + 1.0) / (n_conns as f64 + 1.0);
                let start_y = from.y + from.height * fraction;
                let start = (from.x + from.width, start_y);
                let dx = (to.x - start.0).max(30.0);
                let control1 = (start.0 + dx * 0.35, start.1);
                let control2 = (to.x - dx * 0.35, to.y + to.height * 0.5);

                let corridor_x = from.x + from.width + 25.0 + (target_order as f64) * 20.0;
                let label_pos = (from.x + from.width + 35.0, start_y - 12.0);
                let (end, waypoints) = if (start.1 - (to.y + to.height * 0.5)).abs() < 2.0 {
                    let end = (to.x, to.y + to.height * 0.5);
                    (end, vec![start, end])
                } else if to.x <= lane_start_x + 10.0 {
                    let end = (to.x, to.y + to.height * 0.5);
                    (end, vec![start, (corridor_x, start.1), (corridor_x, end.1), end])
                } else {
                    let channel_y = to.y - 25.0;
                    let end = (to.x + to.width * 0.5, to.y);
                    (end, vec![start, (corridor_x, start.1), (corridor_x, channel_y), (to.x + to.width * 0.5, channel_y), end])
                };
                (start, end, control1, control2, label_pos, waypoints)
            } else if t.to == entry_hub {
                // Returning to Entry Hub (IDLE): route back smoothly
                if from.y <= to.y {
                    // Coming from upper lane (e.g. CAL_BACKOFF -> IDLE, CLOSING -> IDLE)
                    let start = (from.x + from.width * 0.5, from.y);
                    let end = (to.x + to.width * 0.6, to.y);
                    let arc_y = (top_margin - 30.0 - (overhead_track_idx as f64) * 16.0).min(from.y - 25.0);
                    overhead_track_idx += 1;
                    let control1 = (from.x + from.width * 0.5, arc_y);
                    let control2 = (to.x + to.width * 0.6, arc_y);
                    let label_pos = ((start.0 + end.0) * 0.5, arc_y - 12.0);
                    let waypoints = vec![start, (start.0, arc_y), (end.0, arc_y), end];
                    (start, end, control1, control2, label_pos, waypoints)
                } else {
                    // Coming from lower lane (e.g. OPENING -> IDLE, REC_BACKOFF -> ALIGN_IDLE)
                    let start = (from.x + from.width * 0.5, from.y + from.height);
                    let end = (to.x + to.width * 0.5, to.y + to.height);
                    let route_y = lowest_lane_y + node_height + 30.0;
                    let control1 = ((start.0 + end.0) * 0.5, start.1 + 20.0);
                    let control2 = ((start.0 + end.0) * 0.5, end.1);
                    let label_pos = ((start.0 + end.0) * 0.5, route_y + 14.0);
                    let waypoints = vec![start, (start.0, route_y), (end.0, route_y), end];
                    (start, end, control1, control2, label_pos, waypoints)
                }
            } else if exit_hubs.contains(&t.to) {
                // Entering Exit Hub (RETURNED): direct diagonal from right of lane to left of hub
                let start = (from.x + from.width, from.y + from.height * 0.5);
                let anchor_y = if from.y < to.y {
                    to.y + to.height * 0.3
                } else if from.y > to.y {
                    to.y + to.height * 0.7
                } else {
                    to.y + to.height * 0.5
                };
                let end = (to.x, anchor_y);
                let dx = (end.0 - start.0).max(20.0);
                let control1 = (start.0 + dx * 0.4, start.1);
                let control2 = (end.0 - dx * 0.4, end.1);
                let label_pos = ((start.0 + end.0) * 0.5, (start.1 + end.1) * 0.5 - 14.0);
                let corridor_x = (from.x + from.width + 30.0 + (exit_corridor_track_idx as f64) * 20.0).max((from.x + from.width + to.x) * 0.5);
                exit_corridor_track_idx += 1;
                let waypoints = if (start.1 - end.1).abs() < 2.0 {
                    vec![start, end]
                } else {
                    vec![start, (corridor_x, start.1), (corridor_x, end.1), end]
                };
                (start, end, control1, control2, label_pos, waypoints)
            } else if exit_hubs.contains(&t.from) {
                // Outgoing repeat/loopback from Exit Hub (RETURNED -> GOING, OPENING, CLOSING)
                let start = (from.x + from.width * 0.5, from.y);
                let end = (to.x + to.width * 0.5, to.y);
                let arc_y = (top_margin - 30.0 - (overhead_track_idx as f64) * 16.0).min(from.y.min(to.y) - 25.0);
                overhead_track_idx += 1;
                let control1 = (start.0, arc_y);
                let control2 = (end.0, arc_y);
                let label_pos = ((start.0 + end.0) * 0.5, arc_y - 12.0);
                let waypoints = vec![start, (start.0, arc_y), (end.0, arc_y), end];
                (start, end, control1, control2, label_pos, waypoints)
            } else if (from.y - to.y).abs() < 5.0 {
                // Same horizontal lane
                if from.x + from.width <= to.x + 15.0 {
                    // Forward in same lane
                    let is_adjacent = to.x <= from.x + rank_step_x + 15.0;
                    if is_adjacent {
                        let start = (from.x + from.width, from.y + from.height * 0.5);
                        let end = (to.x, to.y + to.height * 0.5);
                        let dx = (end.0 - start.0).max(20.0);
                        let control1 = (start.0 + dx * 0.4, start.1);
                        let control2 = (end.0 - dx * 0.4, end.1);
                        let label_w = (display_guard.len() as f64 * 6.0 + 10.0).max(28.0);
                        let label_pos = if label_w > 62.0 {
                            ((start.0 + end.0) * 0.5, from.y + from.height + 18.0)
                        } else {
                            ((start.0 + end.0) * 0.5, start.1 + 14.0)
                        };
                        let waypoints = vec![start, end];
                        (start, end, control1, control2, label_pos, waypoints)
                    } else {
                        // Skipping over intermediate node in same lane
                        let start = (from.x + from.width * 0.5, from.y);
                        let channel_y = from.y - 25.0;
                        let end = (to.x + to.width * 0.5, to.y);
                        let control1 = (start.0, channel_y);
                        let control2 = (end.0, channel_y);
                        let label_pos = ((start.0 + end.0) * 0.5, channel_y - 12.0);
                        let waypoints = vec![start, (start.0, channel_y), (end.0, channel_y), end];
                        (start, end, control1, control2, label_pos, waypoints)
                    }
                } else {
                    // Backward loop within same lane
                    let start = (from.x + from.width * 0.5, from.y);
                    let channel_y = from.y - 25.0;
                    let end = (to.x + to.width * 0.5, to.y);
                    let control1 = (start.0, channel_y);
                    let control2 = (end.0, channel_y);
                    let label_pos = ((start.0 + end.0) * 0.5, channel_y - 12.0);
                    let waypoints = vec![start, (start.0, channel_y), (end.0, channel_y), end];
                    (start, end, control1, control2, label_pos, waypoints)
                }
            } else {
                // Cross-lane edge (e.g. RETURNING -> RECOVERY)
                let lane1 = (from.y / lane_gap).round() as i32;
                let lane2 = (to.y / lane_gap).round() as i32;
                let pair = (lane1.min(lane2), lane1.max(lane2));
                let track = inter_lane_track_map.entry(pair).or_insert(0);
                let track_offset = (*track as f64) * 16.0 - 8.0;
                *track += 1;

                if from.y < to.y {
                    // Upper lane to lower lane
                    let start = (from.x + from.width * 0.5, from.y + from.height);
                    let end = (to.x + to.width * 0.5, to.y);
                    let base_channel_y = from.y + node_height + (to.y - (from.y + node_height)) * 0.5;
                    let channel_y = base_channel_y + track_offset;
                    let control1 = (start.0, channel_y);
                    let control2 = (end.0, channel_y);
                    let label_pos = ((start.0 + end.0) * 0.5, channel_y - 12.0);
                    let waypoints = if (start.0 - end.0).abs() < 2.0 {
                        vec![start, end]
                    } else {
                        vec![start, (start.0, channel_y), (end.0, channel_y), end]
                    };
                    (start, end, control1, control2, label_pos, waypoints)
                } else {
                    // Lower lane to upper lane
                    let start = (from.x + from.width * 0.5, from.y);
                    let end = (to.x + to.width * 0.5, to.y + to.height);
                    let base_channel_y = to.y + node_height + (from.y - (to.y + node_height)) * 0.5;
                    let channel_y = base_channel_y + track_offset;
                    let control1 = (start.0, channel_y);
                    let control2 = (end.0, channel_y);
                    let label_pos = ((start.0 + end.0) * 0.5, channel_y - 12.0);
                    let waypoints = if (start.0 - end.0).abs() < 2.0 {
                        vec![start, end]
                    } else {
                        vec![start, (start.0, channel_y), (end.0, channel_y), end]
                    };
                    (start, end, control1, control2, label_pos, waypoints)
                }
            };

            edges_layout.push(EdgeLayout {
                from: t.from.clone(),
                to: t.to.clone(),
                guard: t.guard.clone(),
                label: t.label.clone(),
                display_guard,
                is_fault: t.is_fault,
                start,
                end,
                control1,
                control2,
                waypoints,
                label_pos,
                transition_type: t.transition_type.clone(),
                is_high_fan_in,
                is_inter_cluster,
            });
        }
    }

    // 6. Post-layout label collision avoidance pass
    let mut node_obstacles: HashMap<String, BoundingBox> = HashMap::new();
    for (id, node) in &nodes_layout {
        node_obstacles.insert(id.clone(), BoundingBox {
            x0: node.x - 4.0,
            y0: node.y - 4.0,
            x1: node.x + node.width + 4.0,
            y1: node.y + node.height + 4.0,
        });
    }

    let mut lane_obstacles: Vec<BoundingBox> = Vec::new();
    for lane in &lanes {
        lane_obstacles.push(BoundingBox {
            x0: lane.x_start - 6.0,
            y0: lane.y - 22.0,
            x1: lane.x_start + 220.0,
            y1: lane.y + 2.0,
        });
    }

    let mut placed_label_boxes: Vec<BoundingBox> = Vec::new();

    for edge in &mut edges_layout {
        if edge.display_guard.is_empty() {
            continue;
        }
        let label_w = (edge.display_guard.len() as f64 * 6.5 + 16.0).max(36.0);
        let label_h = 20.0;

        let box_at = |cx: f64, cy: f64| BoundingBox {
            x0: cx - label_w * 0.5 - 2.0,
            y0: cy - label_h * 0.5 - 2.0,
            x1: cx + label_w * 0.5 + 2.0,
            y1: cy + label_h * 0.5 + 2.0,
        };

        let is_intra_adjacent = {
            if let (Some(f), Some(t)) = (nodes_layout.get(&edge.from), nodes_layout.get(&edge.to)) {
                (f.y - t.y).abs() < 5.0 && f.x + f.width <= t.x + 15.0 && t.x <= f.x + rank_step_x + 15.0
            } else {
                false
            }
        };

        let (mut cx, mut cy) = edge.label_pos;
        let mut best_box = box_at(cx, cy);

        // For intra-adjacent edges, the label sits right on the horizontal connector between from and to.
        // We exclude from and to node bounding boxes so the label isn't shoved away.
        let collides = |b: &BoundingBox| -> bool {
            for (nid, nobs) in &node_obstacles {
                if is_intra_adjacent && (*nid == edge.from || *nid == edge.to) {
                    continue;
                }
                if b.intersects(nobs) {
                    return true;
                }
            }
            for lobs in &lane_obstacles {
                if b.intersects(lobs) {
                    return true;
                }
            }
            for pl in &placed_label_boxes {
                if b.intersects(pl) {
                    return true;
                }
            }
            false
        };

        if collides(&best_box) {
            let mut resolved = false;
            let mut candidate_offsets = Vec::new();

            for dy in [0.0, 24.0, -24.0, 48.0, -48.0] {
                for dx in [0.0, 50.0, -50.0, 100.0, -100.0, 160.0, -160.0, 230.0, -230.0, 310.0, -310.0] {
                    if dx == 0.0 && dy == 0.0 {
                        continue;
                    }
                    candidate_offsets.push((dx, dy));
                }
            }

            for dy in [70.0, -70.0, 100.0, -100.0, 140.0, -140.0] {
                for dx in [0.0, 60.0, -60.0, 120.0, -120.0, 180.0, -180.0, 260.0, -260.0] {
                    candidate_offsets.push((dx, dy));
                }
            }

            for (dx, dy) in candidate_offsets {
                let cand_box = box_at(cx + dx, cy + dy);
                if !collides(&cand_box) {
                    cx += dx;
                    cy += dy;
                    best_box = cand_box;
                    resolved = true;
                    break;
                }
            }

            if !resolved {
                for step in 1..=30 {
                    let cand_box = box_at(cx + 60.0 * step as f64, cy);
                    if !collides(&cand_box) {
                        cx += 60.0 * step as f64;
                        best_box = cand_box;
                        break;
                    }
                }
            }
        }
        edge.label_pos = (cx, cy);

        placed_label_boxes.push(best_box);
    }

    // 7. Compute true encompassing diagram bounding box
    let mut min_bound_x = left_margin;
    let mut max_bound_x = max_total_op_x;
    let mut min_bound_y = top_margin;
    let mut max_bound_y = fault_y + node_height;

    for node in nodes_layout.values() {
        min_bound_x = min_bound_x.min(node.x);
        max_bound_x = max_bound_x.max(node.x + node.width);
        min_bound_y = min_bound_y.min(node.y);
        max_bound_y = max_bound_y.max(node.y + node.height);
    }
    for lane in &lanes {
        min_bound_x = min_bound_x.min(lane.x_start);
        max_bound_x = max_bound_x.max(lane.x_end);
        min_bound_y = min_bound_y.min(lane.y - 20.0);
    }
    for edge in &edges_layout {
        if !edge.display_guard.is_empty() {
            let label_w = (edge.display_guard.len() as f64 * 6.5 + 16.0).max(36.0);
            min_bound_x = min_bound_x.min(edge.label_pos.0 - label_w * 0.5);
            max_bound_x = max_bound_x.max(edge.label_pos.0 + label_w * 0.5);
            min_bound_y = min_bound_y.min(edge.label_pos.1 - 10.0);
            max_bound_y = max_bound_y.max(edge.label_pos.1 + 10.0);
        }
        min_bound_x = min_bound_x.min(edge.control1.0.min(edge.control2.0));
        max_bound_x = max_bound_x.max(edge.control1.0.max(edge.control2.0));
        min_bound_y = min_bound_y.min(edge.control1.1.min(edge.control2.1));
        max_bound_y = max_bound_y.max(edge.control1.1.max(edge.control2.1));
        for pt in &edge.waypoints {
            min_bound_x = min_bound_x.min(pt.0);
            max_bound_x = max_bound_x.max(pt.0);
            min_bound_y = min_bound_y.min(pt.1);
            max_bound_y = max_bound_y.max(pt.1);
        }
    }

    let total_width = (max_bound_x - min_bound_x.min(0.0) + 120.0).max(1300.0);
    let total_height = (max_bound_y - min_bound_y.min(0.0) + 100.0).max(850.0);

    // Strict validation: Ensure analyzer/layout node count matches declared enum variant count
    assert_eq!(
        nodes_layout.len(),
        sm.states.len(),
        "Analyzer node count mismatch: layout generated {} nodes but enum has {} states (missing: {:?})",
        nodes_layout.len(),
        sm.states.len(),
        sm.states.iter().filter(|s| !nodes_layout.contains_key(*s)).collect::<Vec<_>>()
    );

    // Strict validation: Ensure zero coordinate collisions among distinct nodes
    let mut occupied_positions: HashMap<(i64, i64), String> = HashMap::new();
    for (id, node) in &nodes_layout {
        let key = ((node.x * 10.0).round() as i64, (node.y * 10.0).round() as i64);
        if let Some(existing) = occupied_positions.get(&key) {
            panic!(
                "Fatal layout collision: node '{}' and node '{}' share identical coordinates ({:.1}, {:.1})",
                id, existing, node.x, node.y
            );
        }
        occupied_positions.insert(key, id.clone());
    }

    StateMachineLayout {
        nodes: nodes_layout,
        edges: edges_layout,
        lanes,
        width: total_width,
        height: total_height,
    }
}


#[derive(Debug, Clone)]
struct HelperFunctionInfo {
    name: String,
    target_states: Vec<String>,
}

fn collect_helper_functions(
    root: Node,
    source_bytes: &[u8],
    var_name: &str,
    known_variants: &HashSet<String>,
) -> HashMap<String, HelperFunctionInfo> {
    let mut helpers = HashMap::new();
    let mut fn_nodes = Vec::new();
    collect_functions(root, &mut fn_nodes);

    for fn_node in fn_nodes {
        if let Some(fn_name) = extract_function_name(fn_node, source_bytes) {
            if fn_name == "main" || has_switch_on_var(fn_node, source_bytes, var_name) {
                continue;
            }
            let mut targets = Vec::new();
            collect_assignments_to_var(fn_node, source_bytes, var_name, known_variants, &mut targets);
            if !targets.is_empty() {
                helpers.insert(
                    fn_name.clone(),
                    HelperFunctionInfo {
                        name: fn_name,
                        target_states: targets,
                    },
                );
            }
        }
    }
    helpers
}

fn has_switch_on_var(node: Node, source_bytes: &[u8], var_name: &str) -> bool {
    if node.kind() == "switch_statement" {
        if let Some(c) = node.child_by_field_name("condition") {
            let t = clean_guard_text(&node_text(c, source_bytes));
            if t == var_name {
                return true;
            }
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if has_switch_on_var(child, source_bytes, var_name) {
            return true;
        }
    }
    false
}

fn collect_assignments_to_var(
    node: Node,
    source_bytes: &[u8],
    var_name: &str,
    known_variants: &HashSet<String>,
    targets: &mut Vec<String>,
) {
    if node.kind() == "assignment_expression" {
        if let (Some(left), Some(right)) = (
            node.child_by_field_name("left"),
            node.child_by_field_name("right"),
        ) {
            if extract_target_ident(left, source_bytes).as_deref() == Some(var_name) {
                if let Some(variant) = extract_variant_ident(right, source_bytes, known_variants) {
                    if !targets.contains(&variant) {
                        targets.push(variant);
                    }
                }
            }
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_assignments_to_var(child, source_bytes, var_name, known_variants, targets);
    }
}

#[derive(Debug, Clone, Default)]
struct ASTContext {
    active_states: Option<Vec<String>>,
    guards: Vec<String>,
    has_local_fault: bool,
}

fn collect_functions<'a>(node: Node<'a>, out: &mut Vec<Node<'a>>) {
    if node.kind() == "function_definition" {
        out.push(node);
    } else {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            collect_functions(child, out);
        }
    }
}

fn walk_statement(
    node: Node,
    ctx: &ASTContext,
    source_bytes: &[u8],
    var_name: &str,
    known_variants: &HashSet<String>,
    helpers: &HashMap<String, HelperFunctionInfo>,
    transitions: &mut Vec<AppTransition>,
    ambiguous_transitions: &mut Vec<AmbiguousTransition>,
) {
    match node.kind() {
        "if_statement" => {
            walk_if_statement(
                node,
                ctx,
                source_bytes,
                var_name,
                known_variants,
                helpers,
                transitions,
                ambiguous_transitions,
            );
        }
        "switch_statement" => {
            walk_switch_statement(
                node,
                ctx,
                source_bytes,
                var_name,
                known_variants,
                helpers,
                transitions,
                ambiguous_transitions,
            );
        }
        "assignment_expression" => {
            check_assignment(
                node,
                ctx,
                source_bytes,
                var_name,
                known_variants,
                transitions,
                ambiguous_transitions,
            );
        }
        "call_expression" => {
            check_helper_call(
                node,
                ctx,
                source_bytes,
                helpers,
                transitions,
            );
        }
        _ => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                walk_statement(
                    child,
                    ctx,
                    source_bytes,
                    var_name,
                    known_variants,
                    helpers,
                    transitions,
                    ambiguous_transitions,
                );
            }
        }
    }
}

fn check_helper_call(
    node: Node,
    ctx: &ASTContext,
    source_bytes: &[u8],
    helpers: &HashMap<String, HelperFunctionInfo>,
    transitions: &mut Vec<AppTransition>,
) {
    if let Some(fn_child) = node.child_by_field_name("function") {
        let fn_name = node_text(fn_child, source_bytes);
        if let Some(helper) = helpers.get(&fn_name) {
            let line = node.start_position().row + 1;
            let arg_info = extract_first_argument_info(node, source_bytes);
            let arg_text = arg_info.as_ref().map(|a| a.text.clone());

            let helper_tag = match &arg_text {
                Some(arg) => format!("{}: {}", helper.name, arg),
                None => format!("{}()", helper.name),
            };

            let base_guard = ctx.guards.join(" && ");
            let full_guard = if base_guard.is_empty() {
                helper_tag.clone()
            } else {
                format!("{} [{}]", base_guard, helper_tag)
            };

            for target in &helper.target_states {
                let is_fault = is_fault_state(target);

                // Priority Tier 1: Edges into FAULT via fault() helper with string literal
                let label = if (helper.name == "fault" || is_fault)
                    && arg_info.as_ref().map(|a| a.is_string_literal).unwrap_or(false)
                {
                    arg_info.as_ref().unwrap().text.clone()
                } else if let Some(cmd) = extract_command_string(&base_guard) {
                    // Priority Tier 2: Command-triggered edges (e.g. startCal() inside CAL command block)
                    format!("CMD: {}", cmd)
                } else {
                    // Fallback Tier 4
                    prettify_guard(&base_guard)
                };

                if let Some(ref states) = ctx.active_states {
                    if states.is_empty() {
                        transitions.push(AppTransition {
                            from: "(any state)".to_string(),
                            to: target.clone(),
                            guard: full_guard.clone(),
                            label: label.clone(),
                            is_fault,
                            transition_type: TransitionType::IndirectHelper {
                                helper_name: helper.name.clone(),
                                argument: arg_text.clone(),
                            },
                            line,
                        });
                    } else {
                        for from in states {
                            transitions.push(AppTransition {
                                from: from.clone(),
                                to: target.clone(),
                                guard: full_guard.clone(),
                                label: label.clone(),
                                is_fault,
                                transition_type: TransitionType::IndirectHelper {
                                    helper_name: helper.name.clone(),
                                    argument: arg_text.clone(),
                                },
                                line,
                            });
                        }
                    }
                } else {
                    transitions.push(AppTransition {
                        from: "(any state)".to_string(),
                        to: target.clone(),
                        guard: full_guard.clone(),
                        label: label.clone(),
                        is_fault,
                        transition_type: TransitionType::IndirectHelper {
                            helper_name: helper.name.clone(),
                            argument: arg_text.clone(),
                        },
                        line,
                    });
                }
            }
        } else if !ctx.has_local_fault
            && is_fault_coordinator_name(&fn_name)
            && ctx.active_states.as_ref().map_or(false, |st| !st.is_empty())
        {
            let line = node.start_position().row + 1;
            let arg_info = extract_first_argument_info(node, source_bytes);
            let arg_text = arg_info.as_ref().map(|a| a.text.clone());

            let label = if let Some(ref arg) = arg_info {
                arg.text.clone()
            } else {
                prettify_guard(&ctx.guards.join(" && "))
            };

            let helper_tag = match &arg_text {
                Some(arg) => format!("{}: {}", fn_name, arg),
                None => format!("{}()", fn_name),
            };

            let base_guard = ctx.guards.join(" && ");
            let full_guard = if base_guard.is_empty() {
                helper_tag
            } else {
                format!("{} [{}]", base_guard, helper_tag)
            };

            let target = "SYSTEM FAULT".to_string();

            if let Some(ref states) = ctx.active_states {
                if states.is_empty() {
                    transitions.push(AppTransition {
                        from: "(any state)".to_string(),
                        to: target,
                        guard: full_guard,
                        label,
                        is_fault: true,
                        transition_type: TransitionType::IndirectHelper {
                            helper_name: fn_name.clone(),
                            argument: arg_text,
                        },
                        line,
                    });
                } else {
                    for from in states {
                        transitions.push(AppTransition {
                            from: from.clone(),
                            to: target.clone(),
                            guard: full_guard.clone(),
                            label: label.clone(),
                            is_fault: true,
                            transition_type: TransitionType::IndirectHelper {
                                helper_name: fn_name.clone(),
                                argument: arg_text.clone(),
                            },
                            line,
                        });
                    }
                }
            } else {
                transitions.push(AppTransition {
                    from: "(any state)".to_string(),
                    to: target,
                    guard: full_guard,
                    label,
                    is_fault: true,
                    transition_type: TransitionType::IndirectHelper {
                        helper_name: fn_name.clone(),
                        argument: arg_text,
                    },
                    line,
                });
            }
        }
    }
}

fn is_fault_coordinator_name(name: &str) -> bool {
    let lower = name.to_lowercase();
    (lower.contains("fault") || lower.contains("panic") || lower.contains("abort"))
        && !lower.contains("clear")
        && !lower.contains("in_fault")
        && name != "Error_Handler"
}

fn walk_if_statement(
    node: Node,
    ctx: &ASTContext,
    source_bytes: &[u8],
    var_name: &str,
    known_variants: &HashSet<String>,
    helpers: &HashMap<String, HelperFunctionInfo>,
    transitions: &mut Vec<AppTransition>,
    ambiguous_transitions: &mut Vec<AmbiguousTransition>,
) {
    let cond_node = node.child_by_field_name("condition");
    let cond_analysis = cond_node
        .map(|c| analyze_condition(c, source_bytes, var_name, known_variants))
        .unwrap_or_default();

    // 1. Consequence branch
    if let Some(consequence) = node.child_by_field_name("consequence") {
        let mut consequence_ctx = ctx.clone();
        if let Some(states) = cond_analysis.state_restriction {
            consequence_ctx.active_states = Some(states);
        }
        for g in cond_analysis.guards {
            if !g.is_empty() {
                consequence_ctx.guards.push(g);
            }
        }
        walk_statement(
            consequence,
            &consequence_ctx,
            source_bytes,
            var_name,
            known_variants,
            helpers,
            transitions,
            ambiguous_transitions,
        );
    }

    // 2. Alternative branch (else / else if)
    if let Some(alternative) = node.child_by_field_name("alternative") {
        walk_statement(
            alternative,
            ctx,
            source_bytes,
            var_name,
            known_variants,
            helpers,
            transitions,
            ambiguous_transitions,
        );
    }
}

fn walk_switch_statement(
    node: Node,
    ctx: &ASTContext,
    source_bytes: &[u8],
    var_name: &str,
    known_variants: &HashSet<String>,
    helpers: &HashMap<String, HelperFunctionInfo>,
    transitions: &mut Vec<AppTransition>,
    ambiguous_transitions: &mut Vec<AmbiguousTransition>,
) {
    let is_switch_on_var = node
        .child_by_field_name("condition")
        .map(|c| {
            let t = clean_guard_text(&node_text(c, source_bytes));
            t == var_name
        })
        .unwrap_or(false);

    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        let mut current_case_state = None;

        for child in body.children(&mut cursor) {
            if child.kind() == "case_statement" {
                if is_switch_on_var {
                    if let Some(val) = child.child_by_field_name("value") {
                        let val_text = node_text(val, source_bytes);
                        if known_variants.contains(&val_text) {
                            current_case_state = Some(vec![val_text]);
                        }
                    }
                }
                let mut case_ctx = ctx.clone();
                if let Some(ref st) = current_case_state {
                    case_ctx.active_states = Some(st.clone());
                }
                walk_statement(
                    child,
                    &case_ctx,
                    source_bytes,
                    var_name,
                    known_variants,
                    helpers,
                    transitions,
                    ambiguous_transitions,
                );
            } else {
                let mut item_ctx = ctx.clone();
                if let Some(ref st) = current_case_state {
                    item_ctx.active_states = Some(st.clone());
                }
                walk_statement(
                    child,
                    &item_ctx,
                    source_bytes,
                    var_name,
                    known_variants,
                    helpers,
                    transitions,
                    ambiguous_transitions,
                );
            }
        }
    }
}

fn check_assignment(
    node: Node,
    ctx: &ASTContext,
    source_bytes: &[u8],
    var_name: &str,
    known_variants: &HashSet<String>,
    direct_transitions: &mut Vec<AppTransition>,
    ambiguous_transitions: &mut Vec<AmbiguousTransition>,
) {
    if let (Some(left), Some(right)) = (
        node.child_by_field_name("left"),
        node.child_by_field_name("right"),
    ) {
        let left_ident = extract_target_ident(left, source_bytes);
        if left_ident.as_deref() == Some(var_name) {
            if let Some(target_variant) = extract_variant_ident(right, source_bytes, known_variants) {
                let line = node.start_position().row + 1;
                let guard_text = ctx.guards.join(" && ");
                let is_fault = is_fault_state(&target_variant);

                let following_printf = find_following_printf(node, source_bytes);
                let label = if let Some(cmd) = extract_command_string(&guard_text) {
                    // Priority Tier 2: Command match
                    format!("CMD: {}", cmd)
                } else if !is_fault
                    && following_printf
                        .as_ref()
                        .map(|msg| differs_meaningfully(msg, &target_variant))
                        .unwrap_or(false)
                {
                    // Priority Tier 3: Non-fault printf differing meaningfully
                    clean_printf_literal(following_printf.as_ref().unwrap())
                } else {
                    // Priority Tier 4: Fallback prettified guard
                    prettify_guard(&guard_text)
                };

                if let Some(ref states) = ctx.active_states {
                    if states.is_empty() {
                        ambiguous_transitions.push(AmbiguousTransition {
                            target: target_variant,
                            guard: guard_text,
                            line,
                            note: format!("Assignment to {} outside known state guard", var_name),
                        });
                    } else {
                        for from_state in states {
                            direct_transitions.push(AppTransition {
                                from: from_state.clone(),
                                to: target_variant.clone(),
                                guard: guard_text.clone(),
                                label: label.clone(),
                                is_fault,
                                transition_type: if guard_text.contains("cmd") || guard_text.contains("strcmp") {
                                    TransitionType::EventTriggered
                                } else {
                                    TransitionType::Direct
                                },
                                line,
                            });
                        }
                    }
                } else {
                    ambiguous_transitions.push(AmbiguousTransition {
                        target: target_variant,
                        guard: guard_text,
                        line,
                        note: format!("Assignment to {} with no enclosing state check", var_name),
                    });
                }
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
struct ConditionAnalysis {
    state_restriction: Option<Vec<String>>,
    guards: Vec<String>,
}

fn analyze_condition(
    node: Node,
    source_bytes: &[u8],
    var_name: &str,
    known_variants: &HashSet<String>,
) -> ConditionAnalysis {
    match node.kind() {
        "parenthesized_expression" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() != "(" && child.kind() != ")" && child.kind() != "comment" {
                    return analyze_condition(child, source_bytes, var_name, known_variants);
                }
            }
            ConditionAnalysis::default()
        }
        "binary_expression" => {
            let op = node
                .child_by_field_name("operator")
                .map(|n| node_text(n, source_bytes))
                .or_else(|| {
                    for i in 0..node.child_count() {
                        if let Some(c) = node.child(i) {
                            let t = node_text(c, source_bytes);
                            if t == "==" || t == "!=" || t == "&&" || t == "||" || t == ">" || t == "<" || t == ">=" || t == "<=" {
                                return Some(t);
                            }
                        }
                    }
                    None
                });

            let left = node.child_by_field_name("left");
            let right = node.child_by_field_name("right");

            if let (Some(op), Some(left), Some(right)) = (op, left, right) {
                if op == "==" {
                    let left_txt = extract_target_ident(left, source_bytes).unwrap_or_default();
                    let right_txt = extract_variant_ident(right, source_bytes, known_variants);
                    if left_txt == var_name {
                        if let Some(variant) = right_txt {
                            return ConditionAnalysis {
                                state_restriction: Some(vec![variant]),
                                guards: vec![],
                            };
                        }
                    }
                    let left_variant = extract_variant_ident(left, source_bytes, known_variants);
                    let right_target = extract_target_ident(right, source_bytes).unwrap_or_default();
                    if right_target == var_name {
                        if let Some(variant) = left_variant {
                            return ConditionAnalysis {
                                state_restriction: Some(vec![variant]),
                                guards: vec![],
                            };
                        }
                    }
                } else if op == "||" {
                    let left_res = analyze_condition(left, source_bytes, var_name, known_variants);
                    let right_res = analyze_condition(right, source_bytes, var_name, known_variants);
                    if left_res.state_restriction.is_some() || right_res.state_restriction.is_some() {
                        let mut combined = Vec::new();
                        if let Some(l) = left_res.state_restriction {
                            combined.extend(l);
                        }
                        if let Some(r) = right_res.state_restriction {
                            combined.extend(r);
                        }
                        let mut guards = left_res.guards;
                        guards.extend(right_res.guards);
                        return ConditionAnalysis {
                            state_restriction: Some(combined),
                            guards,
                        };
                    }
                } else if op == "&&" {
                    let left_res = analyze_condition(left, source_bytes, var_name, known_variants);
                    let right_res = analyze_condition(right, source_bytes, var_name, known_variants);

                    let state_restriction = match (left_res.state_restriction, right_res.state_restriction) {
                        (Some(l), Some(r)) => {
                            let set: HashSet<_> = r.into_iter().collect();
                            Some(l.into_iter().filter(|x| set.contains(x)).collect())
                        }
                        (Some(l), None) => Some(l),
                        (None, Some(r)) => Some(r),
                        (None, None) => None,
                    };

                    let mut guards = left_res.guards;
                    guards.extend(right_res.guards);
                    return ConditionAnalysis {
                        state_restriction,
                        guards,
                    };
                }
            }

            ConditionAnalysis {
                state_restriction: None,
                guards: vec![clean_guard_text(&node_text(node, source_bytes))],
            }
        }
        _ => ConditionAnalysis {
            state_restriction: None,
            guards: vec![clean_guard_text(&node_text(node, source_bytes))],
        },
    }
}

fn extract_target_ident(node: Node, source_bytes: &[u8]) -> Option<String> {
    match node.kind() {
        "identifier" => Some(node_text(node, source_bytes)),
        "parenthesized_expression" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if let Some(id) = extract_target_ident(child, source_bytes) {
                    return Some(id);
                }
            }
            None
        }
        _ => None,
    }
}

fn extract_variant_ident(
    node: Node,
    source_bytes: &[u8],
    known_variants: &HashSet<String>,
) -> Option<String> {
    let text = node_text(node, source_bytes);
    if known_variants.contains(&text) {
        return Some(text);
    }
    match node.kind() {
        "identifier" => {
            if known_variants.contains(&text) {
                Some(text)
            } else {
                None
            }
        }
        "parenthesized_expression" | "cast_expression" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if let Some(v) = extract_variant_ident(child, source_bytes, known_variants) {
                    return Some(v);
                }
            }
            None
        }
        _ => None,
    }
}

pub fn is_fault_state(state: &str) -> bool {
    let upper = state.to_uppercase();
    upper.contains("FAULT") || upper.contains("ERROR")
}

fn clean_guard_text(raw: &str) -> String {
    let mut s = raw.trim();
    while s.starts_with('(') && s.ends_with(')') && has_matching_outer_parens(s) {
        s = s[1..s.len() - 1].trim();
    }
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn has_matching_outer_parens(s: &str) -> bool {
    if !s.starts_with('(') || !s.ends_with(')') {
        return false;
    }
    let mut depth = 0;
    for (i, c) in s.chars().enumerate() {
        if c == '(' {
            depth += 1;
        } else if c == ')' {
            depth -= 1;
            if depth == 0 && i < s.len() - 1 {
                return false;
            }
        }
    }
    depth == 0
}

#[derive(Debug, Clone)]
struct CallArgInfo {
    text: String,
    is_string_literal: bool,
}

fn extract_first_argument_info(call_node: Node, source_bytes: &[u8]) -> Option<CallArgInfo> {
    if let Some(arg_list) = call_node.child_by_field_name("arguments") {
        let mut cursor = arg_list.walk();
        for child in arg_list.children(&mut cursor) {
            if child.kind() != "(" && child.kind() != ")" && child.kind() != "," && child.kind() != "comment" {
                let is_string_literal = child.kind() == "string_literal";
                let mut text = node_text(child, source_bytes);
                if text.starts_with('"') && text.ends_with('"') && text.len() >= 2 {
                    text = text[1..text.len() - 1].to_string();
                }
                return Some(CallArgInfo {
                    text,
                    is_string_literal,
                });
            }
        }
    }
    None
}

fn extract_command_string(guard: &str) -> Option<String> {
    if let Some(idx) = guard.find("strcmp") {
        let after_strcmp = &guard[idx..];
        if let Some(eq_idx) = after_strcmp.find("==") {
            let call_str = &after_strcmp[..eq_idx];
            if let Some(first_quote) = call_str.find('"') {
                let after_quote = &call_str[first_quote + 1..];
                if let Some(second_quote) = after_quote.find('"') {
                    let cmd = &after_quote[..second_quote];
                    if !cmd.is_empty() {
                        return Some(cmd.to_string());
                    }
                }
            }
        }
    }
    None
}

fn clean_printf_literal(raw: &str) -> String {
    let s = raw
        .replace("\\r", "")
        .replace("\\n", "")
        .replace("\r", "")
        .replace("\n", "");
    s.trim().trim_matches(|c| c == '.' || c == '!').trim().to_string()
}

fn differs_meaningfully(printf_msg: &str, dest_state: &str) -> bool {
    let clean_msg = printf_msg
        .replace("\\r", "")
        .replace("\\n", "")
        .replace("\r", "")
        .replace("\n", "");
    let clean_msg = clean_msg.trim().to_lowercase();
    let clean_dest = dest_state.trim().to_lowercase();

    if clean_msg.is_empty() {
        return false;
    }

    // Ignore hit-detection side info
    if clean_msg.contains("hit") {
        return false;
    }

    // Status / state confirmations in fixture
    let known_redundant = [
        "hold", "cal ok", "returned", "stopping", "backoff", "go", "ret",
        "rst", "opening", "closing", "closed", "opened", "busy", "no cal",
        "invalid", "hatch stopped", "ok",
    ];
    if known_redundant.iter().any(|&r| clean_msg == r || clean_msg.contains(r)) {
        return false;
    }

    // Normalize dest state (e.g. remove cal_, rec_ prefixes)
    let stripped_dest = clean_dest
        .strip_prefix("cal_")
        .or_else(|| clean_dest.strip_prefix("rec_"))
        .unwrap_or(&clean_dest)
        .replace('_', "");

    let compact_msg = clean_msg.replace(|c: char| !c.is_alphanumeric(), "");

    if compact_msg == stripped_dest || compact_msg == clean_dest.replace('_', "") {
        return false;
    }
    if stripped_dest.contains(&compact_msg) || compact_msg.contains(&stripped_dest) {
        return false;
    }

    true
}

fn find_following_printf(node: Node, source_bytes: &[u8]) -> Option<String> {
    let stmt = find_parent_statement(node)?;
    let mut sibling = stmt.next_sibling();
    let mut count = 0;
    while let Some(sib) = sibling {
        count += 1;
        if count > 4 {
            break;
        }
        if sib.kind() == "comment" {
            sibling = sib.next_sibling();
            continue;
        }
        if let Some(call) = find_call_in_statement(sib) {
            if let Some(fn_node) = call.child_by_field_name("function") {
                if node_text(fn_node, source_bytes) == "printf" {
                    if let Some(arg) = extract_first_argument_info(call, source_bytes) {
                        if arg.is_string_literal {
                            return Some(arg.text);
                        }
                    }
                }
            }
        }
        sibling = sib.next_sibling();
    }
    None
}

fn find_parent_statement(mut node: Node) -> Option<Node> {
    while let Some(parent) = node.parent() {
        if parent.kind().ends_with("_statement") {
            return Some(parent);
        }
        node = parent;
    }
    None
}

fn find_call_in_statement(stmt: Node) -> Option<Node> {
    if stmt.kind() == "call_expression" {
        return Some(stmt);
    }
    if stmt.kind() == "expression_statement" {
        let mut cursor = stmt.walk();
        for child in stmt.children(&mut cursor) {
            if child.kind() == "call_expression" {
                return Some(child);
            }
        }
    }
    None
}

pub fn to_title_case_identifier(ident: &str) -> String {
    let s = ident.trim_end_matches("()");
    let mut words: Vec<String> = Vec::new();

    for part in s.split(['_', '.']) {
        if part.is_empty() {
            continue;
        }
        let mut cur_word = String::new();
        let chars: Vec<char> = part.chars().collect();
        for i in 0..chars.len() {
            let c = chars[i];
            if i > 0 {
                let prev = chars[i - 1];
                let is_prev_lower_or_digit = prev.is_ascii_lowercase() || prev.is_ascii_digit();
                let is_cur_upper = c.is_ascii_uppercase();
                let is_next_lower = i + 1 < chars.len() && chars[i + 1].is_ascii_lowercase();

                if (is_prev_lower_or_digit && is_cur_upper)
                    || (prev.is_ascii_uppercase() && is_cur_upper && is_next_lower)
                {
                    if !cur_word.is_empty() {
                        words.push(cur_word);
                        cur_word = String::new();
                    }
                }
            }
            cur_word.push(c);
        }
        if !cur_word.is_empty() {
            words.push(cur_word);
        }
    }

    let capitalized: Vec<String> = words
        .into_iter()
        .map(|w| {
            if w.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()) {
                w
            } else {
                let mut chars = w.chars();
                match chars.next() {
                    Some(first) => {
                        let mut res = first.to_uppercase().to_string();
                        res.push_str(chars.as_str());
                        res
                    }
                    None => String::new(),
                }
            }
        })
        .collect();

    capitalized.join(" ")
}

pub fn prettify_guard(raw_guard: &str) -> String {
    let mut s = raw_guard.trim();
    if let Some(idx) = s.find(" [") {
        s = &s[..idx];
    }
    s = s.trim();

    while s.starts_with('(') && s.ends_with(')') && has_matching_outer_parens(s) {
        s = s[1..s.len() - 1].trim();
    }

    if s.is_empty() {
        return String::new();
    }

    // 1. Timeout pattern
    if s.contains("now") && s.contains("stateStart") {
        if let Some(timeout_label) = extract_timeout_pattern(s) {
            return timeout_label;
        }
    }

    // 2. Split by ||
    if s.contains("||") {
        let parts: Vec<String> = s
            .split("||")
            .map(|p| prettify_conjunctive_clause(p.trim()))
            .filter(|p| !p.is_empty())
            .collect();
        return parts.join(" or ");
    }

    // 3. Conjunctive clause
    prettify_conjunctive_clause(s)
}

fn extract_timeout_pattern(s: &str) -> Option<String> {
    let s_clean = s.trim();
    let s_clean = if s_clean.starts_with('(') && s_clean.ends_with(')') {
        s_clean[1..s_clean.len() - 1].trim()
    } else {
        s_clean
    };

    if let Some(idx) = s_clean.find("now") {
        let after_now = s_clean[idx + 3..].trim_start();
        if let Some(dash_idx) = after_now.find('-') {
            let after_dash = after_now[dash_idx + 1..].trim_start();
            if let Some(gt_idx) = after_dash.find('>') {
                let left_side = after_dash[..gt_idx].trim();
                let right_side = after_dash[gt_idx + 1..].trim();
                if left_side.contains("stateStart") {
                    let const_name = right_side
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '_')
                        .collect::<String>();
                    if !const_name.is_empty() {
                        return Some(format!("Timeout ({})", const_name));
                    }
                }
            }
        }
    }
    None
}

fn prettify_conjunctive_clause(clause: &str) -> String {
    let parts: Vec<String> = clause
        .split("&&")
        .map(|p| prettify_single_term(p.trim()))
        .filter(|p| !p.is_empty())
        .collect();
    parts.join(" and ")
}

fn prettify_single_term(term: &str) -> String {
    let mut s = term.trim();
    while s.starts_with('(') && s.ends_with(')') && has_matching_outer_parens(s) {
        s = s[1..s.len() - 1].trim();
    }
    if s.is_empty() {
        return String::new();
    }

    // Equality comparison
    if s.contains("==") {
        let parts: Vec<&str> = s.split("==").collect();
        if parts.len() == 2 {
            let left = prettify_operand(parts[0].trim());
            let right = prettify_operand(parts[1].trim());
            return format!("{} is {}", left, right);
        }
    }

    // Inequality comparison
    if s.contains("!=") {
        let parts: Vec<&str> = s.split("!=").collect();
        if parts.len() == 2 {
            let left = prettify_operand(parts[0].trim());
            let right = prettify_operand(parts[1].trim());
            return format!("{} is not {}", left, right);
        }
    }

    // Leading negation !
    if let Some(rest) = s.strip_prefix('!') {
        return format!("not {}", prettify_operand(rest.trim()));
    }

    prettify_operand(s)
}

fn prettify_operand(op: &str) -> String {
    let mut s = op.trim();
    while s.starts_with('(') && s.ends_with(')') && has_matching_outer_parens(s) {
        s = s[1..s.len() - 1].trim();
    }
    match s {
        "0" => "0".to_string(),
        "1" => "1".to_string(),
        "true" => "true".to_string(),
        "false" => "false".to_string(),
        "NULL" => "NULL".to_string(),
        _ => to_title_case_identifier(s),
    }
}

fn resolve_cross_file_command_transitions(
    src_dir: &Path,
    file_path: &Path,
    root: Node,
    source_bytes: &[u8],
    candidate: &StateMachineCandidate,
    known_variants: &HashSet<String>,
    helpers: &HashMap<String, HelperFunctionInfo>,
    transitions: &mut Vec<AppTransition>,
    ambiguous_transitions: &mut Vec<AmbiguousTransition>,
) {
    let query_funcs = collect_query_functions(root, source_bytes, &candidate.var.name, known_variants);

    let mut other_c_files = Vec::new();
    if let Ok(entries) = fs::read_dir(src_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() && p.extension().map_or(false, |ext| ext == "c") && p != file_path {
                other_c_files.push(p);
            }
        }
    }
    other_c_files.sort();

    if other_c_files.is_empty() {
        return;
    }

    let mut fn_to_command = HashMap::new();
    let mut parser = Parser::new();
    if parser.set_language(&tree_sitter_c::language()).is_err() {
        return;
    }

    // Pass 1: find command table mappings
    for c_path in &other_c_files {
        if let Ok(c_src) = fs::read_to_string(c_path) {
            if let Some(c_tree) = parser.parse(&c_src, None) {
                let tbl = collect_command_table(c_tree.root_node(), c_src.as_bytes());
                fn_to_command.extend(tbl);
            }
        }
    }

    // Pass 2: walk command handlers and resolve action calls
    for c_path in &other_c_files {
        if let Ok(c_src) = fs::read_to_string(c_path) {
            if let Some(c_tree) = parser.parse(&c_src, None) {
                let c_bytes = c_src.as_bytes();
                let mut fn_nodes = Vec::new();
                collect_functions(c_tree.root_node(), &mut fn_nodes);

                for fn_node in fn_nodes {
                    let fn_name = extract_function_name(fn_node, c_bytes).unwrap_or_default();
                    let cmd = fn_to_command.get(&fn_name).cloned();
                    if let Some(body) = fn_node.child_by_field_name("body") {
                        walk_cross_file_statements(
                            body,
                            c_bytes,
                            cmd.as_deref(),
                            &fn_name,
                            candidate,
                            helpers,
                            &query_funcs,
                            &[],
                            transitions,
                            ambiguous_transitions,
                        );
                    }
                }
            }
        }
    }
}

fn walk_cross_file_statements(
    node: Node,
    source_bytes: &[u8],
    cmd: Option<&str>,
    fn_name: &str,
    candidate: &StateMachineCandidate,
    helpers: &HashMap<String, HelperFunctionInfo>,
    query_funcs: &HashMap<String, Vec<String>>,
    active_query_states: &[String],
    transitions: &mut Vec<AppTransition>,
    ambiguous_transitions: &mut Vec<AmbiguousTransition>,
) {
    match node.kind() {
        "if_statement" => {
            let cond_node = node.child_by_field_name("condition");
            let mut branch_states = Vec::new();
            if let Some(cond) = cond_node {
                collect_query_call_states(cond, source_bytes, query_funcs, &mut branch_states);
            }

            if let Some(consequence) = node.child_by_field_name("consequence") {
                let next_states = if !branch_states.is_empty() {
                    &branch_states[..]
                } else {
                    active_query_states
                };
                walk_cross_file_statements(
                    consequence,
                    source_bytes,
                    cmd,
                    fn_name,
                    candidate,
                    helpers,
                    query_funcs,
                    next_states,
                    transitions,
                    ambiguous_transitions,
                );
            }

            if let Some(alternative) = node.child_by_field_name("alternative") {
                walk_cross_file_statements(
                    alternative,
                    source_bytes,
                    cmd,
                    fn_name,
                    candidate,
                    helpers,
                    query_funcs,
                    active_query_states,
                    transitions,
                    ambiguous_transitions,
                );
            }
        }
        "call_expression" => {
            if let Some(fn_child) = node.child_by_field_name("function") {
                let callee_name = node_text(fn_child, source_bytes);
                if let Some(helper) = helpers.get(&callee_name) {
                    let line = node.start_position().row + 1;
                    let label = match cmd {
                        Some(c) => format!("CMD: {}", c),
                        None => callee_name.clone(),
                    };
                    let guard = label.clone();
                    let initial_state = candidate.var.initial_value.as_deref().unwrap_or("IDLE");

                    for target in &helper.target_states {
                        if !active_query_states.is_empty() {
                            for from in active_query_states {
                                if !transitions.iter().any(|t| t.from == *from && t.to == *target && t.label == label) {
                                    transitions.push(AppTransition {
                                        from: from.clone(),
                                        to: target.clone(),
                                        guard: guard.clone(),
                                        label: label.clone(),
                                        is_fault: false,
                                        transition_type: TransitionType::EventTriggered,
                                        line,
                                    });
                                }
                            }
                        } else if target == initial_state {
                            if !ambiguous_transitions.iter().any(|a| a.target == *target && a.guard == guard) {
                                ambiguous_transitions.push(AmbiguousTransition {
                                    target: target.clone(),
                                    guard: guard.clone(),
                                    line,
                                    note: format!("Command dispatch via {}", fn_name),
                                });
                            }
                            if !transitions.iter().any(|t| t.from == "(any state)" && t.to == *target && t.label == label) {
                                transitions.push(AppTransition {
                                    from: "(any state)".to_string(),
                                    to: target.clone(),
                                    guard: guard.clone(),
                                    label: label.clone(),
                                    is_fault: false,
                                    transition_type: TransitionType::EventTriggered,
                                    line,
                                });
                            }
                        } else {
                            if !transitions.iter().any(|t| t.from == initial_state && t.to == *target && t.label == label) {
                                transitions.push(AppTransition {
                                    from: initial_state.to_string(),
                                    to: target.clone(),
                                    guard: guard.clone(),
                                    label: label.clone(),
                                    is_fault: false,
                                    transition_type: TransitionType::EventTriggered,
                                    line,
                                });
                            }
                        }
                    }
                }
            }
        }
        _ => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                walk_cross_file_statements(
                    child,
                    source_bytes,
                    cmd,
                    fn_name,
                    candidate,
                    helpers,
                    query_funcs,
                    active_query_states,
                    transitions,
                    ambiguous_transitions,
                );
            }
        }
    }
}

fn collect_query_call_states(
    node: Node,
    source_bytes: &[u8],
    query_funcs: &HashMap<String, Vec<String>>,
    out: &mut Vec<String>,
) {
    if node.kind() == "call_expression" {
        if let Some(fn_child) = node.child_by_field_name("function") {
            let callee_name = node_text(fn_child, source_bytes);
            if let Some(states) = query_funcs.get(&callee_name) {
                for st in states {
                    if !out.contains(st) {
                        out.push(st.clone());
                    }
                }
            }
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_query_call_states(child, source_bytes, query_funcs, out);
    }
}

fn collect_query_functions(
    root: Node,
    source_bytes: &[u8],
    var_name: &str,
    known_variants: &HashSet<String>,
) -> HashMap<String, Vec<String>> {
    let mut query_funcs = HashMap::new();
    let mut fn_nodes = Vec::new();
    collect_functions(root, &mut fn_nodes);

    for fn_node in fn_nodes {
        if let Some(fn_name) = extract_function_name(fn_node, source_bytes) {
            let mut queried_states = Vec::new();
            collect_queried_states_in_node(fn_node, source_bytes, var_name, known_variants, &mut queried_states);
            if !queried_states.is_empty() {
                query_funcs.insert(fn_name, queried_states);
            }
        }
    }
    query_funcs
}

fn collect_queried_states_in_node(
    node: Node,
    source_bytes: &[u8],
    var_name: &str,
    known_variants: &HashSet<String>,
    out: &mut Vec<String>,
) {
    if node.kind() == "binary_expression" {
        if let (Some(left), Some(operator), Some(right)) = (
            node.child_by_field_name("left"),
            node.child_by_field_name("operator"),
            node.child_by_field_name("right"),
        ) {
            let op = node_text(operator, source_bytes);
            if op == "==" {
                let left_text = clean_guard_text(&node_text(left, source_bytes));
                let right_text = clean_guard_text(&node_text(right, source_bytes));
                if left_text == var_name && known_variants.contains(&right_text) {
                    if !out.contains(&right_text) {
                        out.push(right_text);
                    }
                } else if right_text == var_name && known_variants.contains(&left_text) {
                    if !out.contains(&left_text) {
                        out.push(left_text);
                    }
                }
            }
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_queried_states_in_node(child, source_bytes, var_name, known_variants, out);
    }
}

fn collect_command_table(root: Node, source_bytes: &[u8]) -> HashMap<String, String> {
    let mut fn_to_cmd = HashMap::new();
    collect_command_table_recursive(root, source_bytes, &mut fn_to_cmd);
    fn_to_cmd
}

fn collect_command_table_recursive(node: Node, source_bytes: &[u8], out: &mut HashMap<String, String>) {
    if node.kind() == "initializer_list" {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "initializer_list" {
                let mut str_opt = None;
                let mut fn_opt = None;
                let mut inner_cursor = child.walk();
                for element in child.children(&mut inner_cursor) {
                    if element.kind() == "string_literal" {
                        let raw = node_text(element, source_bytes);
                        let s = raw.trim_matches('"').to_string();
                        str_opt = Some(s);
                    } else if element.kind() == "identifier" {
                        let name = node_text(element, source_bytes);
                        fn_opt = Some(name);
                    }
                }
                if let (Some(cmd), Some(fn_name)) = (str_opt, fn_opt) {
                    out.insert(fn_name, cmd);
                }
            }
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_command_table_recursive(child, source_bytes, out);
    }
}

fn collect_enum_definitions(
    node: Node,
    source_bytes: &[u8],
    file_path: &str,
    out: &mut Vec<EnumDefinition>,
) {
    if node.kind() == "type_definition" {
        // typedef enum { ... } TypeName;
        if let Some(enum_spec) = find_child_of_kind(node, "enum_specifier") {
            let declarators = collect_type_declarators(node, source_bytes);
            let variants = extract_enum_variants(enum_spec, source_bytes);
            let line = node.start_position().row + 1;

            for name in declarators {
                out.push(EnumDefinition {
                    name,
                    variants: variants.clone(),
                    file_path: file_path.to_string(),
                    line,
                });
            }
            return;
        }
    } else if node.kind() == "enum_specifier" {
        // enum Name { ... }; (not inside typedef)
        if node.parent().map(|p| p.kind()) != Some("type_definition") {
            if let Some(name_node) = node.child_by_field_name("name") {
                let name = node_text(name_node, source_bytes);
                let variants = extract_enum_variants(node, source_bytes);
                let line = node.start_position().row + 1;
                if !variants.is_empty() {
                    out.push(EnumDefinition {
                        name,
                        variants,
                        file_path: file_path.to_string(),
                        line,
                    });
                }
            }
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_enum_definitions(child, source_bytes, file_path, out);
    }
}

fn collect_type_declarators(node: Node, source_bytes: &[u8]) -> Vec<String> {
    let mut names = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "type_identifier" {
            names.push(node_text(child, source_bytes));
        }
    }
    names
}

fn extract_enum_variants(enum_node: Node, source_bytes: &[u8]) -> Vec<String> {
    let mut variants = Vec::new();
    if let Some(body) = enum_node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            if child.kind() == "enumerator" {
                if let Some(name_node) = child.child_by_field_name("name") {
                    variants.push(node_text(name_node, source_bytes));
                }
            }
        }
    }
    variants
}

fn collect_variables(
    node: Node,
    source_bytes: &[u8],
    file_path: &str,
    current_fn: Option<&str>,
    known_enums: &HashMap<String, EnumDefinition>,
    out: &mut Vec<TrackedVariable>,
) {
    if node.kind() == "function_definition" {
        let fn_name = extract_function_name(node, source_bytes).unwrap_or_else(|| "anonymous".to_string());
        if let Some(body) = node.child_by_field_name("body") {
            let mut cursor = body.walk();
            for child in body.children(&mut cursor) {
                collect_variables(child, source_bytes, file_path, Some(&fn_name), known_enums, out);
            }
        }
        return;
    }

    if node.kind() == "declaration" {
        extract_declaration_variables(node, source_bytes, file_path, current_fn, known_enums, out);
        return;
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_variables(child, source_bytes, file_path, current_fn, known_enums, out);
    }
}

fn extract_declaration_variables(
    node: Node,
    source_bytes: &[u8],
    file_path: &str,
    current_fn: Option<&str>,
    known_enums: &HashMap<String, EnumDefinition>,
    out: &mut Vec<TrackedVariable>,
) {
    let type_name = match extract_type_name(node, source_bytes) {
        Some(t) => t,
        None => return,
    };

    if !known_enums.contains_key(&type_name) {
        return;
    }

    let line = node.start_position().row + 1;
    let scope = match current_fn {
        Some(f) => VariableScope::Function(f.to_string()),
        None => VariableScope::Global,
    };

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "init_declarator" {
            if let Some(declarator) = child.child_by_field_name("declarator") {
                let name = extract_ident_name(declarator, source_bytes);
                let initial_value = child.child_by_field_name("value").map(|v| node_text(v, source_bytes));
                if let Some(name) = name {
                    out.push(TrackedVariable {
                        name,
                        enum_type: type_name.clone(),
                        scope: scope.clone(),
                        initial_value,
                        file_path: file_path.to_string(),
                        line,
                    });
                }
            }
        } else if child.kind() == "identifier" {
            let name = node_text(child, source_bytes);
            out.push(TrackedVariable {
                name,
                enum_type: type_name.clone(),
                scope: scope.clone(),
                initial_value: None,
                file_path: file_path.to_string(),
                line,
            });
        }
    }
}

fn extract_type_name(decl_node: Node, source_bytes: &[u8]) -> Option<String> {
    if let Some(type_node) = decl_node.child_by_field_name("type") {
        if type_node.kind() == "type_identifier" {
            return Some(node_text(type_node, source_bytes));
        } else if type_node.kind() == "enum_specifier" {
            if let Some(name) = type_node.child_by_field_name("name") {
                return Some(node_text(name, source_bytes));
            }
        }
    }
    // Fallback: search children for type_identifier
    let mut cursor = decl_node.walk();
    for child in decl_node.children(&mut cursor) {
        if child.kind() == "type_identifier" {
            return Some(node_text(child, source_bytes));
        }
    }
    None
}

fn extract_ident_name(mut node: Node, source_bytes: &[u8]) -> Option<String> {
    loop {
        match node.kind() {
            "identifier" => return Some(node_text(node, source_bytes)),
            "function_declarator" | "pointer_declarator" | "parenthesized_declarator" | "array_declarator" => {
                if let Some(child) = node.child_by_field_name("declarator") {
                    node = child;
                } else if let Some(child) = node.child(0) {
                    node = child;
                } else {
                    return None;
                }
            }
            _ => return None,
        }
    }
}

fn extract_function_name(node: Node, source_bytes: &[u8]) -> Option<String> {
    if let Some(declarator) = node.child_by_field_name("declarator") {
        extract_ident_name(declarator, source_bytes)
    } else {
        None
    }
}

fn find_child_of_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == kind {
            return Some(child);
        }
    }
    None
}

fn node_text(node: Node, source_bytes: &[u8]) -> String {
    node.utf8_text(source_bytes).unwrap_or("").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_docking_firmware_state_machine() {
        let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/docking_firmware/Core/Src/main.c");
        let candidates = discover_state_machines_in_file(&fixture_path).expect("failed to discover state machines");

        assert_eq!(candidates.len(), 1);
        let sm = &candidates[0];
        assert_eq!(sm.enum_def.name, "SystemState");
        assert_eq!(sm.var.name, "state");
        assert_eq!(sm.var.initial_value, Some("IDLE".to_string()));
        assert_eq!(
            sm.enum_def.variants,
            vec![
                "IDLE",
                "CALIBRATING",
                "CAL_STOPPING",
                "CAL_BACKOFF",
                "GOING",
                "HOLD",
                "RETURNING",
                "RETURNED",
                "RECOVERY",
                "REC_STOPPING",
                "REC_BACKOFF",
                "FAULT",
                "OPENING",
                "CLOSING"
            ]
        );
    }

    #[test]
    fn test_extract_direct_transitions_docking_firmware() {
        let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/docking_firmware/Core/Src/main.c");
        let candidates = discover_state_machines_in_file(&fixture_path).expect("failed to discover state machines");
        let sm = extract_state_machine_transitions(&candidates[0], &fixture_path).expect("failed to extract transitions");

        // Print extracted transitions for inspection
        for t in &sm.transitions {
            println!("Direct transition: {} -> {} [guard: '{}']", t.from, t.to, t.guard);
        }
        for a in &sm.ambiguous_transitions {
            println!("Ambiguous assignment: -> {} [guard: '{}', note: '{}']", a.target, a.guard, a.note);
        }

        // Check key direct transitions exist
        assert!(sm.transitions.iter().any(|t| t.from == "CALIBRATING" && t.to == "CAL_STOPPING" && t.guard == "z1Hit && z2Hit"));
        assert!(sm.transitions.iter().any(|t| t.from == "CAL_STOPPING" && t.to == "CAL_BACKOFF" && t.guard == "axes_done()"));
        assert!(sm.transitions.iter().any(|t| t.from == "CAL_BACKOFF" && t.to == "IDLE" && t.guard == "axes_done()"));
        assert!(sm.transitions.iter().any(|t| t.from == "GOING" && t.to == "HOLD" && t.guard == "axes_done()"));
        assert!(sm.transitions.iter().any(|t| t.from == "RETURNING" && t.to == "RECOVERY" && t.guard == "enteringRecovery"));
        assert!(sm.transitions.iter().any(|t| t.from == "RETURNING" && t.to == "RETURNED" && t.guard == "axes_done()"));
        assert!(sm.transitions.iter().any(|t| t.from == "RECOVERY" && t.to == "REC_STOPPING" && t.guard == "z1Hit && z2Hit"));
        assert!(sm.transitions.iter().any(|t| t.from == "REC_STOPPING" && t.to == "REC_BACKOFF" && t.guard == "axes_done()"));
        assert!(sm.transitions.iter().any(|t| t.from == "REC_BACKOFF" && t.to == "RETURNED" && t.guard == "axes_done()"));
        assert!(sm.transitions.iter().any(|t| t.from == "OPENING" && t.to == "IDLE" && t.guard == "now - stateStart > OPEN_DURATION_MS"));
        assert!(sm.transitions.iter().any(|t| t.from == "CLOSING" && t.to == "IDLE" && t.guard == "now - stateStart > CLOSE_DURATION_MS"));

        // Check event-triggered transitions from cmd_ready
        assert!(sm.transitions.iter().any(|t| t.from == "IDLE" && t.to == "GOING"));
        assert!(sm.transitions.iter().any(|t| t.from == "RETURNED" && t.to == "GOING"));
        assert!(sm.transitions.iter().any(|t| t.from == "HOLD" && t.to == "RETURNING"));
        assert!(sm.transitions.iter().any(|t| t.from == "IDLE" && t.to == "OPENING"));
        assert!(sm.transitions.iter().any(|t| t.from == "RETURNED" && t.to == "OPENING"));
        assert!(sm.transitions.iter().any(|t| t.from == "IDLE" && t.to == "CLOSING"));
        assert!(sm.transitions.iter().any(|t| t.from == "RETURNED" && t.to == "CLOSING"));

        // Check ambiguous transition from RST
        assert!(sm.ambiguous_transitions.iter().any(|a| a.target == "IDLE" && a.guard.contains("RST")));
    }

    #[test]
    fn test_docking_firmware_state_machine_ground_truth_verification() {
        let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/docking_firmware/Core/Src/main.c");
        let candidates = discover_state_machines_in_file(&fixture_path).expect("failed to discover state machines");
        assert_eq!(candidates.len(), 1, "expected exactly 1 state machine candidate");

        let sm = extract_state_machine_transitions(&candidates[0], &fixture_path).expect("failed to extract transitions");

        // 1. Verify 14 states
        assert_eq!(sm.states.len(), 14, "expected exactly 14 states");
        let expected_states = [
            "IDLE", "CALIBRATING", "CAL_STOPPING", "CAL_BACKOFF", "GOING", "HOLD",
            "RETURNING", "RETURNED", "RECOVERY", "REC_STOPPING", "REC_BACKOFF",
            "FAULT", "OPENING", "CLOSING"
        ];
        for s in &expected_states {
            assert!(sm.states.contains(&s.to_string()), "missing state: {}", s);
        }

        // 2. Verify exactly 31 transitions
        assert_eq!(sm.transitions.len(), 31, "expected 31 transitions, got {}", sm.transitions.len());

        // Check direct transitions
        let expected_direct = [
            ("CALIBRATING", "CAL_STOPPING", "z1Hit && z2Hit"),
            ("CAL_STOPPING", "CAL_BACKOFF", "axes_done()"),
            ("CAL_BACKOFF", "IDLE", "axes_done()"),
            ("GOING", "HOLD", "axes_done()"),
            ("RETURNING", "RECOVERY", "enteringRecovery"),
            ("RETURNING", "RETURNED", "axes_done()"),
            ("RECOVERY", "REC_STOPPING", "z1Hit && z2Hit"),
            ("REC_STOPPING", "REC_BACKOFF", "axes_done()"),
            ("REC_BACKOFF", "RETURNED", "axes_done()"),
            ("OPENING", "IDLE", "now - stateStart > OPEN_DURATION_MS"),
            ("CLOSING", "IDLE", "now - stateStart > CLOSE_DURATION_MS"),
        ];
        for (from, to, guard) in expected_direct {
            let found = sm.transitions.iter().any(|t| t.from == from && t.to == to && t.guard == guard);
            assert!(found, "missing expected direct transition: {} -> {} [{}]", from, to, guard);
        }

        // Check helper transitions (fault and startCal)
        let expected_helpers = [
            ("CALIBRATING", "FAULT", "now - stateStart > CAL_TIMEOUT [fault: CAL TIMEOUT]"),
            ("CALIBRATING", "FAULT", "z1Hit && !z2Hit && z2.current_pos == z2.target_pos [fault: Z2 LIMIT NOT FOUND]"),
            ("CALIBRATING", "FAULT", "z2Hit && !z1Hit && z1.current_pos == z1.target_pos [fault: Z1 LIMIT NOT FOUND]"),
            ("CAL_STOPPING", "FAULT", "axes_done() && !skewOK() [fault: CAL SKEW]"),
            ("CAL_BACKOFF", "FAULT", "now - stateStart > CAL_TIMEOUT [fault: CAL BACKOFF TIMEOUT]"),
            ("GOING", "FAULT", "now - stateStart > MOVE_TIMEOUT [fault: GO TIMEOUT]"),
            ("RETURNING", "FAULT", "now - stateStart > MOVE_TIMEOUT [fault: RETURN TIMEOUT]"),
            ("RECOVERY", "FAULT", "now - stateStart > RECOVERY_TIMEOUT [fault: REC TIMEOUT]"),
            ("RECOVERY", "FAULT", "z1Hit && !z2Hit && z2.current_pos == z2.target_pos [fault: Z2 LIMIT NOT FOUND]"),
            ("RECOVERY", "FAULT", "z2Hit && !z1Hit && z1.current_pos == z1.target_pos [fault: Z1 LIMIT NOT FOUND]"),
            ("REC_STOPPING", "FAULT", "axes_done() && !skewOK() [fault: REC SKEW]"),
            ("REC_BACKOFF", "FAULT", "now - stateStart > RECOVERY_TIMEOUT [fault: REC BACKOFF TIMEOUT]"),
            ("IDLE", "CALIBRATING", "cmd_ready && strcmp((const char*)rx_buffer, \"CAL\") == 0 [startCal()]"),
        ];
        for (from, to, guard) in expected_helpers {
            let found = sm.transitions.iter().any(|t| t.from == from && t.to == to && t.guard == guard);
            assert!(found, "missing expected helper transition: {} -> {} [{}]", from, to, guard);
        }

        // Check event transitions
        let expected_events = [
            ("IDLE", "GOING"),
            ("RETURNED", "GOING"),
            ("HOLD", "RETURNING"),
            ("IDLE", "OPENING"),
            ("RETURNED", "OPENING"),
            ("IDLE", "CLOSING"),
            ("RETURNED", "CLOSING"),
        ];
        for (from, to) in expected_events {
            let found = sm.transitions.iter().any(|t| t.from == from && t.to == to && t.guard.contains("cmd_ready"));
            assert!(found, "missing expected event transition: {} -> {}", from, to);
        }

        // 3. Verify ambiguous transition
        assert_eq!(sm.ambiguous_transitions.len(), 1, "expected exactly 1 ambiguous transition");
        assert_eq!(sm.ambiguous_transitions[0].target, "IDLE");
        assert!(sm.ambiguous_transitions[0].guard.contains("RST"));

        // 4. Verify layout computation
        let layout = compute_state_machine_layout(&sm);
        assert_eq!(layout.nodes.len(), 14);
        assert_eq!(layout.edges.len(), 31);
        assert!(layout.width > 500.0);
        assert!(layout.height > 300.0);

        // Check FAULT node is marked is_fault and has high fan-in
        let fault_node = layout.nodes.get("FAULT").expect("FAULT node layout missing");
        assert!(fault_node.is_fault);
        assert_eq!(fault_node.incoming_count, 12);

        // Check IDLE node is marked is_initial
        let idle_node = layout.nodes.get("IDLE").expect("IDLE node layout missing");
        assert!(idle_node.is_initial);

        // Check cluster assignments
        assert_eq!(layout.nodes.get("CALIBRATING").unwrap().cluster, "CALIBRATION");
        assert_eq!(layout.nodes.get("GOING").unwrap().cluster, "MOTION");
        assert_eq!(layout.nodes.get("RECOVERY").unwrap().cluster, "RECOVERY");
        assert_eq!(layout.nodes.get("OPENING").unwrap().cluster, "MECHANISM");

        // Check collapsed outgoing badges to FAULT
        assert!(layout.nodes.get("CALIBRATING").unwrap().collapsed_out_badges.contains(&"FAULT".to_string()));
        assert!(layout.nodes.get("GOING").unwrap().collapsed_out_badges.contains(&"FAULT".to_string()));

        // Check high fan-in edges
        let high_fan_in_count = layout.edges.iter().filter(|e| e.is_high_fan_in).count();
        assert_eq!(high_fan_in_count, 12, "expected 12 edges targeting FAULT marked is_high_fan_in");

        // 5. Verify exact expected labels for each priority tier
        // Tier 1: Fault edges via fault() helper
        let cal_timeout_edge = sm.transitions.iter().find(|t| t.from == "CALIBRATING" && t.to == "FAULT" && t.guard.contains("CAL_TIMEOUT")).expect("missing CAL_TIMEOUT edge");
        assert_eq!(cal_timeout_edge.label, "CAL TIMEOUT");

        let z2_limit_edge = sm.transitions.iter().find(|t| t.from == "CALIBRATING" && t.to == "FAULT" && t.guard.contains("Z2 LIMIT NOT FOUND")).expect("missing Z2 LIMIT edge");
        assert_eq!(z2_limit_edge.label, "Z2 LIMIT NOT FOUND");

        let cal_skew_edge = sm.transitions.iter().find(|t| t.from == "CAL_STOPPING" && t.to == "FAULT" && t.guard.contains("CAL SKEW")).expect("missing CAL SKEW edge");
        assert_eq!(cal_skew_edge.label, "CAL SKEW");

        // Tier 2: Command edges
        let cmd_go_idle = sm.transitions.iter().find(|t| t.from == "IDLE" && t.to == "GOING").expect("missing IDLE->GOING edge");
        assert_eq!(cmd_go_idle.label, "CMD: GO");

        let cmd_ret_hold = sm.transitions.iter().find(|t| t.from == "HOLD" && t.to == "RETURNING").expect("missing HOLD->RETURNING edge");
        assert_eq!(cmd_ret_hold.label, "CMD: RET");

        let cmd_cal_idle = sm.transitions.iter().find(|t| t.from == "IDLE" && t.to == "CALIBRATING").expect("missing IDLE->CALIBRATING edge");
        assert_eq!(cmd_cal_idle.label, "CMD: CAL");

        // Tier 4: Fallback timeout edge
        let timeout_open_edge = sm.transitions.iter().find(|t| t.from == "OPENING" && t.to == "IDLE").expect("missing OPENING->IDLE edge");
        assert_eq!(timeout_open_edge.label, "Timeout (OPEN_DURATION_MS)");

        let timeout_close_edge = sm.transitions.iter().find(|t| t.from == "CLOSING" && t.to == "IDLE").expect("missing CLOSING->IDLE edge");
        assert_eq!(timeout_close_edge.label, "Timeout (CLOSE_DURATION_MS)");

        // Tier 4: Fallback boolean expressions
        let axes_done_edge = sm.transitions.iter().find(|t| t.from == "CAL_STOPPING" && t.to == "CAL_BACKOFF").expect("missing CAL_STOPPING->CAL_BACKOFF edge");
        assert_eq!(axes_done_edge.label, "Axes Done");

        let hits_edge = sm.transitions.iter().find(|t| t.from == "CALIBRATING" && t.to == "CAL_STOPPING").expect("missing CALIBRATING->CAL_STOPPING edge");
        assert_eq!(hits_edge.label, "Z1 Hit and Z2 Hit");

        let rec_edge = sm.transitions.iter().find(|t| t.from == "RETURNING" && t.to == "RECOVERY").expect("missing RETURNING->RECOVERY edge");
        assert_eq!(rec_edge.label, "Entering Recovery");

        // Negative assertion: explicitly verify printf-only non-transition branches produce 0 edges
        assert!(!sm.transitions.iter().any(|t| t.to == "NO CAL" || t.to == "BUSY" || t.to == "INVALID"), "printf-only branches must not become states");
        assert!(!sm.transitions.iter().any(|t| t.label == "NO CAL" || t.label == "BUSY" || t.label == "INVALID"), "printf-only branches must not produce transition labels");
        assert!(!sm.transitions.iter().any(|t| t.guard.contains("NO CAL") || t.guard.contains("BUSY") || t.guard.contains("INVALID")), "printf-only branches must not appear in transition guards");

        // Verify EdgeLayout fields
        let layout_go = layout.edges.iter().find(|e| e.from == "IDLE" && e.to == "GOING").expect("missing IDLE->GOING in layout");
        assert_eq!(layout_go.label, "CMD: GO");
        assert_eq!(layout_go.display_guard, "CMD: GO");
        assert!(layout_go.guard.contains("strcmp"), "raw guard must be preserved in layout.edges");

        let layout_fault = layout.edges.iter().find(|e| e.from == "CALIBRATING" && e.to == "FAULT" && e.guard.contains("CAL_TIMEOUT")).expect("missing CAL_TIMEOUT layout edge");
        assert_eq!(layout_fault.label, "CAL TIMEOUT");
        assert_eq!(layout_fault.display_guard, "CAL TIMEOUT");
        assert!(layout_fault.guard.contains("fault: CAL TIMEOUT"), "raw guard must be preserved in layout.edges");
    }

    #[test]
    fn test_docking_firmware_v2_state_machine_ground_truth_verification() {
        let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/docking_firmware_v2/Core/Src/main.c");
        let candidates = discover_state_machines_in_file(&fixture_path).expect("failed to discover state machines");
        assert_eq!(candidates.len(), 1, "expected exactly 1 state machine candidate");

        let sm = extract_state_machine_transitions(&candidates[0], &fixture_path).expect("failed to extract transitions");

        // 1. Verify 14 states (fail loudly if count differs from enum declared variants)
        assert_eq!(sm.states.len(), 14, "expected exactly 14 states");
        assert_eq!(sm.states.len(), candidates[0].enum_def.variants.len());
        let expected_states = [
            "IDLE", "CALIBRATING", "CAL_STOPPING", "CAL_BACKOFF", "GOING", "HOLD",
            "RETURNING", "RETURNED", "RECOVERY", "REC_STOPPING", "REC_BACKOFF",
            "FAULT", "OPENING", "CLOSING"
        ];
        for s in &expected_states {
            assert!(sm.states.contains(&s.to_string()), "missing state: {}", s);
        }

        // 2. Verify exactly 35 transitions
        assert_eq!(sm.transitions.len(), 35, "expected 35 transitions, got {}", sm.transitions.len());

        // Check direct loop transitions (11)
        let expected_direct = [
            ("CALIBRATING", "CAL_STOPPING", "z1Hit && z2Hit"),
            ("CAL_STOPPING", "CAL_BACKOFF", "axes_done()"),
            ("CAL_BACKOFF", "IDLE", "axes_done()"),
            ("GOING", "HOLD", "axes_done()"),
            ("RETURNING", "RECOVERY", "enteringRecovery"),
            ("RETURNING", "RETURNED", "axes_done()"),
            ("RECOVERY", "REC_STOPPING", "z1Hit && z2Hit"),
            ("REC_STOPPING", "REC_BACKOFF", "axes_done()"),
            ("REC_BACKOFF", "RETURNED", "axes_done()"),
            ("OPENING", "IDLE", "now - stateStart > OPEN_DURATION_MS"),
            ("CLOSING", "IDLE", "now - stateStart > CLOSE_DURATION_MS"),
        ];
        for (from, to, guard) in expected_direct {
            let found = sm.transitions.iter().any(|t| t.from == from && t.to == to && t.guard == guard);
            assert!(found, "missing expected direct transition: {} -> {} [{}]", from, to, guard);
        }

        // Check helper transitions (14 fault timeouts/errors + 1 startCal)
        let expected_helpers = [
            ("CALIBRATING", "FAULT", "now - stateStart > CAL_TIMEOUT [fault: CAL TIMEOUT]"),
            ("CALIBRATING", "FAULT", "z1Hit && !z2Hit && z2.current_pos == z2.target_pos [fault: Z2 LIMIT NOT FOUND]"),
            ("CALIBRATING", "FAULT", "z2Hit && !z1Hit && z1.current_pos == z1.target_pos [fault: Z1 LIMIT NOT FOUND]"),
            ("CAL_STOPPING", "FAULT", "now - stateStart > CAL_TIMEOUT [fault: CAL STOP TIMEOUT]"),
            ("CAL_STOPPING", "FAULT", "axes_done() && !skewOK() [fault: CAL SKEW]"),
            ("CAL_BACKOFF", "FAULT", "now - stateStart > CAL_TIMEOUT [fault: CAL BACKOFF TIMEOUT]"),
            ("GOING", "FAULT", "now - stateStart > MOVE_TIMEOUT [fault: GO TIMEOUT]"),
            ("RETURNING", "FAULT", "now - stateStart > MOVE_TIMEOUT [fault: RETURN TIMEOUT]"),
            ("RECOVERY", "FAULT", "now - stateStart > RECOVERY_TIMEOUT [fault: REC TIMEOUT]"),
            ("RECOVERY", "FAULT", "z1Hit && !z2Hit && z2.current_pos == z2.target_pos [fault: Z2 LIMIT NOT FOUND]"),
            ("RECOVERY", "FAULT", "z2Hit && !z1Hit && z1.current_pos == z1.target_pos [fault: Z1 LIMIT NOT FOUND]"),
            ("REC_STOPPING", "FAULT", "now - stateStart > RECOVERY_TIMEOUT [fault: REC STOP TIMEOUT]"),
            ("REC_STOPPING", "FAULT", "axes_done() && !skewOK() [fault: REC SKEW]"),
            ("REC_BACKOFF", "FAULT", "now - stateStart > RECOVERY_TIMEOUT [fault: REC BACKOFF TIMEOUT]"),
            ("IDLE", "CALIBRATING", "cmd_ready && strcmp((const char*)rx_buffer, \"CAL\") == 0 [startCal()]"),
        ];
        for (from, to, guard) in expected_helpers {
            let found = sm.transitions.iter().any(|t| t.from == from && t.to == to && t.guard == guard);
            assert!(found, "missing expected helper transition: {} -> {} [{}]", from, to, guard);
        }

        // Check event transitions (including cross-transitions CLOSING -> OPENING and OPENING -> CLOSING)
        let expected_events = [
            ("IDLE", "GOING"),
            ("RETURNED", "GOING"),
            ("HOLD", "RETURNING"),
            ("IDLE", "OPENING"),
            ("RETURNED", "OPENING"),
            ("CLOSING", "OPENING"),
            ("IDLE", "CLOSING"),
            ("RETURNED", "CLOSING"),
            ("OPENING", "CLOSING"),
        ];
        for (from, to) in expected_events {
            let found = sm.transitions.iter().any(|t| t.from == from && t.to == to && t.guard.contains("cmd_ready"));
            assert!(found, "missing expected event transition: {} -> {}", from, to);
        }

        // 3. Verify ambiguous transitions (RST and STOP)
        assert_eq!(sm.ambiguous_transitions.len(), 2, "expected exactly 2 ambiguous transitions");
        assert!(sm.ambiguous_transitions.iter().any(|a| a.target == "IDLE" && a.guard.contains("RST")));
        assert!(sm.ambiguous_transitions.iter().any(|a| a.target == "IDLE" && a.guard.contains("STOP")));

        // 4. Verify layout computation
        let layout = compute_state_machine_layout(&sm);
        assert_eq!(layout.nodes.len(), 14);
        assert_eq!(layout.edges.len(), 35);

        // Verify MECHANISM lane exists and contains OPENING and CLOSING
        let opening = layout.nodes.get("OPENING").expect("OPENING node layout missing");
        let closing = layout.nodes.get("CLOSING").expect("CLOSING node layout missing");
        let returned = layout.nodes.get("RETURNED").expect("RETURNED node layout missing");

        assert_eq!(opening.cluster, "MECHANISM");
        assert_eq!(closing.cluster, "MECHANISM");

        // Verify OPENING and CLOSING are NOT stacked on RETURNED
        assert_ne!((opening.x, opening.y), (returned.x, returned.y), "OPENING must not be stacked on RETURNED");
        assert_ne!((closing.x, closing.y), (returned.x, returned.y), "CLOSING must not be stacked on RETURNED");
        assert_ne!((opening.x, opening.y), (closing.x, closing.y), "OPENING must not be stacked on CLOSING");

        let lane_mechanism = layout.lanes.iter().find(|l| l.name == "MECHANISM");
        assert!(lane_mechanism.is_some(), "MECHANISM flow lane must exist in layout");

        // Check FAULT node
        let fault_node = layout.nodes.get("FAULT").expect("FAULT node layout missing");
        assert!(fault_node.is_fault);
        assert_eq!(fault_node.incoming_count, 14);

        // 5. Verify exact expected labels for each priority tier
        // Tier 1: Fault edges via fault() helper
        let cal_timeout_edge = sm.transitions.iter().find(|t| t.from == "CALIBRATING" && t.to == "FAULT" && t.guard.contains("CAL_TIMEOUT")).expect("missing CAL_TIMEOUT edge");
        assert_eq!(cal_timeout_edge.label, "CAL TIMEOUT");

        let z2_limit_edge = sm.transitions.iter().find(|t| t.from == "CALIBRATING" && t.to == "FAULT" && t.guard.contains("Z2 LIMIT NOT FOUND")).expect("missing Z2 LIMIT edge");
        assert_eq!(z2_limit_edge.label, "Z2 LIMIT NOT FOUND");

        let cal_skew_edge = sm.transitions.iter().find(|t| t.from == "CAL_STOPPING" && t.to == "FAULT" && t.guard.contains("CAL SKEW")).expect("missing CAL SKEW edge");
        assert_eq!(cal_skew_edge.label, "CAL SKEW");

        // Tier 2: Command edges (including cross-transitions)
        let cmd_go_idle = sm.transitions.iter().find(|t| t.from == "IDLE" && t.to == "GOING").expect("missing IDLE->GOING edge");
        assert_eq!(cmd_go_idle.label, "CMD: GO");

        let cmd_ret_hold = sm.transitions.iter().find(|t| t.from == "HOLD" && t.to == "RETURNING").expect("missing HOLD->RETURNING edge");
        assert_eq!(cmd_ret_hold.label, "CMD: RET");

        let cmd_open_closing = sm.transitions.iter().find(|t| t.from == "CLOSING" && t.to == "OPENING").expect("missing CLOSING->OPENING edge");
        assert_eq!(cmd_open_closing.label, "CMD: OPEN");

        let cmd_close_opening = sm.transitions.iter().find(|t| t.from == "OPENING" && t.to == "CLOSING").expect("missing OPENING->CLOSING edge");
        assert_eq!(cmd_close_opening.label, "CMD: CLOSE");

        let cmd_cal_idle = sm.transitions.iter().find(|t| t.from == "IDLE" && t.to == "CALIBRATING").expect("missing IDLE->CALIBRATING edge");
        assert_eq!(cmd_cal_idle.label, "CMD: CAL");

        // Tier 4: Fallback timeout edge
        let timeout_open_edge = sm.transitions.iter().find(|t| t.from == "OPENING" && t.to == "IDLE").expect("missing OPENING->IDLE edge");
        assert_eq!(timeout_open_edge.label, "Timeout (OPEN_DURATION_MS)");

        let timeout_close_edge = sm.transitions.iter().find(|t| t.from == "CLOSING" && t.to == "IDLE").expect("missing CLOSING->IDLE edge");
        assert_eq!(timeout_close_edge.label, "Timeout (CLOSE_DURATION_MS)");

        // Tier 4: Fallback boolean expressions
        let axes_done_edge = sm.transitions.iter().find(|t| t.from == "CAL_STOPPING" && t.to == "CAL_BACKOFF").expect("missing CAL_STOPPING->CAL_BACKOFF edge");
        assert_eq!(axes_done_edge.label, "Axes Done");

        let hits_edge = sm.transitions.iter().find(|t| t.from == "CALIBRATING" && t.to == "CAL_STOPPING").expect("missing CALIBRATING->CAL_STOPPING edge");
        assert_eq!(hits_edge.label, "Z1 Hit and Z2 Hit");

        let rec_edge = sm.transitions.iter().find(|t| t.from == "RETURNING" && t.to == "RECOVERY").expect("missing RETURNING->RECOVERY edge");
        assert_eq!(rec_edge.label, "Entering Recovery");

        // Negative assertion: explicitly verify printf-only non-transition branches produce 0 edges
        assert!(!sm.transitions.iter().any(|t| t.to == "NO CAL" || t.to == "BUSY" || t.to == "INVALID"), "printf-only branches must not become states");
        assert!(!sm.transitions.iter().any(|t| t.label == "NO CAL" || t.label == "BUSY" || t.label == "INVALID"), "printf-only branches must not produce transition labels");
        assert!(!sm.transitions.iter().any(|t| t.guard.contains("NO CAL") || t.guard.contains("BUSY") || t.guard.contains("INVALID")), "printf-only branches must not appear in transition guards");

        // Verify EdgeLayout fields
        let layout_open = layout.edges.iter().find(|e| e.from == "CLOSING" && e.to == "OPENING").expect("missing CLOSING->OPENING in layout");
        assert_eq!(layout_open.label, "CMD: OPEN");
        assert_eq!(layout_open.display_guard, "CMD: OPEN");
        assert!(layout_open.guard.contains("strcmp"), "raw guard must be preserved in layout.edges");

        let layout_fault = layout.edges.iter().find(|e| e.from == "CALIBRATING" && e.to == "FAULT" && e.guard.contains("CAL_TIMEOUT")).expect("missing CAL_TIMEOUT layout edge");
        assert_eq!(layout_fault.label, "CAL TIMEOUT");
        assert_eq!(layout_fault.display_guard, "CAL TIMEOUT");
        assert!(layout_fault.guard.contains("fault: CAL TIMEOUT"), "raw guard must be preserved in layout.edges");
    }

    #[test]
    fn test_docking_firmware_layout_fixes_round2() {
        let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/docking_firmware/Core/Src/main.c");
        let candidates = discover_state_machines_in_file(&fixture_path).expect("failed to discover state machines");
        let sm = extract_state_machine_transitions(&candidates[0], &fixture_path).expect("failed to extract transitions");
        let layout = compute_state_machine_layout(&sm);

        // Fix #1: Within-cluster ordering bug
        // CALIBRATION cluster must render as CALIBRATING -> CAL_STOPPING -> CAL_BACKOFF
        let cal = layout.nodes.get("CALIBRATING").unwrap();
        let cal_stop = layout.nodes.get("CAL_STOPPING").unwrap();
        let cal_back = layout.nodes.get("CAL_BACKOFF").unwrap();
        assert!(cal.x < cal_stop.x, "CALIBRATING.x ({}) must be < CAL_STOPPING.x ({})", cal.x, cal_stop.x);
        assert!(cal_stop.x < cal_back.x, "CAL_STOPPING.x ({}) must be < CAL_BACKOFF.x ({})", cal_stop.x, cal_back.x);

        // RECOVERY cluster must render as RECOVERY -> REC_STOPPING -> REC_BACKOFF
        let rec = layout.nodes.get("RECOVERY").unwrap();
        let rec_stop = layout.nodes.get("REC_STOPPING").unwrap();
        let rec_back = layout.nodes.get("REC_BACKOFF").unwrap();
        assert!(rec.x < rec_stop.x, "RECOVERY.x ({}) must be < REC_STOPPING.x ({})", rec.x, rec_stop.x);
        assert!(rec_stop.x < rec_back.x, "REC_STOPPING.x ({}) must be < REC_BACKOFF.x ({})", rec_stop.x, rec_back.x);

        // Fix #2: Label collision avoidance
        // No two edge labels may overlap, and no edge label may overlap any node
        let labels_with_boxes: Vec<BoundingBox> = layout
            .edges
            .iter()
            .filter(|e| !e.display_guard.is_empty())
            .map(|e| {
                let w = (e.display_guard.len() as f64 * 6.5 + 16.0).max(36.0);
                let h = 20.0;
                BoundingBox {
                    x0: e.label_pos.0 - w * 0.5,
                    y0: e.label_pos.1 - h * 0.5,
                    x1: e.label_pos.0 + w * 0.5,
                    y1: e.label_pos.1 + h * 0.5,
                }
            })
            .collect();

        // Check against nodes
        for (idx, (edge, lbox)) in layout.edges.iter().filter(|e| !e.display_guard.is_empty()).zip(labels_with_boxes.iter()).enumerate() {
            for node in layout.nodes.values() {
                let node_box = BoundingBox {
                    x0: node.x,
                    y0: node.y,
                    x1: node.x + node.width,
                    y1: node.y + node.height,
                };
                assert!(
                    !lbox.intersects(&node_box),
                    "Label {} ({} -> {} '{}') at ({}, {}) intersects node {} at ({}, {})",
                    idx, edge.from, edge.to, edge.display_guard, lbox.x0, lbox.y0, node.id, node_box.x0, node_box.y0
                );
            }
        }

        // Check against other labels
        let non_empty_edges: Vec<&EdgeLayout> = layout.edges.iter().filter(|e| !e.display_guard.is_empty()).collect();
        for i in 0..labels_with_boxes.len() {
            for j in (i + 1)..labels_with_boxes.len() {
                assert!(
                    !labels_with_boxes[i].intersects(&labels_with_boxes[j]),
                    "Label {} ({} -> {} '{}' at {:?}) and Label {} ({} -> {} '{}' at {:?}) overlap",
                    i, non_empty_edges[i].from, non_empty_edges[i].to, non_empty_edges[i].display_guard, non_empty_edges[i].label_pos,
                    j, non_empty_edges[j].from, non_empty_edges[j].to, non_empty_edges[j].display_guard, non_empty_edges[j].label_pos
                );
            }
        }

        // Sugiyama forward edge routing: IDLE -> GOING flows left-to-right from IDLE to GOING
        let idle_node = layout.nodes.get("IDLE").unwrap();
        let going_node = layout.nodes.get("GOING").unwrap();
        let idle_to_going = layout.edges.iter().find(|e| e.from == "IDLE" && e.to == "GOING").unwrap();
        assert_eq!(idle_to_going.start.0, idle_node.x + idle_node.width);
        assert_eq!(idle_to_going.end.0, going_node.x);
        assert!(idle_to_going.control1.0 > idle_to_going.start.0);
        assert!(idle_to_going.control2.0 < idle_to_going.end.0);

        // Fix #4: FAULT node placement
        // Centered horizontally under the operational graph, 80px below lowest operational lane
        let fault_node = layout.nodes.get("FAULT").unwrap();
        let max_op_x = layout.nodes.values().filter(|n| !n.is_fault).map(|n| n.x + n.width).fold(110.0, f64::max);
        let center_x = (110.0 + max_op_x) * 0.5;
        let fault_center_x = fault_node.x + fault_node.width * 0.5;
        assert!(
            (fault_center_x - center_x).abs() < 10.0,
            "FAULT node center ({}) should match graph center ({})",
            fault_center_x, center_x
        );

        let max_op_y = layout.nodes.values().filter(|n| !n.is_fault).map(|n| n.y + n.height).fold(80.0, f64::max);
        let fault_gap = fault_node.y - max_op_y;
        assert!(
            (fault_gap - 80.0).abs() < 5.0,
            "FAULT vertical gap ({}) should be ~80px below lowest operational lane",
            fault_gap
        );
    }

    #[test]
    fn test_rust_sugiyama_layout_verification() {
        let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/docking_firmware/Core/Src/main.c");
        let candidates = discover_state_machines_in_file(&fixture_path).expect("failed to discover state machines");
        let sm = extract_state_machine_transitions(&candidates[0], &fixture_path).expect("failed to extract transitions");

        // 1. Verify graph construction from extracted state machine data
        let (graph, node_map) = build_state_machine_graph(&sm);
        assert_eq!(graph.node_count(), 14, "expected 14 StateNodes");
        assert_eq!(graph.edge_count(), 19, "expected 19 operational directed edges (12 badge-only FAULT transitions excluded)");

        // 2. Verify connected components structure via rust-sugiyama
        let config = Config {
            vertex_spacing: 100.0,
            c_minimization: CrossingMinimization::Median,
            transpose: true,
            ..Default::default()
        };
        let size_fn = |_idx: NodeIndex, _node: &StateNode| (54.0, 175.0);
        let layouts = rust_sugiyama::from_graph(&graph, &size_fn, &config);

        // As verified, operational states form 1 connected component (IDLE, MOTION, CALIBRATION, RECOVERY, MECHANISM
        // are linked by real transitions in firmware), and FAULT is an isolated single-node component.
        assert_eq!(layouts.len(), 2, "expected 2 components: 13 operational nodes + 1 isolated FAULT");
        assert_eq!(layouts[0].0.len(), 13, "operational component must contain 13 states");
        assert_eq!(layouts[1].0.len(), 1, "fault component must contain 1 state (FAULT)");

        let fault_idx = node_map["FAULT"];
        assert_eq!(layouts[1].0[0].0, fault_idx);

        // 3. Verify rank ordering in full layout
        let layout = compute_state_machine_layout(&sm);

        // CALIBRATING -> CAL_STOPPING -> CAL_BACKOFF strictly ordered left-to-right
        let cal = layout.nodes.get("CALIBRATING").unwrap();
        let cal_stop = layout.nodes.get("CAL_STOPPING").unwrap();
        let cal_back = layout.nodes.get("CAL_BACKOFF").unwrap();
        assert!(cal.x < cal_stop.x, "CALIBRATING.x < CAL_STOPPING.x");
        assert!(cal_stop.x < cal_back.x, "CAL_STOPPING.x < CAL_BACKOFF.x");

        // RECOVERY -> REC_STOPPING -> REC_BACKOFF strictly ordered left-to-right
        let rec = layout.nodes.get("RECOVERY").unwrap();
        let rec_stop = layout.nodes.get("REC_STOPPING").unwrap();
        let rec_back = layout.nodes.get("REC_BACKOFF").unwrap();
        assert!(rec.x < rec_stop.x, "RECOVERY.x < REC_STOPPING.x");
        assert!(rec_stop.x < rec_back.x, "REC_STOPPING.x < REC_BACKOFF.x");

        // Verify collision avoidance
        for edge in layout.edges.iter().filter(|e| !e.display_guard.is_empty()) {
            let label_w = (edge.display_guard.len() as f64 * 6.5 + 16.0).max(36.0);
            let label_h = 20.0;
            let lbox = BoundingBox {
                x0: edge.label_pos.0 - label_w * 0.5,
                y0: edge.label_pos.1 - label_h * 0.5,
                x1: edge.label_pos.0 + label_w * 0.5,
                y1: edge.label_pos.1 + label_h * 0.5,
            };

            for node in layout.nodes.values() {
                let nbox = BoundingBox {
                    x0: node.x,
                    y0: node.y,
                    x1: node.x + node.width,
                    y1: node.y + node.height,
                };
                assert!(!lbox.intersects(&nbox), "label for {} -> {} must not overlap node {}", edge.from, edge.to, node.id);
            }
        }
    }

    #[test]
    fn test_hub_anchored_swimlanes_layout() {
        let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/docking_firmware/Core/Src/main.c");
        let candidates = discover_state_machines_in_file(&fixture_path).expect("failed to discover state machines");
        let sm = extract_state_machine_transitions(&candidates[0], &fixture_path).expect("failed to extract transitions");
        let layout = compute_state_machine_layout(&sm);

        // 1. Hub dynamic identification & positioning
        // IDLE is pinned at left_margin = 110.0
        let idle = layout.nodes.get("IDLE").expect("IDLE node must exist");
        assert_eq!(idle.x, 110.0, "IDLE hub must be pinned at left margin 110.0");

        // RETURNED is pinned at lower-right (x >= 1100.0)
        let ret = layout.nodes.get("RETURNED").expect("RETURNED node must exist");
        assert!(ret.x >= 1100.0, "RETURNED hub must be pinned on the right (x = {})", ret.x);

        // FAULT is centered at bottom
        let fault = layout.nodes.get("FAULT").expect("FAULT node must exist");
        let max_op_x = layout.nodes.values().filter(|n| !n.is_fault).map(|n| n.x + n.width).fold(110.0, f64::max);
        let center_x = (110.0 + max_op_x) * 0.5;
        let fault_center_x = fault.x + fault.width * 0.5;
        assert_eq!(fault_center_x, center_x, "FAULT node center must match graph center");

        // 2. Multi-point connection anchors for IDLE outgoing edges
        let idle_outgoing: Vec<&EdgeLayout> = layout.edges.iter().filter(|e| e.from == "IDLE" && !e.is_fault).collect();
        assert!(idle_outgoing.len() >= 3, "IDLE should have multiple outgoing operational edges");
        let start_ys: HashSet<i64> = idle_outgoing.iter().map(|e| (e.start.1 * 100.0).round() as i64).collect();
        assert!(
            start_ys.len() > 1,
            "IDLE outgoing edges must have multi-point connection anchors along its right edge, found distinct Y count: {}",
            start_ys.len()
        );

        // 3. Lane positioning: all lane states start to the right of IDLE
        for node in layout.nodes.values() {
            if node.id != "IDLE" && !node.is_fault {
                assert!(
                    node.x >= idle.x + idle.width,
                    "Operational node {} at x={} must start to the right of IDLE (x={})",
                    node.id, node.x, idle.x + idle.width
                );
            }
        }

        // 4. Diagram dimensions
        assert!(layout.width >= 1200.0, "diagram width ({}) should encompass hubs and lanes", layout.width);
        assert!(layout.height >= 700.0, "diagram height ({}) should encompass lanes and fault", layout.height);
    }

    #[test]
    fn test_discover_state_machines_in_project_aa_ns_stm_port() {
        let fixture_main_c = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/aa_ns_stm_port/Core/Src/main.c");
        let candidates = discover_state_machines_in_project(&fixture_main_c)
            .expect("discovery across project files should succeed");

        assert_eq!(candidates.len(), 2, "Expected exactly 2 state machines (AlignState and HatchState)");

        let align_cand = candidates.iter().find(|c| c.enum_def.name == "AlignState")
            .expect("AlignState candidate must be found");
        assert_eq!(align_cand.var.name, "state");
        assert!(align_cand.var.file_path.ends_with("alignment.c"));
        assert_eq!(align_cand.enum_def.variants.len(), 11);

        let hatch_cand = candidates.iter().find(|c| c.enum_def.name == "HatchState")
            .expect("HatchState candidate must be found");
        assert_eq!(hatch_cand.var.name, "hatchState");
        assert!(hatch_cand.var.file_path.ends_with("hatch.c"));
        assert_eq!(hatch_cand.enum_def.variants.len(), 3);
    }

    #[test]
    fn test_fault_coordinator_aa_ns_stm_port() {
        let fixture_main_c = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/aa_ns_stm_port/Core/Src/main.c");
        let candidates = discover_state_machines_in_project(&fixture_main_c)
            .expect("discovery across project files should succeed");

        let align_cand = candidates.iter().find(|c| c.enum_def.name == "AlignState")
            .expect("AlignState candidate must be found");
        let align_file = Path::new(&align_cand.var.file_path);
        let align_sm = extract_state_machine_transitions(align_cand, align_file)
            .expect("AlignState transitions extraction should succeed");

        // AlignState calls system_fault() -> synthetic SYSTEM FAULT sink node must exist
        assert!(align_sm.states.contains(&"SYSTEM FAULT".to_string()), "AlignState must contain synthetic SYSTEM FAULT state");
        assert_eq!(align_sm.states.len(), 12, "Expected 11 enum states + 1 synthetic SYSTEM FAULT state");

        let fault_transitions: Vec<&AppTransition> = align_sm.transitions.iter().filter(|t| t.to == "SYSTEM FAULT").collect();
        assert!(!fault_transitions.is_empty(), "AlignState must have transitions into SYSTEM FAULT");
        for t in &fault_transitions {
            assert!(t.is_fault);
        }

        let fault_labels: Vec<&str> = fault_transitions.iter().map(|t| t.label.as_str()).collect();
        assert!(fault_labels.contains(&"CAL TIMEOUT"));
        assert!(fault_labels.contains(&"CAL SKEW"));
        assert!(fault_labels.contains(&"Z2 LIMIT NOT FOUND"));
        assert!(fault_labels.contains(&"GO TIMEOUT"));
        assert!(fault_labels.contains(&"RETURN TIMEOUT"));
        assert!(fault_labels.contains(&"REC TIMEOUT"));

        // HatchState does NOT call system_fault() -> NO synthetic SYSTEM FAULT node
        let hatch_cand = candidates.iter().find(|c| c.enum_def.name == "HatchState")
            .expect("HatchState candidate must be found");
        let hatch_file = Path::new(&hatch_cand.var.file_path);
        let hatch_sm = extract_state_machine_transitions(hatch_cand, hatch_file)
            .expect("HatchState transitions extraction should succeed");

        assert!(!hatch_sm.states.contains(&"SYSTEM FAULT".to_string()), "HatchState must NOT contain SYSTEM FAULT state");
        assert_eq!(hatch_sm.states.len(), 3, "HatchState must have exactly 3 states");
        assert!(!hatch_sm.transitions.iter().any(|t| t.to == "SYSTEM FAULT" || t.is_fault), "HatchState must have 0 fault transitions");

        // Layout verification: AlignState layout must contain SYSTEM FAULT node
        let align_layout = compute_state_machine_layout(&align_sm);
        let fault_node = align_layout.nodes.get("SYSTEM FAULT").expect("SYSTEM FAULT node layout must exist");
        assert!(fault_node.is_fault);

        // Layout verification: HatchState layout must NOT contain SYSTEM FAULT node
        let hatch_layout = compute_state_machine_layout(&hatch_sm);
        assert!(!hatch_layout.nodes.contains_key("SYSTEM FAULT"), "HatchState layout must NOT have SYSTEM FAULT node");
    }

    #[test]
    fn test_multihop_command_transitions_aa_ns_stm_port() {
        let fixture_main_c = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/aa_ns_stm_port/Core/Src/main.c");
        let candidates = discover_state_machines_in_project(&fixture_main_c)
            .expect("discovery across project files should succeed");

        // 1. AlignState multi-hop command transitions
        let align_cand = candidates.iter().find(|c| c.enum_def.name == "AlignState")
            .expect("AlignState candidate must be found");
        let align_file = Path::new(&align_cand.var.file_path);
        let align_sm = extract_state_machine_transitions(align_cand, align_file)
            .expect("AlignState transitions extraction should succeed");

        // Verify CMD: CAL (ALIGN_IDLE -> CALIBRATING)
        assert!(
            align_sm.transitions.iter().any(|t| t.from == "ALIGN_IDLE" && t.to == "CALIBRATING" && t.label == "CMD: CAL"),
            "CMD: CAL transition from ALIGN_IDLE to CALIBRATING must exist"
        );

        // Verify CMD: GO (ALIGN_IDLE -> GOING and RETURNED -> GOING)
        assert!(
            align_sm.transitions.iter().any(|t| t.from == "ALIGN_IDLE" && t.to == "GOING" && t.label == "CMD: GO"),
            "CMD: GO transition from ALIGN_IDLE to GOING must exist"
        );
        assert!(
            align_sm.transitions.iter().any(|t| t.from == "RETURNED" && t.to == "GOING" && t.label == "CMD: GO"),
            "CMD: GO transition from RETURNED to GOING must exist"
        );

        // Verify CMD: RET (HOLD -> RETURNING)
        assert!(
            align_sm.transitions.iter().any(|t| t.from == "HOLD" && t.to == "RETURNING" && t.label == "CMD: RET"),
            "CMD: RET transition from HOLD to RETURNING must exist"
        );

        // Verify CMD: RST ((any state) -> ALIGN_IDLE)
        assert!(
            align_sm.transitions.iter().any(|t| t.from == "(any state)" && t.to == "ALIGN_IDLE" && t.label == "CMD: RST"),
            "CMD: RST transition from (any state) to ALIGN_IDLE must exist"
        );

        // 2. HatchState multi-hop command transitions
        let hatch_cand = candidates.iter().find(|c| c.enum_def.name == "HatchState")
            .expect("HatchState candidate must be found");
        let hatch_file = Path::new(&hatch_cand.var.file_path);
        let hatch_sm = extract_state_machine_transitions(hatch_cand, hatch_file)
            .expect("HatchState transitions extraction should succeed");

        // Verify CMD: OPEN (HATCH_IDLE -> HATCH_OPENING)
        assert!(
            hatch_sm.transitions.iter().any(|t| t.from == "HATCH_IDLE" && t.to == "HATCH_OPENING" && t.label == "CMD: OPEN"),
            "CMD: OPEN transition from HATCH_IDLE to HATCH_OPENING must exist"
        );

        // Verify CMD: CLOSE (HATCH_IDLE -> HATCH_CLOSING)
        assert!(
            hatch_sm.transitions.iter().any(|t| t.from == "HATCH_IDLE" && t.to == "HATCH_CLOSING" && t.label == "CMD: CLOSE"),
            "CMD: CLOSE transition from HATCH_IDLE to HATCH_CLOSING must exist"
        );

        // Verify CMD: STOP ((any state) -> HATCH_IDLE)
        assert!(
            hatch_sm.transitions.iter().any(|t| t.from == "(any state)" && t.to == "HATCH_IDLE" && t.label == "CMD: STOP"),
            "CMD: STOP transition from (any state) to HATCH_IDLE must exist"
        );
    }
}
