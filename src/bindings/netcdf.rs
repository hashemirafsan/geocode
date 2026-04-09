use std::path::Path;

use netcdf::types::{FloatType, IntType, NcVariableType};

use crate::{
    bindings::{
        ArrayValue, DimensionInfo, NetcdfDatasetHandle, NetcdfMetadata, NetcdfVariableRef,
        VariableMetadata,
    },
    engine::{DatasetKind, DatasetRef, ExecutionError},
};

pub fn open_netcdf_dataset(dataset: &DatasetRef) -> Result<NetcdfDatasetHandle, ExecutionError> {
    if !matches!(dataset.kind, DatasetKind::Netcdf) {
        return Err(ExecutionError::Capability(
            "dataset.open currently expects a NetCDF dataset for netcdf operations".into(),
        ));
    }

    netcdf::open(&dataset.path)
        .map_err(|err| ExecutionError::Command(format!("failed to open netcdf dataset: {err}")))?;

    Ok(NetcdfDatasetHandle {
        dataset: dataset.clone(),
    })
}

pub fn list_netcdf_dimensions(
    handle: &NetcdfDatasetHandle,
) -> Result<Vec<DimensionInfo>, ExecutionError> {
    let file = netcdf::open(&handle.dataset.path)
        .map_err(|err| ExecutionError::Command(format!("failed to open netcdf dataset: {err}")))?;

    Ok(file
        .dimensions()
        .map(|dimension| DimensionInfo {
            name: dimension.name(),
            length: Some(dimension.len()),
        })
        .collect())
}

pub fn list_netcdf_variables(
    handle: &NetcdfDatasetHandle,
) -> Result<Vec<NetcdfVariableRef>, ExecutionError> {
    let file = netcdf::open(&handle.dataset.path)
        .map_err(|err| ExecutionError::Command(format!("failed to open netcdf dataset: {err}")))?;

    Ok(file
        .variables()
        .map(|variable| NetcdfVariableRef {
            dataset: handle.clone(),
            name: variable.name(),
        })
        .collect())
}

pub fn describe_netcdf_variable(
    variable_ref: &NetcdfVariableRef,
) -> Result<VariableMetadata, ExecutionError> {
    let file = netcdf::open(&variable_ref.dataset.dataset.path)
        .map_err(|err| ExecutionError::Command(format!("failed to open netcdf dataset: {err}")))?;
    let variable = file
        .variable(&variable_ref.name)
        .ok_or_else(|| ExecutionError::InvalidVariable(variable_ref.name.clone()))?;
    let dims = variable.dimensions();

    Ok(VariableMetadata {
        name: variable.name(),
        dtype: format_netcdf_dtype(&variable.vartype()),
        dimensions: dims.iter().map(|dimension| dimension.name()).collect(),
        shape: dims.iter().map(|dimension| dimension.len()).collect(),
    })
}

pub fn read_netcdf_variable(
    variable_ref: &NetcdfVariableRef,
) -> Result<ArrayValue, ExecutionError> {
    let metadata = describe_netcdf_variable(variable_ref)?;
    let values =
        read_netcdf_variable_values(&variable_ref.dataset.dataset.path, &variable_ref.name)?;

    ArrayValue::from_shape_values(metadata.shape, values)
}

pub fn read_netcdf_metadata(path: &Path) -> Result<NetcdfMetadata, ExecutionError> {
    let dataset = DatasetRef {
        path: path.to_path_buf(),
        kind: DatasetKind::Netcdf,
    };
    let handle = open_netcdf_dataset(&dataset)?;
    let dimensions = list_netcdf_dimensions(&handle)?;
    let variables = list_netcdf_variables(&handle)?
        .iter()
        .map(describe_netcdf_variable)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(NetcdfMetadata {
        dimensions,
        variables,
    })
}

pub fn read_netcdf_variable_values(
    path: &Path,
    variable_name: &str,
) -> Result<Vec<f64>, ExecutionError> {
    let file = netcdf::open(path)
        .map_err(|err| ExecutionError::Command(format!("failed to open netcdf dataset: {err}")))?;
    let variable = file
        .variable(variable_name)
        .ok_or_else(|| ExecutionError::InvalidVariable(variable_name.to_string()))?;

    let values = match variable.vartype() {
        NcVariableType::Float(FloatType::F32) => variable
            .get_values::<f32, _>(..)
            .map_err(|err| ExecutionError::Parse(format!("failed to read variable values: {err}")))?
            .into_iter()
            .map(f64::from)
            .collect(),
        NcVariableType::Float(FloatType::F64) => {
            variable.get_values::<f64, _>(..).map_err(|err| {
                ExecutionError::Parse(format!("failed to read variable values: {err}"))
            })?
        }
        NcVariableType::Int(IntType::U8) => variable
            .get_values::<u8, _>(..)
            .map_err(|err| ExecutionError::Parse(format!("failed to read variable values: {err}")))?
            .into_iter()
            .map(f64::from)
            .collect(),
        NcVariableType::Int(IntType::U16) => variable
            .get_values::<u16, _>(..)
            .map_err(|err| ExecutionError::Parse(format!("failed to read variable values: {err}")))?
            .into_iter()
            .map(f64::from)
            .collect(),
        NcVariableType::Int(IntType::U32) => variable
            .get_values::<u32, _>(..)
            .map_err(|err| ExecutionError::Parse(format!("failed to read variable values: {err}")))?
            .into_iter()
            .map(|value| value as f64)
            .collect(),
        NcVariableType::Int(IntType::U64) => variable
            .get_values::<u64, _>(..)
            .map_err(|err| ExecutionError::Parse(format!("failed to read variable values: {err}")))?
            .into_iter()
            .map(|value| value as f64)
            .collect(),
        NcVariableType::Int(IntType::I8) => variable
            .get_values::<i8, _>(..)
            .map_err(|err| ExecutionError::Parse(format!("failed to read variable values: {err}")))?
            .into_iter()
            .map(f64::from)
            .collect(),
        NcVariableType::Int(IntType::I16) => variable
            .get_values::<i16, _>(..)
            .map_err(|err| ExecutionError::Parse(format!("failed to read variable values: {err}")))?
            .into_iter()
            .map(f64::from)
            .collect(),
        NcVariableType::Int(IntType::I32) => variable
            .get_values::<i32, _>(..)
            .map_err(|err| ExecutionError::Parse(format!("failed to read variable values: {err}")))?
            .into_iter()
            .map(f64::from)
            .collect(),
        NcVariableType::Int(IntType::I64) => variable
            .get_values::<i64, _>(..)
            .map_err(|err| ExecutionError::Parse(format!("failed to read variable values: {err}")))?
            .into_iter()
            .map(|value| value as f64)
            .collect(),
        other => {
            return Err(ExecutionError::InvalidInput(format!(
                "unsupported netcdf variable type for mean: {other:?}"
            )))
        }
    };

    if values.is_empty() {
        return Err(ExecutionError::Parse(format!(
            "no numeric values found for variable {variable_name}"
        )));
    }

    Ok(values)
}

fn format_netcdf_dtype(dtype: &NcVariableType) -> String {
    match dtype {
        NcVariableType::Float(FloatType::F32) => "float".to_string(),
        NcVariableType::Float(FloatType::F64) => "double".to_string(),
        NcVariableType::Int(IntType::U8) => "ubyte".to_string(),
        NcVariableType::Int(IntType::U16) => "ushort".to_string(),
        NcVariableType::Int(IntType::U32) => "uint".to_string(),
        NcVariableType::Int(IntType::U64) => "uint64".to_string(),
        NcVariableType::Int(IntType::I8) => "byte".to_string(),
        NcVariableType::Int(IntType::I16) => "short".to_string(),
        NcVariableType::Int(IntType::I32) => "int".to_string(),
        NcVariableType::Int(IntType::I64) => "int64".to_string(),
        NcVariableType::Char => "char".to_string(),
        NcVariableType::String => "string".to_string(),
        other => format!("{:?}", other).to_lowercase(),
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path, process::Command};

    use tempfile::TempDir;

    use super::{
        describe_netcdf_variable, list_netcdf_dimensions, list_netcdf_variables,
        open_netcdf_dataset, read_netcdf_metadata, read_netcdf_variable,
        read_netcdf_variable_values,
    };
    use crate::engine::{DatasetKind, DatasetRef};

    #[test]
    fn reads_netcdf_metadata_via_crate_binding() {
        let temp_dir = TempDir::new().expect("temp dir");
        let file = create_sample_netcdf(temp_dir.path());

        let metadata = read_netcdf_metadata(&file).expect("read metadata");

        assert_eq!(metadata.dimensions.len(), 2);
        assert_eq!(metadata.dimensions[0].name, "time");
        assert_eq!(metadata.variables.len(), 1);
        assert_eq!(metadata.variables[0].name, "depth");
        assert_eq!(metadata.variables[0].dtype, "float");
        assert_eq!(metadata.variables[0].shape, vec![2, 3]);
    }

    #[test]
    fn reads_netcdf_variable_values_via_crate_binding() {
        let temp_dir = TempDir::new().expect("temp dir");
        let file = create_sample_netcdf(temp_dir.path());

        let values = read_netcdf_variable_values(&file, "depth").expect("read values");

        assert_eq!(values, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    }

    #[test]
    fn netcdf_binding_family_supports_open_list_describe_load() {
        let temp_dir = TempDir::new().expect("temp dir");
        let file = create_sample_netcdf(temp_dir.path());
        let dataset = DatasetRef {
            path: file,
            kind: DatasetKind::Netcdf,
        };

        let handle = open_netcdf_dataset(&dataset).expect("open netcdf handle");
        let dimensions = list_netcdf_dimensions(&handle).expect("list dimensions");
        let variables = list_netcdf_variables(&handle).expect("list variables");
        let metadata = describe_netcdf_variable(&variables[0]).expect("describe variable");
        let array = read_netcdf_variable(&variables[0]).expect("load variable");

        assert_eq!(dimensions.len(), 2);
        assert_eq!(variables[0].name, "depth");
        assert_eq!(metadata.shape, vec![2, 3]);
        assert_eq!(array.shape(), vec![2, 3]);
        assert_eq!(array.values().len(), 6);
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
}
