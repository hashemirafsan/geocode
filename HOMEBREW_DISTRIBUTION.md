# Homebrew Distribution

## Status

Phase 8 policy for first public macOS package-manager distribution.

## Strategy

First public release uses personal tap first.

Tap path:

- repo: `geocode-cli/homebrew-tap`
- formula path inside tap: `Formula/geocode.rb`

Homebrew core is deferred until:

- release process stable
- bundled native runtime layout stable
- formula updates low-friction and repeatable

## Formula Source

Formula must use official GitHub release assets only.

Source asset mapping:

- Apple Silicon: `geocode-vX.Y.Z-aarch64-apple-darwin.tar.gz`
- Intel macOS: `geocode-vX.Y.Z-x86_64-apple-darwin.tar.gz`

Formula should select asset by macOS architecture and install packaged contents from release archive.

## Formula Naming

Formula name for first public release:

- `geocode`

Tap install path:

- `geocode-cli/tap/geocode`

## Checksum Workflow

Checksum source:

- `geocode-vX.Y.Z-checksums.txt` from GitHub release
- `geocode-vX.Y.Z-release-metadata.json` from GitHub release

Update flow per release:

1. Release workflow publishes macOS archives and checksums.
2. Maintainer reads checksums from release metadata artifact.
3. Maintainer updates `url`, `sha256`, and `version` in tap formula.
4. Maintainer opens or merges tap update commit.

Preferred automation:

- release workflow or follow-up script opens PR in tap repo with updated formula
- if automation unavailable, manual update acceptable for first release

## Formula Installation Model

Homebrew formula should install:

- `geocode` binary into Homebrew `bin`
- packaged `runtime/` folder into stable libexec location
- wrapper or install layout that preserves runtime-relative helper/native library lookup

Requirement:

- Homebrew-installed binary must still find bundled native libraries
- Homebrew-installed binary must still find bundled `gdalinfo` and `ncdump`

## User Commands

Install:

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

## Self-Update Policy

Homebrew-installed GeoCode must not use `geocode self-update`.

Required behavior:

- detect Homebrew install source
- refuse in-place self-update
- print redirect message: `brew upgrade geocode`

## Formula Validation Expectations

Minimum install test expectations:

1. `brew install geocode` succeeds on Apple Silicon.
2. `brew install geocode` succeeds on Intel macOS.
3. `geocode version` works after install.
4. `geocode --version` works after install.
5. `geocode --help` lists public commands.
6. installed binary can find bundled runtime libs.
7. installed binary can find bundled `gdalinfo` helper.
8. installed binary can find bundled `ncdump` helper.
9. `geocode inspect missing.nc` fails with expected public error shape.
10. Homebrew-installed binary redirects away from `self-update`.

## Publish Flow

Per release:

1. GitHub release finishes successfully.
2. Verify both macOS archives exist.
3. Verify release metadata/checksums exist.
4. Update tap formula.
5. Run formula install test on Apple Silicon and Intel.
6. Publish tap change.

## Deliverables Produced

- Homebrew formula path: this document
- Homebrew install/upgrade docs: this document
- Homebrew validation checklist: this document
