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
        let path = locate_known_binary(
            command,
            env::current_exe().ok(),
            env::var_os("PATH"),
            env::var_os("PATHEXT"),
            cfg!(target_os = "windows"),
        );

        Self {
            binary,
            command: command.to_string(),
            available: path.is_some(),
            path,
        }
    }
}

fn locate_in_path(command: &str) -> Option<PathBuf> {
    locate_known_binary(
        command,
        env::current_exe().ok(),
        env::var_os("PATH"),
        env::var_os("PATHEXT"),
        cfg!(target_os = "windows"),
    )
}

fn locate_known_binary(
    command: &str,
    current_exe: Option<PathBuf>,
    path_var: Option<std::ffi::OsString>,
    pathext_var: Option<std::ffi::OsString>,
    windows: bool,
) -> Option<PathBuf> {
    let suffixes = executable_suffixes(command, pathext_var.clone(), windows);

    bundled_candidates(current_exe.as_deref(), command, &suffixes)
        .into_iter()
        .find(|candidate| is_executable(candidate))
        .or_else(|| locate_in_path_for_os(command, path_var, pathext_var, windows))
}

fn bundled_candidates(
    current_exe: Option<&Path>,
    command: &str,
    suffixes: &[String],
) -> Vec<PathBuf> {
    let Some(exe_dir) = current_exe.and_then(|path| path.parent()) else {
        return Vec::new();
    };

    let mut candidates = Vec::new();
    for base in [
        exe_dir.to_path_buf(),
        exe_dir.join("runtime"),
        exe_dir.join("bin"),
    ] {
        for suffix in suffixes {
            let candidate = base.join(format!("{command}{suffix}"));
            if !candidates.iter().any(|existing| existing == &candidate) {
                candidates.push(candidate);
            }
        }
    }

    candidates
}

fn locate_in_path_for_os(
    command: &str,
    path_var: Option<std::ffi::OsString>,
    pathext_var: Option<std::ffi::OsString>,
    windows: bool,
) -> Option<PathBuf> {
    let command_path = Path::new(command);
    if (command_path.is_absolute() || command.contains(std::path::MAIN_SEPARATOR))
        && is_executable(command_path)
    {
        return Some(command_path.to_path_buf());
    }

    let path_var = path_var?;
    let suffixes = executable_suffixes(command, pathext_var, windows);

    env::split_paths(&path_var)
        .flat_map(|entry| {
            suffixes
                .iter()
                .map(move |suffix| entry.join(format!("{command}{suffix}")))
        })
        .find(|candidate| is_executable(candidate))
}

fn executable_suffixes(
    command: &str,
    pathext_var: Option<std::ffi::OsString>,
    windows: bool,
) -> Vec<String> {
    if !windows {
        return vec![String::new()];
    }

    let has_extension = Path::new(command).extension().is_some();
    if has_extension {
        return vec![String::new()];
    }

    let mut suffixes = vec![String::new()];
    let pathext = pathext_var
        .and_then(|value| value.into_string().ok())
        .unwrap_or_else(|| ".COM;.EXE;.BAT;.CMD".to_string());

    for suffix in pathext.split(';').filter(|suffix| !suffix.is_empty()) {
        let normalized = if suffix.starts_with('.') {
            suffix.to_ascii_lowercase()
        } else {
            format!(".{}", suffix.to_ascii_lowercase())
        };

        if !suffixes
            .iter()
            .any(|existing| existing.eq_ignore_ascii_case(&normalized))
        {
            suffixes.push(normalized);
        }
    }

    suffixes
}

fn is_executable(path: &Path) -> bool {
    fs::metadata(path)
        .map(|metadata| metadata.is_file())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use std::{ffi::OsString, fs, path::Path};

    use tempfile::TempDir;

    use super::{
        bundled_candidates, executable_suffixes, locate_in_path_for_os, locate_known_binary,
        KnownBinary,
    };

    #[test]
    fn known_binary_command_names_are_stable() {
        assert_eq!(KnownBinary::GdalInfo.command_name(), "gdalinfo");
        assert_eq!(KnownBinary::NcDump.command_name(), "ncdump");
        assert_eq!(KnownBinary::NcGen.command_name(), "ncgen");
    }

    #[test]
    fn non_windows_suffixes_only_try_bare_command() {
        assert_eq!(
            executable_suffixes("gdalinfo", None, false),
            vec![String::new()]
        );
    }

    #[test]
    fn windows_suffixes_include_pathext_variants() {
        assert_eq!(
            executable_suffixes("gdalinfo", Some(OsString::from(".EXE;.BAT")), true),
            vec!["".to_string(), ".exe".to_string(), ".bat".to_string()]
        );
    }

    #[test]
    fn windows_suffixes_skip_extra_lookup_for_explicit_extension() {
        assert_eq!(
            executable_suffixes("gdalinfo.exe", Some(OsString::from(".EXE;.BAT")), true),
            vec![String::new()]
        );
    }

    #[test]
    fn locate_in_path_for_os_finds_windows_exe_via_pathext() {
        let temp_dir = TempDir::new().expect("temp dir");
        let exe = temp_dir.path().join("gdalinfo.exe");
        fs::write(&exe, b"binary").expect("write fake exe");
        let path_var = env_path(temp_dir.path());

        let found = locate_in_path_for_os(
            "gdalinfo",
            Some(path_var),
            Some(OsString::from(".EXE;.BAT")),
            true,
        )
        .expect("find exe");

        assert_eq!(found, exe);
    }

    #[test]
    fn locate_in_path_for_os_honors_relative_command_with_extension() {
        let temp_dir = TempDir::new().expect("temp dir");
        let exe = temp_dir.path().join("ncgen.exe");
        fs::write(&exe, b"binary").expect("write fake exe");

        let found = locate_in_path_for_os(
            exe.to_string_lossy().as_ref(),
            None,
            Some(OsString::from(".EXE")),
            true,
        )
        .expect("find explicit path");

        assert_eq!(found, exe);
    }

    #[test]
    fn bundled_candidates_prefer_runtime_folder_near_executable() {
        let temp_dir = TempDir::new().expect("temp dir");
        let current_exe = temp_dir.path().join("geocode");
        let runtime_exe = temp_dir.path().join("runtime").join("gdalinfo");
        fs::create_dir_all(runtime_exe.parent().expect("runtime parent")).expect("mkdir");
        fs::write(&runtime_exe, b"binary").expect("write helper");

        let found = locate_known_binary("gdalinfo", Some(current_exe), None, None, false)
            .expect("find bundled helper");

        assert_eq!(found, runtime_exe);
    }

    #[test]
    fn bundled_candidates_beat_path_match() {
        let temp_dir = TempDir::new().expect("temp dir");
        let current_exe = temp_dir.path().join("geocode");
        let runtime_exe = temp_dir.path().join("runtime").join("ncdump");
        let path_dir = temp_dir.path().join("path-bin");
        let path_exe = path_dir.join("ncdump");
        fs::create_dir_all(runtime_exe.parent().expect("runtime parent")).expect("mkdir runtime");
        fs::create_dir_all(&path_dir).expect("mkdir path dir");
        fs::write(&runtime_exe, b"bundled").expect("write bundled helper");
        fs::write(&path_exe, b"path helper").expect("write path helper");

        let found = locate_known_binary(
            "ncdump",
            Some(current_exe),
            Some(env_path(&path_dir)),
            None,
            false,
        )
        .expect("find preferred helper");

        assert_eq!(found, runtime_exe);
    }

    #[test]
    fn bundled_candidates_include_exe_dir_runtime_and_bin() {
        let exe = Path::new("/tmp/geocode/geocode");
        let candidates = bundled_candidates(Some(exe), "gdalinfo", &[String::new()]);

        assert!(candidates.contains(&Path::new("/tmp/geocode/gdalinfo").to_path_buf()));
        assert!(candidates.contains(&Path::new("/tmp/geocode/runtime/gdalinfo").to_path_buf()));
        assert!(candidates.contains(&Path::new("/tmp/geocode/bin/gdalinfo").to_path_buf()));
    }

    fn env_path(path: &Path) -> OsString {
        OsString::from(path.as_os_str())
    }
}
