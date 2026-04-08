use std::{
    io::{self, Read},
    path::PathBuf,
};

use serde::Serialize;

use crate::{
    agent, cli,
    engine::{DatasetKind, ExecutionError, ExecutionResult},
    output::{render, OutputFormat},
    provider::{supported_providers, ProviderKind, ProviderStatus, ProviderStore},
    session::{SessionState, SessionStore},
    tools,
};

pub fn run(cli: cli::Cli) -> Result<(), ExecutionError> {
    let session_store = SessionStore::new();
    let mut session = session_store.load()?;
    let format = OutputFormat::from_json_flag(cli.json);

    let (response, persist_session) = match cli.command {
        cli::Command::Inspect(args) => handle_inspect(args.file, &mut session),
        cli::Command::Mean(args) => handle_mean(args.file, args.var, &mut session),
        cli::Command::Compare(args) => {
            handle_compare(args.file_a, args.file_b, args.var, &mut session)
        }
        cli::Command::Ask(args) => handle_ask(args, &mut session),
        cli::Command::Provider(args) => handle_provider(args),
        cli::Command::Session(args) => handle_session(args, &session_store, &mut session),
    }?;

    println!("{}", render(&response, format)?);
    if persist_session {
        session_store.save(&session)?;
    }

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
) -> Result<(CommandResponse, bool), ExecutionError> {
    Ok((execute_inspect(file, session)?, true))
}

fn execute_inspect(
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
) -> Result<(CommandResponse, bool), ExecutionError> {
    Ok((execute_mean(file, var, session)?, true))
}

fn execute_mean(
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
) -> Result<(CommandResponse, bool), ExecutionError> {
    Ok((execute_compare(file_a, file_b, var, session)?, true))
}

fn execute_compare(
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

fn handle_session(
    args: cli::SessionArgs,
    session_store: &SessionStore,
    session: &mut SessionState,
) -> Result<(CommandResponse, bool), ExecutionError> {
    match args.command {
        cli::SessionCommand::Show => Ok((
            CommandResponse {
                command: "session_show",
                summary: "Displayed current session state".to_string(),
                dataset_kind: None,
                details: serde_json::json!({
                    "path": session_store.path(),
                    "session": session,
                }),
            },
            false,
        )),
        cli::SessionCommand::Clear => {
            session_store.clear()?;
            *session = SessionState::default();

            Ok((
                CommandResponse {
                    command: "session_clear",
                    summary: "Cleared current session state".to_string(),
                    dataset_kind: None,
                    details: serde_json::json!({
                        "path": session_store.path(),
                        "cleared": true,
                    }),
                },
                false,
            ))
        }
    }
}

fn handle_ask(
    args: cli::AskArgs,
    session: &mut SessionState,
) -> Result<(CommandResponse, bool), ExecutionError> {
    let provider = ProviderStatus::current(ProviderKind::OpenAi)?;
    if !provider.config.configured {
        return Err(ExecutionError::ProviderNotConfigured(
            "OpenAI is not configured. Set OPENAI_API_KEY or run `geocode provider set-api-key openai --stdin` to enable `geocode ask`.".into(),
        ));
    }

    let selected_files = args
        .files
        .iter()
        .map(|path| path.display().to_string())
        .collect();
    let request =
        agent::build_request(args.query, selected_files, session, &tools::builtin_tools());
    let plan = agent::plan_with_provider(&request, &provider.config)?;

    if !plan.requires_clarification {
        if let Some(executed) = execute_supported_agent_plan(&plan, session)? {
            return Ok((executed, true));
        }
    }

    Ok((
        CommandResponse {
            command: "ask",
            summary: "Generated agent execution plan".to_string(),
            dataset_kind: None,
            details: serde_json::json!({
                "provider": provider,
                "request": request,
                "plan": plan,
            }),
        },
        false,
    ))
}

fn execute_supported_agent_plan(
    plan: &agent::PlannerResponse,
    session: &mut SessionState,
) -> Result<Option<CommandResponse>, ExecutionError> {
    match plan.intent {
        agent::AgentIntent::Inspect if plan.target_files.len() == 1 => Ok(Some(execute_inspect(
            PathBuf::from(&plan.target_files[0]),
            session,
        )?)),
        agent::AgentIntent::Mean if plan.target_files.len() == 1 => Ok(Some(execute_mean(
            PathBuf::from(&plan.target_files[0]),
            plan.variable.clone(),
            session,
        )?)),
        agent::AgentIntent::Compare if plan.target_files.len() == 2 => Ok(Some(execute_compare(
            PathBuf::from(&plan.target_files[0]),
            PathBuf::from(&plan.target_files[1]),
            plan.variable.clone(),
            session,
        )?)),
        _ => Ok(None),
    }
}

fn handle_provider(args: cli::ProviderArgs) -> Result<(CommandResponse, bool), ExecutionError> {
    match args.command {
        cli::ProviderCommand::List => {
            let providers = supported_providers()?;
            Ok((
                CommandResponse {
                    command: "provider_list",
                    summary: "Listed supported providers".to_string(),
                    dataset_kind: None,
                    details: serde_json::json!({ "providers": providers }),
                },
                false,
            ))
        }
        cli::ProviderCommand::Status => {
            let status = ProviderStatus::current(ProviderKind::OpenAi)?;
            Ok((
                CommandResponse {
                    command: "provider_status",
                    summary: "Displayed provider configuration status".to_string(),
                    dataset_kind: None,
                    details: serde_json::to_value(status)
                        .map_err(|err| ExecutionError::Output(err.to_string()))?,
                },
                false,
            ))
        }
        cli::ProviderCommand::SetApiKey(args) => {
            let api_key = if let Some(api_key) = args.api_key {
                api_key
            } else if args.stdin {
                read_api_key_from_stdin()?
            } else {
                return Err(ExecutionError::InvalidInput(
                    "either --api-key or --stdin is required".into(),
                ));
            };

            if api_key.trim().is_empty() {
                return Err(ExecutionError::InvalidInput(
                    "API key cannot be empty".into(),
                ));
            }

            let store = ProviderStore::new();
            let mut config = store.load(args.provider)?.unwrap_or_default();
            config.api_key = Some(api_key.trim().to_string());
            store.save(args.provider, &config)?;
            let status = ProviderStatus::current(args.provider)?;

            Ok((
                CommandResponse {
                    command: "provider_set_api_key",
                    summary: format!("Stored API key for {:?}", args.provider),
                    dataset_kind: None,
                    details: serde_json::json!({
                        "provider_name": args.provider,
                        "path": store.path(args.provider),
                        "provider": status,
                    }),
                },
                false,
            ))
        }
    }
}

fn read_api_key_from_stdin() -> Result<String, ExecutionError> {
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer).map_err(|err| {
        ExecutionError::Provider(format!("failed to read api key from stdin: {err}"))
    })?;
    Ok(buffer)
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

#[cfg(test)]
mod tests {
    use std::{fs, process::Command};

    use tempfile::TempDir;

    use super::execute_supported_agent_plan;
    use crate::{
        agent::{AgentIntent, PlannerResponse},
        session::SessionState,
    };

    #[test]
    fn execute_supported_agent_plan_runs_inspect_for_single_target_file() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let file = create_sample_netcdf(temp_dir.path());
        let plan = PlannerResponse {
            intent: AgentIntent::Inspect,
            target_files: vec![file.display().to_string()],
            variable: None,
            tool_ids: vec!["inspect_metadata".to_string()],
            requires_clarification: false,
            clarification_question: None,
        };

        let response = execute_supported_agent_plan(&plan, &mut SessionState::default())
            .expect("execute plan")
            .expect("supported inspect plan");

        assert_eq!(response.command, "inspect");
        assert_eq!(
            response.dataset_kind.as_ref().map(|_| "present"),
            Some("present")
        );
    }

    fn create_sample_netcdf(dir: &std::path::Path) -> std::path::PathBuf {
        let cdl = dir.join("sample.cdl");
        let file = dir.join("sample.nc");

        fs::write(
            &cdl,
            r#"netcdf sample {
dimensions:
    time = 2 ;
    x = 3 ;
variables:
    float depth(time, x) ;
data:
    depth = 1, 2, 3, 4, 5, 6 ;
}
"#,
        )
        .expect("write cdl");

        let status = Command::new("ncgen")
            .arg("-o")
            .arg(&file)
            .arg(&cdl)
            .status()
            .expect("run ncgen");

        assert!(status.success(), "ncgen should succeed");
        file
    }
}
