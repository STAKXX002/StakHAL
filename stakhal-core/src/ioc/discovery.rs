use std::fs;
use std::path::{Path, PathBuf};

#[derive(thiserror::Error, Debug)]
pub enum DiscoveryError {
    #[error("no .ioc file found in {0}")]
    NoIocFound(PathBuf),
    #[error("multiple .ioc files found in {0}, expected exactly one: {1:?}")]
    MultipleIocFound(PathBuf, Vec<PathBuf>),
    #[error("no main.c found — checked: {0:?}")]
    NoMainCFound(Vec<PathBuf>),
}

pub fn discover_project_files(project_dir: &Path) -> Result<(PathBuf, PathBuf), DiscoveryError> {
    let read_dir = fs::read_dir(project_dir)
        .map_err(|_| DiscoveryError::NoIocFound(project_dir.to_path_buf()))?;

    let mut ioc_files = Vec::new();

    for entry in read_dir.flatten() {
        let path = entry.path();
        if path.is_file() && path.extension().map_or(false, |ext| ext == "ioc") {
            ioc_files.push(path);
        }
    }

    ioc_files.sort();

    let ioc_path = match ioc_files.len() {
        0 => return Err(DiscoveryError::NoIocFound(project_dir.to_path_buf())),
        1 => ioc_files.remove(0),
        _ => return Err(DiscoveryError::MultipleIocFound(project_dir.to_path_buf(), ioc_files)),
    };

    let candidate1 = project_dir.join("Core").join("Src").join("main.c");
    let candidate2 = project_dir.join("Src").join("main.c");

    let main_c_path = if candidate1.exists() && candidate1.is_file() {
        candidate1
    } else if candidate2.exists() && candidate2.is_file() {
        candidate2
    } else {
        return Err(DiscoveryError::NoMainCFound(vec![candidate1, candidate2]));
    };

    Ok((ioc_path, main_c_path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_discover_valid_core_src_layout() {
        let dir = tempdir().unwrap();
        let ioc = dir.path().join("test.ioc");
        let core_src = dir.path().join("Core").join("Src");
        let main_c = core_src.join("main.c");

        fs::create_dir_all(&core_src).unwrap();
        fs::write(&ioc, "Mcu.Family=STM32F4").unwrap();
        fs::write(&main_c, "int main() {}").unwrap();

        let (discovered_ioc, discovered_main_c) =
            discover_project_files(dir.path()).expect("discovery should succeed");

        assert_eq!(discovered_ioc, ioc);
        assert_eq!(discovered_main_c, main_c);
    }

    #[test]
    fn test_discover_no_ioc_found() {
        let dir = tempdir().unwrap();
        let core_src = dir.path().join("Core").join("Src");
        fs::create_dir_all(&core_src).unwrap();
        fs::write(core_src.join("main.c"), "int main() {}").unwrap();

        let res = discover_project_files(dir.path());
        assert!(matches!(res, Err(DiscoveryError::NoIocFound(_))));
    }

    #[test]
    fn test_discover_multiple_ioc_found() {
        let dir = tempdir().unwrap();
        let ioc1 = dir.path().join("a.ioc");
        let ioc2 = dir.path().join("b.ioc");
        fs::write(&ioc1, "Mcu.Family=STM32F4").unwrap();
        fs::write(&ioc2, "Mcu.Family=STM32F4").unwrap();

        let res = discover_project_files(dir.path());
        if let Err(DiscoveryError::MultipleIocFound(path, files)) = res {
            assert_eq!(path, dir.path());
            assert_eq!(files, vec![ioc1, ioc2]);
        } else {
            panic!("Expected MultipleIocFound, got {:?}", res);
        }
    }

    #[test]
    fn test_discover_no_main_c_found() {
        let dir = tempdir().unwrap();
        let ioc = dir.path().join("test.ioc");
        fs::write(&ioc, "Mcu.Family=STM32F4").unwrap();

        let res = discover_project_files(dir.path());
        if let Err(DiscoveryError::NoMainCFound(candidates)) = res {
            assert_eq!(candidates.len(), 2);
            assert_eq!(candidates[0], dir.path().join("Core").join("Src").join("main.c"));
            assert_eq!(candidates[1], dir.path().join("Src").join("main.c"));
        } else {
            panic!("Expected NoMainCFound, got {:?}", res);
        }
    }

    #[test]
    fn test_discover_valid_legacy_src_layout() {
        let dir = tempdir().unwrap();
        let ioc = dir.path().join("legacy.ioc");
        let src = dir.path().join("Src");
        let main_c = src.join("main.c");

        fs::create_dir_all(&src).unwrap();
        fs::write(&ioc, "Mcu.Family=STM32F4").unwrap();
        fs::write(&main_c, "int main() {}").unwrap();

        let (discovered_ioc, discovered_main_c) =
            discover_project_files(dir.path()).expect("discovery should succeed on legacy layout");

        assert_eq!(discovered_ioc, ioc);
        assert_eq!(discovered_main_c, main_c);
    }
}
