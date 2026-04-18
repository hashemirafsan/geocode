use std::{
    env,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeocodePaths {
    pub config_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub state_dir: PathBuf,
}

impl GeocodePaths {
    pub fn detect() -> Self {
        Self::for_platform(current_platform(), &EnvPaths::detect())
    }

    fn for_platform(platform: Platform, env_paths: &EnvPaths) -> Self {
        match platform {
            Platform::MacOs => {
                let home = env_paths.home_dir();
                let app_support = home
                    .join("Library")
                    .join("Application Support")
                    .join("geocode");
                Self {
                    config_dir: app_support.clone(),
                    cache_dir: home.join("Library").join("Caches").join("geocode"),
                    state_dir: app_support,
                }
            }
            Platform::Linux => {
                let home = env_paths.home_dir();
                Self {
                    config_dir: env_paths
                        .xdg_config_home
                        .clone()
                        .unwrap_or_else(|| home.join(".config"))
                        .join("geocode"),
                    cache_dir: env_paths
                        .xdg_cache_home
                        .clone()
                        .unwrap_or_else(|| home.join(".cache"))
                        .join("geocode"),
                    state_dir: env_paths
                        .xdg_state_home
                        .clone()
                        .unwrap_or_else(|| home.join(".local").join("state"))
                        .join("geocode"),
                }
            }
            Platform::Windows => {
                let home = env_paths.home_dir();
                let roaming = env_paths
                    .appdata
                    .clone()
                    .unwrap_or_else(|| home.join("AppData").join("Roaming"));
                let local = env_paths
                    .localappdata
                    .clone()
                    .unwrap_or_else(|| home.join("AppData").join("Local"));

                Self {
                    config_dir: roaming.join("geocode"),
                    cache_dir: local.join("geocode").join("cache"),
                    state_dir: local.join("geocode").join("state"),
                }
            }
        }
    }
}

pub fn home_dir() -> PathBuf {
    EnvPaths::detect().home_dir()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Platform {
    MacOs,
    Linux,
    Windows,
}

fn current_platform() -> Platform {
    if cfg!(target_os = "macos") {
        Platform::MacOs
    } else if cfg!(target_os = "windows") {
        Platform::Windows
    } else {
        Platform::Linux
    }
}

#[derive(Debug, Clone, Default)]
struct EnvPaths {
    home: Option<PathBuf>,
    userprofile: Option<PathBuf>,
    homedrive: Option<PathBuf>,
    homepath: Option<PathBuf>,
    xdg_config_home: Option<PathBuf>,
    xdg_cache_home: Option<PathBuf>,
    xdg_state_home: Option<PathBuf>,
    appdata: Option<PathBuf>,
    localappdata: Option<PathBuf>,
}

impl EnvPaths {
    fn detect() -> Self {
        Self {
            home: var_path("HOME"),
            userprofile: var_path("USERPROFILE"),
            homedrive: var_path("HOMEDRIVE"),
            homepath: var_path("HOMEPATH"),
            xdg_config_home: var_path("XDG_CONFIG_HOME"),
            xdg_cache_home: var_path("XDG_CACHE_HOME"),
            xdg_state_home: var_path("XDG_STATE_HOME"),
            appdata: var_path("APPDATA"),
            localappdata: var_path("LOCALAPPDATA"),
        }
    }

    fn home_dir(&self) -> PathBuf {
        self.home
            .clone()
            .or_else(|| self.userprofile.clone())
            .or_else(|| match (&self.homedrive, &self.homepath) {
                (Some(drive), Some(path)) => Some(join_drive_path(drive, path)),
                _ => None,
            })
            .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }
}

fn var_path(name: &str) -> Option<PathBuf> {
    env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn join_drive_path(drive: &Path, path: &Path) -> PathBuf {
    let path = path.strip_prefix("\\").unwrap_or(path);
    drive.join(path)
}

#[cfg(test)]
mod tests {
    use super::{EnvPaths, GeocodePaths, Platform};
    use std::path::PathBuf;

    #[test]
    fn linux_paths_follow_xdg_defaults() {
        let paths = GeocodePaths::for_platform(
            Platform::Linux,
            &EnvPaths {
                home: Some(PathBuf::from("/home/alice")),
                ..EnvPaths::default()
            },
        );

        assert_eq!(
            paths.config_dir,
            PathBuf::from("/home/alice/.config/geocode")
        );
        assert_eq!(paths.cache_dir, PathBuf::from("/home/alice/.cache/geocode"));
        assert_eq!(
            paths.state_dir,
            PathBuf::from("/home/alice/.local/state/geocode")
        );
    }

    #[test]
    fn linux_paths_honor_xdg_overrides() {
        let paths = GeocodePaths::for_platform(
            Platform::Linux,
            &EnvPaths {
                home: Some(PathBuf::from("/home/alice")),
                xdg_config_home: Some(PathBuf::from("/tmp/config")),
                xdg_cache_home: Some(PathBuf::from("/tmp/cache")),
                xdg_state_home: Some(PathBuf::from("/tmp/state")),
                ..EnvPaths::default()
            },
        );

        assert_eq!(paths.config_dir, PathBuf::from("/tmp/config/geocode"));
        assert_eq!(paths.cache_dir, PathBuf::from("/tmp/cache/geocode"));
        assert_eq!(paths.state_dir, PathBuf::from("/tmp/state/geocode"));
    }

    #[test]
    fn macos_paths_use_library_dirs() {
        let paths = GeocodePaths::for_platform(
            Platform::MacOs,
            &EnvPaths {
                home: Some(PathBuf::from("/Users/alice")),
                ..EnvPaths::default()
            },
        );

        assert_eq!(
            paths.config_dir,
            PathBuf::from("/Users/alice/Library/Application Support/geocode")
        );
        assert_eq!(paths.state_dir, paths.config_dir);
        assert_eq!(
            paths.cache_dir,
            PathBuf::from("/Users/alice/Library/Caches/geocode")
        );
    }

    #[test]
    fn windows_paths_use_appdata_dirs() {
        let roaming = PathBuf::from(r"C:\Users\alice\AppData\Roaming");
        let local = PathBuf::from(r"C:\Users\alice\AppData\Local");
        let paths = GeocodePaths::for_platform(
            Platform::Windows,
            &EnvPaths {
                userprofile: Some(PathBuf::from(r"C:\Users\alice")),
                appdata: Some(roaming.clone()),
                localappdata: Some(local.clone()),
                ..EnvPaths::default()
            },
        );

        assert_eq!(paths.config_dir, roaming.join("geocode"));
        assert_eq!(paths.cache_dir, local.join("geocode").join("cache"));
        assert_eq!(paths.state_dir, local.join("geocode").join("state"));
    }
}
