use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn self_update_redirects_homebrew_installs() {
    let temp_dir = TempDir::new().expect("create temp dir");

    binary()
        .current_dir(temp_dir.path())
        .env("HOME", temp_dir.path())
        .env("GEOCODE_INSTALL_SOURCE", "homebrew")
        .arg("self-update")
        .assert()
        .success()
        .stdout(predicate::str::contains("brew upgrade geocode"));
}

#[test]
fn self_update_rejects_unknown_install_source() {
    let temp_dir = TempDir::new().expect("create temp dir");

    binary()
        .current_dir(temp_dir.path())
        .env("HOME", temp_dir.path())
        .env("GEOCODE_INSTALL_SOURCE", "unknown")
        .arg("self-update")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "standalone GitHub release installs",
        ));
}

fn binary() -> assert_cmd::Command {
    assert_cmd::Command::cargo_bin("geocode").expect("build test binary")
}
