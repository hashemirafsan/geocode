use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    auth::{oauth::refresh_openai_token, storage::CredentialStore, types::CredentialRef},
    engine::ExecutionError,
    provider::{AuthMethod, OpenAiAuthSource, ProviderConfig, ProviderKind},
};

pub struct AuthService<S: CredentialStore> {
    store: S,
}

impl<S: CredentialStore> AuthService<S> {
    pub fn new(store: S) -> Self {
        Self { store }
    }

    pub fn bearer_token(&self, config: &ProviderConfig) -> Result<Option<String>, ExecutionError> {
        match config.auth_method {
            AuthMethod::None => Ok(None),
            AuthMethod::ApiKey => self.resolve_api_key(config),
            AuthMethod::OAuth => self.resolve_oauth_access_token(config),
        }
    }

    pub fn force_refresh_bearer_token(
        &self,
        config: &ProviderConfig,
    ) -> Result<Option<String>, ExecutionError> {
        match config.auth_method {
            AuthMethod::OAuth => self.refresh_oauth_access_token(config).map(Some),
            _ => self.bearer_token(config),
        }
    }

    fn resolve_api_key(&self, config: &ProviderConfig) -> Result<Option<String>, ExecutionError> {
        if let Some(key) = std::env::var(config.provider.api_key_env_var())
            .ok()
            .filter(|key| !key.trim().is_empty())
        {
            return Ok(Some(key));
        }

        self.store.load_api_key(&CredentialRef {
            provider: config.provider,
        })
    }

    fn resolve_oauth_access_token(
        &self,
        config: &ProviderConfig,
    ) -> Result<Option<String>, ExecutionError> {
        if !matches!(config.provider, ProviderKind::OpenAi) {
            return Ok(None);
        }

        if let Some(token) = std::env::var("OPENAI_OAUTH_ACCESS_TOKEN")
            .ok()
            .filter(|token| !token.trim().is_empty())
        {
            return Ok(Some(token));
        }

        let key = CredentialRef {
            provider: config.provider,
        };
        let Some(tokens) = self.store.load_tokens(&key)? else {
            return Ok(None);
        };

        if self.is_expiring_soon(tokens.expires_at_unix, 60) {
            return self.refresh_oauth_access_token(config).map(Some);
        }

        Ok(Some(tokens.access_token))
    }

    fn refresh_oauth_access_token(
        &self,
        config: &ProviderConfig,
    ) -> Result<String, ExecutionError> {
        let key = CredentialRef {
            provider: config.provider,
        };
        let Some(tokens) = self.store.load_tokens(&key)? else {
            return Err(ExecutionError::ProviderNotConfigured(format!(
                "{} OAuth credentials are missing",
                config.provider.display_name()
            )));
        };

        if matches!(
            config.openai_auth_source,
            Some(OpenAiAuthSource::CodexBrowser | OpenAiAuthSource::CodexHeadless)
        ) {
            if self.is_expiring_soon(tokens.expires_at_unix, 0) {
                return Err(ExecutionError::ProviderNotConfigured(
                    "Codex-backed OpenAI token expired; run ChatGPT Plus/Pro login again".into(),
                ));
            }

            return Ok(tokens.access_token);
        }

        if let Some(refresh_token) = tokens.refresh_token.as_ref() {
            let refreshed = refresh_openai_token(refresh_token)?;
            self.store.save_tokens(&key, &refreshed)?;
            return Ok(refreshed.access_token);
        }

        if self.is_expiring_soon(tokens.expires_at_unix, 0) {
            return Err(ExecutionError::ProviderNotConfigured(format!(
                "{} OAuth access token expired; sign in again",
                config.provider.display_name()
            )));
        }

        Ok(tokens.access_token)
    }

    fn is_expiring_soon(&self, expires_at_unix: Option<u64>, threshold_secs: u64) -> bool {
        let Some(expires_at_unix) = expires_at_unix else {
            return false;
        };

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0);
        expires_at_unix <= now.saturating_add(threshold_secs)
    }
}

impl AuthService<crate::auth::storage::FileCredentialStore> {
    pub fn for_file_store() -> Self {
        Self::new(crate::auth::storage::FileCredentialStore::new())
    }
}
