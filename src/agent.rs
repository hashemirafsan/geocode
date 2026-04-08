#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct PlannerRequest {
    pub user_input: String,
    pub session_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlannerResponse {
    pub intent: String,
    pub tool_ids: Vec<String>,
}
