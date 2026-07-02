bead_id: vb-vcmq
bead_title: quality: provide public API evidence tooling
phase: 11
updated_at: 2026-05-18T21:14:50Z
attempt: 1-of-7

STATUS: PASS
Focused public API gate:
- `rustup run nightly-2026-04-28 cargo public-api --version` -> PASS, cargo-public-api 0.51.0.
- `rustup run nightly-2026-04-28 cargo public-api --workspace --all-features` -> exit 2; upstream unsupported `--workspace`; raw log `.beads/vb-vcmq/public-api-workspace-unsupported.log`; covered by WVR-API-001.
- Per library package loop using `cargo metadata` and `cargo public-api -p <package> --all-features` -> PASS for 20 packages; raw log `.beads/vb-vcmq/public-api-per-package.log`; grep evidence found 0 nonzero exit markers and 20 `[exit=0]` markers.
Canonical verify classification:
- `moon run :verify-standard` -> exit 1 due `GATE-IGNORED-FALLIBLE-RESULTS` in crates/vb_storage/src/kani_recovery_hydrate.rs lines 78 and 111; raw log `.beads/vb-vcmq/verify-standard.log`; classified DEFERRED_GLOBAL/known separate blocker vb-ybi5 because this bead made no source changes.
Semver-checks workspace probe:
- `cargo semver-checks check-release --workspace --baseline-rev origin/main` -> exit 1 because jj workspace lacks git repository; raw log `.beads/vb-vcmq/semver-checks-workspace.log`; not used as pass evidence.
