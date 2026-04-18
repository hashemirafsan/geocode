# Release Validation

## Status

Phase 13 release-readiness and signoff checklist.

## Automated Validation Suite

Current automated coverage:

- public CLI help/version tests
- diagnostics command tests
- self-update redirect/install-source tests
- path resolution unit tests
- Windows binary discovery unit tests
- packaged release workflow smoke tests in `.github/workflows/release.yml`

## Release-Readiness Checklist

Before public release, verify all:

1. `cargo test` passes on `main`.
2. CI passes on macOS, Linux, Windows.
3. Release workflow builds all Tier 1 artifacts.
4. Release workflow smoke tests packaged artifacts.
5. Checksums generated and published.
6. Release metadata JSON generated and published.
7. Public CLI contract matches docs.
8. `geocode --version` and `geocode version` match release metadata.
9. `geocode doctor` reports expected install source and runtime visibility.
10. `geocode self-update` updates standalone installs or redirects package-manager installs.
11. Homebrew install path validated.
12. Scoop install path validated.
13. winget install path validated.
14. Package-manager installs redirect away from `self-update`.
15. Bundled runtime helpers present in packaged artifacts.

## Manual Signoff Checklist

Maintainer signoff should record:

1. release tag
2. commit SHA
3. artifact names
4. checksum manifest verified
5. macOS Apple Silicon smoke result
6. macOS Intel smoke result
7. Windows smoke result
8. Linux smoke result
9. Homebrew smoke result
10. Scoop smoke result
11. winget smoke result
12. standalone self-update smoke result
13. known issues accepted or release blocked

## Cross-Platform Smoke Results Template

```text
Release: vX.Y.Z
Commit: <sha>

macOS aarch64:
- install:
- geocode version:
- geocode doctor:
- geocode inspect missing.nc:
- result:

macOS x86_64:
- install:
- geocode version:
- geocode doctor:
- geocode inspect missing.nc:
- result:

Windows x86_64:
- install:
- geocode version:
- geocode doctor:
- geocode inspect missing.nc:
- result:

Linux x86_64:
- install:
- geocode version:
- geocode doctor:
- geocode inspect missing.nc:
- result:

Homebrew:
- install:
- upgrade:
- self-update redirect:
- result:

Scoop:
- install:
- upgrade:
- self-update redirect:
- result:

winget:
- install:
- upgrade:
- self-update redirect:
- result:

Standalone self-update:
- current version:
- target version:
- checksum verify:
- runtime preserved:
- result:
```

## Deliverables Produced

- release validation suite: tests + release workflow smoke checks
- signoff checklist: this document
- cross-platform smoke results template: this document
