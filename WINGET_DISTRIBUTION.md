# winget Distribution

## Status

Phase 10 policy for first public Windows package-manager distribution through winget.

## Strategy

First public release uses zip/portable manifest first.

Manifest path:

- target repo: `microsoft/winget-pkgs`
- manifest package identifier: `GeoCode.GeoCode`

Installer/MSI flow is deferred until:

- release process stable
- bundled Windows runtime layout stable
- portable package updates low-friction and repeatable

## Manifest Source

Manifest must use official GitHub release assets only.

Source asset mapping:

- Windows x86_64: `geocode-vX.Y.Z-x86_64-pc-windows-msvc.zip`

Manifest should reference official portable zip and preserve packaged runtime-relative layout.

## Manifest Structure

First public release should use winget portable/zip manifest set with:

- version manifest
- installer manifest
- locale/default locale manifest

Manifest package name:

- `GeoCode`

Manifest identifier:

- `GeoCode.GeoCode`

## Checksum Workflow

Checksum source:

- `geocode-vX.Y.Z-checksums.txt` from GitHub release
- `geocode-vX.Y.Z-release-metadata.json` from GitHub release

Update flow per release:

1. Release workflow publishes Windows zip and checksums.
2. Maintainer reads checksum from release metadata artifact.
3. Maintainer updates winget manifest files with `PackageVersion`, `InstallerUrl`, and `InstallerSha256`.
4. Maintainer submits manifest PR to `microsoft/winget-pkgs`.

Preferred automation:

- release workflow or follow-up script generates manifest files automatically
- maintainer still reviews and submits PR if direct automation to winget repo not used

## Installation Model

winget portable package should install:

- `geocode.exe` into winget-managed install location
- packaged `runtime/` folder beside installed binary
- packaged helper binaries and native libraries without flattening runtime layout

Requirement:

- winget-installed binary must run supported commands immediately after install
- winget-installed binary must still find bundled native libraries
- winget-installed binary must still find bundled `gdalinfo.exe` and `ncdump.exe`

## User Commands

Install:

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

## Self-Update Policy

winget-installed GeoCode must not use `geocode self-update`.

Required behavior:

- detect winget install source
- refuse in-place self-update
- print redirect message: `winget upgrade GeoCode.GeoCode`

## Validation Expectations

Minimum install test expectations:

1. `winget install GeoCode.GeoCode` succeeds on supported Windows machine.
2. `geocode version` works after install.
3. `geocode --version` works after install.
4. `geocode --help` lists public commands.
5. installed binary can find bundled runtime libs.
6. installed binary can find bundled `gdalinfo.exe` helper.
7. installed binary can find bundled `ncdump.exe` helper.
8. `geocode inspect missing.nc` fails with expected public error shape.
9. winget-installed binary redirects away from `self-update`.
10. supported public commands run immediately without extra machine setup.

## Submission Flow

Per release:

1. GitHub release finishes successfully.
2. Verify Windows zip exists.
3. Verify release metadata/checksums exist.
4. Generate or update winget manifest set.
5. Run winget install validation on clean Windows environment.
6. Submit or merge manifest PR in `microsoft/winget-pkgs`.

## Deliverables Produced

- winget manifest set path: this document
- winget install/upgrade docs: this document
- winget validation checklist: this document
