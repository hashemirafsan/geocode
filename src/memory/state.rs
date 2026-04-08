use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TurnMemory {
    pub current_goal: Option<String>,
    pub recent_turns: Vec<TurnSummary>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionMemory {
    pub loaded_dataset_aliases: Vec<String>,
    pub prior_results: Vec<CachedResult>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PersistentUserMemory {
    pub preferred_provider: Option<String>,
    pub preferred_model: Option<String>,
    pub preferred_output_format: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryState {
    pub turn: TurnMemory,
    pub session: SessionMemory,
    pub persistent: PersistentUserMemory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnSummary {
    pub user_input: String,
    pub outcome: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedResult {
    pub key: String,
    pub summary: String,
}
