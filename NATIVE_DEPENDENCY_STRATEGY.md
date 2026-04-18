# GeoCode Native Dependency Strategy

## Status

Phase 4 policy. Defines how official public builds satisfy native GDAL and NetCDF requirements without manual end-user setup.

## Operational Definition

"Users should not manually install GDAL or NetCDF" means:

- official GeoCode release artifacts include all required native runtime pieces for supported commands
- official GeoCode release artifacts include required helper binaries for first public release
- supported commands run on clean supported machines after unpack/install only
- users do not need separate GDAL, NetCDF, `gdalinfo`, or `ncdump` installation steps

This promise applies to official GitHub release artifacts and package-manager distributions built from them.

This promise does not apply to:

- source builds from repo
- `cargo build`
- `cargo install --git`
- unsupported Linux distributions or architectures outside locked Phase 0 support matrix

## First-Release Strategy

GeoCode first public release uses sidecar runtime folder packaging.

Bundle shape:

- main `geocode` executable
- bundled native shared libraries required by GDAL and NetCDF paths
- bundled helper binaries used by current runtime capability paths
- checksums for full artifact

First public release keeps helper binaries:

- `gdalinfo`
- `ncdump`

Rationale:

- matches current implementation reality
- lowers risk versus forcing immediate full in-process replacement
- preserves fastest safe path to public release

Test-only helpers remain test-only and are not required in shipped artifacts:

- `ncgen`
- `gdal_translate`

## Runtime Dependency Policy

### Required In Official Builds

Official builds must ship runtime components needed for supported CLI behavior:

- GDAL shared libraries needed by `gdal` crate usage
- NetCDF shared libraries needed by `netcdf` crate usage
- `gdalinfo` helper binary for current GeoTIFF stats and fallback flows
- `ncdump` helper binary for current fallback capability flows

### Not Required From End User

End users of official builds must not need to install separately:

- GDAL SDK/runtime
- NetCDF SDK/runtime
- `gdalinfo`
- `ncdump`

### Allowed Runtime Assumptions

Official builds may assume:

- standard OS loader availability for system C/C++ runtime pieces normally present on supported baseline systems
- writable temporary directory
- writable app config/state directories

Official builds must not assume:

- GDAL already on `PATH`
- NetCDF already installed system-wide
- helper binaries already on `PATH`
- repo checkout location

## Per-Platform Bundling Strategy

### macOS

Artifact shape:

- `.tar.gz` archive per target triple
- `geocode` executable at archive root or stable `bin/` path
- sidecar `runtime/` folder for native libraries and helper binaries

Strategy:

- bundle required `.dylib` files in artifact
- bundle `gdalinfo` and `ncdump` in artifact
- configure executable/library lookup so bundled libraries resolve relative to packaged layout
- ensure both Apple Silicon and Intel artifacts carry matching runtime layout

### Windows

Artifact shape:

- `.zip` archive for `x86_64-pc-windows-msvc`
- `geocode.exe`
- sidecar `runtime\` folder or same-directory bundled `.dll` files and helper `.exe` files

Strategy:

- bundle required `.dll` files in artifact
- bundle `gdalinfo.exe` and `ncdump.exe` in artifact
- prefer same-directory or packaged-runtime lookup model over PATH-dependent resolution
- ensure packaged command execution uses resolved bundled paths, not ambient system installation

### Linux

Artifact shape:

- `.tar.gz` archive for `x86_64-unknown-linux-gnu`
- `geocode` executable
- sidecar `runtime/` folder with bundled `.so` files and helper binaries

Strategy:

- bundle required shared libraries in artifact
- bundle `gdalinfo` and `ncdump` in artifact
- target locked glibc baseline only
- validate runtime lookup against packaged sidecar libraries on clean baseline environment

Linux limitation policy:

- support promise limited to declared glibc baseline
- unsupported distributions or incompatible libc environments may fail outside support contract
- release notes must state baseline clearly

## Dynamic Library Lookup Policy

GeoCode official builds should prefer packaged-runtime-relative lookup.

Policy:

- executable should locate bundled helper binaries relative to install location when possible
- bundled native libraries should resolve from artifact-local paths before relying on ambient system installs
- package-manager installs must preserve or recreate same runtime-relative layout

Avoid for first public release:

- requiring users to export loader-path environment variables manually
- relying on globally installed GDAL or NetCDF components
- mixing official builds with arbitrary system library versions unless explicitly documented as fallback only

## Helper Binary Policy

`gdalinfo` and `ncdump` remain allowed runtime helpers for first public release, but only as bundled official runtime components.

Policy:

- helper binaries are implementation detail, not public user prerequisite
- helper binary invocation should prefer packaged copies over PATH discovery in official builds when install-source detection exists
- future releases may replace helpers with in-process Rust logic gradually

## Source Build Policy

Source builds remain developer-oriented for now.

Source-build users may still need:

- system GDAL development/runtime packages
- system NetCDF development/runtime packages
- helper binaries used by tests or current runtime flows

Docs must keep this distinction clear:

- official builds: no manual geo dependency install
- source builds: native toolchain/dependency setup still expected

## Fallback Policy

If official packaged runtime cannot satisfy a required geo capability:

- command must fail clearly
- error must identify missing runtime component category when possible
- docs must provide troubleshooting path
- unsupported package-manager or source-build cases may point users to source-build setup guidance

Do not silently fall back to incompatible ambient system installs if doing so risks inconsistent behavior across machines.

## Packaged-Build Validation Rules

Every official artifact must pass validation before publish.

### Required Validation

1. Archive contains expected executable name for target platform.
2. Archive contains required bundled native libraries.
3. Archive contains bundled `gdalinfo` helper.
4. Archive contains bundled `ncdump` helper.
5. Executable starts on clean machine or clean CI image.
6. `geocode --version` works from packaged artifact.
7. Supported CLI commands can access required native capabilities without extra machine setup.
8. Bundled helper binaries execute from packaged layout.
9. GeoTIFF path works without system GDAL on PATH.
10. NetCDF path works without system NetCDF on machine.

### Failure Conditions

Release workflow must fail if:

- required runtime files missing from archive
- packaged executable cannot load bundled native libraries
- bundled helper binaries cannot execute
- supported commands require ambient system GDAL or NetCDF unexpectedly
- packaged behavior differs across supported targets in a way not documented by contract

## Release Artifact Layout Convention

Recommended archive layout:

```text
geocode-v0.x.y-<target>/
  geocode[.exe]
  runtime/
    <shared libraries>
    gdalinfo[.exe]
    ncdump[.exe]
  LICENSES/
    <third-party notices as needed>
```

Stable layout matters for:

- package-manager formulas/manifests
- future `self-update`
- release validation automation

## Future Simplification Path

Preferred medium-term direction:

1. remove runtime dependence on `gdalinfo` where crate/in-process stats path is good enough
2. reduce planner-visible `ncdump` fallback dependence where crate-backed NetCDF coverage is complete
3. keep official artifact layout stable while internals simplify

Public contract should stay stable even if bundled internals change later.

## Deliverables Produced By This Strategy

- per-platform bundling strategy: this document
- runtime dependency policy: this document
- packaged-build validation rules: this document
