# GeoCode Public Release Policy

## Status

Phase 0 policy. Locks public release contract for `v0.x` and guides later implementation phases.

## Public Distribution Promise For `v0.x`

GeoCode `v0.x` is published as prebuilt CLI-first release artifacts for supported platforms. Official public installs must not require end users to manually install GDAL or NetCDF for supported release artifacts.

GeoCode supports:

- versioned standalone release artifacts from GitHub Releases
- package-manager installs through official channels
- deterministic CLI usage as primary compatibility surface
- shipped TUI as secondary surface without same compatibility promise as CLI

GeoCode does not promise for first public release:

- `cargo install --git` as public install path
- broad Linux distro compatibility beyond declared baseline
- long-term stability for every TUI interaction or layout detail
- schema-versioned JSON contract in `v0.1`

## Canonical Release Source

GitHub Releases is canonical source of truth for:

- published versions
- release assets
- checksums
- release notes

All other public distribution channels must consume or reference GitHub release artifacts.

## Official Install Channels

Phase 1 official public install channels:

- GitHub Releases
- Homebrew
- winget
- Scoop

Install precedence for docs and support:

1. GitHub Releases
2. Homebrew
3. winget
4. Scoop

`cargo install --git` remains contributor-only and undocumented as public install guidance.

## Support Matrix

Initial public support matrix:

| OS | Architecture | Target triple | Support level |
| --- | --- | --- | --- |
| macOS | Apple Silicon | `aarch64-apple-darwin` | Tier 1 |
| macOS | Intel | `x86_64-apple-darwin` | Tier 1 |
| Windows | x86_64 | `x86_64-pc-windows-msvc` | Tier 1 |
| Linux | x86_64 glibc baseline | `x86_64-unknown-linux-gnu` | Tier 1 with baseline limits |

Linux support policy for first public release:

- official support limited to `x86_64-unknown-linux-gnu`
- support promise assumes glibc-based baseline environment
- non-glibc distributions and other architectures out of scope until explicitly added
- if native bundling constraints remain, release notes and docs must state Linux limits clearly

## Compatibility Contract

CLI is supported public compatibility contract for `v0.x`.

CLI compatibility promise covers:

- command names documented as public
- documented flags and arguments
- documented exit behavior
- documented human-readable behavior where materially user-visible
- documented JSON output fields for commands declared stable enough for public use

TUI policy:

- TUI ships with public builds
- TUI is supported feature, not primary compatibility contract
- TUI interaction details may change faster than CLI during `v0.x`
- TUI regressions still count as bugs, but TUI layout and interaction details do not carry same stability promise as documented CLI surface

## Update Policy

`geocode self-update` policy for phase 1:

- supported only for eligible standalone installs sourced from official GitHub release artifacts
- unsupported for Homebrew installs
- unsupported for winget installs
- unsupported for Scoop installs
- unsupported for `cargo install --git` installs

Package-manager installs must be detected and redirected to native upgrade flow.

Initial redirect policy:

- Homebrew: tell user to run `brew upgrade geocode`
- winget: tell user to run `winget upgrade geocode`
- Scoop: tell user to run `scoop update geocode`
- unknown package-managed install: refuse self-update and explain unsupported install source

Detection policy for package-managed installs:

- must prefer reliable install-source markers over path heuristics when available
- may use install path heuristics as fallback
- if install source cannot be proven safe for in-place replacement, `self-update` must refuse update

## Versioning Policy For `v0.x`

GeoCode uses moderate SemVer during `v0.x`.

Rules:

- release tags use `vMAJOR.MINOR.PATCH` format
- `v0.x` may still include breaking changes between minor releases when needed
- patch releases should not intentionally break documented CLI behavior
- minor releases may refine public surface, but must call out breaking changes explicitly
- once command or JSON behavior is documented as public, changes need release-note disclosure before shipping

## Breaking Change Policy

Breaking CLI changes include:

- removing public command
- renaming public command, flag, or argument
- changing required-vs-optional argument behavior
- changing exit status semantics in documented cases
- changing default behavior in way that materially changes result or workflow
- changing version output contract once documented

Breaking JSON changes include:

- removing documented field
- renaming documented field
- changing field type
- changing semantic meaning of existing field
- changing top-level shape for documented command output

Non-breaking JSON changes generally include:

- adding new optional fields
- adding new enum values when docs mark field as extensible
- adding new commands without changing existing documented command output

## JSON Schema Versioning

Formal JSON schema versioning is deferred for early public release.

Policy for `v0.x` first public release:

- JSON output is supported
- JSON stability expectations must be documented command by command
- if schema-version field becomes necessary later, add it in backward-compatible way before promising stricter schema guarantees

## Release Notes Policy

Every public release must include:

- version number
- supported platform assets
- user-visible fixes and additions
- install or upgrade notes if behavior changed

Releases with breaking or behavior-changing changes must also include:

- explicit breaking-change section
- migration guidance when practical
- note about affected commands, flags, or JSON fields
- note about install/update impact if package manager or standalone behavior changed

## Naming Conventions

Release tag naming:

- `v0.1.0`
- `v0.1.1`
- `v0.2.0`

Release asset naming format:

`geocode-v{version}-{target-triple}.{archive-ext}`

Examples:

- `geocode-v0.1.0-aarch64-apple-darwin.tar.gz`
- `geocode-v0.1.0-x86_64-apple-darwin.tar.gz`
- `geocode-v0.1.0-x86_64-unknown-linux-gnu.tar.gz`
- `geocode-v0.1.0-x86_64-pc-windows-msvc.zip`

Checksums should use matching versioned manifest names, such as `geocode-v0.1.0-checksums.txt`.

## Contributor Toolchain Policy

Contributor minimum supported Rust version is not pinned in Phase 0.

Current policy:

- contributors should use current stable Rust toolchain
- exact MSRV must be pinned before public release workflow is finalized
- pinned `rust-version` metadata belongs in later release hygiene phase

## First Public Release Success Criteria

First public release counts as successful when all statements are true:

1. Supported macOS installs work without manual GDAL or NetCDF setup.
2. Supported Windows installs work without manual GDAL or NetCDF setup.
3. Official Linux artifact runs in declared glibc baseline environment.
4. GitHub Releases publishes working versioned artifacts and checksums.
5. Homebrew, winget, and Scoop installs consume official artifacts successfully.
6. CLI help, docs, and shipped behavior agree on supported public commands.
7. `geocode --version` reports release metadata defined by public contract.
8. `geocode self-update` works for eligible standalone installs only.
9. Package-manager installs redirect users to native upgrade command instead of self-updating.
10. Release notes and diagnostics are sufficient to triage first-wave install failures.

## Deliverables Produced By This Policy

- written release policy: this document
- support matrix: defined in this document
- versioning policy: defined in this document
- update policy: defined in this document
