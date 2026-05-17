# Domain Model Review: CLI Lifecycle Event-Applied Tracker

## Domain Entities

### RunId
- **Type**: Primary key for workflow runs
- **Origin**: `vb_core::ids::RunId`
- **Invariant**: Unique per run, stable across replay

### LifecycleState
- **Variants**: `Pending`, `Active`, `WaitingAnswer`, `Completed`, `Failed`, `Cancelled`
- **Encoding**: Sum type in `vb_core::workflow::LifecycleState`
- **Terminal States**: `Completed`, `Cancelled` — no outgoing transitions
- **Valid Transitions**:
  ```
  Pending → Active
  Active → WaitingAnswer (on ask scheduled)
  Active → Failed
  Active → Cancelled
  WaitingAnswer → Completed (on answer)
  WaitingAnswer → Cancelled
  Failed → Active (on retry)
  ```

### JournalEvent
- **Variants**: `RunCancelled`, `RunResumed`, `RunRetried`, `RunAnswered`, `RunAccepted`, `RunAdmission`, `RunFailedEvent`, `WaitScheduledEvent`, `AskScheduledEvent`, `AskAnsweredEvent`, `ActionFailedEvent`
- **Ordering**: Append-only, chronological per run
- **Invariant**: Last event determines current `LifecycleState`

### RunState
- **Fields**: `RunId`, `LifecycleState`, event sequence number
- **Derivation**: Always from `derive_lifecycle_state_from_events(events.last())`

### FjallJournal
- **Interface**: `events_for_run(run_id) → Vec<JournalEvent>`
- **Behavior**: Returns complete ordered event sequence for a run
- **Storage**: Fjall LSM-tree key-value store

## State Derivation Function

```rust
fn derive_lifecycle_state_from_events(events: &[JournalEvent]) -> LifecycleState {
    // Last event determines state:
    // RunCancelled → Cancelled
    // RunResumed/RunRetried/RunAccepted/RunAdmission → Active
    // RunAnswered/RunFinished → Completed
    // RunFailedEvent → Failed
    // WaitScheduledEvent/AskScheduledEvent/AskAnsweredEvent → WaitingAnswer
    // ActionFailedEvent → Failed
}
```

**Correctness Claim**: This function is pure and total — every event sequence maps to exactly one state.

## Transition Validation

```rust
fn check_lifecycle_transition(current: LifecycleState, cmd: LifecycleCommand) -> bool
```

**Command Variants**: `Cancel`, `Resume`, `Retry`, `Answer`

**Invariant**: A transition is valid if and only if `check_lifecycle_transition` returns true.

## Current Architecture (Pre-Refactoring)

```
┌─────────────────────────────────────────────────────────────┐
│                      CLI Commands                           │
│  (cancel, resume, retry, answer)                            │
└─────────────────────────┬───────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│                  lifecycle.rs API                            │
│  with_tracker_mut(run_id, |tracker| {                       │
│      tracker.set_state(new_state);  // IN-MEMORY            │
│  })                                                         │
│  journal.append_event(...);  // JOURNAL WRITE                │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│              static TRACKER: LazyLock<Mutex<...>>           │
│              HashMap<RunId, LifecycleState>                  │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│                     FjallJournal                            │
└─────────────────────────────────────────────────────────────┘
```

**Problem**: In-memory state update happens AFTER journal write, creating divergence window. If process crashes between write and update, state is inconsistent.

## Target Architecture (Post-Refactoring)

```
┌─────────────────────────────────────────────────────────────┐
│                      CLI Commands                           │
│  (cancel, resume, retry, answer)                            │
└─────────────────────────┬───────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  journal.events_for_run(run_id)                             │
│  → derive_lifecycle_state_from_events(events)              │
│  → check_lifecycle_transition(current, cmd)                 │
│  → journal.append_event(new_event)                         │
│  NO IN-MEMORY TRACKER                                      │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│                     FjallJournal                            │
└─────────────────────────────────────────────────────────────┘
```

**Guarantee**: State is always derived from persisted journal. No divergence possible.

## Review Findings

### Finding 1: State Derivation is Correct
- `derive_lifecycle_state_from_events()` is pure and total
- Maps last event → correct state
- No partiality or ambiguity

### Finding 2: Transition Logic is Centralized
- `check_lifecycle_transition()` in `vb_core` is the single source of truth
- All CLI commands must route through this validator
- Post-refactoring, this remains unchanged

### Finding 3: Journal is Authoritative
- Events are append-only
- Complete history is available via `events_for_run()`
- No need for in-memory shadow state

### Finding 4: Test Helpers are Problematic
- `set_lifecycle_state(run, state)` bypasses journal — TEST ONLY
- `reset_tracker()` clears in-memory state — TEST ONLY
- Post-refactoring: these helpers become unnecessary or must be removed

### Finding 5: Replay Already Works Correctly
- `replay()` already derives all states from events
- No changes needed to replay functionality

## Risk Assessment

| Risk | Severity | Mitigation |
|------|----------|------------|
| State divergence | HIGH | Remove in-memory tracker; always derive from journal |
| Transition validation bypass | HIGH | Ensure all commands call `check_lifecycle_transition` with journal-derived state |
| Performance regression | MEDIUM | Journal read on every command; may need caching strategy |
| Test breakage | MEDIUM | Rewrite tests to use journal events, not direct state set |

## Domain Model Correctness Criteria

1. **Consistency**: For any run at any time, journal-derived state equals observed state
2. **Completeness**: Every valid transition is representable via journal events
3. **Soundness**: No invalid transition can be expressed via journal events
4. **Durability**: State survives process restarts (journal is persisted)
5. **Recoverability**: Full state can be reconstructed from journal via `replay()`
