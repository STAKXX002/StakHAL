use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use tree_sitter::{Node, Parser};

use crate::source::marker_scan::ScanError;
use super::labels::{clean_guard_text, node_text};
use super::model::{EnumDefinition, StateMachineCandidate, TrackedVariable, VariableScope};
use super::{collect_helper_functions, extract_target_ident, extract_variant_ident, HelperFunctionInfo};

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

    // 3. Match each variable to its enum definition (qualifying only variables with guarded dispatch)
    let mut candidates = Vec::new();
    for var in variables {
        if let Some(enum_def) = enum_map.get(&var.enum_type) {
            let known_variants: HashSet<String> = enum_def.variants.iter().cloned().collect();
            let helpers = collect_helper_functions(root, source_bytes, &var.name, &known_variants);
            if has_guarded_dispatch_assigning_var(root, source_bytes, &var.name, enum_def, &helpers) {
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
                let known_variants: HashSet<String> = enum_def.variants.iter().cloned().collect();
                let mut qualifies = false;
                for c_path in &c_files {
                    if let Ok(source) = fs::read_to_string(c_path) {
                        if let Some(tree) = parser.parse(&source, None) {
                            let root = tree.root_node();
                            let s_bytes = source.as_bytes();
                            let helpers = collect_helper_functions(root, s_bytes, &var.name, &known_variants);
                            if has_guarded_dispatch_assigning_var(root, s_bytes, &var.name, enum_def, &helpers) {
                                qualifies = true;
                                break;
                            }
                        }
                    }
                }

                if qualifies {
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
    }

    candidates.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    Ok(candidates)
}

/// Determine whether a candidate variable qualifies as its own state machine.
/// A variable only qualifies if it is itself the subject of guarded dispatch logic
/// (a switch(var) or an if/else-if chain comparing var against its own enum constants)
/// where at least one branch assigns a new value back to var.
pub(crate) fn has_guarded_dispatch_assigning_var(
    node: Node,
    source_bytes: &[u8],
    var_name: &str,
    enum_def: &EnumDefinition,
    helpers: &HashMap<String, HelperFunctionInfo>,
) -> bool {
    let known_variants: HashSet<String> = enum_def.variants.iter().cloned().collect();
    check_guarded_dispatch_recursive(node, source_bytes, var_name, &known_variants, helpers)
}

pub(crate) fn check_guarded_dispatch_recursive(
    node: Node,
    source_bytes: &[u8],
    var_name: &str,
    known_variants: &HashSet<String>,
    helpers: &HashMap<String, HelperFunctionInfo>,
) -> bool {
    match node.kind() {
        "switch_statement" => {
            if let Some(cond) = node.child_by_field_name("condition") {
                let cond_text = clean_guard_text(&node_text(cond, source_bytes));
                if cond_text == var_name {
                    if let Some(body) = node.child_by_field_name("body") {
                        if block_assigns_var(body, source_bytes, var_name, helpers) {
                            return true;
                        }
                    }
                }
            }
        }
        "if_statement" => {
            if let Some(cond) = node.child_by_field_name("condition") {
                if condition_compares_var_to_variants(cond, source_bytes, var_name, known_variants) {
                    if let Some(consequence) = node.child_by_field_name("consequence") {
                        if block_assigns_var(consequence, source_bytes, var_name, helpers) {
                            return true;
                        }
                    }
                    if let Some(alternative) = node.child_by_field_name("alternative") {
                        if block_assigns_var(alternative, source_bytes, var_name, helpers) {
                            return true;
                        }
                    }
                }
            }
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if check_guarded_dispatch_recursive(child, source_bytes, var_name, known_variants, helpers) {
            return true;
        }
    }
    false
}

pub(crate) fn block_assigns_var(
    node: Node,
    source_bytes: &[u8],
    var_name: &str,
    helpers: &HashMap<String, HelperFunctionInfo>,
) -> bool {
    if node.kind() == "assignment_expression" {
        if let Some(left) = node.child_by_field_name("left") {
            if extract_target_ident(left, source_bytes).as_deref() == Some(var_name) {
                return true;
            }
        }
    } else if node.kind() == "call_expression" {
        if let Some(fn_child) = node.child_by_field_name("function") {
            let fn_name = node_text(fn_child, source_bytes);
            if helpers.contains_key(&fn_name) {
                return true;
            }
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if block_assigns_var(child, source_bytes, var_name, helpers) {
            return true;
        }
    }
    false
}

pub(crate) fn condition_compares_var_to_variants(
    node: Node,
    source_bytes: &[u8],
    var_name: &str,
    known_variants: &HashSet<String>,
) -> bool {
    match node.kind() {
        "parenthesized_expression" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() != "(" && child.kind() != ")" && child.kind() != "comment" {
                    if condition_compares_var_to_variants(child, source_bytes, var_name, known_variants) {
                        return true;
                    }
                }
            }
            false
        }
        "binary_expression" => {
            let op = node.child_by_field_name("operator").map(|n| node_text(n, source_bytes));
            let left = node.child_by_field_name("left");
            let right = node.child_by_field_name("right");
            if let (Some(op), Some(left), Some(right)) = (op, left, right) {
                if op == "==" || op == "!=" {
                    let left_txt = extract_target_ident(left, source_bytes).unwrap_or_default();
                    let right_variant = extract_variant_ident(right, source_bytes, known_variants);
                    if left_txt == var_name && right_variant.is_some() {
                        return true;
                    }
                    let right_txt = extract_target_ident(right, source_bytes).unwrap_or_default();
                    let left_variant = extract_variant_ident(left, source_bytes, known_variants);
                    if right_txt == var_name && left_variant.is_some() {
                        return true;
                    }
                } else if op == "&&" || op == "||" {
                    return condition_compares_var_to_variants(left, source_bytes, var_name, known_variants)
                        || condition_compares_var_to_variants(right, source_bytes, var_name, known_variants);
                }
            }
            false
        }
        _ => false,
    }
}

pub(crate) fn collect_enum_definitions(
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

pub(crate) fn collect_type_declarators(node: Node, source_bytes: &[u8]) -> Vec<String> {
    let mut names = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "type_identifier" {
            names.push(node_text(child, source_bytes));
        }
    }
    names
}

pub(crate) fn extract_enum_variants(enum_node: Node, source_bytes: &[u8]) -> Vec<String> {
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

pub(crate) fn collect_variables(
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

pub(crate) fn extract_declaration_variables(
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

pub(crate) fn extract_type_name(decl_node: Node, source_bytes: &[u8]) -> Option<String> {
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

pub(crate) fn extract_ident_name(mut node: Node, source_bytes: &[u8]) -> Option<String> {
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

pub(crate) fn extract_function_name(node: Node, source_bytes: &[u8]) -> Option<String> {
    if let Some(declarator) = node.child_by_field_name("declarator") {
        extract_ident_name(declarator, source_bytes)
    } else {
        None
    }
}

pub(crate) fn find_child_of_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == kind {
            return Some(child);
        }
    }
    None
}
