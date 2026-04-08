use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DatasetKind {
    Netcdf,
    Geotiff,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetRef {
    pub path: PathBuf,
    pub kind: DatasetKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableRef {
    pub dataset: DatasetRef,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEvent {
    pub stage: String,
    pub detail: String,
}

#[derive(Debug, Serialize)]
pub struct ExecutionResult {
    pub command: &'static str,
    pub summary: String,
    pub dataset_kind: Option<DatasetKind>,
    pub details: serde_json::Value,
    pub trace: Vec<TraceEvent>,
}
