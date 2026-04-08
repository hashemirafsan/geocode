use predicates::prelude::*;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn provider_list_reports_supported_providers() {
    let temp_dir = TempDir::new().expect("create temp dir");

    let assert = binary()
        .current_dir(temp_dir.path())
        .env("HOME", temp_dir.path())
        .env_remove("OPENAI_API_KEY")
        .arg("provider")
        .arg("list")
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8 stdout");
    assert!(stdout.contains("Supported Providers:"));
    assert!(stdout.contains("- openai (api_key, configured=false)"));
}

#[test]
fn provider_status_reports_unconfigured_openai_by_default() {
    let temp_dir = TempDir::new().expect("create temp dir");

    let assert = binary()
        .current_dir(temp_dir.path())
        .env("HOME", temp_dir.path())
        .env_remove("OPENAI_API_KEY")
        .env_remove("OPENAI_MODEL")
        .env_remove("OPENAI_BASE_URL")
        .arg("provider")
        .arg("status")
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8 stdout");
    assert!(stdout.contains("Provider: openai"));
    assert!(stdout.contains("Auth Method: api_key"));
    assert!(stdout.contains("Configured: false"));
    assert!(stdout.contains("API Key Env Var: OPENAI_API_KEY"));
    assert!(stdout.contains("Credential Source: none"));
}

#[test]
fn provider_status_json_reports_unconfigured_openai_by_default() {
    let temp_dir = TempDir::new().expect("create temp dir");

    let assert = binary()
        .current_dir(temp_dir.path())
        .env("HOME", temp_dir.path())
        .env_remove("OPENAI_API_KEY")
        .env_remove("OPENAI_MODEL")
        .env_remove("OPENAI_BASE_URL")
        .arg("--json")
        .arg("provider")
        .arg("status")
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8 stdout");
    let json: Value = serde_json::from_str(&stdout).expect("json output");

    assert_eq!(json["command"], "provider_status");
    assert_eq!(json["details"]["config"]["provider"], "open_ai");
    assert_eq!(json["details"]["config"]["auth_method"], "api_key");
    assert_eq!(json["details"]["config"]["configured"], false);
    assert_eq!(json["details"]["credential_source"], "none");
}

#[test]
fn ask_fails_cleanly_when_openai_is_not_configured() {
    let temp_dir = TempDir::new().expect("create temp dir");

    binary()
        .current_dir(temp_dir.path())
        .env("HOME", temp_dir.path())
        .env_remove("OPENAI_API_KEY")
        .arg("ask")
        .arg("show all variables in base.nc")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "OpenAI is not configured. Set OPENAI_API_KEY or run `geocode provider set-api-key openai --stdin` to enable `geocode ask`.",
        ));
}

#[test]
fn ask_with_selected_files_still_fails_cleanly_when_unconfigured() {
    let temp_dir = TempDir::new().expect("create temp dir");

    binary()
        .current_dir(temp_dir.path())
        .env("HOME", temp_dir.path())
        .env_remove("OPENAI_API_KEY")
        .arg("ask")
        .arg("--file")
        .arg("base.nc")
        .arg("--file")
        .arg("scenario.nc")
        .arg("compare these")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "OpenAI is not configured. Set OPENAI_API_KEY or run `geocode provider set-api-key openai --stdin` to enable `geocode ask`.",
        ));
}

#[test]
fn provider_set_api_key_persists_configuration() {
    let temp_dir = TempDir::new().expect("create temp dir");

    binary()
        .current_dir(temp_dir.path())
        .env("HOME", temp_dir.path())
        .env_remove("OPENAI_API_KEY")
        .arg("provider")
        .arg("set-api-key")
        .arg("openai")
        .arg("--api-key")
        .arg("test-key-123")
        .assert()
        .success()
        .stdout(predicate::str::contains("Stored API key"));

    let config_file = temp_dir.path().join(".config/geocode/openai.json");
    assert!(config_file.exists());

    let assert = binary()
        .current_dir(temp_dir.path())
        .env("HOME", temp_dir.path())
        .env_remove("OPENAI_API_KEY")
        .arg("--json")
        .arg("provider")
        .arg("status")
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8 stdout");
    let json: Value = serde_json::from_str(&stdout).expect("json output");

    assert_eq!(json["details"]["config"]["configured"], true);
    assert_eq!(json["details"]["credential_source"], "stored");
}

fn binary() -> assert_cmd::Command {
    assert_cmd::Command::cargo_bin("geocode").expect("build test binary")
}
