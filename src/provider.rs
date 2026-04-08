#![allow(dead_code)]

use std::{env, fs, path::PathBuf};

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

use crate::engine::ExecutionError;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    #[value(name = "openai")]
    OpenAi,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthMethod {
    ApiKey,
    OAuth,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub provider: ProviderKind,
    pub auth_method: AuthMethod,
    pub configured: bool,
    pub model: String,
    pub base_url: String,
}

impl ProviderConfig {
    pub fn resolve(provider: ProviderKind) -> Result<Self, ExecutionError> {
        let store = ProviderStore::new();
        let stored = store.load(provider)?;

        match provider {
            ProviderKind::OpenAi => {
                let env_api_key = env::var("OPENAI_API_KEY").ok();
                let stored_api_key = stored.as_ref().and_then(|config| config.api_key.clone());
                let model = env::var("OPENAI_MODEL")
                    .ok()
                    .or_else(|| stored.as_ref().and_then(|config| config.model.clone()))
                    .unwrap_or_else(|| "gpt-4.1-mini".to_string());
                let base_url = env::var("OPENAI_BASE_URL")
                    .ok()
                    .or_else(|| stored.as_ref().and_then(|config| config.base_url.clone()))
                    .unwrap_or_else(|| "https://api.openai.com/v1".to_string());

                Ok(Self {
                    provider,
                    auth_method: AuthMethod::ApiKey,
                    configured: env_api_key
                        .as_ref()
                        .is_some_and(|key| !key.trim().is_empty())
                        || stored_api_key
                            .as_ref()
                            .is_some_and(|key| !key.trim().is_empty()),
                    model,
                    base_url,
                })
            }
        }
    }

    pub fn api_key(&self) -> Result<Option<String>, ExecutionError> {
        match self.provider {
            ProviderKind::OpenAi => {
                if let Some(key) = env::var("OPENAI_API_KEY")
                    .ok()
                    .filter(|key| !key.trim().is_empty())
                {
                    return Ok(Some(key));
                }

                Ok(ProviderStore::new()
                    .load(self.provider)?
                    .and_then(|config| config.api_key)
                    .filter(|key| !key.trim().is_empty()))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSummary {
    pub provider: ProviderKind,
    pub auth_method: AuthMethod,
    pub configured: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStatus {
    pub config: ProviderConfig,
    pub api_key_env_var: &'static str,
    pub config_path: String,
    pub credential_source: &'static str,
}

impl ProviderStatus {
    pub fn current(provider: ProviderKind) -> Result<Self, ExecutionError> {
        let store = ProviderStore::new();
        let stored = store.load(provider)?;

        match provider {
            ProviderKind::OpenAi => {
                let env_api_key = env::var("OPENAI_API_KEY")
                    .ok()
                    .filter(|key| !key.trim().is_empty());

                let credential_source = if env_api_key.is_some() {
                    "env"
                } else if stored
                    .as_ref()
                    .and_then(|config| config.api_key.as_ref())
                    .is_some_and(|key| !key.trim().is_empty())
                {
                    "stored"
                } else {
                    "none"
                };

                Ok(Self {
                    config: ProviderConfig::resolve(provider)?,
                    api_key_env_var: "OPENAI_API_KEY",
                    config_path: store.path(provider).display().to_string(),
                    credential_source,
                })
            }
        }
    }
}

pub fn supported_providers() -> Result<Vec<ProviderSummary>, ExecutionError> {
    let providers = [ProviderKind::OpenAi];

    providers
        .into_iter()
        .map(|provider| {
            let status = ProviderStatus::current(provider)?;
            Ok(ProviderSummary {
                provider,
                auth_method: status.config.auth_method,
                configured: status.config.configured,
            })
        })
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StoredProviderConfig {
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub base_url: Option<String>,
}

pub struct ProviderStore {
    base_dir: PathBuf,
}

impl ProviderStore {
    pub fn new() -> Self {
        let base_dir = env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(".config")
            .join("geocode");

        Self { base_dir }
    }

    pub fn load(
        &self,
        provider: ProviderKind,
    ) -> Result<Option<StoredProviderConfig>, ExecutionError> {
        let path = self.path(provider);

        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&path).map_err(|err| {
            ExecutionError::Provider(format!("failed to read provider config: {err}"))
        })?;

        let config = serde_json::from_str(&content).map_err(|err| {
            ExecutionError::Provider(format!("failed to parse provider config: {err}"))
        })?;

        Ok(Some(config))
    }

    pub fn save(
        &self,
        provider: ProviderKind,
        config: &StoredProviderConfig,
    ) -> Result<(), ExecutionError> {
        let path = self.path(provider);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                ExecutionError::Provider(format!(
                    "failed to create provider config directory: {err}"
                ))
            })?;
        }

        let content = serde_json::to_string_pretty(config).map_err(|err| {
            ExecutionError::Provider(format!("failed to serialize provider config: {err}"))
        })?;

        fs::write(&path, content).map_err(|err| {
            ExecutionError::Provider(format!("failed to write provider config: {err}"))
        })
    }

    pub fn path(&self, provider: ProviderKind) -> PathBuf {
        let filename = match provider {
            ProviderKind::OpenAi => "openai.json",
        };

        self.base_dir.join(filename)
    }
}

pub trait PlannerProvider {
    fn name(&self) -> &'static str;
}
