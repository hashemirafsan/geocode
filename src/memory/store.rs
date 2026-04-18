use std::{fs, path::PathBuf};

use crate::{engine::ExecutionError, memory::state::MemoryState, paths::GeocodePaths};

pub struct MemoryStore {
    path: PathBuf,
    legacy_paths: Vec<PathBuf>,
}

impl MemoryStore {
    pub fn new() -> Self {
        let path = GeocodePaths::detect().state_dir.join("memory.json");
        let home = crate::paths::home_dir();

        Self {
            path,
            legacy_paths: vec![home.join(".config").join("geocode").join("memory.json")],
        }
    }

    pub fn load(&self) -> Result<MemoryState, ExecutionError> {
        let path = self.resolve_existing_path();
        if !path.exists() {
            return Ok(MemoryState::default());
        }

        let content =
            fs::read_to_string(&path).map_err(|err| ExecutionError::Session(err.to_string()))?;

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

    fn resolve_existing_path(&self) -> PathBuf {
        if self.path.exists() {
            return self.path.clone();
        }

        self.legacy_paths
            .iter()
            .find(|path| path.exists())
            .cloned()
            .unwrap_or_else(|| self.path.clone())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::MemoryStore;
    use crate::memory::state::MemoryState;

    #[test]
    fn memory_store_loads_from_legacy_path_when_new_path_missing() {
        let temp_dir = TempDir::new().expect("temp dir");
        let legacy_path = temp_dir.path().join("legacy-memory.json");
        fs::write(
            &legacy_path,
            serde_json::to_string(&sample_memory()).expect("serialize"),
        )
        .expect("write legacy memory");

        let store = MemoryStore {
            path: temp_dir.path().join("state").join("memory.json"),
            legacy_paths: vec![legacy_path],
        };

        let memory = store.load().expect("load memory");
        assert_eq!(
            memory.persistent.preferred_provider.as_deref(),
            Some("openai")
        );
    }

    fn sample_memory() -> MemoryState {
        let mut memory = MemoryState::default();
        memory.persistent.preferred_provider = Some("openai".to_string());
        memory
    }
}
