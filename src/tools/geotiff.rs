use std::path::Path;

use crate::{
    bindings::{read_geotiff_mean, read_geotiff_metadata},
    engine::{DatasetKind, ExecutionError},
};

use super::types::{InspectReport, MeanReport};

pub fn inspect_geotiff(path: &Path) -> Result<InspectReport, ExecutionError> {
    let metadata = read_geotiff_metadata(path)?;

    Ok(InspectReport {
        file: path.to_path_buf(),
        kind: DatasetKind::Geotiff,
        netcdf: None,
        geotiff: Some(metadata),
    })
}

pub fn mean_geotiff(path: &Path) -> Result<MeanReport, ExecutionError> {
    let (mean, nodata) = read_geotiff_mean(path)?;

    Ok(MeanReport {
        file: path.to_path_buf(),
        kind: DatasetKind::Geotiff,
        variable: None,
        mean,
        nodata,
    })
}
