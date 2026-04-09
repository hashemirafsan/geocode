use std::path::Path;

use serde::Serialize;

use crate::{
    array,
    bindings::{
        describe_netcdf_variable, list_netcdf_dimensions, list_netcdf_variables,
        open_netcdf_dataset, read_netcdf_variable, ArrayValue, DatasetHandle, NetcdfDatasetHandle,
        NetcdfVariableRef, RasterDatasetHandle,
    },
    capability::CapabilityRegistry,
    engine::{DatasetKind, DatasetRef, ExecutionError, TraceEvent},
    executor::values::{
        DescribedNetcdfVariable, RuntimeValue, ScalarValue, TableRow, TableValue, TextValue,
        ValueStore,
    },
    plan::{CapabilityInput, ExecutionPlan, PlanValueRef},
    policy::ExecutionPolicy,
    runtime::{HostRuntime, KnownBinary},
    session::SessionState,
    tools,
};

#[derive(Debug, Clone, Serialize)]
pub struct ExecutionOutcome {
    pub goal: String,
    pub final_step: Option<String>,
    pub values: ValueStore,
    pub trace: Vec<TraceEvent>,
}

impl ExecutionOutcome {
    pub fn final_value(&self) -> Option<&RuntimeValue> {
        self.final_step
            .as_deref()
            .and_then(|step| self.values.get(step))
    }
}

#[derive(Debug, Clone)]
pub struct PlanExecutor {
    registry: CapabilityRegistry,
    policy: ExecutionPolicy,
    runtime: HostRuntime,
}

impl PlanExecutor {
    pub fn new(registry: CapabilityRegistry, policy: ExecutionPolicy) -> Self {
        Self {
            registry,
            policy,
            runtime: HostRuntime::discover(),
        }
    }

    pub fn execute(
        &self,
        plan: &ExecutionPlan,
        session: &mut SessionState,
    ) -> Result<ExecutionOutcome, ExecutionError> {
        self.policy.validate_plan(plan, &self.registry, session)?;

        let mut values = ValueStore::default();
        let mut trace = Vec::new();

        for step in &plan.steps {
            trace.push(TraceEvent {
                stage: "validate".to_string(),
                detail: format!("validated {}", step.capability.as_str()),
            });

            let value = match &step.input {
                CapabilityInput::DatasetResolve { alias, path } => RuntimeValue::DatasetRef(
                    self.resolve_dataset(alias.as_deref(), path.as_deref(), session)?,
                ),
                CapabilityInput::DatasetOpen { dataset } => {
                    let dataset = self.resolve_dataset_ref(dataset, &values, session)?;
                    match dataset.kind {
                        DatasetKind::Netcdf => RuntimeValue::DatasetHandle(DatasetHandle::Netcdf(
                            open_netcdf_dataset(&dataset)?,
                        )),
                        DatasetKind::Geotiff => RuntimeValue::DatasetHandle(DatasetHandle::Raster(
                            RasterDatasetHandle { dataset },
                        )),
                    }
                }
                CapabilityInput::DatasetInspect { dataset } => {
                    let dataset = self.resolve_dataset_ref(dataset, &values, session)?;
                    RuntimeValue::InspectReport(tools::inspect(Path::new(&dataset.path))?)
                }
                CapabilityInput::NetcdfDimensionList { dataset } => {
                    let handle = self.resolve_netcdf_dataset_handle(dataset, &values)?;
                    RuntimeValue::NetcdfDimensions(list_netcdf_dimensions(&handle)?)
                }
                CapabilityInput::NetcdfVariableList { dataset } => {
                    let handle = self.resolve_netcdf_dataset_handle(dataset, &values)?;
                    RuntimeValue::NetcdfVariables(list_netcdf_variables(&handle)?)
                }
                CapabilityInput::NetcdfVariableDescribe { dataset, name } => {
                    let handle = self.resolve_netcdf_dataset_handle(dataset, &values)?;
                    let variable_ref = NetcdfVariableRef {
                        dataset: handle,
                        name: name.clone(),
                    };
                    RuntimeValue::VariableMetadata(DescribedNetcdfVariable {
                        metadata: describe_netcdf_variable(&variable_ref)?,
                        reference: variable_ref,
                    })
                }
                CapabilityInput::NetcdfVariableLoad { dataset, name } => {
                    let variable_ref =
                        if let Ok(reference) = self.resolve_netcdf_variable_ref(dataset, &values) {
                            reference
                        } else {
                            let handle = self.resolve_netcdf_dataset_handle(dataset, &values)?;
                            NetcdfVariableRef {
                                dataset: handle,
                                name: name.clone(),
                            }
                        };
                    RuntimeValue::ArrayValue(read_netcdf_variable(&variable_ref)?)
                }
                CapabilityInput::ArraySort { input, descending } => {
                    let array = self.resolve_required_array_value(input, &values)?;
                    RuntimeValue::ArrayValue(array::sort(array, *descending))
                }
                CapabilityInput::ArrayTake {
                    input,
                    count,
                    from_end,
                } => {
                    let array = self.resolve_required_array_value(input, &values)?;
                    RuntimeValue::ArrayValue(array::take(array, *count, *from_end))
                }
                CapabilityInput::StatsMean { input, variable } => {
                    if let Some(array) = self.resolve_array_value(input, &values)? {
                        RuntimeValue::ScalarValue(ScalarValue {
                            label: variable.clone().unwrap_or_else(|| "mean".to_string()),
                            value: array::mean(array)?,
                        })
                    } else {
                        let dataset = self.resolve_dataset_target(input, &values, session)?;
                        RuntimeValue::MeanReport(tools::mean(
                            Path::new(&dataset.path),
                            variable.as_deref(),
                        )?)
                    }
                }
                CapabilityInput::StatsMin { input, variable } => {
                    let array = self.resolve_required_array_value(input, &values)?;
                    RuntimeValue::ScalarValue(ScalarValue {
                        label: variable.clone().unwrap_or_else(|| "min".to_string()),
                        value: array::min(array)?,
                    })
                }
                CapabilityInput::StatsMax { input, variable } => {
                    let array = self.resolve_required_array_value(input, &values)?;
                    RuntimeValue::ScalarValue(ScalarValue {
                        label: variable.clone().unwrap_or_else(|| "max".to_string()),
                        value: array::max(array)?,
                    })
                }
                CapabilityInput::CompareMeanDelta {
                    left,
                    right,
                    variable,
                } => {
                    let left = self.resolve_dataset_ref(left, &values, session)?;
                    let right = self.resolve_dataset_ref(right, &values, session)?;

                    RuntimeValue::CompareReport(tools::compare(
                        Path::new(&left.path),
                        Path::new(&right.path),
                        variable.as_deref(),
                    )?)
                }
                CapabilityInput::RenderScalar { input, label } => {
                    RuntimeValue::ScalarValue(self.render_scalar(label, input, &values)?)
                }
                CapabilityInput::RenderTable { inputs, title } => {
                    RuntimeValue::TableValue(self.render_table(title, inputs, &values)?)
                }
                CapabilityInput::ProcessRunKnown { binary, args } => {
                    let binary = match binary.as_str() {
                        "gdalinfo" => KnownBinary::GdalInfo,
                        "ncdump" => KnownBinary::NcDump,
                        other => {
                            return Err(ExecutionError::Capability(format!(
                                "unsupported known binary `{other}`"
                            )))
                        }
                    };
                    let owned_args = args.iter().map(String::as_str).collect::<Vec<_>>();
                    let output = self.runtime.run_known(binary, &owned_args)?;
                    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    RuntimeValue::TextValue(TextValue { text })
                }
            };

            trace.push(TraceEvent {
                stage: "execute".to_string(),
                detail: format!("executed {} as {}", step.id, step.capability.as_str()),
            });
            values.insert(step.id.clone(), value);
        }

        Ok(ExecutionOutcome {
            goal: plan.goal.clone(),
            final_step: plan.final_step_id().map(|step| step.to_string()),
            values,
            trace,
        })
    }

    fn resolve_dataset(
        &self,
        alias: Option<&str>,
        path: Option<&str>,
        session: &SessionState,
    ) -> Result<DatasetRef, ExecutionError> {
        let path = if let Some(alias) = alias {
            session
                .aliases
                .iter()
                .find(|entry| entry.alias == alias)
                .map(|entry| entry.path.clone())
                .ok_or_else(|| {
                    ExecutionError::InvalidInput(format!("unknown dataset alias: {alias}"))
                })?
        } else if let Some(path) = path {
            std::path::PathBuf::from(path)
        } else {
            return Err(ExecutionError::Plan(
                "dataset.resolve requires either alias or path".into(),
            ));
        };

        let kind = tools::detect_dataset_kind(&path)?;
        Ok(DatasetRef { path, kind })
    }

    fn resolve_dataset_ref(
        &self,
        reference: &PlanValueRef,
        values: &ValueStore,
        session: &SessionState,
    ) -> Result<DatasetRef, ExecutionError> {
        match reference {
            PlanValueRef::Step { step } => match values.get(step) {
                Some(RuntimeValue::DatasetRef(dataset)) => Ok(dataset.clone()),
                _ => Err(ExecutionError::Plan(format!(
                    "step `{step}` did not produce a dataset reference"
                ))),
            },
            PlanValueRef::Alias { alias } => self.resolve_dataset(Some(alias), None, session),
            PlanValueRef::Path { path } => self.resolve_dataset(None, Some(path), session),
        }
    }

    fn resolve_dataset_target(
        &self,
        reference: &PlanValueRef,
        values: &ValueStore,
        session: &SessionState,
    ) -> Result<DatasetRef, ExecutionError> {
        match reference {
            PlanValueRef::Step { step } => match values.get(step) {
                Some(RuntimeValue::DatasetRef(dataset)) => Ok(dataset.clone()),
                Some(RuntimeValue::DatasetHandle(DatasetHandle::Netcdf(handle))) => {
                    Ok(handle.dataset.clone())
                }
                Some(RuntimeValue::DatasetHandle(DatasetHandle::Raster(handle))) => {
                    Ok(handle.dataset.clone())
                }
                _ => Err(ExecutionError::Plan(format!(
                    "step `{step}` did not produce a dataset target"
                ))),
            },
            _ => self.resolve_dataset_ref(reference, values, session),
        }
    }

    fn resolve_netcdf_dataset_handle(
        &self,
        reference: &PlanValueRef,
        values: &ValueStore,
    ) -> Result<NetcdfDatasetHandle, ExecutionError> {
        match self.resolve_value(reference, values)? {
            RuntimeValue::DatasetHandle(DatasetHandle::Netcdf(handle)) => Ok(handle.clone()),
            _ => Err(ExecutionError::Plan(
                "step did not produce a netcdf dataset handle".into(),
            )),
        }
    }

    fn resolve_array_value<'a>(
        &self,
        reference: &PlanValueRef,
        values: &'a ValueStore,
    ) -> Result<Option<&'a ArrayValue>, ExecutionError> {
        match self.resolve_value(reference, values)? {
            RuntimeValue::ArrayValue(array) => Ok(Some(array)),
            _ => Ok(None),
        }
    }

    fn resolve_required_array_value<'a>(
        &self,
        reference: &PlanValueRef,
        values: &'a ValueStore,
    ) -> Result<&'a ArrayValue, ExecutionError> {
        self.resolve_array_value(reference, values)?
            .ok_or_else(|| ExecutionError::Plan("step did not produce an array value".into()))
    }

    fn resolve_netcdf_variable_ref(
        &self,
        reference: &PlanValueRef,
        values: &ValueStore,
    ) -> Result<NetcdfVariableRef, ExecutionError> {
        match self.resolve_value(reference, values)? {
            RuntimeValue::NetcdfVariables(variables) if variables.len() == 1 => {
                Ok(variables[0].clone())
            }
            RuntimeValue::VariableMetadata(described) => Ok(described.reference.clone()),
            _ => Err(ExecutionError::Plan(
                "step did not produce a netcdf variable reference".into(),
            )),
        }
    }

    fn render_scalar(
        &self,
        label: &str,
        input: &PlanValueRef,
        values: &ValueStore,
    ) -> Result<ScalarValue, ExecutionError> {
        match self.resolve_value(input, values)? {
            RuntimeValue::MeanReport(report) => Ok(ScalarValue {
                label: label.to_string(),
                value: report.mean,
            }),
            RuntimeValue::ScalarValue(value) => Ok(value.clone()),
            RuntimeValue::CompareReport(report) => Ok(ScalarValue {
                label: label.to_string(),
                value: report.difference,
            }),
            _ => Err(ExecutionError::Plan(
                "render.scalar requires a scalar-like input".into(),
            )),
        }
    }

    fn render_table(
        &self,
        title: &str,
        inputs: &[PlanValueRef],
        values: &ValueStore,
    ) -> Result<TableValue, ExecutionError> {
        if inputs.len() > 1 {
            let mut rows = Vec::with_capacity(inputs.len());
            for input in inputs {
                let value = self.resolve_value(input, values)?;
                match value {
                    RuntimeValue::ScalarValue(scalar) => rows.push(TableRow {
                        label: scalar.label.clone(),
                        value: format!("{:.6}", scalar.value),
                    }),
                    RuntimeValue::MeanReport(report) => rows.push(TableRow {
                        label: report
                            .variable
                            .clone()
                            .unwrap_or_else(|| "mean".to_string()),
                        value: format!("{:.6}", report.mean),
                    }),
                    RuntimeValue::ArrayValue(array) => {
                        rows.extend(array.values().iter().enumerate().map(|(index, value)| {
                            TableRow {
                                label: format!("value_{}", index + 1),
                                value: format!("{value:.6}"),
                            }
                        }));
                    }
                    _ => {
                        return Err(ExecutionError::Plan(
                            "render.table multi-input mode requires scalar-like or array inputs"
                                .into(),
                        ))
                    }
                }
            }

            return Ok(TableValue {
                title: title.to_string(),
                rows,
            });
        }

        let input = inputs.first().ok_or_else(|| {
            ExecutionError::Plan("render.table requires at least one input".into())
        })?;

        match self.resolve_value(input, values)? {
            RuntimeValue::NetcdfVariables(variables) => Ok(TableValue {
                title: title.to_string(),
                rows: variables
                    .iter()
                    .map(|variable| TableRow {
                        label: variable.name.clone(),
                        value: "netcdf variable".to_string(),
                    })
                    .collect(),
            }),
            RuntimeValue::ArrayValue(array) => Ok(TableValue {
                title: title.to_string(),
                rows: array
                    .values()
                    .iter()
                    .enumerate()
                    .map(|(index, value)| TableRow {
                        label: format!("value_{}", index + 1),
                        value: format!("{value:.6}"),
                    })
                    .collect(),
            }),
            RuntimeValue::CompareReport(report) => Ok(TableValue {
                title: title.to_string(),
                rows: vec![
                    TableRow {
                        label: "mean_a".to_string(),
                        value: format!("{:.6}", report.mean_a),
                    },
                    TableRow {
                        label: "mean_b".to_string(),
                        value: format!("{:.6}", report.mean_b),
                    },
                    TableRow {
                        label: "difference".to_string(),
                        value: format!("{:.6}", report.difference),
                    },
                ],
            }),
            RuntimeValue::InspectReport(report) => Ok(TableValue {
                title: title.to_string(),
                rows: vec![
                    TableRow {
                        label: "file".to_string(),
                        value: report.file.display().to_string(),
                    },
                    TableRow {
                        label: "kind".to_string(),
                        value: format!("{:?}", report.kind).to_lowercase(),
                    },
                ],
            }),
            RuntimeValue::ScalarValue(scalar) => Ok(TableValue {
                title: title.to_string(),
                rows: vec![TableRow {
                    label: scalar.label.clone(),
                    value: format!("{:.6}", scalar.value),
                }],
            }),
            _ => Err(ExecutionError::Plan(
                "render.table requires a table-like input".into(),
            )),
        }
    }

    fn resolve_value<'a>(
        &self,
        reference: &PlanValueRef,
        values: &'a ValueStore,
    ) -> Result<&'a RuntimeValue, ExecutionError> {
        match reference {
            PlanValueRef::Step { step } => values.get(step).ok_or_else(|| {
                ExecutionError::Plan(format!("step `{step}` is not available in the value store"))
            }),
            _ => Err(ExecutionError::Plan(
                "render steps currently require a step reference input".into(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        capability::{CapabilityId, CapabilityRegistry},
        executor::{PlanExecutor, RuntimeValue},
        plan::{CapabilityInput, ExecutionPlan, PlanStep},
        policy::ExecutionPolicy,
        session::SessionState,
    };

    #[test]
    fn dataset_resolve_plan_produces_dataset_ref() {
        let path = std::env::current_dir()
            .expect("cwd")
            .join("fixtures/netcdf/simple.nc");
        let registry = CapabilityRegistry::discover();
        let policy = ExecutionPolicy::from_registry(&registry);
        let executor = PlanExecutor::new(registry, policy);
        let plan = ExecutionPlan {
            goal: "resolve dataset".to_string(),
            steps: vec![PlanStep {
                id: "s1".to_string(),
                capability: CapabilityId::DatasetResolve,
                input: CapabilityInput::DatasetResolve {
                    alias: None,
                    path: Some(path.display().to_string()),
                },
            }],
        };

        let outcome = executor
            .execute(&plan, &mut SessionState::default())
            .expect("execute plan");

        assert!(matches!(
            outcome.final_value(),
            Some(RuntimeValue::DatasetRef(_))
        ));
    }
}
