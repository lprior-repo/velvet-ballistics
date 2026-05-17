# Contract Specification: CLI Lifecycle Tracker Event-Applied

## Context

- **Bead ID**: vb-0253.7
- **Title**: cli: Make lifecycle tracker event-applied
- **Phase**: 1 (Explore and scope)
- **Domain**: Workflow lifecycle management via journal events
- **Primary Crate**: `vb_cli` (`crates/vb_cli/src/lifecycle.rs`)
- **Source**: `/home/lewis/src/velvet-ballistics` (read-only checkout)

## Domain Terms

| Term | Definition |
|------|------------|
| `RunId` | Unique identifier for a workflow run |
| `LifecycleState` | Enum: `Pending`, `Active`, `WaitingAnswer`, `Completed`, `Failed`, `Cancelled` |
| `JournalEvent` | Persisted event: `RunCancelled`, `RunResumed`, `RunRetried`, `RunAnswered`, `RunFailedEvent`, etc. |
| `RunStateTracker` | Current in-memory `HashMap<RunId, LifecycleState>` with global static mutex |
| `FjallJournal` | Fjall-based journal storage maintaining event sequence per run |
| `derive_lifecycle_state_from_events` | Pure function: last event in sequence determines current state |

## Problem Statement

The current `RunStateTracker` maintains in-memory state via `static TRACKER: LazyLock<Mutex<RunStateTracker>>`. State is set AFTER journal writes, creating a window where in-memory state can diverge from the persisted journal. The "event-applied" refactoring makes state derivation ALWAYS come from journal events.

## Open Questions

- **Q1**: Does `journal.events_for_run(run)` return events in guaranteed chronological order?
- **Q2**: Is there any existing code that bypasses journal writes and directly calls `with_tracker_mut`?
- **Q3**: Are there external consumers of the in-memory tracker state besides the CLI commands?

## Assumptions

- **A1**: `journal.events_for_run(run)` returns a complete, ordered event sequence for the run
- **A2**: `derive_lifecycle_state_from_events()` is pure and correctly maps last event → state
- **A3**: `check_lifecycle_transition()` in `vb_core` is the authoritative transition validator
- **A4**: The public API surface (`cancel`, `resume`, `retry`, `answer`, `replay`) remains unchanged

## Preconditions

- PRE-001: For all lifecycle commands (`cancel`, `resume`, `retry`, `answer`), the run identified by `RunId` must exist in the journal
- PRE-002: For `answer`, the run must be in `WaitingAnswer` state
- PRE-003: For `cancel`, `resume`, `retry`, the run must be in a non-terminal state (`Active`, `WaitingAnswer`, `Failed`)
- PRE-004: The journal must be accessible and return a valid event sequence

## Postconditions

- POST-001: After `cancel(run)`, the journal contains `JournalEvent::RunCancelled` and the derived state is `Cancelled`
- POST-002: After `resume(run)`, the journal contains `JournalEvent::RunResumed` and the derived state is `Active`
- POST-003: After `retry(run)`, the journal contains `JournalEvent::RunRetried` and the derived state is `Active`
- POST-004: After `answer(run, answer)`, the journal contains `JournalEvent::RunAnswered` and the derived state is `Completed`
- POST-005: All lifecycle functions return `Ok(())` on success; error variants on failure
- POST-006: `replay(journal)` returns `Vec<RunState>` where each `RunState` is derived purely from journal events

## Invariants

- INV-001: **State-Journal Consistency**: For any run, the derived state from `journal.events_for_run(run)` MUST equal the state that would be observed by an external observer
- INV-002: **No Divergence**: There must be no window where in-memory tracker state differs from journal-derived state
- INV-003: **Valid Transitions Only**: All state transitions MUST pass `check_lifecycle_transition()` validation
- INV-004: **Event Immutability**: Once written, journal events are never modified or deleted (append-only)
- INV-005: **Terminal States Final**: `Completed`, `Cancelled` are terminal — no transitions out

## Error Taxonomy

| Error Variant | Trigger Condition |
|--------------|-------------------|
| `LifecycleInvalidTransition` | Transition not allowed from current state per `check_lifecycle_transition` |
| `LifecycleDuplicateRequest` | Command already applied (e.g., cancel on cancelled run) |
| `LifecycleStaleRequest` | Run already in terminal state (`Completed`, `Cancelled`) |
| `LifecycleStorageUnavailable` | Journal read failure or lock acquisition failure |
| `JournalWriteFailure` | Failed to append event to journal |
| `RunNotFound` | Requested `RunId` does not exist in journal |

## Contract Signatures

```rust
pub type LifecycleResult<T> = Result<T, CoreError>;

pub fn cancel(run: RunId, journal: &FjallJournal) -> LifecycleResult<()>
pub fn resume(run: RunId, journal: &FjallJournal) -> LifecycleResult<()>
pub fn retry(run: RunId, journal: &FjallJournal) -> LifecycleResult<()>
pub fn answer(run: RunId, answer: String, journal: &FjallJournal) -> LifecycleResult<()>
pub fn replay(journal: &FjallJournal) -> LifecycleResult<Vec<RunState>>
```

## Refactoring Contract

The refactoring must satisfy:

1. **Remove in-memory state**: `RunStateTracker`, `static TRACKER`, `with_tracker`, `with_tracker_mut` must be removed
2. **Event-applied reads**: Every public lifecycle function MUST derive state from `journal.events_for_run(run)` before validation
3. **Event-applied writes**: State is determined by journal event, not by direct assignment
4. **Transition validation**: `check_lifecycle_transition()` is called with current journal-derived state
5. **Public API parity**: External callers see identical behavior; only internal implementation changes

## TLA+-Owned Clauses

- INV-001, INV-002: Journal-state consistency is a TLA+ invariant over the lifecycle state machine
- The state machine `Pending → Active → WaitingAnswer ↔ Cancelled` with retry/answer transitions

## Verus-Owned Clauses

- INV-003, INV-004: `check_lifecycle_transition` correctness and event immutability proven in `vb_core`
- The pure state derivation function `derive_lifecycle_state_from_events` must be verified

## Theorem-Owned Clauses

- None required for this refactoring; the problem is well-contained to TLA+/Verus scope

## Non-goals

- No changes to `vb_core` state machine logic (already verified)
- No changes to `vb_storage` journal implementation
- No changes to public API
- No changes to `replay()` function (already event-applied)
