use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn help_lists_public_root_commands() {
    binary()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("inspect"))
        .stdout(predicate::str::contains("mean"))
        .stdout(predicate::str::contains("compare"))
        .stdout(predicate::str::contains("ask"))
        .stdout(predicate::str::contains("version"));
}

#[test]
fn version_command_reports_version_and_target() {
    binary()
        .arg("version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")))
        .stdout(predicate::str::contains("Target:"));
}

#[test]
fn version_flag_reports_version_and_target() {
    binary()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")))
        .stdout(predicate::str::contains("Target:"));
}

#[test]
fn inspect_command_is_public_and_reports_missing_file() {
    let temp_dir = TempDir::new().expect("create temp dir");

    binary()
        .current_dir(temp_dir.path())
        .env("HOME", temp_dir.path())
        .arg("inspect")
        .arg("missing.nc")
        .assert()
        .failure()
        .stderr(predicate::str::contains("file not found:"));
}

fn binary() -> assert_cmd::Command {
    assert_cmd::Command::cargo_bin("geocode").expect("build test binary")
}
