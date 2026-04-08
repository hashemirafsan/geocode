#![allow(dead_code)]

mod compare;
mod geotiff;
mod netcdf;
mod shared;
pub(crate) mod types;

pub use compare::compare;
pub use shared::{detect_dataset_kind, inspect, mean};
pub use types::{CompareReport, InspectReport, MeanReport};
