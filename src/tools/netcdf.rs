use std::path::Path;

use crate::{
    bindings::{read_netcdf_metadata, read_netcdf_variable_values},
    engine::{DatasetKind, ExecutionError},
};

use super::types::{InspectReport, MeanReport};

pub fn inspect_netcdf(path: &Path) -> Result<InspectReport, ExecutionError> {
    let metadata = read_netcdf_metadata(path)?;

    Ok(InspectReport {
        file: path.to_path_buf(),
        kind: DatasetKind::Netcdf,
        netcdf: Some(metadata),
        geotiff: None,
    })
}

pub fn mean_netcdf(path: &Path, variable: Option<&str>) -> Result<MeanReport, ExecutionError> {
    let variable = variable.ok_or(ExecutionError::MissingVariable)?;
    let metadata = read_netcdf_metadata(path)?;

    if !metadata
        .variables
        .iter()
        .any(|entry| entry.name == variable)
    {
        return Err(ExecutionError::InvalidVariable(variable.to_string()));
    }

    let values = read_netcdf_variable_values(path, variable)?;
    let mean = arithmetic_mean(&values)?;

    Ok(MeanReport {
        file: path.to_path_buf(),
        kind: DatasetKind::Netcdf,
        variable: Some(variable.to_string()),
        mean,
        nodata: None,
    })
}

fn arithmetic_mean(values: &[f64]) -> Result<f64, ExecutionError> {
    if values.is_empty() {
        return Err(ExecutionError::InvalidInput(
            "cannot compute a mean over zero values".into(),
        ));
    }

    let sum: f64 = values.iter().sum();
    Ok(sum / values.len() as f64)
}
