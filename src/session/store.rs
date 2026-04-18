use std::{env, fs, path::PathBuf};

use crate::{engine::ExecutionError, paths::GeocodePaths, session::state::SessionState};

pub struct SessionStore {
    path: PathBuf,
    legacy_path: Option<PathBuf>,
}

impl SessionStore {
    pub fn new() -> Self {
        let path = GeocodePaths::detect().state_dir.join("session.json");
        let legacy_path = env::current_dir()
            .ok()
            .map(|dir| dir.join(".geocode-session.json"));

        Self { path, legacy_path }
    }

    pub fn load(&self) -> Result<SessionState, ExecutionError> {
        let path = self.resolve_existing_path();
        if !path.exists() {
            return Ok(SessionState::default());
        }

        let content =
            fs::read_to_string(&path).map_err(|err| ExecutionError::Session(err.to_string()))?;

        let mut session: SessionState = serde_json::from_str(&content)
            .map_err(|err| ExecutionError::Session(err.to_string()))?;

        if session.session_id.is_none() {
            session.session_id = SessionState::default().session_id;
        }

        Ok(session)
    }

    pub fn clear(&self) -> Result<(), ExecutionError> {
        if self.path.exists() {
            fs::remove_file(&self.path).map_err(|err| ExecutionError::Session(err.to_string()))?;
        }

        if let Some(legacy_path) = &self.legacy_path {
            if legacy_path.exists() {
                fs::remove_file(legacy_path)
                    .map_err(|err| ExecutionError::Session(err.to_string()))?;
            }
        }

        Ok(())
    }

    pub fn save(&self, session: &SessionState) -> Result<(), ExecutionError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|err| ExecutionError::Session(err.to_string()))?;
        }

        let content = serde_json::to_string_pretty(session)
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

        self.legacy_path
            .clone()
            .unwrap_or_else(|| self.path.clone())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::SessionStore;
    use crate::session::state::SessionState;

    #[test]
    fn session_store_loads_from_legacy_path_when_new_path_missing() {
        let temp_dir = TempDir::new().expect("temp dir");
        let legacy_path = temp_dir.path().join(".geocode-session.json");
        let mut session = SessionState::default();
        session.current_goal = Some("compare files".to_string());
        fs::write(
            &legacy_path,
            serde_json::to_string(&session).expect("serialize"),
        )
        .expect("write legacy session");

        let store = SessionStore {
            path: temp_dir.path().join("state").join("session.json"),
            legacy_path: Some(legacy_path),
        };

        let loaded = store.load().expect("load session");
        assert_eq!(loaded.current_goal.as_deref(), Some("compare files"));
    }

    #[test]
    fn session_store_save_creates_parent_directory() {
        let temp_dir = TempDir::new().expect("temp dir");
        let store = SessionStore {
            path: temp_dir.path().join("state").join("session.json"),
            legacy_path: None,
        };

        store.save(&SessionState::default()).expect("save session");
        assert!(store.path.exists());
    }
}
