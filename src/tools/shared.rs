use std::path::Path;

use crate::engine::{DatasetKind, ExecutionError};

use super::{
    geotiff::{inspect_geotiff, mean_geotiff},
    netcdf::{inspect_netcdf, mean_netcdf},
    types::{CompareReport, InspectReport, MeanReport},
};

pub fn detect_dataset_kind(path: &Path) -> Result<DatasetKind, ExecutionError> {
    crate::bindings::detect_dataset_kind(path)
}

pub fn validate_local_file(path: &Path) -> Result<(), ExecutionError> {
    crate::bindings::validate_local_file(path)
}

pub fn inspect(path: &Path) -> Result<InspectReport, ExecutionError> {
    validate_local_file(path)?;

    match detect_dataset_kind(path)? {
        DatasetKind::Netcdf => inspect_netcdf(path),
        DatasetKind::Geotiff => inspect_geotiff(path),
    }
}

pub fn mean(path: &Path, variable: Option<&str>) -> Result<MeanReport, ExecutionError> {
    validate_local_file(path)?;

    match detect_dataset_kind(path)? {
        DatasetKind::Netcdf => mean_netcdf(path, variable),
        DatasetKind::Geotiff => mean_geotiff(path),
    }
}

pub fn compare(
    path_a: &Path,
    path_b: &Path,
    variable: Option<&str>,
) -> Result<CompareReport, ExecutionError> {
    super::compare::compare(path_a, path_b, variable)
}
