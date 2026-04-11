use serde::{Deserialize, Deserializer, Serialize, de::Error as _};
use serde_json::Value;

use crate::capability::CapabilityId;

#[derive(Debug, Clone, Serialize)]
pub struct ExecutionPlan {
    pub goal: String,
    pub steps: Vec<PlanStep>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlanStep {
    pub id: String,
    pub capability: CapabilityId,
    pub input: CapabilityInput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PlanValueRef {
    Step { step: String },
    Alias { alias: String },
    Path { path: String },
}

impl PlanValueRef {
    pub fn step(step: impl Into<String>) -> Self {
        Self::Step { step: step.into() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CapabilityInput {
    DatasetResolve {
        alias: Option<String>,
        path: Option<String>,
    },
    DatasetOpen {
        dataset: PlanValueRef,
    },
    DatasetInspect {
        dataset: PlanValueRef,
    },
    NetcdfDimensionList {
        dataset: PlanValueRef,
    },
    NetcdfVariableList {
        dataset: PlanValueRef,
    },
    NetcdfVariableDescribe {
        dataset: PlanValueRef,
        name: String,
    },
    NetcdfVariableLoad {
        dataset: PlanValueRef,
        name: String,
    },
    ArraySort {
        input: PlanValueRef,
        descending: bool,
    },
    ArrayTake {
        input: PlanValueRef,
        count: usize,
        from_end: bool,
    },
    StatsMean {
        input: PlanValueRef,
        variable: Option<String>,
    },
    StatsMin {
        input: PlanValueRef,
        variable: Option<String>,
    },
    StatsMax {
        input: PlanValueRef,
        variable: Option<String>,
    },
    CompareMeanDelta {
        left: PlanValueRef,
        right: PlanValueRef,
        variable: Option<String>,
    },
    RenderScalar {
        input: PlanValueRef,
        label: String,
    },
    RenderTable {
        inputs: Vec<PlanValueRef>,
        title: String,
    },
    ProcessRunKnown {
        binary: String,
        args: Vec<String>,
    },
}

impl ExecutionPlan {
    pub fn final_step_id(&self) -> Option<&str> {
        self.steps.last().map(|step| step.id.as_str())
    }
}

impl<'de> Deserialize<'de> for ExecutionPlan {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        parse_execution_plan(value).map_err(D::Error::custom)
    }
}

fn parse_execution_plan(value: Value) -> Result<ExecutionPlan, String> {
    if let Some(steps_value) = value.as_array() {
        let steps = steps_value
            .iter()
            .map(parse_plan_step)
            .collect::<Result<Vec<_>, _>>()?;
        return Ok(ExecutionPlan {
            goal: String::new(),
            steps,
        });
    }

    let object = value
        .as_object()
        .ok_or_else(|| "plan must be a JSON object".to_string())?;
    let goal = object
        .get("goal")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let steps = if let Some(steps) = object.get("steps") {
        parse_steps_value(steps)?
    } else if let Some(actions) = object.get("actions") {
        parse_steps_value(actions)?
    } else {
        return Err("plan.steps must be present".to_string());
    };

    Ok(ExecutionPlan { goal, steps })
}

fn parse_steps_value(value: &Value) -> Result<Vec<PlanStep>, String> {
    if let Some(steps) = value.as_array() {
        return steps.iter().map(parse_plan_step).collect();
    }

    if let Some(object) = value.as_object() {
        if object.contains_key("id")
            && (object.contains_key("capability") || object.contains_key("op"))
        {
            return Ok(vec![parse_plan_step(value)?]);
        }

        return object
            .iter()
            .map(|(id, step)| parse_plan_step_with_fallback_id(step, Some(id)))
            .collect();
    }

    Err("plan.steps must be an array or step object".to_string())
}

fn parse_plan_step(value: &Value) -> Result<PlanStep, String> {
    parse_plan_step_with_fallback_id(value, None)
}

fn parse_plan_step_with_fallback_id(
    value: &Value,
    fallback_id: Option<&str>,
) -> Result<PlanStep, String> {
    let object = value
        .as_object()
        .ok_or_else(|| "plan step must be an object".to_string())?;
    let id = object
        .get("id")
        .and_then(Value::as_str)
        .or(fallback_id)
        .ok_or_else(|| "plan step id must be a string".to_string())?
        .to_string();
    let capability_name = object
        .get("capability")
        .or_else(|| object.get("op"))
        .or_else(|| object.get("operation"))
        .or_else(|| object.get("tool"))
        .or_else(|| object.get("tool_id"))
        .or_else(|| object.get("capability_id"))
        .and_then(Value::as_str)
        .ok_or_else(|| "plan step capability/op must be a string".to_string())?;
    let capability = CapabilityId::parse(capability_name)
        .ok_or_else(|| format!("unsupported capability `{capability_name}`"))?;
    let input_value = object
        .get("input")
        .or_else(|| object.get("args"))
        .cloned()
        .unwrap_or(Value::Object(Default::default()));
    let input = parse_capability_input(capability, input_value)?;

    Ok(PlanStep {
        id,
        capability,
        input,
    })
}

fn parse_capability_input(
    capability: CapabilityId,
    value: Value,
) -> Result<CapabilityInput, String> {
    if let Ok(parsed) = serde_json::from_value::<CapabilityInput>(value.clone()) {
        return Ok(parsed);
    }

    match capability {
        CapabilityId::DatasetResolve => {
            if let Value::String(path) = &value {
                return Ok(CapabilityInput::DatasetResolve {
                    alias: None,
                    path: Some(path.clone()),
                });
            }
        }
        CapabilityId::DatasetOpen => {
            if let Ok(reference) = parse_plan_value_ref(&value) {
                return Ok(CapabilityInput::DatasetOpen { dataset: reference });
            }
        }
        CapabilityId::DatasetInspect => {
            if let Ok(reference) = parse_plan_value_ref(&value) {
                return Ok(CapabilityInput::DatasetInspect { dataset: reference });
            }
        }
        CapabilityId::NetcdfDimensionList => {
            if let Ok(reference) = parse_plan_value_ref(&value) {
                return Ok(CapabilityInput::NetcdfDimensionList { dataset: reference });
            }
        }
        CapabilityId::NetcdfVariableList => {
            if let Ok(reference) = parse_plan_value_ref(&value) {
                return Ok(CapabilityInput::NetcdfVariableList { dataset: reference });
            }
        }
        CapabilityId::StatsMean => {
            if let Ok(reference) = parse_plan_value_ref(&value) {
                return Ok(CapabilityInput::StatsMean {
                    input: reference,
                    variable: None,
                });
            }
        }
        CapabilityId::ArraySort => {
            if let Ok(reference) = parse_plan_value_ref(&value) {
                return Ok(CapabilityInput::ArraySort {
                    input: reference,
                    descending: true,
                });
            }
        }
        CapabilityId::ArrayTake => {
            if let Ok(reference) = parse_plan_value_ref(&value) {
                return Ok(CapabilityInput::ArrayTake {
                    input: reference,
                    count: 5,
                    from_end: false,
                });
            }
        }
        CapabilityId::StatsMin => {
            if let Ok(reference) = parse_plan_value_ref(&value) {
                return Ok(CapabilityInput::StatsMin {
                    input: reference,
                    variable: None,
                });
            }
        }
        CapabilityId::StatsMax => {
            if let Ok(reference) = parse_plan_value_ref(&value) {
                return Ok(CapabilityInput::StatsMax {
                    input: reference,
                    variable: None,
                });
            }
        }
        CapabilityId::RenderTable => {
            if let Some(inputs) = value
                .as_array()
                .map(|items| {
                    items
                        .iter()
                        .map(parse_plan_value_ref)
                        .collect::<Result<Vec<_>, _>>()
                })
                .transpose()?
            {
                return Ok(CapabilityInput::RenderTable {
                    inputs,
                    title: "Result".to_string(),
                });
            }
        }
        _ => {}
    }

    let object = value
        .as_object()
        .ok_or_else(|| format!("input for {} must be an object", capability.as_str()))?;

    match capability {
        CapabilityId::DatasetResolve => {
            let alias = string_field(object, "alias")
                .or_else(|| string_field(object, "dataset_alias"))
                .or_else(|| string_field(object, "name"));
            let path = string_field(object, "path")
                .or_else(|| string_field(object, "file"))
                .or_else(|| string_field(object, "file_path"))
                .or_else(|| string_field(object, "dataset"))
                .or_else(|| string_field(object, "dataset_path"))
                .or_else(|| string_field(object, "selector"))
                .or_else(|| string_field(object, "target_file"))
                .or_else(|| string_field(object, "target_path"));

            if alias.is_none() && path.is_none() {
                return Err(format!(
                    "dataset.resolve input is missing alias/path fields: {}",
                    serde_json::to_string(object)
                        .unwrap_or_else(|_| "<unserializable input>".to_string())
                ));
            }

            Ok(CapabilityInput::DatasetResolve { alias, path })
        }
        CapabilityId::DatasetOpen => Ok(CapabilityInput::DatasetOpen {
            dataset: value_ref_field(object, "dataset")
                .or_else(|_| value_ref_field(object, "dataset_ref"))
                .or_else(|_| value_ref_field(object, "ref"))?,
        }),
        CapabilityId::DatasetInspect => Ok(CapabilityInput::DatasetInspect {
            dataset: value_ref_field(object, "dataset")
                .or_else(|_| value_ref_field(object, "dataset_ref"))
                .or_else(|_| value_ref_field(object, "ref"))?,
        }),
        CapabilityId::NetcdfDimensionList => Ok(CapabilityInput::NetcdfDimensionList {
            dataset: value_ref_field(object, "dataset")
                .or_else(|_| value_ref_field(object, "handle"))
                .or_else(|_| value_ref_field(object, "dataset_handle"))?,
        }),
        CapabilityId::NetcdfVariableList => Ok(CapabilityInput::NetcdfVariableList {
            dataset: value_ref_field(object, "dataset")
                .or_else(|_| value_ref_field(object, "handle"))
                .or_else(|_| value_ref_field(object, "dataset_handle"))?,
        }),
        CapabilityId::NetcdfVariableDescribe => Ok(CapabilityInput::NetcdfVariableDescribe {
            dataset: value_ref_field(object, "dataset")
                .or_else(|_| value_ref_field(object, "handle"))
                .or_else(|_| value_ref_field_in_object(object, "variable_selector", "handle"))
                .or_else(|_| value_ref_field_in_object(object, "selector", "handle"))
                .or_else(|_| value_ref_field_in_object(object, "selector", "dataset_ref"))?,
            name: string_field(object, "name")
                .or_else(|| string_field_in_object(object, "variable_selector", "name"))
                .or_else(|| string_field_in_object(object, "selector", "name"))
                .or_else(|| string_field_in_object(object, "selector", "variable_name"))
                .ok_or_else(|| "missing `name` string field".to_string())?,
        }),
        CapabilityId::NetcdfVariableLoad => Ok(CapabilityInput::NetcdfVariableLoad {
            dataset: value_ref_field(object, "dataset")
                .or_else(|_| value_ref_field(object, "handle"))
                .or_else(|_| value_ref_field(object, "variable_ref"))
                .or_else(|_| value_ref_field_in_object(object, "variable_selector", "handle"))
                .or_else(|_| value_ref_field_in_object(object, "selector", "handle"))
                .or_else(|_| value_ref_field_in_object(object, "selector", "dataset_ref"))?,
            name: string_field(object, "name")
                .or_else(|| string_field_in_object(object, "variable_selector", "name"))
                .or_else(|| string_field_in_object(object, "selector", "name"))
                .or_else(|| string_field_in_object(object, "selector", "variable_name"))
                .unwrap_or_default(),
        }),
        CapabilityId::ArraySort => Ok(CapabilityInput::ArraySort {
            input: value_ref_field(object, "input")
                .or_else(|_| value_ref_field(object, "target"))
                .or_else(|_| value_ref_field(object, "array_value"))?,
            descending: bool_field(object, &["descending", "largest", "top"]).unwrap_or(true),
        }),
        CapabilityId::ArrayTake => Ok(CapabilityInput::ArrayTake {
            input: value_ref_field(object, "input")
                .or_else(|_| value_ref_field(object, "target"))
                .or_else(|_| value_ref_field(object, "array_value"))?,
            count: usize_field(object, &["count", "k", "n", "limit"]).unwrap_or(5),
            from_end: bool_field(object, &["from_end", "last", "tail"]).unwrap_or(false),
        }),
        CapabilityId::StatsMean => Ok(CapabilityInput::StatsMean {
            input: value_ref_field(object, "input")
                .or_else(|_| value_ref_field(object, "dataset"))
                .or_else(|_| value_ref_field(object, "target"))?,
            variable: string_field(object, "variable"),
        }),
        CapabilityId::StatsMin => Ok(CapabilityInput::StatsMin {
            input: value_ref_field(object, "input")
                .or_else(|_| value_ref_field(object, "dataset"))
                .or_else(|_| value_ref_field(object, "target"))
                .or_else(|_| value_ref_field(object, "array_value"))?,
            variable: string_field(object, "variable"),
        }),
        CapabilityId::StatsMax => Ok(CapabilityInput::StatsMax {
            input: value_ref_field(object, "input")
                .or_else(|_| value_ref_field(object, "dataset"))
                .or_else(|_| value_ref_field(object, "target"))
                .or_else(|_| value_ref_field(object, "array_value"))?,
            variable: string_field(object, "variable"),
        }),
        CapabilityId::CompareMeanDelta => Ok(CapabilityInput::CompareMeanDelta {
            left: value_ref_field(object, "left")?,
            right: value_ref_field(object, "right")?,
            variable: string_field(object, "variable"),
        }),
        CapabilityId::RenderScalar => Ok(CapabilityInput::RenderScalar {
            input: value_ref_field(object, "input")
                .or_else(|_| value_ref_field(object, "value"))
                .or_else(|_| value_ref_field(object, "scalar_like_value"))?,
            label: string_field(object, "label").unwrap_or_else(|| "Result".to_string()),
        }),
        CapabilityId::RenderTable => Ok(CapabilityInput::RenderTable {
            inputs: value_ref_list_field(object, &["inputs", "values"]).or_else(|_| {
                value_ref_field(object, "input")
                    .or_else(|_| value_ref_field(object, "value"))
                    .or_else(|_| value_ref_field(object, "table_like_value"))
                    .map(|input| vec![input])
            })?,
            title: string_field(object, "title").unwrap_or_else(|| "Result".to_string()),
        }),
        CapabilityId::ProcessRunKnown => Ok(CapabilityInput::ProcessRunKnown {
            binary: required_string_field(object, "binary")?,
            args: object
                .get("args")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str().map(ToString::to_string))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
        }),
    }
}

fn value_ref_field(
    object: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<PlanValueRef, String> {
    let value = object
        .get(key)
        .ok_or_else(|| format!("missing `{key}` reference"))?;
    parse_plan_value_ref(value)
}

fn value_ref_list_field(
    object: &serde_json::Map<String, Value>,
    keys: &[&str],
) -> Result<Vec<PlanValueRef>, String> {
    for key in keys {
        if let Some(value) = object.get(*key) {
            let items = value
                .as_array()
                .ok_or_else(|| format!("`{key}` must be an array of references"))?;
            return items.iter().map(parse_plan_value_ref).collect();
        }
    }

    Err(format!("missing one of {:?} reference lists", keys))
}

fn value_ref_field_in_object(
    object: &serde_json::Map<String, Value>,
    parent_key: &str,
    child_key: &str,
) -> Result<PlanValueRef, String> {
    let nested = object
        .get(parent_key)
        .and_then(Value::as_object)
        .ok_or_else(|| format!("missing `{parent_key}` object"))?;
    value_ref_field(nested, child_key)
}

fn parse_plan_value_ref(value: &Value) -> Result<PlanValueRef, String> {
    if let Ok(parsed) = serde_json::from_value::<PlanValueRef>(value.clone()) {
        return Ok(parsed);
    }

    match value {
        Value::Array(items) if items.len() == 1 => parse_plan_value_ref(&items[0]),
        Value::String(text) if text.starts_with('$') => Ok(PlanValueRef::Step {
            step: text.trim_start_matches('$').to_string(),
        }),
        Value::String(text) => Ok(PlanValueRef::Path { path: text.clone() }),
        Value::Object(object) => {
            if let Some(step) = object.get("step").and_then(Value::as_str) {
                return Ok(PlanValueRef::Step {
                    step: step.trim_start_matches('$').to_string(),
                });
            }
            for alias_key in ["scalar", "value", "input", "ref", "dataset_ref", "handle"] {
                if let Some(alias_value) = object.get(alias_key) {
                    return parse_plan_value_ref(alias_value);
                }
            }
            if let Some(alias) = object.get("alias").and_then(Value::as_str) {
                return Ok(PlanValueRef::Alias {
                    alias: alias.to_string(),
                });
            }
            if let Some(path) = object.get("path").and_then(Value::as_str) {
                return Ok(PlanValueRef::Path {
                    path: path.to_string(),
                });
            }
            Err("unsupported value reference object".to_string())
        }
        _ => Err("unsupported value reference".to_string()),
    }
}

fn string_field(object: &serde_json::Map<String, Value>, key: &str) -> Option<String> {
    object.get(key).and_then(loose_string_value)
}

fn string_field_in_object(
    object: &serde_json::Map<String, Value>,
    parent_key: &str,
    child_key: &str,
) -> Option<String> {
    object
        .get(parent_key)
        .and_then(Value::as_object)
        .and_then(|nested| string_field(nested, child_key))
}

fn loose_string_value(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Array(items) if items.len() == 1 => loose_string_value(&items[0]),
        Value::Object(object) => {
            for key in [
                "path",
                "file",
                "file_path",
                "dataset",
                "dataset_path",
                "target_file",
                "target_path",
                "dataset_ref",
                "ref",
                "name",
                "alias",
                "dataset_alias",
                "variable_name",
            ] {
                if let Some(value) = object.get(key).and_then(loose_string_value) {
                    return Some(value);
                }
            }
            None
        }
        _ => None,
    }
}

fn required_string_field(
    object: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<String, String> {
    string_field(object, key).ok_or_else(|| format!("missing `{key}` string field"))
}

fn usize_field(object: &serde_json::Map<String, Value>, keys: &[&str]) -> Option<usize> {
    keys.iter().find_map(|key| {
        object.get(*key).and_then(|value| match value {
            Value::Number(number) => number.as_u64().map(|n| n as usize),
            Value::String(text) => text.parse::<usize>().ok(),
            Value::Array(items) if items.len() == 1 => {
                items[0].as_u64().map(|n| n as usize).or_else(|| {
                    items[0]
                        .as_str()
                        .and_then(|text| text.parse::<usize>().ok())
                })
            }
            _ => None,
        })
    })
}

fn bool_field(object: &serde_json::Map<String, Value>, keys: &[&str]) -> Option<bool> {
    keys.iter().find_map(|key| {
        object.get(*key).and_then(|value| match value {
            Value::Bool(flag) => Some(*flag),
            Value::String(text) => match text.to_ascii_lowercase().as_str() {
                "true" | "yes" | "top" | "largest" | "desc" | "descending" => Some(true),
                "false" | "no" | "bottom" | "smallest" | "asc" | "ascending" => Some(false),
                _ => None,
            },
            _ => None,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::{CapabilityId, CapabilityInput, ExecutionPlan, PlanValueRef};

    #[test]
    fn execution_plan_accepts_relaxed_planner_shape() {
        let plan: ExecutionPlan = serde_json::from_str(
            r#"{
                "goal": "show variables",
                "steps": [
                    {
                        "id": "s1",
                        "op": "dataset.resolve",
                        "args": { "path": "base.nc" }
                    },
                    {
                        "id": "s2",
                        "op": "dataset.open",
                        "args": { "dataset": "$s1" }
                    },
                    {
                        "id": "s3",
                        "op": "netcdf.variable.describe",
                        "args": { "dataset": "$s2", "name": "depth" }
                    }
                ]
            }"#,
        )
        .expect("plan should parse");

        assert_eq!(plan.steps.len(), 3);
        assert!(matches!(
            plan.steps[0].capability,
            CapabilityId::DatasetResolve
        ));
        assert!(matches!(
            plan.steps[1].input,
            CapabilityInput::DatasetOpen { .. }
        ));
        assert!(matches!(
            plan.steps[2].input,
            CapabilityInput::NetcdfVariableDescribe {
                dataset: PlanValueRef::Step { .. },
                ..
            }
        ));
    }
}
