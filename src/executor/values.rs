use std::collections::BTreeMap;

use serde::Serialize;

use crate::{
    engine::DatasetRef,
    tools::{CompareReport, InspectReport, MeanReport},
};

#[derive(Debug, Clone, Serialize)]
pub struct ScalarValue {
    pub label: String,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TableRow {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TableValue {
    pub title: String,
    pub rows: Vec<TableRow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TextValue {
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum RuntimeValue {
    DatasetRef(DatasetRef),
    InspectReport(InspectReport),
    MeanReport(MeanReport),
    CompareReport(CompareReport),
    ScalarValue(ScalarValue),
    TableValue(TableValue),
    TextValue(TextValue),
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ValueStore {
    pub values: BTreeMap<String, RuntimeValue>,
}

impl ValueStore {
    pub fn insert(&mut self, id: impl Into<String>, value: RuntimeValue) {
        self.values.insert(id.into(), value);
    }

    pub fn get(&self, id: &str) -> Option<&RuntimeValue> {
        self.values.get(id)
    }
}
