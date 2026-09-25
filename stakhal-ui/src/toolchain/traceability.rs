use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Check if a directory is inside a Git repository.
pub fn is_git_repository(project_dir: &Path) -> bool {
    let output = Command::new("git")
        .arg("-C")
        .arg(project_dir)
        .arg("rev-parse")
        .arg("--is-inside-work-tree")
        .output();

    match output {
        Ok(out) => out.status.success() && String::from_utf8_lossy(&out.stdout).trim() == "true",
        Err(_) => false,
    }
}

/// Retrieve the current short commit hash, appending "-dirty" if the working tree has changes.
/// Returns None if the directory is not inside a git repository or git fails.
pub fn get_git_build_hash(project_dir: &Path) -> Option<String> {
    if !is_git_repository(project_dir) {
        return None;
    }

    let rev_output = Command::new("git")
        .arg("-C")
        .arg(project_dir)
        .arg("rev-parse")
        .arg("--short")
        .arg("HEAD")
        .output()
        .ok()?;

    if !rev_output.status.success() {
        return None;
    }

    let hash = String::from_utf8_lossy(&rev_output.stdout).trim().to_string();
    if hash.is_empty() {
        return None;
    }

    let status_output = Command::new("git")
        .arg("-C")
        .arg(project_dir)
        .arg("status")
        .arg("--porcelain")
        .output()
        .ok()?;

    let status_str = String::from_utf8_lossy(&status_output.stdout);
    // Filter out stakhal_build_info.h itself if untracked to avoid self-dirtying
    let is_dirty = status_str.lines().any(|line| {
        let trimmed = line.trim();
        !trimmed.is_empty() && !trimmed.contains("stakhal_build_info.h")
    });

    if is_dirty {
        Some(format!("{}-dirty", hash))
    } else {
        Some(hash)
    }
}

/// Locate the include directory for the project (e.g. Core/Inc or Inc).
pub fn resolve_include_directory(project_dir: &Path) -> PathBuf {
    if project_dir.join("Core").join("Inc").exists() {
        project_dir.join("Core").join("Inc")
    } else if project_dir.join("Inc").exists() {
        project_dir.join("Inc")
    } else {
        project_dir.join("Core").join("Inc")
    }
}

/// Ensure that Core/Inc/stakhal_build_info.h is present in the project's .gitignore.
/// Returns Ok(true) if .gitignore was modified, Ok(false) if already present or no .gitignore.
pub fn ensure_gitignore_ignores_build_info(project_dir: &Path) -> Result<bool, std::io::Error> {
    let gitignore_path = project_dir.join(".gitignore");
    if !gitignore_path.is_file() {
        return Ok(false);
    }

    let content = fs::read_to_string(&gitignore_path)?;
    for line in content.lines() {
        let t = line.trim();
        if t == "Core/Inc/stakhal_build_info.h"
            || t == "stakhal_build_info.h"
            || t == "**/stakhal_build_info.h"
        {
            return Ok(false);
        }
    }

    let mut new_content = content;
    if !new_content.ends_with('\n') && !new_content.is_empty() {
        new_content.push('\n');
    }
    new_content.push_str("\n# StakHAL build traceability (generated)\nCore/Inc/stakhal_build_info.h\n");
    fs::write(&gitignore_path, new_content)?;
    Ok(true)
}

/// Generate the Core/Inc/stakhal_build_info.h header for the project.
/// Overwrites the file if it already exists.
/// Returns the build hash that was written into the header.
pub fn generate_build_info_header(project_dir: &Path) -> Result<String, std::io::Error> {
    let hash = get_git_build_hash(project_dir).unwrap_or_else(|| "unknown".to_string());

    let inc_dir = resolve_include_directory(project_dir);
    fs::create_dir_all(&inc_dir)?;

    let header_path = inc_dir.join("stakhal_build_info.h");
    let content = format!(
        "/* Generated automatically by StakHAL. Do not edit or commit. */\n\
         #ifndef STAKHAL_BUILD_INFO_H\n\
         #define STAKHAL_BUILD_INFO_H\n\n\
         #define STAKHAL_BUILD_HASH \"{}\"\n\n\
         #endif /* STAKHAL_BUILD_INFO_H */\n",
        hash
    );

    fs::write(&header_path, content)?;

    // Ensure .gitignore ignores the generated header if .gitignore exists
    let _ = ensure_gitignore_ignores_build_info(project_dir);

    Ok(hash)
}

/// Check if build traceability symbols are already present in main.c.
pub fn is_traceability_enabled_in_source(main_c_path: &Path) -> bool {
    if !main_c_path.is_file() {
        return false;
    }
    match fs::read_to_string(main_c_path) {
        Ok(content) => {
            content.contains("stakhal_build_info.h") && content.contains("STAKHAL_BUILD_HASH")
        }
        Err(_) => false,
    }
}

/// Generate a diff preview showing what will be inserted into main.c.
pub fn generate_traceability_diff_preview(main_c_path: &Path) -> String {
    let filename = main_c_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("main.c");

    format!(
        "--- {file}\n\
         +++ {file}\n\n\
         /* USER CODE BEGIN Includes */\n\
         +#include \"stakhal_build_info.h\"\n\
         /* USER CODE END Includes */\n\n\
         /* USER CODE BEGIN 2 */\n\
         +  printf(\"STAKHAL_BUILD: %s\\r\\n\", STAKHAL_BUILD_HASH);\n\
         /* USER CODE END 2 */\n",
        file = filename
    )
}

fn detect_region_indentation(content: &str) -> &'static str {
    for line in content.lines().rev() {
        let trimmed = line.trim();
        if !trimmed.is_empty() && !trimmed.starts_with("/*") && !trimmed.starts_with('*') {
            let leading = line.len() - line.trim_start().len();
            if leading >= 4 {
                return "    ";
            } else if leading >= 2 {
                return "  ";
            }
        }
    }
    "  "
}

/// Insert build traceability #include and boot banner printf into main.c.
pub fn insert_traceability_into_source(main_c_path: &Path, project_dir: &Path) -> Result<(), String> {
    if !main_c_path.is_file() {
        return Err(format!("Source file not found: {}", main_c_path.display()));
    }

    if is_traceability_enabled_in_source(main_c_path) {
        let _ = generate_build_info_header(project_dir);
        return Ok(());
    }

    // Verify required USER CODE regions exist before making any edits
    let initial_regions = stakhal_core::source::marker_scan::scan_file(main_c_path)
        .map_err(|e| format!("Failed to parse USER CODE markers in {}: {}", main_c_path.display(), e))?;

    let has_includes = initial_regions.iter().any(|r| r.tag == "Includes");
    let has_post_init = initial_regions.iter().any(|r| r.tag == "2");

    if !has_includes {
        return Err("USER CODE region 'Includes' not found in main.c".to_string());
    }
    if !has_post_init {
        return Err("USER CODE region '2' not found in main.c".to_string());
    }

    // Step 1: Insert into "Includes" if not already present
    let content = fs::read_to_string(main_c_path)
        .map_err(|e| format!("Failed to read {}: {}", main_c_path.display(), e))?;

    let inc_region = initial_regions
        .iter()
        .find(|r| r.tag == "Includes")
        .expect("Includes region verified above");

    let inc_content = &content[inc_region.byte_range.0..inc_region.byte_range.1];
    if !inc_content.contains("stakhal_build_info.h") {
        let mut new_inc = inc_content.to_string();
        if new_inc.is_empty() {
            new_inc.push('\n');
        } else if !new_inc.ends_with('\n') {
            new_inc.push('\n');
        }
        new_inc.push_str("#include \"stakhal_build_info.h\"\n");

        stakhal_core::source::writeback::write_region(main_c_path, inc_region, &new_inc)
            .map_err(|e| format!("Failed to write to Includes region: {}", e))?;
    }

    // Step 2: Re-scan file since byte ranges have shifted after the first write
    let fresh_regions = stakhal_core::source::marker_scan::scan_file(main_c_path)
        .map_err(|e| format!("Failed to re-scan markers in {}: {}", main_c_path.display(), e))?;

    let post_init_region = fresh_regions
        .iter()
        .find(|r| r.tag == "2")
        .ok_or_else(|| "USER CODE region '2' not found after re-scan".to_string())?;

    let fresh_content = fs::read_to_string(main_c_path)
        .map_err(|e| format!("Failed to re-read {}: {}", main_c_path.display(), e))?;

    let post_init_content = &fresh_content[post_init_region.byte_range.0..post_init_region.byte_range.1];
    if !post_init_content.contains("STAKHAL_BUILD_HASH") {
        let indent = detect_region_indentation(post_init_content);
        let trailing_ws = post_init_content
            .chars()
            .rev()
            .take_while(|c| *c == ' ' || *c == '\t')
            .collect::<String>();

        let trimmed_content = &post_init_content[..post_init_content.len() - trailing_ws.len()];
        let mut new_post_init = trimmed_content.to_string();
        if new_post_init.is_empty() {
            new_post_init.push('\n');
        } else if !new_post_init.ends_with('\n') {
            new_post_init.push('\n');
        }
        new_post_init.push_str(&format!(
            "{}printf(\"STAKHAL_BUILD: %s\\r\\n\", STAKHAL_BUILD_HASH);\n{}",
            indent, trailing_ws
        ));

        stakhal_core::source::writeback::write_region(main_c_path, post_init_region, &new_post_init)
            .map_err(|e| format!("Failed to write to USER CODE 2 region: {}", e))?;
    }

    // Step 3: Ensure build info header and .gitignore entry exist
    let _ = generate_build_info_header(project_dir);

    Ok(())
}

/// Parse an incoming serial line for the STAKHAL_BUILD: banner.
/// Returns Some((hash, is_dirty)) if matched.
pub fn parse_build_banner_line(line: &str) -> Option<(String, bool)> {
    let tag = "STAKHAL_BUILD:";
    if let Some(idx) = line.find(tag) {
        let after = &line[idx + tag.len()..];
        let hash = after
            .trim()
            .trim_matches(|c: char| c == '\r' || c == '\n')
            .to_string();
        if !hash.is_empty() {
            let is_dirty = hash.ends_with("-dirty");
            return Some((hash, is_dirty));
        }
    }
    None
}

/// Parse an incoming serial stream line for exact state-name transitions.
/// Matches trimmed line content against known state machine node names.
pub fn parse_state_transition_line(line: &str, known_nodes: &[String]) -> Option<String> {
    for subline in line.split('\n') {
        let trimmed = subline.trim().trim_matches(|c: char| c == '\r' || c == '\n');
        if trimmed.is_empty() {
            continue;
        }
        if let Some(matched) = known_nodes.iter().find(|n| n.as_str() == trimmed) {
            return Some(matched.clone());
        }
    }
    None
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraceabilityStatus {
    /// No captured hash yet (or not connected / not a git repo)
    Unknown,
    /// Flashed from uncommitted working tree near base_hash
    Dirty { base_hash: String },
    /// Matches current HEAD (0 commits behind)
    MatchesWorkingTree { hash: String },
    /// Behind HEAD by count commits
    BehindWorkingTree { hash: String, count: usize },
    /// Captured hash is not an ancestor of current HEAD (different branch or history)
    Diverged { hash: String },
}

/// Compare a captured build hash against project Git repository's HEAD.
pub fn compare_build_hash_to_head(project_dir: &Path, captured_hash: &str) -> TraceabilityStatus {
    let trimmed = captured_hash.trim();
    if trimmed.is_empty() || trimmed == "unknown" {
        return TraceabilityStatus::Unknown;
    }

    if let Some(base) = trimmed.strip_suffix("-dirty") {
        return TraceabilityStatus::Dirty {
            base_hash: base.to_string(),
        };
    }

    if !is_git_repository(project_dir) {
        return TraceabilityStatus::Unknown;
    }

    // Run `git rev-list --count <captured_hash>..HEAD`
    let range = format!("{}..HEAD", trimmed);
    let output = Command::new("git")
        .arg("-C")
        .arg(project_dir)
        .arg("rev-list")
        .arg("--count")
        .arg(&range)
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let count_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if let Ok(count) = count_str.parse::<usize>() {
                if count == 0 {
                    TraceabilityStatus::MatchesWorkingTree {
                        hash: trimmed.to_string(),
                    }
                } else {
                    TraceabilityStatus::BehindWorkingTree {
                        hash: trimmed.to_string(),
                        count,
                    }
                }
            } else {
                TraceabilityStatus::Diverged {
                    hash: trimmed.to_string(),
                }
            }
        }
        _ => TraceabilityStatus::Diverged {
            hash: trimmed.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_non_git_directory_returns_unknown() {
        let temp_dir = std::env::temp_dir().join(format!("stakhal_test_nongit_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        assert!(!is_git_repository(&temp_dir));
        assert_eq!(get_git_build_hash(&temp_dir), None);

        let hash = generate_build_info_header(&temp_dir).unwrap();
        assert_eq!(hash, "unknown");

        let header_path = temp_dir.join("Core/Inc/stakhal_build_info.h");
        assert!(header_path.is_file());
        let content = fs::read_to_string(&header_path).unwrap();
        assert!(content.contains("#define STAKHAL_BUILD_HASH \"unknown\""));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_git_repo_clean_and_dirty_hashes() {
        let temp_dir = std::env::temp_dir().join(format!("stakhal_test_git_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        // Initialize a git repo
        Command::new("git")
            .arg("-C")
            .arg(&temp_dir)
            .arg("init")
            .output()
            .unwrap();

        Command::new("git")
            .arg("-C")
            .arg(&temp_dir)
            .arg("config")
            .arg("user.name")
            .arg("TestUser")
            .output()
            .unwrap();

        Command::new("git")
            .arg("-C")
            .arg(&temp_dir)
            .arg("config")
            .arg("user.email")
            .arg("test@example.com")
            .output()
            .unwrap();

        // Create an initial file and commit
        let dummy_file = temp_dir.join("dummy.txt");
        let mut f = File::create(&dummy_file).unwrap();
        writeln!(f, "initial content").unwrap();

        Command::new("git")
            .arg("-C")
            .arg(&temp_dir)
            .arg("add")
            .arg("dummy.txt")
            .output()
            .unwrap();

        Command::new("git")
            .arg("-C")
            .arg(&temp_dir)
            .arg("commit")
            .arg("-m")
            .arg("initial commit")
            .output()
            .unwrap();

        assert!(is_git_repository(&temp_dir));

        // Test clean state
        let clean_hash = get_git_build_hash(&temp_dir).unwrap();
        assert!(!clean_hash.ends_with("-dirty"));
        assert!(!clean_hash.is_empty());

        let gen_clean = generate_build_info_header(&temp_dir).unwrap();
        assert_eq!(gen_clean, clean_hash);

        // Make working tree dirty
        let mut f2 = std::fs::OpenOptions::new()
            .append(true)
            .open(&dummy_file)
            .unwrap();
        writeln!(f2, "dirty change").unwrap();

        let dirty_hash = get_git_build_hash(&temp_dir).unwrap();
        assert!(dirty_hash.ends_with("-dirty"));
        assert_eq!(dirty_hash, format!("{}-dirty", clean_hash));

        let gen_dirty = generate_build_info_header(&temp_dir).unwrap();
        assert_eq!(gen_dirty, dirty_hash);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_gitignore_append() {
        let temp_dir = std::env::temp_dir().join(format!("stakhal_test_gitignore_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let gitignore = temp_dir.join(".gitignore");
        fs::write(&gitignore, "build/\n").unwrap();

        let added = ensure_gitignore_ignores_build_info(&temp_dir).unwrap();
        assert!(added);

        let content = fs::read_to_string(&gitignore).unwrap();
        assert!(content.contains("Core/Inc/stakhal_build_info.h"));

        // Running again should not duplicate
        let added_again = ensure_gitignore_ignores_build_info(&temp_dir).unwrap();
        assert!(!added_again);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_insert_traceability_into_source_lifecycle() {
        let temp_dir = std::env::temp_dir().join(format!("stakhal_test_trace_src_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(temp_dir.join("Core/Src")).unwrap();
        fs::create_dir_all(temp_dir.join("Core/Inc")).unwrap();

        let main_c_path = temp_dir.join("Core/Src/main.c");
        let initial_main_c = r#"/* USER CODE BEGIN Header */
/* USER CODE END Header */

/* USER CODE BEGIN Includes */
#include <stdio.h>
#include "commands.h"
/* USER CODE END Includes */

int main(void)
{
  HAL_Init();

  /* USER CODE BEGIN 2 */
  commands_init();
  printf("BOOTING\r\n");
  /* USER CODE END 2 */

  while (1)
  {
  }
}
"#;
        fs::write(&main_c_path, initial_main_c).unwrap();

        assert!(!is_traceability_enabled_in_source(&main_c_path));

        let diff_preview = generate_traceability_diff_preview(&main_c_path);
        assert!(diff_preview.contains("#include \"stakhal_build_info.h\""));
        assert!(diff_preview.contains("printf(\"STAKHAL_BUILD: %s\\r\\n\", STAKHAL_BUILD_HASH);"));

        let res = insert_traceability_into_source(&main_c_path, &temp_dir);
        assert!(res.is_ok(), "Insertion failed: {:?}", res);

        assert!(is_traceability_enabled_in_source(&main_c_path));

        let modified_c = fs::read_to_string(&main_c_path).unwrap();
        assert!(modified_c.contains("#include \"stakhal_build_info.h\""));
        assert!(modified_c.contains("printf(\"STAKHAL_BUILD: %s\\r\\n\", STAKHAL_BUILD_HASH);"));

        // Verify generated header
        let header_path = temp_dir.join("Core/Inc/stakhal_build_info.h");
        assert!(header_path.is_file());

        // Test idempotency: calling again should succeed without duplicating lines
        let res2 = insert_traceability_into_source(&main_c_path, &temp_dir);
        assert!(res2.is_ok());

        let modified_again = fs::read_to_string(&main_c_path).unwrap();
        assert_eq!(modified_c, modified_again);

        // Verify counts of inserted lines
        let inc_count = modified_again.matches("#include \"stakhal_build_info.h\"").count();
        let print_count = modified_again.matches("printf(\"STAKHAL_BUILD: %s\\r\\n\", STAKHAL_BUILD_HASH);").count();
        assert_eq!(inc_count, 1);
        assert_eq!(print_count, 1);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_insert_traceability_missing_regions() {
        let temp_dir = std::env::temp_dir().join(format!("stakhal_test_missing_reg_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(temp_dir.join("Core/Src")).unwrap();

        let main_c_path = temp_dir.join("Core/Src/main.c");
        // Missing "Includes" region
        let no_includes = r#"/* USER CODE BEGIN 2 */
printf("BOOT\r\n");
/* USER CODE END 2 */
"#;
        fs::write(&main_c_path, no_includes).unwrap();

        let res = insert_traceability_into_source(&main_c_path, &temp_dir);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Includes"));

        // Missing "2" region
        let no_post_init = r#"/* USER CODE BEGIN Includes */
#include <stdio.h>
/* USER CODE END Includes */
"#;
        fs::write(&main_c_path, no_post_init).unwrap();

        let res2 = insert_traceability_into_source(&main_c_path, &temp_dir);
        assert!(res2.is_err());
        assert!(res2.unwrap_err().contains("2"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_parse_build_banner_line() {
        assert_eq!(
            parse_build_banner_line("STAKHAL_BUILD: a41f5f7\r\n"),
            Some(("a41f5f7".to_string(), false))
        );
        assert_eq!(
            parse_build_banner_line("STAKHAL_BUILD: a41f5f7-dirty\r\n"),
            Some(("a41f5f7-dirty".to_string(), true))
        );
        assert_eq!(
            parse_build_banner_line("STAKHAL_BUILD: unknown\n"),
            Some(("unknown".to_string(), false))
        );
        assert_eq!(
            parse_build_banner_line("[BOOT] STAKHAL_BUILD: b65ad0d\r\n"),
            Some(("b65ad0d".to_string(), false))
        );
        assert_eq!(parse_build_banner_line("READY\r\nCAL REQUIRED\r\n"), None);
        assert_eq!(parse_build_banner_line("STAKHAL_BUILD: \r\n"), None);
        assert_eq!(parse_build_banner_line(""), None);
    }

    #[test]
    fn test_parse_state_transition_line_exact_matching() {
        let known = vec![
            "IDLE".to_string(),
            "CALIBRATING".to_string(),
            "HOLD".to_string(),
            "RETURNED".to_string(),
            "OPENING".to_string(),
            "CLOSING".to_string(),
            "FAULT".to_string(),
        ];

        // Exact matches with CRLF
        assert_eq!(
            parse_state_transition_line("HOLD\r\n", &known),
            Some("HOLD".to_string())
        );
        assert_eq!(
            parse_state_transition_line("RETURNED\r\n", &known),
            Some("RETURNED".to_string())
        );
        assert_eq!(
            parse_state_transition_line("OPENING\n", &known),
            Some("OPENING".to_string())
        );
        assert_eq!(
            parse_state_transition_line("  CLOSING  \r\n", &known),
            Some("CLOSING".to_string())
        );

        // Multi-line burst where one line is a state
        assert_eq!(
            parse_state_transition_line("REC OK\r\nZERO\r\nRETURNED\r\n", &known),
            Some("RETURNED".to_string())
        );

        // Rejection of substring-only matches
        assert_eq!(
            parse_state_transition_line("FAULT: motor jammed\r\n", &known),
            None
        );
        assert_eq!(parse_state_transition_line("CAL OK\r\n", &known), None);
        assert_eq!(parse_state_transition_line("Z1 HIT\r\n", &known), None);
        assert_eq!(
            parse_state_transition_line("READY\r\nCAL REQUIRED\r\n", &known),
            None
        );
        assert_eq!(parse_state_transition_line("GO\r\n", &known), None);
        assert_eq!(parse_state_transition_line("", &known), None);
    }

    #[test]
    fn test_compare_build_hash_to_head() {
        let temp_dir = std::env::temp_dir().join(format!("stakhal_test_cmp_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        // Non-git
        assert_eq!(
            compare_build_hash_to_head(&temp_dir, "unknown"),
            TraceabilityStatus::Unknown
        );
        assert_eq!(
            compare_build_hash_to_head(&temp_dir, "a1b2c3d-dirty"),
            TraceabilityStatus::Dirty {
                base_hash: "a1b2c3d".to_string()
            }
        );
        assert_eq!(
            compare_build_hash_to_head(&temp_dir, "a1b2c3d"),
            TraceabilityStatus::Unknown
        );

        // Init git repo
        Command::new("git").arg("-C").arg(&temp_dir).arg("init").output().unwrap();
        Command::new("git").arg("-C").arg(&temp_dir).arg("config").arg("user.name").arg("Test").output().unwrap();
        Command::new("git").arg("-C").arg(&temp_dir).arg("config").arg("user.email").arg("t@example.com").output().unwrap();

        // Commit 1
        let f1 = temp_dir.join("f1.txt");
        fs::write(&f1, "1").unwrap();
        Command::new("git").arg("-C").arg(&temp_dir).arg("add").arg("f1.txt").output().unwrap();
        Command::new("git").arg("-C").arg(&temp_dir).arg("commit").arg("-m").arg("c1").output().unwrap();
        let c1_out = Command::new("git").arg("-C").arg(&temp_dir).arg("rev-parse").arg("--short").arg("HEAD").output().unwrap();
        let c1 = String::from_utf8_lossy(&c1_out.stdout).trim().to_string();

        // Commit 2
        let f2 = temp_dir.join("f2.txt");
        fs::write(&f2, "2").unwrap();
        Command::new("git").arg("-C").arg(&temp_dir).arg("add").arg("f2.txt").output().unwrap();
        Command::new("git").arg("-C").arg(&temp_dir).arg("commit").arg("-m").arg("c2").output().unwrap();
        let c2_out = Command::new("git").arg("-C").arg(&temp_dir).arg("rev-parse").arg("--short").arg("HEAD").output().unwrap();
        let c2 = String::from_utf8_lossy(&c2_out.stdout).trim().to_string();

        // Commit 3 (HEAD)
        let f3 = temp_dir.join("f3.txt");
        fs::write(&f3, "3").unwrap();
        Command::new("git").arg("-C").arg(&temp_dir).arg("add").arg("f3.txt").output().unwrap();
        Command::new("git").arg("-C").arg(&temp_dir).arg("commit").arg("-m").arg("c3").output().unwrap();
        let c3_out = Command::new("git").arg("-C").arg(&temp_dir).arg("rev-parse").arg("--short").arg("HEAD").output().unwrap();
        let c3 = String::from_utf8_lossy(&c3_out.stdout).trim().to_string();

        // Compare HEAD (c3) -> 0 commits behind
        assert_eq!(
            compare_build_hash_to_head(&temp_dir, &c3),
            TraceabilityStatus::MatchesWorkingTree { hash: c3.clone() }
        );

        // Compare c2 -> 1 commit behind
        assert_eq!(
            compare_build_hash_to_head(&temp_dir, &c2),
            TraceabilityStatus::BehindWorkingTree { hash: c2.clone(), count: 1 }
        );

        // Compare c1 -> 2 commits behind
        assert_eq!(
            compare_build_hash_to_head(&temp_dir, &c1),
            TraceabilityStatus::BehindWorkingTree { hash: c1.clone(), count: 2 }
        );

        // Dirty hash
        assert_eq!(
            compare_build_hash_to_head(&temp_dir, &format!("{}-dirty", c3)),
            TraceabilityStatus::Dirty { base_hash: c3.clone() }
        );

        // Non-existent hash
        assert_eq!(
            compare_build_hash_to_head(&temp_dir, "0000000"),
            TraceabilityStatus::Diverged { hash: "0000000".to_string() }
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
