use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CapabilityId {
    #[serde(rename = "dataset.resolve")]
    DatasetResolve,
    #[serde(rename = "dataset.inspect")]
    DatasetInspect,
    #[serde(rename = "stats.mean")]
    StatsMean,
    #[serde(rename = "compare.mean_delta")]
    CompareMeanDelta,
    #[serde(rename = "render.scalar")]
    RenderScalar,
    #[serde(rename = "render.table")]
    RenderTable,
    #[serde(rename = "process.run_known")]
    ProcessRunKnown,
}

impl CapabilityId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DatasetResolve => "dataset.resolve",
            Self::DatasetInspect => "dataset.inspect",
            Self::StatsMean => "stats.mean",
            Self::CompareMeanDelta => "compare.mean_delta",
            Self::RenderScalar => "render.scalar",
            Self::RenderTable => "render.table",
            Self::ProcessRunKnown => "process.run_known",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "dataset.resolve" => Some(Self::DatasetResolve),
            "dataset.inspect" => Some(Self::DatasetInspect),
            "stats.mean" => Some(Self::StatsMean),
            "compare.mean_delta" => Some(Self::CompareMeanDelta),
            "render.scalar" => Some(Self::RenderScalar),
            "render.table" => Some(Self::RenderTable),
            "process.run_known" => Some(Self::ProcessRunKnown),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityKind {
    Core,
    Composite,
    Host,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingKind {
    LocalRuntime,
    KnownBinary,
    RustCrate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityBinding {
    pub kind: BindingKind,
    pub target: String,
    pub requirement: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityDescriptor {
    pub id: CapabilityId,
    pub label: String,
    pub summary: String,
    pub kind: CapabilityKind,
    pub input_type: String,
    pub output_type: String,
    pub bindings: Vec<CapabilityBinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerCapabilityDescriptor {
    pub id: CapabilityId,
    pub summary: String,
    pub kind: CapabilityKind,
    pub input_type: String,
    pub output_type: String,
}
