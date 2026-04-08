use reqwest::blocking::Client;
use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    engine::DatasetKind,
    engine::ExecutionError,
    provider::{ProviderConfig, ProviderKind},
    session::SessionState,
    tools::{detect_dataset_kind, ToolDescriptor},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentIntent {
    Inspect,
    Mean,
    Compare,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerRequest {
    pub user_input: String,
    pub selected_files: Vec<String>,
    pub session_id: Option<String>,
    pub workspace_path: Option<String>,
    pub last_variable: Option<String>,
    pub available_tools: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerResponse {
    pub intent: AgentIntent,
    #[serde(default, deserialize_with = "deserialize_string_or_vec")]
    pub target_files: Vec<String>,
    pub variable: Option<String>,
    #[serde(default, deserialize_with = "deserialize_string_or_vec")]
    pub tool_ids: Vec<String>,
    #[serde(default)]
    pub requires_clarification: bool,
    pub clarification_question: Option<String>,
}

pub fn build_request(
    user_input: String,
    selected_files: Vec<String>,
    session: &SessionState,
    tools: &[ToolDescriptor],
) -> PlannerRequest {
    PlannerRequest {
        user_input,
        selected_files,
        session_id: session.session_id.clone(),
        workspace_path: session
            .workspace_path
            .as_ref()
            .map(|path| path.display().to_string()),
        last_variable: session.last_variable.clone(),
        available_tools: tools.iter().map(|tool| tool.id.to_string()).collect(),
    }
}

pub fn plan_with_provider(
    request: &PlannerRequest,
    provider: &ProviderConfig,
) -> Result<PlannerResponse, ExecutionError> {
    if let Some(response) = preflight_scope_check(request) {
        return Ok(response);
    }

    match provider.provider {
        ProviderKind::OpenAi => plan_with_openai(request, provider),
    }
}

fn plan_with_openai(
    request: &PlannerRequest,
    provider: &ProviderConfig,
) -> Result<PlannerResponse, ExecutionError> {
    let api_key = provider
        .api_key()?
        .ok_or_else(|| ExecutionError::ProviderNotConfigured("OpenAI API key is missing".into()))?;

    let prompt = format!(
        "You are GeoCode's planner. Convert the user request into JSON only. \
Return exactly one JSON object with keys: intent, target_files, variable, tool_ids, requires_clarification, clarification_question. \
Allowed intent values: inspect, mean, compare, unknown. \
Tool ids must come from the available_tools list. \
Do not execute anything.\n\nRequest JSON:\n{}",
        serde_json::to_string_pretty(request)
            .map_err(|err| ExecutionError::Agent(err.to_string()))?
    );

    let body = serde_json::json!({
        "model": provider.model,
        "messages": [
            {
                "role": "system",
                "content": "You are a strict JSON planner for a geospatial CLI. Return JSON only with no markdown."
            },
            {
                "role": "user",
                "content": prompt
            }
        ],
        "response_format": { "type": "json_object" }
    });

    let client = Client::new();
    let response = client
        .post(format!(
            "{}/chat/completions",
            provider.base_url.trim_end_matches('/')
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

    let content = response
        .choices
        .first()
        .and_then(|choice| choice.message.content.as_deref())
        .ok_or_else(|| {
            ExecutionError::Provider("openai response missing message content".into())
        })?;

    let response: PlannerResponse = serde_json::from_str(content)
        .map_err(|err| ExecutionError::Agent(format!("invalid planner json: {err}")))?;

    Ok(normalize_planner_response(request, response))
}

fn preflight_scope_check(request: &PlannerRequest) -> Option<PlannerResponse> {
    let input = request.user_input.to_ascii_lowercase();
    let filesystem_cues = [
        "how many files",
        "list files",
        "count files",
        "folder",
        "directory",
        "workspace files",
        "testing",
    ];
    let geospatial_cues = [
        "inspect",
        "metadata",
        "variable",
        "variables",
        "mean",
        "average",
        "compare",
        ".nc",
        ".tif",
        ".tiff",
        "netcdf",
        "geotiff",
        "raster",
        "band",
    ];

    let looks_like_filesystem_question = filesystem_cues.iter().any(|cue| input.contains(cue));
    let looks_like_geospatial_request = geospatial_cues.iter().any(|cue| input.contains(cue));

    if looks_like_filesystem_question && !looks_like_geospatial_request {
        return Some(unsupported_scope_response());
    }

    None
}

fn normalize_planner_response(
    request: &PlannerRequest,
    mut response: PlannerResponse,
) -> PlannerResponse {
    if preflight_scope_check(request).is_some() {
        return unsupported_scope_response();
    }

    if response.target_files.is_empty() && !request.selected_files.is_empty() {
        response.target_files = request.selected_files.clone();
    }

    if query_mentions_variables(&request.user_input)
        && selected_files_are_all_geotiff(&request.selected_files)
    {
        response.intent = AgentIntent::Unknown;
        response.requires_clarification = true;
        response.clarification_question = Some(
            "The selected file is a GeoTIFF, which exposes raster bands rather than NetCDF-style variables. Do you want band metadata for this file?"
                .to_string(),
        );
        response.tool_ids.clear();
        response.variable = None;
        return response;
    }

    if matches!(response.intent, AgentIntent::Unknown) && !response.requires_clarification {
        response.requires_clarification = true;
        if response.clarification_question.is_none() {
            response.clarification_question = Some(
                "I can currently plan inspect, mean, and compare requests for NetCDF and GeoTIFF inputs only."
                    .to_string(),
            );
        }
        response.tool_ids.clear();
        response.target_files.clear();
    }

    response
}

fn query_mentions_variables(input: &str) -> bool {
    let input = input.to_ascii_lowercase();
    input.contains(" variable")
        || input.contains("variables")
        || input.starts_with("variable")
        || input.starts_with("variables")
}

fn selected_files_are_all_geotiff(selected_files: &[String]) -> bool {
    if selected_files.is_empty() {
        return false;
    }

    selected_files.iter().all(|path| {
        detect_dataset_kind(std::path::Path::new(path))
            .map(|kind| matches!(kind, DatasetKind::Geotiff))
            .unwrap_or(false)
    })
}

fn unsupported_scope_response() -> PlannerResponse {
    PlannerResponse {
        intent: AgentIntent::Unknown,
        target_files: Vec::new(),
        variable: None,
        tool_ids: Vec::new(),
        requires_clarification: true,
        clarification_question: Some(
            "I can currently plan inspect, mean, and compare requests for NetCDF and GeoTIFF inputs only; I cannot answer general filesystem questions yet."
                .to_string(),
        ),
    }
}

fn deserialize_string_or_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrVec {
        String(String),
        Vec(Vec<String>),
        Null(()),
    }

    match StringOrVec::deserialize(deserializer)? {
        StringOrVec::String(value) => Ok(vec![value]),
        StringOrVec::Vec(values) => Ok(values),
        StringOrVec::Null(_) => Ok(Vec::new()),
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

#[cfg(test)]
mod tests {
    use super::{AgentIntent, PlannerResponse};

    #[test]
    fn planner_response_accepts_string_target_files() {
        let response: PlannerResponse = serde_json::from_str(
            r#"{
                "intent": "unknown",
                "target_files": "testing",
                "variable": null,
                "tool_ids": "inspect_metadata",
                "requires_clarification": true,
                "clarification_question": "Which file do you mean?"
            }"#,
        )
        .expect("planner response should parse");

        assert!(matches!(response.intent, AgentIntent::Unknown));
        assert_eq!(response.target_files, vec!["testing"]);
        assert_eq!(response.tool_ids, vec!["inspect_metadata"]);
        assert!(response.requires_clarification);
    }

    #[test]
    fn planner_response_defaults_missing_lists() {
        let response: PlannerResponse = serde_json::from_str(
            r#"{
                "intent": "inspect",
                "variable": null,
                "requires_clarification": false,
                "clarification_question": null
            }"#,
        )
        .expect("planner response should parse");

        assert!(matches!(response.intent, AgentIntent::Inspect));
        assert!(response.target_files.is_empty());
        assert!(response.tool_ids.is_empty());
    }

    #[test]
    fn preflight_rejects_filesystem_questions_outside_scope() {
        let request = super::PlannerRequest {
            user_input: "how many files are there for testing".to_string(),
            selected_files: Vec::new(),
            session_id: None,
            workspace_path: None,
            last_variable: None,
            available_tools: vec!["inspect_metadata".to_string()],
        };

        let response = super::preflight_scope_check(&request).expect("should reject question");
        assert!(matches!(response.intent, AgentIntent::Unknown));
        assert!(response.tool_ids.is_empty());
        assert!(response.requires_clarification);
    }

    #[test]
    fn build_request_carries_selected_files() {
        let request = super::build_request(
            "compare these".to_string(),
            vec!["base.nc".to_string(), "scenario.nc".to_string()],
            &crate::session::SessionState::default(),
            &[],
        );

        assert_eq!(request.selected_files, vec!["base.nc", "scenario.nc"]);
    }

    #[test]
    fn normalize_uses_selected_files_as_target_files() {
        let request = super::PlannerRequest {
            user_input: "inspect this file".to_string(),
            selected_files: vec!["base.nc".to_string()],
            session_id: None,
            workspace_path: None,
            last_variable: None,
            available_tools: vec!["inspect_metadata".to_string()],
        };
        let response = super::PlannerResponse {
            intent: AgentIntent::Inspect,
            target_files: Vec::new(),
            variable: None,
            tool_ids: vec!["inspect_metadata".to_string()],
            requires_clarification: false,
            clarification_question: None,
        };

        let normalized = super::normalize_planner_response(&request, response);
        assert_eq!(normalized.target_files, vec!["base.nc"]);
    }

    #[test]
    fn normalize_rejects_variable_language_for_geotiff_selection() {
        let request = super::PlannerRequest {
            user_input: "show all variables in this file".to_string(),
            selected_files: vec!["fixture_small.tif".to_string()],
            session_id: None,
            workspace_path: None,
            last_variable: None,
            available_tools: vec!["inspect_metadata".to_string()],
        };
        let response = super::PlannerResponse {
            intent: AgentIntent::Inspect,
            target_files: vec!["fixture_small.tif".to_string()],
            variable: None,
            tool_ids: vec!["inspect_metadata".to_string()],
            requires_clarification: false,
            clarification_question: None,
        };

        let normalized = super::normalize_planner_response(&request, response);
        assert!(matches!(normalized.intent, AgentIntent::Unknown));
        assert!(normalized.requires_clarification);
        assert!(normalized.tool_ids.is_empty());
    }
}
