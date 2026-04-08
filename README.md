# GeoCode

GeoCode is a local-first, CLI-first geospatial analysis tool for NetCDF and GeoTIFF datasets.

Current focus:
- deterministic command execution
- explicit CLI workflows
- machine-readable JSON output
- minimal session foundation

## Provider Auth Strategy
Current provider implementation:
- OpenAI is API key based
- expected env var: `OPENAI_API_KEY`
- CLI-based key storage is also supported

Planned provider architecture:
- provider abstraction must support both API key and OAuth-based providers
- GeoCode should only promise OAuth where the target provider exposes a product-safe OAuth flow
- OpenAI should be treated as API-key first in the current product shape

This distinction is important:
- provider abstraction supports OAuth-capable providers later
- current OpenAI support is not an OAuth flow

### Set API Key From CLI
Recommended:

```bash
printf '%s' "$OPENAI_API_KEY" | geocode provider set-api-key openai --stdin
```

Also supported:

```bash
geocode provider set-api-key openai --api-key "sk-..."
```

Notes:
- `--stdin` is safer than passing the key directly on the command line
- stored provider config is written outside the repo under `~/.config/geocode/openai.json`
- `OPENAI_API_KEY` still overrides stored config when set

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

### Provider Status
Inspect the current provider configuration.

```bash
geocode provider list
geocode provider status
geocode --json provider status
```

Current behavior:
- lists supported providers explicitly
- reports provider name
- reports auth method
- reports configured state
- reports model and base URL
- reports config path and credential source

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
- unconfigured provider usage for `ask`

## Agent Entry
Current agent entrypoint:

```bash
geocode ask "show all variables in base.nc"
geocode ask --file base.nc "show all variables in this file"
geocode ask --file base.nc --file scenario.nc "compare these datasets"
```

Current behavior:
- `ask` supports explicit file selection via repeatable `--file`
- if `OPENAI_API_KEY` is missing, GeoCode fails cleanly with setup guidance
- a stored CLI-configured key also enables `ask`
- if configured, GeoCode attempts a planner-only OpenAI request and returns a structured plan
- execution still remains outside the LLM

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
