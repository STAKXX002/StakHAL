use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::source::marker_scan::{scan_file, ScanError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PvDeclaration {
    pub name: String,
    pub type_str: String,              // e.g. "uint32_t", "float", "TIM_HandleTypeDef"
    pub initial_value: Option<String>, // e.g. "0", "3.14f" - None if uninitialized
    pub is_pointer: bool,
    pub is_array: bool,
    pub array_dims: Option<String>, // raw text inside [], e.g. "64" - None if not an array
    pub raw_text: String, // the exact full declaration statement text, verbatim, as a fallback for display even if structured fields are imperfect
    pub byte_range: (usize, usize), // full declaration statement's span in the whole file (not region-relative) - needed later for write-back of a single declaration
    pub line: usize, // 1-indexed line number where the declaration statement begins
}

#[derive(thiserror::Error, Debug)]
pub enum PvExtractError {
    #[error(transparent)]
    ScanError(#[from] ScanError),
    #[error("no region tagged 'PV' found in file")]
    NoPvRegion,
    #[error("failed to parse C source: {0}")]
    ParseError(String),
}

/// Parses individual C variable declarations out of the PV UserRegion's content.
///
/// Limitations:
/// For complex or atypical C declarations (e.g., function pointers, complex inline struct/union definitions),
/// structured field extraction performs a best-effort fallback to capture the variable name and raw source text.
/// `raw_text` always contains the exact verbatim declaration slice from the source file.
pub fn extract_pv_declarations(path: &Path) -> Result<Vec<PvDeclaration>, PvExtractError> {
    let regions = scan_file(path)?;
    let pv_region = regions
        .into_iter()
        .find(|r| r.tag == "PV")
        .ok_or(PvExtractError::NoPvRegion)?;

    let source = fs::read_to_string(path).map_err(|e| ScanError::IoError(e.to_string()))?;
    let source_bytes = source.as_bytes();

    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_c::language())
        .map_err(|e| PvExtractError::ParseError(e.to_string()))?;

    let tree = parser
        .parse(&source, None)
        .ok_or_else(|| PvExtractError::ParseError("tree-sitter parse returned None".to_string()))?;

    let mut out = Vec::new();
    find_declarations_in_region(
        tree.root_node(),
        pv_region.byte_range,
        source_bytes,
        &mut out,
    );

    Ok(out)
}

fn find_declarations_in_region(
    node: tree_sitter::Node,
    region_range: (usize, usize),
    source_bytes: &[u8],
    out: &mut Vec<PvDeclaration>,
) {
    let node_start = node.start_byte();
    let node_end = node.end_byte();

    // Skip nodes completely outside the PV region
    if node_end <= region_range.0 || node_start >= region_range.1 {
        return;
    }

    if node.kind() == "declaration" {
        if node_start >= region_range.0 && node_end <= region_range.1 {
            process_declaration_node(node, source_bytes, out);
        }
        return;
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        find_declarations_in_region(child, region_range, source_bytes, out);
    }
}

fn process_declaration_node(
    node: tree_sitter::Node,
    source_bytes: &[u8],
    out: &mut Vec<PvDeclaration>,
) {
    let line = node.start_position().row + 1;
    let raw_text = node
        .utf8_text(source_bytes)
        .unwrap_or("")
        .trim()
        .to_string();
    let byte_range = (node.start_byte(), node.end_byte());

    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();

    // Identify declarator nodes (children that represent declarators rather than type specifiers/qualifiers/semi)
    let declarator_nodes: Vec<_> = children
        .iter()
        .copied()
        .filter(|c| is_declarator_node(c.kind()))
        .collect();

    if declarator_nodes.is_empty() {
        return;
    }

    // Base type_str is everything in the declaration before the first declarator
    let first_decl = declarator_nodes[0];
    let type_slice = &source_bytes[node.start_byte()..first_decl.start_byte()];
    let mut type_str = std::str::from_utf8(type_slice)
        .unwrap_or("")
        .trim()
        .to_string();

    if type_str.is_empty() {
        type_str = "int".to_string();
    }

    for decl_node in declarator_nodes {
        let info = parse_declarator(decl_node, source_bytes);
        out.push(PvDeclaration {
            name: info.name,
            type_str: type_str.clone(),
            initial_value: info.initial_value,
            is_pointer: info.is_pointer,
            is_array: info.is_array,
            array_dims: info.array_dims,
            raw_text: raw_text.clone(),
            byte_range,
            line,
        });
    }
}


fn is_declarator_node(kind: &str) -> bool {
    matches!(
        kind,
        "init_declarator"
            | "pointer_declarator"
            | "array_declarator"
            | "function_declarator"
            | "parenthesized_declarator"
            | "identifier"
            | "field_identifier"
    )
}

struct DeclaratorInfo {
    name: String,
    initial_value: Option<String>,
    is_pointer: bool,
    is_array: bool,
    array_dims: Option<String>,
}

fn parse_declarator(node: tree_sitter::Node, source_bytes: &[u8]) -> DeclaratorInfo {
    let mut info = DeclaratorInfo {
        name: String::new(),
        initial_value: None,
        is_pointer: false,
        is_array: false,
        array_dims: None,
    };

    match node.kind() {
        "init_declarator" => {
            let mut cursor = node.walk();
            let children: Vec<_> = node.children(&mut cursor).collect();

            let mut eq_idx = None;
            for (i, child) in children.iter().enumerate() {
                if child.kind() == "=" {
                    eq_idx = Some(i);
                    break;
                }
            }

            let decl_child = if let Some(idx) = eq_idx {
                if idx + 1 < children.len() {
                    let val_node = children[idx + 1];
                    info.initial_value = Some(
                        val_node
                            .utf8_text(source_bytes)
                            .unwrap_or("")
                            .trim()
                            .to_string(),
                    );
                }
                children[0]
            } else {
                children[0]
            };

            let inner = parse_declarator(decl_child, source_bytes);
            info.name = inner.name;
            info.is_pointer = inner.is_pointer;
            info.is_array = inner.is_array;
            info.array_dims = inner.array_dims;
        }
        "pointer_declarator" => {
            info.is_pointer = true;
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() != "*" && child.kind() != "type_qualifier" {
                    let inner = parse_declarator(child, source_bytes);
                    info.name = inner.name;
                    info.is_pointer = true;
                    info.is_array = inner.is_array || info.is_array;
                    if inner.array_dims.is_some() {
                        info.array_dims = inner.array_dims;
                    }
                    if inner.initial_value.is_some() {
                        info.initial_value = inner.initial_value;
                    }
                    break;
                }
            }
        }
        "array_declarator" => {
            info.is_array = true;
            let mut cursor = node.walk();
            let children: Vec<_> = node.children(&mut cursor).collect();

            let mut open_bracket = None;
            let mut close_bracket = None;
            for child in &children {
                if child.kind() == "[" {
                    open_bracket = Some(child.end_byte());
                } else if child.kind() == "]" {
                    close_bracket = Some(child.start_byte());
                }
            }
            if let (Some(start), Some(end)) = (open_bracket, close_bracket) {
                let dims_slice = &source_bytes[start..end];
                let dims = std::str::from_utf8(dims_slice)
                    .unwrap_or("")
                    .trim()
                    .to_string();
                info.array_dims = Some(dims);
            }

            if let Some(&first_child) = children.first() {
                let inner = parse_declarator(first_child, source_bytes);
                info.name = inner.name;
                info.is_pointer = inner.is_pointer || info.is_pointer;
                if inner.initial_value.is_some() {
                    info.initial_value = inner.initial_value;
                }
            }
        }
        "parenthesized_declarator" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() != "(" && child.kind() != ")" {
                    return parse_declarator(child, source_bytes);
                }
            }
        }
        "identifier" | "field_identifier" => {
            info.name = node
                .utf8_text(source_bytes)
                .unwrap_or("")
                .trim()
                .to_string();
        }
        _ => {
            info.name = find_first_identifier(node, source_bytes)
                .unwrap_or_else(|| node.utf8_text(source_bytes).unwrap_or("").trim().to_string());
        }
    }

    info
}

fn find_first_identifier(node: tree_sitter::Node, source_bytes: &[u8]) -> Option<String> {
    if node.kind() == "identifier" || node.kind() == "field_identifier" {
        return Some(node.utf8_text(source_bytes).ok()?.trim().to_string());
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(name) = find_first_identifier(child, source_bytes) {
            return Some(name);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_single_simple_declaration() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("main.c");
        let content = r#"
/* USER CODE BEGIN PV */
uint32_t counter = 0;
/* USER CODE END PV */
"#;
        fs::write(&file_path, content).unwrap();

        let decls = extract_pv_declarations(&file_path).unwrap();
        assert_eq!(decls.len(), 1);
        let decl = &decls[0];
        assert_eq!(decl.name, "counter");
        assert_eq!(decl.type_str, "uint32_t");
        assert_eq!(decl.initial_value, Some("0".to_string()));
        assert!(!decl.is_pointer);
        assert!(!decl.is_array);
        assert_eq!(decl.array_dims, None);
        assert_eq!(decl.raw_text, "uint32_t counter = 0;");
        assert_eq!(decl.line, 3);
    }

    #[test]
    fn test_multiple_declarators_in_one_statement() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("main.c");
        let content = r#"
/* USER CODE BEGIN PV */
int a, b = 5;
/* USER CODE END PV */
"#;
        fs::write(&file_path, content).unwrap();

        let decls = extract_pv_declarations(&file_path).unwrap();
        assert_eq!(decls.len(), 2);

        assert_eq!(decls[0].name, "a");
        assert_eq!(decls[0].type_str, "int");
        assert_eq!(decls[0].initial_value, None);
        assert_eq!(decls[0].raw_text, "int a, b = 5;");
        assert_eq!(decls[0].line, 3);

        assert_eq!(decls[1].name, "b");
        assert_eq!(decls[1].type_str, "int");
        assert_eq!(decls[1].initial_value, Some("5".to_string()));
        assert_eq!(decls[1].raw_text, "int a, b = 5;");
        assert_eq!(decls[1].line, 3);
    }

    #[test]
    fn test_array_declaration() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("main.c");
        let content = r#"
/* USER CODE BEGIN PV */
uint8_t buffer[64];
/* USER CODE END PV */
"#;
        fs::write(&file_path, content).unwrap();

        let decls = extract_pv_declarations(&file_path).unwrap();
        assert_eq!(decls.len(), 1);
        let decl = &decls[0];
        assert_eq!(decl.name, "buffer");
        assert_eq!(decl.type_str, "uint8_t");
        assert!(decl.is_array);
        assert_eq!(decl.array_dims, Some("64".to_string()));
        assert!(!decl.is_pointer);
        assert_eq!(decl.initial_value, None);
        assert_eq!(decl.line, 3);
    }

    #[test]
    fn test_pointer_declaration() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("main.c");
        let content = r#"
/* USER CODE BEGIN PV */
TIM_HandleTypeDef *htim2;
/* USER CODE END PV */
"#;
        fs::write(&file_path, content).unwrap();

        let decls = extract_pv_declarations(&file_path).unwrap();
        assert_eq!(decls.len(), 1);
        let decl = &decls[0];
        assert_eq!(decl.name, "htim2");
        assert_eq!(decl.type_str, "TIM_HandleTypeDef");
        assert!(decl.is_pointer);
        assert!(!decl.is_array);
        assert_eq!(decl.line, 3);
    }


    #[test]
    fn test_pv_region_with_only_comments() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("main.c");
        let content = r#"
/* USER CODE BEGIN PV */
/* Just a comment inside PV */
// Another comment
/* USER CODE END PV */
"#;
        fs::write(&file_path, content).unwrap();

        let decls = extract_pv_declarations(&file_path).unwrap();
        assert!(decls.is_empty());
    }

    #[test]
    fn test_no_pv_region_error() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("main.c");
        let content = r#"
/* USER CODE BEGIN 0 */
int x = 1;
/* USER CODE END 0 */
"#;
        fs::write(&file_path, content).unwrap();

        let res = extract_pv_declarations(&file_path);
        assert!(matches!(res, Err(PvExtractError::NoPvRegion)));
    }
}
