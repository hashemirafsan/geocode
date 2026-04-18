use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn version_json_output_contains_contract_fields() {
    binary()
        .args(["--json", "version"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"command\": \"version\""))
        .stdout(predicate::str::contains("\"version\""))
        .stdout(predicate::str::contains("\"target\""));
}

#[test]
fn doctor_json_output_contains_runtime_binaries() {
    let temp_dir = TempDir::new().expect("create temp dir");

    binary()
        .current_dir(temp_dir.path())
        .env("HOME", temp_dir.path())
        .env("GEOCODE_INSTALL_SOURCE", "standalone")
        .args(["--json", "doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"binaries\""))
        .stdout(predicate::str::contains("\"gdalinfo\""))
        .stdout(predicate::str::contains("\"ncdump\""));
}

#[test]
fn self_update_redirects_scoop_installs() {
    let temp_dir = TempDir::new().expect("create temp dir");

    binary()
        .current_dir(temp_dir.path())
        .env("HOME", temp_dir.path())
        .env("GEOCODE_INSTALL_SOURCE", "scoop")
        .arg("self-update")
        .assert()
        .success()
        .stdout(predicate::str::contains("scoop update geocode"));
}

#[test]
fn self_update_redirects_winget_installs() {
    let temp_dir = TempDir::new().expect("create temp dir");

    binary()
        .current_dir(temp_dir.path())
        .env("HOME", temp_dir.path())
        .env("GEOCODE_INSTALL_SOURCE", "winget")
        .arg("self-update")
        .assert()
        .success()
        .stdout(predicate::str::contains("winget upgrade GeoCode.GeoCode"));
}

#[test]
fn cli_commands_work_on_clean_machine_paths() {
    let temp_dir = TempDir::new().expect("create temp dir");

    binary()
        .current_dir(temp_dir.path())
        .env("HOME", temp_dir.path())
        .env_remove("GEOCODE_INSTALL_SOURCE")
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("Config Dir:"));

    binary()
        .current_dir(temp_dir.path())
        .env("HOME", temp_dir.path())
        .arg("version")
        .assert()
        .success()
        .stdout(predicate::str::contains("GeoCode"));
}

fn binary() -> assert_cmd::Command {
    assert_cmd::Command::cargo_bin("geocode").expect("build test binary")
}
