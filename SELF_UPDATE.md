# Self Update

## Status

Phase 11 design and implementation notes for `geocode self-update`.

## Scope

`geocode self-update` supports only standalone installs from official GitHub release artifacts.

Unsupported install sources:

- Homebrew
- Scoop
- winget
- unknown/source-build installs without packaged runtime layout

Redirect commands:

- Homebrew: `brew upgrade geocode`
- Scoop: `scoop update geocode`
- winget: `winget upgrade GeoCode.GeoCode`

## Current Behavior

- queries latest GitHub release from `geocode-cli/geocode`
- selects asset matching current target triple
- downloads matching archive and checksum manifest
- verifies SHA-256 before applying update
- requires packaged `runtime/` layout for standalone eligibility
- Unix-like systems replace binary and runtime dir in place
- Windows stages replacement through helper script after process exits

## Current Limitations

- pinned-version update requests not implemented yet
- Windows flow stages swap for process exit rather than fully in-process replacement
- package-manager/source-build detection still heuristic outside known install paths

## User Contract

- standalone install: update in place from GitHub Releases
- package-manager install: refuse and redirect to native upgrade command
- unknown install: refuse with explicit standalone-only message
