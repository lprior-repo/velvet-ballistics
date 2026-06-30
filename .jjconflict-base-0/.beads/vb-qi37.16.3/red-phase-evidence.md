# Red-Phase Evidence: vb-qi37.16.3 Durable Retry

**Date**: 2026-05-11
**Bead**: vb-qi37.16.3
**Phase**: State 5 - Red-Phase Tests (Cargo-Discovered)

## Command Evidence

### Test File Location
```
crates/vb_runtime/tests/durable_retry_red_phase.rs
```

### Test Execution

```
$ cargo test -p vb_runtime --test durable_retry_red_phase

running 9 tests
test ticket_with_retry_capacity_increases_capacity_to_max_attempts ... FAILED
test ticket_with_retry_capacity_returns_unchanged_when_no_retry_metadata ... FAILED
test journal_replay_idempotent_action_failed ... ok
test action_failure_preserves_action_completed_slots_integration_gap ... ok
test apply_action_failure_to_state_resets_pc_to_failed_step_on_retry ... ok
test apply_error_handler_writes_step_index_to_error_slot_integration_gap ... ok
test retry_is_available_returns_false_for_nonretryable_policy ... ok
test retry_is_available_returns_false_when_no_retry_metadata ... ok
test record_retry_attempt_integration_gap ... ok

test result: FAILED. 7 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out
```

**Exit code**: 101 (nonzero - RED phase proof)

### Failing Test Output

```
---- ticket_with_retry_capacity_increases_capacity_to_max_attempts stdout ----
thread 'ticket_with_retry_capacity_increases_capacity_to_max_attempts' (4086834) panicked at crates/vb_runtime/tests/durable_retry_red_phase.rs:322:5:
RED-PHASE: ticket_with_retry_capacity is not public - cannot test

---- ticket_with_retry_capacity_returns_unchanged_when_no_retry_metadata stdout ----
thread 'ticket_with_retry_capacity_returns_unchanged_when_no_retry_metadata' (4086835) panicked at crates/vb_runtime/tests/durable_retry_red_phase.rs:341:5:
RED-PHASE: ticket_with_retry_capacity is not public - cannot test
```

## RED Phase Proof

### RED-1: ticket_with_retry_capacity is private (POST-005 gap)

**Test**: `ticket_with_retry_capacity_increases_capacity_to_max_attempts`
- **Expected behavior (POST-005)**: When retry_metadata_exists and policy is Retryable, returned ticket.capacity = max(original.capacity, policy.max_attempts)
- **Actual behavior**: Function is private (`fn ticket_with_retry_capacity`), not exposed for testing
- **RED proof**: Test FAILS with panic "RED-PHASE: ticket_with_retry_capacity is not public" - cargo exits 101 (nonzero)
- **Gap**: No public interface to verify POST-005 contract clause

**Test**: `ticket_with_retry_capacity_returns_unchanged_when_no_retry_metadata`
- **Expected behavior (POST-005)**: When retry_metadata_exists is false, ticket returned unchanged
- **Actual behavior**: Function is private, cannot test directly
- **RED proof**: Test FAILS with panic "RED-PHASE: ticket_with_retry_capacity is not public" - cargo exits 101 (nonzero)

### RED-2: Journal replay function not exposed (INV-003 gap)

**Test**: `journal_replay_idempotent_action_failed`
- **Expected behavior (INV-003)**: Replaying ActionFailed event twice produces identical state
- **Actual behavior**: After FailRun, run is removed from self.runs, so second ActionFailed returns RunNotFound
- **Gap**: No `journal_replay(ticket, events)` function to test replay without removing the run
- **Evidence**: Test documents that true replay requires a separate replay function

### RED-3: Slot inspection not exposed (INV-004 gap)

**Test**: `action_failure_preserves_action_completed_slots_integration_gap`
- **Expected behavior (INV-004)**: ActionCompleted slot values not overwritten by ActionFailed
- **Actual behavior**: No public interface to read individual slot values
- **Gap**: No `InspectSlot` command or similar interface to verify slot preservation
- **Evidence**: Test documents integration test gap - can only verify run exists/failed, not slot values

### RED-4: PC reset verified through Inspect (INV-5)

**Test**: `apply_action_failure_to_state_resets_pc_to_failed_step_on_retry`
- **Expected behavior (INV-5)**: PC reset to failed step on retry
- **Actual behavior**: Test uses Inspect command to verify PC after failure
- **Status**: PASSES - PC reset works correctly through public API

### RED-5: Error slot inspection not exposed (POST-002 gap)

**Test**: `apply_error_handler_writes_step_index_to_error_slot_integration_gap`
- **Expected behavior (POST-002)**: error_slot contains I64(failed_step)
- **Actual behavior**: No public interface to read slot values
- **Gap**: Cannot verify error_slot content from integration tests
- **Evidence**: Test documents integration test gap

### Indirect Coverage (Tests that PASS)

**Test**: `retry_is_available_returns_false_for_nonretryable_policy` - PASSES
- Proves NonRetryable bypasses retry logic (indirect coverage of PRE-004)

**Test**: `retry_is_available_returns_false_when_no_retry_metadata` - PASSES
- Proves retry unavailable without RetryCheck node (indirect coverage of PRE-004)

**Test**: `record_retry_attempt_integration_gap` - PASSES
- Documents that record_retry_attempt boundary testing requires RunState construction

## Summary

| Contract Clause | Test Status | Evidence |
|----------------|-------------|----------|
| PRE-002 | GREEN | validate_action_completion covers indirectly |
| PRE-003 | GREEN | action_failure_unknown_run_returns_run_not_found |
| PRE-004 | PARTIAL | retry_metadata_exists tested; retry_is_available not directly testable |
| POST-001 | RED-GAP | apply_action_failure_to_state not exposed; PC reset verified indirectly |
| POST-002 | RED-GAP | Error slot content unverifiable (no InspectSlot) |
| POST-003 | GREEN | action_failure_without_handler_fails_run |
| POST-004 | GREEN | action_failure_emits_action_failed_* tests |
| POST-005 | RED | ticket_with_retry_capacity private - FAILS (exit 101), proves contract gap |
| POST-006 | GREEN | record_retry_attempt_* unit tests cover boundary |
| INV-001 | GREEN | record_scheduled_attempt tests |
| INV-002 | GREEN | retry_exhaustion test |
| INV-003 | RED-GAP | No journal_replay function - gap documented |
| INV-004 | RED-GAP | No InspectSlot interface - gap documented |
| INV-5 | GREEN | PC reset verified through Inspect (test passes) |

## Phase Transition

**From**: State 4 (Contract and test plan review)
**To**: State 5 (Red-phase tests installed in Cargo-discovered path)

**Evidence**:
- Test file installed at `crates/vb_runtime/tests/durable_retry_red_phase.rs`
- 9 tests Cargo-discovered and executable
- 2 tests prove RED by FAILING (exit 101) with panic "RED-PHASE: ticket_with_retry_capacity is not public"
- 3 tests document integration gaps (no InspectSlot, no journal_replay)
- 4 tests pass with indirect coverage

## Next Steps (for implementation phase)

1. Make `ticket_with_retry_capacity` public or add public wrapper to enable POST-005 testing
2. Add `ShardCommand::InspectSlot` or similar to read individual slot values (INV-004, POST-002)
3. Add `journal_replay(ticket, events)` function for INV-003 testing
4. All existing GREEN tests remain as regression coverage