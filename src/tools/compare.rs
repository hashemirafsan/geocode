use std::path::Path;

use crate::engine::ExecutionError;

use super::{
    shared::{detect_dataset_kind, mean, validate_local_file},
    types::CompareReport,
};

pub fn compare(
    path_a: &Path,
    path_b: &Path,
    variable: Option<&str>,
) -> Result<CompareReport, ExecutionError> {
    validate_local_file(path_a)?;
    validate_local_file(path_b)?;

    let kind_a = detect_dataset_kind(path_a)?;
    let kind_b = detect_dataset_kind(path_b)?;

    if kind_a != kind_b {
        return Err(ExecutionError::InvalidCompare(
            "compare supports same-type files only".into(),
        ));
    }

    let mean_a = mean(path_a, variable)?;
    let mean_b = mean(path_b, variable)?;

    Ok(CompareReport {
        file_a: path_a.to_path_buf(),
        file_b: path_b.to_path_buf(),
        kind: mean_a.kind,
        variable: mean_a.variable.clone(),
        mean_a: mean_a.mean,
        mean_b: mean_b.mean,
        difference: mean_b.mean - mean_a.mean,
        nodata_a: mean_a.nodata,
        nodata_b: mean_b.nodata,
    })
}
