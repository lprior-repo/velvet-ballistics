bead_id: vb-qi37.16.1
bead_title: cli/runtime: Implement durable cancel transition
phase: 2
updated_at: 2026-05-09T00:00:00Z

# Codebase Map: Cancel Command Implementation

## Summary
The runtime and IPC layers already have cancel infrastructure. The CLI surface is missing entirely. This bead adds the `cancel` CLI command with cancellation reason tracking and durable journal evidence.

## Existing Cancel Infrastructure

### Runtime Layer (vb_runtime)
- `vb_runtime/src/runtime.rs:100` — `Runtime::cancel_run(&self, run: RunId) -> RuntimeResult<()>`
  - Enqueues `ShardCommand::Cancel { run }` to the appropriate shard.
- `vb_runtime/src/shard/types.rs:93` — `ShardCommand::Cancel { run }` variant exists.
- `vb_runtime/src/shard/lifecycle.rs:376` — `Shard::handle_cancel(&mut self, run: RunId) -> RuntimeResult<()>`
  - Removes pending timers.
  - Appends `RuntimeJournalEvent::RunCancelled { run }` to journal if run exists.
  - Removes run from `runs` map, releases frame, increments failed counter.
  - Pushes `TraceEvent::RunCancelled { run }`.
  - **Idempotent**: canceling a non-existent run returns `Ok(())` silently.
  - **Missing**: no cancellation reason field.
- `vb_runtime/src/journal.rs:42` — `RuntimeJournalEvent::RunCancelled { run }`
  - **Missing**: no `reason` field.

### Storage Layer (vb_storage)
- `vb_storage/src/events.rs:156` — `JournalEvent::RunCancelled { run, seq, attempt }`
  - **Missing**: no `reason` field.
- Storage journal events are read by CLI commands like `events`, `trace`, `replay`, `inspect`.

### IPC Layer (vb_ipc)
- `vb_ipc/src/commands.rs:17` — `IpcCommand::CancelRun = 3`
- `vb_ipc/src/payloads.rs:26` — `IpcPayload::CancelRun { run_id }`
  - **Missing**: no `reason` field.
- `vb_ipc/src/server/handlers.rs:158` — `handle_cancel_run` delegates to `runtime.cancel_run()`.

### CLI Layer (velvet_ballastics)
- `velvet_ballastics/src/args.rs` — **NO** `Cancel` variant in `Command` enum.
- `velvet_ballastics/src/args.rs:178` — `VALID_COMMANDS` does not include "cancel".
- `velvet_ballastics/src/main.rs` — **NO** `cmd_cancel` function or dispatch arm.
- `velvet_ballastics/src/main.rs:53-91` — HELP text has no cancel entry.
- CLI HAS lifecycle commands: `inspect`, `events`, `replay`, `trace`, `retry`, `resume`, `answer`, `submit`.
- These commands all take `<run_id> --db <path> [--json|--jsonl]`.

## Files to Modify
1. `crates/velvet_ballastics/src/args.rs` — Add `Cancel` command variant, parsing, HELP text.
2. `crates/velvet_ballastics/src/main.rs` — Add `cmd_cancel` dispatch and implementation.
3. `crates/vb_runtime/src/journal.rs` — Add optional `reason` to `RunCancelled`.
4. `crates/vb_runtime/src/shard/lifecycle.rs` — Pass reason through `handle_cancel`.
5. `crates/vb_runtime/src/shard/types.rs` — Add `reason` to `ShardCommand::Cancel`.
6. `crates/vb_storage/src/events.rs` — Add optional `reason` to `JournalEvent::RunCancelled`.
7. `crates/vb_ipc/src/payloads.rs` — Add optional `reason` to `CancelRun` payload.
8. `crates/vb_ipc/src/server/handlers.rs` — Pass reason through to runtime.

## Test Files to Extend
1. `crates/vb_runtime/src/shard/tests.rs` — Extensive cancel tests; add reason + idempotent cancel reason tests.
2. `crates/vb_runtime/tests/durability_matrix_integration.rs` — Add cancel with reason.
3. `crates/vb_storage/src/tests.rs` — `RunCancelled` roundtrip tests; add reason variant.
4. `crates/vb_ipc/src/tests.rs` — `CancelRun` payload roundtrip; add reason.
5. `crates/velvet_ballastics/src/main.rs` — CLI tests (inline in main.rs file).

## Idempotency Behavior (Already Implemented)
- Canceling a non-existent run: silent `Ok(())` (no journal event, no trace, no counter increment).
- Canceling the same run twice: second cancel is a no-op because run was removed.
- Canceling a finished run: silent `Ok(())` (no counter increment).

## Engineering Constraints
- No `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`.
- No unchecked indexing, slicing, casts, or arithmetic.
- Moon v2 CI gates (`moon ci`) are canonical.
- Source lint is zero tolerance.
