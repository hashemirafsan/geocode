use std::process::{Command, Output};

use crate::{
    engine::ExecutionError,
    runtime::{HostDiscovery, KnownBinary},
};

#[derive(Debug, Clone)]
pub struct HostRuntime {
    discovery: HostDiscovery,
}

impl HostRuntime {
    pub fn discover() -> Self {
        Self {
            discovery: HostDiscovery::discover(),
        }
    }

    pub fn discovery(&self) -> &HostDiscovery {
        &self.discovery
    }

    pub fn run_known(&self, binary: KnownBinary, args: &[&str]) -> Result<Output, ExecutionError> {
        let status = self.discovery.binary(binary).ok_or_else(|| {
            ExecutionError::Policy(format!(
                "known binary `{}` is not available on this machine",
                binary.command_name()
            ))
        })?;

        let path = status.path.as_ref().ok_or_else(|| {
            ExecutionError::Policy(format!(
                "known binary `{}` is not available on this machine",
                binary.command_name()
            ))
        })?;

        Command::new(path).args(args).output().map_err(|err| {
            ExecutionError::Command(format!(
                "failed to execute {}: {err}",
                binary.command_name()
            ))
        })
    }
}
