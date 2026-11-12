# vb-3l40a: Exact State Recovery Test Report

**Bead**: vb-3l40a
**Workspace**: /home/lewis/src/isoloated/velvet-ballistics-vb-3l40a-runtime-5090
**Date**: 2026-10-21
**Agent**: local5090

## Test Implemented

Added `recover_restores_action_contracts_and_schedules` test to `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs` (module `vb_3l40a_action_recovery_tests`).

The test verifies:
1. **Action contract restoration**: After `handle_recover`, the recovered run state's `action_contracts` matches the artifact's `action_contracts` (by count, id, and name)
2. **Pending action scheduling**: After recovery drives to the Do action, `pending_action_get` returns `Some(ticket)` with correct run, step, action, and attempt fields
3. **Store preservation**: `store.clone()` is passed to `Shard::new_with_journal_and_artifact_store` so the original artifact store remains available for comparison

## Key Design Decisions

- **Prebuilt contract**: `ActionName::new` is called outside the trait method using `Result/?` to avoid `unwrap`
- **Input slot seeding**: The recovery frame is seeded with the required input value for the Do action using `frame.write_slot`
- **Explicit assertions**: Each assertion includes a descriptive message indicating exactly what failed
- **Mutation resistance**: The test catches the regression mutation `Box::new([])` vs `artifact.action_contracts.clone()`

## Mutation Testing Results

**Mutation applied**: `action_contracts: artifact.action_contracts.clone()` → `action_contracts: Box::new([])`

**Result**: Test FAILED (as expected)
- Error: "run state not found after recovery; recovery failed to construct or restore state"
- With empty action contracts, the drive fails when resolving the contract for the Do action, causing `apply_terminal_failed` to remove the run state

**Restoration**: After restoring the original code, the test passes.

## Evidence

- Mutation failure log: `.beads/vb-3l40a/evidence/exact-state/mutation-failure-improved.log`
- Production file location: `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs` line 481
- Test file location: `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs` (test module at end of file)

## Commands Used

```bash
# Test execution
RUSTC_WRAPPER= CARGO_BUILD_JOBS=2 timeout 300 cargo test -p vb_runtime \
  recover_restores_action_contracts_and_schedules --lib -- --test-threads=1

# Mutation testing
sed -i 's/action_contracts: artifact.action_contracts.clone(),/action_contracts: Box::new([]),/' \
  crates/vb_runtime/src/shard/lifecycle/chunk_001.rs
```

## Conclusion

The test proves that:
1. Recovery restores action contracts from the artifact
2. The restored contracts enable the run to drive to the Do action
3. The Do action is correctly scheduled as a pending action ticket

This closes the critical gap identified in the test suite review where no test would catch the empty action contracts regression.
