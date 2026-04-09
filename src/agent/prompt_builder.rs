use crate::{
    agent::schema::PlannerRequest,
    capability::{CapabilityKind, PlannerCapabilityDescriptor},
    provider::ProviderKind,
};

pub struct PromptBuilder {
    provider: ProviderKind,
}

pub struct PromptSections {
    pub system_prompt: String,
    pub user_prompt: String,
}

impl PromptBuilder {
    pub fn new(provider: ProviderKind) -> Self {
        Self { provider }
    }

    pub fn build(&self, request: &PlannerRequest) -> PromptSections {
        let system_prompt = self.build_system_prompt();
        let user_prompt = self.build_user_prompt(request);

        PromptSections {
            system_prompt,
            user_prompt,
        }
    }

    fn build_system_prompt(&self) -> String {
        match self.provider {
            ProviderKind::LmStudio => {
                "/no_think You are a strict JSON planner for a geospatial CLI. Return JSON only with no markdown. Keep the response minimal and do not include reasoning.".to_string()
            }
            ProviderKind::OpenAi => {
                "You are a strict JSON planner for a geospatial CLI. Return JSON only with no markdown.".to_string()
            }
        }
    }

    fn build_user_prompt(&self, request: &PlannerRequest) -> String {
        let request_json = if matches!(self.provider, ProviderKind::LmStudio) {
            serde_json::to_string(request).unwrap_or_else(|_| "{}".to_string())
        } else {
            serde_json::to_string_pretty(request).unwrap_or_else(|_| "{}".to_string())
        };

        let mut prompt = String::new();

        // Core instructions
        prompt.push_str(self.core_instructions());
        prompt.push_str("\n\n");

        // Capability catalog
        prompt.push_str(&self.format_capabilities(&request.available_capabilities));
        prompt.push_str("\n\n");

        // Plan structure rules
        prompt.push_str(self.plan_structure_rules());
        prompt.push_str("\n\n");

        // Response contract
        prompt.push_str(self.response_contract());
        prompt.push_str("\n\n");

        // Request JSON
        prompt.push_str("Request JSON:\n");
        prompt.push_str(&request_json);

        prompt
    }

    fn format_capabilities(&self, capabilities: &[PlannerCapabilityDescriptor]) -> String {
        let mut output = String::from("Available Capabilities:");

        for cap in capabilities {
            let kind_label = match cap.kind {
                CapabilityKind::Core => "Core",
                CapabilityKind::Composite => "Composite",
                CapabilityKind::Host => "Host",
            };

            output.push_str(&format!(
                "\n- {} ({}): {}\n  Input: {} → Output: {}",
                cap.id.as_str(),
                kind_label,
                cap.summary,
                cap.input_type,
                cap.output_type
            ));

            // Add schema from registry metadata
            if let Some(schema) = &cap.input_schema_example {
                output.push_str(&format!("\n  Schema: {}", schema));
            }

            // Add planning notes from registry metadata
            if let Some(notes) = &cap.planning_notes {
                output.push_str(&format!("\n  Note: {}", notes));
            }
        }

        output
    }

    fn core_instructions(&self) -> &'static str {
        CORE_INSTRUCTIONS
    }

    fn plan_structure_rules(&self) -> &'static str {
        PLAN_STRUCTURE
    }

    fn response_contract(&self) -> &'static str {
        RESPONSE_CONTRACT
    }
}

const CORE_INSTRUCTIONS: &str = "\
You are GeoCode's planner. Convert user requests into executable JSON plans.

Output Format:
Return a JSON object with: intent, target_files, variable, tool_ids, requires_clarification, clarification_question, plan.

Intent Guide:
- inspect: Requests to view, list, show, describe, or get metadata/structure (e.g., \"list variables\", \"show metadata\", \"what's in this file\")
- mean/min/max: Compute a specific statistic on a named variable
- stats: Compute multiple statistics at once, or ranked/ordered value queries that require array operations
- compare: Compare values or metadata across datasets
- unknown: Request doesn't fit above categories or is ambiguous

Variable Field:
- Extract the specific variable/band name mentioned in the query
- Leave null for requests that list all variables or describe metadata

Plan Construction:
- Chain capabilities by matching Input → Output types
- Start from data access (dataset.resolve → dataset.open)
- Use list capabilities for \"show all\" / \"list\" requests
- Use load capabilities when a specific variable name is mentioned
- Use array.sort and array.take for top/bottom/first/last N value queries rather than asking clarification
- The word 'first' means array.take(count=N, from_end=false)
- The word 'last' means array.take(count=N, from_end=true)
- Example compositions: top 5 = array.sort(descending=true) -> array.take(count=5); first 5 = array.take(count=5, from_end=false); last 5 = array.take(count=5, from_end=true)";

const PLAN_STRUCTURE: &str = "\
Plan Construction:
- Each step needs: id (e.g., 's1', 's2'), capability (from Available Capabilities), input (following the Schema)
- Reference previous steps: {\"type\":\"step\",\"step\":\"s1\"} or \"$s1\"
- Chain capabilities by type: capability output_type → next capability input_type
- Use the Schema examples for correct input structure
- Follow Note field constraints (they explain when/how to use capabilities)

Capability Chaining Principles:
- To access specific named data: use capabilities that accept a 'name' parameter
- To list all data: use list-type capabilities (their output is typically not used as input to other operations)
- For ranked or ordered value queries, compose load -> array.sort/array.take -> render.table or render.scalar
- If the user asks for multiple outputs in one response, finish with a single render.table step that combines all final scalar or array results
- Build the shortest valid chain from data source to final output";

const RESPONSE_CONTRACT: &str = "\
Response Validation:
- Ensure tool_ids contains all capability IDs used in your plan
- If the query is ambiguous or cannot be executed safely, set requires_clarification=true and provide a clarification_question
- Return valid JSON only - no markdown, no explanations, no execution";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::CapabilityId;

    #[test]
    fn test_format_capabilities() {
        let builder = PromptBuilder::new(ProviderKind::OpenAi);
        let capabilities = vec![
            PlannerCapabilityDescriptor {
                id: CapabilityId::DatasetResolve,
                summary: "Resolve a dataset alias or path into a dataset reference".to_string(),
                kind: CapabilityKind::Core,
                input_type: "dataset_selector".to_string(),
                output_type: "dataset_ref".to_string(),
                input_schema_example: Some(
                    r#"{"path": "file.nc"} or {"alias": "name"}"#.to_string(),
                ),
                planning_notes: None,
            },
            PlannerCapabilityDescriptor {
                id: CapabilityId::NetcdfVariableLoad,
                summary: "Load a specific NetCDF variable for analysis".to_string(),
                kind: CapabilityKind::Core,
                input_type: "dataset_handle + variable_name".to_string(),
                output_type: "variable_data".to_string(),
                input_schema_example: Some(
                    r#"{"dataset": "$s1", "name": "variable_name"}"#.to_string(),
                ),
                planning_notes: Some("Do not pass a whole variable list".to_string()),
            },
        ];

        let output = builder.format_capabilities(&capabilities);

        assert!(output.contains("Available Capabilities:"));
        assert!(output.contains("dataset.resolve (Core)"));
        assert!(output.contains("Resolve a dataset alias or path into a dataset reference"));
        assert!(output.contains("Input: dataset_selector → Output: dataset_ref"));
        assert!(output.contains(r#"Schema: {"path": "file.nc"} or {"alias": "name"}"#));
        assert!(output.contains("netcdf.variable.load (Core)"));
        assert!(output.contains("Load a specific NetCDF variable for analysis"));
        assert!(output.contains("Input: dataset_handle + variable_name → Output: variable_data"));
        assert!(output.contains(r#"Schema: {"dataset": "$s1", "name": "variable_name"}"#));
        assert!(output.contains("Note: Do not pass a whole variable list"));
    }

    #[test]
    fn test_system_prompt_lmstudio() {
        let builder = PromptBuilder::new(ProviderKind::LmStudio);
        let system_prompt = builder.build_system_prompt();

        assert!(system_prompt.starts_with("/no_think"));
        assert!(system_prompt.contains("strict JSON planner"));
    }

    #[test]
    fn test_system_prompt_openai() {
        let builder = PromptBuilder::new(ProviderKind::OpenAi);
        let system_prompt = builder.build_system_prompt();

        assert!(!system_prompt.starts_with("/no_think"));
        assert!(system_prompt.contains("strict JSON planner"));
    }

    #[test]
    fn test_build_includes_all_sections() {
        let builder = PromptBuilder::new(ProviderKind::OpenAi);
        let request = PlannerRequest {
            user_input: "what is the mean temperature?".to_string(),
            selected_files: vec!["data.nc".to_string()],
            session: crate::agent::schema::PlannerSessionContext {
                session_id: None,
                workspace_path: None,
                last_variable: None,
                current_goal: None,
                aliases: Vec::new(),
            },
            available_capabilities: vec![PlannerCapabilityDescriptor {
                id: CapabilityId::StatsMean,
                summary: "Compute arithmetic mean of variable data".to_string(),
                kind: CapabilityKind::Core,
                input_type: "variable_data".to_string(),
                output_type: "scalar_value".to_string(),
                input_schema_example: Some(r#"{"input": "$s1"}"#.to_string()),
                planning_notes: Some("Test note".to_string()),
            }],
        };

        let prompts = builder.build(&request);

        // Check system prompt
        assert!(prompts.system_prompt.contains("strict JSON planner"));

        // Check user prompt includes all sections
        assert!(prompts.user_prompt.contains("You are GeoCode's planner"));
        assert!(prompts.user_prompt.contains("Available Capabilities:"));
        assert!(prompts.user_prompt.contains("stats.mean"));
        assert!(prompts.user_prompt.contains(r#"Schema: {"input": "$s1"}"#));
        assert!(prompts.user_prompt.contains("Note: Test note"));
        assert!(prompts.user_prompt.contains("Plan Construction:"));
        assert!(prompts
            .user_prompt
            .contains("Use the Schema examples for correct input structure"));
        assert!(prompts.user_prompt.contains("Response Validation:"));
        assert!(prompts.user_prompt.contains("Request JSON:"));
        assert!(prompts
            .user_prompt
            .contains("what is the mean temperature?"));
    }
}
