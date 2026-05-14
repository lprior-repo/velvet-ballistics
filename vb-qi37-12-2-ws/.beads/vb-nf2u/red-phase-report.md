# Red Phase Report: vb-nf2u

## Files changed
- Added `tests/vb_nf2u_ui_release_acceptance.rs`.
- Added this report at `.beads/vb-nf2u/red-phase-report.md`.

## Tests added
- `test_all_eight_screens_pass_reachability_and_overlap_gates`
- `test_secret_values_are_redacted_in_every_screen`
- `test_intentional_overlap_fixture_fails_gate`
- `test_intentional_secret_fixture_fails_redaction_gate`

All four tests use `cargo xtask ai-release --bead vb-nf2u` as the acceptance boundary. Helper code only removes stale evidence, prepares test-only negative fixture marker files under `target/vb-nf2u-negative-fixtures/`, serializes access to the shared `.evidence/vb-nf2u` path, and reads emitted artifacts.

## Command run

```text
rm -rf "target/vb-nf2u-acceptance.lock" && cargo nextest run --test vb_nf2u_ui_release_acceptance
```

## Result

```text
Finished `test` profile [unoptimized + debuginfo] target(s) in 0.15s
Nextest run ID ef292eeb-1632-4c10-8bfd-929de12b1d8b with nextest profile: default
Starting 4 tests across 1 binary
FAIL velvet-ballastics-workspace::vb_nf2u_ui_release_acceptance test_secret_values_are_redacted_in_every_screen
FAIL velvet-ballastics-workspace::vb_nf2u_ui_release_acceptance test_intentional_secret_fixture_fails_redaction_gate
FAIL velvet-ballastics-workspace::vb_nf2u_ui_release_acceptance test_intentional_overlap_fixture_fails_gate
FAIL velvet-ballastics-workspace::vb_nf2u_ui_release_acceptance test_all_eight_screens_pass_reachability_and_overlap_gates
Summary [0.373s] 4 tests run: 0 passed, 4 failed, 0 skipped
error: test run failed
```

Representative failure from every acceptance test:

```text
assertion `left == right` failed: expected `cargo xtask ai-release --bead vb-nf2u` to succeed and emit UI release evidence
stderr:
Running `target/debug/xtask ai-release --bead vb-nf2u`
Error: Subcommand not found: 'run_profile'
left: Some(1)
right: Some(0)
```

## Expected RED failures
- `ai-release` currently exits non-zero before any UI evidence is emitted because `xtask::evidence::run_profile` is still a red-phase stub returning `Subcommand not found: 'run_profile'`.
- Therefore the four acceptance tests fail at the required command boundary before artifact-shape assertions run.
- These failures are expected in State 5 RED and expose the missing `ai-release` UI release-gate implementation, including omitted UI snapshot, layout/readability, redaction, negative-fixture, deterministic-capture, and evidence-shape subgates.

## Self-audit
- Zero production code modified.
- No `#[ignore]` added.
- No sole `is_ok()`/`is_err()` assertions added.
- The required four bead-named acceptance tests exist in `tests/vb_nf2u_ui_release_acceptance.rs`.
