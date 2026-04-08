use std::{fs, path::Path, process::Command};

use predicates::prelude::*;
use serde_json::Value;
use tempfile::TempDir;
use tiff::encoder::{colortype::Gray8, TiffEncoder};

#[test]
fn compare_netcdf_text_output_reports_difference_b_minus_a() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let file_a = create_sample_netcdf(temp_dir.path(), "a.cdl", "a.nc", &[1.0, 2.0, 3.0, 4.0]);
    let file_b = create_sample_netcdf(temp_dir.path(), "b.cdl", "b.nc", &[2.0, 4.0, 6.0, 8.0]);

    let assert = binary()
        .current_dir(temp_dir.path())
        .arg("compare")
        .arg(&file_a)
        .arg(&file_b)
        .arg("--var")
        .arg("depth")
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8 stdout");
    assert!(stdout.contains("Type: netcdf"));
    assert!(stdout.contains("Variable: depth"));
    assert!(stdout.contains("Mean A: 2.500000"));
    assert!(stdout.contains("Mean B: 5.000000"));
    assert!(stdout.contains("Difference (B - A): 2.500000"));
}

#[test]
fn compare_netcdf_json_output_reports_structured_difference() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let file_a = create_sample_netcdf(temp_dir.path(), "a.cdl", "a.nc", &[1.0, 2.0, 3.0, 4.0]);
    let file_b = create_sample_netcdf(temp_dir.path(), "b.cdl", "b.nc", &[2.0, 4.0, 6.0, 8.0]);

    let assert = binary()
        .current_dir(temp_dir.path())
        .arg("--json")
        .arg("compare")
        .arg(&file_a)
        .arg(&file_b)
        .arg("--var")
        .arg("depth")
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8 stdout");
    let json: Value = serde_json::from_str(&stdout).expect("json output");

    assert_eq!(json["command"], "compare");
    assert_eq!(json["dataset_kind"], "netcdf");
    assert_eq!(json["details"]["mean_a"], 2.5);
    assert_eq!(json["details"]["mean_b"], 5.0);
    assert_eq!(json["details"]["difference"], 2.5);
}

#[test]
fn compare_geotiff_text_output_reports_difference_b_minus_a() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let file_a = create_nodata_tiff(temp_dir.path(), "a_base.tif", "a.tif", &[1, 2, 3, 4, 0, 6]);
    let file_b = create_nodata_tiff(temp_dir.path(), "b_base.tif", "b.tif", &[2, 4, 6, 8, 0, 10]);

    let assert = binary()
        .current_dir(temp_dir.path())
        .arg("compare")
        .arg(&file_a)
        .arg(&file_b)
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8 stdout");
    assert!(stdout.contains("Type: geotiff"));
    assert!(stdout.contains("Mean A: 3.200000"));
    assert!(stdout.contains("Mean B: 6.000000"));
    assert!(stdout.contains("Difference (B - A): 2.800000"));
}

#[test]
fn compare_geotiff_json_output_reports_structured_difference() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let file_a = create_nodata_tiff(temp_dir.path(), "a_base.tif", "a.tif", &[1, 2, 3, 4, 0, 6]);
    let file_b = create_nodata_tiff(temp_dir.path(), "b_base.tif", "b.tif", &[2, 4, 6, 8, 0, 10]);

    let assert = binary()
        .current_dir(temp_dir.path())
        .arg("--json")
        .arg("compare")
        .arg(&file_a)
        .arg(&file_b)
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8 stdout");
    let json: Value = serde_json::from_str(&stdout).expect("json output");

    assert_eq!(json["command"], "compare");
    assert_eq!(json["dataset_kind"], "geotiff");
    assert_eq!(json["details"]["mean_a"], 3.2);
    assert_eq!(json["details"]["mean_b"], 6.0);
    assert_eq!(json["details"]["difference"], 2.8);
}

#[test]
fn compare_mixed_types_fails_explicitly() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let file_a = create_sample_netcdf(temp_dir.path(), "a.cdl", "a.nc", &[1.0, 2.0, 3.0, 4.0]);
    let file_b = create_nodata_tiff(temp_dir.path(), "b_base.tif", "b.tif", &[2, 4, 6, 8, 0, 10]);

    binary()
        .current_dir(temp_dir.path())
        .arg("compare")
        .arg(&file_a)
        .arg(&file_b)
        .arg("--var")
        .arg("depth")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "compare supports same-type files only",
        ));
}

#[test]
fn compare_netcdf_missing_var_fails_explicitly() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let file_a = create_sample_netcdf(temp_dir.path(), "a.cdl", "a.nc", &[1.0, 2.0, 3.0, 4.0]);
    let file_b = create_sample_netcdf(temp_dir.path(), "b.cdl", "b.nc", &[2.0, 4.0, 6.0, 8.0]);

    binary()
        .current_dir(temp_dir.path())
        .arg("compare")
        .arg(&file_a)
        .arg(&file_b)
        .assert()
        .failure()
        .stderr(predicate::str::contains("variable selection is required"));
}

fn binary() -> assert_cmd::Command {
    assert_cmd::Command::cargo_bin("geocode").expect("build test binary")
}

fn create_sample_netcdf(
    dir: &Path,
    cdl_name: &str,
    file_name: &str,
    values: &[f64],
) -> std::path::PathBuf {
    let cdl = dir.join(cdl_name);
    let file = dir.join(file_name);
    let values = values
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(", ");

    fs::write(
        &cdl,
        format!(
            "netcdf sample {{\ndimensions:\n    time = 2 ;\n    x = 2 ;\nvariables:\n    float depth(time, x) ;\ndata:\n    depth = {values} ;\n}}\n"
        ),
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

fn create_nodata_tiff(
    dir: &Path,
    base_name: &str,
    output_name: &str,
    values: &[u8],
) -> std::path::PathBuf {
    let base = dir.join(base_name);
    let output = dir.join(output_name);

    let writer = std::fs::File::create(&base).expect("create tiff file");
    let mut encoder = TiffEncoder::new(writer).expect("create tiff encoder");
    let image = encoder.new_image::<Gray8>(3, 2).expect("create gray image");

    image.write_data(values).expect("write raster data");

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
