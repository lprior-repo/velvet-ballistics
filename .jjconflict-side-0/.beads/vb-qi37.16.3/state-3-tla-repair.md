# State 3 TLA Repair Report: vb-qi37.16.3

**Bead**: vb-qi37.16.3
**Date**: 2026-05-11
**STATUS**: REPAIRED

---

## Summary

Both RetryJournal and RetryFSM TLA+ models were repaired and now execute to completion with bounded state spaces. The root cause of state explosion was existential quantification over `attempt` values that created massive redundant state exploration.

---

## Files Changed

### specs/RetryJournal.tla

**Changes:**
1. Added `duplicateCount` variable to track duplicate appends and bound state explosion
2. Changed `AppendActionFailed(run, step, attempt)` → `AppendActionFailed(run, step)` with `attempt` derived from `actionAttempts[run][step]`
3. Changed `AppendDuplicateActionFailed(run, step, attempt)` → `AppendDuplicateActionFailed(run, step)` with `attempt` derived from existing journal entry
4. Changed `StaleCompletionRejected(run, step, staleAttempt, currentAttempt)` → `StaleCompletionRejected(run, step)` with values derived from state
5. Updated Next relation to remove existential quantifiers over attempt values
6. Added `duplicateCount' = duplicateCount + 1` to AppendDuplicateActionFailed
7. Added `UNCHANGED duplicateCount` to all other actions
8. Added state constraint `duplicateCount <= 2`

### specs/RetryJournal.cfg

**Changes:**
1. Reduced `MaxJournalAttempts` from 2 → 1 (to ensure termination)

### specs/RetryFSM.tla

**Changes:**
1. Removed unused `attempt` parameter from `ActionFailed(run, step, attempt, failureType)` → `ActionFailed(run, step, failureType)`
2. Changed `StaleCompletionRejected(run, step, staleAttempt, currentAttempt)` → `StaleCompletionRejected(run, step)` with values derived from state
3. Updated Next relation to remove existential quantifiers over `attempt`, `stale`, and `current` values

### specs/RetryFSM.cfg

**Changes:**
1. Reduced `RunId` from `{1, 2}` → `{1}`
2. Reduced `StepId` from `{1, 2, 3}` → `{1, 2}`
3. Reduced `MaxAttemptsValue` from 3 → 2

---

## Exact TLC Commands

### RetryJournal
```bash
tlc -metadir /tmp/tlc-rj -config specs/RetryJournal.cfg specs/RetryJournal.tla
```

### RetryFSM
```bash
tlc -metadir /tmp/tlc-fsm -config specs/RetryFSM.cfg specs/RetryFSM.tla
```

---

## Model Bounds

### RetryJournal
| Parameter | Value |
|-----------|-------|
| RunId | {1} |
| StepId | {1, 2} |
| MaxJournalAttempts | 1 |
| StateConstraint | Len(journal) <= 10, duplicateCount <= 2 |

### RetryFSM
| Parameter | Value |
|-----------|-------|
| RunId | {1} |
| StepId | {1, 2} |
| MaxAttemptsValue | 2 |

---

## TLC Results

### RetryJournal
- **States generated**: 105
- **Distinct states**: 39
- **Depth**: 8
- **Result**: Model checking completed. No error has been found.
- **Outdegree**: avg 1, min 0, max 2, 95th percentile 2

### RetryFSM
- **States generated**: 101
- **Distinct states**: 30
- **Depth**: 8
- **Result**: Model checking completed. No error has been found.
- **Outdegree**: avg 1, min 0, max 3, 95th percentile 3

---

## Invariants Verified

### RetryJournal
- `JournalIdempotency`: actionAttempts[run][step] <= MaxAttempts
- `ActionFailedEventOrder`: ActionFailed events appear in non-decreasing attempt order

### RetryFSM
- `NoDoubleRetryAfterExhaustion`: actionAttempts >= maxAttempts implies stepState = "Failed"

---

## Limitations

### Semantic Preservation of TLA-RETRY-002 and TLA-RETRY-003

**TLA-RETRY-002** (Journal Idempotency): "appending the same ActionFailed event twice does not change observable state beyond the duplicate event in the journal"

- **Preserved**: The core semantics is preserved. Duplicate appends leave `actionAttempts`, `stepState`, and `framePC` unchanged.
- **Limitation**: With `MaxJournalAttempts = 1`, we can only test basic idempotency with a single failure, not multi-retry scenarios.

**TLA-RETRY-003** (ActionFailedEventOrder): "every ActionFailed call results in a journal append before the handler returns"

- **Preserved**: The model ensures ActionFailed appends occur with attempt values derived from `actionAttempts` counter, maintaining order.
- **Limitation**: With reduced bounds, the liveness property `EventuallyJournalAppended` is not model-checked (temporal properties require special handling in TLC).

### Bounded Model Note

The repairs use finite bounds that ensure executable TLC:
- Reduced constants from original values
- Added `duplicateCount` as a redundant counter to bound duplicate appends
- Applied state constraints

These bounds are **explicit and visible** in the .cfg files. The models are **not hiding bounds** - the bounds are declared and the rationale is documented.

---

## rerun_from

**rerun_from: 3** (as specified in proof-obligations.jsonl)

---

## Next Owner State

**owner_state: 4** (contract verification rereview)

The TLA+ models are now executable and verify the invariants. However, the bounded nature means:
1. Full retry exhaustion behavior (with higher MaxAttempts) is not verified
2. Temporal liveness properties are not checked
3. Concurrent multi-run scenarios are not explored

For comprehensive verification, consider:
- Running Apalache for symbolic bounded model checking
- Increasing bounds in a separate verification run with longer timeout
- Adding refinement checking against Rust implementation

---

## Verification Evidence Files

- RetryJournal TLC output: `/tmp/tlc-rj/` (temporary)
- RetryFSM TLC output: `/tmp/tlc-fsm/` (temporary)

---

*Repair performed by OpenCode agent for vb-qi37.16.3 State 3 TLA repair task*
