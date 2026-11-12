bead_id: vb-qi37.1.3
bead_title: runtime/recovery: Hydrate RunFrame from snapshot and journal
phase: 3
updated_at: 2026-05-09T00:00:00Z

# Contract Specification — Hydrate RunFrame from Snapshot + Journal

## Context
- **Feature**: `hydrate_run_frame` — reconstruct a live `RunFrame` from the latest `RunSnapshot` plus ordered tail `JournalEvent`s.
- **Domain terms**: `RunFrame`, `RunSnapshot`, `JournalEvent`, `RecoveryFrameSeed`, `StepState`, `RecoveredStepState`, `SlotValue`, `Taint`, `EventSeq`, `ActionReplayTracker`
- **Assumptions**:
  - Snapshot slot/taint bytes are postcard-encoded `Vec<(SlotIdx, SlotValue, Taint)>`.
  - Journal events for a run are ordered by `EventSeq`.
  - Tail events are strictly after the snapshot sequence.
- **Open questions**: None at contract time.

## Preconditions

1. `snapshot.run` must equal the requested `run_id`.
2. Tail events must all belong to `run_id`.
3. Tail events must be strictly after `snapshot.seq` (no overlap).
4. Snapshot `slots` and `taint` byte vectors, if non-empty, must be decodable.
5. `step_count` derived from snapshot + events must be > 0.

## Postconditions

1. Returns `Ok(RunFrame)` with all fields populated from durable evidence.
2. `RunFrame::run_id()` equals the requested `run_id`.
3. `RunFrame::pc()` equals the program counter inferred from the last state-affecting event.
4. `RunFrame::step_count()` and `RunFrame::slot_count()` are derived from max observed indices + 1.
5. `RunFrame::states` reflects step states from snapshot base + tail event transitions.
6. `RunFrame::slots` and `RunFrame::taint` reflect snapshot base overwritten by tail `SlotWrittenEvent`s.
7. `RunFrame::executed()` equals the count of state-transitioning tail events applied.
8. `RunFrame::parallel_in_flight()` and `RunFrame::max_parallel_in_flight()` are reconstructed from action scheduling/completion events.
9. No empty-frame success path: if any required data is missing or corrupt, returns `Err(RecoveryError)`.

## Invariants

1. **Dimension integrity**: `states.len() == step_count`, `slots.len() == slot_count`, `taint.len() == slot_count`.
2. **Slot-taint parity**: Every initialized slot has a corresponding taint marker.
3. **Step state machine legality**: All step states in the reconstructed frame must be valid under `RunFrame::validate_transition` from `Pending`.
4. **Deterministic ordering**: Same snapshot + same tail events always produce the same `RunFrame` (or same error).
5. **No silent defaults**: Missing or corrupt data always produces a typed error, never a silently defaulted field.

## Error Taxonomy

- `RecoveryError::CorruptSnapshot { run, seq }` — snapshot bytes fail decode or have dimension mismatch.
- `RecoveryError::NoRecoveryData { run }` — no snapshot and no tail events provided.
- `RecoveryError::ReplayDivergence { step, detail }` — tail event violates ordering (e.g., seq <= snapshot.seq, wrong run_id).
- `RecoveryError::FrameDimensionOverflow { run }` — derived step_count or slot_count exceeds `u16::MAX`.
- `RecoveryError::NonIdempotentActionBlocked { action, step }` — replay detects a resolved action that would be re-executed.
- `RecoveryError::InvalidCompiledWorkflow { reason: "step_count_zero" }` — derived dimensions produce zero steps.

## Contract Signatures

```rust
/// Hydrates a live RunFrame from a snapshot plus ordered tail journal events.
pub fn hydrate_run_frame(
    snapshot: &RunSnapshot,
    tail_events: &[JournalEvent],
    run_id: RunId,
) -> RecoveryResult<RunFrame>;

/// Hydrates a live RunFrame from full journal events (no snapshot).
pub fn hydrate_run_frame_from_events(
    events: &[JournalEvent],
    run_id: RunId,
) -> RecoveryResult<RunFrame>;

/// Decodes snapshot slot/taint bytes into initialized slot entries.
fn decode_snapshot_slots(
    slots_bytes: &[u8],
    taint_bytes: &[u8],
    run: RunId,
) -> RecoveryResult<Vec<RecoveredSlotEntry>>;

/// Applies tail events to a mutable RunFrame, tracking action resolution.
fn apply_tail_events(
    frame: &mut RunFrame,
    tail_events: &[JournalEvent],
    tracker: &mut ActionReplayTracker,
) -> RecoveryResult<u64>; // returns executed count
```

## Non-goals
- Does not resume pending actions into the runtime scheduler (out of scope).
- Does not verify workflow digests during hydration (done in caller).
- Does not trim or compact journal (separate bead).
