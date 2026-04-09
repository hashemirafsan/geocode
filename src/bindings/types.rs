use serde::Serialize;

pub use crate::array::ArrayValue;
use crate::engine::DatasetRef;

#[derive(Debug, Clone, Serialize)]
pub struct NetcdfDatasetHandle {
    pub dataset: DatasetRef,
}

#[derive(Debug, Clone, Serialize)]
pub struct RasterDatasetHandle {
    pub dataset: DatasetRef,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DatasetHandle {
    Netcdf(NetcdfDatasetHandle),
    Raster(RasterDatasetHandle),
}

#[derive(Debug, Clone, Serialize)]
pub struct NetcdfVariableRef {
    pub dataset: NetcdfDatasetHandle,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DimensionInfo {
    pub name: String,
    pub length: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VariableMetadata {
    pub name: String,
    pub dtype: String,
    pub dimensions: Vec<String>,
    pub shape: Vec<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NetcdfMetadata {
    pub dimensions: Vec<DimensionInfo>,
    pub variables: Vec<VariableMetadata>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GeotiffBandMetadata {
    pub band: u64,
    pub dtype: String,
    pub nodata: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GeotiffMetadata {
    pub width: u64,
    pub height: u64,
    pub band_count: usize,
    pub bands: Vec<GeotiffBandMetadata>,
}
