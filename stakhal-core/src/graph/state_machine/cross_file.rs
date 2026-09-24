use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use tree_sitter::{Node, Parser};

use super::discovery::extract_function_name;
use super::labels::{clean_guard_text, node_text};
use super::model::{AmbiguousTransition, AppTransition, StateMachineCandidate, TransitionType};
use super::{collect_functions, HelperFunctionInfo};

pub(crate) fn resolve_cross_file_command_transitions(
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
