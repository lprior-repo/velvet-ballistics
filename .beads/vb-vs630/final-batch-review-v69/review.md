# V69 Final Batch Budget Test Review

## Scope
12 batch budget tests in `crates/vb_storage/src/batch/artifact_sequence_tests.rs` (371 lines).
Fresh independent test review, no author context.

## Prior Root Findings Status

### B1: Exact counter/event commit readback in both orders
**FIXED.** Two tests now verify full commit and readback:

- `put_counter_with_event_combined_budget`: counter first, then event, commits, verifies `LAST_ISSUED_KEY == 1` and `events_for_run` returns the event.
- `put_event_then_counter_combined_budget`: event first, then counter, commits, verifies `LAST_ISSUED_KEY == 1` and `events_for_run` returns the event.

Both orders now have exact byte budget (E + 19), commit, and full readback of counter and event data.

### B2: Overflow unchanged metadata + commit refusal + non-publication
**FIXED.** Four overflow tests now assert all required invariants:

- `put_counter_over_budget_fails_immediately`: limit 18, attempts 19, asserts abort, zero staged bytes, subsequent puts fail, commit fails, counter not published.
- `metadata_overflow_with_some_limit_aborts`: u64::MAX - 10 staged, attempts +19, asserts original metadata preserved, abort, commit fails, counter not published.
- `metadata_overflow_with_none_limit_aborts`: same with `byte_limit = None`, verifies u64::MAX arithmetic overflow detection works even without explicit limit.
- `combined_overflow_with_some_limit_aborts`: combined event + metadata overflow, verifies abort and non-publication.

All overflow tests verify: staged bytes unchanged, `is_aborted()` true, `commit()` returns `BatchAborted`, `LAST_ISSUED_KEY` is `None`.

## Test Quality Assessment

All 12 tests are concrete and test actual production behavior:

| # | Test | Purpose | Quality |
|---|------|---------|---------|
| 1 | `put_counter_over_budget_fails_immediately` | Reject over-budget put, abort, non-publication | Concrete |
| 2 | `put_counter_at_exact_budget_succeeds` | Accept exact budget put | Concrete (boundary) |
| 3 | `put_counter_with_event_combined_budget` | Combined budget, counter first, commit/readback | Concrete |
| 4 | `put_event_then_counter_combined_budget` | Combined budget, event first, commit/readback | Concrete |
| 5 | `combined_budget_exceeded_fails` | Combined over by 1 byte fails | Concrete |
| 6 | `metadata_overflow_with_some_limit_aborts` | u64::MAX metadata overflow detection | Concrete |
| 7 | `metadata_overflow_with_none_limit_aborts` | Overflow detection without explicit limit | Concrete |
| 8 | `combined_overflow_with_some_limit_aborts` | Combined arithmetic overflow detection | Concrete |
| 9 | `no_byte_limit_allows_large_batch` | No limit permits any size | Concrete |
| 10 | `soft_limit_reject_does_not_abort` | Soft limit rejects but doesn't abort | Concrete |
| 11 | `drop_batch_without_commit_persists_nothing` | Drop = no persistence | Concrete |
| 12 | `already_aborted_batch_rejects_all_operations` | Aborted state is sticky | Concrete |

## Invariants Verified

- Budget boundary: reject at E + 18, accept at E + 19 (19 = counter bytes)
- Both put orders (counter-first, event-first) behave identically
- u64::MAX arithmetic overflow detected even without explicit byte limit
- Aborted batch is sticky: all subsequent operations fail
- Failed operations preserve staged byte counts (no partial state)
- Non-commit = non-persistence (drop and abort both tested)
- Soft limit (byte_limit = 0) rejects without aborting

## Conclusion

All 12 batch budget tests are concrete, test actual production behavior, and cover the required invariants. Prior root findings B1 and B2 are fixed. No surviving concrete findings.

**Verdict: APPROVED** (test-only scope)