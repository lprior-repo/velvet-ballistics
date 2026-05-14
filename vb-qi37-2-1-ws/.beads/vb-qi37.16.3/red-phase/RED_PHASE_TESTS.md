# Red-Phase Tests: Durable Retry vb-qi37.16.3
# These tests define the expected behavior for durable retry transitions.
# They are written in RED-phase style: tests FAIL until production code implements the behavior.
# Evidence: See red-phase-evidence.md for command execution evidence.

## Test Coverage Summary

| Contract Clause | Test Name | Expected Behavior | Red-Phase Status |
|-----------------|-----------|------------------|------------------|
| PRE-002 | `validate_ticket_attempt_rejects_zero_attempt` | attempt=0 returns AttemptBeyondMax | FAIL - validate_ticket_attempt is private |
| PRE-002 | `validate_ticket_attempt_rejects_zero_capacity` | capacity=0 returns AttemptBeyondMax | FAIL - validate_ticket_attempt is private |
| PRE-002 | `validate_ticket_attempt_rejects_attempt_beyond_capacity` | attempt>capacity returns AttemptBeyondMax | FAIL - validate_ticket_attempt is private |
| PRE-002 | `validate_ticket_attempt_accepts_valid_attempt` | valid 1<=attempt<=capacity returns Ok | FAIL - validate_ticket_attempt is private |
| PRE-004 | `retry_is_available_requires_retryable_policy` | NonRetryable returns false | FAIL - not yet exposed |
| PRE-004 | `retry_metadata_exists_when_retry_check_follows` | metadata exists when RetryCheck follows | FAIL - helpers is private |
| POST-001 | `apply_action_failure_to_state_sets_pc_to_failed_step_on_retry` | PC reset to failed step on RetryNow | FAIL - not yet exposed |
| POST-002 | `apply_error_handler_writes_error_slot_and_sets_pc_to_handler` | error slot written, PC=handler | FAIL - existing test incomplete |
| POST-003 | `apply_error_handler_returns_fail_run_when_no_handler` | FailRun when no handler | PASS - already implemented |
| POST-004 | `action_failure_emits_action_failed_journal_event` | exactly one ActionFailed in journal | PASS - already implemented |
| POST-005 | `ticket_with_retry_capacity_returns_unchanged_when_no_retry_metadata` | ticket unchanged when no metadata | FAIL - ticket_with_retry_capacity is private |
| POST-005 | `ticket_with_retry_capacity_increases_capacity_to_max_attempts` | capacity = max(capacity, max_attempts) | FAIL - ticket_with_retry_capacity is private |
| POST-006 | `record_retry_attempt_increments_and_allows_retry` | counter increments below max | PASS - already implemented |
| POST-006 | `record_retry_attempt_blocks_when_max_reached` | returns false at max, no increment | PASS - already implemented |
| POST-007 | `stale_attempt_completion_leaves_run_counters_journal_and_frame_unchanged` | stale attempt rejected, state unchanged | PASS - already implemented |
| POST-007 | `future_attempt_completion_rejected_when_current_attempt_exists` | gap attempt rejected | PASS - already implemented |
| INV-001 | `record_scheduled_attempt_records_first_attempt` | action_attempts[step]=1 on first schedule | PASS - already implemented |
| INV-001 | `record_scheduled_attempt_updates_higher_attempt` | counter updates to max | PASS - already implemented |
| INV-002 | `retry_exhaustion_journal` | max_attempts failures -> RunFailed | PASS - already implemented |
| INV-003 | `journal_replay_idempotent_action_failed` | duplicate ActionFailed same state | FAIL - journal replay not exposed |
| INV-004 | `slot_preservation_on_action_failure` | ActionCompleted slots not overwritten | FAIL - not yet tested |
| INV-005 | `pc_reset_semantics_on_retry` | PC reset to failed step (not advanced) | FAIL - not yet exposed |

## Red-Phase Failures to Implement

### 1. `validate_ticket_attempt` is private and not tested directly
The `validate_ticket_attempt` function in `helpers.rs` is `pub(crate)` but only accessible through `validate_action_completion`. The test plan requires direct unit tests for PRE-002 ticket bounds validation.

Command to verify:
```
cargo test -p vb_runtime helpers::tests::validate_ticket_attempt --lib -- --nocapture
```

Expected: Tests for zero attempt, zero capacity, attempt>capacity should FAIL because `validate_ticket_attempt` is not public.

### 2. `retry_is_available` is private
The `retry_is_available` function in `lifecycle.rs` is private and not directly testable.

Command to verify:
```
cargo test -p vb_runtime lifecycle::tests::retry_is_available --lib -- --nocapture
```

Expected: FAIL - retry_is_available not exposed for direct testing.

### 3. `apply_action_failure_to_state` is private
The `apply_action_failure_to_state` function is private and returns `ActionFailureOutcome` which is also private.

Command to verify:
```
cargo test -p vb_runtime apply_action_failure_to_state --lib -- --nocapture
```

Expected: FAIL - function not exposed for testing.

### 4. `ticket_with_retry_capacity` is private
The `ticket_with_retry_capacity` function is private.

Command to verify:
```
cargo test -p vb_runtime ticket_with_retry_capacity --lib -- --nocapture
```

Expected: FAIL - function not exposed for testing.

### 5. Journal replay idempotency not testable
The journal replay functionality is not exposed as a testable interface.

Command to verify:
```
cargo test -p vb_runtime journal_replay --lib -- --nocapture
```

Expected: FAIL - no journal replay test exists.

### 6. Slot preservation INV-004 not tested
No test exists to verify that ActionCompleted slot values are not overwritten by ActionFailed.

Command to verify:
```
cargo test -p vb_runtime slot_preservation --lib -- --nocapture
```

Expected: FAIL - test does not exist.

### 7. PC reset semantics INV-005 not exposed
The PC reset semantics (INV-005) require Verus proofs and are not directly testable via cargo test.

Command to verify:
```
cargo test -p vb_runtime apply_action_failure_to_state_sets_pc --lib -- --nocapture
```

Expected: FAIL - no test exists for PC reset behavior.

## Implementation Notes

The following issues prevent full test coverage of the durable retry contract:

1. **Privacy barriers**: Key functions (`validate_ticket_attempt`, `retry_is_available`, `apply_action_failure_to_state`, `ticket_with_retry_capacity`) are private and not directly testable.

2. **ActionFailureOutcome is private**: The enum variant returned by `apply_action_failure_to_state` is not public, preventing assertion on retry outcomes.

3. **Journal replay not exposed**: The journal replay functionality is embedded in the shard and not accessible for idempotency testing.

4. **No public helpers module**: The `helpers` module is `pub(crate)` so tests outside the crate cannot use it directly.

## Next Steps (Green Phase)

To make these tests pass, the following changes are needed:

1. Make `validate_ticket_attempt` public or add public wrapper functions
2. Make `retry_is_available` public for direct testing
3. Make `apply_action_failure_to_state` return a public enum
4. Make `ticket_with_retry_capacity` public
5. Add journal replay test interface
6. Add slot preservation test for INV-004
7. Expose PC state inspection for INV-005 verification