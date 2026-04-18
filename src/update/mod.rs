use std::{
    env, fs,
    io::Cursor,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use flate2::read::GzDecoder;
use reqwest::blocking::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tar::Archive;
use zip::ZipArchive;

use crate::engine::ExecutionError;

const RELEASE_REPO: &str = "geocode-cli/geocode";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallSource {
    Standalone,
    Homebrew,
    Scoop,
    Winget,
    Unknown,
}

impl InstallSource {
    pub fn redirect_command(self) -> Option<&'static str> {
        match self {
            Self::Homebrew => Some("brew upgrade geocode"),
            Self::Scoop => Some("scoop update geocode"),
            Self::Winget => Some("winget upgrade GeoCode.GeoCode"),
            Self::Standalone | Self::Unknown => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Standalone => "standalone",
            Self::Homebrew => "homebrew",
            Self::Scoop => "scoop",
            Self::Winget => "winget",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize, Clone)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug)]
struct ExtractedLayout {
    binary_path: PathBuf,
    runtime_dir: PathBuf,
}

pub fn current_target_triple() -> &'static str {
    if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        "aarch64-apple-darwin"
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        "x86_64-apple-darwin"
    } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        "x86_64-pc-windows-msvc"
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        "x86_64-unknown-linux-gnu"
    } else {
        "unknown-target"
    }
}

pub fn detect_install_source() -> Result<InstallSource, ExecutionError> {
    let current_exe = env::current_exe().map_err(|err| ExecutionError::Io(err.to_string()))?;
    Ok(detect_install_source_from_exe(
        &current_exe,
        env::var("GEOCODE_INSTALL_SOURCE").ok().as_deref(),
    ))
}

pub fn run_self_update() -> Result<SelfUpdateResult, ExecutionError> {
    let install_source = detect_install_source()?;
    if let Some(command) = install_source.redirect_command() {
        return Ok(SelfUpdateResult::Redirect {
            install_source,
            command: command.to_string(),
        });
    }

    if !matches!(install_source, InstallSource::Standalone) {
        return Err(ExecutionError::Command(
            "self-update supports only standalone GitHub release installs with packaged runtime layout"
                .into(),
        ));
    }

    let current_exe = env::current_exe().map_err(|err| ExecutionError::Io(err.to_string()))?;
    let client = github_client()?;
    let release = fetch_latest_release(&client)?;
    let current_version = env!("CARGO_PKG_VERSION");
    let latest_version = release.tag_name.trim_start_matches('v');
    if latest_version == current_version {
        return Ok(SelfUpdateResult::UpToDate {
            version: current_version.to_string(),
        });
    }

    let asset = select_release_asset(&release, current_target_triple())?;
    let checksum_asset = select_checksum_asset(&release)?;
    let archive_bytes = download_bytes(&client, &asset.browser_download_url)?;
    verify_checksum(
        &client,
        &checksum_asset.browser_download_url,
        &asset.name,
        &archive_bytes,
    )?;

    let stage_dir = create_stage_dir()?;
    let layout = extract_release_archive(&stage_dir, &asset.name, &archive_bytes)?;
    let result = apply_update(&current_exe, &layout)?;

    Ok(SelfUpdateResult::Updated {
        from_version: current_version.to_string(),
        to_version: latest_version.to_string(),
        target: current_target_triple().to_string(),
        mode: result,
    })
}

#[derive(Debug)]
pub enum ApplyMode {
    Replaced,
    StagedWindows,
}

#[derive(Debug)]
pub enum SelfUpdateResult {
    Redirect {
        install_source: InstallSource,
        command: String,
    },
    UpToDate {
        version: String,
    },
    Updated {
        from_version: String,
        to_version: String,
        target: String,
        mode: ApplyMode,
    },
}

fn github_client() -> Result<Client, ExecutionError> {
    Client::builder()
        .user_agent(format!("geocode/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|err| ExecutionError::Command(format!("failed to create update client: {err}")))
}

fn fetch_latest_release(client: &Client) -> Result<GitHubRelease, ExecutionError> {
    let url = format!("https://api.github.com/repos/{RELEASE_REPO}/releases/latest");
    client
        .get(url)
        .send()
        .and_then(|response| response.error_for_status())
        .map_err(|err| ExecutionError::Command(format!("failed to query latest release: {err}")))?
        .json()
        .map_err(|err| ExecutionError::Parse(format!("invalid release response: {err}")))
}

fn select_release_asset(
    release: &GitHubRelease,
    target: &str,
) -> Result<GitHubAsset, ExecutionError> {
    let expected = format!(
        "geocode-{}-{}.{}",
        release.tag_name,
        target,
        archive_ext_for_target(target)
    );
    release
        .assets
        .iter()
        .find(|asset| asset.name == expected)
        .cloned()
        .ok_or_else(|| ExecutionError::Command(format!("release asset not found: {expected}")))
}

fn select_checksum_asset(release: &GitHubRelease) -> Result<GitHubAsset, ExecutionError> {
    let expected = format!("geocode-{}-checksums.txt", release.tag_name);
    release
        .assets
        .iter()
        .find(|asset| asset.name == expected)
        .cloned()
        .ok_or_else(|| ExecutionError::Command(format!("checksum asset not found: {expected}")))
}

fn download_bytes(client: &Client, url: &str) -> Result<Vec<u8>, ExecutionError> {
    client
        .get(url)
        .send()
        .and_then(|response| response.error_for_status())
        .map_err(|err| ExecutionError::Command(format!("failed to download update asset: {err}")))?
        .bytes()
        .map(|bytes| bytes.to_vec())
        .map_err(|err| ExecutionError::Command(format!("failed to read update asset: {err}")))
}

fn verify_checksum(
    client: &Client,
    checksum_url: &str,
    asset_name: &str,
    archive_bytes: &[u8],
) -> Result<(), ExecutionError> {
    let checksums = client
        .get(checksum_url)
        .send()
        .and_then(|response| response.error_for_status())
        .map_err(|err| ExecutionError::Command(format!("failed to download checksums: {err}")))?
        .text()
        .map_err(|err| ExecutionError::Command(format!("failed to read checksums: {err}")))?;

    let expected = checksums
        .lines()
        .find_map(|line| parse_checksum_line(line, asset_name))
        .ok_or_else(|| {
            ExecutionError::Command(format!("checksum missing for asset: {asset_name}"))
        })?;

    let actual = format!("{:x}", Sha256::digest(archive_bytes));
    if actual != expected {
        return Err(ExecutionError::Command(format!(
            "checksum mismatch for {asset_name}"
        )));
    }

    Ok(())
}

fn parse_checksum_line(line: &str, asset_name: &str) -> Option<String> {
    let mut parts = line.split_whitespace();
    let checksum = parts.next()?;
    let filename = parts.next()?;
    (filename == asset_name).then(|| checksum.to_string())
}

fn create_stage_dir() -> Result<PathBuf, ExecutionError> {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let path = env::temp_dir().join(format!("geocode-self-update-{nonce}"));
    fs::create_dir_all(&path).map_err(|err| ExecutionError::Io(err.to_string()))?;
    Ok(path)
}

fn extract_release_archive(
    stage_dir: &Path,
    asset_name: &str,
    archive_bytes: &[u8],
) -> Result<ExtractedLayout, ExecutionError> {
    if asset_name.ends_with(".tar.gz") {
        let decoder = GzDecoder::new(Cursor::new(archive_bytes));
        let mut archive = Archive::new(decoder);
        archive.unpack(stage_dir).map_err(|err| {
            ExecutionError::Command(format!("failed to unpack update archive: {err}"))
        })?;
    } else if asset_name.ends_with(".zip") {
        let mut archive = ZipArchive::new(Cursor::new(archive_bytes))
            .map_err(|err| ExecutionError::Command(format!("failed to read update zip: {err}")))?;
        for index in 0..archive.len() {
            let mut file = archive.by_index(index).map_err(|err| {
                ExecutionError::Command(format!("failed to read zip entry: {err}"))
            })?;
            let outpath = stage_dir.join(file.mangled_name());
            if file.is_dir() {
                fs::create_dir_all(&outpath).map_err(|err| ExecutionError::Io(err.to_string()))?;
                continue;
            }
            if let Some(parent) = outpath.parent() {
                fs::create_dir_all(parent).map_err(|err| ExecutionError::Io(err.to_string()))?;
            }
            let mut outfile =
                fs::File::create(&outpath).map_err(|err| ExecutionError::Io(err.to_string()))?;
            std::io::copy(&mut file, &mut outfile)
                .map_err(|err| ExecutionError::Io(err.to_string()))?;
        }
    } else {
        return Err(ExecutionError::Command(format!(
            "unsupported update archive: {asset_name}"
        )));
    }

    let root_dir = fs::read_dir(stage_dir)
        .map_err(|err| ExecutionError::Io(err.to_string()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.is_dir())
        .ok_or_else(|| ExecutionError::Command("update archive missing root directory".into()))?;

    let binary_name = if cfg!(target_os = "windows") {
        "geocode.exe"
    } else {
        "geocode"
    };
    let binary_path = root_dir.join(binary_name);
    let runtime_dir = root_dir.join("runtime");
    if !binary_path.is_file() {
        return Err(ExecutionError::Command(
            "update archive missing geocode binary".into(),
        ));
    }
    if !runtime_dir.is_dir() {
        return Err(ExecutionError::Command(
            "update archive missing runtime directory".into(),
        ));
    }

    Ok(ExtractedLayout {
        binary_path,
        runtime_dir,
    })
}

fn apply_update(current_exe: &Path, layout: &ExtractedLayout) -> Result<ApplyMode, ExecutionError> {
    if cfg!(target_os = "windows") {
        apply_windows_update(current_exe, layout)
    } else {
        apply_unix_update(current_exe, layout)
    }
}

fn apply_unix_update(
    current_exe: &Path,
    layout: &ExtractedLayout,
) -> Result<ApplyMode, ExecutionError> {
    let install_dir = current_exe
        .parent()
        .ok_or_else(|| ExecutionError::Io("current executable has no parent directory".into()))?;
    let staged_binary = install_dir.join("geocode.new");
    let runtime_dir = install_dir.join("runtime");
    let staged_runtime = install_dir.join("runtime.new");
    let backup_binary = install_dir.join("geocode.old");
    let backup_runtime = install_dir.join("runtime.old");

    if staged_binary.exists() {
        fs::remove_file(&staged_binary).map_err(|err| ExecutionError::Io(err.to_string()))?;
    }
    if staged_runtime.exists() {
        fs::remove_dir_all(&staged_runtime).map_err(|err| ExecutionError::Io(err.to_string()))?;
    }

    copy_file_with_permissions(&layout.binary_path, &staged_binary)?;
    copy_dir_all(&layout.runtime_dir, &staged_runtime)?;

    if backup_binary.exists() {
        fs::remove_file(&backup_binary).map_err(|err| ExecutionError::Io(err.to_string()))?;
    }
    fs::rename(current_exe, &backup_binary).map_err(|err| ExecutionError::Io(err.to_string()))?;
    if let Err(err) = fs::rename(&staged_binary, current_exe) {
        let _ = fs::rename(&backup_binary, current_exe);
        return Err(ExecutionError::Io(err.to_string()));
    }
    let _ = fs::remove_file(&backup_binary);

    if backup_runtime.exists() {
        fs::remove_dir_all(&backup_runtime).map_err(|err| ExecutionError::Io(err.to_string()))?;
    }
    if runtime_dir.exists() {
        fs::rename(&runtime_dir, &backup_runtime)
            .map_err(|err| ExecutionError::Io(err.to_string()))?;
    }
    if let Err(err) = fs::rename(&staged_runtime, &runtime_dir) {
        let _ = fs::rename(&backup_runtime, &runtime_dir);
        return Err(ExecutionError::Io(err.to_string()));
    }
    if backup_runtime.exists() {
        let _ = fs::remove_dir_all(&backup_runtime);
    }

    Ok(ApplyMode::Replaced)
}

fn apply_windows_update(
    current_exe: &Path,
    layout: &ExtractedLayout,
) -> Result<ApplyMode, ExecutionError> {
    let install_dir = current_exe
        .parent()
        .ok_or_else(|| ExecutionError::Io("current executable has no parent directory".into()))?;
    let staged_binary = install_dir.join("geocode.new.exe");
    let staged_runtime = install_dir.join("runtime.new");
    if staged_binary.exists() {
        fs::remove_file(&staged_binary).map_err(|err| ExecutionError::Io(err.to_string()))?;
    }
    if staged_runtime.exists() {
        fs::remove_dir_all(&staged_runtime).map_err(|err| ExecutionError::Io(err.to_string()))?;
    }
    copy_file_with_permissions(&layout.binary_path, &staged_binary)?;
    copy_dir_all(&layout.runtime_dir, &staged_runtime)?;

    let script_path =
        env::temp_dir().join(format!("geocode-self-update-{}.cmd", std::process::id()));
    fs::write(
        &script_path,
        windows_update_script(
            std::process::id(),
            current_exe,
            &staged_binary,
            &install_dir.join("runtime"),
            &staged_runtime,
        ),
    )
    .map_err(|err| ExecutionError::Io(err.to_string()))?;

    Command::new("cmd")
        .args(["/C", script_path.to_string_lossy().as_ref()])
        .spawn()
        .map_err(|err| {
            ExecutionError::Command(format!("failed to stage windows updater: {err}"))
        })?;

    Ok(ApplyMode::StagedWindows)
}

fn windows_update_script(
    pid: u32,
    current_exe: &Path,
    staged_binary: &Path,
    runtime_dir: &Path,
    staged_runtime: &Path,
) -> String {
    format!(
        "@echo off\r\n:wait\r\ntasklist /FI \"PID eq {pid}\" 2>NUL | find \"{pid}\" >NUL\r\nif not errorlevel 1 (timeout /t 1 /nobreak >NUL\r\ngoto wait)\r\nif exist \"{exe}.old\" del /f /q \"{exe}.old\"\r\nmove /Y \"{exe}\" \"{exe}.old\" >NUL\r\nmove /Y \"{new_exe}\" \"{exe}\" >NUL\r\nif exist \"{runtime}.old\" rmdir /s /q \"{runtime}.old\"\r\nif exist \"{runtime}\" move /Y \"{runtime}\" \"{runtime}.old\" >NUL\r\nmove /Y \"{new_runtime}\" \"{runtime}\" >NUL\r\ndel /f /q \"{exe}.old\"\r\nif exist \"{runtime}.old\" rmdir /s /q \"{runtime}.old\"\r\ndel /f /q \"%~f0\"\r\n",
        exe = current_exe.display(),
        new_exe = staged_binary.display(),
        runtime = runtime_dir.display(),
        new_runtime = staged_runtime.display(),
    )
}

fn copy_file_with_permissions(src: &Path, dst: &Path) -> Result<(), ExecutionError> {
    fs::copy(src, dst).map_err(|err| ExecutionError::Io(err.to_string()))?;
    let permissions = fs::metadata(src)
        .map_err(|err| ExecutionError::Io(err.to_string()))?
        .permissions();
    fs::set_permissions(dst, permissions).map_err(|err| ExecutionError::Io(err.to_string()))
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), ExecutionError> {
    fs::create_dir_all(dst).map_err(|err| ExecutionError::Io(err.to_string()))?;
    for entry in fs::read_dir(src).map_err(|err| ExecutionError::Io(err.to_string()))? {
        let entry = entry.map_err(|err| ExecutionError::Io(err.to_string()))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            copy_file_with_permissions(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

fn archive_ext_for_target(target: &str) -> &'static str {
    if target.contains("windows") {
        "zip"
    } else {
        "tar.gz"
    }
}

fn detect_install_source_from_exe(
    current_exe: &Path,
    override_source: Option<&str>,
) -> InstallSource {
    if let Some(source) = override_source {
        return match source {
            "standalone" => InstallSource::Standalone,
            "homebrew" => InstallSource::Homebrew,
            "scoop" => InstallSource::Scoop,
            "winget" => InstallSource::Winget,
            _ => InstallSource::Unknown,
        };
    }

    let path = current_exe.to_string_lossy().to_ascii_lowercase();
    if path.contains("/cellar/geocode/") || path.contains("/homebrew/cellar/geocode/") {
        return InstallSource::Homebrew;
    }
    if path.contains("\\scoop\\apps\\geocode\\") || path.contains("/scoop/apps/geocode/") {
        return InstallSource::Scoop;
    }
    if path.contains("geocode.geocode_")
        && (path.contains("windowsapps") || path.contains("winget") || path.contains("packages"))
    {
        return InstallSource::Winget;
    }
    if current_exe
        .parent()
        .map(|parent| parent.join("runtime").is_dir())
        .unwrap_or(false)
    {
        return InstallSource::Standalone;
    }
    InstallSource::Unknown
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use tempfile::TempDir;

    use super::{
        archive_ext_for_target, detect_install_source_from_exe, parse_checksum_line,
        select_release_asset, GitHubAsset, GitHubRelease, InstallSource,
    };

    #[test]
    fn install_source_detects_package_managers_and_standalone() {
        assert_eq!(
            detect_install_source_from_exe(
                Path::new("/opt/homebrew/Cellar/geocode/0.1.0/bin/geocode"),
                None,
            ),
            InstallSource::Homebrew
        );
        assert_eq!(
            detect_install_source_from_exe(
                Path::new(r"C:\Users\alice\scoop\apps\geocode\current\geocode.exe"),
                None,
            ),
            InstallSource::Scoop
        );
        assert_eq!(
            detect_install_source_from_exe(
                Path::new(
                    r"C:\Program Files\WindowsApps\Microsoft.Winget\Packages\GeoCode.GeoCode_1.0.0\geocode.exe"
                ),
                None,
            ),
            InstallSource::Winget
        );

        let temp_dir = TempDir::new().expect("temp dir");
        fs::create_dir_all(temp_dir.path().join("runtime")).expect("runtime dir");
        assert_eq!(
            detect_install_source_from_exe(&temp_dir.path().join("geocode"), None),
            InstallSource::Standalone
        );
    }

    #[test]
    fn override_install_source_wins() {
        assert_eq!(
            detect_install_source_from_exe(Path::new("/tmp/geocode"), Some("scoop")),
            InstallSource::Scoop
        );
    }

    #[test]
    fn selects_matching_release_asset() {
        let release = GitHubRelease {
            tag_name: "v0.2.0".to_string(),
            assets: vec![
                GitHubAsset {
                    name: "geocode-v0.2.0-x86_64-unknown-linux-gnu.tar.gz".to_string(),
                    browser_download_url: "https://example.test/linux".to_string(),
                },
                GitHubAsset {
                    name: "geocode-v0.2.0-checksums.txt".to_string(),
                    browser_download_url: "https://example.test/checksums".to_string(),
                },
            ],
        };

        let asset = select_release_asset(&release, "x86_64-unknown-linux-gnu").expect("asset");
        assert_eq!(asset.name, "geocode-v0.2.0-x86_64-unknown-linux-gnu.tar.gz");
    }

    #[test]
    fn parses_checksum_lines() {
        assert_eq!(
            parse_checksum_line(
                "abc123 geocode-v0.2.0-x86_64-unknown-linux-gnu.tar.gz",
                "geocode-v0.2.0-x86_64-unknown-linux-gnu.tar.gz",
            )
            .as_deref(),
            Some("abc123")
        );
    }

    #[test]
    fn target_archive_extensions_match_platform() {
        assert_eq!(archive_ext_for_target("x86_64-pc-windows-msvc"), "zip");
        assert_eq!(archive_ext_for_target("x86_64-unknown-linux-gnu"), "tar.gz");
    }
}
