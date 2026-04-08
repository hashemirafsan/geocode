#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub provider: String,
    pub model: Option<String>,
}

pub trait PlannerProvider {
    fn name(&self) -> &'static str;
}
