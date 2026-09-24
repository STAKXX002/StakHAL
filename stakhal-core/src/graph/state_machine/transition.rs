use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use tree_sitter::{Node, Parser};

use crate::source::marker_scan::ScanError;
use super::cross_file::resolve_cross_file_command_transitions;
use super::discovery::extract_function_name;
use super::labels::{
    clean_guard_text, clean_printf_literal, differs_meaningfully, extract_command_string,
    extract_first_argument_info, find_following_printf, node_text, prettify_guard,
    strip_variant_prefix,
};
use super::model::{
    AmbiguousTransition, AppStateMachine, AppTransition, StateMachineCandidate, TransitionType,
};

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

#[derive(Debug, Clone)]
pub(crate) struct HelperFunctionInfo {
    pub(crate) name: String,
    pub(crate) target_states: Vec<String>,
}

pub(crate) fn collect_helper_functions(
    root: Node,
    source_bytes: &[u8],
    var_name: &str,
    known_variants: &HashSet<String>,
) -> HashMap<String, HelperFunctionInfo> {
    let mut helpers = HashMap::new();
    let mut fn_nodes = Vec::new();
    collect_functions(root, &mut fn_nodes);

    for fn_node in &fn_nodes {
        if let Some(fn_name) = extract_function_name(*fn_node, source_bytes) {
            if fn_name == "main" || has_switch_on_var(*fn_node, source_bytes, var_name) {
                continue;
            }
            let mut targets = Vec::new();
            collect_assignments_to_var(*fn_node, source_bytes, var_name, known_variants, &mut targets);
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

    // Delegating helper propagation: if function A has no direct assignments and calls helper B,
    // function A acts as a delegating helper and inherits B's target states.
    let mut changed = true;
    while changed {
        changed = false;
        for fn_node in &fn_nodes {
            if let Some(fn_name) = extract_function_name(*fn_node, source_bytes) {
                if fn_name == "main" || has_switch_on_var(*fn_node, source_bytes, var_name) {
                    continue;
                }
                if let Some(existing) = helpers.get(&fn_name) {
                    if !existing.target_states.is_empty() {
                        continue;
                    }
                }
                let mut called_helpers = Vec::new();
                collect_called_helpers(*fn_node, source_bytes, &helpers, &mut called_helpers);
                for h_name in called_helpers {
                    if let Some(h_info) = helpers.get(&h_name).cloned() {
                        let entry = helpers.entry(fn_name.clone()).or_insert_with(|| HelperFunctionInfo {
                            name: fn_name.clone(),
                            target_states: Vec::new(),
                        });
                        for t in h_info.target_states {
                            if !entry.target_states.contains(&t) {
                                entry.target_states.push(t);
                                changed = true;
                            }
                        }
                    }
                }
            }
        }
    }

    helpers
}

fn collect_called_helpers(
    node: Node,
    source_bytes: &[u8],
    helpers: &HashMap<String, HelperFunctionInfo>,
    out: &mut Vec<String>,
) {
    if node.kind() == "call_expression" {
        if let Some(fn_child) = node.child_by_field_name("function") {
            let fn_name = node_text(fn_child, source_bytes);
            if helpers.contains_key(&fn_name) && !out.contains(&fn_name) {
                out.push(fn_name);
            }
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_called_helpers(child, source_bytes, helpers, out);
    }
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

pub(crate) fn collect_functions<'a>(node: Node<'a>, out: &mut Vec<Node<'a>>) {
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
            } else if let Some(right_ident) = extract_target_ident(right, source_bytes) {
                if right_ident != var_name {
                    resolve_variable_assignment(
                        node,
                        ctx,
                        source_bytes,
                        var_name,
                        &right_ident,
                        known_variants,
                        direct_transitions,
                    );
                }
            }
        }
    }
}

fn resolve_variable_assignment(
    node: Node,
    ctx: &ASTContext,
    source_bytes: &[u8],
    _var_name: &str,
    staging_var: &str,
    known_variants: &HashSet<String>,
    direct_transitions: &mut Vec<AppTransition>,
) {
    let active_states = match &ctx.active_states {
        Some(st) if !st.is_empty() => st.clone(),
        _ => return,
    };

    let mut resolved_branches: Vec<(String, String, String)> = Vec::new();

    let mut curr = node.parent();
    while let Some(parent_node) = curr {
        if parent_node.kind() == "compound_statement" || parent_node.kind() == "case_statement" {
            find_staging_branches(parent_node, source_bytes, staging_var, known_variants, &active_states, &mut resolved_branches);
            if !resolved_branches.is_empty() {
                break;
            }
        }
        if parent_node.kind() == "function_definition" {
            break;
        }
        curr = parent_node.parent();
    }

    let line = node.start_position().row + 1;
    for from_state in &active_states {
        for (target, guard, label) in &resolved_branches {
            if !direct_transitions.iter().any(|t| &t.from == from_state && &t.to == target && &t.label == label) {
                direct_transitions.push(AppTransition {
                    from: from_state.clone(),
                    to: target.clone(),
                    guard: guard.clone(),
                    label: label.clone(),
                    is_fault: false,
                    transition_type: TransitionType::Direct,
                    line,
                });
            }
        }
    }
}

fn find_staging_branches(
    block_node: Node,
    source_bytes: &[u8],
    staging_var: &str,
    known_variants: &HashSet<String>,
    active_states: &[String],
    out: &mut Vec<(String, String, String)>,
) {
    let mut cursor = block_node.walk();
    for child in block_node.children(&mut cursor) {
        if child.kind() == "if_statement" {
            if let Some(cond) = child.child_by_field_name("condition") {
                if let Some(tested_variant) = extract_tested_variant(cond, source_bytes, staging_var, known_variants) {
                    let stripped_tested = strip_variant_prefix(&tested_variant).unwrap_or_else(|| tested_variant.clone());
                    let tested_guard = format!("{} == {}", staging_var, tested_variant);
                    let tested_label = format!("{} is {}", staging_var, stripped_tested);
                    out.push((tested_variant.clone(), tested_guard, tested_label));

                    if let Some(alt) = child.child_by_field_name("alternative") {
                        let mut chained_if = None;
                        if alt.kind() == "if_statement" {
                            chained_if = Some(alt);
                        } else {
                            let mut c = alt.walk();
                            for ch in alt.children(&mut c) {
                                if ch.kind() == "if_statement" {
                                    chained_if = Some(ch);
                                    break;
                                }
                            }
                        }

                        if let Some(else_if_node) = chained_if {
                            find_staging_branches(else_if_node, source_bytes, staging_var, known_variants, active_states, out);
                        } else {
                            let mut remaining: Vec<String> = known_variants
                                .iter()
                                .filter(|v| *v != &tested_variant && !active_states.contains(v) && !is_initial_variant(v) && !is_fault_state(v))
                                .cloned()
                                .collect();
                            remaining.sort();
                            if let Some(other_var) = remaining.first() {
                                let stripped_other = strip_variant_prefix(other_var).unwrap_or_else(|| other_var.clone());
                                let other_guard = format!("{} == {}", staging_var, other_var);
                                let other_label = format!("{} is {}", staging_var, stripped_other);
                                out.push((other_var.clone(), other_guard, other_label));
                            }
                        }
                    }
                }
            }
        }
    }
}

fn extract_tested_variant(
    cond: Node,
    source_bytes: &[u8],
    staging_var: &str,
    known_variants: &HashSet<String>,
) -> Option<String> {
    match cond.kind() {
        "parenthesized_expression" => {
            let mut cursor = cond.walk();
            for child in cond.children(&mut cursor) {
                if child.kind() != "(" && child.kind() != ")" && child.kind() != "comment" {
                    if let Some(v) = extract_tested_variant(child, source_bytes, staging_var, known_variants) {
                        return Some(v);
                    }
                }
            }
            None
        }
        "binary_expression" => {
            let op = cond.child_by_field_name("operator").map(|n| node_text(n, source_bytes));
            let left = cond.child_by_field_name("left");
            let right = cond.child_by_field_name("right");
            if let (Some(op), Some(left), Some(right)) = (op, left, right) {
                if op == "==" {
                    let left_txt = extract_target_ident(left, source_bytes).unwrap_or_default();
                    let right_variant = extract_variant_ident(right, source_bytes, known_variants);
                    if left_txt == staging_var && right_variant.is_some() {
                        return right_variant;
                    }
                    let right_txt = extract_target_ident(right, source_bytes).unwrap_or_default();
                    let left_variant = extract_variant_ident(left, source_bytes, known_variants);
                    if right_txt == staging_var && left_variant.is_some() {
                        return left_variant;
                    }
                }
            }
            None
        }
        _ => None,
    }
}

fn is_initial_variant(v: &str) -> bool {
    let lower = v.to_lowercase();
    lower.contains("idle") || lower.contains("init") || lower.contains("boot") || lower.contains("reset")
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

pub(crate) fn extract_target_ident(node: Node, source_bytes: &[u8]) -> Option<String> {
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

pub(crate) fn extract_variant_ident(
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
