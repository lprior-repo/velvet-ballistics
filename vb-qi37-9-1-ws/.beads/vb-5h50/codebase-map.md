bead_id: vb-5h50
bead_title: storage: Trim journal events after durable snapshots
phase: state-2-complete
updated_at: 2026-05-09T00:00:00Z

# Codebase Map — vb_storage Journal Trimming

## Relevant Crates
- `crates/vb_storage/` — Fjall-backed append journal, trimming module, recovery
- `crates/vb_core/` — `RunId`, `EventSeq`, `WorkflowDigest`, `StepIdx`, `SlotIdx`, `ActionId`

## Key Files

### `crates/vb_storage/src/trimming.rs` (515 lines) — PRIMARY TARGET
- `TrimPolicy` { skip_noop_runs: bool } — NO retention policy for terminal runs
- `TrimError` — Fjall, Journal, NoSnapshot, IncompleteTrim
- `FjallJournal::latest_snapshot_seq(run)` — scans keyspace for highest seq snapshot
- `FjallJournal::trim_events_for_run(run, policy)` — deletes events with seq < snapshot seq
- `FjallJournal::trim_all_eligible_runs(policy)` — iterates all runs, skips no-snapshot runs
- `TrimmedRunResult` / `TrimStatus` — outcome reporting
- Tests: basic trim, idempotency, no-snapshot error, header/snapshot preservation, multi-run, latest seq

### `crates/vb_storage/src/journal.rs` (1328 lines) — Journal core
- `FjallJournal::open()` — opens 9 keyspaces including `run_snapshot`, `events`
- `FjallJournal::events_for_run(run)` — replays from latest snapshot seq or 0
- `FjallJournal::events_for_run_from(run, start_seq)` — validates contiguous sequence
- `FjallJournal::append_strict_batch(events)` — writes with `SyncAll` durability
- `FjallJournal::persist_strict()` — forces `fjall::PersistMode::SyncAll`
- `FjallJournal::put_snapshot()` in `snapshots.rs` — does NOT call `persist_strict()`

### `crates/vb_storage/src/recovery/replay/core.rs`
- `is_terminal_event(event)` — true for `RunFinished`, `RunCancelled`, `RunFailedEvent`
- `extract_terminal(events)` — finds last terminal event in sequence

### `crates/vb_storage/src/recovery/types.rs`
- `RunSnapshot` { run, seq, workflow, slots, taint }
- `RecoveryTerminalState` — Cancelled, Finished, Failed

### `crates/vb_storage/src/records.rs`
- `RunHeaderRecord` { run, workflow_id, compiled_digest, status, accepted_at_ms }

## Gaps vs MASTER.md §73 Trimming Contract
1. **No retention policy**: `TrimPolicy` lacks `terminal_retention_count` or `min_terminal_age_ms`
2. **No terminal run detection**: Trimming does not check if run is terminal before applying retention
3. **No explicit durability confirmation**: `latest_snapshot_seq` just scans keyspace; does not verify snapshot was fsynced
4. **Missing acceptance tests**: No test for "trim cannot delete events newer than safe replay point"

## Dependencies
- `fjall` 3.x — LSM-tree key-value storage
- `vb_core` — domain types
- `serde` + `postcard` — record serialization
- `blake3` — content digests

## Existing Test Coverage
- 7 trimming tests exist, all passing
- No proptest coverage for trimming
- No property test for "replay equivalence before/after trim"
