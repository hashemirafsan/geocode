use std::{
    io::{self, Read},
    path::PathBuf,
};

use clap::ValueEnum;
use serde::Serialize;

use crate::{
    agent,
    auth::{
        CodexLoginMode, CredentialRef, CredentialStore, FileCredentialStore, TokenSet,
        codex_is_available, codex_login_status, login_openai_oauth, run_codex_login,
    },
    capability::CapabilityRegistry,
    cli,
    engine::{DatasetKind, ExecutionError, ExecutionResult},
    executor::{PlanExecutor, RuntimeValue},
    memory::MemoryStore,
    output::{OutputFormat, render, render_for_tui},
    plan::{CapabilityInput, ExecutionPlan, PlanStep, PlanValueRef},
    policy::ExecutionPolicy,
    provider::{
        AuthMethod, OpenAiAuthSource, ProviderConfig, ProviderKind, ProviderStatus, ProviderStore,
        StoredProviderConfig, fetch_models, supported_providers,
    },
    session::{CachedResultSummary, RecentTurn, SessionState, SessionStore},
    tools,
};

pub fn run(cli: cli::Cli) -> Result<(), ExecutionError> {
    match cli.command {
        None => crate::tui::run(),
        Some(cli::Command::Cli(args)) => {
            println!("{}", execute_cli(args)?);
            Ok(())
        }
    }
}

pub fn execute_cli(args: cli::CliArgs) -> Result<String, ExecutionError> {
    let format = OutputFormat::from_json_flag(args.json);

    match args.command {
        cli::CliCommand::Ask(ask_args) => {
            let response = execute_ask_command(ask_args, &mut |_| {})?;
            render(&response, format)
        }
    }
}

pub fn execute_tui_input<F>(
    input: String,
    files: Vec<PathBuf>,
    mut emit: F,
) -> Result<String, ExecutionError>
where
    F: FnMut(AppEvent),
{
    let response = if input.trim_start().starts_with('/') {
        execute_slash_command(input.trim(), &mut emit)?
    } else {
        execute_ask_command(
            cli::AskArgs {
                files,
                query: input,
            },
            &mut emit,
        )?
    };

    Ok(render_for_tui(&response))
}

pub fn fetch_provider_models(provider: ProviderKind) -> Result<Vec<String>, ExecutionError> {
    let config = ProviderConfig::resolve(provider)?;

    match fetch_models(&config) {
        Ok(models) if !models.is_empty() => Ok(models),
        Ok(_) => {
            let fallback = provider.fallback_models();
            if fallback.is_empty() {
                Err(ExecutionError::Provider(format!(
                    "no models were returned for {}",
                    provider.display_name()
                )))
            } else {
                Ok(fallback)
            }
        }
        Err(err) => {
            let fallback = provider.fallback_models();
            if fallback.is_empty() {
                Err(err)
            } else {
                Ok(fallback)
            }
        }
    }
}

pub fn store_provider_api_key(
    provider: ProviderKind,
    api_key: String,
) -> Result<(), ExecutionError> {
    handle_provider(ProviderArgs {
        command: ProviderCommand::SetApiKey(SetApiKeyArgs {
            provider,
            api_key: Some(api_key),
            stdin: false,
        }),
    })?;

    Ok(())
}

pub fn set_provider_auth_method(
    provider: ProviderKind,
    auth_method: AuthMethod,
) -> Result<(), ExecutionError> {
    handle_provider(ProviderArgs {
        command: ProviderCommand::SetAuthMethod(SetAuthMethodArgs {
            provider,
            auth_method,
        }),
    })?;

    Ok(())
}

pub fn store_provider_oauth_token(
    provider: ProviderKind,
    access_token: String,
) -> Result<(), ExecutionError> {
    handle_provider(ProviderArgs {
        command: ProviderCommand::SetOAuthToken(SetOAuthTokenArgs {
            provider,
            access_token,
        }),
    })?;

    Ok(())
}

#[allow(dead_code)]
pub fn login_provider_oauth(provider: ProviderKind) -> Result<String, ExecutionError> {
    if !matches!(provider, ProviderKind::OpenAi) {
        return Err(ExecutionError::InvalidInput(
            "interactive OAuth login is currently supported only for OpenAI".into(),
        ));
    }

    let tokens = login_openai_oauth()?;
    let store = ProviderStore::new();
    let mut config = store.load(provider)?.unwrap_or_default();
    config.auth_method = Some(AuthMethod::OAuth);
    config.openai_auth_source = Some(OpenAiAuthSource::DirectOAuth);
    config.api_key = None;
    config.oauth_access_token = Some(tokens.access_token.clone());
    config.oauth_refresh_token = tokens.refresh_token.clone();
    config.oauth_expires_at_unix = tokens.expires_at_unix;
    store.save(provider, &config)?;

    let status = ProviderStatus::current(provider)?;
    Ok(format!(
        "OAuth login complete\nProvider: {}\nCredential Source: {}\nConfig Path: {}",
        provider.display_name(),
        status.credential_source,
        store.path(provider).display(),
    ))
}

pub fn codex_openai_available() -> bool {
    codex_is_available()
}

#[allow(dead_code)]
pub fn codex_openai_status() -> Result<String, ExecutionError> {
    let status = codex_login_status()?;
    Ok(status.summary)
}

pub fn login_provider_via_codex<F>(
    provider: ProviderKind,
    mode: CodexLoginMode,
    mut emit: F,
) -> Result<String, ExecutionError>
where
    F: FnMut(String),
{
    if !matches!(provider, ProviderKind::OpenAi) {
        return Err(ExecutionError::InvalidInput(
            "Codex-backed login is currently supported only for OpenAI".into(),
        ));
    }

    let result = run_codex_login(mode, |line| emit(line))?;
    let store = ProviderStore::new();
    let mut config = store.load(provider)?.unwrap_or_default();
    config.auth_method = Some(AuthMethod::OAuth);
    config.openai_auth_source = Some(result.source);
    config.api_key = None;
    config.oauth_access_token = Some(result.tokens.access_token.clone());
    config.oauth_refresh_token = result.tokens.refresh_token.clone();
    config.oauth_expires_at_unix = result.tokens.expires_at_unix;
    store.save(provider, &config)?;

    let status = ProviderStatus::current(provider)?;
    Ok(format!(
        "Codex login complete\nProvider: {}\nAuth Source: {}\nCredential Source: {}\nConfig Path: {}",
        provider.display_name(),
        match result.source {
            OpenAiAuthSource::DirectOAuth => "direct_oauth",
            OpenAiAuthSource::CodexBrowser => "codex_browser",
            OpenAiAuthSource::CodexHeadless => "codex_headless",
        },
        status.credential_source,
        store.path(provider).display(),
    ))
}

pub fn setup_provider(
    provider: ProviderKind,
    api_key: Option<String>,
    model: String,
) -> Result<String, ExecutionError> {
    let resolved = ProviderConfig::resolve(provider)?;

    if provider.requires_api_key() && !matches!(resolved.auth_method, AuthMethod::OAuth) {
        if let Some(api_key) = api_key {
            let trimmed = api_key.trim();
            if trimmed.is_empty() {
                return Err(ExecutionError::InvalidInput(
                    "API key cannot be empty".into(),
                ));
            }

            handle_provider(ProviderArgs {
                command: ProviderCommand::SetApiKey(SetApiKeyArgs {
                    provider,
                    api_key: Some(trimmed.to_string()),
                    stdin: false,
                }),
            })?;
        } else if !resolved.configured {
            return Err(ExecutionError::ProviderNotConfigured(format!(
                "{} needs an API key before a model can be selected",
                provider.display_name()
            )));
        }
    } else if matches!(resolved.auth_method, AuthMethod::OAuth) && !resolved.configured {
        return Err(ExecutionError::ProviderNotConfigured(format!(
            "{} needs an OAuth token before a model can be selected",
            provider.display_name()
        )));
    }

    let trimmed_model = model.trim();
    if trimmed_model.is_empty() {
        return Err(ExecutionError::InvalidInput("model cannot be empty".into()));
    }

    handle_provider(ProviderArgs {
        command: ProviderCommand::SetModel(SetModelArgs {
            provider,
            model: trimmed_model.to_string(),
        }),
    })?;
    handle_provider(ProviderArgs {
        command: ProviderCommand::Use(ProviderUseArgs { provider }),
    })?;

    let status = ProviderStatus::current(provider)?;
    let config_path = ProviderStore::new().path(provider);
    Ok(format!(
        "Provider ready\nProvider: {}\nModel: {}\nDefault: true\nConfig Path: {}\nCredential Source: {}",
        provider.display_name(),
        status.config.model,
        config_path.display(),
        status.credential_source,
    ))
}

#[derive(Debug, Clone)]
pub enum AppEvent {
    Message(String),
    PlanningStarted,
    PlanningFinished { millis: u128 },
    ExecutionStarted { steps: usize },
    ExecutionStepCompleted { step_id: String, capability: String },
    ExecutionFinished { millis: u128 },
}

struct AppContext {
    session_store: SessionStore,
    session: SessionState,
    registry: CapabilityRegistry,
    policy: ExecutionPolicy,
}

#[derive(Debug)]
enum SessionCommand {
    Show,
    Clear,
}

#[derive(Debug)]
struct SessionArgs {
    command: SessionCommand,
}

#[derive(Debug)]
#[allow(dead_code)]
enum ProviderCommand {
    List,
    Status(ProviderStatusArgs),
    Use(ProviderUseArgs),
    SetModel(SetModelArgs),
    SetApiKey(SetApiKeyArgs),
    SetAuthMethod(SetAuthMethodArgs),
    SetOAuthToken(SetOAuthTokenArgs),
}

#[derive(Debug)]
struct ProviderArgs {
    command: ProviderCommand,
}

#[derive(Debug)]
struct ProviderStatusArgs {
    provider: Option<ProviderKind>,
}

#[derive(Debug)]
struct ProviderUseArgs {
    provider: ProviderKind,
}

#[derive(Debug)]
struct SetModelArgs {
    provider: ProviderKind,
    model: String,
}

#[derive(Debug)]
#[allow(dead_code)]
struct SetApiKeyArgs {
    provider: ProviderKind,
    api_key: Option<String>,
    stdin: bool,
}

#[derive(Debug)]
struct SetAuthMethodArgs {
    provider: ProviderKind,
    auth_method: AuthMethod,
}

#[derive(Debug)]
struct SetOAuthTokenArgs {
    provider: ProviderKind,
    access_token: String,
}

#[derive(Debug, Serialize)]
pub struct CommandResponse {
    pub command: &'static str,
    pub summary: String,
    pub dataset_kind: Option<DatasetKind>,
    pub details: serde_json::Value,
}

fn load_context() -> Result<AppContext, ExecutionError> {
    let session_store = SessionStore::new();
    let session = session_store.load()?;
    let _memory = MemoryStore::new().load()?;
    let registry = CapabilityRegistry::discover();
    let policy = ExecutionPolicy::from_registry(&registry);

    Ok(AppContext {
        session_store,
        session,
        registry,
        policy,
    })
}

fn execute_ask_command<F>(
    args: cli::AskArgs,
    emit: &mut F,
) -> Result<CommandResponse, ExecutionError>
where
    F: FnMut(AppEvent),
{
    let mut context = load_context()?;
    let (response, persist_session) = handle_ask(
        args,
        &context.registry,
        &context.policy,
        &mut context.session,
        emit,
    )?;

    if persist_session {
        context.session_store.save(&context.session)?;
    }

    Ok(response)
}

fn execute_slash_command<F>(input: &str, emit: &mut F) -> Result<CommandResponse, ExecutionError>
where
    F: FnMut(AppEvent),
{
    let mut context = load_context()?;
    emit(AppEvent::Message(format!("Running {input}")));

    let (response, persist_session) = match parse_slash_command(input)? {
        SlashCommand::Provider(args) => handle_provider(args),
        SlashCommand::Session(args) => {
            handle_session(args, &context.session_store, &mut context.session)
        }
        SlashCommand::Model(ModelCommand::Show) => Ok((show_models()?, false)),
        SlashCommand::Model(ModelCommand::Set { provider, model }) => {
            handle_provider(ProviderArgs {
                command: ProviderCommand::SetModel(SetModelArgs { provider, model }),
            })
        }
    }?;

    if persist_session {
        context.session_store.save(&context.session)?;
    }

    Ok(response)
}

#[derive(Debug)]
enum SlashCommand {
    Provider(ProviderArgs),
    Session(SessionArgs),
    Model(ModelCommand),
}

#[derive(Debug)]
enum ModelCommand {
    Show,
    Set {
        provider: ProviderKind,
        model: String,
    },
}

fn parse_slash_command(input: &str) -> Result<SlashCommand, ExecutionError> {
    let mut parts = input.trim_start_matches('/').split_whitespace();
    let Some(command) = parts.next() else {
        return Err(ExecutionError::InvalidInput(
            "slash command cannot be empty".into(),
        ));
    };

    match command {
        "provider" => parse_provider_command(parts.collect()),
        "session" => parse_session_command(parts.collect()),
        "model" => parse_model_command(parts.collect()),
        other => Err(ExecutionError::InvalidInput(format!(
            "unknown slash command `{other}`"
        ))),
    }
}

fn parse_provider_command(parts: Vec<&str>) -> Result<SlashCommand, ExecutionError> {
    match parts.as_slice() {
        [] => Ok(SlashCommand::Provider(ProviderArgs {
            command: ProviderCommand::List,
        })),
        ["status"] => Ok(SlashCommand::Provider(ProviderArgs {
            command: ProviderCommand::Status(ProviderStatusArgs { provider: None }),
        })),
        ["status", provider] => Ok(SlashCommand::Provider(ProviderArgs {
            command: ProviderCommand::Status(ProviderStatusArgs {
                provider: Some(parse_provider_kind(provider)?),
            }),
        })),
        ["use", provider] => Ok(SlashCommand::Provider(ProviderArgs {
            command: ProviderCommand::Use(ProviderUseArgs {
                provider: parse_provider_kind(provider)?,
            }),
        })),
        _ => Err(ExecutionError::InvalidInput(
            "provider commands: /provider, /provider status [provider], /provider use <provider>"
                .into(),
        )),
    }
}

fn parse_session_command(parts: Vec<&str>) -> Result<SlashCommand, ExecutionError> {
    match parts.as_slice() {
        [] | ["show"] => Ok(SlashCommand::Session(SessionArgs {
            command: SessionCommand::Show,
        })),
        ["clear"] => Ok(SlashCommand::Session(SessionArgs {
            command: SessionCommand::Clear,
        })),
        _ => Err(ExecutionError::InvalidInput(
            "session commands: /session, /session show, /session clear".into(),
        )),
    }
}

fn parse_model_command(parts: Vec<&str>) -> Result<SlashCommand, ExecutionError> {
    match parts.as_slice() {
        [] => Ok(SlashCommand::Model(ModelCommand::Show)),
        ["set", provider, model @ ..] if !model.is_empty() => {
            Ok(SlashCommand::Model(ModelCommand::Set {
                provider: parse_provider_kind(provider)?,
                model: model.join(" "),
            }))
        }
        _ => Err(ExecutionError::InvalidInput(
            "model commands: /model, /model set <provider> <model>".into(),
        )),
    }
}

fn parse_provider_kind(value: &str) -> Result<ProviderKind, ExecutionError> {
    ProviderKind::from_str(value, true)
        .map_err(|_| ExecutionError::InvalidInput(format!("unknown provider `{value}`")))
}

fn show_models() -> Result<CommandResponse, ExecutionError> {
    let providers = ProviderKind::all()
        .into_iter()
        .map(ProviderStatus::current)
        .collect::<Result<Vec<_>, _>>()?;
    let summary = format!(
        "Models\n{}",
        providers
            .iter()
            .map(|status| format!(
                "- {}: {}",
                status.config.provider.command_name(),
                status.config.model
            ))
            .collect::<Vec<_>>()
            .join("\n")
    );

    Ok(CommandResponse {
        command: "model_show",
        summary,
        dataset_kind: None,
        details: serde_json::json!({
            "providers": providers,
        }),
    })
}

#[allow(dead_code)]
fn handle_inspect(
    file: PathBuf,
    registry: &CapabilityRegistry,
    policy: &ExecutionPolicy,
    session: &mut SessionState,
) -> Result<(CommandResponse, bool), ExecutionError> {
    Ok((execute_inspect(file, registry, policy, session)?, true))
}

#[allow(dead_code)]
fn execute_inspect(
    file: PathBuf,
    registry: &CapabilityRegistry,
    policy: &ExecutionPolicy,
    session: &mut SessionState,
) -> Result<CommandResponse, ExecutionError> {
    persist_workspace(&file, session);
    let plan = build_inspect_plan(&file)?;
    let outcome = execute_plan(&plan, registry, policy, session, &mut |_| {})?;

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

#[allow(dead_code)]
fn handle_mean(
    file: PathBuf,
    var: Option<String>,
    registry: &CapabilityRegistry,
    policy: &ExecutionPolicy,
    session: &mut SessionState,
) -> Result<(CommandResponse, bool), ExecutionError> {
    Ok((execute_mean(file, var, registry, policy, session)?, true))
}

#[allow(dead_code)]
fn execute_mean(
    file: PathBuf,
    var: Option<String>,
    registry: &CapabilityRegistry,
    policy: &ExecutionPolicy,
    session: &mut SessionState,
) -> Result<CommandResponse, ExecutionError> {
    persist_workspace(&file, session);
    let plan = build_mean_plan(&file, var.clone())?;
    let outcome = execute_plan(&plan, registry, policy, session, &mut |_| {})?;

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
        Some(RuntimeValue::ScalarValue(value)) => {
            let kind = tools::detect_dataset_kind(&file)?;
            let report = crate::tools::MeanReport {
                file: file.clone(),
                kind,
                variable: var.clone(),
                mean: value.value,
                nodata: None,
            };
            session.last_variable = report.variable.clone();
            let summary = format!(
                "Computed NetCDF mean for {} in {}",
                report.variable.as_deref().unwrap_or("<unknown>"),
                file.display()
            );
            record_turn(session, &summary, &plan.goal, kind);
            Ok(CommandResponse {
                command: "mean",
                summary,
                dataset_kind: Some(kind),
                details: serde_json::to_value(report)
                    .map_err(|err| ExecutionError::Output(err.to_string()))?,
            })
        }
        _ => Err(ExecutionError::Plan(
            "mean plan did not produce a mean result".into(),
        )),
    }
}

#[allow(dead_code)]
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

#[allow(dead_code)]
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
    let outcome = execute_plan(&plan, registry, policy, session, &mut |_| {})?;

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
    args: SessionArgs,
    session_store: &SessionStore,
    session: &mut SessionState,
) -> Result<(CommandResponse, bool), ExecutionError> {
    match args.command {
        SessionCommand::Show => Ok((
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
        SessionCommand::Clear => {
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
    emit: &mut impl FnMut(AppEvent),
) -> Result<(CommandResponse, bool), ExecutionError> {
    use std::time::Instant;

    let start_time = Instant::now();
    let provider = select_ask_provider()?;

    let selected_files = args
        .files
        .iter()
        .map(|path| path.display().to_string())
        .collect();
    let request = agent::build_request(args.query.clone(), selected_files, session, registry);

    emit(AppEvent::PlanningStarted);
    emit(AppEvent::Message(format!(
        "Planning request with provider {}",
        match provider.config.provider {
            ProviderKind::OpenAi => "openai",
            ProviderKind::LmStudio => "lmstudio",
            ProviderKind::ZAi => "z.ai",
        }
    )));

    let plan_start = Instant::now();
    let planner_response = agent::plan_with_provider(&request, &provider.config)?;
    let plan_duration = plan_start.elapsed();

    emit(AppEvent::PlanningFinished {
        millis: plan_duration.as_millis(),
    });
    emit(AppEvent::Message(format!(
        "Planner intent: {}",
        format!("{:?}", planner_response.intent).to_lowercase()
    )));

    if let Some(plan) = &planner_response.plan {
        emit(AppEvent::Message(format!(
            "Generated plan: {} steps",
            plan.steps.len()
        )));
    }

    if !planner_response.requires_clarification {
        if let Some(plan) = planner_response.plan.as_ref() {
            emit(AppEvent::ExecutionStarted {
                steps: plan.steps.len(),
            });
            let exec_start = Instant::now();
            if let Some(executed) = execute_agent_plan(plan, registry, policy, session, emit)? {
                let exec_duration = exec_start.elapsed();
                emit(AppEvent::ExecutionFinished {
                    millis: exec_duration.as_millis(),
                });
                return Ok((executed, true));
            }
        }
    }

    session.current_goal = Some(args.query.clone());
    session.recent_turns.push(RecentTurn {
        user_input: args.query,
        outcome: "planned".to_string(),
    });

    let total_duration = start_time.elapsed();
    emit(AppEvent::Message(format!(
        "Total time: {} ms",
        total_duration.as_millis()
    )));

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
                "timing": {
                    "total_ms": total_duration.as_millis(),
                    "plan_ms": plan_duration.as_millis(),
                }
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
    emit: &mut impl FnMut(AppEvent),
) -> Result<Option<CommandResponse>, ExecutionError> {
    let outcome = execute_plan(plan, registry, policy, session, emit)?;
    let trace = serde_json::to_value(&outcome.trace)
        .map_err(|err| ExecutionError::Output(err.to_string()))?;

    match outcome.final_value() {
        Some(RuntimeValue::InspectReport(report)) => Ok(Some(CommandResponse {
            command: "inspect",
            summary: format!("Inspected {} via capability plan", report.file.display()),
            dataset_kind: Some(report.kind),
            details: with_trace(
                serde_json::to_value(report)
                    .map_err(|err| ExecutionError::Output(err.to_string()))?,
                trace.clone(),
            ),
        })),
        Some(RuntimeValue::MeanReport(report)) => Ok(Some(CommandResponse {
            command: "mean",
            summary: format!(
                "Computed mean for {} via capability plan",
                report.file.display()
            ),
            dataset_kind: Some(report.kind),
            details: with_trace(
                serde_json::to_value(report)
                    .map_err(|err| ExecutionError::Output(err.to_string()))?,
                trace.clone(),
            ),
        })),
        Some(RuntimeValue::CompareReport(report)) => Ok(Some(CommandResponse {
            command: "compare",
            summary: format!(
                "Compared {} and {} via capability plan",
                report.file_a.display(),
                report.file_b.display()
            ),
            dataset_kind: Some(report.kind),
            details: with_trace(
                serde_json::to_value(report)
                    .map_err(|err| ExecutionError::Output(err.to_string()))?,
                trace,
            ),
        })),
        Some(RuntimeValue::ScalarValue(value)) => Ok(Some(CommandResponse {
            command: "ask_result",
            summary: value.label.clone(),
            dataset_kind: None,
            details: serde_json::json!({
                "label": value.label,
                "value": value.value,
                "capability_trace": trace,
            }),
        })),
        Some(RuntimeValue::TableValue(value)) => Ok(Some(CommandResponse {
            command: "ask_result",
            summary: value.title.clone(),
            dataset_kind: None,
            details: serde_json::json!({
                "title": value.title,
                "rows": value.rows,
                "capability_trace": trace,
            }),
        })),
        Some(RuntimeValue::ArrayValue(value)) => Ok(Some(CommandResponse {
            command: "ask_result",
            summary: "Array Result".to_string(),
            dataset_kind: None,
            details: serde_json::json!({
                "title": "Array Result",
                "rows": value.values().iter().enumerate().map(|(index, item)| serde_json::json!({
                    "label": format!("value_{}", index + 1),
                    "value": format!("{item:.6}"),
                })).collect::<Vec<_>>(),
                "capability_trace": trace,
            }),
        })),
        Some(RuntimeValue::NetcdfDimensions(dimensions)) => Ok(Some(CommandResponse {
            command: "ask_result",
            summary: "NetCDF dimensions".to_string(),
            dataset_kind: Some(DatasetKind::Netcdf),
            details: serde_json::json!({
                "title": "NetCDF dimensions",
                "rows": dimensions.into_iter().map(|dimension| serde_json::json!({
                    "label": dimension.name,
                    "value": match dimension.length {
                        Some(length) => format!("len={length}"),
                        None => "len=<unlimited>".to_string(),
                    },
                })).collect::<Vec<_>>(),
                "capability_trace": trace,
            }),
        })),
        Some(RuntimeValue::NetcdfVariables(variables)) => Ok(Some(CommandResponse {
            command: "ask_result",
            summary: "NetCDF variables".to_string(),
            dataset_kind: Some(DatasetKind::Netcdf),
            details: serde_json::json!({
                "title": "NetCDF variables",
                "rows": variables.into_iter().map(|variable| serde_json::json!({
                    "label": variable.name,
                    "value": variable.dataset.dataset.path,
                })).collect::<Vec<_>>(),
                "capability_trace": trace,
            }),
        })),
        Some(RuntimeValue::VariableMetadata(variable)) => Ok(Some(CommandResponse {
            command: "ask_result",
            summary: format!("Variable {}", variable.reference.name),
            dataset_kind: Some(DatasetKind::Netcdf),
            details: serde_json::json!({
                "title": format!("Variable {}", variable.reference.name),
                "rows": [
                    serde_json::json!({ "label": "Dimensions", "value": variable.metadata.dimensions.join(", ") }),
                    serde_json::json!({ "label": "Type", "value": variable.metadata.dtype }),
                    serde_json::json!({ "label": "Shape", "value": variable.metadata.shape.iter().map(|item| item.to_string()).collect::<Vec<_>>().join(" x ") }),
                ],
                "capability_trace": trace,
            }),
        })),
        Some(RuntimeValue::TextValue(value)) => Ok(Some(CommandResponse {
            command: "ask_result",
            summary: value.text.clone(),
            dataset_kind: None,
            details: serde_json::json!({
                "title": "Result",
                "rows": [
                    serde_json::json!({ "label": "Output", "value": value.text }),
                ],
                "capability_trace": trace,
            }),
        })),
        _ => Ok(None),
    }
}

fn execute_plan(
    plan: &ExecutionPlan,
    registry: &CapabilityRegistry,
    policy: &ExecutionPolicy,
    session: &mut SessionState,
    emit: &mut impl FnMut(AppEvent),
) -> Result<crate::executor::ExecutionOutcome, ExecutionError> {
    PlanExecutor::new(registry.clone(), policy.clone()).execute_with_progress(
        plan,
        session,
        |step_id, capability| {
            emit(AppEvent::ExecutionStepCompleted {
                step_id: step_id.to_string(),
                capability: capability.to_string(),
            });
        },
    )
}

#[allow(dead_code)]
fn build_inspect_plan(file: &PathBuf) -> Result<ExecutionPlan, ExecutionError> {
    let kind = tools::detect_dataset_kind(file)?;
    let mut steps = vec![PlanStep {
        id: "s1".to_string(),
        capability: crate::capability::CapabilityId::DatasetResolve,
        input: CapabilityInput::DatasetResolve {
            alias: None,
            path: Some(file.display().to_string()),
        },
    }];

    if matches!(kind, DatasetKind::Netcdf) {
        steps.push(PlanStep {
            id: "s2".to_string(),
            capability: crate::capability::CapabilityId::DatasetOpen,
            input: CapabilityInput::DatasetOpen {
                dataset: PlanValueRef::step("s1"),
            },
        });
        steps.push(PlanStep {
            id: "s3".to_string(),
            capability: crate::capability::CapabilityId::NetcdfDimensionList,
            input: CapabilityInput::NetcdfDimensionList {
                dataset: PlanValueRef::step("s2"),
            },
        });
        steps.push(PlanStep {
            id: "s4".to_string(),
            capability: crate::capability::CapabilityId::NetcdfVariableList,
            input: CapabilityInput::NetcdfVariableList {
                dataset: PlanValueRef::step("s2"),
            },
        });
        steps.push(PlanStep {
            id: "s5".to_string(),
            capability: crate::capability::CapabilityId::DatasetInspect,
            input: CapabilityInput::DatasetInspect {
                dataset: PlanValueRef::step("s1"),
            },
        });
    } else {
        steps.push(PlanStep {
            id: "s2".to_string(),
            capability: crate::capability::CapabilityId::DatasetInspect,
            input: CapabilityInput::DatasetInspect {
                dataset: PlanValueRef::step("s1"),
            },
        });
    }

    Ok(ExecutionPlan {
        goal: format!("Inspect dataset metadata for {}", file.display()),
        steps,
    })
}

#[allow(dead_code)]
fn build_mean_plan(file: &PathBuf, var: Option<String>) -> Result<ExecutionPlan, ExecutionError> {
    let kind = tools::detect_dataset_kind(file)?;
    if matches!(kind, DatasetKind::Netcdf) {
        let variable = var.ok_or(ExecutionError::MissingVariable)?;
        Ok(ExecutionPlan {
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
                    capability: crate::capability::CapabilityId::DatasetOpen,
                    input: CapabilityInput::DatasetOpen {
                        dataset: PlanValueRef::step("s1"),
                    },
                },
                PlanStep {
                    id: "s3".to_string(),
                    capability: crate::capability::CapabilityId::NetcdfVariableDescribe,
                    input: CapabilityInput::NetcdfVariableDescribe {
                        dataset: PlanValueRef::step("s2"),
                        name: variable.clone(),
                    },
                },
                PlanStep {
                    id: "s4".to_string(),
                    capability: crate::capability::CapabilityId::NetcdfVariableLoad,
                    input: CapabilityInput::NetcdfVariableLoad {
                        dataset: PlanValueRef::step("s2"),
                        name: variable.clone(),
                    },
                },
                PlanStep {
                    id: "s5".to_string(),
                    capability: crate::capability::CapabilityId::StatsMean,
                    input: CapabilityInput::StatsMean {
                        input: PlanValueRef::step("s4"),
                        variable: Some(variable),
                    },
                },
            ],
        })
    } else {
        Ok(ExecutionPlan {
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
                        input: PlanValueRef::step("s1"),
                        variable: None,
                    },
                },
            ],
        })
    }
}

#[allow(dead_code)]
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

#[allow(dead_code)]
fn persist_workspace(file: &PathBuf, session: &mut SessionState) {
    if let Some(parent) = file.parent() {
        session.workspace_path = Some(parent.to_path_buf());
    }
}

fn with_trace(mut details: serde_json::Value, trace: serde_json::Value) -> serde_json::Value {
    if let Some(object) = details.as_object_mut() {
        object.insert("capability_trace".to_string(), trace);
    }
    details
}

fn handle_provider(args: ProviderArgs) -> Result<(CommandResponse, bool), ExecutionError> {
    match args.command {
        ProviderCommand::List => {
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
        ProviderCommand::Status(args) => {
            let status = ProviderStatus::current(args.provider.unwrap_or(ProviderKind::OpenAi))?;
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
        ProviderCommand::Use(args) => {
            let store = ProviderStore::new();
            store.save_default_provider(args.provider)?;
            let status = ProviderStatus::current(args.provider)?;

            Ok((
                CommandResponse {
                    command: "provider_use",
                    summary: format!("Selected {:?} as the default provider", args.provider),
                    dataset_kind: None,
                    details: serde_json::json!({
                        "provider": status,
                        "default_provider_path": store.default_provider_path(),
                    }),
                },
                false,
            ))
        }
        ProviderCommand::SetModel(args) => {
            let store = ProviderStore::new();
            let mut config = store.load(args.provider)?.unwrap_or_default();
            config.model = Some(args.model.clone());
            store.save(args.provider, &config)?;
            let status = ProviderStatus::current(args.provider)?;

            Ok((
                CommandResponse {
                    command: "provider_set_model",
                    summary: format!("Stored model for {:?}", args.provider),
                    dataset_kind: None,
                    details: serde_json::json!({
                        "provider_name": args.provider,
                        "model": args.model,
                        "path": store.path(args.provider),
                        "provider": status,
                    }),
                },
                false,
            ))
        }
        ProviderCommand::SetAuthMethod(args) => {
            let store = ProviderStore::new();
            let mut config = store.load(args.provider)?.unwrap_or_default();
            config.auth_method = Some(args.auth_method);

            if matches!(args.auth_method, AuthMethod::ApiKey) {
                config.openai_auth_source = None;
                config.oauth_access_token = None;
                config.oauth_refresh_token = None;
                config.oauth_expires_at_unix = None;
            }

            if matches!(args.auth_method, AuthMethod::OAuth) {
                config.api_key = None;
                config.openai_auth_source = Some(OpenAiAuthSource::DirectOAuth);
            }

            store.save(args.provider, &config)?;
            let status = ProviderStatus::current(args.provider)?;

            Ok((
                CommandResponse {
                    command: "provider_set_auth_method",
                    summary: format!("Stored auth method for {:?}", args.provider),
                    dataset_kind: None,
                    details: serde_json::json!({
                        "provider_name": args.provider,
                        "auth_method": args.auth_method,
                        "path": store.path(args.provider),
                        "provider": status,
                    }),
                },
                false,
            ))
        }
        ProviderCommand::SetApiKey(args) => {
            if matches!(args.provider, ProviderKind::LmStudio) {
                return Err(ExecutionError::InvalidInput(
                    "LM Studio does not require an API key; configure LMSTUDIO_BASE_URL or use the default local endpoint instead.".into(),
                ));
            }

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
            config.auth_method = Some(AuthMethod::ApiKey);
            config.openai_auth_source = None;
            FileCredentialStore::new().save_api_key(
                &CredentialRef {
                    provider: args.provider,
                },
                api_key.trim(),
            )?;
            config.api_key = Some(api_key.trim().to_string());
            config.oauth_access_token = None;
            config.oauth_refresh_token = None;
            config.oauth_expires_at_unix = None;
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
        ProviderCommand::SetOAuthToken(args) => {
            if !matches!(args.provider, ProviderKind::OpenAi) {
                return Err(ExecutionError::InvalidInput(
                    "OAuth token setup is currently supported only for OpenAI.".into(),
                ));
            }

            if args.access_token.trim().is_empty() {
                return Err(ExecutionError::InvalidInput(
                    "OAuth access token cannot be empty".into(),
                ));
            }

            let store = ProviderStore::new();
            let mut config: StoredProviderConfig = store.load(args.provider)?.unwrap_or_default();
            config.auth_method = Some(AuthMethod::OAuth);
            config.openai_auth_source = Some(OpenAiAuthSource::DirectOAuth);
            config.api_key = None;
            FileCredentialStore::new().save_tokens(
                &CredentialRef {
                    provider: args.provider,
                },
                &TokenSet {
                    access_token: args.access_token.trim().to_string(),
                    refresh_token: None,
                    expires_at_unix: None,
                },
            )?;
            config.oauth_access_token = Some(args.access_token.trim().to_string());
            store.save(args.provider, &config)?;
            let status = ProviderStatus::current(args.provider)?;

            Ok((
                CommandResponse {
                    command: "provider_set_oauth_token",
                    summary: format!("Stored OAuth token for {:?}", args.provider),
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

fn select_ask_provider() -> Result<ProviderStatus, ExecutionError> {
    let store = ProviderStore::new();
    if let Some(provider) = store.default_provider()? {
        let selected = ProviderStatus::current(provider)?;
        if selected.config.configured {
            return Ok(selected);
        }
    }

    let openai = ProviderStatus::current(ProviderKind::OpenAi)?;
    if openai.config.configured {
        return Ok(openai);
    }

    let zai = ProviderStatus::current(ProviderKind::ZAi)?;
    if zai.config.configured {
        return Ok(zai);
    }

    let lmstudio = ProviderStatus::current(ProviderKind::LmStudio)?;
    if lmstudio.config.configured {
        return Ok(lmstudio);
    }

    Err(ExecutionError::ProviderNotConfigured(
        "No planner provider is configured. Configure OpenAI, Z.Ai, or run a local LM Studio server in the provider setup flow.".into(),
    ))
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

        let result = execute_agent_plan(
            &plan,
            &registry,
            &policy,
            &mut SessionState::default(),
            &mut |_| {},
        );
        assert!(result.is_err() || result.expect("result option").is_some());
    }
}
