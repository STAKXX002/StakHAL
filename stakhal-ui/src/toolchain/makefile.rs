use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactResolution {
    Exact(PathBuf),
    MultipleCandidates(Vec<PathBuf>),
    NoneFound(PathBuf),
}

/// Parse TARGET and BUILD_DIR variables from a CubeMX-generated Makefile.
pub fn parse_makefile_variables(makefile_content: &str) -> (Option<String>, String) {
    let mut target = None;
    let mut build_dir = None;

    for line in makefile_content.lines() {
        let trimmed = line.trim();
        // Ignore full comment lines
        if trimmed.starts_with('#') {
            continue;
        }

        // Strip inline comments
        let content = if let Some(idx) = trimmed.find('#') {
            &trimmed[..idx]
        } else {
            trimmed
        };

        if let Some((key, val)) = content.split_once('=') {
            let key = key.trim();
            let val = val.trim();
            if key == "TARGET" && !val.is_empty() && target.is_none() {
                target = Some(val.to_string());
            } else if key == "BUILD_DIR" && !val.is_empty() && build_dir.is_none() {
                build_dir = Some(val.to_string());
            }
        }
    }

    let resolved_build_dir = build_dir.unwrap_or_else(|| "build".to_string());
    (target, resolved_build_dir)
}

/// Resolves the expected .bin output path from project Makefile.
#[allow(dead_code)]
pub fn get_expected_artifact_path(project_dir: &Path) -> Option<PathBuf> {
    let makefile_path = project_dir.join("Makefile");
    let content = std::fs::read_to_string(&makefile_path).ok()?;
    let (target, build_dir) = parse_makefile_variables(&content);
    let target = target?;
    Some(project_dir.join(&build_dir).join(format!("{}.bin", target)))
}

/// Find all `.bin` files in the given directory (non-recursive).
pub fn find_bin_files_in_dir(dir: &Path) -> Vec<PathBuf> {
    let mut bins = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext.eq_ignore_ascii_case("bin") {
                        bins.push(path);
                    }
                }
            }
        }
    }
    bins.sort();
    bins
}

/// Resolve the binary artifact after build:
/// 1. Try the parsed TARGET.bin.
/// 2. If missing or TARGET wasn't found, search BUILD_DIR for .bin files.
/// 3. If exactly 1 candidate, return Exact. If > 1, return MultipleCandidates.
#[allow(dead_code)]
pub fn resolve_build_artifact(project_dir: &Path) -> ArtifactResolution {
    let makefile_path = project_dir.join("Makefile");
    let (target, build_dir_name) = if let Ok(content) = std::fs::read_to_string(&makefile_path) {
        parse_makefile_variables(&content)
    } else {
        (None, "build".to_string())
    };

    let build_dir = project_dir.join(&build_dir_name);

    if let Some(ref tgt) = target {
        let expected = build_dir.join(format!("{}.bin", tgt));
        if expected.is_file() {
            return ArtifactResolution::Exact(expected);
        }
    }

    // Fallback: search build directory for any .bin files
    let candidates = find_bin_files_in_dir(&build_dir);
    match candidates.len() {
        1 => ArtifactResolution::Exact(candidates.into_iter().next().unwrap()),
        n if n > 1 => ArtifactResolution::MultipleCandidates(candidates),
        _ => {
            let fallback_expected = if let Some(tgt) = target {
                build_dir.join(format!("{}.bin", tgt))
            } else {
                build_dir.join("output.bin")
            };
            ArtifactResolution::NoneFound(fallback_expected)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_standard_cubemx_makefile() {
        let content = r#"
######################################
# target
######################################
TARGET = docking_firmware


######################################
# building variables
######################################
# debug build?
DEBUG = 1
# optimization
OPT = -Og


#######################################
# paths
#######################################
# Build path
BUILD_DIR = build
"#;
        let (target, build_dir) = parse_makefile_variables(content);
        assert_eq!(target.as_deref(), Some("docking_firmware"));
        assert_eq!(build_dir, "build");
    }

    #[test]
    fn test_parse_makefile_with_inline_comments_and_custom_dir() {
        let content = r#"
TARGET = my_app # Main application target
BUILD_DIR = out_bin # Directory for build output
"#;
        let (target, build_dir) = parse_makefile_variables(content);
        assert_eq!(target.as_deref(), Some("my_app"));
        assert_eq!(build_dir, "out_bin");
    }

    #[test]
    fn test_parse_makefile_defaults_build_dir_to_build() {
        let content = r#"
TARGET = blinky
"#;
        let (target, build_dir) = parse_makefile_variables(content);
        assert_eq!(target.as_deref(), Some("blinky"));
        assert_eq!(build_dir, "build");
    }

    #[test]
    fn test_resolve_build_artifact_fallback() {
        let temp_dir = std::env::temp_dir().join("stakhal_test_makefile_resolve");
        let _ = std::fs::remove_dir_all(&temp_dir);
        let build_dir = temp_dir.join("build");
        std::fs::create_dir_all(&build_dir).unwrap();

        // 1. None found
        let res = resolve_build_artifact(&temp_dir);
        assert!(matches!(res, ArtifactResolution::NoneFound(_)));

        // 2. Exactly one .bin found
        let bin1 = build_dir.join("firmware.bin");
        std::fs::write(&bin1, b"bin content").unwrap();
        let res = resolve_build_artifact(&temp_dir);
        assert_eq!(res, ArtifactResolution::Exact(bin1.clone()));

        // 3. Multiple candidates found
        let bin2 = build_dir.join("bootloader.bin");
        std::fs::write(&bin2, b"bin content 2").unwrap();
        let res = resolve_build_artifact(&temp_dir);
        if let ArtifactResolution::MultipleCandidates(candidates) = res {
            assert_eq!(candidates.len(), 2);
            assert!(candidates.contains(&bin1));
            assert!(candidates.contains(&bin2));
        } else {
            panic!("Expected MultipleCandidates, got {:?}", res);
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
