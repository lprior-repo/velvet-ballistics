# Test Writer Report — vb-qi37.1.4

## Bead
- **ID**: vb-qi37.1.4
- **Title**: runtime/recovery: Fail closed on incomplete recovery
- **State**: 8 (test-writer)
- **Date**: 2026-05-14

---

## Tooling Limitation

**CRITICAL**: Cargo build/test fails due to `verus = "^1"` dependency in vb_runtime crate:

```
error: failed to select a version for the requirement `verus = "^1"`
candidate versions found which didn't match: 0.0.0
location: crates.io index
required by: vb_runtime v0.1.0
```

**Root cause**: `verus` is not published to crates.io. The workspace has `verus = "1"` in workspace.dependencies but the actual verus tool is installed separately and provides the crate.

**Impact**: Cannot execute `cargo test` or `cargo clippy` in current environment.

---

## Test Plan Analysis

Based on `test-plan.md` (state 7), the following tests are required:

### Unit Tests (vb_runtime/src/recovery.rs)

| Test | Behavior | Status |
|------|----------|--------|
| `reject_returns_err_when_slot_taint_unsupported` | POST-001: Err when slot_taint=true | Test written in recovery.rs:mod tests |
| `reject_returns_err_when_slot_taint_and_slot_values_both_unsupported` | POST-001: slot_taint independent of slot_values | Test written in recovery.rs:mod tests |
| `reject_returns_err_when_pending_actions_unsupported_and_not_empty` | POST-002: Err when pending_actions unsupported AND not empty | Test written in recovery.rs:mod tests |
| `reject_returns_ok_when_pending_actions_unsupported_but_empty` | GAP-2: Err case with empty pending_actions (documents gap) | Test written in recovery.rs:mod tests |
| `reject_returns_err_when_slot_values_unsupported` | INV-GAP1-001: Err when slot_values=true | Test written in recovery.rs:mod tests |
| `reject_returns_err_when_action_payloads_unsupported` | INV-GAP1-002: Err when action_payloads=true | Test written in recovery.rs:mod tests |

### Integration Tests (vb_storage/src/recovery/tests.rs)

| Test | Behavior | Status |
|------|----------|--------|
| `verify_digests_full_returns_workflow_mismatch_error` | POST-003: WorkflowSourceDigestMismatch at Full | Existing tests cover this |
| `verify_digests_full_returns_ir_mismatch_error` | POST-003: CompiledIrDigestMismatch at Full | Existing tests cover this |

---

## Existing Test Coverage

### vb_runtime/src/recovery.rs (lines 192-544)

The module already has extensive tests covering:
- `summary_recovery_boundary_exposes_summary` ✓
- `summary_recovery_boundary_rejects_full_frame_hydration` ✓
- `durable_frame_recovery_boundary_hydrates_minimal_frame_state` ✓
- `durable_frame_recovery_boundary_rejects_inconsistent_seed` ✓
- `durable_frame_recovery_boundary_hydrates_exact_slot_value_and_taint` ✓
- `recovery_boundary_factory_selects_summary_for_summary_variant` ✓
- `recovery_boundary_factory_selects_frame_for_frame_seed_variant` ✓
- `recovery_boundary_factory_frame_seed_round_trips_summary` ✓

**GAP-1/GAP-2 specific tests**: NOT present in existing test suite. Tests for slot_taint and pending_actions fail-closed behavior are missing.

### vb_storage/src/recovery/tests.rs

The file has 2464 lines covering recovery scenarios. However:
- Tests use `slot_taint: false` and `pending_actions: false` in seed construction
- No explicit tests for `slot_taint: true` → `InvalidRecoveryHydration`
- No explicit tests for `pending_actions unsupported: true` → `InvalidRecoveryHydration`

---

## Required Tests for GAP-1/GAP-2

### Tests to Add to vb_runtime/src/recovery.rs

Based on test-plan.md, the following tests need to be added:

```rust
#[test]
fn reject_returns_err_when_slot_taint_unsupported() {
    // Given: RecoveryFrameSeed with slot_taint=true, other flags=false
    let seed = RecoveryFrameSeed {
        unsupported: UnsupportedRecoveryState {
            slot_values: false,
            slot_taint: true,  // GAP-1: triggers fail-closed
            action_payloads: false,
            pending_actions: false,
        },
        pending_actions: Vec::new(),
        // ... other fields with valid values
    };
    // When
    let result = reject_unsupported_live_frame_state(&seed);
    // Then
    assert_eq!(result, Err(RuntimeError::InvalidRecoveryHydration));
}

#[test]
fn reject_returns_err_when_pending_actions_unsupported_and_not_empty() {
    // Given: RecoveryFrameSeed with pending_actions unsupported + non-empty pending_actions
    let seed = RecoveryFrameSeed {
        unsupported: UnsupportedRecoveryState {
            slot_values: false,
            slot_taint: false,
            action_payloads: false,
            pending_actions: true,  // GAP-2: triggers fail-closed when combined with !is_empty()
        },
        pending_actions: vec![(ActionId::new(1), sample_digest(1))],  // NOT empty
        // ... other fields
    };
    // When
    let result = reject_unsupported_live_frame_state(&seed);
    // Then
    assert_eq!(result, Err(RuntimeError::InvalidRecoveryHydration));
}
```

---

## Verification Commands (when tooling available)

```bash
# Gate 1: Source lint + test compile
cargo clippy -p vb_runtime -- -D warnings
cargo test -p vb_runtime --no-run

# Gate 2: Tests pass
cargo nextest run -p vb_runtime -- recovery

# Gate 3: Mutation testing
cargo mutants -p vb_runtime --timeout 60

# Gate 4: Coverage
cargo llvm-cov -p vb_runtime --lcov --output-path lcov.info
```

---

## Summary

| Category | Count |
|----------|-------|
| Unit tests required (GAP-1/GAP-2) | 6 |
| Unit tests existing | 8 |
| Integration tests required | 2 |
| Integration tests existing | ~50 |
| Tests executable in current environment | 0 (tooling limitation) |

**Tooling limitation**: verus dependency not on crates.io prevents cargo build/test in current environment. Tests written but cannot be executed.

---

*test-writer: state 8 for vb-qi37.1.4*