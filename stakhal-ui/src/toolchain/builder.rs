use std::path::{Path, PathBuf};
use std::process::Command;

use crate::toolchain::makefile::{find_bin_files_in_dir, parse_makefile_variables, ArtifactResolution};
use crate::toolchain::runner::{get_make_jobs_flag, is_executable_on_path};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildSystem {
    Makefile {
        makefile_path: PathBuf,
        target: Option<String>,
        build_dir: PathBuf,
    },
    CMake {
        cmake_file: PathBuf,
        build_dir: PathBuf,
        target: Option<String>,
    },
    Ninja {
        ninja_file: PathBuf,
        build_dir: PathBuf,
    },
}

impl BuildSystem {
    pub fn display_name(&self) -> &'static str {
        match self {
            BuildSystem::Makefile { .. } => "Makefile",
            BuildSystem::CMake { .. } => "CMake",
            BuildSystem::Ninja { .. } => "Ninja",
        }
    }

    pub fn target_name(&self) -> Option<&str> {
        match self {
            BuildSystem::Makefile { target, .. } => target.as_deref(),
            BuildSystem::CMake { target, .. } => target.as_deref(),
            BuildSystem::Ninja { .. } => None,
        }
    }

    pub fn build_directory(&self) -> &Path {
        match self {
            BuildSystem::Makefile { build_dir, .. } => build_dir,
            BuildSystem::CMake { build_dir, .. } => build_dir,
            BuildSystem::Ninja { build_dir, .. } => build_dir,
        }
    }
}

/// Detect the build system configured in a project directory.
/// Priority:
/// 1. Makefile / makefile
/// 2. CMakeLists.txt
/// 3. build.ninja
pub fn detect_build_system(project_dir: &Path) -> Option<BuildSystem> {
    if !project_dir.is_dir() {
        return None;
    }

    // 1. Check for Makefile
    let makefile_candidate = if project_dir.join("Makefile").is_file() {
        Some(project_dir.join("Makefile"))
    } else if project_dir.join("makefile").is_file() {
        Some(project_dir.join("makefile"))
    } else {
        None
    };

    if let Some(makefile_path) = makefile_candidate {
        let (target, build_dir_name) = if let Ok(content) = std::fs::read_to_string(&makefile_path) {
            parse_makefile_variables(&content)
        } else {
            (None, "build".to_string())
        };
        let build_dir = project_dir.join(build_dir_name);
        return Some(BuildSystem::Makefile {
            makefile_path,
            target,
            build_dir,
        });
    }

    // 2. Check for CMakeLists.txt
    let cmake_file = project_dir.join("CMakeLists.txt");
    if cmake_file.is_file() {
        let target = parse_cmake_target_name(&cmake_file);
        let build_dir = resolve_cmake_build_dir(project_dir);
        return Some(BuildSystem::CMake {
            cmake_file,
            build_dir,
            target,
        });
    }

    // 3. Check for standalone Ninja
    let root_ninja = project_dir.join("build.ninja");
    if root_ninja.is_file() {
        return Some(BuildSystem::Ninja {
            ninja_file: root_ninja,
            build_dir: project_dir.to_path_buf(),
        });
    }

    let build_ninja = project_dir.join("build").join("build.ninja");
    if build_ninja.is_file() {
        return Some(BuildSystem::Ninja {
            ninja_file: build_ninja,
            build_dir: project_dir.join("build"),
        });
    }

    None
}

/// Parse target name from a CMakeLists.txt file.
pub fn parse_cmake_target_name(cmake_file: &Path) -> Option<String> {
    let content = std::fs::read_to_string(cmake_file).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }

        // Check `set(CMAKE_PROJECT_NAME <name>)`
        if let Some(rest) = trimmed.strip_prefix("set(") {
            let inner = rest.trim_end_matches(')').trim();
            let mut parts = inner.split_whitespace();
            if let (Some(var), Some(val)) = (parts.next(), parts.next()) {
                if var == "CMAKE_PROJECT_NAME" {
                    return Some(val.trim_matches('"').to_string());
                }
            }
        }

        // Check `project(<name> ...)`
        if let Some(rest) = trimmed.strip_prefix("project(") {
            let inner = rest.trim_end_matches(')').trim();
            if let Some(proj_name) = inner.split_whitespace().next() {
                let name = proj_name.trim_matches('"');
                if !name.starts_with('$') && !name.is_empty() {
                    return Some(name.to_string());
                }
            }
        }

        // Check `add_executable(<name> ...)`
        if let Some(rest) = trimmed.strip_prefix("add_executable(") {
            let inner = rest.trim_end_matches(')').trim();
            if let Some(exe_name) = inner.split_whitespace().next() {
                let name = exe_name.trim_matches('"');
                if !name.starts_with('$') && !name.is_empty() {
                    return Some(name.to_string());
                }
            }
        }
    }
    None
}

/// Resolve the configured build directory for CMake.
/// Checks `build/Debug`, `build/Release`, `build`, etc.
pub fn resolve_cmake_build_dir(project_dir: &Path) -> PathBuf {
    let candidates = [
        project_dir.join("build").join("Debug"),
        project_dir.join("build").join("Release"),
        project_dir.join("build"),
        project_dir.join("cmake-build-debug"),
        project_dir.join("cmake-build-release"),
    ];

    for c in &candidates {
        if c.join("build.ninja").is_file() || c.join("CMakeCache.txt").is_file() {
            return c.clone();
        }
    }

    // Default to build/Debug if build directory exists or build
    if project_dir.join("build").is_dir() {
        if project_dir.join("build").join("Debug").is_dir() {
            return project_dir.join("build").join("Debug");
        }
        return project_dir.join("build");
    }

    project_dir.join("build")
}

/// Generate the executable command, args, and execution directory for the build.
pub fn get_build_command(
    build_sys: &BuildSystem,
    project_dir: &Path,
) -> Result<(String, Vec<String>, PathBuf), String> {
    match build_sys {
        BuildSystem::Makefile { .. } => {
            if !is_executable_on_path("make") {
                return Err("'make' executable not found on PATH. Please install build-essential.".to_string());
            }
            let jobs_flag = get_make_jobs_flag();
            Ok(("make".to_string(), vec![jobs_flag], project_dir.to_path_buf()))
        }
        BuildSystem::CMake { build_dir, .. } => {
            if is_executable_on_path("cmake") {
                let rel_or_abs = if let Ok(rel) = build_dir.strip_prefix(project_dir) {
                    rel.to_path_buf()
                } else {
                    build_dir.clone()
                };
                Ok((
                    "cmake".to_string(),
                    vec!["--build".to_string(), rel_or_abs.display().to_string()],
                    project_dir.to_path_buf(),
                ))
            } else if is_executable_on_path("ninja") && build_dir.join("build.ninja").is_file() {
                Ok((
                    "ninja".to_string(),
                    vec!["-C".to_string(), build_dir.display().to_string()],
                    project_dir.to_path_buf(),
                ))
            } else {
                Err("'cmake' executable not found on PATH. Please install cmake.".to_string())
            }
        }
        BuildSystem::Ninja { build_dir, .. } => {
            if !is_executable_on_path("ninja") {
                return Err("'ninja' executable not found on PATH. Please install ninja-build.".to_string());
            }
            Ok((
                "ninja".to_string(),
                vec!["-C".to_string(), build_dir.display().to_string()],
                project_dir.to_path_buf(),
            ))
        }
    }
}

/// Convert an `.elf` file to `.bin` using `arm-none-eabi-objcopy` (or system `objcopy`).
pub fn convert_elf_to_bin(elf_path: &Path, bin_path: &Path) -> Result<(), String> {
    let objcopy_tool = if is_executable_on_path("arm-none-eabi-objcopy") {
        "arm-none-eabi-objcopy"
    } else if is_executable_on_path("objcopy") {
        "objcopy"
    } else {
        return Err("Neither 'arm-none-eabi-objcopy' nor 'objcopy' is found on PATH.".to_string());
    };

    let status = Command::new(objcopy_tool)
        .arg("-O")
        .arg("binary")
        .arg(elf_path)
        .arg(bin_path)
        .status()
        .map_err(|e| format!("Failed to execute '{}': {}", objcopy_tool, e))?;

    if status.success() && bin_path.is_file() {
        Ok(())
    } else {
        Err(format!("'{}' exited with error code {:?}", objcopy_tool, status.code()))
    }
}

/// Find all `.elf` files in the given directory (non-recursive).
pub fn find_elf_files_in_dir(dir: &Path) -> Vec<PathBuf> {
    let mut elfs = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext.eq_ignore_ascii_case("elf") {
                        elfs.push(path);
                    }
                }
            }
        }
    }
    elfs.sort();
    elfs
}

/// Resolve the build artifact across any build system, automatically converting
/// `.elf` files to `.bin` if no `.bin` currently exists.
pub fn resolve_artifact_for_build_system(
    project_dir: &Path,
    build_sys: &BuildSystem,
) -> ArtifactResolution {
    let build_dir = build_sys.build_directory();

    // 1. If target name is known, check for target.bin directly
    if let Some(target) = build_sys.target_name() {
        let expected_bin = build_dir.join(format!("{}.bin", target));
        if expected_bin.is_file() {
            return ArtifactResolution::Exact(expected_bin);
        }

        // Check if target.elf exists, and convert it
        let expected_elf = build_dir.join(format!("{}.elf", target));
        if expected_elf.is_file() {
            if convert_elf_to_bin(&expected_elf, &expected_bin).is_ok() {
                return ArtifactResolution::Exact(expected_bin);
            }
        }
    }

    // 2. Search build_dir for existing .bin files
    let bin_candidates = find_bin_files_in_dir(build_dir);
    if bin_candidates.len() == 1 {
        return ArtifactResolution::Exact(bin_candidates.into_iter().next().unwrap());
    } else if bin_candidates.len() > 1 {
        return ArtifactResolution::MultipleCandidates(bin_candidates);
    }

    // 3. Search build_dir for .elf files to auto-convert
    let elf_candidates = find_elf_files_in_dir(build_dir);
    if elf_candidates.len() == 1 {
        let elf = elf_candidates.into_iter().next().unwrap();
        let bin = elf.with_extension("bin");
        if convert_elf_to_bin(&elf, &bin).is_ok() {
            return ArtifactResolution::Exact(bin);
        }
    } else if elf_candidates.len() > 1 {
        let mut converted_bins = Vec::new();
        for elf in &elf_candidates {
            let bin = elf.with_extension("bin");
            if bin.is_file() || convert_elf_to_bin(elf, &bin).is_ok() {
                converted_bins.push(bin);
            }
        }
        if !converted_bins.is_empty() {
            return ArtifactResolution::MultipleCandidates(converted_bins);
        }
    }

    // 4. Fallback: check project_dir/build if different
    let default_build = project_dir.join("build");
    if default_build != build_dir && default_build.is_dir() {
        let fallback_bins = find_bin_files_in_dir(&default_build);
        if fallback_bins.len() == 1 {
            return ArtifactResolution::Exact(fallback_bins.into_iter().next().unwrap());
        }
    }

    let fallback_target = build_sys.target_name().unwrap_or("firmware");
    ArtifactResolution::NoneFound(build_dir.join(format!("{}.bin", fallback_target)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cmake_target_name_set_project() {
        let cmake_content = r#"
cmake_minimum_required(VERSION 3.22)
set(CMAKE_PROJECT_NAME MyTestMcu)
project(${CMAKE_PROJECT_NAME})
"#;
        let temp_dir = std::env::temp_dir().join("stakhal_test_cmake_1");
        let _ = std::fs::create_dir_all(&temp_dir);
        let file_path = temp_dir.join("CMakeLists.txt");
        std::fs::write(&file_path, cmake_content).unwrap();

        let target = parse_cmake_target_name(&file_path);
        assert_eq!(target.as_deref(), Some("MyTestMcu"));
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_parse_cmake_target_name_direct_project() {
        let cmake_content = r#"
cmake_minimum_required(VERSION 3.22)
project(DockingFirmware C ASM)
"#;
        let temp_dir = std::env::temp_dir().join("stakhal_test_cmake_2");
        let _ = std::fs::create_dir_all(&temp_dir);
        let file_path = temp_dir.join("CMakeLists.txt");
        std::fs::write(&file_path, cmake_content).unwrap();

        let target = parse_cmake_target_name(&file_path);
        assert_eq!(target.as_deref(), Some("DockingFirmware"));
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_detect_cmake_project_with_debug_preset() {
        let temp_dir = std::env::temp_dir().join("stakhal_test_cmake_preset");
        let debug_dir = temp_dir.join("build").join("Debug");
        let _ = std::fs::create_dir_all(&debug_dir);
        std::fs::write(temp_dir.join("CMakeLists.txt"), "project(AA_NS_STM_V1)\n").unwrap();
        std::fs::write(debug_dir.join("build.ninja"), "# ninja file\n").unwrap();

        let build_sys = detect_build_system(&temp_dir).expect("CMake project should be detected");
        match build_sys {
            BuildSystem::CMake { build_dir, target, .. } => {
                assert_eq!(build_dir, debug_dir);
                assert_eq!(target.as_deref(), Some("AA_NS_STM_V1"));
            }
            _ => panic!("Expected CMake build system"),
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_detect_makefile_project() {
        let temp_dir = std::env::temp_dir().join("stakhal_test_makefile_detect");
        let _ = std::fs::create_dir_all(&temp_dir);
        std::fs::write(temp_dir.join("Makefile"), "TARGET = MyFirmware\nBUILD_DIR = build\n").unwrap();

        let build_sys = detect_build_system(&temp_dir).expect("Makefile project should be detected");
        match build_sys {
            BuildSystem::Makefile { target, build_dir, .. } => {
                assert_eq!(target.as_deref(), Some("MyFirmware"));
                assert_eq!(build_dir, temp_dir.join("build"));
            }
            _ => panic!("Expected Makefile build system"),
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_get_build_command_cmake() {
        let temp_dir = std::env::temp_dir().join("stakhal_test_cmake_cmd");
        let debug_dir = temp_dir.join("build").join("Debug");
        let _ = std::fs::create_dir_all(&debug_dir);

        let build_sys = BuildSystem::CMake {
            cmake_file: temp_dir.join("CMakeLists.txt"),
            build_dir: debug_dir.clone(),
            target: Some("TestApp".to_string()),
        };

        let res = get_build_command(&build_sys, &temp_dir);
        assert!(res.is_ok());
        let (cmd, args, exec_dir) = res.unwrap();
        assert_eq!(cmd, "cmake");
        assert_eq!(args, vec!["--build", "build/Debug"]);
        assert_eq!(exec_dir, temp_dir);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_resolve_artifact_cmake_auto_convert_elf() {
        let temp_dir = std::env::temp_dir().join("stakhal_test_artifact_convert");
        let debug_dir = temp_dir.join("build").join("Debug");
        let _ = std::fs::create_dir_all(&debug_dir);

        let elf_path = debug_dir.join("TestApp.elf");
        // Create an empty dummy file for testing resolution fallback when conversion tool fails or test mode
        std::fs::write(&elf_path, b"dummy elf").unwrap();

        let build_sys = BuildSystem::CMake {
            cmake_file: temp_dir.join("CMakeLists.txt"),
            build_dir: debug_dir.clone(),
            target: Some("TestApp".to_string()),
        };

        // If bin already exists, it resolves directly
        let expected_bin = debug_dir.join("TestApp.bin");
        std::fs::write(&expected_bin, b"firmware binary").unwrap();

        let resolution = resolve_artifact_for_build_system(&temp_dir, &build_sys);
        match resolution {
            ArtifactResolution::Exact(p) => assert_eq!(p, expected_bin),
            other => panic!("Expected Exact resolution, got {:?}", other),
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_detect_real_aa_ns_stm_v1_if_present() {
        let real_path = PathBuf::from("/home/stakxx002/AA_NS_STM_V1");
        if real_path.is_dir() {
            let detected = detect_build_system(&real_path);
            assert!(detected.is_some(), "AA_NS_STM_V1 should be detected as a CMake project");
            match detected.unwrap() {
                BuildSystem::CMake { target, build_dir, .. } => {
                    assert_eq!(target.as_deref(), Some("AA_NS_STM_V1"));
                    assert!(build_dir.ends_with("build/Debug") || build_dir.ends_with("build"));
                }
                other => panic!("Expected CMake build system for AA_NS_STM_V1, got {:?}", other),
            }
        }
    }
}
