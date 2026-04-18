#![allow(dead_code)]

use std::{
    io::{Read, Write},
    net::TcpListener,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::{distr::Alphanumeric, rng, Rng};
use reqwest::blocking::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use url::Url;

use crate::{auth::TokenSet, engine::ExecutionError};

const OPENAI_DEFAULT_AUTH_URL: &str = "https://auth.openai.com/oauth/authorize";
const OPENAI_DEFAULT_TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
const OPENAI_DEFAULT_SCOPES: &str = "openid profile offline_access";

#[derive(Debug, Clone)]
pub struct OAuthClientConfig {
    pub client_id: String,
    pub auth_url: String,
    pub token_url: String,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct OAuthSession {
    pub state: String,
    pub code_verifier: String,
    pub redirect_uri: String,
    pub authorize_url: String,
}

pub fn openai_client_config() -> Result<OAuthClientConfig, ExecutionError> {
    let client_id = std::env::var("OPENAI_OAUTH_CLIENT_ID").map_err(|_| {
        ExecutionError::ProviderNotConfigured(
            "OPENAI_OAUTH_CLIENT_ID is required for OpenAI OAuth login".into(),
        )
    })?;
    let auth_url = std::env::var("OPENAI_OAUTH_AUTH_URL")
        .unwrap_or_else(|_| OPENAI_DEFAULT_AUTH_URL.to_string());
    let token_url = std::env::var("OPENAI_OAUTH_TOKEN_URL")
        .unwrap_or_else(|_| OPENAI_DEFAULT_TOKEN_URL.to_string());
    let scopes = std::env::var("OPENAI_OAUTH_SCOPES")
        .unwrap_or_else(|_| OPENAI_DEFAULT_SCOPES.to_string())
        .split_whitespace()
        .map(str::to_string)
        .collect::<Vec<_>>();

    Ok(OAuthClientConfig {
        client_id,
        auth_url,
        token_url,
        scopes,
    })
}

pub fn login_openai_oauth() -> Result<TokenSet, ExecutionError> {
    let client = openai_client_config()?;
    let (session, listener) = build_oauth_session(&client)?;
    open_browser(&session.authorize_url)?;
    let code = wait_for_callback(listener, &session.state, Duration::from_secs(180))?;
    exchange_authorization_code(&client, &session, &code)
}

pub fn refresh_openai_token(refresh_token: &str) -> Result<TokenSet, ExecutionError> {
    let client = openai_client_config()?;
    let http = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|err| ExecutionError::Provider(format!("failed to build oauth client: {err}")))?;

    let response = http
        .post(&client.token_url)
        .form(&[
            ("grant_type", "refresh_token"),
            ("client_id", client.client_id.as_str()),
            ("refresh_token", refresh_token),
        ])
        .send()
        .map_err(|err| ExecutionError::Provider(format!("oauth refresh request failed: {err}")))?;

    parse_token_response(response)
}

fn build_oauth_session(
    client: &OAuthClientConfig,
) -> Result<(OAuthSession, TcpListener), ExecutionError> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|err| ExecutionError::Io(format!("failed to bind oauth callback port: {err}")))?;
    listener
        .set_nonblocking(true)
        .map_err(|err| ExecutionError::Io(format!("failed to configure oauth callback: {err}")))?;

    let port = listener
        .local_addr()
        .map_err(|err| ExecutionError::Io(format!("failed to inspect oauth callback port: {err}")))?
        .port();
    let redirect_uri = format!("http://127.0.0.1:{port}/callback");
    let code_verifier = random_token(96);
    let state = random_token(48);
    let code_challenge = code_challenge(&code_verifier);

    let mut auth_url = Url::parse(&client.auth_url)
        .map_err(|err| ExecutionError::Provider(format!("invalid oauth auth url: {err}")))?;
    auth_url
        .query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", &client.client_id)
        .append_pair("redirect_uri", &redirect_uri)
        .append_pair("scope", &client.scopes.join(" "))
        .append_pair("state", &state)
        .append_pair("code_challenge", &code_challenge)
        .append_pair("code_challenge_method", "S256");

    Ok((
        OAuthSession {
            state,
            code_verifier,
            redirect_uri,
            authorize_url: auth_url.into(),
        },
        listener,
    ))
}

fn wait_for_callback(
    listener: TcpListener,
    expected_state: &str,
    timeout: Duration,
) -> Result<String, ExecutionError> {
    let started = Instant::now();

    while started.elapsed() < timeout {
        match listener.accept() {
            Ok((mut stream, _)) => {
                let mut buffer = [0_u8; 4096];
                let read = stream.read(&mut buffer).map_err(|err| {
                    ExecutionError::Io(format!("failed to read oauth callback: {err}"))
                })?;
                let request = String::from_utf8_lossy(&buffer[..read]);
                let path = request
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    .ok_or_else(|| {
                        ExecutionError::Provider("oauth callback was malformed".into())
                    })?;

                let url = Url::parse(&format!("http://127.0.0.1{path}")).map_err(|err| {
                    ExecutionError::Provider(format!("invalid oauth callback: {err}"))
                })?;
                let mut code = None;
                let mut state = None;
                let mut error = None;
                for (key, value) in url.query_pairs() {
                    match key.as_ref() {
                        "code" => code = Some(value.into_owned()),
                        "state" => state = Some(value.into_owned()),
                        "error" => error = Some(value.into_owned()),
                        _ => {}
                    }
                }

                let response_body = if let Some(error) = error {
                    write_callback_response(&mut stream, false)?;
                    return Err(ExecutionError::Provider(format!(
                        "oauth login failed in browser: {error}"
                    )));
                } else if state.as_deref() != Some(expected_state) {
                    write_callback_response(&mut stream, false)?;
                    return Err(ExecutionError::Provider(
                        "oauth callback state did not match the login request".into(),
                    ));
                } else if let Some(code) = code {
                    write_callback_response(&mut stream, true)?;
                    return Ok(code);
                } else {
                    "missing code".to_string()
                };

                write_callback_response(&mut stream, false)?;
                return Err(ExecutionError::Provider(format!(
                    "oauth callback did not include an authorization code: {response_body}"
                )));
            }
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(err) => {
                return Err(ExecutionError::Io(format!(
                    "failed while waiting for oauth callback: {err}"
                )));
            }
        }
    }

    Err(ExecutionError::Provider(
        "timed out waiting for the browser to finish OAuth login".into(),
    ))
}

fn exchange_authorization_code(
    client: &OAuthClientConfig,
    session: &OAuthSession,
    code: &str,
) -> Result<TokenSet, ExecutionError> {
    let http = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|err| ExecutionError::Provider(format!("failed to build oauth client: {err}")))?;

    let response = http
        .post(&client.token_url)
        .form(&[
            ("grant_type", "authorization_code"),
            ("client_id", client.client_id.as_str()),
            ("code", code),
            ("redirect_uri", session.redirect_uri.as_str()),
            ("code_verifier", session.code_verifier.as_str()),
        ])
        .send()
        .map_err(|err| ExecutionError::Provider(format!("oauth token exchange failed: {err}")))?;

    parse_token_response(response)
}

fn parse_token_response(response: reqwest::blocking::Response) -> Result<TokenSet, ExecutionError> {
    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .unwrap_or_else(|_| "<unable to read response body>".to_string());
        return Err(ExecutionError::Provider(format!(
            "oauth token request failed with status {status}: {body}"
        )));
    }

    let payload: TokenResponse = response
        .json()
        .map_err(|err| ExecutionError::Provider(format!("invalid oauth token response: {err}")))?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);

    Ok(TokenSet {
        access_token: payload.access_token,
        refresh_token: payload.refresh_token,
        expires_at_unix: payload.expires_in.map(|ttl| now.saturating_add(ttl)),
    })
}

fn write_callback_response(
    stream: &mut std::net::TcpStream,
    success: bool,
) -> Result<(), ExecutionError> {
    let body = if success {
        "<html><body><h1>Login complete</h1><p>You can return to geocode.</p></body></html>"
    } else {
        "<html><body><h1>Login failed</h1><p>You can close this tab and return to geocode.</p></body></html>"
    };
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(response.as_bytes()).map_err(|err| {
        ExecutionError::Io(format!("failed to write oauth callback response: {err}"))
    })
}

fn open_browser(url: &str) -> Result<(), ExecutionError> {
    open::that(url)
        .map(|_| ())
        .map_err(|err| ExecutionError::Provider(format!("failed to open browser: {err}")))
}

fn random_token(length: usize) -> String {
    rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

fn code_challenge(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
}
