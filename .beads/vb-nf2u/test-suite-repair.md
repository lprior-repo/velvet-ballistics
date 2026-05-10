STATUS: RED

# Test Suite Repair: vb-nf2u

## Files repaired/added
- Rewrote `tests/vb_nf2u_ui_release_acceptance.rs` to remove banned Tier 0 patterns: no `test_` names, no sleeps, no silent cleanup discard, no explicit assertion-hiding loops, no evidence tree traversal loop.
- Added exact per-screen acceptance assertions for eight canonical screens, seven checks per screen, six UI subgates, deterministic/fixture-backed/no-parity evidence, and six redaction classes per screen.
- Added command-boundary negative fixture consumption tests for overlap and secret fixtures. These intentionally fail because the implementation currently emits hard-coded negative fixture evidence instead of consuming `target/vb-nf2u-negative-fixtures/*`.
- Added false-pass detector tests for overlap and secret fixtures. These intentionally fail because current evidence never reports `FalsePassFixtureViolation`.
- Added planned test files where practical:
  - `crates/vb_ui_snapshot/tests/inventory_bijection.rs`
  - `crates/vb_ui_snapshot/tests/layout_checks.rs`
  - `crates/vb_ui_snapshot/tests/redaction_checks.rs`
  - `crates/vb_ui_snapshot/tests/report_evidence_shape.rs`
  - `crates/vb_ui_makepad/tests/shell_reachability.rs`
  - `xtask/tests/ui_release_gates.rs`
  - `xtask/tests/ui_release_errors.rs`
  - `xtask/tests/ui_release_tooling_red_phase.rs`
  - `fuzz/fuzz_targets/ui_redaction_artifact.rs`
  - `crates/vb_ui_snapshot/kani/inventory.rs`
  - `crates/vb_ui_snapshot/kani/layout_predicates.rs`

## RED failures proving implementation repair is still required

### Acceptance command
Command:
```text
cargo nextest run -p velvet-ballastics-workspace --test vb_nf2u_ui_release_acceptance
```
Result:
```text
8 tests run: 4 passed, 4 failed, 0 skipped
FAIL secret_false_pass_fixture_is_rejected
FAIL secret_negative_fixture_is_consumed_by_command_boundary
FAIL overlap_negative_fixture_is_consumed_by_command_boundary
FAIL overlap_false_pass_fixture_is_rejected
```
Meaning: positive acceptance evidence is now stronger and passes, but the command boundary still ignores negative fixture file contents and lacks false-pass detector evidence.

### Package command
Command:
```text
cargo nextest run -p vb_ui_snapshot -p vb_ui_makepad -p xtask
```
Result:
```text
32/82 tests run before fail-fast: 12 passed, 20 failed, 0 skipped
```
Representative failures:
```text
invalid_screen_inventory_error_type_is_missing_from_release_gate_api: source lacks UiReleaseGateError / InvalidScreenInventory
layout_violation_release_gate_error_type_is_exposed: source lacks UiReleaseGateError::LayoutViolation
redaction_violation_release_gate_error_type_is_exposed: source lacks UiReleaseGateError::RedactionViolation
overlap_violation_error_returns_contract_shape_when_controls_intersect: current check_overlap returns Ok(0) instead of exact OverlapDetected diagnostic
ui_snapshot_returns_*: current UiSnapshotError variants are tuple/legacy-shaped, not the exact fielded variants required by test-plan lines 315-403
```

## Planned lanes still blocked
- `UiReleaseGateError` exact variant tests are runtime RED source-presence tests because no production `UiReleaseGateError` type is exposed.
- `UiSnapshotError` exact tests are runtime RED debug-shape tests because variants exist but do not expose the field names/diagnostics required by the approved plan.
- Kani files were added as harness artifacts, but the workspace has no command wiring for `cargo kani -p vb_ui_snapshot inventory` / `layout_predicates`.
- Fuzz target file was added, but the fuzz crate is not yet wired with a `ui_redaction_artifact` bin entry or redaction scanner API.

## Static repair notes
- Tier 0 acceptance-specific banned patterns from the rejection are removed from `tests/vb_nf2u_ui_release_acceptance.rs`.
- No production implementation files were modified.
