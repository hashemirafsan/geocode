use std::{env, fs, path::PathBuf};

use crate::{engine::ExecutionError, session::state::SessionState};

pub struct SessionStore {
    path: PathBuf,
}

impl SessionStore {
    pub fn new() -> Self {
        let path = env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(".geocode-session.json");

        Self { path }
    }

    pub fn load(&self) -> Result<SessionState, ExecutionError> {
        if !self.path.exists() {
            return Ok(SessionState::default());
        }

        let content = fs::read_to_string(&self.path)
            .map_err(|err| ExecutionError::Session(err.to_string()))?;

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

        Ok(())
    }

    pub fn save(&self, session: &SessionState) -> Result<(), ExecutionError> {
        let content = serde_json::to_string_pretty(session)
            .map_err(|err| ExecutionError::Session(err.to_string()))?;

        fs::write(&self.path, content).map_err(|err| ExecutionError::Session(err.to_string()))
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}
