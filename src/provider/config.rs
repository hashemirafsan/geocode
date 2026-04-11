use std::{env, fs, path::PathBuf};

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

use crate::engine::ExecutionError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    #[value(name = "openai")]
    OpenAi,
    #[value(name = "lmstudio")]
    LmStudio,
    #[value(name = "z.ai", alias = "zai", alias = "z-ai")]
    ZAi,
}

impl ProviderKind {
    pub const fn all() -> [Self; 3] {
        [Self::OpenAi, Self::LmStudio, Self::ZAi]
    }

    pub const fn display_name(self) -> &'static str {
        match self {
            Self::OpenAi => "OpenAI",
            Self::LmStudio => "LMStudio",
            Self::ZAi => "Z.Ai",
        }
    }

    pub const fn command_name(self) -> &'static str {
        match self {
            Self::OpenAi => "openai",
            Self::LmStudio => "lmstudio",
            Self::ZAi => "z.ai",
        }
    }

    pub const fn api_key_env_var(self) -> &'static str {
        match self {
            Self::OpenAi => "OPENAI_API_KEY",
            Self::LmStudio => "<none>",
            Self::ZAi => "ZAI_API_KEY",
        }
    }

    pub const fn requires_api_key(self) -> bool {
        matches!(self, Self::OpenAi | Self::ZAi)
    }

    pub const fn default_model(self) -> &'static str {
        match self {
            Self::OpenAi => "gpt-4o-mini",
            Self::LmStudio => "qwen/qwen3.5-9b",
            Self::ZAi => "glm-4.5-air",
        }
    }

    pub const fn default_base_url(self) -> &'static str {
        match self {
            Self::OpenAi => "https://api.openai.com/v1",
            Self::LmStudio => "http://127.0.0.1:1234/v1",
            Self::ZAi => "https://api.z.ai/api/paas/v4",
        }
    }

    pub const fn config_filename(self) -> &'static str {
        match self {
            Self::OpenAi => "openai.json",
            Self::LmStudio => "lmstudio.json",
            Self::ZAi => "zai.json",
        }
    }

    pub fn fallback_models(self) -> Vec<String> {
        match self {
            Self::OpenAi => vec![
                "gpt-4.1".to_string(),
                "gpt-4.1-mini".to_string(),
                "gpt-4.1-nano".to_string(),
                "gpt-4o".to_string(),
                "gpt-4o-mini".to_string(),
                "o4-mini".to_string(),
            ],
            Self::LmStudio => Vec::new(),
            Self::ZAi => vec![
                "glm-4.5".to_string(),
                "glm-4.5-air".to_string(),
                "glm-4.5-flash".to_string(),
                "glm-4.5v".to_string(),
            ],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum AuthMethod {
    None,
    ApiKey,
    OAuth,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpenAiAuthSource {
    DirectOAuth,
    CodexBrowser,
    CodexHeadless,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub provider: ProviderKind,
    pub auth_method: AuthMethod,
    pub openai_auth_source: Option<OpenAiAuthSource>,
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
                let auth_method = env::var("OPENAI_AUTH_METHOD")
                    .ok()
                    .and_then(|value| AuthMethod::from_str(&value, true).ok())
                    .or_else(|| stored.as_ref().and_then(|config| config.auth_method))
                    .unwrap_or(AuthMethod::ApiKey);
                let openai_auth_source =
                    stored.as_ref().and_then(|config| config.openai_auth_source);
                let env_api_key = env::var(provider.api_key_env_var()).ok();
                let stored_api_key = stored.as_ref().and_then(|config| config.api_key.clone());
                let env_oauth_token = env::var("OPENAI_OAUTH_ACCESS_TOKEN").ok();
                let stored_oauth_token = stored
                    .as_ref()
                    .and_then(|config| config.oauth_access_token.clone());
                let model = env::var("OPENAI_MODEL")
                    .ok()
                    .or_else(|| stored.as_ref().and_then(|config| config.model.clone()))
                    .unwrap_or_else(|| provider.default_model().to_string());
                let base_url = env::var("OPENAI_BASE_URL")
                    .ok()
                    .or_else(|| stored.as_ref().and_then(|config| config.base_url.clone()))
                    .unwrap_or_else(|| provider.default_base_url().to_string());

                Ok(Self {
                    provider,
                    auth_method,
                    openai_auth_source,
                    configured: match auth_method {
                        AuthMethod::ApiKey => {
                            env_api_key
                                .as_ref()
                                .is_some_and(|key| !key.trim().is_empty())
                                || stored_api_key
                                    .as_ref()
                                    .is_some_and(|key| !key.trim().is_empty())
                        }
                        AuthMethod::OAuth => {
                            matches!(
                                openai_auth_source,
                                Some(
                                    OpenAiAuthSource::CodexBrowser
                                        | OpenAiAuthSource::CodexHeadless
                                )
                            ) || env_oauth_token
                                .as_ref()
                                .is_some_and(|token| !token.trim().is_empty())
                                || stored_oauth_token
                                    .as_ref()
                                    .is_some_and(|token| !token.trim().is_empty())
                        }
                        AuthMethod::None => false,
                    },
                    model,
                    base_url,
                })
            }
            ProviderKind::LmStudio => {
                let model = env::var("LMSTUDIO_MODEL")
                    .ok()
                    .or_else(|| stored.as_ref().and_then(|config| config.model.clone()))
                    .unwrap_or_else(|| provider.default_model().to_string());
                let base_url = env::var("LMSTUDIO_BASE_URL")
                    .ok()
                    .or_else(|| stored.as_ref().and_then(|config| config.base_url.clone()))
                    .unwrap_or_else(|| provider.default_base_url().to_string());

                Ok(Self {
                    provider,
                    auth_method: AuthMethod::None,
                    openai_auth_source: None,
                    configured: !base_url.trim().is_empty(),
                    model,
                    base_url,
                })
            }
            ProviderKind::ZAi => {
                let env_api_key = env::var(provider.api_key_env_var()).ok();
                let stored_api_key = stored.as_ref().and_then(|config| config.api_key.clone());
                let model = env::var("ZAI_MODEL")
                    .ok()
                    .or_else(|| stored.as_ref().and_then(|config| config.model.clone()))
                    .unwrap_or_else(|| provider.default_model().to_string());
                let base_url = env::var("ZAI_BASE_URL")
                    .ok()
                    .or_else(|| stored.as_ref().and_then(|config| config.base_url.clone()))
                    .unwrap_or_else(|| provider.default_base_url().to_string());

                Ok(Self {
                    provider,
                    auth_method: AuthMethod::ApiKey,
                    openai_auth_source: None,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSummary {
    pub provider: ProviderKind,
    pub auth_method: AuthMethod,
    pub configured: bool,
    pub default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStatus {
    pub config: ProviderConfig,
    pub api_key_env_var: &'static str,
    pub config_path: String,
    pub credential_source: String,
    pub is_default: bool,
}

impl ProviderStatus {
    pub fn current(provider: ProviderKind) -> Result<Self, ExecutionError> {
        let store = ProviderStore::new();
        let stored = store.load(provider)?;

        match provider {
            ProviderKind::OpenAi => {
                let resolved = ProviderConfig::resolve(provider)?;
                let credential_source = match resolved.auth_method {
                    AuthMethod::ApiKey => {
                        let env_api_key = env::var(provider.api_key_env_var())
                            .ok()
                            .filter(|key| !key.trim().is_empty());

                        if env_api_key.is_some() {
                            "env_api_key".to_string()
                        } else if stored
                            .as_ref()
                            .and_then(|config| config.api_key.as_ref())
                            .is_some_and(|key| !key.trim().is_empty())
                        {
                            "stored_api_key".to_string()
                        } else {
                            "none".to_string()
                        }
                    }
                    AuthMethod::OAuth => {
                        let env_token = env::var("OPENAI_OAUTH_ACCESS_TOKEN")
                            .ok()
                            .filter(|token| !token.trim().is_empty());

                        if env_token.is_some() {
                            "env_oauth".to_string()
                        } else if matches!(
                            resolved.openai_auth_source,
                            Some(OpenAiAuthSource::CodexBrowser | OpenAiAuthSource::CodexHeadless)
                        ) {
                            "codex".to_string()
                        } else if stored
                            .as_ref()
                            .and_then(|config| config.oauth_access_token.as_ref())
                            .is_some_and(|token| !token.trim().is_empty())
                        {
                            "stored_oauth".to_string()
                        } else {
                            "none".to_string()
                        }
                    }
                    AuthMethod::None => "none".to_string(),
                };

                Ok(Self {
                    config: resolved,
                    api_key_env_var: provider.api_key_env_var(),
                    config_path: store.path(provider).display().to_string(),
                    credential_source,
                    is_default: store
                        .default_provider()?
                        .is_some_and(|selected| selected == provider),
                })
            }
            ProviderKind::LmStudio => Ok(Self {
                config: ProviderConfig::resolve(provider)?,
                api_key_env_var: "<none>",
                config_path: store.path(provider).display().to_string(),
                credential_source: "none".to_string(),
                is_default: store
                    .default_provider()?
                    .is_some_and(|selected| selected == provider),
            }),
            ProviderKind::ZAi => {
                let env_api_key = env::var(provider.api_key_env_var())
                    .ok()
                    .filter(|key| !key.trim().is_empty());

                let credential_source = if env_api_key.is_some() {
                    "env".to_string()
                } else if stored
                    .as_ref()
                    .and_then(|config| config.api_key.as_ref())
                    .is_some_and(|key| !key.trim().is_empty())
                {
                    "stored".to_string()
                } else {
                    "none".to_string()
                };

                Ok(Self {
                    config: ProviderConfig::resolve(provider)?,
                    api_key_env_var: provider.api_key_env_var(),
                    config_path: store.path(provider).display().to_string(),
                    credential_source,
                    is_default: store
                        .default_provider()?
                        .is_some_and(|selected| selected == provider),
                })
            }
        }
    }
}

pub fn supported_providers() -> Result<Vec<ProviderSummary>, ExecutionError> {
    ProviderKind::all()
        .into_iter()
        .map(|provider| {
            let status = ProviderStatus::current(provider)?;
            Ok(ProviderSummary {
                provider,
                auth_method: status.config.auth_method,
                configured: status.config.configured,
                default: status.is_default,
            })
        })
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StoredProviderConfig {
    pub auth_method: Option<AuthMethod>,
    pub openai_auth_source: Option<OpenAiAuthSource>,
    pub api_key: Option<String>,
    pub oauth_access_token: Option<String>,
    pub oauth_refresh_token: Option<String>,
    pub oauth_expires_at_unix: Option<u64>,
    pub model: Option<String>,
    pub base_url: Option<String>,
}

pub struct ProviderStore {
    base_dir: PathBuf,
    legacy_base_dir: PathBuf,
}

impl ProviderStore {
    pub fn new() -> Self {
        let home_dir = env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));
        let base_dir = home_dir.join(".geocode");
        let legacy_base_dir = home_dir.join(".config").join("geocode");

        Self {
            base_dir,
            legacy_base_dir,
        }
    }

    pub fn load(
        &self,
        provider: ProviderKind,
    ) -> Result<Option<StoredProviderConfig>, ExecutionError> {
        let path = self.path(provider);

        let path = if path.exists() {
            path
        } else {
            let legacy_path = self.legacy_path(provider);
            if legacy_path.exists() {
                legacy_path
            } else {
                return Ok(None);
            }
        };

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
        self.base_dir.join(provider.config_filename())
    }

    fn legacy_path(&self, provider: ProviderKind) -> PathBuf {
        self.legacy_base_dir.join(provider.config_filename())
    }

    pub fn default_provider_path(&self) -> PathBuf {
        self.base_dir.join("default-provider.json")
    }

    pub fn default_provider(&self) -> Result<Option<ProviderKind>, ExecutionError> {
        let path = self.default_provider_path();

        let path = if path.exists() {
            path
        } else {
            let legacy_path = self.legacy_default_provider_path();
            if legacy_path.exists() {
                legacy_path
            } else {
                return Ok(None);
            }
        };

        let content = fs::read_to_string(&path).map_err(|err| {
            ExecutionError::Provider(format!("failed to read default provider config: {err}"))
        })?;

        let config: DefaultProviderConfig = serde_json::from_str(&content).map_err(|err| {
            ExecutionError::Provider(format!("failed to parse default provider config: {err}"))
        })?;

        Ok(Some(config.provider))
    }

    pub fn save_default_provider(&self, provider: ProviderKind) -> Result<(), ExecutionError> {
        let path = self.default_provider_path();

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                ExecutionError::Provider(format!(
                    "failed to create provider config directory: {err}"
                ))
            })?;
        }

        let content =
            serde_json::to_string_pretty(&DefaultProviderConfig { provider }).map_err(|err| {
                ExecutionError::Provider(format!(
                    "failed to serialize default provider config: {err}"
                ))
            })?;

        fs::write(&path, content).map_err(|err| {
            ExecutionError::Provider(format!("failed to write default provider config: {err}"))
        })
    }

    fn legacy_default_provider_path(&self) -> PathBuf {
        self.legacy_base_dir.join("default-provider.json")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DefaultProviderConfig {
    provider: ProviderKind,
}

#[cfg(test)]
mod tests {
    use super::{ProviderKind, ProviderStore};

    #[test]
    fn provider_store_uses_home_geocode_directory() {
        let store = ProviderStore::new();
        let path = store.path(ProviderKind::OpenAi);
        assert!(path.to_string_lossy().contains(".geocode/openai.json"));
    }

    #[test]
    fn provider_kind_catalog_includes_zai() {
        let names = ProviderKind::all()
            .into_iter()
            .map(ProviderKind::command_name)
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["openai", "lmstudio", "z.ai"]);
    }
}
