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

pub struct OpenAiPlannerClient;

impl PlannerProviderClient for OpenAiPlannerClient {
    fn plan_json(
        &self,
        config: &ProviderConfig,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String, ExecutionError> {
        let api_key = config.api_key()?.ok_or_else(|| {
            ExecutionError::ProviderNotConfigured("OpenAI API key is missing".into())
        })?;

        let body = serde_json::json!({
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
            ],
            "response_format": { "type": "json_object" }
        });

        let client = Client::new();
        let response = client
            .post(format!(
                "{}/chat/completions",
                config.base_url.trim_end_matches('/')
            ))
            .bearer_auth(api_key)
            .json(&body)
            .send()
            .map_err(|err| ExecutionError::Provider(format!("openai request failed: {err}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .unwrap_or_else(|_| "<unable to read response body>".to_string());
            return Err(ExecutionError::Provider(format!(
                "openai request failed with status {status}: {body}"
            )));
        }

        let response: OpenAiChatResponse = response
            .json()
            .map_err(|err| ExecutionError::Provider(format!("invalid openai response: {err}")))?;

        response
            .choices
            .first()
            .and_then(|choice| choice.message.content.clone())
            .ok_or_else(|| {
                ExecutionError::Provider("openai response missing message content".into())
            })
    }
}

pub fn planner_client(provider: ProviderKind) -> Box<dyn PlannerProviderClient> {
    match provider {
        ProviderKind::OpenAi => Box::new(OpenAiPlannerClient),
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
