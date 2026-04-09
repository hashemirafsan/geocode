use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SessionState {
    pub session_id: Option<String>,
    pub workspace_path: Option<PathBuf>,
    pub aliases: Vec<DatasetAlias>,
    pub last_variable: Option<String>,
    pub current_goal: Option<String>,
    pub recent_turns: Vec<RecentTurn>,
    pub prior_results: Vec<CachedResultSummary>,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            session_id: Some(generate_session_id()),
            workspace_path: None,
            aliases: Vec::new(),
            last_variable: None,
            current_goal: None,
            recent_turns: Vec::new(),
            prior_results: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetAlias {
    pub alias: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentTurn {
    pub user_input: String,
    pub outcome: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedResultSummary {
    pub kind: String,
    pub summary: String,
}

fn generate_session_id() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();

    format!("session-{nanos}")
}
