use std::{
    env, fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnownBinary {
    GdalInfo,
    NcDump,
    NcGen,
}

impl KnownBinary {
    pub fn command_name(self) -> &'static str {
        match self {
            Self::GdalInfo => "gdalinfo",
            Self::NcDump => "ncdump",
            Self::NcGen => "ncgen",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryStatus {
    pub binary: KnownBinary,
    pub command: String,
    pub available: bool,
    pub path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostDiscovery {
    pub binaries: Vec<BinaryStatus>,
    pub local_filesystem: bool,
    pub network_access: bool,
}

impl HostDiscovery {
    pub fn discover() -> Self {
        let binaries = [
            KnownBinary::GdalInfo,
            KnownBinary::NcDump,
            KnownBinary::NcGen,
        ]
        .into_iter()
        .map(BinaryStatus::discover)
        .collect();

        Self {
            binaries,
            local_filesystem: true,
            network_access: false,
        }
    }

    pub fn binary(&self, binary: KnownBinary) -> Option<&BinaryStatus> {
        self.binaries.iter().find(|status| status.binary == binary)
    }

    pub fn is_available(&self, binary: KnownBinary) -> bool {
        self.binary(binary).is_some_and(|status| status.available)
    }
}

impl BinaryStatus {
    fn discover(binary: KnownBinary) -> Self {
        let command = binary.command_name();
        let path = locate_in_path(command);

        Self {
            binary,
            command: command.to_string(),
            available: path.is_some(),
            path,
        }
    }
}

fn locate_in_path(command: &str) -> Option<PathBuf> {
    let command_path = Path::new(command);
    if command_path.is_absolute() && is_executable(command_path) {
        return Some(command_path.to_path_buf());
    }

    let path_var = env::var_os("PATH")?;
    env::split_paths(&path_var)
        .map(|entry| entry.join(command))
        .find(|candidate| is_executable(candidate))
}

fn is_executable(path: &Path) -> bool {
    fs::metadata(path)
        .map(|metadata| metadata.is_file())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::KnownBinary;

    #[test]
    fn known_binary_command_names_are_stable() {
        assert_eq!(KnownBinary::GdalInfo.command_name(), "gdalinfo");
        assert_eq!(KnownBinary::NcDump.command_name(), "ncdump");
        assert_eq!(KnownBinary::NcGen.command_name(), "ncgen");
    }
}
