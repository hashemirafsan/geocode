use ndarray::{Array1, ArrayD, IxDyn, s};
use serde::Serialize;
use serde::ser::{SerializeStruct, Serializer};

use crate::engine::ExecutionError;

#[derive(Debug, Clone)]
pub struct ArrayValue {
    data: ArrayD<f64>,
}

impl ArrayValue {
    pub fn from_shape_values(shape: Vec<usize>, values: Vec<f64>) -> Result<Self, ExecutionError> {
        ArrayD::from_shape_vec(IxDyn(&shape), values)
            .map(|data| Self { data })
            .map_err(|err| ExecutionError::Parse(format!("invalid array shape/values: {err}")))
    }

    pub fn from_flat(values: Vec<f64>) -> Self {
        Self {
            data: Array1::from_vec(values).into_dyn(),
        }
    }

    pub fn values(&self) -> Vec<f64> {
        self.data.iter().copied().collect()
    }

    pub fn shape(&self) -> Vec<usize> {
        self.data.shape().to_vec()
    }
}

impl Serialize for ArrayValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("ArrayValue", 2)?;
        state.serialize_field("values", &self.values())?;
        state.serialize_field("shape", &self.shape())?;
        state.end()
    }
}

pub fn sort(array: &ArrayValue, descending: bool) -> ArrayValue {
    let mut values = array.values();
    if descending {
        values.sort_by(|a, b| b.total_cmp(a));
    } else {
        values.sort_by(|a, b| a.total_cmp(b));
    }

    ArrayValue::from_flat(values)
}

pub fn take(array: &ArrayValue, count: usize, from_end: bool) -> ArrayValue {
    let values = Array1::from_vec(array.values());
    let taken = if from_end {
        let len = values.len();
        let start = len.saturating_sub(count);
        values.slice(s![start..]).to_vec()
    } else {
        values.slice(s![..count.min(values.len())]).to_vec()
    };

    ArrayValue::from_flat(taken)
}

pub fn mean(array: &ArrayValue) -> Result<f64, ExecutionError> {
    let values = array.values();
    if values.is_empty() {
        return Err(ExecutionError::InvalidInput(
            "cannot compute a mean over zero values".into(),
        ));
    }

    Ok(values.iter().sum::<f64>() / values.len() as f64)
}

pub fn min(array: &ArrayValue) -> Result<f64, ExecutionError> {
    array
        .values()
        .into_iter()
        .reduce(f64::min)
        .ok_or_else(|| ExecutionError::InvalidInput("cannot compute min over zero values".into()))
}

pub fn max(array: &ArrayValue) -> Result<f64, ExecutionError> {
    array
        .values()
        .into_iter()
        .reduce(f64::max)
        .ok_or_else(|| ExecutionError::InvalidInput("cannot compute max over zero values".into()))
}
