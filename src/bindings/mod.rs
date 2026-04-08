mod dataset;
mod gdal;
mod netcdf;
mod process;
mod types;

pub use dataset::{detect_dataset_kind, validate_local_file};
pub use gdal::{read_geotiff_mean, read_geotiff_metadata};
pub use netcdf::{read_netcdf_metadata, read_netcdf_variable_values};
pub use process::run_gdalinfo_json;
pub use types::{
    DimensionInfo, GeotiffBandMetadata, GeotiffMetadata, NetcdfMetadata, VariableMetadata,
};
