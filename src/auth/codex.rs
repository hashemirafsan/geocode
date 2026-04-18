use std::{
    fs,
    io::{BufRead, BufReader},
    path::PathBuf,
    process::{Command, Stdio},
    sync::mpsc,
};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::Deserialize;

use crate::{auth::TokenSet, engine::ExecutionError, paths::home_dir, provider::OpenAiAuthSource};

#[derive(Debug, Clone, Copy)]
pub enum CodexLoginMode {
    Browser,
    Headless,
}

#[derive(Debug, Clone)]
pub struct CodexStatus {
    #[allow(dead_code)]
    pub installed: bool,
    pub logged_in: bool,
    pub summary: String,
}

#[derive(Debug, Clone)]
pub struct CodexAuthResult {
    pub source: OpenAiAuthSource,
    pub tokens: TokenSet,
    #[allow(dead_code)]
    pub summary: String,
}

pub fn codex_is_available() -> bool {
    Command::new("codex")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

pub fn codex_login_status() -> Result<CodexStatus, ExecutionError> {
    if !codex_is_available() {
        return Ok(CodexStatus {
            installed: false,
            logged_in: false,
            summary: "Codex CLI is not installed".to_string(),
        });
    }

    let output = Command::new("codex")
        .args(["login", "status"])
        .output()
        .map_err(|err| {
            ExecutionError::Provider(format!("failed to check codex login status: {err}"))
        })?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let summary = if !stdout.is_empty() { stdout } else { stderr };

    Ok(CodexStatus {
        installed: true,
        logged_in: output.status.success() && summary.to_ascii_lowercase().contains("logged in"),
        summary,
    })
}

pub fn run_codex_login<F>(
    mode: CodexLoginMode,
    mut emit: F,
) -> Result<CodexAuthResult, ExecutionError>
where
    F: FnMut(String),
{
    if !codex_is_available() {
        return Err(ExecutionError::ProviderNotConfigured(
            "Codex CLI is required for ChatGPT Plus/Pro login".into(),
        ));
    }

    let mut command = Command::new("codex");
    command.arg("login");
    if matches!(mode, CodexLoginMode::Headless) {
        command.arg("--device-auth");
    }
    command.stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = command
        .spawn()
        .map_err(|err| ExecutionError::Provider(format!("failed to start codex login: {err}")))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| ExecutionError::Provider("failed to capture codex stdout".into()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| ExecutionError::Provider("failed to capture codex stderr".into()))?;

    let (tx, rx) = mpsc::channel();
    let tx_stdout = tx.clone();
    std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            if let Ok(line) = line {
                let _ = tx_stdout.send(line);
            }
        }
    });
    std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            if let Ok(line) = line {
                let _ = tx.send(line);
            }
        }
    });

    while let Ok(line) = rx.recv() {
        emit(line);
    }

    let status = child.wait().map_err(|err| {
        ExecutionError::Provider(format!("failed while waiting for codex login: {err}"))
    })?;
    if !status.success() {
        return Err(ExecutionError::Provider(format!(
            "codex login exited with status {status}"
        )));
    }

    let status = codex_login_status()?;
    if !status.logged_in {
        return Err(ExecutionError::Provider(format!(
            "codex login did not complete successfully: {}",
            status.summary
        )));
    }

    let tokens = load_codex_tokens()?;
    Ok(CodexAuthResult {
        source: match mode {
            CodexLoginMode::Browser => OpenAiAuthSource::CodexBrowser,
            CodexLoginMode::Headless => OpenAiAuthSource::CodexHeadless,
        },
        tokens,
        summary: status.summary,
    })
}

pub fn load_codex_tokens() -> Result<TokenSet, ExecutionError> {
    let path = codex_auth_path()?;
    let content = fs::read_to_string(&path).map_err(|err| {
        ExecutionError::Provider(format!("failed to read Codex auth file: {err}"))
    })?;
    let auth: CodexAuthFile = serde_json::from_str(&content).map_err(|err| {
        ExecutionError::Provider(format!("failed to parse Codex auth file: {err}"))
    })?;

    let access_token = auth.tokens.access_token.trim().to_string();
    if access_token.is_empty() {
        return Err(ExecutionError::ProviderNotConfigured(
            "Codex auth file does not contain an access token".into(),
        ));
    }

    Ok(TokenSet {
        expires_at_unix: token_expiry(&access_token),
        access_token,
        refresh_token: Some(auth.tokens.refresh_token),
    })
}

pub fn load_codex_models() -> Result<Vec<String>, ExecutionError> {
    let home = home_dir();
    let path = home.join(".codex").join("models_cache.json");
    let content = fs::read_to_string(&path).map_err(|err| {
        ExecutionError::Provider(format!("failed to read Codex models cache: {err}"))
    })?;
    let cache: CodexModelCache = serde_json::from_str(&content).map_err(|err| {
        ExecutionError::Provider(format!("failed to parse Codex models cache: {err}"))
    })?;

    let mut models = cache
        .models
        .into_iter()
        .filter(|model| model.visibility == "list")
        .map(|model| model.slug)
        .collect::<Vec<_>>();
    models.sort();
    models.dedup();

    if models.is_empty() {
        return Err(ExecutionError::Provider(
            "Codex models cache did not contain any visible models".into(),
        ));
    }

    Ok(models)
}

fn codex_auth_path() -> Result<PathBuf, ExecutionError> {
    let home = home_dir();
    Ok(home.join(".codex").join("auth.json"))
}

fn token_expiry(jwt: &str) -> Option<u64> {
    let payload = jwt.split('.').nth(1)?;
    let decoded = URL_SAFE_NO_PAD.decode(payload).ok()?;
    let payload: JwtPayload = serde_json::from_slice(&decoded).ok()?;
    payload.exp
}

#[derive(Debug, Deserialize)]
struct CodexAuthFile {
    tokens: CodexTokens,
}

#[derive(Debug, Deserialize)]
struct CodexTokens {
    access_token: String,
    refresh_token: String,
}

#[derive(Debug, Deserialize)]
struct CodexModelCache {
    models: Vec<CodexModelEntry>,
}

#[derive(Debug, Deserialize)]
struct CodexModelEntry {
    slug: String,
    visibility: String,
}

#[derive(Debug, Deserialize)]
struct JwtPayload {
    exp: Option<u64>,
}
