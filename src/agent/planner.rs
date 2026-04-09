use serde::Deserialize;
use serde_json::Value;

use crate::{
    agent::{
        prompt_builder::PromptBuilder,
        schema::{
            deserialize_string_or_vec, normalize_planner_response, preflight_scope_check,
            AgentIntent, PlannerRequest, PlannerResponse,
        },
    },
    engine::ExecutionError,
    provider::planner_client,
    provider::ProviderConfig,
};

pub fn plan_with_provider(
    request: &PlannerRequest,
    provider: &ProviderConfig,
) -> Result<PlannerResponse, ExecutionError> {
    if let Some(response) = preflight_scope_check(request) {
        return Ok(response);
    }

    let prompt_builder = PromptBuilder::new(provider.provider);
    let prompts = prompt_builder.build(request);

    let content = planner_client(provider.provider).plan_json(
        provider,
        &prompts.system_prompt,
        &prompts.user_prompt,
    )?;

    let raw: RawPlannerResponse = serde_json::from_str(&content).map_err(|err| {
        let snippet = if content.len() > 2000 {
            format!("{}...", &content[..2000])
        } else {
            content.clone()
        };
        ExecutionError::Agent(format!(
            "invalid planner json: {err}\nplanner payload:\n{snippet}"
        ))
    })?;

    let parsed_plan = match raw.plan {
        Some(value) => match serde_json::from_value(value.clone()) {
            Ok(plan) => Some(plan),
            Err(err) => {
                let snippet = match serde_json::to_string_pretty(&value) {
                    Ok(text) if text.len() > 3000 => format!("{}...", &text[..3000]),
                    Ok(text) => text,
                    Err(_) => "<unable to format raw plan>".to_string(),
                };
                return Err(ExecutionError::Agent(format!(
                    "planner returned an unparseable plan: {err}\nraw plan:\n{snippet}"
                )));
            }
        },
        None => None,
    };

    let response = PlannerResponse {
        intent: raw.intent,
        target_files: raw.target_files,
        variable: raw.variable,
        tool_ids: raw.tool_ids,
        requires_clarification: raw.requires_clarification,
        clarification_question: raw.clarification_question,
        plan: parsed_plan,
    };

    Ok(normalize_planner_response(request, response))
}

#[derive(Debug, Deserialize)]
struct RawPlannerResponse {
    intent: AgentIntent,
    #[serde(default, deserialize_with = "deserialize_string_or_vec")]
    target_files: Vec<String>,
    variable: Option<String>,
    #[serde(default, deserialize_with = "deserialize_string_or_vec")]
    tool_ids: Vec<String>,
    #[serde(default)]
    requires_clarification: bool,
    clarification_question: Option<String>,
    #[serde(default)]
    plan: Option<Value>,
}
