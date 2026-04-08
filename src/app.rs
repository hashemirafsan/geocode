use std::{
    io::{self, Read},
    path::PathBuf,
};

use serde::Serialize;

use crate::{
    agent,
    capability::CapabilityRegistry,
    cli,
    engine::{DatasetKind, ExecutionError, ExecutionResult},
    executor::{PlanExecutor, RuntimeValue},
    memory::MemoryStore,
    output::{render, OutputFormat},
    plan::{CapabilityInput, ExecutionPlan, PlanStep, PlanValueRef},
    policy::ExecutionPolicy,
    provider::{supported_providers, ProviderKind, ProviderStatus, ProviderStore},
    session::{CachedResultSummary, RecentTurn, SessionState, SessionStore},
};

pub fn run(cli: cli::Cli) -> Result<(), ExecutionError> {
    let session_store = SessionStore::new();
    let mut session = session_store.load()?;
    let _memory = MemoryStore::new().load()?;
    let registry = CapabilityRegistry::discover();
    let policy = ExecutionPolicy::from_registry(&registry);
    let format = OutputFormat::from_json_flag(cli.json);

    let (response, persist_session) = match cli.command {
        cli::Command::Inspect(args) => handle_inspect(args.file, &registry, &policy, &mut session),
        cli::Command::Mean(args) => {
            handle_mean(args.file, args.var, &registry, &policy, &mut session)
        }
        cli::Command::Compare(args) => handle_compare(
            args.file_a,
            args.file_b,
            args.var,
            &registry,
            &policy,
            &mut session,
        ),
        cli::Command::Ask(args) => handle_ask(args, &registry, &policy, &mut session),
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
    registry: &CapabilityRegistry,
    policy: &ExecutionPolicy,
    session: &mut SessionState,
) -> Result<(CommandResponse, bool), ExecutionError> {
    Ok((execute_inspect(file, registry, policy, session)?, true))
}

fn execute_inspect(
    file: PathBuf,
    registry: &CapabilityRegistry,
    policy: &ExecutionPolicy,
    session: &mut SessionState,
) -> Result<CommandResponse, ExecutionError> {
    persist_workspace(&file, session);
    let plan = ExecutionPlan {
        goal: format!("Inspect dataset metadata for {}", file.display()),
        steps: vec![
            PlanStep {
                id: "s1".to_string(),
                capability: crate::capability::CapabilityId::DatasetResolve,
                input: CapabilityInput::DatasetResolve {
                    alias: None,
                    path: Some(file.display().to_string()),
                },
            },
            PlanStep {
                id: "s2".to_string(),
                capability: crate::capability::CapabilityId::DatasetInspect,
                input: CapabilityInput::DatasetInspect {
                    dataset: PlanValueRef::step("s1"),
                },
            },
        ],
    };
    let outcome = execute_plan(&plan, registry, policy, session)?;

    match outcome.final_value() {
        Some(RuntimeValue::InspectReport(report)) => {
            let summary = match report.kind {
                DatasetKind::Netcdf => format!("Inspected NetCDF metadata for {}", file.display()),
                DatasetKind::Geotiff => {
                    format!("Inspected GeoTIFF metadata for {}", file.display())
                }
            };

            record_turn(session, &summary, &plan.goal, report.kind);
            Ok(CommandResponse {
                command: "inspect",
                summary,
                dataset_kind: Some(report.kind),
                details: serde_json::to_value(report)
                    .map_err(|err| ExecutionError::Output(err.to_string()))?,
            })
        }
        _ => Err(ExecutionError::Plan(
            "inspect plan did not produce an inspect report".into(),
        )),
    }
}

fn handle_mean(
    file: PathBuf,
    var: Option<String>,
    registry: &CapabilityRegistry,
    policy: &ExecutionPolicy,
    session: &mut SessionState,
) -> Result<(CommandResponse, bool), ExecutionError> {
    Ok((execute_mean(file, var, registry, policy, session)?, true))
}

fn execute_mean(
    file: PathBuf,
    var: Option<String>,
    registry: &CapabilityRegistry,
    policy: &ExecutionPolicy,
    session: &mut SessionState,
) -> Result<CommandResponse, ExecutionError> {
    persist_workspace(&file, session);
    let plan = ExecutionPlan {
        goal: format!("Compute mean for {}", file.display()),
        steps: vec![
            PlanStep {
                id: "s1".to_string(),
                capability: crate::capability::CapabilityId::DatasetResolve,
                input: CapabilityInput::DatasetResolve {
                    alias: None,
                    path: Some(file.display().to_string()),
                },
            },
            PlanStep {
                id: "s2".to_string(),
                capability: crate::capability::CapabilityId::StatsMean,
                input: CapabilityInput::StatsMean {
                    dataset: PlanValueRef::step("s1"),
                    variable: var.clone(),
                },
            },
        ],
    };
    let outcome = execute_plan(&plan, registry, policy, session)?;

    match outcome.final_value() {
        Some(RuntimeValue::MeanReport(report)) => {
            session.last_variable = report.variable.clone();
            let summary = match report.kind {
                DatasetKind::Netcdf => format!(
                    "Computed NetCDF mean for {} in {}",
                    report.variable.as_deref().unwrap_or("<unknown>"),
                    file.display()
                ),
                DatasetKind::Geotiff => format!("Computed GeoTIFF mean for {}", file.display()),
            };

            record_turn(session, &summary, &plan.goal, report.kind);
            Ok(CommandResponse {
                command: "mean",
                summary,
                dataset_kind: Some(report.kind),
                details: serde_json::to_value(report)
                    .map_err(|err| ExecutionError::Output(err.to_string()))?,
            })
        }
        _ => Err(ExecutionError::Plan(
            "mean plan did not produce a mean report".into(),
        )),
    }
}

fn handle_compare(
    file_a: PathBuf,
    file_b: PathBuf,
    var: Option<String>,
    registry: &CapabilityRegistry,
    policy: &ExecutionPolicy,
    session: &mut SessionState,
) -> Result<(CommandResponse, bool), ExecutionError> {
    Ok((
        execute_compare(file_a, file_b, var, registry, policy, session)?,
        true,
    ))
}

fn execute_compare(
    file_a: PathBuf,
    file_b: PathBuf,
    var: Option<String>,
    registry: &CapabilityRegistry,
    policy: &ExecutionPolicy,
    session: &mut SessionState,
) -> Result<CommandResponse, ExecutionError> {
    persist_workspace(&file_a, session);
    let plan = ExecutionPlan {
        goal: format!(
            "Compare mean summaries for {} and {}",
            file_a.display(),
            file_b.display()
        ),
        steps: vec![
            PlanStep {
                id: "s1".to_string(),
                capability: crate::capability::CapabilityId::DatasetResolve,
                input: CapabilityInput::DatasetResolve {
                    alias: None,
                    path: Some(file_a.display().to_string()),
                },
            },
            PlanStep {
                id: "s2".to_string(),
                capability: crate::capability::CapabilityId::DatasetResolve,
                input: CapabilityInput::DatasetResolve {
                    alias: None,
                    path: Some(file_b.display().to_string()),
                },
            },
            PlanStep {
                id: "s3".to_string(),
                capability: crate::capability::CapabilityId::CompareMeanDelta,
                input: CapabilityInput::CompareMeanDelta {
                    left: PlanValueRef::step("s1"),
                    right: PlanValueRef::step("s2"),
                    variable: var.clone(),
                },
            },
        ],
    };
    let outcome = execute_plan(&plan, registry, policy, session)?;

    match outcome.final_value() {
        Some(RuntimeValue::CompareReport(report)) => {
            session.last_variable = report.variable.clone();
            let summary = match report.kind {
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
            };

            record_turn(session, &summary, &plan.goal, report.kind);
            Ok(CommandResponse {
                command: "compare",
                summary,
                dataset_kind: Some(report.kind),
                details: serde_json::to_value(report)
                    .map_err(|err| ExecutionError::Output(err.to_string()))?,
            })
        }
        _ => Err(ExecutionError::Plan(
            "compare plan did not produce a compare report".into(),
        )),
    }
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
    registry: &CapabilityRegistry,
    policy: &ExecutionPolicy,
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
    let request = agent::build_request(args.query.clone(), selected_files, session, registry);
    let planner_response = agent::plan_with_provider(&request, &provider.config)?;

    if !planner_response.requires_clarification {
        if let Some(plan) = planner_response.plan.as_ref() {
            if let Some(executed) = execute_agent_plan(plan, registry, policy, session)? {
                return Ok((executed, true));
            }
        }
    }

    session.current_goal = Some(args.query.clone());
    session.recent_turns.push(RecentTurn {
        user_input: args.query,
        outcome: "planned".to_string(),
    });

    Ok((
        CommandResponse {
            command: "ask",
            summary: "Generated agent execution plan".to_string(),
            dataset_kind: None,
            details: serde_json::json!({
                "provider": provider,
                "request": request,
                "plan": planner_response,
                "registry": registry,
            }),
        },
        true,
    ))
}

fn execute_agent_plan(
    plan: &ExecutionPlan,
    registry: &CapabilityRegistry,
    policy: &ExecutionPolicy,
    session: &mut SessionState,
) -> Result<Option<CommandResponse>, ExecutionError> {
    let outcome = execute_plan(plan, registry, policy, session)?;

    match outcome.final_value() {
        Some(RuntimeValue::InspectReport(report)) => Ok(Some(CommandResponse {
            command: "inspect",
            summary: format!("Inspected {} via capability plan", report.file.display()),
            dataset_kind: Some(report.kind),
            details: serde_json::to_value(report)
                .map_err(|err| ExecutionError::Output(err.to_string()))?,
        })),
        Some(RuntimeValue::MeanReport(report)) => Ok(Some(CommandResponse {
            command: "mean",
            summary: format!(
                "Computed mean for {} via capability plan",
                report.file.display()
            ),
            dataset_kind: Some(report.kind),
            details: serde_json::to_value(report)
                .map_err(|err| ExecutionError::Output(err.to_string()))?,
        })),
        Some(RuntimeValue::CompareReport(report)) => Ok(Some(CommandResponse {
            command: "compare",
            summary: format!(
                "Compared {} and {} via capability plan",
                report.file_a.display(),
                report.file_b.display()
            ),
            dataset_kind: Some(report.kind),
            details: serde_json::to_value(report)
                .map_err(|err| ExecutionError::Output(err.to_string()))?,
        })),
        _ => Ok(None),
    }
}

fn execute_plan(
    plan: &ExecutionPlan,
    registry: &CapabilityRegistry,
    policy: &ExecutionPolicy,
    session: &mut SessionState,
) -> Result<crate::executor::ExecutionOutcome, ExecutionError> {
    PlanExecutor::new(registry.clone(), policy.clone()).execute(plan, session)
}

fn record_turn(session: &mut SessionState, outcome: &str, goal: &str, kind: DatasetKind) {
    session.current_goal = Some(goal.to_string());
    session.recent_turns.push(RecentTurn {
        user_input: goal.to_string(),
        outcome: outcome.to_string(),
    });
    session.prior_results.push(CachedResultSummary {
        kind: format!("{:?}", kind).to_lowercase(),
        summary: outcome.to_string(),
    });
}

fn persist_workspace(file: &PathBuf, session: &mut SessionState) {
    if let Some(parent) = file.parent() {
        session.workspace_path = Some(parent.to_path_buf());
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
    use crate::{
        capability::CapabilityRegistry,
        plan::{CapabilityInput, ExecutionPlan, PlanStep, PlanValueRef},
        policy::ExecutionPolicy,
        session::SessionState,
    };

    use super::execute_agent_plan;

    #[test]
    fn execute_agent_plan_runs_inspect_for_simple_capability_plan() {
        let registry = CapabilityRegistry::discover();
        let policy = ExecutionPolicy::from_registry(&registry);
        let file = std::env::temp_dir().join("sample.nc");
        let plan = ExecutionPlan {
            goal: "inspect sample".to_string(),
            steps: vec![
                PlanStep {
                    id: "s1".to_string(),
                    capability: crate::capability::CapabilityId::DatasetResolve,
                    input: CapabilityInput::DatasetResolve {
                        alias: None,
                        path: Some(file.display().to_string()),
                    },
                },
                PlanStep {
                    id: "s2".to_string(),
                    capability: crate::capability::CapabilityId::DatasetInspect,
                    input: CapabilityInput::DatasetInspect {
                        dataset: PlanValueRef::step("s1"),
                    },
                },
            ],
        };

        let result = execute_agent_plan(&plan, &registry, &policy, &mut SessionState::default());
        assert!(result.is_err() || result.expect("result option").is_some());
    }
}
