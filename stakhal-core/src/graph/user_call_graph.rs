use std::collections::HashSet;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use tree_sitter::Parser;

use crate::graph::hal_rules::HAL_IRQ_MAPPINGS;
use crate::source::marker_scan::ScanError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserFunction {
    pub name: String,
    pub byte_range: (usize, usize),   // full function definition span
    pub line: usize,                   // 1-indexed, definition line
    pub is_hal_callback: bool,         // true if this name matches a known weak-callback name from hal_rules.rs
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserCallEdge {
    pub from: String,   // calling function's name
    pub to: String,     // called function's name
}

pub fn build_user_call_graph(
    path: &Path,
) -> Result<(Vec<UserFunction>, Vec<UserCallEdge>), ScanError> {
    if !path.exists() {
        return Err(ScanError::FileNotFound(path.to_path_buf()));
    }

    let source = fs::read_to_string(path).map_err(|e| ScanError::IoError(e.to_string()))?;

    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_c::language())
        .map_err(|e| ScanError::ParseError(e.to_string()))?;

    let tree = parser
        .parse(&source, None)
        .ok_or_else(|| ScanError::ParseError("Failed to parse C file".to_string()))?;

    let mut fn_nodes = Vec::new();
    collect_function_definitions(tree.root_node(), &source, &mut fn_nodes);

    let mut user_functions = Vec::new();
    for (node, name) in fn_nodes {
        let byte_range = (node.start_byte(), node.end_byte());
        let line = node.start_position().row + 1;
        let is_hal_callback = is_known_hal_callback(&name);

        user_functions.push((
            node,
            UserFunction {
                name,
                byte_range,
                line,
                is_hal_callback,
            },
        ));
    }

    let user_function_names: HashSet<String> = user_functions
        .iter()
        .map(|(_, f)| f.name.clone())
        .collect();

    let mut edge_set = HashSet::new();
    let mut edges = Vec::new();

    for (node, user_fn) in &user_functions {
        let mut calls = Vec::new();
        collect_call_expressions(*node, &source, &mut calls);

        for called_name in calls {
            if user_function_names.contains(&called_name) {
                let key = (user_fn.name.clone(), called_name.clone());
                if edge_set.insert(key) {
                    edges.push(UserCallEdge {
                        from: user_fn.name.clone(),
                        to: called_name,
                    });
                }
            }
        }
    }

    let result_functions = user_functions.into_iter().map(|(_, f)| f).collect();
    Ok((result_functions, edges))
}

fn is_known_hal_callback(name: &str) -> bool {
    HAL_IRQ_MAPPINGS
        .iter()
        .any(|m| m.weak_callbacks.contains(&name))
}

fn collect_function_definitions<'a>(
    node: tree_sitter::Node<'a>,
    source: &'a str,
    out: &mut Vec<(tree_sitter::Node<'a>, String)>,
) {
    if node.kind() == "function_definition" {
        if let Some(declarator) = node.child_by_field_name("declarator") {
            if let Some(name) = extract_function_name(declarator, source) {
                out.push((node, name));
            }
        }
    } else {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            collect_function_definitions(child, source, out);
        }
    }
}

fn extract_function_name(mut node: tree_sitter::Node, source: &str) -> Option<String> {
    loop {
        match node.kind() {
            "identifier" => return Some(source[node.byte_range()].to_string()),
            "function_declarator" | "pointer_declarator" | "parenthesized_declarator" | "array_declarator" => {
                if let Some(child) = node.child_by_field_name("declarator") {
                    node = child;
                } else if let Some(child) = node.child(0) {
                    node = child;
                } else {
                    return None;
                }
            }
            _ => {
                let mut found_next = false;
                for i in 0..node.child_count() {
                    if let Some(child) = node.child(i) {
                        if child.kind() == "identifier" {
                            return Some(source[child.byte_range()].to_string());
                        }
                        if child.child_by_field_name("declarator").is_some() {
                            node = child;
                            found_next = true;
                            break;
                        }
                    }
                }
                if !found_next {
                    return None;
                }
            }
        }
    }
}

fn collect_call_expressions<'a>(
    node: tree_sitter::Node<'a>,
    source: &'a str,
    out: &mut Vec<String>,
) {
    if node.kind() == "call_expression" {
        if let Some(fn_child) = node.child_by_field_name("function") {
            if let Some(called_name) = extract_called_name(fn_child, source) {
                out.push(called_name);
            }
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_call_expressions(child, source, out);
    }
}

fn extract_called_name(mut node: tree_sitter::Node, source: &str) -> Option<String> {
    loop {
        match node.kind() {
            "identifier" => return Some(source[node.byte_range()].to_string()),
            "parenthesized_expression" => {
                if let Some(child) = node.child(0) {
                    node = child;
                } else {
                    return None;
                }
            }
            "field_expression" => {
                if let Some(field) = node.child_by_field_name("field") {
                    node = field;
                } else {
                    return None;
                }
            }
            _ => return None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_two_user_functions_calling_each_other() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("main.c");
        let content = r#"
void worker(void) {
    // work
}

void caller(void) {
    worker();
}
"#;
        fs::write(&file_path, content).unwrap();

        let (fns, edges) = build_user_call_graph(&file_path).unwrap();
        assert_eq!(fns.len(), 2);
        assert!(fns.iter().any(|f| f.name == "worker"));
        assert!(fns.iter().any(|f| f.name == "caller"));

        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].from, "caller");
        assert_eq!(edges[0].to, "worker");
    }

    #[test]
    fn test_hal_library_calls_filtered_out() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("main.c");
        let content = r#"
void do_something(void) {
    HAL_GPIO_TogglePin(GPIOA, GPIO_PIN_5);
    memcpy(dest, src, 10);
}
"#;
        fs::write(&file_path, content).unwrap();

        let (fns, edges) = build_user_call_graph(&file_path).unwrap();
        assert_eq!(fns.len(), 1);
        assert_eq!(fns[0].name, "do_something");
        assert_eq!(fns[0].is_hal_callback, false);

        assert!(edges.is_empty());
    }

    #[test]
    fn test_weak_callback_identification() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("main.c");
        let content = r#"
void HAL_TIM_PeriodElapsedCallback(TIM_HandleTypeDef *htim) {
    // callback
}

void normal_user_func(void) {
    // normal
}
"#;
        fs::write(&file_path, content).unwrap();

        let (fns, _) = build_user_call_graph(&file_path).unwrap();
        assert_eq!(fns.len(), 2);

        let cb = fns.iter().find(|f| f.name == "HAL_TIM_PeriodElapsedCallback").unwrap();
        assert!(cb.is_hal_callback);

        let normal = fns.iter().find(|f| f.name == "normal_user_func").unwrap();
        assert!(!normal.is_hal_callback);
    }

    #[test]
    fn test_stm32_03_timers_fixture_regression() {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let main_c_path = manifest_dir.join("tests/fixtures/stm32_03_timers/Core/Src/main.c");

        let (fns, edges) = build_user_call_graph(&main_c_path).unwrap();

        let cb = fns
            .iter()
            .find(|f| f.name == "HAL_TIM_PeriodElapsedCallback")
            .expect("HAL_TIM_PeriodElapsedCallback should be found in stm32_03_timers main.c");

        assert!(cb.is_hal_callback, "HAL_TIM_PeriodElapsedCallback must be flagged as HAL callback");

        let cb_outgoing_edges: Vec<&UserCallEdge> = edges.iter().filter(|e| e.from == "HAL_TIM_PeriodElapsedCallback").collect();
        assert!(
            cb_outgoing_edges.is_empty(),
            "HAL_TIM_PeriodElapsedCallback in stm32_03_timers only does inline increment, should have 0 outgoing UserCallEdges"
        );
    }
}
