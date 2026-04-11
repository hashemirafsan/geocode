use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn cli_ask_uses_new_non_interactive_namespace() {
    let temp_dir = TempDir::new().expect("create temp dir");

    binary()
        .current_dir(temp_dir.path())
        .env("HOME", temp_dir.path())
        .env_remove("OPENAI_API_KEY")
        .env("LMSTUDIO_BASE_URL", "http://127.0.0.1:9/v1")
        .arg("cli")
        .arg("ask")
        .arg("show all variables in base.nc")
        .assert()
        .failure()
        .stderr(predicate::str::contains("provider error:"));
}

fn binary() -> assert_cmd::Command {
    assert_cmd::Command::cargo_bin("geocode").expect("build test binary")
}
