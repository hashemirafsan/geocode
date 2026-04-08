use std::path::PathBuf;

use serde::Serialize;

use crate::{
    cli,
    engine::{DatasetKind, ExecutionError, ExecutionResult},
    output::{render, OutputFormat},
    session::{SessionState, SessionStore},
    tools,
};

pub fn run(cli: cli::Cli) -> Result<(), ExecutionError> {
    let session_store = SessionStore::new();
    let mut session = session_store.load()?;
    let format = OutputFormat::from_json_flag(cli.json);

    let response = match cli.command {
        cli::Command::Inspect(args) => handle_inspect(args.file, &mut session),
        cli::Command::Mean(args) => handle_mean(args.file, args.var, &mut session),
        cli::Command::Compare(args) => {
            handle_compare(args.file_a, args.file_b, args.var, &mut session)
        }
    }?;

    println!("{}", render(&response, format)?);
    session_store.save(&session)?;
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct CommandResponse {
    pub command: &'static str,
    pub summary: String,
    pub dataset_kind: Option<DatasetKind>,
    pub details: serde_json::Value,
}

fn handle_inspect(
    file: PathBuf,
    session: &mut SessionState,
) -> Result<CommandResponse, ExecutionError> {
    if let Some(parent) = file.parent() {
        session.workspace_path = Some(parent.to_path_buf());
    }

    let report = tools::inspect(&file)?;
    let summary = match report.kind {
        DatasetKind::Netcdf => format!("Inspected NetCDF metadata for {}", file.display()),
        DatasetKind::Geotiff => format!("Inspected GeoTIFF metadata for {}", file.display()),
    };

    Ok(CommandResponse {
        command: "inspect",
        summary,
        dataset_kind: Some(report.kind),
        details: serde_json::to_value(report)
            .map_err(|err| ExecutionError::Output(err.to_string()))?,
    })
}

fn handle_mean(
    file: PathBuf,
    var: Option<String>,
    session: &mut SessionState,
) -> Result<CommandResponse, ExecutionError> {
    if let Some(parent) = file.parent() {
        session.workspace_path = Some(parent.to_path_buf());
    }

    let report = tools::mean(&file, var.as_deref())?;
    session.last_variable = report.variable.clone();

    Ok(CommandResponse {
        command: "mean",
        summary: match report.kind {
            DatasetKind::Netcdf => format!(
                "Computed NetCDF mean for {} in {}",
                report.variable.as_deref().unwrap_or("<unknown>"),
                file.display()
            ),
            DatasetKind::Geotiff => format!("Computed GeoTIFF mean for {}", file.display()),
        },
        dataset_kind: Some(report.kind),
        details: serde_json::to_value(report)
            .map_err(|err| ExecutionError::Output(err.to_string()))?,
    })
}

fn handle_compare(
    file_a: PathBuf,
    file_b: PathBuf,
    var: Option<String>,
    session: &mut SessionState,
) -> Result<CommandResponse, ExecutionError> {
    if let Some(parent) = file_a.parent() {
        session.workspace_path = Some(parent.to_path_buf());
    }

    let report = tools::compare(&file_a, &file_b, var.as_deref())?;
    session.last_variable = report.variable.clone();

    Ok(CommandResponse {
        command: "compare",
        summary: match report.kind {
            DatasetKind::Netcdf => format!(
                "Compared NetCDF mean for {} between {} and {}",
                report.variable.as_deref().unwrap_or("<unknown>"),
                file_a.display(),
                file_b.display()
            ),
            DatasetKind::Geotiff => format!(
                "Compared GeoTIFF mean between {} and {}",
                file_a.display(),
                file_b.display()
            ),
        },
        dataset_kind: Some(report.kind),
        details: serde_json::to_value(report)
            .map_err(|err| ExecutionError::Output(err.to_string()))?,
    })
}

impl From<ExecutionResult> for CommandResponse {
    fn from(result: ExecutionResult) -> Self {
        Self {
            command: result.command,
            summary: result.summary,
            dataset_kind: result.dataset_kind,
            details: result.details,
        }
    }
}
