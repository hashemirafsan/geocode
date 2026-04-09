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
        "ask" => render_ask(response),
        "ask_result" => render_ask_result(response),
        "provider_list" => render_provider_list(response),
        "provider_status" => render_provider_status(response),
        "provider_set_model" => render_provider_set_model(response),
        "provider_set_api_key" => render_provider_set_api_key(response),
        "session_show" => render_session_show(response),
        "session_clear" => render_session_clear(response),
        _ => response.summary.clone(),
    }
}

fn render_provider_set_model(response: &CommandResponse) -> String {
    let provider = display_provider_name(
        response.details["provider_name"]
            .as_str()
            .unwrap_or("unknown"),
    );
    let model = response.details["model"].as_str().unwrap_or("<unknown>");
    let path = response.details["path"].as_str().unwrap_or("<unknown>");

    format!("Stored model\nProvider: {provider}\nModel: {model}\nConfig Path: {path}")
}

fn render_ask_result(response: &CommandResponse) -> String {
    if let Some(value) = response.details["value"].as_f64() {
        let label = response.details["label"].as_str().unwrap_or("Result");
        return append_capability_trace(response, format!("{label}: {value:.6}"));
    }

    if let Some(rows) = response.details["rows"].as_array() {
        let title = response.details["title"].as_str().unwrap_or("Result");
        let body = rows
            .iter()
            .filter_map(|row| {
                Some(format!(
                    "- {}: {}",
                    row["label"].as_str()?,
                    row["value"].as_str()?
                ))
            })
            .collect::<Vec<_>>()
            .join("\n");
        return append_capability_trace(response, format!("{title}\n{body}"));
    }

    response.summary.clone()
}

fn render_provider_set_api_key(response: &CommandResponse) -> String {
    let provider = display_provider_name(
        response.details["provider_name"]
            .as_str()
            .unwrap_or("unknown"),
    );
    let path = response.details["path"].as_str().unwrap_or("<unknown>");
    let configured = response.details["provider"]["config"]["configured"]
        .as_bool()
        .unwrap_or(false);
    let source = response.details["provider"]["credential_source"]
        .as_str()
        .unwrap_or("unknown");

    format!(
        "Stored API key\nProvider: {provider}\nConfig Path: {path}\nConfigured: {configured}\nCredential Source: {source}"
    )
}

fn render_provider_list(response: &CommandResponse) -> String {
    let providers = response.details["providers"]
        .as_array()
        .map(|providers| {
            providers
                .iter()
                .map(|provider| {
                    let name =
                        display_provider_name(provider["provider"].as_str().unwrap_or("unknown"));
                    let auth_method = provider["auth_method"].as_str().unwrap_or("unknown");
                    let configured = provider["configured"].as_bool().unwrap_or(false);
                    let default = provider["default"].as_bool().unwrap_or(false);
                    let suffix = if default { ", default=true" } else { "" };
                    format!("- {name} ({auth_method}, configured={configured}{suffix})")
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();

    format!("Supported Providers:\n{providers}")
}

fn render_ask(response: &CommandResponse) -> String {
    let provider = display_provider_name(
        response.details["provider"]["config"]["provider"]
            .as_str()
            .unwrap_or("unknown"),
    );
    let intent = response.details["plan"]["intent"]
        .as_str()
        .unwrap_or("unknown");
    let variable = response.details["plan"]["variable"]
        .as_str()
        .unwrap_or("<none>");
    let requires_clarification = response.details["plan"]["requires_clarification"]
        .as_bool()
        .unwrap_or(false);
    let tools = response.details["plan"]["tool_ids"]
        .as_array()
        .map(|tools| {
            tools
                .iter()
                .filter_map(|tool| tool.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    let step_count = response.details["plan"]["plan"]["steps"]
        .as_array()
        .map(|steps| steps.len())
        .unwrap_or_default();
    let selected_files = response.details["request"]["selected_files"]
        .as_array()
        .map(|files| {
            files
                .iter()
                .filter_map(|file| file.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();

    let mut lines = vec![format!("Provider: {provider}"), format!("Intent: {intent}")];

    if !selected_files.is_empty() {
        lines.push(format!("Selected Files: {selected_files}"));
    }

    if intent != "unknown" {
        lines.push(format!("Variable: {variable}"));
        lines.push(format!("Planned Capabilities: {tools}"));
        lines.push(format!("Plan Steps: {step_count}"));
    }

    lines.push(format!("Needs Clarification: {requires_clarification}"));

    if let Some(question) = response.details["plan"]["clarification_question"].as_str() {
        lines.push(format!("Clarification: {question}"));
    }

    lines.join("\n")
}

fn render_provider_status(response: &CommandResponse) -> String {
    let provider = display_provider_name(
        response.details["config"]["provider"]
            .as_str()
            .unwrap_or("unknown"),
    );
    let auth_method = response.details["config"]["auth_method"]
        .as_str()
        .unwrap_or("unknown");
    let configured = response.details["config"]["configured"]
        .as_bool()
        .unwrap_or(false);
    let model = response.details["config"]["model"]
        .as_str()
        .unwrap_or("<unknown>");
    let base_url = response.details["config"]["base_url"]
        .as_str()
        .unwrap_or("<unknown>");
    let env_var = response.details["api_key_env_var"]
        .as_str()
        .unwrap_or("OPENAI_API_KEY");
    let config_path = response.details["config_path"]
        .as_str()
        .unwrap_or("<unknown>");
    let credential_source = response.details["credential_source"]
        .as_str()
        .unwrap_or("unknown");
    let is_default = response.details["is_default"].as_bool().unwrap_or(false);

    format!(
        "Provider: {provider}\nAuth Method: {auth_method}\nConfigured: {configured}\nDefault: {is_default}\nModel: {model}\nBase URL: {base_url}\nAPI Key Env Var: {env_var}\nConfig Path: {config_path}\nCredential Source: {credential_source}"
    )
}

fn display_provider_name(name: &str) -> &str {
    match name {
        "open_ai" => "openai",
        "lm_studio" => "lmstudio",
        other => other,
    }
}

fn render_session_show(response: &CommandResponse) -> String {
    let path = response.details["path"].as_str().unwrap_or("<unknown>");
    let session = &response.details["session"];
    let session_id = session["session_id"].as_str().unwrap_or("<none>");
    let workspace_path = session["workspace_path"].as_str().unwrap_or("<none>");
    let last_variable = session["last_variable"].as_str().unwrap_or("<none>");
    let alias_count = session["aliases"]
        .as_array()
        .map(|aliases| aliases.len())
        .unwrap_or_default();

    format!(
        "Session File: {path}\nSession ID: {session_id}\nWorkspace Path: {workspace_path}\nLast Variable: {last_variable}\nAliases: {alias_count}"
    )
}

fn render_session_clear(response: &CommandResponse) -> String {
    let path = response.details["path"].as_str().unwrap_or("<unknown>");
    format!("Session cleared\nSession File: {path}")
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
            append_capability_trace(
                response,
                format!(
                "File A: {file_a}\nFile B: {file_b}\nType: netcdf\nVariable: {variable}\nMean A: {mean_a:.6}\nMean B: {mean_b:.6}\nDifference (B - A): {difference:.6}"
                ),
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

            append_capability_trace(response, lines.join("\n"))
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
            append_capability_trace(
                response,
                format!("File: {file}\nType: netcdf\nVariable: {variable}\nMean: {mean:.6}"),
            )
        }
        "geotiff" => match response.details["nodata"].as_f64() {
            Some(nodata) => append_capability_trace(
                response,
                format!("File: {file}\nType: geotiff\nMean: {mean:.6}\nNodata: {nodata}"),
            ),
            None => append_capability_trace(
                response,
                format!("File: {file}\nType: geotiff\nMean: {mean:.6}"),
            ),
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
        "netcdf" => {
            append_capability_trace(response, render_netcdf_inspect(file, &response.details))
        }
        "geotiff" => {
            append_capability_trace(response, render_geotiff_inspect(file, &response.details))
        }
        _ => response.summary.clone(),
    }
}

fn append_capability_trace(response: &CommandResponse, body: String) -> String {
    let trace = response.details["capability_trace"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .take(8)
                .filter_map(|item| item["detail"].as_str())
                .map(|detail| format!("- {detail}"))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if trace.is_empty() {
        body
    } else {
        format!("{body}\nCapability Trace:\n{}", trace.join("\n"))
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
