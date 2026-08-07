use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::source::marker_scan::{is_byte_in_user_region, ScanError, UserRegion};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineTier {
    Generated,
    Normal,
    Declaration,
    Usage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderedLine {
    pub line_number: usize, // 1-indexed
    pub text: String,        // the line's raw text, no trailing newline
    pub tier: LineTier,
}

/// Builds a line-by-line render model for the source file at `path`, classifying
/// each line into a `LineTier` (Declaration, Usage, Normal, or Generated).
///
/// Precedence (first match wins):
/// 1. Line's byte range overlaps `declaration_byte_range` -> `Declaration`
/// 2. Line's byte range overlaps any range in `usage_byte_ranges` -> `Usage`
/// 3. Line's start byte falls within a user code region -> `Normal`
/// 4. Otherwise -> `Generated`
pub fn build_source_render_model(
    path: &Path,
    regions: &[UserRegion],
    declaration_byte_range: (usize, usize),
    usage_byte_ranges: &[(usize, usize)],
) -> Result<Vec<RenderedLine>, ScanError> {
    let source = fs::read_to_string(path).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => ScanError::FileNotFound(path.to_path_buf()),
        _ => ScanError::IoError(e.to_string()),
    })?;

    Ok(build_source_render_model_from_str(
        &source,
        regions,
        declaration_byte_range,
        usage_byte_ranges,
    ))
}

pub fn build_source_render_model_from_str(
    source: &str,
    regions: &[UserRegion],
    declaration_byte_range: (usize, usize),
    usage_byte_ranges: &[(usize, usize)],
) -> Vec<RenderedLine> {
    let bytes = source.as_bytes();
    let mut rendered_lines = Vec::new();

    let mut line_number = 1;
    let mut i = 0;

    while i < bytes.len() {
        let start = i;
        while i < bytes.len() && bytes[i] != b'\n' {
            i += 1;
        }

        let end_with_newline = if i < bytes.len() { i + 1 } else { i };

        let line_bytes = &bytes[start..i];
        let line_str_bytes = if line_bytes.ends_with(b"\r") {
            &line_bytes[..line_bytes.len() - 1]
        } else {
            line_bytes
        };
        let text = String::from_utf8_lossy(line_str_bytes).to_string();

        let line_start = start;
        let line_end = end_with_newline;

        let is_decl = !(line_end <= declaration_byte_range.0
            || line_start >= declaration_byte_range.1);

        let is_usage = !is_decl
            && usage_byte_ranges
                .iter()
                .any(|u| !(line_end <= u.0 || line_start >= u.1));

        let tier = if is_decl {
            LineTier::Declaration
        } else if is_usage {
            LineTier::Usage
        } else if is_byte_in_user_region(line_start, regions) {
            LineTier::Normal
        } else {
            LineTier::Generated
        };

        rendered_lines.push(RenderedLine {
            line_number,
            text,
            tier,
        });

        i = end_with_newline;
        line_number += 1;
    }

    rendered_lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::marker_scan::scan_source;
    use tempfile::tempdir;

    #[test]
    fn test_small_fixture_all_four_tiers() {
        let source = "/* USER CODE BEGIN Header */\n * Some header comment\n/* USER CODE END Header */\nint unused = 0;\n/* USER CODE BEGIN PV */\nint counter = 0;\n/* USER CODE END PV */\n\nvoid step(void) {\n/* USER CODE BEGIN 0 */\n    counter = 1;\n/* USER CODE END 0 */\n}\n";
        let path = Path::new("main.c");
        let regions = scan_source(path, source).unwrap();

        let decl_range = (
            source.find("int counter = 0;").unwrap(),
            source.find("int counter = 0;").unwrap() + "int counter = 0;".len(),
        );

        let usage_idx = source.find("counter = 1;").unwrap();
        let usage_range = (usage_idx, usage_idx + "counter".len());

        let model = build_source_render_model_from_str(
            source,
            &regions,
            decl_range,
            &[usage_range],
        );

        // Line 4: "int unused = 0;" -> Generated (outside user region)
        let line4 = model.iter().find(|l| l.line_number == 4).unwrap();
        assert_eq!(line4.text, "int unused = 0;");
        assert_eq!(line4.tier, LineTier::Generated);

        // Line 6: "int counter = 0;" -> Declaration
        let line6 = model.iter().find(|l| l.line_number == 6).unwrap();
        assert_eq!(line6.text, "int counter = 0;");
        assert_eq!(line6.tier, LineTier::Declaration);

        // Line 11: "    counter = 1;" -> Usage
        let line11 = model.iter().find(|l| l.line_number == 11).unwrap();
        assert_eq!(line11.text, "    counter = 1;");
        assert_eq!(line11.tier, LineTier::Usage);

        // Line 2: " * Some header comment" inside Header region -> Normal
        let line2 = model.iter().find(|l| l.line_number == 2).unwrap();
        assert_eq!(line2.text, " * Some header comment");
        assert_eq!(line2.tier, LineTier::Normal);
    }


    #[test]
    fn test_crlf_fixture_line_splitting() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("main.c");
        let content = "/* USER CODE BEGIN PV */\r\nint val = 42;\r\n/* USER CODE END PV */\r\n\r\nval++;\r\n";
        fs::write(&file_path, content).unwrap();

        let path = Path::new("main.c");
        let regions = scan_source(path, content).unwrap();

        let decl_start = content.find("int val = 42;").unwrap();
        let decl_range = (decl_start, decl_start + "int val = 42;".len());

        let usage_start = content.find("val++").unwrap();
        let usage_range = (usage_start, usage_start + "val".len());

        let model = build_source_render_model(&file_path, &regions, decl_range, &[usage_range]).unwrap();

        assert_eq!(model.len(), 5);

        // Check line text has no trailing \r
        assert_eq!(model[0].text, "/* USER CODE BEGIN PV */");
        assert_eq!(model[1].text, "int val = 42;");
        assert_eq!(model[1].tier, LineTier::Declaration);
        assert_eq!(model[4].text, "val++;");
        assert_eq!(model[4].tier, LineTier::Usage);
    }
}
