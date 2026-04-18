# Release Process

## Status

Maintainer guide for Phase 7 CI and release workflows.

## Workflows

- `CI`: runs on push to `main` and pull requests
- `Release`: runs on version tags matching `v*.*.*`

## Before Tagging Release

1. Ensure version in `Cargo.toml` matches intended tag without leading `v`.
2. Ensure release tag follows `vMAJOR.MINOR.PATCH`.
3. Ensure packaged runtime inputs exist under `packaging/runtime/<target>/` for each supported target:
   - `aarch64-apple-darwin`
   - `x86_64-apple-darwin`
   - `x86_64-unknown-linux-gnu`
   - `x86_64-pc-windows-msvc`
4. Ensure each runtime folder contains required bundled helper binaries and native libraries.
5. Ensure tests pass on `main`.

## Release Workflow Output

Release workflow builds:

- `geocode-vX.Y.Z-aarch64-apple-darwin.tar.gz`
- `geocode-vX.Y.Z-x86_64-apple-darwin.tar.gz`
- `geocode-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz`
- `geocode-vX.Y.Z-x86_64-pc-windows-msvc.zip`
- `geocode-vX.Y.Z-checksums.txt`

## Tagging Release

```bash
git tag v0.1.0
git push origin v0.1.0
```

## Fail-Fast Rules

Release workflow should fail if:

- runtime packaging directory missing for any supported target
- packaged native runtime files missing
- build fails for supported target
- checksum generation fails

## Current Limitation

Workflow assumes native runtime payload is prepared in repo under `packaging/runtime/<target>/`.
Until bundling payload exists, release tags should not be pushed.

## Rollback Or Yank Procedure

If release is bad:

1. Do not reuse or move existing tag.
2. Mark GitHub release as pre-release or draft if issue caught before announcement.
3. If already published, update release notes with clear broken-release warning.
4. Delete or rename package-manager update PRs/manifests that point at bad release.
5. Publish follow-up fixed release with new version tag such as `v0.1.1`.
6. Keep bad tag history intact; do not force-push rewritten tag.

If artifact must be yanked from GitHub Releases:

1. Remove release assets from GitHub release page.
2. Leave short note in release body explaining yank reason.
3. Regenerate package-manager metadata from replacement release only.
4. Validate replacement artifacts before announcement.
