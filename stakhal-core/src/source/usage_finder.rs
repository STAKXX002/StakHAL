use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::source::marker_scan::ScanError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageSite {
    pub line: usize,                // 1-indexed
    pub byte_range: (usize, usize), // the identifier token's own span
    pub context_snippet: String,    // the containing line, trimmed, for display
}

/// Finds all AST identifier references to `variable_name` in the given file,
/// excluding nodes that fall within `declaration_byte_range`.
///
/// Known limitation: This lookup is name-based and not scope-aware. If a shadowed
/// local variable with the exact same name exists elsewhere in the file, it will be
/// included as a usage. This is acceptable for typical embedded main.c structure,
/// but is not semantically correct C scope resolution.
pub fn find_variable_usages(
    path: &Path,
    variable_name: &str,
    declaration_byte_range: (usize, usize),
) -> Result<Vec<UsageSite>, ScanError> {
    let mut batch_res = find_variable_usages_batch(path, &[(variable_name, declaration_byte_range)])?;
    Ok(batch_res.pop().unwrap_or_default())
}

pub fn find_variable_usages_batch(
    path: &Path,
    targets: &[(&str, (usize, usize))],
) -> Result<Vec<Vec<UsageSite>>, ScanError> {
    let source = fs::read_to_string(path).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => ScanError::FileNotFound(path.to_path_buf()),
        _ => ScanError::IoError(e.to_string()),
    })?;
    find_variable_usages_batch_from_source(&source, targets)
}

pub fn find_variable_usages_batch_from_source(
    source: &str,
    targets: &[(&str, (usize, usize))],
) -> Result<Vec<Vec<UsageSite>>, ScanError> {
    if targets.is_empty() {
        return Ok(Vec::new());
    }

    let source_bytes = source.as_bytes();

    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_c::language())
        .map_err(|e| ScanError::ParseError(e.to_string()))?;

    let tree = parser
        .parse(source, None)
        .ok_or_else(|| ScanError::ParseError("tree-sitter parse returned None".to_string()))?;

    let lines: Vec<&str> = source.lines().collect();

    let mut target_map: std::collections::HashMap<&str, Vec<usize>> = std::collections::HashMap::new();
    for (idx, (name, _)) in targets.iter().enumerate() {
        target_map.entry(name.trim()).or_default().push(idx);
    }

    let mut results = vec![Vec::new(); targets.len()];

    walk_and_collect_usages_batch(
        tree.root_node(),
        targets,
        &target_map,
        source_bytes,
        &lines,
        &mut results,
    );

    Ok(results)
}

fn walk_and_collect_usages_batch(
    node: tree_sitter::Node,
    targets: &[(&str, (usize, usize))],
    target_map: &std::collections::HashMap<&str, Vec<usize>>,
    source_bytes: &[u8],
    lines: &[&str],
    results: &mut [Vec<UsageSite>],
) {
    let node_start = node.start_byte();
    let node_end = node.end_byte();

    if node.kind() == "identifier" || node.kind() == "field_identifier" {
        if let Ok(text) = node.utf8_text(source_bytes) {
            let key = text.trim();
            if let Some(target_indices) = target_map.get(key) {
                let row = node.start_position().row; // 0-indexed
                let line = row + 1; // 1-indexed
                let context_snippet = if row < lines.len() {
                    lines[row].trim().to_string()
                } else {
                    String::new()
                };

                for &idx in target_indices {
                    let decl_range = targets[idx].1;
                    let overlaps_decl = !(node_end <= decl_range.0 || node_start >= decl_range.1);
                    if !overlaps_decl {
                        results[idx].push(UsageSite {
                            line,
                            byte_range: (node_start, node_end),
                            context_snippet: context_snippet.clone(),
                        });
                    }
                }
            }
        }
        return;
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_and_collect_usages_batch(child, targets, target_map, source_bytes, lines, results);
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_variable_declared_and_used_twice() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("main.c");
        let content = r#"int counter = 0;

void step(void) {
    if (counter > 10) {
        counter = 0;
    }
}
"#;
        fs::write(&file_path, content).unwrap();

        // "int counter = 0;" declaration byte range is (0, 16)
        let decl_range = (0, 16);
        let usages = find_variable_usages(&file_path, "counter", decl_range).unwrap();

        assert_eq!(usages.len(), 2);
        assert_eq!(usages[0].line, 4);
        assert_eq!(usages[0].context_snippet, "if (counter > 10) {");

        assert_eq!(usages[1].line, 5);
        assert_eq!(usages[1].context_snippet, "counter = 0;");
    }

    #[test]
    fn test_exact_match_no_substring_matches() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("main.c");
        let content = r#"int count = 0;
int isrCount = 5;

void process(void) {
    count++;
    isrCount++;
}
"#;
        fs::write(&file_path, content).unwrap();

        // Declaration range of count is (0, 14)
        let decl_range = (0, 14);
        let usages = find_variable_usages(&file_path, "count", decl_range).unwrap();

        assert_eq!(usages.len(), 1);
        assert_eq!(usages[0].line, 5);
        assert_eq!(usages[0].context_snippet, "count++;");
    }

    #[test]
    fn test_ignore_comments_and_string_literals() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("main.c");
        let content = r#"int flag = 1;

void check(void) {
    // This comment mentions flag
    printf("flag is active\n");
    if (flag) {
        // do something
    }
}
"#;
        fs::write(&file_path, content).unwrap();

        let decl_range = (0, 13);
        let usages = find_variable_usages(&file_path, "flag", decl_range).unwrap();

        assert_eq!(usages.len(), 1);
        assert_eq!(usages[0].line, 6);
        assert_eq!(usages[0].context_snippet, "if (flag) {");
    }

    #[test]
    fn test_batch_find_variable_usages() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("main.c");
        let content = r#"int flag = 1;
int counter = 0;
int unusedVar = 10;

void check(void) {
    if (flag) {
        counter++;
    }
}
"#;
        fs::write(&file_path, content).unwrap();

        let targets = vec![
            ("flag", (0, 13)),
            ("counter", (14, 30)),
            ("unusedVar", (31, 50)),
        ];

        let batch_usages = find_variable_usages_batch(&file_path, &targets).unwrap();

        assert_eq!(batch_usages.len(), 3);
        assert_eq!(batch_usages[0].len(), 1);
        assert_eq!(batch_usages[0][0].context_snippet, "if (flag) {");

        assert_eq!(batch_usages[1].len(), 1);
        assert_eq!(batch_usages[1][0].context_snippet, "counter++;");

        assert_eq!(batch_usages[2].len(), 0);
    }
}

