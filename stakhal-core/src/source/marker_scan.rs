use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use tree_sitter::Parser;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserRegion {
    pub tag: String,
    pub file: PathBuf,
    pub byte_range: (usize, usize), // content strictly between BEGIN and END markers
    pub line_range: (usize, usize), // display only, never used for write-back
    pub begin_marker_range: (usize, usize), // byte span of the "/* USER CODE BEGIN <TAG> */" comment itself
    pub end_marker_range: (usize, usize), // byte span of the "/* USER CODE END <TAG> */" comment itself
}

#[derive(thiserror::Error, Debug)]
pub enum ScanError {
    #[error("file not found: {0}")]
    FileNotFound(PathBuf),
    #[error("I/O error: {0}")]
    IoError(String),
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
    begin_marker_range: (usize, usize),
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
    let content = fs::read_to_string(path).map_err(|err| match err.kind() {
        std::io::ErrorKind::NotFound => ScanError::FileNotFound(path.to_path_buf()),
        _ => ScanError::IoError(err.to_string()),
    })?;
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
                            begin_marker_range: (node.start_byte(), node.end_byte()),
                        });
                    }
                    MarkerType::End(tag) => match open_stack.pop() {
                        Some(open) => {
                            if open.tag != tag {
                                return Err(ScanError::MismatchedMarkers(open.tag));
                            }
                            let byte_range = (open.end_byte, node.start_byte());
                            let line_range = (open.start_line, node.end_position().row + 1);
                            let end_marker_range = (node.start_byte(), node.end_byte());
                            regions.push(UserRegion {
                                tag,
                                file: path.to_path_buf(),
                                byte_range,
                                line_range,
                                begin_marker_range: open.begin_marker_range,
                                end_marker_range,
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

/// Finds the implicit loop body user code region between `USER CODE END WHILE` and the next region's `USER CODE BEGIN`.
///
/// CubeMX's generated main loop structure places the main loop body between `/* USER CODE END WHILE */`
/// and `/* USER CODE BEGIN 3 */` (or whichever region immediately follows `WHILE`).
///
/// Note: For this synthetic region, `begin_marker_range` and `end_marker_range` are both set to `(0, 0)`
/// because it is an implicit gap region bounded by adjacent markers rather than possessing its own dedicated BEGIN/END marker comments.
pub fn find_loop_body_gap(regions: &[UserRegion]) -> Option<UserRegion> {
    let while_idx = regions.iter().position(|r| r.tag == "WHILE")?;
    let while_region = regions.get(while_idx)?;
    let next_region = regions.get(while_idx + 1)?;

    let byte_range = (
        while_region.end_marker_range.1,
        next_region.begin_marker_range.0,
    );
    let line_range = (while_region.line_range.1, next_region.line_range.0);

    Some(UserRegion {
        tag: "__loop_body__".to_string(),
        file: while_region.file.clone(),
        byte_range,
        line_range,
        begin_marker_range: (0, 0),
        end_marker_range: (0, 0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_file_not_found_error() {
        let path = Path::new("non_existent_file_12345.c");
        let res = scan_file(path);
        assert!(matches!(res, Err(ScanError::FileNotFound(ref p)) if p == path));
    }

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
        assert_eq!(
            &source[regions[0].byte_range.0..regions[0].byte_range.1],
            "\nint my_var = 42;\n"
        );

        assert_eq!(
            &source[regions[0].begin_marker_range.0..regions[0].begin_marker_range.1],
            "/* USER CODE BEGIN PV */"
        );
        assert_eq!(
            &source[regions[0].end_marker_range.0..regions[0].end_marker_range.1],
            "/* USER CODE END PV */"
        );
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
        assert_eq!(
            &source[regions[0].byte_range.0..regions[0].byte_range.1],
            "\n#include <stdio.h>\n"
        );

        assert_eq!(regions[1].tag, "PV");
        assert_eq!(
            &source[regions[1].byte_range.0..regions[1].byte_range.1],
            "\nstatic int x = 0;\n"
        );
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

    #[test]
    fn test_find_loop_body_gap_realistic() {
        let source = r#"
  /* USER CODE BEGIN WHILE */
  while (1)
  {
    /* USER CODE END WHILE */

    /* USER CODE BEGIN 3 */
  }
  /* USER CODE END 3 */
"#;
        let path = Path::new("main.c");
        let regions = scan_source(path, source).unwrap();
        let gap = find_loop_body_gap(&regions).unwrap();

        assert_eq!(gap.tag, "__loop_body__");
        assert_eq!(gap.begin_marker_range, (0, 0));
        assert_eq!(gap.end_marker_range, (0, 0));

        let gap_text = &source[gap.byte_range.0..gap.byte_range.1];
        assert_eq!(gap_text, "\n\n    ");
    }

    #[test]
    fn test_find_loop_body_gap_with_user_code() {
        let source = r#"
  /* USER CODE BEGIN WHILE */
  while (1)
  {
    /* USER CODE END WHILE */
    HAL_GPIO_TogglePin(GPIOA, GPIO_PIN_5);
    HAL_Delay(500);
    /* USER CODE BEGIN 3 */
  }
  /* USER CODE END 3 */
"#;
        let path = Path::new("main.c");
        let regions = scan_source(path, source).unwrap();
        let gap = find_loop_body_gap(&regions).unwrap();

        assert_eq!(gap.tag, "__loop_body__");
        let gap_text = &source[gap.byte_range.0..gap.byte_range.1];
        assert_eq!(
            gap_text,
            "\n    HAL_GPIO_TogglePin(GPIOA, GPIO_PIN_5);\n    HAL_Delay(500);\n    "
        );
    }

    #[test]
    fn test_find_loop_body_gap_no_while_tag() {
        let source = r#"
/* USER CODE BEGIN PV */
int x = 0;
/* USER CODE END PV */
"#;
        let path = Path::new("main.c");
        let regions = scan_source(path, source).unwrap();
        assert!(find_loop_body_gap(&regions).is_none());
    }

    #[test]
    fn test_find_loop_body_gap_while_is_last_region() {
        let source = r#"
/* USER CODE BEGIN WHILE */
while (1) {
/* USER CODE END WHILE */
"#;
        let path = Path::new("main.c");
        let regions = scan_source(path, source).unwrap();
        assert!(find_loop_body_gap(&regions).is_none());
    }
}
