use std::{env, fs, path::PathBuf};

use crate::{engine::ExecutionError, memory::state::MemoryState};

pub struct MemoryStore {
    path: PathBuf,
}

impl MemoryStore {
    pub fn new() -> Self {
        let path = env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(".config")
            .join("geocode")
            .join("memory.json");

        Self { path }
    }

    pub fn load(&self) -> Result<MemoryState, ExecutionError> {
        if !self.path.exists() {
            return Ok(MemoryState::default());
        }

        let content = fs::read_to_string(&self.path)
            .map_err(|err| ExecutionError::Session(err.to_string()))?;

        serde_json::from_str(&content).map_err(|err| ExecutionError::Session(err.to_string()))
    }

    pub fn save(&self, memory: &MemoryState) -> Result<(), ExecutionError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|err| ExecutionError::Session(err.to_string()))?;
        }

        let content = serde_json::to_string_pretty(memory)
            .map_err(|err| ExecutionError::Session(err.to_string()))?;

        fs::write(&self.path, content).map_err(|err| ExecutionError::Session(err.to_string()))
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}
