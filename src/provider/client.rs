use std::time::Duration;

use reqwest::blocking::Client;
use serde::Deserialize;

use crate::{
    engine::ExecutionError,
    provider::{ProviderConfig, ProviderKind},
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

        let client = Client::builder()
            .timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(0)
            .tcp_keepalive(None)
            .build()
            .map_err(|err| {
                ExecutionError::Provider(format!("failed to build provider client: {err}"))
            })?;
        let url = format!("{}/chat/completions", config.base_url.trim_end_matches('/'));

        let response = loop {
            let request = client.post(&url);
            let request = if let Some(api_key) = config.api_key()? {
                request.bearer_auth(api_key)
            } else {
                request
            };

            match request.json(&body).send() {
                Ok(response) => break response,
                Err(err) => {
                    if !matches!(config.provider, ProviderKind::LmStudio) {
                        return Err(ExecutionError::Provider(format!(
                            "provider request failed: {err:?}"
                        )));
                    }

                    std::thread::sleep(Duration::from_millis(250));
                    let request = client.post(&url);
                    let request = if let Some(api_key) = config.api_key()? {
                        request.bearer_auth(api_key)
                    } else {
                        request
                    };

                    match request.json(&body).send() {
                        Ok(response) => break response,
                        Err(err) => {
                            return Err(ExecutionError::Provider(format!(
                                "provider request failed: {err:?}"
                            )))
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

pub fn planner_client(provider: ProviderKind) -> Box<dyn PlannerProviderClient> {
    match provider {
        ProviderKind::OpenAi | ProviderKind::LmStudio => Box::new(OpenAiCompatiblePlannerClient),
    }
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
