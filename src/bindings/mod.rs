mod dataset;
mod gdal;
mod netcdf;
mod process;
mod types;

pub use dataset::{detect_dataset_kind, validate_local_file};
pub use gdal::{read_geotiff_mean, read_geotiff_metadata};
pub use netcdf::{
    describe_netcdf_variable, list_netcdf_dimensions, list_netcdf_variables, open_netcdf_dataset,
    read_netcdf_metadata, read_netcdf_variable, read_netcdf_variable_values,
};
pub use process::run_gdalinfo_json;
pub use types::{
    ArrayValue, DatasetHandle, DimensionInfo, GeotiffBandMetadata, GeotiffMetadata,
    NetcdfDatasetHandle, NetcdfMetadata, NetcdfVariableRef, RasterDatasetHandle, VariableMetadata,
};
