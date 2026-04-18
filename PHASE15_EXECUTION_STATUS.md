# Phase 15 Execution Status

## Status

Phase 15 execution prepared. Actual public release is blocked right now.

## Ready

- public release policy locked
- CLI contract locked
- package-manager plans documented
- self-update implemented
- diagnostics implemented
- release workflows added
- automated test suite passing locally before final worktree review

## Current Blockers

1. `packaging/runtime/<target>/` payloads do not exist yet.
2. `CHANGELOG.md` does not exist yet.
3. release notes not finalized.
4. Homebrew tap formula not implemented yet.
5. Scoop bucket manifest not implemented yet.
6. winget manifest set not implemented yet.
7. final clean-machine smoke results not recorded yet.

## Evidence

- release workflow requires `packaging/runtime/<target>/`
- repo currently has no `packaging/runtime/**`
- repo currently has no `CHANGELOG.md`

## Phase 15 Tasks Status

1. Freeze scope for first public release: ready to do.
2. Pick first public version number: currently `0.1.0` in `Cargo.toml`, but not finalized by maintainer.
3. Finalize changelog entries: blocked by missing `CHANGELOG.md`.
4. Finalize release notes: blocked.
5. Tag release: blocked until maintainer approval and blockers cleared.
6. Run release workflow: blocked until runtime payloads exist.
7. Verify published GitHub assets: blocked until release exists.
8. Verify checksums: blocked until release exists.
9. Verify Homebrew installation: blocked until formula exists.
10. Verify Scoop installation: blocked until manifest exists.
11. Verify winget installation: blocked until manifest exists.
12. Verify standalone binary installation: blocked until release exists.
13. Verify `geocode self-update` on standalone install: blocked until release exists.
14. Verify Homebrew redirect behavior: implementation-level docs exist, install path not yet published.
15. Verify Scoop redirect behavior: implementation-level docs exist, install path not yet published.
16. Verify winget redirect behavior: implementation-level docs exist, install path not yet published.
17. Perform final clean-machine smoke tests: blocked until release artifacts exist.
18. Publish release announcement: blocked.
19. Monitor install issues: blocked until launch.
20. Triage first-wave regressions: blocked until launch.

## Approval Needed Before Real Execution

Explicit maintainer approval needed before:

1. creating git tag
2. pushing tag to remote
3. triggering public release workflow intentionally
4. publishing release announcement

## Fastest Path To Unblock Phase 15

1. add packaged runtime payloads under `packaging/runtime/<target>/`
2. add `CHANGELOG.md`
3. finalize release notes
4. implement Homebrew/Scoop/winget manifests in actual external repos
5. rerun release validation checklist
6. get explicit approval for tag and release
