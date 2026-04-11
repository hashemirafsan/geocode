use std::time::Duration;

use reqwest::{
    Method, StatusCode,
    blocking::{Client, Response},
};

use crate::{
    auth::AuthService,
    engine::ExecutionError,
    provider::{AuthMethod, OpenAiAuthSource, ProviderConfig},
};

pub struct AuthenticatedClient {
    http: Client,
    auth: AuthService<crate::auth::FileCredentialStore>,
}

impl AuthenticatedClient {
    pub fn new(timeout: Duration) -> Result<Self, ExecutionError> {
        let http = Client::builder()
            .timeout(timeout)
            .pool_max_idle_per_host(0)
            .tcp_keepalive(None)
            .build()
            .map_err(|err| {
                ExecutionError::Provider(format!("failed to build provider client: {err}"))
            })?;

        Ok(Self {
            http,
            auth: AuthService::for_file_store(),
        })
    }

    pub fn get(&self, config: &ProviderConfig, url: &str) -> Result<Response, ExecutionError> {
        self.send(Method::GET, config, url, None)
    }

    pub fn post_json(
        &self,
        config: &ProviderConfig,
        url: &str,
        body: &serde_json::Value,
    ) -> Result<Response, ExecutionError> {
        self.send(Method::POST, config, url, Some(body))
    }

    fn send(
        &self,
        method: Method,
        config: &ProviderConfig,
        url: &str,
        body: Option<&serde_json::Value>,
    ) -> Result<Response, ExecutionError> {
        if matches!(
            config.openai_auth_source,
            Some(OpenAiAuthSource::CodexBrowser | OpenAiAuthSource::CodexHeadless)
        ) {
            return Err(ExecutionError::Provider(
                "Codex-backed ChatGPT Plus/Pro login can provide available model metadata, but it cannot be used for direct OpenAI API requests from GeoCode. Codex tokens are missing the `model.request` scope. Use an OpenAI API key for GeoCode requests, or route requests through Codex itself in a later integration.".into(),
            ));
        }

        let token = self.auth.bearer_token(config)?;
        let response = self.send_once(method.clone(), config, url, body, token.as_deref())?;

        if response.status() != StatusCode::UNAUTHORIZED
            || !matches!(config.auth_method, AuthMethod::OAuth)
        {
            return Ok(response);
        }

        let refreshed = self.auth.force_refresh_bearer_token(config)?;
        self.send_once(method, config, url, body, refreshed.as_deref())
    }

    fn send_once(
        &self,
        method: Method,
        _config: &ProviderConfig,
        url: &str,
        body: Option<&serde_json::Value>,
        token: Option<&str>,
    ) -> Result<Response, ExecutionError> {
        let mut request = self.http.request(method, url);
        if let Some(token) = token {
            request = request.bearer_auth(token);
        }
        if let Some(body) = body {
            request = request.json(body);
        }

        request
            .send()
            .map_err(|err| ExecutionError::Provider(format!("provider request failed: {err}")))
    }
}
