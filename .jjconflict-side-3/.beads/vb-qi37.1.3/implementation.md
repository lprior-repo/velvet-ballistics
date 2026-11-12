bead_id: vb-qi37.1.3
bead_title: runtime/recovery: Hydrate RunFrame from snapshot and journal
phase: 6
updated_at: 2026-05-09T00:00:00Z

# Implementation Summary

## Changed Files

1. `crates/vb_storage/src/recovery/recover.rs` — Added hydrate functions and helpers
2. `crates/vb_storage/src/recovery/mod.rs` — Added re-exports for new functions
3. `crates/vb_storage/src/recovery/tests.rs` — Added 16 hydrate-specific unit tests

## Implementation Details

### `hydrate_run_frame(snapshot, tail_events, run_id)`

**Preconditions enforced:**
- `snapshot.run == run_id` → `ReplayDivergence`
- All tail events belong to `run_id` → `ReplayDivergence`
- All tail event seq > `snapshot.seq` → `ReplayDivergence`
- Empty snapshot + empty tail → `NoRecoveryData`
- Derived `step_count == 0` → `ReplayDivergence`

**Process:**
1. Decode snapshot slots/taint bytes via postcard into `Vec<RecoveredSlotEntry>`
2. Derive dimensions (`step_count`, `slot_count`, `first_step`) from snapshot + tail events
3. Construct `RunFrame` with derived dimensions
4. Apply snapshot slot entries to frame (preserving taint)
5. Apply tail events via `apply_tail_events`:
   - `StepStarted` → `mark_running`
   - `StepSucceeded` → `mark_succeeded`
   - `ActionScheduled` → `add_parallel_in_flight(1)`
   - `ActionCompletedEvent` → `sub_parallel_in_flight(1)` + tracker
   - `ActionFailedEvent` → `sub_parallel_in_flight(1)` + tracker
   - `SlotWrittenEvent` → `write_slot_with_taint` (preserves existing taint)
   - `WaitScheduledEvent` → `mark_waiting`
   - `AskScheduledEvent` → `mark_asking`
6. Set `executed` counter from applied event count
7. Return fully populated frame

### `hydrate_run_frame_from_events(events, run_id)`

**Process:**
1. Delegate to existing `recover_runtime_frame_seed_from_events` for dimension/state derivation
2. Construct `RunFrame` from seed dimensions
3. Apply seed step states (Running, Succeeded, Failed, Waiting, Asking)
4. Apply seed slots with taint
5. Set PC from seed
6. Set executed counter from state-affecting event count
7. Compute parallel in-flight from action events via `compute_parallel_in_flight`
8. Set `max_parallel_in_flight` to observed peak (fixes u16::MAX default)

### Key Design Decisions

- **Taint preservation**: SlotWrittenEvent preserves existing taint from snapshot rather than defaulting to Clean.
- **No empty-frame success**: Every error path returns typed RecoveryError; no silent defaults.
- **Deterministic**: Same inputs always produce same output (no randomness, no env deps).
- **Parallel tracking**: Tracks both current in-flight and peak via `add_parallel_in_flight`/`sub_parallel_in_flight`.

## Contract Clause Mapping

| Contract Clause | Implementation Location |
|---|---|
| PRE-1 (snapshot.run == run_id) | `hydrate_run_frame` lines 14-24 |
| PRE-2 (tail events run_id) | `hydrate_run_frame` lines 26-38 |
| PRE-3 (tail seq > snapshot.seq) | `hydrate_run_frame` lines 40-53 |
| PRE-4 (snapshot bytes decodable) | `decode_snapshot_slots` |
| PRE-5 (step_count > 0) | `hydrate_run_frame` lines 68-72 |
| POST-1 (Ok(RunFrame) populated) | Entire `hydrate_run_frame` |
| POST-2 (run_id equality) | `RunFrame::new(run_id, ...)` |
| POST-3 (pc from last event) | `apply_tail_events` updates PC via step events |
| POST-4 (dimensions from max indices) | `derive_dimensions_from_snapshot_and_tail` |
| POST-5 (states from snapshot + events) | `apply_tail_events` state transitions |
| POST-6 (slots/taint from snapshot + events) | Snapshot decode + SlotWrittenEvent handling |
| POST-7 (executed count) | `apply_tail_events` return value |
| POST-8 (parallel tracking) | `compute_parallel_in_flight` |
| POST-9 (no empty-frame success) | All error paths return typed errors |
| INV-1 (dimension integrity) | `RunFrame::new` guarantees array lengths |
| INV-2 (slot-taint parity) | `write_slot_with_taint` atomic write |
| INV-3 (step state machine legality) | `RunFrame::validate_transition` |
| INV-4 (deterministic) | Pure functions, no randomness |
| INV-5 (no silent defaults) | Every failure returns RecoveryError |

## Test Results

- 16/16 hydrate-specific tests pass
- 892/894 total vb_storage tests pass (2 pre-existing unrelated failures)
