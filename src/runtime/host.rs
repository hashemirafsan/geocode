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
        if !self.discovery.is_available(binary) {
            return Err(ExecutionError::Policy(format!(
                "known binary `{}` is not available on this machine",
                binary.command_name()
            )));
        }

        Command::new(binary.command_name())
            .args(args)
            .output()
            .map_err(|err| {
                ExecutionError::Command(format!(
                    "failed to execute {}: {err}",
                    binary.command_name()
                ))
            })
    }
}
