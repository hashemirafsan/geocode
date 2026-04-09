use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CapabilityId {
    #[serde(rename = "dataset.resolve")]
    DatasetResolve,
    #[serde(rename = "dataset.open")]
    DatasetOpen,
    #[serde(rename = "dataset.inspect")]
    DatasetInspect,
    #[serde(rename = "netcdf.dimension.list")]
    NetcdfDimensionList,
    #[serde(rename = "netcdf.variable.list")]
    NetcdfVariableList,
    #[serde(rename = "netcdf.variable.describe")]
    NetcdfVariableDescribe,
    #[serde(rename = "netcdf.variable.load")]
    NetcdfVariableLoad,
    #[serde(rename = "stats.mean")]
    StatsMean,
    #[serde(rename = "stats.min")]
    StatsMin,
    #[serde(rename = "stats.max")]
    StatsMax,
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
            Self::DatasetOpen => "dataset.open",
            Self::DatasetInspect => "dataset.inspect",
            Self::NetcdfDimensionList => "netcdf.dimension.list",
            Self::NetcdfVariableList => "netcdf.variable.list",
            Self::NetcdfVariableDescribe => "netcdf.variable.describe",
            Self::NetcdfVariableLoad => "netcdf.variable.load",
            Self::StatsMean => "stats.mean",
            Self::StatsMin => "stats.min",
            Self::StatsMax => "stats.max",
            Self::CompareMeanDelta => "compare.mean_delta",
            Self::RenderScalar => "render.scalar",
            Self::RenderTable => "render.table",
            Self::ProcessRunKnown => "process.run_known",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "dataset.resolve" => Some(Self::DatasetResolve),
            "dataset.open" => Some(Self::DatasetOpen),
            "dataset.inspect" => Some(Self::DatasetInspect),
            "netcdf.dimension.list" => Some(Self::NetcdfDimensionList),
            "netcdf.variable.list" => Some(Self::NetcdfVariableList),
            "netcdf.variable.describe" => Some(Self::NetcdfVariableDescribe),
            "netcdf.variable.load" => Some(Self::NetcdfVariableLoad),
            "stats.mean" => Some(Self::StatsMean),
            "stats.min" => Some(Self::StatsMin),
            "stats.max" => Some(Self::StatsMax),
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
    pub planner_visible: bool,
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
