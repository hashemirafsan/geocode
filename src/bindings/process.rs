use std::path::Path;

use crate::{
    engine::ExecutionError,
    runtime::{HostRuntime, KnownBinary},
};

pub fn run_gdalinfo_json(
    path: &Path,
    with_stats: bool,
) -> Result<serde_json::Value, ExecutionError> {
    let path = path.display().to_string();
    let mut args = vec!["-json"];
    if with_stats {
        args.push("-stats");
    }
    args.push(&path);

    let output = HostRuntime::discover().run_known(KnownBinary::GdalInfo, &args)?;

    if !output.status.success() {
        return Err(ExecutionError::Command(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }

    serde_json::from_slice(&output.stdout)
        .map_err(|err| ExecutionError::Parse(format!("invalid gdalinfo json: {err}")))
}

#[cfg(test)]
mod tests {
    use std::{path::Path, process::Command};

    use tempfile::TempDir;
    use tiff::encoder::{TiffEncoder, colortype::Gray8};

    use super::run_gdalinfo_json;

    #[test]
    fn run_gdalinfo_json_returns_expected_shape() {
        let temp_dir = TempDir::new().expect("temp dir");
        let file = create_sample_tiff(temp_dir.path());

        let json = run_gdalinfo_json(&file, false).expect("run gdalinfo");

        assert_eq!(json["size"][0].as_u64(), Some(3));
        assert_eq!(json["size"][1].as_u64(), Some(2));
        assert_eq!(json["bands"].as_array().map(|bands| bands.len()), Some(1));
    }

    #[test]
    fn run_gdalinfo_json_with_stats_returns_band_mean() {
        let temp_dir = TempDir::new().expect("temp dir");
        let file = create_nodata_tiff(temp_dir.path());

        let json = run_gdalinfo_json(&file, true).expect("run gdalinfo with stats");

        let mean = json["bands"][0]["mean"].as_f64().expect("mean present");
        assert!((mean - 3.2).abs() < 1e-9);
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
}
