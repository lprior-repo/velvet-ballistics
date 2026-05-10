STATUS: PASS

# State 10 Final Repair: vb-nf2u

## Files changed
- `crates/vb_ui_snapshot/src/report.rs`
- `crates/vb_ui_snapshot/tests/redaction_checks.rs`
- `crates/vb_ui_snapshot/tests/layout_checks.rs`
- `xtask/src/evidence.rs`
- `xtask/tests/ui_release_tooling_red_phase.rs`
- `xtask/tests/integration_gates.rs`

## Repair summary
- Fixed `vb_ui_snapshot::report` clippy blockers by using slice `.first()` and keeping parse/field checks failure-producing without assertion-in-`Result` lint failure.
- Added direct scanner assertions for `idempotency_key` and `tainted_fixture_value` secret classes.
- Replaced source-string-only tooling test with public `ui_release_tooling_lanes()` classification plus gate-profile assertions for lanes executable by the public xtask evidence model.
- Cleaned strict clippy failures surfaced by the required scoped command in adjacent test/evidence code: no unchecked indexing in redaction/layout tests, no unwrap/expect/panic in scoped xtask tests, no ignored cleanup/file-write results, and no UTF-8 string slicing.

## Commands run
- `bd prime` — PASS; Dolt auto-push warned non-fast-forward remote.
- `bd update vb-nf2u --claim` — PASS; Dolt auto-push warned non-fast-forward remote.
- `rtk cargo fmt --all` — PASS.
- `rtk cargo clippy -p vb_ui_snapshot -p vb_ui_makepad -p xtask --tests --all-features -- -D warnings` — PASS: 0 errors, 2 warnings.
- `cargo nextest run -p velvet-ballastics-workspace --test vb_nf2u_ui_release_acceptance` — PASS: 8 run, 8 passed, 0 skipped.
- `cargo nextest run -p vb_ui_snapshot -p vb_ui_makepad -p xtask` — PASS: 131 run, 131 passed, 0 skipped.
- `rtk cargo fmt --all --check` — PASS.

## Residual risks and skipped gates
- Full `moon ci`, fuzz sanitizer execution, supply-chain vetting, Miri, coverage, and mutation were not requested in this final blocker repair pass.
- Clippy still prints dependency/workspace warnings, but the required scoped lint gate exits successfully with zero errors.
- No performance claim was made; no benchmark or profiler was run.
