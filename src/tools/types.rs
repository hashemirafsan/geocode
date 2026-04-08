use std::path::PathBuf;

use serde::Serialize;

use crate::{
    bindings::{GeotiffMetadata, NetcdfMetadata},
    engine::DatasetKind,
};

#[derive(Debug, Clone, Serialize)]
pub struct InspectReport {
    pub file: PathBuf,
    pub kind: DatasetKind,
    pub netcdf: Option<NetcdfMetadata>,
    pub geotiff: Option<GeotiffMetadata>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MeanReport {
    pub file: PathBuf,
    pub kind: DatasetKind,
    pub variable: Option<String>,
    pub mean: f64,
    pub nodata: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CompareReport {
    pub file_a: PathBuf,
    pub file_b: PathBuf,
    pub kind: DatasetKind,
    pub variable: Option<String>,
    pub mean_a: f64,
    pub mean_b: f64,
    pub difference: f64,
    pub nodata_a: Option<f64>,
    pub nodata_b: Option<f64>,
}
