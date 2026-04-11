use std::{fs, process::Command, time::Duration};

use serde::Deserialize;

use crate::{
    auth::load_codex_models,
    engine::ExecutionError,
    http::AuthenticatedClient,
    provider::{OpenAiAuthSource, ProviderConfig, ProviderKind},
};

#[allow(dead_code)]
pub trait PlannerProvider {
    fn name(&self) -> &'static str;
}

pub trait PlannerProviderClient {
    fn plan_json(
        &self,
        config: &ProviderConfig,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String, ExecutionError>;
}

pub struct CodexPlannerClient;

pub fn fetch_models(config: &ProviderConfig) -> Result<Vec<String>, ExecutionError> {
    if matches!(
        config.openai_auth_source,
        Some(OpenAiAuthSource::CodexBrowser | OpenAiAuthSource::CodexHeadless)
    ) {
        return load_codex_models();
    }

    let client = AuthenticatedClient::new(Duration::from_secs(30))?;

    let url = format!("{}/models", config.base_url.trim_end_matches('/'));
    let response = client.get(config, &url)?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .unwrap_or_else(|_| "<unable to read response body>".to_string());
        return Err(ExecutionError::Provider(format!(
            "failed to fetch models with status {status}: {body}"
        )));
    }

    let response: ModelListResponse = response
        .json()
        .map_err(|err| ExecutionError::Provider(format!("invalid model list response: {err}")))?;

    let mut models = response
        .data
        .into_iter()
        .map(|model| model.id)
        .collect::<Vec<_>>();
    models.sort();
    models.dedup();
    Ok(models)
}

pub struct OpenAiCompatiblePlannerClient;

impl PlannerProviderClient for OpenAiCompatiblePlannerClient {
    fn plan_json(
        &self,
        config: &ProviderConfig,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String, ExecutionError> {
        let mut body = serde_json::json!({
            "model": config.model,
            "messages": [
                {
                    "role": "system",
                    "content": system_prompt
                },
                {
                    "role": "user",
                    "content": user_prompt
                }
            ]
        });

        if matches!(config.provider, ProviderKind::OpenAi) {
            body["response_format"] = serde_json::json!({ "type": "json_object" });
        }

        if matches!(config.provider, ProviderKind::LmStudio) {
            body["temperature"] = serde_json::json!(0.0);
            body["stream"] = serde_json::json!(false);
        }

        let client = AuthenticatedClient::new(Duration::from_secs(90))?;
        let url = format!("{}/chat/completions", config.base_url.trim_end_matches('/'));

        let response = loop {
            match client.post_json(config, &url, &body) {
                Ok(response) => break response,
                Err(err) => {
                    if !matches!(config.provider, ProviderKind::LmStudio) {
                        return Err(err);
                    }

                    std::thread::sleep(Duration::from_millis(250));
                    match client.post_json(config, &url, &body) {
                        Ok(response) => break response,
                        Err(err) => {
                            return Err(err);
                        }
                    }
                }
            }
        };

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .unwrap_or_else(|_| "<unable to read response body>".to_string());
            return Err(ExecutionError::Provider(format!(
                "provider request failed with status {status}: {body}"
            )));
        }

        let response: OpenAiChatResponse = response
            .json()
            .map_err(|err| ExecutionError::Provider(format!("invalid provider response: {err}")))?;

        response
            .choices
            .first()
            .and_then(|choice| choice.message.content.clone())
            .ok_or_else(|| {
                ExecutionError::Provider("provider response missing message content".into())
            })
    }
}

impl PlannerProviderClient for CodexPlannerClient {
    fn plan_json(
        &self,
        config: &ProviderConfig,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String, ExecutionError> {
        let prompt = format!(
            "{system_prompt}\n\n{user_prompt}\n\nReturn the planner JSON as a single string field named `response`. The string itself must contain the raw planner JSON object only, with no markdown fences and no extra commentary."
        );
        let schema_path = planner_schema_file()?;
        let output_path = std::env::temp_dir().join(format!(
            "geocode-codex-last-message-{}.json",
            std::process::id()
        ));

        let status = Command::new("codex")
            .args([
                "exec",
                "--sandbox",
                "read-only",
                "--skip-git-repo-check",
                "--ephemeral",
                "--color",
                "never",
                "--output-schema",
            ])
            .arg(&schema_path)
            .args(["--output-last-message"])
            .arg(&output_path)
            .args(["-m", &config.model, "-"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                if let Some(mut stdin) = child.stdin.take() {
                    stdin.write_all(prompt.as_bytes())?;
                }
                child.wait_with_output()
            })
            .map_err(|err| {
                ExecutionError::Provider(format!("failed to run codex planner: {err}"))
            })?;

        let stderr = String::from_utf8_lossy(&status.stderr).trim().to_string();
        if !status.status.success() {
            let _ = fs::remove_file(&schema_path);
            let _ = fs::remove_file(&output_path);
            return Err(ExecutionError::Provider(format!(
                "codex planner failed: {}",
                if stderr.is_empty() {
                    format!("exit status {}", status.status)
                } else {
                    stderr
                }
            )));
        }

        let content = fs::read_to_string(&output_path).map_err(|err| {
            ExecutionError::Provider(format!("failed to read codex planner output: {err}"))
        })?;
        let _ = fs::remove_file(&schema_path);
        let _ = fs::remove_file(&output_path);

        let wrapped: CodexPlannerEnvelope = serde_json::from_str(&content).map_err(|err| {
            ExecutionError::Provider(format!("invalid codex planner envelope: {err}"))
        })?;
        Ok(wrapped.response)
    }
}

pub fn planner_client(config: &ProviderConfig) -> Box<dyn PlannerProviderClient> {
    if matches!(
        config.openai_auth_source,
        Some(OpenAiAuthSource::CodexBrowser | OpenAiAuthSource::CodexHeadless)
    ) {
        return Box::new(CodexPlannerClient);
    }

    match config.provider {
        ProviderKind::OpenAi | ProviderKind::LmStudio | ProviderKind::ZAi => {
            Box::new(OpenAiCompatiblePlannerClient)
        }
    }
}

fn planner_schema_file() -> Result<std::path::PathBuf, ExecutionError> {
    let path = std::env::temp_dir().join(format!(
        "geocode-planner-schema-{}.json",
        std::process::id()
    ));
    let schema = serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["response"],
        "properties": {
            "response": { "type": "string" }
        }
    });

    fs::write(
        &path,
        serde_json::to_vec_pretty(&schema).map_err(|err| {
            ExecutionError::Provider(format!("failed to serialize planner schema: {err}"))
        })?,
    )
    .map_err(|err| ExecutionError::Provider(format!("failed to write planner schema: {err}")))?;

    Ok(path)
}

#[derive(Debug, Deserialize)]
struct CodexPlannerEnvelope {
    response: String,
}

#[derive(Debug, Deserialize)]
struct OpenAiChatResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessage,
}

#[derive(Debug, Deserialize)]
struct OpenAiMessage {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ModelListResponse {
    data: Vec<ModelItem>,
}

#[derive(Debug, Deserialize)]
struct ModelItem {
    id: String,
}
