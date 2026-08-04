use crate::source::marker_scan::{scan_file, ScanError, UserRegion};
use std::fs;
use std::io::Write;
use std::path::Path;

#[derive(thiserror::Error, Debug)]
pub enum WritebackError {
    #[error("region is stale: file has changed since this region was scanned")]
    StaleRegion,
    #[error("byte range {0:?} is invalid for a file of length {1}")]
    InvalidRange((usize, usize), usize),
    #[error("target region tag '{0}' not found in current scan of file")]
    RegionNotFound(String),
    #[error(transparent)]
    ScanError(#[from] ScanError),
    #[error("I/O error: {0}")]
    IoError(String),
}

pub fn write_region(
    path: &Path,
    region: &UserRegion,
    new_content: &str,
) -> Result<(), WritebackError> {
    let fresh_regions = scan_file(path)?;
    let fresh_region = fresh_regions
        .iter()
        .find(|r| r.tag == region.tag)
        .ok_or_else(|| WritebackError::RegionNotFound(region.tag.clone()))?;

    if fresh_region.byte_range != region.byte_range {
        return Err(WritebackError::StaleRegion);
    }

    let content_bytes = fs::read(path).map_err(|e| WritebackError::IoError(e.to_string()))?;
    let (start_byte, end_byte) = region.byte_range;

    if start_byte > end_byte || end_byte > content_bytes.len() {
        return Err(WritebackError::InvalidRange(
            region.byte_range,
            content_bytes.len(),
        ));
    }

    let mut new_bytes = Vec::with_capacity(
        start_byte + new_content.len() + (content_bytes.len() - end_byte),
    );
    new_bytes.extend_from_slice(&content_bytes[..start_byte]);
    new_bytes.extend_from_slice(new_content.as_bytes());
    new_bytes.extend_from_slice(&content_bytes[end_byte..]);

    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("stakhal_tmp");

    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp_path = parent.join(format!(".{}.tmp.{}", file_name, nanos));

    let write_result = (|| -> Result<(), WritebackError> {
        let mut tmp_file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp_path)
            .map_err(|e| WritebackError::IoError(e.to_string()))?;

        tmp_file
            .write_all(&new_bytes)
            .map_err(|e| WritebackError::IoError(e.to_string()))?;
        tmp_file
            .flush()
            .map_err(|e| WritebackError::IoError(e.to_string()))?;

        fs::rename(&tmp_path, path).map_err(|e| WritebackError::IoError(e.to_string()))?;
        Ok(())
    })();

    if write_result.is_err() {
        let _ = fs::remove_file(&tmp_path);
    }

    write_result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::marker_scan::scan_file;
    use tempfile::tempdir;

    #[test]
    fn test_successful_write_region() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("main.c");
        let initial_content = r#"/* USER CODE BEGIN PV */
int old_var = 0;
/* USER CODE END PV */
"#;
        fs::write(&file_path, initial_content).unwrap();

        let regions = scan_file(&file_path).unwrap();
        assert_eq!(regions.len(), 1);
        let region = &regions[0];

        let replacement = "\nint new_var = 42;\n";
        write_region(&file_path, region, replacement).unwrap();

        let updated_file_content = fs::read_to_string(&file_path).unwrap();
        let expected_content = r#"/* USER CODE BEGIN PV */
int new_var = 42;
/* USER CODE END PV */
"#;
        assert_eq!(updated_file_content, expected_content);

        let rescan_regions = scan_file(&file_path).unwrap();
        assert_eq!(rescan_regions.len(), 1);
        let new_range = rescan_regions[0].byte_range;
        assert_eq!(&updated_file_content[new_range.0..new_range.1], replacement);
    }

    #[test]
    fn test_stale_region_detection() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("main.c");
        let initial_content = r#"/* USER CODE BEGIN PV */
int old_var = 0;
/* USER CODE END PV */
"#;
        fs::write(&file_path, initial_content).unwrap();

        let regions = scan_file(&file_path).unwrap();
        let region = regions[0].clone();

        let modified_content = r#"// Extra line inserted at top
/* USER CODE BEGIN PV */
int old_var = 0;
/* USER CODE END PV */
"#;
        fs::write(&file_path, modified_content).unwrap();

        let res = write_region(&file_path, &region, "int new_var = 1;");
        assert!(matches!(res, Err(WritebackError::StaleRegion)));
        assert_eq!(fs::read_to_string(&file_path).unwrap(), modified_content);
    }

    #[test]
    fn test_region_not_found() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("main.c");
        let initial_content = r#"/* USER CODE BEGIN PV */
int old_var = 0;
/* USER CODE END PV */
"#;
        fs::write(&file_path, initial_content).unwrap();

        let mut dummy_region = scan_file(&file_path).unwrap()[0].clone();
        dummy_region.tag = "NONEXISTENT".to_string();

        let res = write_region(&file_path, &dummy_region, "int new_var = 1;");
        assert!(matches!(res, Err(WritebackError::RegionNotFound(tag)) if tag == "NONEXISTENT"));
    }

    #[test]
    fn test_no_leftover_temp_files() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("main.c");
        let initial_content = r#"/* USER CODE BEGIN 0 */
/* USER CODE END 0 */
"#;
        fs::write(&file_path, initial_content).unwrap();

        let regions = scan_file(&file_path).unwrap();
        write_region(&file_path, &regions[0], "\n// User code here\n").unwrap();

        let dir_entries: Vec<String> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok().map(|de| de.file_name().to_string_lossy().to_string()))
            .collect();

        assert_eq!(dir_entries, vec!["main.c"]);
    }
}
