# Code Review: vb-2b170 Strict Callback Probe Test

**Date:** 2026-10-21
**Commit:** cc8bb9ab724ed4d408f0a41e1308962c40dd6366
**Reviewer:** local5090-ninfer/qwen3.8-27b-uncensored
**Status:** APPROVED

## Summary

The frozen commit moves the ticket authority probe test from `crates/vb_runtime/tests/` to `crates/workspace_tests/tests/` with significant assertion strengthening. The new test is explicit in `crates/workspace_tests/Cargo.toml` at lines 198-200.

## Findings

### Strengths

1. **Exact typed error assertions** (lines 294-307): Asserts `RuntimeError::StaleAttempt { incoming: 1, current: 3 }` with field values, replacing old `is_err()` smoke check.

2. **Queued failure tick rejection** (lines 336-347): Asserts `RuntimeError::InvalidActionCompletion` exact type from `tick_all()`, not just "tick completed OK despite stale failure queue".

3. **Full serialized journal vector equality** (lines 285-318, 352-358): Uses `serialize_event_vector()` to encode entire `JournalEvent` slice as one postcard value, then asserts byte-identical before/after. This verifies record boundaries are preserved (not unframed concat or count >=), directly satisfying the "no state mutation" invariant.

4. **Public snapshot state equality** (lines 277-377): Captures `Runtime::snapshot_run(run, correlation)` before stale rejection and after stale tick, asserts full `InspectSnapshot` equality. This verifies the public inspect surface is unchanged by stale events.

5. **Option equality for pending ticket** (lines 321-327, 361-367): Uses full `Option<ActionTicket>` equality to verify the current pending ticket is present and unchanged after both stale completion rejection and stale failure tick.

6. **Actual first ticket retained** (lines 214-215): Uses the real first-run `ActionTicket` directly as the stale ticket (leverages `Copy` trait), rather than manually crafting a ticket with matching fields. This is the actual production code path.

7. **Specific completion event verification** (lines 402-424): Asserts `RunFinished` event matches `run` and `result` slot 1, plus `ActionCompletedEnvelope` matches `current_ticket`, output slot 1, and value `I64(42)`.

8. **Pending ticket disappearance** (lines 426-432): Verifies `still_pending.is_none()` after successful completion.

9. **No forbidden constructs**: No `unwrap`, `expect`, `panic`, indexing, unchecked arithmetic, or ignored results. All results use `?` or explicit match.

### Test Suite Coverage

All 6 original cases preserved:
- `stale_completion_rejected_after_two_retries` (strengthened)
- `future_run_attempt_ticket_rejected`
- `zero_run_attempt_ticket_rejected`
- `cold_recovery_preserves_run_attempt`
- `idempotency_key_differs_across_run_attempts`
- `ticket_key_valid_after_serialization`

### Public API Limitation

The public `Runtime::snapshot_run()` returns `InspectSnapshot` which contains public surface fields (run, correlation, pc, executed, etc.) but does not expose every private run state field. The test correctly limits its claims to public snapshot fields, which is appropriate for a public API probe. Full private state verification would require different test infrastructure.

### Cargo.toml Entry

Explicit `[[test]]` registration at lines 198-200 of `crates/workspace_tests/Cargo.toml` follows workspace convention. No root or old duplicate test files.

## Verification

Independent rerun via `vb-run-gate.py` completed in 16.9s:
```
running 6 tests
test idempotency_key_differs_across_run_attempts ... ok
test ticket_key_valid_after_serialization ... ok
test zero_run_attempt_ticket_rejected ... ok
test future_run_attempt_ticket_rejected ... ok
test cold_recovery_preserves_run_attempt ... ok
test stale_completion_rejected_after_two_retries ... ok
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## STATUS: APPROVED

The test meets all specified criteria: exact typed error assertions, full serialized journal vector equality (not just count >=), public snapshot equality, Option equality for pending ticket, specific completion event verification, all 6 original cases preserved, no forbidden constructs, and explicit Cargo.toml registration.