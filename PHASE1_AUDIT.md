# GeoCode Phase 1 Audit

## Scope

Phase 1 audit against `PUBLIC_RELEASE_PLAN.md` and locked Phase 0 policy in `PUBLIC_RELEASE_POLICY.md`.

## Summary

GeoCode not release-ready for public distribution yet. Biggest gaps:

1. Public docs and actual CLI surface do not match.
2. Platform path and storage handling is Unix-centric.
3. Windows executable discovery is not `PATHEXT` aware.
4. Official builds still depend on native GDAL and NetCDF installation.
5. Release hygiene and automation are mostly absent.

## Dependency Inventory

### Compile-Time Native Dependencies

- `gdal` crate via `Cargo.toml`
- `netcdf` crate via `Cargo.toml`

Direct `gdal` crate usage:

- `src/bindings/gdal.rs`
  - `Dataset::open` for GeoTIFF metadata reads
  - raster band access for band dtype and nodata metadata

Direct `netcdf` crate usage:

- `src/bindings/netcdf.rs`
  - `netcdf::open` for dataset open
  - dimension enumeration
  - variable enumeration
  - variable metadata reads
  - numeric variable value reads for mean computation

### Runtime Native Dependencies

Current runtime geospatial binaries:

- `gdalinfo`
  - used by `src/bindings/process.rs`
  - required by current GeoTIFF mean path through `src/bindings/gdal.rs`
- `ncdump`
  - discovered and exposed in capability catalog as fallback backend
  - planner-visible fallback route exists even if current direct CLI path mostly uses crate binding

Current runtime non-geo external binary with packaging impact:

- `codex`
  - invoked by `src/auth/codex.rs`
  - invoked by `src/provider/client.rs`
  - affects auth/model discovery and planner integration behavior

### Test-Only Native Dependencies

- `ncgen`
  - used in `src/bindings/netcdf.rs` tests to build sample NetCDF fixtures
- `gdal_translate`
  - used in `src/bindings/gdal.rs` and `src/bindings/process.rs` tests to create nodata-tagged TIFF fixtures

### Optional Dependencies

- `codex` CLI appears optional for Codex-backed login/planning only
- provider network access depends on configured OpenAI-compatible endpoints

## Platform Risk Inventory

### Path And Storage Risks

- `src/provider/config.rs`
  - raw `HOME` lookup
  - active config path uses `~/.geocode`
  - legacy fallback path uses `~/.config/geocode`
- `src/memory/store.rs`
  - raw `HOME` lookup
  - hardcoded `~/.config/geocode/memory.json`
- `src/auth/codex.rs`
  - raw `HOME` lookup
  - hardcoded `~/.codex/auth.json`
  - hardcoded `~/.codex/models_cache.json`
- `src/session/store.rs`
  - session stored in current working directory as `.geocode-session.json`
  - behavior depends on repo or launch directory

Impact:

- Windows compatibility weak
- config, cache, state, session behavior inconsistent
- current storage policy not aligned with Phase 0 public install expectations

### Binary Discovery And Command Risks

- `src/runtime/discovery.rs`
  - searches `PATH` by joining plain command name only
  - not `PATHEXT` aware
  - not `.exe` aware
  - executable check only verifies file exists
- `src/runtime/host.rs`
  - executes `binary.command_name()` instead of discovered resolved path
  - discovery result not reused for execution

Impact:

- Windows known-binary discovery likely fails for standard installs
- even discovered paths are not authoritative execution source
- path-space and suffix handling not hardened

### CLI Surface And Contract Risks

- `src/cli.rs`
  - clap surface exposes TUI by default and `geocode cli ask`
  - direct root CLI commands like `inspect`, `mean`, `compare`, `provider`, `session` are not public clap commands
- `src/app.rs`
  - `inspect`, `mean`, and `compare` logic exists internally
  - provider/session/model commands exist as TUI slash commands, not public clap commands
- `README.md`
  - documents commands and behaviors not present in current clap surface

Impact:

- current docs violate Phase 0 rule that docs and CLI surface must align before public release
- current public contract cannot yet claim CLI-first stable surface

### Version And Release Hygiene Risks

- `Cargo.toml`
  - missing `rust-version`
  - missing `license`
  - missing `repository`
  - missing `homepage`
- repo state
  - no `.github/workflows/` release automation found
  - no `CHANGELOG.md` found
  - no release-note template found

Impact:

- package metadata incomplete
- no reproducible public release flow yet

### Native Dependency Risks

- GeoTIFF mean still shells out to `gdalinfo` for stats
- official artifact bundling strategy for GDAL and NetCDF not defined
- README still states external tools currently used by implementation

Impact:

- Phase 0 public distribution promise not yet satisfied
- clean-machine installs will still fail without native runtime setup

### TUI And Provider Packaging Risks

- TUI uses current directory and session persistence paths directly
- Codex auth/model cache assumes Unix home layout
- provider config path split between `.geocode` and `.config/geocode` legacy fallback

Impact:

- Windows path behavior likely inconsistent
- packaged support burden high without path hardening and diagnostics

## Current-State Audit Notes

### Direct `gdal` Crate API Usage

- limited to `src/bindings/gdal.rs`
- metadata path uses crate directly
- mean path still depends on `gdalinfo` JSON output for stats parity

### Direct `netcdf` Crate API Usage

- concentrated in `src/bindings/netcdf.rs`
- crate currently handles open, enumerate, describe, and load operations directly
- tests still require `ncgen` to synthesize fixtures

### Runtime Shell-Outs To Geospatial Binaries

- `gdalinfo` runtime shell-out in `src/bindings/process.rs`
- capability catalog models `ncdump` and `gdalinfo` fallback families

### Test-Only Shell-Outs To Geospatial Binaries

- `ncgen` in NetCDF binding tests
- `gdal_translate` in GDAL binding and process binding tests

### HOME Usage Audit

- `src/auth/codex.rs`
- `src/provider/config.rs`
- `src/memory/store.rs`
- tests set `HOME` explicitly in `tests/cli_ask.rs`

### Config, Cache, State, Session Locations

- provider config: `~/.geocode/*.json`
- provider legacy fallback: `~/.config/geocode/*.json`
- memory state: `~/.config/geocode/memory.json`
- session state: `./.geocode-session.json`
- Codex auth/model cache: `~/.codex/*`

No unified platform-aware path abstraction yet.

### Windows Compatibility Audit

- binary discovery not `PATHEXT` aware
- command execution assumes Unix-style bare names
- raw `HOME` use instead of Windows profile/app-data locations
- no Windows-specific workflow or smoke-test coverage found

## Release Readiness Gap Report

### Blockers

1. CLI contract not stabilized; README documents commands not present in clap surface.
2. Public install promise cannot be met while GDAL/NetCDF runtime setup remains external.
3. Windows Tier 1 support blocked by path storage and executable discovery assumptions.
4. No release automation, checksums, changelog, or package metadata baseline.
5. `self-update` policy locked in Phase 0, but implementation and install-source detection not present.

### Medium Gaps

1. Session persistence tied to current working directory.
2. Provider and memory paths inconsistent across features.
3. Codex integration introduces path and external-binary packaging issues.
4. Linux baseline support policy locked, but artifact validation strategy absent.

## Recommended Next Execution Order

1. Phase 2 path/storage hardening
2. Phase 3 Windows executable discovery hardening
3. Phase 4 native dependency strategy
4. Phase 5 CLI contract stabilization
5. Phase 6 release hygiene

## Deliverables Produced

- dependency inventory: this document
- platform-risk inventory: this document
- release readiness gap report: this document
