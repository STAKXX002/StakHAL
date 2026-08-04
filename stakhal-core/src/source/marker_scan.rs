use std::fs;
use std::path::{Path, PathBuf};
use tree_sitter::Parser;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserRegion {
    pub tag: String,
    pub file: PathBuf,
    pub byte_range: (usize, usize), // content strictly between BEGIN and END markers
    pub line_range: (usize, usize), // display only, never used for write-back
}

#[derive(thiserror::Error, Debug)]
pub enum ScanError {
    #[error("file not found: {0}")]
    FileNotFound(PathBuf),
    #[error("failed to parse C source: {0}")]
    ParseError(String),
    #[error("mismatched USER CODE BEGIN/END for tag '{0}'")]
    MismatchedMarkers(String),
    #[error("USER CODE END tag '{0}' does not match any open BEGIN")]
    UnknownEndTag(String),
}

struct OpenMarker {
    tag: String,
    end_byte: usize,
    start_line: usize,
}

enum MarkerType {
    Begin(String),
    End(String),
}

fn parse_comment_marker(text: &str) -> Option<MarkerType> {
    let inner = if let Some(stripped) = text.strip_prefix("/*") {
        stripped.strip_suffix("*/").unwrap_or(stripped)
    } else if let Some(stripped) = text.strip_prefix("//") {
        stripped
    } else {
        return None;
    };

    let trimmed = inner.trim();

    if let Some(tag_part) = trimmed.strip_prefix("USER CODE BEGIN ") {
        let tag = tag_part.trim();
        if !tag.is_empty() {
            return Some(MarkerType::Begin(tag.to_string()));
        }
    } else if let Some(tag_part) = trimmed.strip_prefix("USER CODE END ") {
        let tag = tag_part.trim();
        if !tag.is_empty() {
            return Some(MarkerType::End(tag.to_string()));
        }
    }

    None
}

pub fn scan_file(path: &Path) -> Result<Vec<UserRegion>, ScanError> {
    let content = fs::read_to_string(path)
        .map_err(|_| ScanError::FileNotFound(path.to_path_buf()))?;
    scan_source(path, &content)
}

pub fn scan_source(path: &Path, source: &str) -> Result<Vec<UserRegion>, ScanError> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_c::language())
        .map_err(|e| ScanError::ParseError(e.to_string()))?;

    let tree = parser
        .parse(source, None)
        .ok_or_else(|| ScanError::ParseError("tree-sitter parser returned None".into()))?;

    let mut regions = Vec::new();
    let mut open_stack: Vec<OpenMarker> = Vec::new();

    let mut cursor = tree.walk();
    let mut reach_end = false;

    loop {
        let node = cursor.node();
        if node.kind() == "comment" {
            let text = &source[node.byte_range()];
            if let Some(marker) = parse_comment_marker(text) {
                match marker {
                    MarkerType::Begin(tag) => {
                        open_stack.push(OpenMarker {
                            tag,
                            end_byte: node.end_byte(),
                            start_line: node.start_position().row + 1,
                        });
                    }
                    MarkerType::End(tag) => match open_stack.pop() {
                        Some(open) => {
                            if open.tag != tag {
                                return Err(ScanError::MismatchedMarkers(open.tag));
                            }
                            let byte_range = (open.end_byte, node.start_byte());
                            let line_range = (open.start_line, node.end_position().row + 1);
                            regions.push(UserRegion {
                                tag,
                                file: path.to_path_buf(),
                                byte_range,
                                line_range,
                            });
                        }
                        None => {
                            return Err(ScanError::UnknownEndTag(tag));
                        }
                    },
                }
            }
        }

        if cursor.goto_first_child() {
            continue;
        }
        if cursor.goto_next_sibling() {
            continue;
        }
        loop {
            if !cursor.goto_parent() {
                reach_end = true;
                break;
            }
            if cursor.goto_next_sibling() {
                break;
            }
        }
        if reach_end {
            break;
        }
    }

    if let Some(open) = open_stack.pop() {
        return Err(ScanError::MismatchedMarkers(open.tag));
    }

    Ok(regions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_single_well_formed_region() {
        let source = r#"
/* USER CODE BEGIN PV */
int my_var = 42;
/* USER CODE END PV */
"#;
        let path = Path::new("main.c");
        let regions = scan_source(path, source).unwrap();
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].tag, "PV");
        assert_eq!(regions[0].file, path);
        assert_eq!(&source[regions[0].byte_range.0..regions[0].byte_range.1], "\nint my_var = 42;\n");
    }

    #[test]
    fn test_two_sibling_regions() {
        let source = r#"
/* USER CODE BEGIN Includes */
#include <stdio.h>
/* USER CODE END Includes */

/* USER CODE BEGIN PV */
static int x = 0;
/* USER CODE END PV */
"#;
        let path = Path::new("main.c");
        let regions = scan_source(path, source).unwrap();
        assert_eq!(regions.len(), 2);
        assert_eq!(regions[0].tag, "Includes");
        assert_eq!(&source[regions[0].byte_range.0..regions[0].byte_range.1], "\n#include <stdio.h>\n");

        assert_eq!(regions[1].tag, "PV");
        assert_eq!(&source[regions[1].byte_range.0..regions[1].byte_range.1], "\nstatic int x = 0;\n");
    }

    #[test]
    fn test_unmatched_begin() {
        let source = r#"
/* USER CODE BEGIN PV */
int x = 1;
"#;
        let path = Path::new("main.c");
        let res = scan_source(path, source);
        assert!(matches!(res, Err(ScanError::MismatchedMarkers(tag)) if tag == "PV"));
    }

    #[test]
    fn test_mismatched_begin_end_tags() {
        let source = r#"
/* USER CODE BEGIN PV */
int x = 1;
/* USER CODE END 0 */
"#;
        let path = Path::new("main.c");
        let res = scan_source(path, source);
        assert!(matches!(res, Err(ScanError::MismatchedMarkers(tag)) if tag == "PV"));
    }

    #[test]
    fn test_unknown_end_tag() {
        let source = r#"
/* USER CODE END PV */
"#;
        let path = Path::new("main.c");
        let res = scan_source(path, source);
        assert!(matches!(res, Err(ScanError::UnknownEndTag(tag)) if tag == "PV"));
    }

    #[test]
    fn test_comment_mentioning_user_code_ignored() {
        let source = r#"
// this mentions USER CODE but isn't a real marker elsewhere in the file
/* USER CODE BEGIN PV */
int val = 10;
/* USER CODE END PV */
"#;
        let path = Path::new("main.c");
        let regions = scan_source(path, source).unwrap();
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].tag, "PV");
    }

    #[test]
    fn test_string_literal_user_code_ignored() {
        let source = r#"
const char* s = "USER CODE BEGIN PV";
/* USER CODE BEGIN PV */
int val = 20;
/* USER CODE END PV */
const char* e = "USER CODE END PV";
"#;
        let path = Path::new("main.c");
        let regions = scan_source(path, source).unwrap();
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].tag, "PV");
    }
}
