use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn doctor_text_output_includes_support_fields() {
    let temp_dir = TempDir::new().expect("create temp dir");

    binary()
        .current_dir(temp_dir.path())
        .env("HOME", temp_dir.path())
        .env("GEOCODE_INSTALL_SOURCE", "standalone")
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("Install Source:"))
        .stdout(predicate::str::contains("Known Binaries:"))
        .stdout(predicate::str::contains("Update Eligible:"));
}

#[test]
fn doctor_json_output_exposes_update_and_paths() {
    let temp_dir = TempDir::new().expect("create temp dir");

    binary()
        .current_dir(temp_dir.path())
        .env("HOME", temp_dir.path())
        .env("GEOCODE_INSTALL_SOURCE", "homebrew")
        .args(["--json", "doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"install_source\": \"homebrew\""))
        .stdout(predicate::str::contains("\"eligible\": false"))
        .stdout(predicate::str::contains("\"config_dir\""));
}

fn binary() -> assert_cmd::Command {
    assert_cmd::Command::cargo_bin("geocode").expect("build test binary")
}
