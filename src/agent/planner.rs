use crate::{
    agent::schema::{
        normalize_planner_response, preflight_scope_check, PlannerRequest, PlannerResponse,
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

    let prompt = format!(
        "You are GeoCode's planner. Convert the user request into JSON only. \
Return exactly one JSON object with keys: intent, target_files, variable, tool_ids, requires_clarification, clarification_question. \
Allowed intent values: inspect, mean, compare, unknown. \
Tool ids must map to available capability ids when possible. \
Do not execute anything.\n\nRequest JSON:\n{}",
        serde_json::to_string_pretty(request)
            .map_err(|err| ExecutionError::Agent(err.to_string()))?
    );

    let content = planner_client(provider.provider).plan_json(
        provider,
        "You are a strict JSON planner for a geospatial CLI. Return JSON only with no markdown.",
        &prompt,
    )?;

    let response: PlannerResponse = serde_json::from_str(&content)
        .map_err(|err| ExecutionError::Agent(format!("invalid planner json: {err}")))?;

    Ok(normalize_planner_response(request, response))
}
