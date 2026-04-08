use std::{fs, path::Path, process::Command};

use predicates::prelude::*;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn session_show_reports_current_persisted_state() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let file = create_sample_netcdf(temp_dir.path());

    binary()
        .current_dir(temp_dir.path())
        .arg("mean")
        .arg(&file)
        .arg("--var")
        .arg("depth")
        .assert()
        .success();

    let assert = binary()
        .current_dir(temp_dir.path())
        .arg("session")
        .arg("show")
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8 stdout");
    assert!(stdout.contains("Workspace Path:"));
    assert!(stdout.contains(temp_dir.path().to_str().expect("temp dir str")));
    assert!(stdout.contains("Last Variable: depth"));
}

#[test]
fn session_show_json_reports_current_persisted_state() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let file = create_sample_netcdf(temp_dir.path());

    binary()
        .current_dir(temp_dir.path())
        .arg("mean")
        .arg(&file)
        .arg("--var")
        .arg("depth")
        .assert()
        .success();

    let assert = binary()
        .current_dir(temp_dir.path())
        .arg("--json")
        .arg("session")
        .arg("show")
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8 stdout");
    let json: Value = serde_json::from_str(&stdout).expect("json output");

    assert_eq!(json["command"], "session_show");
    assert_eq!(json["details"]["session"]["last_variable"], "depth");
    assert_eq!(
        json["details"]["session"]["workspace_path"],
        temp_dir.path().to_str().expect("temp dir str")
    );
}

#[test]
fn session_clear_resets_persisted_state() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let file = create_sample_netcdf(temp_dir.path());

    binary()
        .current_dir(temp_dir.path())
        .arg("mean")
        .arg(&file)
        .arg("--var")
        .arg("depth")
        .assert()
        .success();

    let session_file = temp_dir.path().join(".geocode-session.json");
    assert!(session_file.exists());

    binary()
        .current_dir(temp_dir.path())
        .arg("session")
        .arg("clear")
        .assert()
        .success()
        .stdout(predicate::str::contains("Session cleared"));

    assert!(!session_file.exists());

    let assert = binary()
        .current_dir(temp_dir.path())
        .arg("session")
        .arg("show")
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8 stdout");
    assert!(stdout.contains("Last Variable: <none>"));
    assert!(stdout.contains("Workspace Path: <none>"));
}

fn binary() -> assert_cmd::Command {
    assert_cmd::Command::cargo_bin("geocode").expect("build test binary")
}

fn create_sample_netcdf(dir: &Path) -> std::path::PathBuf {
    let cdl = dir.join("sample.cdl");
    let file = dir.join("sample.nc");

    fs::write(
        &cdl,
        r#"netcdf sample {
dimensions:
    time = 2 ;
    x = 3 ;
variables:
    float depth(time, x) ;
data:
    depth = 1, 2, 3, 4, 5, 6 ;
}
"#,
    )
    .expect("write cdl");

    let status = Command::new("ncgen")
        .arg("-o")
        .arg(&file)
        .arg(&cdl)
        .status()
        .expect("run ncgen");

    assert!(status.success(), "ncgen should succeed");
    file
}
