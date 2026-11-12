bead_id: vb-qi37.1.3
bead_title: runtime/recovery: Hydrate RunFrame from snapshot and journal
phase: 2
updated_at: 2026-05-09T00:00:00Z

# codebase-map.md — Recovery / RunFrame Hydration Domain

## Workspace Layout

```
crates/
  vb_core/           — Core types: RunFrame, StepState, SlotValue, Taint, ids
  vb_storage/        — Journal, recovery, snapshots
    src/
      recovery/
        mod.rs       — Re-exports: RecoveryFrameSeed, RecoveryError, RunSnapshot, etc.
        types.rs     — Recovery types: RecoveryFrameSeed, RecoveryError, RunSnapshot, etc.
        replay/
          mod.rs     — Re-exports: recover_full_journal, recover_snapshot_plus_tail, replay_events
          core.rs    — replay_events, recover_full_journal, recover_snapshot_plus_tail, load_snapshot
          summary.rs — summarize_recovery_events, recover_runtime_frame_seed_from_events*
        recover.rs   — High-level: recover_runtime_summary, recover_runtime_frame_seed, recover_all_incomplete_runs
        tests.rs     — Recovery unit tests
      events.rs      — JournalEvent enum (RunAccepted, StepStarted, StepSucceeded, SlotWrittenEvent, etc.)
      snapshots.rs   — FjallJournal::put_snapshot, FjallJournal::snapshot
      journal.rs     — FjallJournal implementation
      lib.rs         — Crate re-exports
```

## Key Types

### RunFrame (`vb_core/src/frame.rs`)
```rust
pub struct RunFrame {
    run_id: RunId,
    pc: StepIdx,
    executed: u64,
    step_count: u16,
    slot_count: u16,
    max_parallel_in_flight: u16,
    parallel_in_flight: u16,
    states: Box<[StepState]>,
    slots: Box<[Option<SlotValue>]>,
    taint: Box<[Taint]>,
}
```
- Constructor: `RunFrame::new(run_id, first_step, step_count, slot_count)`
- State transitions: mark_running/succeeded/failed/skipped/waiting/asking/cancelled
- Slot ops: write_slot, write_slot_with_taint, read_slot, read_taint
- PC management: set_pc, increment_executed

### RecoveryFrameSeed (`vb_storage/src/recovery/types.rs`)
```rust
pub struct RecoveryFrameSeed {
    pub summary: RecoveryRuntimeSummary,
    pub first_step: StepIdx,
    pub step_count: u16,
    pub slot_count: u16,
    pub pc: StepIdx,
    pub steps: Vec<RecoveredStepEntry>,
    pub slots: Vec<RecoveredSlotEntry>,
    pub pending_actions: Vec<RecoveredPendingAction>,
    pub unsupported: UnsupportedRecoveryState,
}
```
- Produced by `recover_runtime_frame_seed_from_events()` and `recover_runtime_frame_seed_from_events_with_workflow()`
- Maps `RecoveredStepState` (Running, Succeeded, Failed, Waiting, Asking) to `StepState`
- Does NOT yet produce a live `RunFrame`

### RunSnapshot (`vb_storage/src/recovery/types.rs`)
```rust
pub struct RunSnapshot {
    pub run: RunId,
    pub seq: EventSeq,
    pub workflow: WorkflowDigest,
    pub slots: Vec<u8>,   // compact binary
    pub taint: Vec<u8>,   // compact binary
}
```
- Stored/loaded via `FjallJournal::put_snapshot` / `FjallJournal::snapshot`
- Currently stores slots and taint as raw bytes (encoding TBD — likely postcard)

### JournalEvent (`vb_storage/src/events.rs`)
Key events for hydration:
- `RunAccepted { run, seq, workflow }`
- `StepStarted { run, seq, step, attempt }`
- `StepSucceeded { run, seq, step, output }`
- `ActionScheduled { run, seq, step, action, attempt }`
- `ActionCompletedEvent { run, seq, step, action, attempt }`
- `ActionFailedEvent { run, seq, step, action, attempt }`
- `SlotWrittenEvent { run, seq, slot, value, extra, attempt }`
- `WaitScheduledEvent { run, seq, step, attempt }`
- `AskScheduledEvent { run, seq, step, attempt }`
- `RunCancelled { run, seq }`
- `RunFinished { run, seq, result }`
- `RunFailedEvent { run, seq }`

## Recovery Pipeline (Current)

1. `recover_runtime_frame_seed(journal, run)` → `RecoveryFrameSeed`
   - Loads events via `journal.events_for_run(run)`
   - Calls `recover_runtime_frame_seed_from_events(&events)`
   - `FrameSeedAccumulator` processes events to derive dimensions, PC, step states, slots, pending actions
2. `recover_snapshot_plus_tail(snapshot, tail_events, tracker)` → replayed tail events
   - Validates tail events are after snapshot seq
   - Replays tail events
3. `recover_full_journal(journal, run, tracker)` → all replayed events

## Gap for This Bead

There is **no function** that:
1. Takes a `RunSnapshot` + tail `JournalEvent`s (or full journal)
2. Hydrates a live `RunFrame` from the snapshot's binary slot/taint data
3. Applies tail journal events to update the frame's step states, PC, slots, taint
4. Returns the fully reconstructed `RunFrame` or a typed `RecoveryError`

The `RecoveryFrameSeed` → `RunFrame` bridge is missing. This bead must implement:
- `hydrate_run_frame(snapshot, tail_events, run_id)` → `RecoveryResult<RunFrame>`
- Decode snapshot slot/taint bytes into `SlotValue`/`Taint` arrays
- Map `RecoveredStepState` → `StepState` (note: RecoveredStepState lacks Pending/Cancelled)
- Handle incomplete/corrupted snapshot data with typed errors
- Ensure deterministic ordering of event application
- Track `executed` counter from event count
- Handle `parallel_in_flight` / `max_parallel_in_flight` reconstruction

## Acceptance Criteria Context

> hydrate_run_frame reconstructs a non-empty faithful RunFrame or returns typed unsupported/incomplete recovery errors; no empty-frame success path remains.

This means:
- Must return `RecoveryError` (not an empty/zeroed frame) when data is missing/corrupt
- Must populate all RunFrame fields from durable evidence
- Must not silently default missing fields
