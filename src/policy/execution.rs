use serde::{Deserialize, Serialize};

use crate::{
    capability::CapabilityRegistry,
    engine::ExecutionError,
    plan::{CapabilityInput, ExecutionPlan},
    runtime::KnownBinary,
    session::SessionState,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPolicy {
    pub allow_local_filesystem: bool,
    pub allow_network: bool,
    pub allowed_binaries: Vec<KnownBinary>,
}

impl ExecutionPolicy {
    pub fn from_registry(registry: &CapabilityRegistry) -> Self {
        let allowed_binaries = registry
            .discovery
            .binaries
            .iter()
            .filter(|binary| binary.available)
            .map(|binary| binary.binary)
            .collect();

        Self {
            allow_local_filesystem: true,
            allow_network: false,
            allowed_binaries,
        }
    }

    pub fn validate_plan(
        &self,
        plan: &ExecutionPlan,
        registry: &CapabilityRegistry,
        _session: &SessionState,
    ) -> Result<(), ExecutionError> {
        for step in &plan.steps {
            if registry.descriptor(step.capability).is_none() {
                return Err(ExecutionError::Capability(format!(
                    "capability `{}` is not registered",
                    step.capability.as_str()
                )));
            }

            match &step.input {
                CapabilityInput::DatasetResolve { alias: _, path } => {
                    if let Some(path) = path {
                        validate_local_path(path)?;
                    }
                }
                CapabilityInput::ProcessRunKnown { binary, .. } => {
                    let binary = match binary.as_str() {
                        "gdalinfo" => KnownBinary::GdalInfo,
                        "ncdump" => KnownBinary::NcDump,
                        other => {
                            return Err(ExecutionError::Policy(format!(
                                "binary `{other}` is not allowed by policy"
                            )));
                        }
                    };

                    if !self.allowed_binaries.contains(&binary) {
                        return Err(ExecutionError::Policy(format!(
                            "binary `{}` is not available on this machine",
                            binary.command_name()
                        )));
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }
}

fn validate_local_path(path: &str) -> Result<(), ExecutionError> {
    if path.contains("://") {
        return Err(ExecutionError::Policy(
            "only local filesystem paths are allowed in the current runtime policy".into(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        capability::{CapabilityId, CapabilityRegistry},
        plan::{CapabilityInput, ExecutionPlan, PlanStep},
        session::SessionState,
    };

    use super::ExecutionPolicy;

    #[test]
    fn policy_rejects_remote_dataset_paths() {
        let registry = CapabilityRegistry::discover();
        let policy = ExecutionPolicy::from_registry(&registry);
        let plan = ExecutionPlan {
            goal: "reject remote path".to_string(),
            steps: vec![PlanStep {
                id: "s1".to_string(),
                capability: CapabilityId::DatasetResolve,
                input: CapabilityInput::DatasetResolve {
                    alias: None,
                    path: Some("https://example.com/file.nc".to_string()),
                },
            }],
        };

        assert!(
            policy
                .validate_plan(&plan, &registry, &SessionState::default())
                .is_err()
        );
    }
}
