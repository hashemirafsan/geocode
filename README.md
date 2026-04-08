# GeoCode

GeoCode is a local-first, CLI-first geospatial analysis tool for NetCDF and GeoTIFF datasets.

Current focus:
- deterministic command execution
- explicit CLI workflows
- machine-readable JSON output
- minimal session foundation

## Status
Implemented core `v0.1` commands:
- `inspect`
- `mean`
- `compare`

Supported formats:
- NetCDF (`.nc`)
- GeoTIFF (`.tif`, `.tiff`)

## Requirements
Rust toolchain:
- `cargo`

External tools currently used by the implementation:
- `ncdump`
- `ncgen` for tests
- `gdalinfo`
- `gdal_translate` for tests

These are used pragmatically for `v0.1` behavior. The long-term architecture still allows replacing internals with fully in-process Rust readers later.

## Build
```bash
cargo build
```

## Run
```bash
cargo run -- --help
```

## Commands
### Inspect
Inspect essential metadata for a local file.

```bash
geocode inspect path/to/file.nc
geocode inspect path/to/file.tif
geocode --json inspect path/to/file.nc
```

NetCDF inspect includes:
- variable names
- dimensions
- shape
- dtype where available

GeoTIFF inspect includes:
- raster size
- band count
- band dtype
- nodata if available

### Mean
Compute a mean summary.

```bash
geocode mean path/to/file.nc --var depth
geocode mean path/to/file.tif
geocode --json mean path/to/file.nc --var depth
```

Rules:
- NetCDF requires explicit `--var`
- NetCDF mean is computed over the full selected variable
- GeoTIFF mean is nodata-aware when nodata metadata exists
- GeoTIFF mean currently supports single-band files only

### Compare
Compare scalar means between two files of the same type.

```bash
geocode compare base.nc scenario.nc --var depth
geocode compare base.tif scenario.tif
geocode --json compare base.nc scenario.nc --var depth
```

Rules:
- same-type only
- scalar-summary only
- no alignment, reprojection, or resampling in `v0.1`
- difference is always `mean_b - mean_a`

## JSON Output
All core commands support `--json`.

Examples:
```bash
geocode --json inspect sample.nc
geocode --json mean sample.nc --var depth
geocode --json compare base.tif scenario.tif
```

JSON is available now, but the schema should be treated as early-stage until `v0.2`.

## Errors
GeoCode returns explicit CLI errors for cases such as:
- missing files
- unsupported file types
- missing NetCDF `--var`
- invalid NetCDF variables
- mixed-type compare requests

## Tests
Run the full test suite:

```bash
cargo test
```

Current test coverage includes:
- inspect for NetCDF and GeoTIFF
- mean for NetCDF and GeoTIFF
- compare for NetCDF and GeoTIFF
- failure-path checks for invalid inputs

## Project Docs
The longer architecture, business plan, and ticket backlog live in:

```text
geocode_architecture_plan.md
```

## Near-Term Next Steps
- session inspection commands
- README/examples refinement
- output normalization for golden files
- eventual replacement of shell-tool-backed internals where it makes sense
