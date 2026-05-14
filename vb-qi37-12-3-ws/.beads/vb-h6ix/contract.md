# Contract Specification: vb-h6ix — Replay Latest Execution Attempt Only

## Context

- **Feature**: Runtime recovery replay that reconstructs the latest execution attempt per run and ignores stale attempt events without losing diagnostic evidence.
- **Domain terms**:
  - **Run**: A single execution instance of a workflow, identified by `RunId`.
  - **Attempt**: A single execution attempt within a run (for retries/replays). The first attempt is attempt 1.
  - **Attempt Number**: A `u16` that uniquely identifies which attempt an event belongs to, carried on action-scheduling and action-completion events.
  - **Journal Event**: A durable event recorded during workflow execution (`StepStarted`, `ActionScheduled`, `ActionCompletedEvent`, `RunFinished`, etc.).
  - **Replay**: The process of reconstructing runtime state from journal events.
  - **Latest Attempt Filtering**: During replay, only processing events from the highest attempt number into live state.
  - **Stale Events**: Events from older attempts that are preserved as diagnostics but do NOT mutate live state.
  - **Live Hydration**: Populating live runtime state (frame, slots, pending action tickets) from replay.
- **Assumptions**:
  - Attempt numbers are durably present on admission (`RunAdmission`) and action completion events (`ActionCompletedEvent`, `ActionFailedEvent`).
  - Replay code has deterministic ordering for journal events via `EventSeq` sequence numbers.
  - The latest attempt is identified by the maximum attempt number seen across all events for a run.
  - A run that retried will have events from multiple attempts interleaved in the journal.
- **Open questions**:
  - None identified after reading `vb_storage/src/events.rs`, `vb_storage/src/recovery/replay/core.rs`, and `vb_storage/src/recovery/types.rs`.

---

## Preconditions

- **PRE-001**: The journal contains events with attempt numbers for a given run. Events without attempt numbers are treated as attempt 1 (the default/initial attempt).
- **PRE-002**: Events are stored in deterministic sequence order via `EventSeq` per run. Sequence numbers are monotonic and gap-free for a given run.
- **PRE-003**: The replay function receives a consistent, ordered slice of `JournalEvent` values retrieved from `FjallJournal::events_for_run`.

---

## Postconditions

- **POST-001**: Recovered run state (frame seed, slot values, pending actions) reflects only the latest attempt's events.
- **POST-002**: Events from stale (older) attempts are observable as ignored diagnostics — they do not appear in the live `RecoveryFrameSeed` or `ActionReplayTracker`.
- **POST-003**: The latest attempt is determined by the maximum attempt number observed across all action-scheduling and action-completion events for the run.
- **POST-004**: Stale events are retained in the returned replay event list for diagnostic purposes, but their effects (slot writes, action completions, terminal markers) do not influence the recovered live state.
- **POST-005**: If a stale `RunFinished` or `RunFailedEvent` event exists after a newer attempt's events, the recovered terminal state reflects the newer attempt's outcome, not the stale event.

---

## Invariants

- **INV-001**: Replay is deterministic for any fixed journal event sequence. Given the same input events in the same order, the replay produces identical `RecoveryFrameSeed` and `ActionReplayTracker` state.
- **INV-002**: Latest attempt selection is independent of wall clock time. Ordering is determined solely by `EventSeq` sequence numbers and attempt number comparison.
- **INV-003**: Ignored stale events cannot allocate live timers, pending action tickets, or slot values in the recovered frame seed.
- **INV-004**: The `ActionReplayTracker` only records completed/failed actions from the latest attempt.
- **INV-005**: A stale `RunFinished` event from an older attempt MUST NOT cause the recovered run to appear finished if a newer attempt's events show the run as still in-progress or failed.

---

## Error Taxonomy

- **RecoveryError::ReplayDivergence** — Step ordering invariant violated during replay.
- **RecoveryError::NonIdempotentActionBlocked** — Duplicate action scheduling detected from stale events during replay.
- **RecoveryError::Journal**(JournalError) — Underlying journal read/validation failure.
- **RecoveryError::NoRecoveryData** — No events found for the run.
- **RecoveryError::CorruptSnapshot** — Snapshot decode failure.

---

## Contract Signatures

```rust
// vb_storage/src/recovery/replay/core.rs

/// Core replay logic for all journal event kinds.
/// Filters events to the latest attempt and populates the action tracker.
///
/// # Arguments
/// * `events` — Ordered journal events for one run, retrieved via `FjallJournal::events_for_run`.
/// * `tracker` — Action replay tracker to populate with latest-attempt completions.
///
/// # Returns
/// * `Ok(replayed)` — All input events (including stale ones for diagnostics), but with
///   live hydration applied only from the latest attempt.
/// * `Err(RecoveryError)` — Replay divergence or non-idempotent action violation.
pub fn replay_events(
    events: &[JournalEvent],
    tracker: &mut ActionReplayTracker,
) -> RecoveryResult<Vec<JournalEvent>>;

/// Replays a full journal for a run when no snapshot is available.
pub fn recover_full_journal(
    journal: &FjallJournal,
    run: RunId,
    tracker: &mut ActionReplayTracker,
) -> RecoveryResult<Vec<JournalEvent>>;

/// Replays from a snapshot plus tail events.
pub fn recover_snapshot_plus_tail(
    snapshot: &RunSnapshot,
    tail_events: &[JournalEvent],
    tracker: &mut ActionReplayTracker,
) -> RecoveryResult<Vec<JournalEvent>>;

/// Checks whether a run has reached a terminal state.
#[must_use]
pub fn is_terminal_event(event: &JournalEvent) -> bool;

/// Extracts the terminal event from a replay sequence, if any.
pub fn extract_terminal(events: &[JournalEvent]) -> Option<&JournalEvent>;
```

---

## Non-goals

- Replay of events from multiple runs simultaneously (each run is replayed independently).
- Modifying the Fjall journal layout or key format.
- Recovery of runs that have no attempt number metadata (treated as single attempt 1).
- Automatic retry policy enforcement (that's a runtime concern, not a replay concern).
