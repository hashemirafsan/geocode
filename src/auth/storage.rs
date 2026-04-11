use crate::{
    auth::types::{CredentialRef, TokenSet},
    engine::ExecutionError,
    provider::{ProviderStore, StoredProviderConfig},
};

pub trait CredentialStore {
    fn load_api_key(&self, key: &CredentialRef) -> Result<Option<String>, ExecutionError>;
    fn save_api_key(&self, key: &CredentialRef, api_key: &str) -> Result<(), ExecutionError>;
    fn load_tokens(&self, key: &CredentialRef) -> Result<Option<TokenSet>, ExecutionError>;
    fn save_tokens(&self, key: &CredentialRef, tokens: &TokenSet) -> Result<(), ExecutionError>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct FileCredentialStore;

impl FileCredentialStore {
    pub fn new() -> Self {
        Self
    }
}

impl CredentialStore for FileCredentialStore {
    fn load_api_key(&self, key: &CredentialRef) -> Result<Option<String>, ExecutionError> {
        Ok(ProviderStore::new()
            .load(key.provider)?
            .and_then(|config| config.api_key)
            .filter(|api_key| !api_key.trim().is_empty()))
    }

    fn save_api_key(&self, key: &CredentialRef, api_key: &str) -> Result<(), ExecutionError> {
        let store = ProviderStore::new();
        let mut config = store.load(key.provider)?.unwrap_or_default();
        config.api_key = Some(api_key.to_string());
        store.save(key.provider, &config)
    }

    fn load_tokens(&self, key: &CredentialRef) -> Result<Option<TokenSet>, ExecutionError> {
        Ok(ProviderStore::new().load(key.provider)?.and_then(|config| {
            config.oauth_access_token.map(|access_token| TokenSet {
                access_token,
                refresh_token: config.oauth_refresh_token,
                expires_at_unix: config.oauth_expires_at_unix,
            })
        }))
    }

    fn save_tokens(&self, key: &CredentialRef, tokens: &TokenSet) -> Result<(), ExecutionError> {
        let store = ProviderStore::new();
        let mut config: StoredProviderConfig = store.load(key.provider)?.unwrap_or_default();
        config.oauth_access_token = Some(tokens.access_token.clone());
        config.oauth_refresh_token = tokens.refresh_token.clone();
        config.oauth_expires_at_unix = tokens.expires_at_unix;
        store.save(key.provider, &config)
    }
}
