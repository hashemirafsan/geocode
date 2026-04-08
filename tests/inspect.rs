use std::{fs, path::Path, process::Command};

use serde_json::Value;
use tempfile::TempDir;
use tiff::encoder::{colortype::Gray8, TiffEncoder};

#[test]
fn inspect_netcdf_text_output_includes_essential_metadata() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let file = create_sample_netcdf(temp_dir.path());

    let assert = binary()
        .current_dir(temp_dir.path())
        .arg("inspect")
        .arg(&file)
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8 stdout");
    assert!(stdout.contains("File:"));
    assert!(stdout.contains("Type: netcdf"));
    assert!(stdout.contains("Variables:"));
    assert!(stdout.contains("depth (float) [time=2, x=3]"));
    assert!(stdout.contains("Dimensions: time=2, x=3"));
}

#[test]
fn inspect_netcdf_json_output_includes_structured_metadata() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let file = create_sample_netcdf(temp_dir.path());

    let assert = binary()
        .current_dir(temp_dir.path())
        .arg("--json")
        .arg("inspect")
        .arg(&file)
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8 stdout");
    let json: Value = serde_json::from_str(&stdout).expect("json output");

    assert_eq!(json["command"], "inspect");
    assert_eq!(json["dataset_kind"], "netcdf");
    assert_eq!(json["details"]["kind"], "netcdf");
    assert_eq!(json["details"]["netcdf"]["dimensions"][0]["name"], "time");
    assert_eq!(json["details"]["netcdf"]["variables"][0]["name"], "depth");
}

#[test]
fn inspect_geotiff_text_output_includes_essential_metadata() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let file = create_sample_tiff(temp_dir.path());

    let assert = binary()
        .current_dir(temp_dir.path())
        .arg("inspect")
        .arg(&file)
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8 stdout");
    assert!(stdout.contains("File:"));
    assert!(stdout.contains("Type: geotiff"));
    assert!(stdout.contains("Size: 3 x 2"));
    assert!(stdout.contains("Bands: 1"));
    assert!(stdout.contains("Band 1: Byte"));
}

#[test]
fn inspect_geotiff_json_output_includes_structured_metadata() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let file = create_sample_tiff(temp_dir.path());

    let assert = binary()
        .current_dir(temp_dir.path())
        .arg("--json")
        .arg("inspect")
        .arg(&file)
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8 stdout");
    let json: Value = serde_json::from_str(&stdout).expect("json output");

    assert_eq!(json["command"], "inspect");
    assert_eq!(json["dataset_kind"], "geotiff");
    assert_eq!(json["details"]["geotiff"]["width"], 3);
    assert_eq!(json["details"]["geotiff"]["height"], 2);
    assert_eq!(json["details"]["geotiff"]["band_count"], 1);
}

#[test]
fn inspect_unsupported_file_fails_explicitly() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let file = temp_dir.path().join("unsupported.txt");
    fs::write(&file, "plain text").expect("write test file");

    binary()
        .current_dir(temp_dir.path())
        .arg("inspect")
        .arg(&file)
        .assert()
        .failure()
        .stderr(predicates::str::contains("unsupported dataset type"));
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

fn create_sample_tiff(dir: &Path) -> std::path::PathBuf {
    let file = dir.join("sample.tif");
    let writer = std::fs::File::create(&file).expect("create tiff file");
    let mut encoder = TiffEncoder::new(writer).expect("create tiff encoder");
    let image = encoder.new_image::<Gray8>(3, 2).expect("create gray image");

    image
        .write_data(&[1_u8, 2, 3, 4, 5, 6])
        .expect("write raster data");

    file
}
