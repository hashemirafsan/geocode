use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    capability::{CapabilityRegistry, PlannerCapabilityDescriptor},
    engine::DatasetKind,
    plan::ExecutionPlan,
    session::SessionState,
    tools::detect_dataset_kind,
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
pub struct PlannerSessionContext {
    pub session_id: Option<String>,
    pub workspace_path: Option<String>,
    pub last_variable: Option<String>,
    pub current_goal: Option<String>,
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerRequest {
    pub user_input: String,
    pub selected_files: Vec<String>,
    pub session: PlannerSessionContext,
    pub available_capabilities: Vec<PlannerCapabilityDescriptor>,
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
    #[serde(default)]
    pub plan: Option<ExecutionPlan>,
}

pub fn build_request(
    user_input: String,
    selected_files: Vec<String>,
    session: &SessionState,
    registry: &CapabilityRegistry,
) -> PlannerRequest {
    let include_session_goal = should_include_session_goal(&user_input, &selected_files);

    PlannerRequest {
        user_input,
        selected_files,
        session: PlannerSessionContext {
            session_id: session.session_id.clone(),
            workspace_path: session
                .workspace_path
                .as_ref()
                .map(|path| path.display().to_string()),
            last_variable: session.last_variable.clone(),
            current_goal: if include_session_goal {
                session.current_goal.clone()
            } else {
                None
            },
            aliases: session
                .aliases
                .iter()
                .map(|alias| alias.alias.clone())
                .collect(),
        },
        available_capabilities: registry.planner_surface(),
    }
}

pub(crate) fn preflight_scope_check(request: &PlannerRequest) -> Option<PlannerResponse> {
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

pub(crate) fn normalize_planner_response(
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
        response.plan = None;
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
        response.plan = None;
        return response;
    }

    if response.plan.is_none() && !response.requires_clarification {
        response.requires_clarification = true;
        if response.clarification_question.is_none() {
            response.clarification_question = Some(
                "I could not build a valid execution plan from the current response. Please restate the request more explicitly with the dataset target and variable if needed."
                    .to_string(),
            );
        }
    }

    if let Some(plan) = response.plan.as_mut() {
        if plan.goal.trim().is_empty() {
            plan.goal = request.user_input.clone();
        }
    }

    if response.tool_ids.is_empty() {
        response.tool_ids = response
            .plan
            .as_ref()
            .map(|plan| {
                plan.steps
                    .iter()
                    .map(|step| step.capability.as_str().to_string())
                    .collect()
            })
            .unwrap_or_default();
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
        plan: None,
    }
}

fn should_include_session_goal(user_input: &str, selected_files: &[String]) -> bool {
    if !selected_files.is_empty() {
        return false;
    }

    let input = user_input.to_ascii_lowercase();
    ["that", "those", "same", "previous", "again", "it", "them"]
        .iter()
        .any(|token| input.split_whitespace().any(|word| word == *token))
}

pub(crate) fn deserialize_string_or_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
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

#[cfg(test)]
mod tests {
    use super::{
        build_request, normalize_planner_response, AgentIntent, PlannerRequest, PlannerResponse,
        PlannerSessionContext,
    };
    use crate::{
        capability::{CapabilityId, CapabilityRegistry},
        plan::{CapabilityInput, ExecutionPlan, PlanStep, PlanValueRef},
        session::SessionState,
    };

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
        let request = PlannerRequest {
            user_input: "how many files are there for testing".to_string(),
            selected_files: Vec::new(),
            session: PlannerSessionContext {
                session_id: None,
                workspace_path: None,
                last_variable: None,
                current_goal: None,
                aliases: Vec::new(),
            },
            available_capabilities: Vec::new(),
        };

        let response = super::preflight_scope_check(&request).expect("should reject question");
        assert!(matches!(response.intent, AgentIntent::Unknown));
        assert!(response.tool_ids.is_empty());
        assert!(response.requires_clarification);
    }

    #[test]
    fn build_request_uses_capability_registry() {
        let registry = CapabilityRegistry::discover();
        let request = build_request(
            "compare these".to_string(),
            vec!["base.nc".to_string(), "scenario.nc".to_string()],
            &SessionState::default(),
            &registry,
        );

        assert_eq!(request.selected_files, vec!["base.nc", "scenario.nc"]);
        assert!(!request.available_capabilities.is_empty());
    }

    #[test]
    fn normalize_uses_selected_files_as_target_files() {
        let request = PlannerRequest {
            user_input: "inspect this file".to_string(),
            selected_files: vec!["base.nc".to_string()],
            session: PlannerSessionContext {
                session_id: None,
                workspace_path: None,
                last_variable: None,
                current_goal: None,
                aliases: Vec::new(),
            },
            available_capabilities: Vec::new(),
        };
        let response = PlannerResponse {
            intent: AgentIntent::Inspect,
            target_files: Vec::new(),
            variable: None,
            tool_ids: vec!["dataset.inspect".to_string()],
            requires_clarification: false,
            clarification_question: None,
            plan: None,
        };

        let normalized = normalize_planner_response(&request, response);
        assert_eq!(normalized.target_files, vec!["base.nc"]);
        assert_eq!(normalized.target_files, vec!["base.nc"]);
    }

    #[test]
    fn normalize_rejects_variable_language_for_geotiff_selection() {
        let request = PlannerRequest {
            user_input: "show all variables in this file".to_string(),
            selected_files: vec!["fixture_small.tif".to_string()],
            session: PlannerSessionContext {
                session_id: None,
                workspace_path: None,
                last_variable: None,
                current_goal: None,
                aliases: Vec::new(),
            },
            available_capabilities: Vec::new(),
        };
        let response = PlannerResponse {
            intent: AgentIntent::Inspect,
            target_files: vec!["fixture_small.tif".to_string()],
            variable: None,
            tool_ids: vec!["dataset.inspect".to_string()],
            requires_clarification: false,
            clarification_question: None,
            plan: None,
        };

        let normalized = normalize_planner_response(&request, response);
        assert!(matches!(normalized.intent, AgentIntent::Unknown));
        assert!(normalized.requires_clarification);
        assert!(normalized.tool_ids.is_empty());
    }

    #[test]
    fn normalize_preserves_planner_authored_plan() {
        let request = PlannerRequest {
            user_input: "show variables".to_string(),
            selected_files: vec!["base.nc".to_string()],
            session: PlannerSessionContext {
                session_id: None,
                workspace_path: None,
                last_variable: None,
                current_goal: None,
                aliases: Vec::new(),
            },
            available_capabilities: Vec::new(),
        };
        let authored_plan = ExecutionPlan {
            goal: "show variables".to_string(),
            steps: vec![
                PlanStep {
                    id: "s1".to_string(),
                    capability: CapabilityId::DatasetResolve,
                    input: CapabilityInput::DatasetResolve {
                        alias: None,
                        path: Some("base.nc".to_string()),
                    },
                },
                PlanStep {
                    id: "s2".to_string(),
                    capability: CapabilityId::DatasetOpen,
                    input: CapabilityInput::DatasetOpen {
                        dataset: PlanValueRef::step("s1"),
                    },
                },
            ],
        };
        let response = PlannerResponse {
            intent: AgentIntent::Inspect,
            target_files: vec!["base.nc".to_string()],
            variable: None,
            tool_ids: Vec::new(),
            requires_clarification: false,
            clarification_question: None,
            plan: Some(authored_plan.clone()),
        };

        let normalized = normalize_planner_response(&request, response);
        assert_eq!(
            normalized.plan.expect("plan").steps.len(),
            authored_plan.steps.len()
        );
        assert_eq!(normalized.tool_ids, vec!["dataset.resolve", "dataset.open"]);
    }

    #[test]
    fn missing_plan_becomes_generic_clarification() {
        let request = PlannerRequest {
            user_input: "inspect this file".to_string(),
            selected_files: vec!["base.nc".to_string()],
            session: PlannerSessionContext {
                session_id: None,
                workspace_path: None,
                last_variable: None,
                current_goal: None,
                aliases: Vec::new(),
            },
            available_capabilities: Vec::new(),
        };
        let response = PlannerResponse {
            intent: AgentIntent::Inspect,
            target_files: vec!["base.nc".to_string()],
            variable: None,
            tool_ids: Vec::new(),
            requires_clarification: false,
            clarification_question: None,
            plan: None,
        };

        let normalized = normalize_planner_response(&request, response);
        assert!(normalized.requires_clarification);
        assert!(normalized.plan.is_none());
    }

    #[test]
    fn explicit_file_request_does_not_carry_stale_session_goal() {
        let registry = CapabilityRegistry::discover();
        let mut session = SessionState::default();
        session.current_goal = Some("old goal".to_string());

        let request = build_request(
            "Average Water depth Query (using depth column)".to_string(),
            vec!["base.nc".to_string()],
            &session,
            &registry,
        );

        assert!(request.session.current_goal.is_none());
    }
}
