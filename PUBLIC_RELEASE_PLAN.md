# Public Release Plan

## Goal

Prepare GeoCode for public distribution across macOS, Linux, and Windows with:

- GitHub Releases as the canonical release source
- Homebrew support for macOS
- winget and Scoop support for Windows
- `geocode self-update` for supported standalone installs
- CLI as the supported contract
- TUI shipped, but not the primary compatibility contract
- no manual GDAL/NetCDF installation required for end users of official builds

## Product Decisions

- Audience: technical users first
- Public install priority: prebuilt binaries
- Release channels:
  - GitHub Releases
  - Homebrew
  - winget
  - Scoop
- Upgrade command: `geocode self-update`
- Compatibility policy: moderate SemVer during `v0.x`
- Supported contract: CLI
- TUI status: included, secondary contract
- Native dependency goal: users should not manually install GDAL/NetCDF themselves
- Priority: fastest path to public release

## Core Constraints

1. Native geospatial dependencies are the main release bottleneck.
2. `cargo install --git` is not the right primary user installation path.
3. Windows Tier 1 support requires dedicated path, executable, and packaging work.
4. Self-update should depend on stable GitHub release artifacts, not precede them.
5. Docs and CLI surface must align before public release.

## Release Strategy

### Canonical Distribution Model

1. Publish platform-specific release artifacts to GitHub Releases.
2. Use those assets as the source for:
   - Homebrew
   - Scoop
   - winget
3. Support `geocode self-update` only for eligible standalone installs at first.
4. Redirect package-manager installs to native package-manager upgrade flows.

### Supported Platforms

Initial target matrix:

- macOS:
  - `aarch64-apple-darwin`
  - `x86_64-apple-darwin`
- Windows:
  - `x86_64-pc-windows-msvc`
- Linux:
  - `x86_64-unknown-linux-gnu`

## Phase 0: Product And Contract Decisions

### Objectives

- lock down support promises
- avoid implementation drift
- define release and compatibility rules

### Tasks

1. Define the public distribution promise for `v0.x`.
2. Declare GitHub Releases as the canonical source of truth for versions.
3. Define official install channels for phase 1:
   - `GitHub Releases`
   - `Homebrew`
   - `winget`
   - `Scoop`
4. Define official support matrix by OS and architecture.
5. Define CLI as the supported compatibility contract.
6. Define TUI as shipped but not the primary compatibility contract.
7. Define `geocode self-update` scope for phase 1.
8. Decide whether `self-update` supports only standalone GitHub installs initially.
9. Decide how package-manager installs are detected and redirected.
10. Define versioning policy for `v0.x`.
11. Define what counts as a breaking change in CLI behavior.
12. Define what counts as a breaking change in JSON output.
13. Decide whether JSON schema versioning is needed now or deferred.
14. Define release-note requirements for breaking or behavior-changing releases.
15. Define naming convention for release tags like `v0.1.0`.
16. Define naming convention for release assets by platform and architecture.
17. Define support policy for Linux limitations if bundling is harder there.
18. Define whether `cargo install --git` remains developer-only and undocumented for public users.
19. Define minimum supported Rust version for contributors.
20. Define success criteria for first public release.

### Deliverables

- written release policy
- support matrix
- versioning policy
- update policy

## Phase 1: Current-State Audit And Gap Mapping

### Objectives

- identify all cross-platform blockers
- separate compile-time vs runtime dependency risks
- map release work to current implementation

### Tasks

1. Audit all direct uses of `gdal` crate APIs.
2. Audit all direct uses of `netcdf` crate APIs.
3. Audit all runtime shell-outs to geospatial binaries.
4. Audit all test-only shell-outs to geospatial binaries.
5. Audit all path handling assumptions across macOS, Linux, and Windows.
6. Audit all uses of `HOME`.
7. Audit all config, cache, state, and session file locations.
8. Audit all binary discovery logic for Windows compatibility.
9. Audit all command names that assume Unix naming rather than Windows executables.
10. Audit all README install instructions for mismatch with actual code.
11. Audit all documented CLI commands against current clap surface.
12. Audit all version-related behavior already present in the CLI.
13. Audit current git/release hygiene:
   - tags
   - changelog
   - workflow files
   - metadata completeness
14. Audit how current binaries behave if GDAL or NetCDF runtime pieces are missing.
15. Audit whether TUI startup depends on any platform-sensitive assumptions.
16. Audit whether provider and Codex integration introduce packaging or path issues.
17. Produce a dependency map separating:
   - compile-time native deps
   - runtime native deps
   - optional deps
   - test-only deps

### Deliverables

- dependency inventory
- platform-risk inventory
- release readiness gap report

## Phase 2: Platform Path And Storage Hardening

### Objectives

- remove Unix-only path assumptions
- make config/state storage platform-correct
- support clean installs on Windows, macOS, and Linux

### Tasks

1. Replace all raw `HOME` assumptions with platform-aware directory resolution.
2. Standardize config directory handling for macOS.
3. Standardize config directory handling for Linux.
4. Standardize config directory handling for Windows.
5. Standardize cache directory handling for macOS.
6. Standardize cache directory handling for Linux.
7. Standardize cache directory handling for Windows.
8. Standardize state/session storage directory handling across platforms.
9. Decide whether current working directory session storage remains supported.
10. Migrate provider config storage to platform-appropriate app paths.
11. Migrate memory storage to platform-appropriate app paths.
12. Migrate session storage to platform-appropriate app paths.
13. Add backward-compatible migration from legacy paths if needed.
14. Decide migration policy for old Unix-style locations like `.geocode` and `.config/geocode`.
15. Define file-permission expectations for stored credentials and tokens.
16. Review Codex auth/model cache path logic for Windows compatibility.
17. Review any TUI save/read paths for platform compatibility.
18. Add tests for path resolution behavior on each platform abstraction.
19. Add tests for legacy-path migration behavior.
20. Add tests proving config and state writes do not depend on repo location.

### Deliverables

- unified path abstraction
- migrated storage behavior
- platform path tests

## Phase 3: Windows Tier 1 Hardening

### Objectives

- make Windows a first-class supported platform
- fix executable discovery and runtime path behavior
- validate install and update flows on Windows

### Tasks

1. Make known-binary discovery `PATHEXT` aware.
2. Make known-binary discovery handle `.exe`.
3. Make known-binary discovery handle platform-specific executable suffix rules.
4. Review path joining and executable lookup for Windows path semantics.
5. Review Unicode/path-space handling on Windows command invocations.
6. Review temp-file and replace flow for Windows locking semantics.
7. Review stdout/stderr behavior for Windows console and TUI mode.
8. Review whether any shell command assumptions depend on Unix shells.
9. Ensure provider, auth, and session flows work without relying on Unix environment conventions.
10. Define Windows installer/archive format expectations for release assets.
11. Add Windows CI coverage for the supported binary targets.
12. Add Windows-specific smoke tests for binary startup and help output.
13. Add Windows-specific smoke tests for config path creation.
14. Add Windows-specific smoke tests for self-update staging and replacement flow.
15. Add Windows-specific troubleshooting notes for docs.

### Deliverables

- Windows-safe binary discovery
- Windows install/update validation
- Windows support guidance

## Phase 4: Native Dependency Strategy

### Objectives

- ensure official builds work without manual GDAL/NetCDF installation
- reduce runtime reliance on external tools where practical
- make packaged artifacts self-sufficient

### Tasks

1. Define what "users should not manually install GDAL/NetCDF" means operationally.
2. Decide how official builds will ship native GDAL libraries.
3. Decide how official builds will ship native NetCDF libraries.
4. Decide whether geospatial helper binaries are still needed at runtime in official builds.
5. Decide whether `gdalinfo` remains a runtime dependency or is replaced with in-process logic.
6. Decide whether `ncdump` remains a runtime dependency or is replaced with in-process logic.
7. Separate required production geo capabilities from optional or legacy fallbacks.
8. Define packaging approach for macOS native libraries.
9. Define packaging approach for Windows native libraries.
10. Define packaging approach for Linux native libraries.
11. Define dynamic library lookup strategy for each OS.
12. Define whether release assets include sidecar libraries, bundled runtime folders, or installer-managed placement.
13. Reduce runtime dependency on external CLI tools where practical.
14. Ensure official packaged builds can satisfy runtime capability detection out of the box.
15. Decide whether Linux support is limited to a specific baseline environment for first release.
16. Add a documented fallback policy if a geo capability is unavailable.
17. Add startup or diagnostics command to expose geo runtime status clearly.
18. Add test coverage for packaged-build capability detection assumptions.
19. Add smoke tests for geospatial commands against official-style packaged builds.
20. Add release validation checks ensuring packaged artifacts contain required runtime components.

### Deliverables

- per-platform bundling strategy
- runtime dependency policy
- packaged-build validation rules

## Phase 5: CLI Contract Stabilization

### Objectives

- define the official public CLI surface
- align docs with implementation
- reduce compatibility risk before distribution

### Tasks

1. Enumerate the CLI commands that are officially supported in first public release.
2. Reconcile documented commands with actual clap implementation.
3. Decide whether unsupported or in-progress commands stay hidden or are removed from docs.
4. Define public CLI help quality bar.
5. Define public JSON output expectations for the supported commands.
6. Decide whether to expose JSON schema version information.
7. Add or standardize a `version` command or `--version` output contract.
8. Include version, target triple, and optional commit metadata in version output.
9. Decide whether TUI and CLI share version metadata presentation.
10. Define error-message quality bar for public use.
11. Review command names and flags for future compatibility risk.
12. Review default behaviors for breaking-change risk.
13. Add CLI integration tests for all supported public commands.
14. Add CLI integration tests for public help and error output.
15. Add CLI integration tests for JSON mode where applicable.
16. Add compatibility notes for any command names likely to evolve later.

### Deliverables

- public CLI contract
- version output contract
- CLI integration coverage

## Phase 6: Package Metadata And Release Hygiene

### Objectives

- make the crate and repository release-ready
- establish a repeatable versioning and changelog workflow

### Tasks

1. Add `rust-version` to `Cargo.toml`.
2. Add `license` metadata.
3. Add `repository` metadata.
4. Add `homepage` metadata.
5. Add package description improvements if needed.
6. Decide whether `documentation` metadata should be added.
7. Create changelog policy.
8. Create initial `CHANGELOG.md`.
9. Define release note template.
10. Define commit/tag to release version workflow.
11. Define whether version bumps are manual or automated.
12. Define whether release notes are curated manually or generated from commits.
13. Define checksum generation policy for release assets.
14. Define artifact signing policy if desired now or later.
15. Define how pre-release versions are named and published.
16. Define whether nightly or preview channels exist or are deferred.
17. Add a support matrix section to docs.
18. Add installation-method support matrix to docs.
19. Add upgrade-policy section to docs.
20. Add troubleshooting section skeleton to docs.

### Deliverables

- complete crate metadata
- changelog and release-note policy
- release hygiene documentation

## Phase 7: GitHub Release Automation

### Objectives

- create tagged, reproducible releases
- publish canonical artifacts and checksums
- validate packaged binaries before publishing

### Tasks

1. Create CI workflow for build and test on all supported platforms.
2. Create release workflow triggered by version tags.
3. Build release artifacts for macOS Intel.
4. Build release artifacts for macOS Apple Silicon.
5. Build release artifacts for Windows MSVC.
6. Build release artifacts for Linux x86_64.
7. Package each artifact in a stable archive format.
8. Publish release artifacts to GitHub Releases.
9. Generate and publish checksums.
10. Attach release notes automatically or semi-automatically.
11. Add smoke-test steps for produced artifacts.
12. Validate that packaged artifacts run on clean environments.
13. Validate that packaged artifacts include required native components.
14. Validate that packaged artifacts can execute supported CLI commands.
15. Validate that `--version` output is correct in release artifacts.
16. Ensure release workflow fails fast on missing bundled dependencies.
17. Ensure reproducible artifact naming across versions.
18. Ensure release workflow exports metadata needed by package managers.
19. Document maintainer release process.
20. Document rollback or yank procedure for bad releases.

### Deliverables

- CI workflow
- tag-driven release workflow
- published canonical release assets

## Phase 8: Homebrew Distribution

### Objectives

- support easy macOS installation and upgrades
- consume official GitHub release assets

### Tasks

1. Decide whether to use a personal tap or aim for Homebrew core later.
2. Define formula source as GitHub release assets.
3. Define formula naming convention.
4. Define checksum workflow for formula updates.
5. Ensure macOS release assets are suitable for Homebrew consumption.
6. Add install test expectations for formula validation.
7. Document Homebrew install command.
8. Document Homebrew upgrade command.
9. Document Homebrew uninstall command.
10. Define how formula updates are published for each release.
11. Automate formula update if possible.
12. Validate install on Apple Silicon.
13. Validate install on Intel macOS.
14. Validate that installed binary can find bundled native libs.
15. Validate that package-manager-installed binary does not use `self-update`.
16. Add docs for package-manager-specific update redirection.

### Deliverables

- Homebrew formula path
- Homebrew install/upgrade docs
- Homebrew validation checklist

## Phase 9: Scoop Distribution

### Objectives

- support Windows install and upgrade through Scoop
- reuse official Windows release assets

### Tasks

1. Define Scoop bucket strategy.
2. Define Windows zip artifact format for Scoop.
3. Create Scoop manifest structure.
4. Add checksum/update workflow for Scoop manifest.
5. Document Scoop install command.
6. Document Scoop upgrade command.
7. Document Scoop uninstall command.
8. Validate fresh install on Windows.
9. Validate installed binary can find packaged native libs.
10. Validate path integration after Scoop install.
11. Validate package-manager-installed binary redirects away from `self-update`.
12. Define manifest update automation if possible.

### Deliverables

- Scoop manifest
- Scoop install/upgrade docs
- Scoop validation checklist

## Phase 10: winget Distribution

### Objectives

- support Windows install and upgrade through winget
- align artifact format and metadata with winget requirements

### Tasks

1. Decide installer or zip approach for winget.
2. Ensure Windows release asset format matches winget requirements.
3. Create winget manifest set.
4. Define checksum/update workflow for winget manifests.
5. Document winget install command.
6. Document winget upgrade command.
7. Document winget uninstall command.
8. Validate fresh install on Windows.
9. Validate package-manager-installed binary can run supported commands immediately.
10. Validate package-manager-installed binary redirects away from `self-update`.
11. Define winget manifest submission/update process.
12. Automate manifest generation where practical.

### Deliverables

- winget manifest set
- winget install/upgrade docs
- winget validation checklist

## Phase 11: Self-Update Design And Implementation

### Objectives

- implement safe standalone upgrades
- reuse GitHub Releases as the version source
- avoid breaking package-manager installs

### Tasks

1. Define supported install sources for `geocode self-update`.
2. Define unsupported install sources and redirect messages.
3. Detect current platform and architecture reliably.
4. Detect current installed version reliably.
5. Query latest GitHub release metadata.
6. Support pinned-version update requests if desired.
7. Match current platform to correct release artifact.
8. Download artifact securely.
9. Verify checksum before install.
10. Stage update in a temporary location.
11. Replace executable safely on macOS/Linux.
12. Replace executable safely on Windows.
13. Preserve file permissions and executability.
14. Add rollback behavior if replacement fails.
15. Ensure updater does not corrupt package-manager-owned installs.
16. Ensure updater does not corrupt running binary state.
17. Add user-facing output for update available, up to date, and failure states.
18. Add non-interactive and script-friendly behavior if needed.
19. Add tests for version parsing and release selection.
20. Add tests for checksum verification.
21. Add tests for unsupported install-source detection.
22. Add tests for Windows replacement flow.
23. Add tests for Unix replacement flow.
24. Add docs for `geocode self-update`.

### Deliverables

- self-update command
- checksum and rollback safety
- update-policy docs

## Phase 12: Diagnostics And Supportability

### Objectives

- make installation and runtime problems diagnosable
- reduce support burden after release

### Tasks

1. Add a diagnostics or doctor command if needed.
2. Expose current config, state, and runtime paths for support.
3. Expose detected geo/native runtime status for support.
4. Expose detected install source if possible.
5. Expose current update eligibility state.
6. Add troubleshooting guidance for missing native runtime issues.
7. Add troubleshooting guidance for package-manager install/update behavior.
8. Add troubleshooting guidance for Windows path issues.
9. Add troubleshooting guidance for Linux runtime-lib issues.
10. Add troubleshooting guidance for macOS Gatekeeper or quarantine issues if relevant.
11. Add support docs for stale config or migration issues.
12. Add support docs for CLI vs TUI expectations.

### Deliverables

- diagnostics surface
- support-focused runtime visibility
- troubleshooting docs

## Phase 13: Testing And Release Validation

### Objectives

- ensure each official install path works
- validate release readiness before public launch

### Tasks

1. Build a release-readiness checklist.
2. Add CI tests for supported CLI commands.
3. Add CI tests for JSON output on supported commands.
4. Add CI tests for config/state path behavior.
5. Add CI tests for Windows executable discovery.
6. Add CI tests for package-manager install smoke paths where feasible.
7. Add release validation tests for GitHub standalone binaries.
8. Add release validation tests for Homebrew install.
9. Add release validation tests for Scoop install.
10. Add release validation tests for winget install.
11. Add release validation tests for `geocode self-update`.
12. Add regression tests for version output.
13. Add regression tests for update redirection on package-manager installs.
14. Add regression tests for bundled-native-capability availability.
15. Add tests for clean-machine startup assumptions.
16. Run full manual smoke tests on each Tier 1 platform.
17. Document release signoff checklist.

### Deliverables

- release validation suite
- signoff checklist
- cross-platform smoke results

## Phase 14: Documentation And Public Launch Readiness

### Objectives

- make public docs accurate and install-ready
- clearly explain supported paths and upgrade behavior

### Tasks

1. Rewrite install section around GitHub Releases, Homebrew, winget, and Scoop.
2. Reframe `cargo install --git` as a contributor/developer path only if retained.
3. Document supported OS and architecture matrix.
4. Document CLI as the supported contract.
5. Document TUI as included but secondary contract.
6. Document current command set accurately.
7. Document JSON stability expectations for `v0.x`.
8. Document versioning policy.
9. Document upgrade policy.
10. Document `geocode self-update` behavior and limitations.
11. Document package-manager-specific upgrade commands.
12. Document troubleshooting for native dependency/runtime issues.
13. Document troubleshooting for Windows users.
14. Document troubleshooting for Linux users.
15. Document troubleshooting for macOS users.
16. Document provider/auth storage locations by platform.
17. Document config/cache/state locations by platform.
18. Document release-note reading expectations for breaking changes.
19. Prepare launch checklist for first public announcement.
20. Prepare post-release support checklist.

### Deliverables

- accurate install docs
- support matrix docs
- launch-readiness docs

## Phase 15: First Release Execution

### Objectives

- execute the first public release safely
- validate all supported distribution paths
- monitor and triage early issues quickly

### Tasks

1. Freeze scope for first public release.
2. Pick the first public version number.
3. Finalize changelog entries.
4. Finalize release notes.
5. Tag the release.
6. Run release workflow.
7. Verify published GitHub assets.
8. Verify checksums.
9. Verify Homebrew installation.
10. Verify Scoop installation.
11. Verify winget installation.
12. Verify standalone binary installation.
13. Verify `geocode self-update` behavior on standalone install.
14. Verify redirection behavior on Homebrew install.
15. Verify redirection behavior on Scoop install.
16. Verify redirection behavior on winget install.
17. Perform final clean-machine smoke tests.
18. Publish release announcement.
19. Monitor install issues.
20. Triage first-wave packaging or update regressions quickly.

### Deliverables

- first public release
- validated distribution paths
- launch monitoring checklist

## Cross-Cutting Execution Rules

1. Do not present `cargo install --git` as the easy public install path.
2. Keep GitHub Releases as the canonical version source.
3. Treat native dependency bundling as the critical path.
4. Prioritize CLI compatibility over TUI polish.
5. Keep Windows parity in scope from the beginning.
6. Do not ship `self-update` before release artifacts are stable.
7. Do not document commands that are not actually supported.
8. Prefer package-manager-native upgrade flows where appropriate.
9. Validate every install path on clean environments.
10. Prefer the smallest correct implementation in each phase.

## Suggested Execution Order

1. Phase 0
2. Phase 1
3. Phase 2
4. Phase 3
5. Phase 4
6. Phase 5
7. Phase 6
8. Phase 7
9. Phase 8
10. Phase 9
11. Phase 10
12. Phase 11
13. Phase 12
14. Phase 13
15. Phase 14
16. Phase 15

## First Critical Path

The fastest safe path to public release is:

1. lock product/release contract
2. harden platform paths and Windows support
3. solve native dependency bundling
4. stabilize public CLI contract
5. build GitHub release automation
6. wire Homebrew, Scoop, and winget
7. implement `geocode self-update`
8. finish docs, validation, and release execution

## Success Criteria

The first public release is successful when:

1. a fresh macOS machine can install and run GeoCode without manual GDAL/NetCDF setup
2. a fresh Windows machine can install and run GeoCode without manual GDAL/NetCDF setup
3. Linux official artifacts run in the supported baseline environment
4. GitHub Releases publishes working versioned assets
5. Homebrew, Scoop, and winget install successfully from official assets
6. CLI help and docs match the real supported contract
7. `geocode --version` reports useful release metadata
8. `geocode self-update` works for eligible standalone installs
9. package-manager installs redirect correctly to native upgrade commands
10. first-wave install failures can be diagnosed quickly through docs and diagnostics
