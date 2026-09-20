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
}
