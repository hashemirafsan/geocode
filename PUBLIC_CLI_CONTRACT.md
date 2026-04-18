# GeoCode Public CLI Contract

## Status

Phase 5 contract for first public CLI surface.

## Supported Public Commands

First public clap contract includes:

- `geocode inspect <file>`
- `geocode mean <file> [--var <name>]`
- `geocode compare <file-a> <file-b> [--var <name>]`
- `geocode ask [--file <path> ...] <query>`
- `geocode version`
- `geocode --help`
- `geocode --version`

TUI remains available at bare `geocode`, but CLI compatibility promise centers on documented root commands above.

## Command Semantics

### `inspect`

- inspects one local dataset
- supported file types: NetCDF and GeoTIFF
- validates file existence before backend execution
- returns human-readable text or JSON with `--json`

### `mean`

- computes mean summary for one dataset
- NetCDF requires `--var`
- GeoTIFF currently supports single-band path only
- returns human-readable text or JSON with `--json`

### `compare`

- compares scalar mean summaries for two same-type datasets
- NetCDF requires `--var`
- difference remains `mean_b - mean_a`
- no alignment, reprojection, or resampling promise in first public contract
- returns human-readable text or JSON with `--json`

### `ask`

- planner-backed natural-language entrypoint
- supports repeatable `--file`
- may execute local typed capability plan after planning
- provider configuration required
- JSON support available with `--json`

### `version`

- human-readable output includes version and target
- JSON output includes `name`, `version`, `target`, and optional `commit`

## Version Output Contract

Text contract:

```text
GeoCode <version>
Target: <target-triple>
Commit: <sha>    # optional when available
```

JSON contract:

```json
{
  "command": "version",
  "summary": "GeoCode 0.x.y\nTarget: <target-triple>",
  "dataset_kind": null,
  "details": {
    "name": "geocode",
    "version": "0.x.y",
    "target": "<target-triple>",
    "commit": null
  }
}
```

## Compatibility Notes

- root commands above are supported public contract
- hidden legacy `geocode cli ask` compatibility path may exist temporarily, but is not public contract
- TUI slash commands remain secondary contract
- JSON schema versioning still deferred per Phase 0 policy

## Error Quality Bar

Public CLI should prefer:

- explicit missing-file errors
- explicit unsupported-type errors
- clear provider configuration errors for `ask`
- deterministic command summaries for successful output

## Test Expectations

Public CLI coverage should include:

- `--help`
- `version`
- root command parsing
- representative error-path coverage for public commands
