#![allow(dead_code)]

use std::path::PathBuf;

use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DatasetKind {
    Netcdf,
    Geotiff,
}

#[derive(Debug, Clone, Serialize)]
pub struct DatasetRef {
    pub path: PathBuf,
    pub kind: DatasetKind,
}

#[derive(Debug, Clone, Serialize)]
pub struct VariableRef {
    pub dataset: DatasetRef,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Error)]
pub enum ExecutionError {
    #[error("file not found: {0}")]
    FileNotFound(String),
    #[error("path is not a regular file: {0}")]
    InvalidFile(String),
    #[error("unsupported dataset type: {0}")]
    UnsupportedDatasetType(String),
    #[error("variable selection is required for this operation")]
    MissingVariable,
    #[error("invalid variable: {0}")]
    InvalidVariable(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("invalid compare request: {0}")]
    InvalidCompare(String),
    #[error("command failed: {0}")]
    Command(String),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("i/o error: {0}")]
    Io(String),
    #[error("session error: {0}")]
    Session(String),
    #[error("output error: {0}")]
    Output(String),
}
