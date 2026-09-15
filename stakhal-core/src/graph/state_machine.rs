use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use tree_sitter::{Node, Parser};

use crate::source::marker_scan::ScanError;

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
    pub is_fault: bool,
    pub transition_type: TransitionType,
    pub line: usize,
}

impl AppTransition {
    pub fn display_guard(&self, max_len: usize) -> String {
        if self.guard.len() <= max_len {
            self.guard.clone()
        } else {
            format!("{}...", &self.guard[..max_len.saturating_sub(3)])
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
pub struct StateMachineLayout {
    pub nodes: HashMap<String, NodeLayout>,
    pub edges: Vec<EdgeLayout>,
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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdgeLayout {
    pub from: String,
    pub to: String,
    pub guard: String,
    pub display_guard: String,
    pub is_fault: bool,
    pub start: (f64, f64),
    pub end: (f64, f64),
    pub control1: (f64, f64),
    pub control2: (f64, f64),
    pub label_pos: (f64, f64),
    pub transition_type: TransitionType,
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
            let ctx = ASTContext::default();
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

    Ok(AppStateMachine {
        id: candidate.id.clone(),
        display_name: candidate.display_name.clone(),
        enum_def: candidate.enum_def.clone(),
        var: candidate.var.clone(),
        states: candidate.enum_def.variants.clone(),
        transitions,
        ambiguous_transitions,
    })
}

/// Compute layered node-link graph layout for a state machine.
pub fn compute_state_machine_layout(sm: &AppStateMachine) -> StateMachineLayout {
    let node_width = 160.0;
    let node_height = 48.0;
    let h_gap = 100.0;
    let v_gap = 48.0;

    let initial_name = sm.var.initial_value.as_deref().unwrap_or("IDLE");

    // Separate fault nodes and normal nodes
    let mut normal_nodes = Vec::new();
    let mut fault_nodes = Vec::new();

    for st in &sm.states {
        if is_fault_state(st) {
            fault_nodes.push(st.clone());
        } else {
            normal_nodes.push(st.clone());
        }
    }

    // Build forward adjacency list for normal transitions
    let mut adj: HashMap<String, Vec<String>> = HashMap::new();
    for t in &sm.transitions {
        if t.from != "(any state)" && !is_fault_state(&t.from) && !is_fault_state(&t.to) {
            adj.entry(t.from.clone()).or_default().push(t.to.clone());
        }
    }

    // BFS from initial_name
    let mut dist: HashMap<String, usize> = HashMap::new();
    let mut queue = std::collections::VecDeque::new();
    if normal_nodes.contains(&initial_name.to_string()) {
        dist.insert(initial_name.to_string(), 0);
        queue.push_back(initial_name.to_string());
    }

    while let Some(curr) = queue.pop_front() {
        let d = dist[&curr];
        if let Some(neighbors) = adj.get(&curr) {
            for n in neighbors {
                if !dist.contains_key(n) {
                    dist.insert(n.clone(), d + 1);
                    queue.push_back(n.clone());
                }
            }
        }
    }

    // Group normal nodes into layers
    let max_dist = dist.values().copied().max().unwrap_or(0);
    let mut layers: Vec<Vec<String>> = vec![Vec::new(); max_dist + 2];

    for n in &normal_nodes {
        if let Some(&d) = dist.get(n) {
            layers[d].push(n.clone());
        } else {
            // Unreached node placed in fallback layer
            layers[max_dist + 1].push(n.clone());
        }
    }

    // Filter out empty layers
    let active_layers: Vec<Vec<String>> = layers.into_iter().filter(|l| !l.is_empty()).collect();

    let mut nodes_layout = HashMap::new();
    let left_margin = 60.0;
    let top_margin = 70.0;

    let mut max_x = left_margin;
    let mut max_y = top_margin;

    for (layer_idx, layer_nodes) in active_layers.iter().enumerate() {
        let x = left_margin + layer_idx as f64 * (node_width + h_gap);
        for (row_idx, node_id) in layer_nodes.iter().enumerate() {
            let y = top_margin + row_idx as f64 * (node_height + v_gap);
            let is_initial = node_id == initial_name;

            nodes_layout.insert(
                node_id.clone(),
                NodeLayout {
                    id: node_id.clone(),
                    label: node_id.clone(),
                    x,
                    y,
                    width: node_width,
                    height: node_height,
                    is_fault: false,
                    is_initial,
                },
            );

            if x + node_width > max_x {
                max_x = x + node_width;
            }
            if y + node_height > max_y {
                max_y = y + node_height;
            }
        }
    }

    // Position fault nodes in bottom area
    let fault_y = max_y + 80.0;
    let fault_start_x = left_margin + (max_x - left_margin - fault_nodes.len() as f64 * (node_width + 40.0)).max(0.0) * 0.5;

    for (idx, fault_id) in fault_nodes.iter().enumerate() {
        let x = fault_start_x + idx as f64 * (node_width + 40.0);
        nodes_layout.insert(
            fault_id.clone(),
            NodeLayout {
                id: fault_id.clone(),
                label: fault_id.clone(),
                x,
                y: fault_y,
                width: node_width,
                height: node_height,
                is_fault: true,
                is_initial: false,
            },
        );
        if x + node_width > max_x {
            max_x = x + node_width;
        }
        if fault_y + node_height > max_y {
            max_y = fault_y + node_height;
        }
    }

    // Compute edge layouts
    let mut edges_layout = Vec::new();
    for t in &sm.transitions {
        let from_layout = nodes_layout.get(&t.from);
        let to_layout = nodes_layout.get(&t.to);

        if let (Some(from), Some(to)) = (from_layout, to_layout) {
            let display_guard = t.display_guard(35);
            let (start, end, control1, control2, label_pos) = if to.is_fault {
                // Route down toward fault
                let start = (from.x + from.width * 0.5, from.y + from.height);
                let end = (to.x + to.width * 0.5, to.y);
                let dy = (end.1 - start.1).max(20.0);
                let control1 = (start.0, start.1 + dy * 0.4);
                let control2 = (end.0, end.1 - dy * 0.4);
                let label_pos = ((start.0 + end.0) * 0.5, (start.1 + end.1) * 0.5);
                (start, end, control1, control2, label_pos)
            } else if from.x < to.x {
                // Forward edge
                let start = (from.x + from.width, from.y + from.height * 0.5);
                let end = (to.x, to.y + to.height * 0.5);
                let dx = (end.0 - start.0).max(20.0);
                let control1 = (start.0 + dx * 0.4, start.1);
                let control2 = (end.0 - dx * 0.4, end.1);
                let label_pos = ((start.0 + end.0) * 0.5, (start.1 + end.1) * 0.5);
                (start, end, control1, control2, label_pos)
            } else if from.x == to.x {
                // Same column (vertical)
                let start = (from.x + from.width * 0.5, if from.y < to.y { from.y + from.height } else { from.y });
                let end = (to.x + to.width * 0.5, if from.y < to.y { to.y } else { to.y + to.height });
                let bend_x = from.x + from.width + 30.0;
                let control1 = (bend_x, start.1);
                let control2 = (bend_x, end.1);
                let label_pos = (bend_x, (start.1 + end.1) * 0.5);
                (start, end, control1, control2, label_pos)
            } else {
                // Backward edge (e.g. loops back to IDLE/RETURNED)
                let start = (from.x + from.width * 0.5, from.y);
                let end = (to.x + to.width * 0.5, to.y);
                let curve_y = (from.y.min(to.y) - 40.0 - (from.x - to.x).abs() * 0.05).max(20.0);
                let control1 = (from.x + from.width * 0.5, curve_y);
                let control2 = (to.x + to.width * 0.5, curve_y);
                let label_pos = ((start.0 + end.0) * 0.5, curve_y);
                (start, end, control1, control2, label_pos)
            };

            edges_layout.push(EdgeLayout {
                from: t.from.clone(),
                to: t.to.clone(),
                guard: t.guard.clone(),
                display_guard,
                is_fault: t.is_fault,
                start,
                end,
                control1,
                control2,
                label_pos,
                transition_type: t.transition_type.clone(),
            });
        }
    }

    StateMachineLayout {
        nodes: nodes_layout,
        edges: edges_layout,
        width: max_x + 80.0,
        height: max_y + 80.0,
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
            if fn_name == "main" {
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
            let arg_text = extract_first_argument_text(node, source_bytes);

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
                if let Some(ref states) = ctx.active_states {
                    if states.is_empty() {
                        transitions.push(AppTransition {
                            from: "(any state)".to_string(),
                            to: target.clone(),
                            guard: full_guard.clone(),
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
                        is_fault,
                        transition_type: TransitionType::IndirectHelper {
                            helper_name: helper.name.clone(),
                            argument: arg_text.clone(),
                        },
                        line,
                    });
                }
            }
        }
    }
}

fn extract_first_argument_text(call_node: Node, source_bytes: &[u8]) -> Option<String> {
    if let Some(arg_list) = call_node.child_by_field_name("arguments") {
        let mut cursor = arg_list.walk();
        for child in arg_list.children(&mut cursor) {
            if child.kind() != "(" && child.kind() != ")" && child.kind() != "," && child.kind() != "comment" {
                let mut text = node_text(child, source_bytes);
                if text.starts_with('"') && text.ends_with('"') && text.len() >= 2 {
                    text = text[1..text.len() - 1].to_string();
                }
                return Some(text);
            }
        }
    }
    None
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

fn is_fault_state(state: &str) -> bool {
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

        // Check FAULT node is marked is_fault
        let fault_node = layout.nodes.get("FAULT").expect("FAULT node layout missing");
        assert!(fault_node.is_fault);

        // Check IDLE node is marked is_initial
        let idle_node = layout.nodes.get("IDLE").expect("IDLE node layout missing");
        assert!(idle_node.is_initial);
    }
}
