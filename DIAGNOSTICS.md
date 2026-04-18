# Diagnostics

## Status

Phase 12 support surface for runtime diagnostics.

## Command

Use:

```bash
geocode doctor
geocode --json doctor
```

## Current Output

`doctor` exposes:

- current version
- target triple
- executable path
- detected install source
- self-update eligibility and redirect command
- config/cache/state directories
- provider/session/memory paths
- known runtime helper availability and resolved paths
- provider configured state summary

## Support Use

Use `doctor` first when triaging:

- missing bundled helper issues
- wrong install-source detection
- self-update redirect confusion
- path/storage location confusion
- package-manager runtime lookup failures
