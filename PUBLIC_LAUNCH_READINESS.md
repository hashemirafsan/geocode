# Public Launch Readiness

## Status

Phase 14 launch and support checklist.

## First Announcement Checklist

Before public announcement:

1. GitHub release assets published for all Tier 1 targets.
2. checksums published.
3. release metadata JSON published.
4. Homebrew tap update live.
5. Scoop bucket update live.
6. winget manifest submitted or live.
7. `README.md` matches shipped CLI behavior.
8. `RELEASE_VALIDATION.md` signoff complete.
9. `geocode doctor` and `geocode self-update` verified on packaged installs.
10. release notes include breaking-change/install notes if needed.

## Post-Release Support Checklist

After announcement:

1. monitor install failures by channel
2. monitor helper/runtime lookup failures
3. monitor package-manager update lag
4. monitor self-update failures on standalone installs
5. update troubleshooting docs if repeated issue appears
6. cut patch release quickly for packaging regressions

## Support Triage Order

1. confirm install method
2. collect `geocode doctor`
3. collect `geocode version`
4. confirm target platform and architecture
5. check release notes for known issue or breaking change
