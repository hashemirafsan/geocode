use std::{fs, path::Path, process::Command};

use predicates::prelude::*;
use serde_json::Value;
use tempfile::TempDir;
use tiff::encoder::{colortype::Gray8, TiffEncoder};

#[test]
fn mean_netcdf_text_output_reports_variable_mean() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let file = create_sample_netcdf(temp_dir.path());

    let assert = binary()
        .current_dir(temp_dir.path())
        .arg("mean")
        .arg(&file)
        .arg("--var")
        .arg("depth")
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8 stdout");
    assert!(stdout.contains("Type: netcdf"));
    assert!(stdout.contains("Variable: depth"));
    assert!(stdout.contains("Mean: 3.500000"));
}

#[test]
fn mean_netcdf_json_output_reports_structured_mean() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let file = create_sample_netcdf(temp_dir.path());

    let assert = binary()
        .current_dir(temp_dir.path())
        .arg("--json")
        .arg("mean")
        .arg(&file)
        .arg("--var")
        .arg("depth")
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8 stdout");
    let json: Value = serde_json::from_str(&stdout).expect("json output");

    assert_eq!(json["command"], "mean");
    assert_eq!(json["dataset_kind"], "netcdf");
    assert_eq!(json["details"]["variable"], "depth");
    assert_eq!(json["details"]["mean"], 3.5);
}

#[test]
fn mean_geotiff_text_output_reports_nodata_aware_mean() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let file = create_nodata_tiff(temp_dir.path());

    let assert = binary()
        .current_dir(temp_dir.path())
        .arg("mean")
        .arg(&file)
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8 stdout");
    assert!(stdout.contains("Type: geotiff"));
    assert!(stdout.contains("Mean: 3.200000"));
    assert!(stdout.contains("Nodata: 0"));
}

#[test]
fn mean_geotiff_json_output_reports_structured_mean() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let file = create_nodata_tiff(temp_dir.path());

    let assert = binary()
        .current_dir(temp_dir.path())
        .arg("--json")
        .arg("mean")
        .arg(&file)
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8 stdout");
    let json: Value = serde_json::from_str(&stdout).expect("json output");

    assert_eq!(json["command"], "mean");
    assert_eq!(json["dataset_kind"], "geotiff");
    assert_eq!(json["details"]["mean"], 3.2);
    assert_eq!(json["details"]["nodata"], 0.0);
}

#[test]
fn mean_netcdf_without_var_fails_explicitly() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let file = create_sample_netcdf(temp_dir.path());

    binary()
        .current_dir(temp_dir.path())
        .arg("mean")
        .arg(&file)
        .assert()
        .failure()
        .stderr(predicate::str::contains("variable selection is required"));
}

#[test]
fn mean_netcdf_with_invalid_var_fails_explicitly() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let file = create_sample_netcdf(temp_dir.path());

    binary()
        .current_dir(temp_dir.path())
        .arg("mean")
        .arg(&file)
        .arg("--var")
        .arg("missing")
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid variable: missing"));
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

fn create_nodata_tiff(dir: &Path) -> std::path::PathBuf {
    let base = dir.join("base.tif");
    let output = dir.join("sample.tif");

    let writer = std::fs::File::create(&base).expect("create tiff file");
    let mut encoder = TiffEncoder::new(writer).expect("create tiff encoder");
    let image = encoder.new_image::<Gray8>(3, 2).expect("create gray image");

    image
        .write_data(&[1_u8, 2, 3, 4, 0, 6])
        .expect("write raster data");

    let status = Command::new("gdal_translate")
        .arg("-q")
        .arg("-a_nodata")
        .arg("0")
        .arg(&base)
        .arg(&output)
        .status()
        .expect("run gdal_translate");

    assert!(status.success(), "gdal_translate should succeed");
    output
}
