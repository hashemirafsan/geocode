use serde::Deserialize;
use serde_json::Value;

use crate::{
    agent::schema::{
        deserialize_string_or_vec, normalize_planner_response, preflight_scope_check, AgentIntent,
        PlannerRequest, PlannerResponse,
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

    let request_json = if matches!(provider.provider, crate::provider::ProviderKind::LmStudio) {
        serde_json::to_string(request).map_err(|err| ExecutionError::Agent(err.to_string()))?
    } else {
        serde_json::to_string_pretty(request)
            .map_err(|err| ExecutionError::Agent(err.to_string()))?
    };

    let prompt = format!(
        "You are GeoCode's planner. Convert the user request into JSON only. \
Return exactly one JSON object with keys: intent, target_files, variable, tool_ids, requires_clarification, clarification_question, plan. \
Allowed intent values: inspect, mean, compare, unknown. \
Treat the current user_input as authoritative. Use session.current_goal only when the current query is referential, such as 'that', 'same', 'previous', or 'again'. \
If explicit file selections are provided, do not ask clarification questions about prior goals unless the new query is genuinely ambiguous. \
The primary contract is the plan field. Build a typed plan using only available_capabilities. \
Tool ids must map to capability ids used by the plan. \
Each plan step must have: id, capability, input. \
Use step references like {{\"type\":\"step\",\"step\":\"s1\"}} or \"$s1\". \
Do not use foreach, action, output, final_output, loops, or free-form workflow syntax. \
Prefer the smallest executable plan that answers the current query. \
If explicit files and an explicit variable name are present, prefer execution over unnecessary clarification. \
If the query asks for multiple scalar outputs such as min and max, end with a single render.table step using an inputs array rather than multiple render.scalar steps. \
If the user asks for all variables or all parameters in a single selected dataset, that is not ambiguous: produce a plan that lists them all instead of asking clarification. \
Do not encode clarification text inside render steps; use requires_clarification and clarification_question instead. \
Capability constraints: dataset.open consumes a dataset reference and returns a dataset handle. netcdf.variable.list consumes a dataset handle and returns a variable list. netcdf.variable.describe and netcdf.variable.load require a single variable selector with a handle plus a variable name; do not pass a whole variable list into those steps. For single-file requests about all variables, shapes, and sizes, prefer dataset.inspect or dataset.open + netcdf.variable.list rather than per-variable describe steps. \
If a safe valid plan cannot be formed, set requires_clarification=true and ask one question. \
Do not execute anything.\n\nRequest JSON:\n{}",
        request_json
    );

    let system_prompt = if matches!(provider.provider, crate::provider::ProviderKind::LmStudio) {
        "/no_think You are a strict JSON planner for a geospatial CLI. Return JSON only with no markdown. Keep the response minimal and do not include reasoning.".to_string()
    } else {
        "You are a strict JSON planner for a geospatial CLI. Return JSON only with no markdown."
            .to_string()
    };

    let content = planner_client(provider.provider).plan_json(provider, &system_prompt, &prompt)?;

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
