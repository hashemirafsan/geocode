#![allow(dead_code)]

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde::{Deserialize, Serialize};

use crate::engine::{DatasetKind, ExecutionError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDescriptor {
    pub id: &'static str,
    pub description: &'static str,
}

pub fn builtin_tools() -> Vec<ToolDescriptor> {
    vec![
        ToolDescriptor {
            id: "inspect_metadata",
            description: "Inspect essential metadata for a dataset",
        },
        ToolDescriptor {
            id: "mean",
            description: "Compute a mean summary for a dataset target",
        },
        ToolDescriptor {
            id: "compare_mean",
            description: "Compare scalar mean summaries between two datasets",
        },
    ]
}

#[derive(Debug, Clone, Serialize)]
pub struct InspectReport {
    pub file: PathBuf,
    pub kind: DatasetKind,
    pub netcdf: Option<NetcdfMetadata>,
    pub geotiff: Option<GeotiffMetadata>,
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

fn validate_local_file(path: &Path) -> Result<(), ExecutionError> {
    let metadata =
        fs::metadata(path).map_err(|_| ExecutionError::FileNotFound(path.display().to_string()))?;

    if !metadata.is_file() {
        return Err(ExecutionError::InvalidFile(path.display().to_string()));
    }

    Ok(())
}

fn inspect_netcdf(path: &Path) -> Result<InspectReport, ExecutionError> {
    let output = Command::new("ncdump")
        .arg("-h")
        .arg(path)
        .output()
        .map_err(|err| ExecutionError::Command(format!("failed to execute ncdump: {err}")))?;

    if !output.status.success() {
        return Err(ExecutionError::Command(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }

    let header = String::from_utf8_lossy(&output.stdout);
    let metadata = parse_netcdf_header(&header)?;

    Ok(InspectReport {
        file: path.to_path_buf(),
        kind: DatasetKind::Netcdf,
        netcdf: Some(metadata),
        geotiff: None,
    })
}

fn inspect_geotiff(path: &Path) -> Result<InspectReport, ExecutionError> {
    let value = run_gdalinfo_json(path, false)?;
    let metadata = parse_geotiff_metadata(&value)?;

    Ok(InspectReport {
        file: path.to_path_buf(),
        kind: DatasetKind::Geotiff,
        netcdf: None,
        geotiff: Some(metadata),
    })
}

fn mean_netcdf(path: &Path, variable: Option<&str>) -> Result<MeanReport, ExecutionError> {
    let variable = variable.ok_or(ExecutionError::MissingVariable)?;
    let metadata = inspect_netcdf(path)?.netcdf.ok_or_else(|| {
        ExecutionError::Parse("internal error: missing parsed netcdf metadata".into())
    })?;

    if !metadata
        .variables
        .iter()
        .any(|entry| entry.name == variable)
    {
        return Err(ExecutionError::InvalidVariable(variable.to_string()));
    }

    let output = Command::new("ncdump")
        .arg("-v")
        .arg(variable)
        .arg(path)
        .output()
        .map_err(|err| ExecutionError::Command(format!("failed to execute ncdump: {err}")))?;

    if !output.status.success() {
        return Err(ExecutionError::Command(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }

    let values = parse_netcdf_variable_values(&String::from_utf8_lossy(&output.stdout), variable)?;
    let mean = arithmetic_mean(&values)?;

    Ok(MeanReport {
        file: path.to_path_buf(),
        kind: DatasetKind::Netcdf,
        variable: Some(variable.to_string()),
        mean,
        nodata: None,
    })
}

fn mean_geotiff(path: &Path) -> Result<MeanReport, ExecutionError> {
    let value = run_gdalinfo_json(path, true)?;
    let metadata = parse_geotiff_metadata(&value)?;

    if metadata.band_count != 1 {
        return Err(ExecutionError::InvalidInput(
            "mean currently supports single-band GeoTIFF files only".into(),
        ));
    }

    let band = value["bands"]
        .as_array()
        .and_then(|bands| bands.first())
        .ok_or_else(|| ExecutionError::Parse("gdalinfo output missing first band".into()))?;

    let mean = band
        .get("mean")
        .and_then(|value| value.as_f64())
        .ok_or_else(|| ExecutionError::Parse("gdalinfo output missing band mean".into()))?;

    let nodata = metadata.bands.first().and_then(|band| band.nodata);

    Ok(MeanReport {
        file: path.to_path_buf(),
        kind: DatasetKind::Geotiff,
        variable: None,
        mean,
        nodata,
    })
}

fn parse_netcdf_header(header: &str) -> Result<NetcdfMetadata, ExecutionError> {
    enum Section {
        None,
        Dimensions,
        Variables,
    }

    let mut section = Section::None;
    let mut dimensions = Vec::new();
    let mut variables = Vec::new();

    for raw_line in header.lines() {
        let line = raw_line.trim();

        if line.is_empty() || line == "}" {
            continue;
        }

        if line == "dimensions:" {
            section = Section::Dimensions;
            continue;
        }

        if line == "variables:" {
            section = Section::Variables;
            continue;
        }

        if line == "data:" {
            break;
        }

        match section {
            Section::Dimensions => {
                if let Some(dimension) = parse_dimension_line(line) {
                    dimensions.push(dimension);
                }
            }
            Section::Variables => {
                if let Some(variable) = parse_variable_line(line, &dimensions)? {
                    variables.push(variable);
                }
            }
            Section::None => {}
        }
    }

    Ok(NetcdfMetadata {
        dimensions,
        variables,
    })
}

fn parse_netcdf_variable_values(output: &str, variable: &str) -> Result<Vec<f64>, ExecutionError> {
    let data_section = output.split("data:").nth(1).ok_or_else(|| {
        ExecutionError::Parse(format!("missing data section for variable {variable}"))
    })?;

    let marker = format!("{variable} =");
    let start = data_section.find(&marker).ok_or_else(|| {
        ExecutionError::Parse(format!("could not locate values for variable {variable}"))
    })?;

    let after_marker = &data_section[start + marker.len()..];
    let end = after_marker.find(';').ok_or_else(|| {
        ExecutionError::Parse(format!("unterminated data section for variable {variable}"))
    })?;

    let values_text = &after_marker[..end];
    let mut values = Vec::new();

    for token in values_text
        .split(|ch: char| ch == ',' || ch.is_whitespace())
        .filter(|token| !token.is_empty())
    {
        if token == "_" {
            continue;
        }

        let normalized = token.trim_end_matches(['f', 'F']);
        let value = normalized.parse::<f64>().map_err(|err| {
            ExecutionError::Parse(format!(
                "failed to parse value '{token}' for {variable}: {err}"
            ))
        })?;

        values.push(value);
    }

    if values.is_empty() {
        return Err(ExecutionError::Parse(format!(
            "no numeric values found for variable {variable}"
        )));
    }

    Ok(values)
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

fn parse_dimension_line(line: &str) -> Option<DimensionInfo> {
    let trimmed = line.strip_suffix(';')?.trim();
    let (name, value) = trimmed.split_once('=')?;
    let length = value
        .chars()
        .filter(|ch| ch.is_ascii_digit())
        .collect::<String>()
        .parse::<usize>()
        .ok();

    Some(DimensionInfo {
        name: name.trim().to_string(),
        length,
    })
}

fn parse_variable_line(
    line: &str,
    dimensions: &[DimensionInfo],
) -> Result<Option<VariableMetadata>, ExecutionError> {
    if line.contains(':') {
        return Ok(None);
    }

    let trimmed = match line.strip_suffix(';') {
        Some(line) => line.trim(),
        None => return Ok(None),
    };

    let Some((dtype, remainder)) = trimmed.split_once(char::is_whitespace) else {
        return Ok(None);
    };

    let remainder = remainder.trim();
    let (name, dimensions_list) = if let Some((name, dims)) = remainder.split_once('(') {
        let dims = dims.trim_end_matches(')').trim();
        let dimensions = if dims.is_empty() {
            Vec::new()
        } else {
            dims.split(',').map(|dim| dim.trim().to_string()).collect()
        };
        (name.trim().to_string(), dimensions)
    } else {
        (remainder.to_string(), Vec::new())
    };

    let shape = dimensions_list
        .iter()
        .filter_map(|dimension_name| {
            dimensions
                .iter()
                .find(|dimension| dimension.name == *dimension_name)
                .and_then(|dimension| dimension.length)
        })
        .collect();

    Ok(Some(VariableMetadata {
        name,
        dtype: dtype.to_string(),
        dimensions: dimensions_list,
        shape,
    }))
}

fn run_gdalinfo_json(path: &Path, with_stats: bool) -> Result<serde_json::Value, ExecutionError> {
    let mut command = Command::new("gdalinfo");
    command.arg("-json");

    if with_stats {
        command.arg("-stats");
    }

    let output = command
        .arg(path)
        .output()
        .map_err(|err| ExecutionError::Command(format!("failed to execute gdalinfo: {err}")))?;

    if !output.status.success() {
        return Err(ExecutionError::Command(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }

    serde_json::from_slice(&output.stdout)
        .map_err(|err| ExecutionError::Parse(format!("invalid gdalinfo json: {err}")))
}

fn parse_geotiff_metadata(value: &serde_json::Value) -> Result<GeotiffMetadata, ExecutionError> {
    let size = value
        .get("size")
        .and_then(|size| size.as_array())
        .ok_or_else(|| ExecutionError::Parse("gdalinfo output missing size".into()))?;

    if size.len() != 2 {
        return Err(ExecutionError::Parse(
            "gdalinfo size must contain width and height".into(),
        ));
    }

    let width = size[0]
        .as_u64()
        .ok_or_else(|| ExecutionError::Parse("gdalinfo width is not an integer".into()))?;
    let height = size[1]
        .as_u64()
        .ok_or_else(|| ExecutionError::Parse("gdalinfo height is not an integer".into()))?;

    let bands_value = value
        .get("bands")
        .and_then(|bands| bands.as_array())
        .ok_or_else(|| ExecutionError::Parse("gdalinfo output missing bands".into()))?;

    let mut bands = Vec::with_capacity(bands_value.len());
    for band in bands_value {
        bands.push(GeotiffBandMetadata {
            band: band
                .get("band")
                .and_then(|value| value.as_u64())
                .ok_or_else(|| {
                    ExecutionError::Parse("band number missing from gdalinfo output".into())
                })?,
            dtype: band
                .get("type")
                .and_then(|value| value.as_str())
                .ok_or_else(|| {
                    ExecutionError::Parse("band type missing from gdalinfo output".into())
                })?
                .to_string(),
            nodata: band.get("noDataValue").and_then(|value| value.as_f64()),
        });
    }

    Ok(GeotiffMetadata {
        width,
        height,
        band_count: bands.len(),
        bands,
    })
}
