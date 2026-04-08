use std::{fs, path::Path};

use crate::engine::{DatasetKind, ExecutionError};

pub fn detect_dataset_kind(path: &Path) -> Result<DatasetKind, ExecutionError> {
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase());

    match extension.as_deref() {
        Some("nc") => Ok(DatasetKind::Netcdf),
        Some("tif") | Some("tiff") => Ok(DatasetKind::Geotiff),
        _ => Err(ExecutionError::UnsupportedDatasetType(
            path.display().to_string(),
        )),
    }
}

pub fn validate_local_file(path: &Path) -> Result<(), ExecutionError> {
    let metadata =
        fs::metadata(path).map_err(|_| ExecutionError::FileNotFound(path.display().to_string()))?;

    if !metadata.is_file() {
        return Err(ExecutionError::InvalidFile(path.display().to_string()));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::{detect_dataset_kind, validate_local_file};
    use crate::engine::{DatasetKind, ExecutionError};

    #[test]
    fn detect_dataset_kind_supports_known_extensions() {
        assert!(matches!(
            detect_dataset_kind(std::path::Path::new("sample.nc")).expect("netcdf kind"),
            DatasetKind::Netcdf
        ));
        assert!(matches!(
            detect_dataset_kind(std::path::Path::new("sample.tif")).expect("geotiff kind"),
            DatasetKind::Geotiff
        ));
        assert!(matches!(
            detect_dataset_kind(std::path::Path::new("sample.TIFF")).expect("geotiff kind"),
            DatasetKind::Geotiff
        ));
    }

    #[test]
    fn detect_dataset_kind_rejects_unknown_extension() {
        let error =
            detect_dataset_kind(std::path::Path::new("sample.txt")).expect_err("unsupported");
        assert!(matches!(error, ExecutionError::UnsupportedDatasetType(_)));
    }

    #[test]
    fn validate_local_file_accepts_regular_file_and_rejects_directory() {
        let temp_dir = TempDir::new().expect("temp dir");
        let file = temp_dir.path().join("sample.nc");
        fs::write(&file, "data").expect("write file");

        validate_local_file(&file).expect("regular file should validate");

        let error = validate_local_file(temp_dir.path()).expect_err("directory should fail");
        assert!(matches!(error, ExecutionError::InvalidFile(_)));
    }
}
