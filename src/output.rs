use crate::{app::CommandResponse, engine::ExecutionError};

#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    Text,
    Json,
}

impl OutputFormat {
    pub fn from_json_flag(json: bool) -> Self {
        if json {
            Self::Json
        } else {
            Self::Text
        }
    }
}

pub fn render(response: &CommandResponse, format: OutputFormat) -> Result<String, ExecutionError> {
    match format {
        OutputFormat::Text => Ok(render_text(response)),
        OutputFormat::Json => serde_json::to_string_pretty(response)
            .map_err(|err| ExecutionError::Output(err.to_string())),
    }
}

fn render_text(response: &CommandResponse) -> String {
    match response.command {
        "inspect" => render_inspect(response),
        "mean" => render_mean(response),
        "compare" => render_compare(response),
        _ => response.summary.clone(),
    }
}

fn render_compare(response: &CommandResponse) -> String {
    let kind = response
        .dataset_kind
        .map(|kind| match kind {
            crate::engine::DatasetKind::Netcdf => "netcdf",
            crate::engine::DatasetKind::Geotiff => "geotiff",
        })
        .unwrap_or("unknown");

    let file_a = response.details["file_a"].as_str().unwrap_or("<unknown>");
    let file_b = response.details["file_b"].as_str().unwrap_or("<unknown>");
    let mean_a = response.details["mean_a"].as_f64().unwrap_or_default();
    let mean_b = response.details["mean_b"].as_f64().unwrap_or_default();
    let difference = response.details["difference"].as_f64().unwrap_or_default();

    match kind {
        "netcdf" => {
            let variable = response.details["variable"].as_str().unwrap_or("<unknown>");
            format!(
                "File A: {file_a}\nFile B: {file_b}\nType: netcdf\nVariable: {variable}\nMean A: {mean_a:.6}\nMean B: {mean_b:.6}\nDifference (B - A): {difference:.6}"
            )
        }
        "geotiff" => {
            let mut lines = vec![
                format!("File A: {file_a}"),
                format!("File B: {file_b}"),
                "Type: geotiff".to_string(),
                format!("Mean A: {mean_a:.6}"),
                format!("Mean B: {mean_b:.6}"),
                format!("Difference (B - A): {difference:.6}"),
            ];

            if let Some(nodata_a) = response.details["nodata_a"].as_f64() {
                lines.push(format!("Nodata A: {nodata_a}"));
            }

            if let Some(nodata_b) = response.details["nodata_b"].as_f64() {
                lines.push(format!("Nodata B: {nodata_b}"));
            }

            lines.join("\n")
        }
        _ => response.summary.clone(),
    }
}

fn render_mean(response: &CommandResponse) -> String {
    let kind = response
        .dataset_kind
        .map(|kind| match kind {
            crate::engine::DatasetKind::Netcdf => "netcdf",
            crate::engine::DatasetKind::Geotiff => "geotiff",
        })
        .unwrap_or("unknown");

    let file = response.details["file"].as_str().unwrap_or("<unknown>");
    let mean = response.details["mean"].as_f64().unwrap_or_default();

    match kind {
        "netcdf" => {
            let variable = response.details["variable"].as_str().unwrap_or("<unknown>");
            format!("File: {file}\nType: netcdf\nVariable: {variable}\nMean: {mean:.6}")
        }
        "geotiff" => match response.details["nodata"].as_f64() {
            Some(nodata) => {
                format!("File: {file}\nType: geotiff\nMean: {mean:.6}\nNodata: {nodata}")
            }
            None => format!("File: {file}\nType: geotiff\nMean: {mean:.6}"),
        },
        _ => response.summary.clone(),
    }
}

fn render_inspect(response: &CommandResponse) -> String {
    let kind = response
        .dataset_kind
        .map(|kind| match kind {
            crate::engine::DatasetKind::Netcdf => "netcdf",
            crate::engine::DatasetKind::Geotiff => "geotiff",
        })
        .unwrap_or("unknown");

    let file = response.details["file"].as_str().unwrap_or("<unknown>");

    match kind {
        "netcdf" => render_netcdf_inspect(file, &response.details),
        "geotiff" => render_geotiff_inspect(file, &response.details),
        _ => response.summary.clone(),
    }
}

fn render_netcdf_inspect(file: &str, details: &serde_json::Value) -> String {
    let dimensions = details["netcdf"]["dimensions"]
        .as_array()
        .map(|dimensions| {
            dimensions
                .iter()
                .map(|dimension| {
                    let name = dimension["name"].as_str().unwrap_or("unknown");
                    let length = dimension["length"]
                        .as_u64()
                        .map(|length| length.to_string())
                        .unwrap_or_else(|| "unknown".to_string());
                    format!("{name}={length}")
                })
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();

    let variables = details["netcdf"]["variables"]
        .as_array()
        .map(|variables| {
            variables
                .iter()
                .map(|variable| {
                    let name = variable["name"].as_str().unwrap_or("unknown");
                    let dtype = variable["dtype"].as_str().unwrap_or("unknown");
                    let dimensions = variable["dimensions"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .zip(variable["shape"].as_array().into_iter().flatten())
                        .map(|(dimension, size)| {
                            format!(
                                "{}={}",
                                dimension.as_str().unwrap_or("unknown"),
                                size.as_u64().unwrap_or_default()
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(", ");

                    format!("- {name} ({dtype}) [{dimensions}]")
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();

    format!("File: {file}\nType: netcdf\nDimensions: {dimensions}\nVariables:\n{variables}")
}

fn render_geotiff_inspect(file: &str, details: &serde_json::Value) -> String {
    let geotiff = &details["geotiff"];
    let width = geotiff["width"].as_u64().unwrap_or_default();
    let height = geotiff["height"].as_u64().unwrap_or_default();
    let band_count = geotiff["band_count"].as_u64().unwrap_or_default();
    let bands = geotiff["bands"]
        .as_array()
        .map(|bands| {
            bands
                .iter()
                .map(|band| {
                    let band_number = band["band"].as_u64().unwrap_or_default();
                    let dtype = band["dtype"].as_str().unwrap_or("unknown");

                    match band["nodata"].as_f64() {
                        Some(nodata) => format!("- Band {band_number}: {dtype} (nodata={nodata})"),
                        None => format!("- Band {band_number}: {dtype}"),
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();

    format!(
        "File: {file}\nType: geotiff\nSize: {width} x {height}\nBands: {band_count}\nBand Details:\n{bands}"
    )
}
