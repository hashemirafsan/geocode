use std::path::Path;

use gdal::Dataset;

use crate::{
    bindings::{GeotiffBandMetadata, GeotiffMetadata, run_gdalinfo_json},
    engine::ExecutionError,
};

pub fn read_geotiff_metadata(path: &Path) -> Result<GeotiffMetadata, ExecutionError> {
    let dataset = Dataset::open(path)
        .map_err(|err| ExecutionError::Command(format!("failed to open geotiff dataset: {err}")))?;
    let (width, height) = dataset.raster_size();

    let mut bands = Vec::with_capacity(dataset.raster_count());
    for index in 1..=dataset.raster_count() {
        let band = dataset.rasterband(index).map_err(|err| {
            ExecutionError::Command(format!("failed to access raster band: {err}"))
        })?;
        bands.push(GeotiffBandMetadata {
            band: index as u64,
            dtype: format!("{}", band.band_type()),
            nodata: band.no_data_value(),
        });
    }

    Ok(GeotiffMetadata {
        width: width as u64,
        height: height as u64,
        band_count: bands.len(),
        bands,
    })
}

pub fn read_geotiff_mean(path: &Path) -> Result<(f64, Option<f64>), ExecutionError> {
    let value = run_gdalinfo_json(path, true)?;
    let metadata = read_geotiff_metadata(path)?;

    if metadata.band_count != 1 {
        return Err(ExecutionError::InvalidInput(
            "mean currently supports single-band GeoTIFF files only".into(),
        ));
    }

    let band = value["bands"]
        .as_array()
        .and_then(|bands| bands.first())
        .ok_or_else(|| ExecutionError::Parse("gdalinfo output missing first band".into()))?;

    let mean = band
        .get("mean")
        .and_then(|value| value.as_f64())
        .ok_or_else(|| ExecutionError::Parse("gdalinfo output missing band mean".into()))?;

    let nodata = metadata.bands.first().and_then(|band| band.nodata);
    Ok((mean, nodata))
}

#[cfg(test)]
mod tests {
    use std::{path::Path, process::Command};

    use tempfile::TempDir;
    use tiff::encoder::{TiffEncoder, colortype::Gray8};

    use super::{read_geotiff_mean, read_geotiff_metadata};

    #[test]
    fn reads_geotiff_metadata_via_gdal_crate() {
        let temp_dir = TempDir::new().expect("temp dir");
        let file = create_sample_tiff(temp_dir.path());

        let metadata = read_geotiff_metadata(&file).expect("read metadata");

        assert_eq!(metadata.width, 3);
        assert_eq!(metadata.height, 2);
        assert_eq!(metadata.band_count, 1);
        assert_eq!(metadata.bands[0].dtype, "Byte");
    }

    #[test]
    fn reads_geotiff_mean_via_process_backed_stats_path() {
        let temp_dir = TempDir::new().expect("temp dir");
        let file = create_nodata_tiff(temp_dir.path());

        let (mean, nodata) = read_geotiff_mean(&file).expect("read mean");

        assert!((mean - 3.2).abs() < 1e-9);
        assert_eq!(nodata, Some(0.0));
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
