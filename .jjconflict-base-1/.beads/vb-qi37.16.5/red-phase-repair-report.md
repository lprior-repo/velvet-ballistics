# Red Phase Repair Report: vb-qi37.16.5

## bead_id: vb-qi37.16.5
## phase: state-6 (red-phase repair)
## repair_attempt: red-phase-test-repair

---

## Verification Command

```bash
rtk cargo test --package velvet_ballistics --test lifecycle_integration -- --test-threads=1
```

## Previous State: 39 passed; 4 failed

## After Repair: 41 passed; 2 failed

---

## Test 1: `answer_returns_stale_request_when_not_in_waiting_answer_state`

**Previous Failure**: Test expected `LifecycleStaleRequest` when calling `answer` from `Completed` state.
**Actual Behavior**: Product returns `LifecycleDuplicateRequest` (correct per POST-004).

**Root Cause**: Test expectation was wrong per contract.

**Contract Analysis**:
- POST-004: `E_DUPLICATE_REQUEST` when same command already processed for this bead
- POST-005: `E_STALE_REQUEST` when bead state has already advanced past expected prior state

For `answer`:
- `Completed` state → already answered → `DuplicateRequest` (POST-004 ✓)
- `Active/Failed/Cancelled` → passed the point where answer is valid but not completed → `StaleRequest` (POST-005 ✓)
- `Pending` → never reached WaitingAnswer → `InvalidTransition` (POST-003 ✓)

**Fix**: Changed state from `Completed` to `Active` so answer triggers `StaleRequest` (passed WaitingAnswer but not Completed).

**STATUS: REPAIRED** ✓

---

## Test 2: `replay_from_empty_journal_produces_valid_initial_state`

**Previous Failure**: `replay()` returned accumulated states from shared in-memory TRACKER (9 runs from previous tests), not empty/Pending state.

**Root Cause**: `temp_journal()` creates fresh journal but does NOT reset the global TRACKER. Previous tests accumulate state in TRACKER, which `replay()` returns.

**Fix**: Added `reset_tracker()` call at start of test to ensure clean tracker state before replay.

**STATUS: REPAIRED** ✓

---

## Test 3: `replay_with_malformed_event_returns_replay_corruption`

**Previous Failure**: Test assertion expected `ReplayCorruption` but got `Ok(states)` (accumulated tracker states).

**Root Cause Analysis**:
1. **Cannot inject corruption**: `journal.events` is `pub(crate)` — inaccessible from tests. Cannot write malformed bytes to keyspace.
2. **replay() doesn't use journal**: `lifecycle::replay()` in minimal implementation returns in-memory tracker state. It takes `_journal` parameter (unused) and never reads from it.

**Contract Conformance**:
- Storage layer `events_for_run_from` DOES validate events via `validate_replayed_event`
- `decode_record` returns `JournalError::PostcardDecodeFailed` or `JournalError::BadMagic` for corrupt data
- `validate_replayed_event` returns `JournalError::SequenceGap` for sequence gaps

**Evidence of Correct Contract Implementation**:
- `validate_replayed_event_rejects_wrong_run` (codec.rs:874) — proven
- `validate_replayed_event_rejects_sequence_gap` (codec.rs:891) — proven
- These prove the corruption detection contract IS implemented at storage layer

**Blockage**: The corruption detection is NOT wired into `lifecycle::replay()` in minimal implementation.

**STATUS: BLOCKED** — Cannot inject corruption (keyspace inaccessible). Cannot test replay corruption (replay doesn't use journal). Product implementation required to wire storage-layer corruption detection into `lifecycle::replay()`.

**Contract-equivalent evidence**: Storage layer corruption detection verified via vb_storage codec tests.

---

## Test 4: `replay_with_missing_event_returns_replay_corruption`

**Previous Failure**: Test assertion expected `ReplayCorruption` but got `Ok(states)`.

**Root Cause Analysis**:
1. **Cannot create sequence gap**: `append_journaled` doesn't enforce sequential sequences. Even if it did, we can't write raw records (keyspace inaccessible).
2. **replay() doesn't use journal**: Same as Test 3 — minimal impl returns tracker state, ignores journal.

**Contract Conformance**:
- Storage layer `events_for_run_from` validates sequence continuity
- `validate_replayed_event` returns `JournalError::SequenceGap` when seq doesn't match expected

**Evidence of Correct Contract Implementation**:
- `validate_replayed_event_rejects_sequence_gap` (codec.rs:891) — proven
- When gap exists (e.g., seq=0, seq=2 missing seq=1), `events_for_run_from` returns `SequenceGap`

**Blockage**: Gap creation blocked (keyspace inaccessible, API doesn't enforce seq ordering). `replay()` doesn't use journal.

**STATUS: BLOCKED** — Cannot create sequence gap via available APIs. Cannot test replay gap detection (replay doesn't use journal). Product implementation required.

**Contract-equivalent evidence**: Storage layer gap detection verified via vb_storage codec tests.

---

## Summary

| Test | Before | After | Root Cause | Status |
|------|--------|-------|------------|--------|
| `answer_returns_stale_request_when_not_in_waiting_answer_state` | FAIL (wrong error) | PASS | Wrong state (Completed vs Active) | **REPAIRED** |
| `replay_from_empty_journal_produces_valid_initial_state` | FAIL (no reset) | PASS | Missing `reset_tracker()` call | **REPAIRED** |
| `replay_with_malformed_event_returns_replay_corruption` | FAIL (no corruption) | BLOCKED | Cannot inject corruption, replay() doesn't use journal | **BLOCKED** |
| `replay_with_missing_event_returns_replay_corruption` | FAIL (no gap) | BLOCKED | Cannot create gap, replay() doesn't use journal | **BLOCKED** |

---

## Root Cause: Tests 3 & 4

The minimal implementation of `lifecycle::replay()` returns in-memory tracker state:

```rust
pub fn replay(_journal: &FjallJournal) -> LifecycleResult<Vec<RunState>> {
    let tracker = TRACKER.lock().map_err(...)?;
    let states: Vec<RunState> = tracker.states.iter().map(...).collect();
    Ok(states)  // Never touches _journal
}
```

The `_journal` parameter is unused. All state comes from the in-memory TRACKER, not from journal replay.

**What would fix tests 3 & 4**: Product implementation that calls `events_for_run_from` to validate journal contents and returns `ReplayCorruption` when corruption/gaps are detected.

**Current status**: Storage layer HAS the corruption detection (`validate_replayed_event`), but lifecycle layer doesn't USE it.

---

## Final Status: PARTIALLY REPAIRED (tests 1-2), BLOCKED (tests 3-4)

Tests 1 and 2 are fully repaired and pass.
Tests 3 and 4 are blocked by minimal implementation architecture — cannot test replay corruption/gap detection because `replay()` doesn't use the journal.

**Overall bead status**: Cannot advance — tests 3 and 4 require product changes to wire storage-layer corruption detection into `lifecycle::replay()`.
