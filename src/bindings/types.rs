use serde::Serialize;

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
