# Test Plan: vb-qi37.12.4

## Summary

- Behaviors identified: 5
- Trophy allocation: 2 static / 3 integration / 0 e2e
- Proptest invariants: 0, direct shell/static gate scope only
- Fuzz targets: 0, no parser/deserializer added
- Kani harnesses: existing standard Kani compile obligations via `moon run :verify-standard`

## Behavior Inventory

1. Direct gate reports ignored bare fallible calls when present.
2. Direct gate reports `let _ =`, `.ok()/.err()`, empty `Err` handlers, `drop(fallible())`, and undocumented allow markers.
3. Direct gate rejects malformed and overbroad allow entries.
4. Production/test source has no unhandled `DISCARD-*` violations after repair.
5. `moon run :verify-standard` invokes the direct gate and fails closed before downstream verification if it fails.

## BDD Scenarios

### Scenario: direct_gate_rejects_discard_patterns
Given: gate self-test fixtures containing DISCARD-001 through DISCARD-006.
When: `scripts/check-ignored-fallible-results.sh` runs.
Then: each bad fixture exits 2 and the script prints `FixturePass` for every discard class.

### Scenario: direct_gate_rejects_bad_exceptions
Given: allow-file fixtures with `ALL` class and missing fields.
When: `scripts/check-ignored-fallible-results.sh` runs.
Then: overbroad and malformed exceptions exit 3.

### Scenario: direct_gate_accepts_clean_workspace
Given: repaired production/test source.
When: `scripts/check-ignored-fallible-results.sh` scans `crates/*/src` and `xtask/src`.
Then: stdout ends with `NoViolationFound` and exit is 0.

### Scenario: verify_standard_propagates_gate
Given: Moon task `:verify-standard`.
When: `moon run :verify-standard` runs.
Then: `GATE-IGNORED-FALLIBLE-RESULTS` executes before lint/unit/Kani lanes and all standard checks pass.

### Scenario: repaired_call_sites_handle_results
Given: touched runtime, IPC, storage, UI, and CLI packages.
When: affected package tests run.
Then: test commands pass or known excluded-UI baseline compile debt is reported separately.

## Mutation Checkpoints

- Removing the gate invocation from `rust-verification-gauntlet.sh` must be caught by absence of `GATE-IGNORED-FALLIBLE-RESULTS` in `moon run :verify-standard` evidence.
- Replacing explicit result assertions with `.ok()` or `let _ =` must be caught by `scripts/check-ignored-fallible-results.sh`.
- Weakening allow-file validation to accept `ALL` must be caught by the overbroad exception fixture.

## Coverage Matrix

| Scenario | Input Class | Expected Output | Layer |
| --- | --- | --- | --- |
| DISCARD fixtures | bad source fixtures | exit 2 per class | static/integration |
| Exception fixtures | malformed/overbroad allow rows | exit 3 | static/integration |
| Clean scan | repaired workspace | `NoViolationFound`, exit 0 | static/integration |
| Verify standard | Moon task | all standard lanes pass | integration |
| Affected tests | touched packages | pass; excluded UI debt isolated | integration |
