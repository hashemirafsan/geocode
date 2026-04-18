# GeoCode

GeoCode is local-first geospatial analysis CLI and TUI for NetCDF and GeoTIFF datasets.

## Public Status

Current public contract:

- CLI is supported compatibility contract
- TUI ships, but is secondary contract
- canonical releases come from GitHub Releases
- official package-manager channels are Homebrew, Scoop, and winget
- `geocode self-update` supports standalone GitHub release installs only

Locked policy/docs:

- `PUBLIC_RELEASE_POLICY.md`
- `PUBLIC_CLI_CONTRACT.md`
- `NATIVE_DEPENDENCY_STRATEGY.md`
- `HOMEBREW_DISTRIBUTION.md`
- `SCOOP_DISTRIBUTION.md`
- `WINGET_DISTRIBUTION.md`
- `SELF_UPDATE.md`
- `DIAGNOSTICS.md`
- `RELEASE_VALIDATION.md`
- `PUBLIC_LAUNCH_READINESS.md`

## Install

Official install priority:

1. GitHub Releases
2. Homebrew
3. winget
4. Scoop

### GitHub Releases

Download release asset for your platform from GitHub Releases.

Supported first-release assets:

- `geocode-vX.Y.Z-aarch64-apple-darwin.tar.gz`
- `geocode-vX.Y.Z-x86_64-apple-darwin.tar.gz`
- `geocode-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz`
- `geocode-vX.Y.Z-x86_64-pc-windows-msvc.zip`

Standalone release artifacts are only install path supported by `geocode self-update`.

### Homebrew

```bash
brew tap geocode-cli/tap
brew install geocode
```

Upgrade:

```bash
brew update
brew upgrade geocode
```

Uninstall:

```bash
brew uninstall geocode
```

### winget

```powershell
winget install GeoCode.GeoCode
```

Upgrade:

```powershell
winget upgrade GeoCode.GeoCode
```

Uninstall:

```powershell
winget uninstall GeoCode.GeoCode
```

### Scoop

```powershell
scoop bucket add geocode https://github.com/geocode-cli/scoop-bucket
scoop install geocode
```

Upgrade:

```powershell
scoop update geocode
```

Uninstall:

```powershell
scoop uninstall geocode
```

### Developer Path

`cargo install --git` is contributor/developer path only. Not recommended public install path.

## Support Matrix

| OS | Architecture | Target triple | Support |
| --- | --- | --- | --- |
| macOS | Apple Silicon | `aarch64-apple-darwin` | Tier 1 |
| macOS | Intel | `x86_64-apple-darwin` | Tier 1 |
| Windows | x86_64 | `x86_64-pc-windows-msvc` | Tier 1 |
| Linux | x86_64 glibc baseline | `x86_64-unknown-linux-gnu` | Tier 1 with baseline limits |

Linux notes:

- support limited to glibc baseline environment
- non-glibc Linux distributions are out of scope for first public release

## Commands

Supported public CLI commands:

```bash
geocode inspect <file>
geocode mean <file> --var <name>
geocode compare <file-a> <file-b> --var <name>
geocode ask [--file <path> ...] <query>
geocode version
geocode doctor
geocode self-update
```

Notes:

- `inspect` supports NetCDF and GeoTIFF
- `mean` requires `--var` for NetCDF
- `compare` requires same dataset type and `--var` for NetCDF
- `ask` is planner-backed and may require provider configuration
- `doctor` exposes support/runtime diagnostics
- `self-update` is for standalone GitHub release installs only

TUI:

- launch with `geocode`
- included in public builds
- secondary contract; CLI compatibility has priority

## JSON Stability

Public commands support `--json`.

Examples:

```bash
geocode --json inspect sample.nc
geocode --json version
geocode --json doctor
geocode --json self-update
```

JSON schema versioning is deferred in `v0.x`.

Expectations:

- documented fields are treated as public contract
- optional fields may be added
- breaking JSON changes must be called out in release notes

## Versioning

GeoCode uses moderate SemVer during `v0.x`.

Rules:

- tags use `vMAJOR.MINOR.PATCH`
- patch releases should not intentionally break documented CLI behavior
- minor releases may include breaking changes when needed, but must be explicit in release notes

## Upgrade Policy

`geocode self-update`:

- supported for standalone GitHub release installs
- unsupported for Homebrew, Scoop, winget, and developer/source installs

Package-manager upgrade commands:

- Homebrew: `brew upgrade geocode`
- Scoop: `scoop update geocode`
- winget: `winget upgrade GeoCode.GeoCode`

## Native Runtime Policy

Official release artifacts aim to work without manual GDAL or NetCDF installation.

First-release runtime model:

- bundled native runtime beside executable
- bundled helper binaries including `gdalinfo` and `ncdump`
- packaged installs should prefer bundled runtime over ambient machine state

Source builds still may require native geospatial dependencies on machine.

## Paths And Storage

### App Paths

macOS:

- config/state: `~/Library/Application Support/geocode`
- cache: `~/Library/Caches/geocode`

Linux:

- config: `~/.config/geocode` or `XDG_CONFIG_HOME/geocode`
- cache: `~/.cache/geocode` or `XDG_CACHE_HOME/geocode`
- state: `~/.local/state/geocode` or `XDG_STATE_HOME/geocode`

Windows:

- config: `%APPDATA%/geocode`
- cache: `%LOCALAPPDATA%/geocode/cache`
- state: `%LOCALAPPDATA%/geocode/state`

### Current Stored Files

- provider configs: platform config dir under `providers/`
- default provider: platform config dir `default-provider.json`
- memory store: platform state dir `memory.json`
- session store: platform state dir `session.json`
- Codex auth/model cache: existing `.codex` home-based path

### Provider/Auth Setup

Current OpenAI path:

```bash
export OPENAI_API_KEY="sk-..."
geocode ask "inspect sample.nc"
```

`OPENAI_API_KEY` overrides stored provider config when set.

## Diagnostics

Use:

```bash
geocode doctor
geocode --json doctor
```

`doctor` exposes:

- executable path
- target triple
- detected install source
- self-update eligibility
- config/cache/state paths
- helper binary availability
- provider configured state summary

## Troubleshooting

### Native Runtime Issues

If command fails due to runtime dependency issue:

1. run `geocode doctor`
2. confirm expected helper binaries are available
3. confirm install method matches supported channel
4. prefer official packaged install over source build for end-user machines

### macOS

- use matching Intel vs Apple Silicon build
- if install came from Homebrew, use `brew upgrade geocode` not `self-update`
- if artifact came from GitHub Releases, prefer fresh unpack over moving internal runtime files manually

### Windows

- use official `x86_64-pc-windows-msvc` artifact
- if install came from Scoop or winget, use package-manager upgrade command
- run `geocode doctor` to confirm runtime helper discovery and install source

### Linux

- use supported glibc baseline environment only
- unsupported libc or distro combinations may fail outside support contract
- prefer official packaged artifact over ad hoc source build for end-user use

### Release Notes

Read release notes for:

- breaking CLI changes
- JSON behavior changes
- install/upgrade flow changes
- runtime support limitations

## Build From Source

```bash
cargo build
cargo test
```

Current source builds may require GDAL/NetCDF runtime and development pieces on machine.

## Launch Readiness

Launch/support docs:

- `RELEASE_VALIDATION.md`
- `RELEASE_PROCESS.md`
- `DIAGNOSTICS.md`

## Project Docs

```text
PUBLIC_RELEASE_PLAN.md
PUBLIC_RELEASE_POLICY.md
PHASE1_AUDIT.md
NATIVE_DEPENDENCY_STRATEGY.md
PUBLIC_CLI_CONTRACT.md
HOMEBREW_DISTRIBUTION.md
SCOOP_DISTRIBUTION.md
WINGET_DISTRIBUTION.md
SELF_UPDATE.md
DIAGNOSTICS.md
RELEASE_VALIDATION.md
RELEASE_PROCESS.md
PUBLIC_LAUNCH_READINESS.md
```
