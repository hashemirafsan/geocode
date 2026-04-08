use serde::{Deserialize, Serialize};

use crate::capability::CapabilityId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub goal: String,
    pub steps: Vec<PlanStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    DatasetInspect {
        dataset: PlanValueRef,
    },
    StatsMean {
        dataset: PlanValueRef,
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
        input: PlanValueRef,
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
