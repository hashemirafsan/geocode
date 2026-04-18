# Scoop Distribution

## Status

Phase 9 policy for first public Windows package-manager distribution through Scoop.

## Strategy

First public release uses dedicated bucket first.

Bucket path:

- repo: `geocode-cli/scoop-bucket`
- manifest path inside bucket: `bucket/geocode.json`

Scoop main bucket is deferred until:

- release process stable
- bundled Windows runtime layout stable
- manifest updates low-friction and repeatable

## Manifest Source

Manifest must use official GitHub release assets only.

Source asset mapping:

- Windows x86_64: `geocode-vX.Y.Z-x86_64-pc-windows-msvc.zip`

Manifest should install packaged contents from release archive and preserve runtime-relative layout.

## Manifest Naming

Manifest name for first public release:

- `geocode`

Bucket install path:

- `geocode-cli/scoop-bucket/geocode`

## Checksum Workflow

Checksum source:

- `geocode-vX.Y.Z-checksums.txt` from GitHub release
- `geocode-vX.Y.Z-release-metadata.json` from GitHub release

Update flow per release:

1. Release workflow publishes Windows zip and checksums.
2. Maintainer reads checksum from release metadata artifact.
3. Maintainer updates `version`, `url`, and `hash` in Scoop manifest.
4. Maintainer opens or merges bucket update commit.

Preferred automation:

- release workflow or follow-up script opens PR in bucket repo with updated manifest
- if automation unavailable, manual update acceptable for first release

## Manifest Installation Model

Scoop manifest should install:

- `geocode.exe` into Scoop app dir with shims on PATH
- packaged `runtime/` folder beside installed binary
- packaged helper binaries and native libraries without flattening runtime layout

Requirement:

- Scoop-installed binary must still find bundled native libraries
- Scoop-installed binary must still find bundled `gdalinfo.exe` and `ncdump.exe`
- Scoop shim/path integration must expose `geocode`

## User Commands

Install:

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

## Self-Update Policy

Scoop-installed GeoCode must not use `geocode self-update`.

Required behavior:

- detect Scoop install source
- refuse in-place self-update
- print redirect message: `scoop update geocode`

## Manifest Validation Expectations

Minimum install test expectations:

1. `scoop install geocode` succeeds on supported Windows machine.
2. `geocode version` works after install.
3. `geocode --version` works after install.
4. `geocode --help` lists public commands.
5. installed binary can find bundled runtime libs.
6. installed binary can find bundled `gdalinfo.exe` helper.
7. installed binary can find bundled `ncdump.exe` helper.
8. PATH/shim integration resolves `geocode` without manual path edits.
9. `geocode inspect missing.nc` fails with expected public error shape.
10. Scoop-installed binary redirects away from `self-update`.

## Publish Flow

Per release:

1. GitHub release finishes successfully.
2. Verify Windows zip exists.
3. Verify release metadata/checksums exist.
4. Update bucket manifest.
5. Run Scoop install validation on clean Windows environment.
6. Publish bucket change.

## Deliverables Produced

- Scoop manifest path: this document
- Scoop install/upgrade docs: this document
- Scoop validation checklist: this document
