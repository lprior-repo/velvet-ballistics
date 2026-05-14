bead_id: vb-qi37.16.1
bead_title: cli/runtime: Implement durable cancel transition
phase: 6
updated_at: 2026-05-09T00:00:00Z

# Implementation Summary

## Changes Made

### 1. CLI Layer (velvet_ballastics)
- **args.rs**: Added `Command::Cancel { run_id, db, reason, output }` variant, `parse_cancel()` function, `ReasonTooLong` parse error, and HELP text.
- **main.rs**: Added `cmd_cancel()` dispatch and full implementation.

### 2. Runtime Layer (vb_runtime)
- **shard/types.rs**: Added `reason: Option<String>` to `ShardCommand::Cancel`
- **shard/lifecycle.rs**: Updated `handle_cancel()` to accept and persist reason through journal
- **journal.rs**: Added `reason: Option<String>` to `RuntimeJournalEvent::RunCancelled`
- **runtime.rs**: Updated `cancel_run()` call site with `reason: None`

### 3. Storage Layer (vb_storage)
- **events.rs**: Added `reason: Option<String>` to `JournalEvent::RunCancelled`
- **codec.rs**: Added roundtrip test for RunCancelled with reason
- **journal.rs**: Updated mapping to pass reason through

### 4. IPC Layer (vb_ipc)
- **payloads.rs**: Added `reason: Option<String>` to `IpcPayload::CancelRun`
- **server/handlers.rs**: Updated `handle_cancel_run` to decode and pass reason

### 5. UI Layer (vb_ui)
- **incident/screen.rs**: Fixed pattern matches for `attempt` field
- **replay/controller.rs**: Fixed event constructors for `attempt` field

### 6. Pre-existing fixes
- **commands_verify.rs**: Fixed `vec!` type mismatches and missing VerifyError variants
- **mode_activation_tests.rs**: Fixed `prop_assert!` format string issues

## Implementation Details

### cmd_cancel Algorithm
1. Parse `run_id` → `RunId`
2. Open Fjall journal at `--db` path
3. Read events for the run
4. **Idempotency**: If no events → return success (run never existed)
5. **Idempotency**: If already terminal (Finished/Failed/Cancelled) → return success
6. Compute next sequence number from last event
7. Append `JournalEvent::RunCancelled { run, seq, attempt: 1, reason }`
8. Output structured JSON/JSONL or human-readable confirmation

### Key Design Decisions
- **Journal-first approach**: The CLI writes cancel events directly to the journal. This is consistent with the existing architecture where `submit` writes to the journal and `inspect/events/replay` read from it.
- **Idempotency at all layers**: The CLI checks for terminal states before writing. The shard's `handle_cancel` already silently succeeds for non-existent runs.
- **Reason preservation**: The optional reason flows from CLI → args → journal event → storage codec → journal read-back.

## Test Results
- 6 CLI parsing tests: PASS
- 3 CLI integration tests: PASS
- 2 shard cancel-with-reason tests: PASS
- 1 storage codec roundtrip test: PASS
- Total: 16 cancel-related tests passing

## Contract Clause Mapping
| Clause | Implementation | Tests |
|--------|---------------|-------|
| PRE-001 | db path validation in cmd_cancel | cli_cancel_json_output_contains_success_and_status |
| PRE-002 | parse_run_id in cmd_cancel | parse_cancel_accepts_run_id_and_db |
| PRE-003 | reason length check in parse_cancel | parse_cancel_rejects_reason_longer_than_256_bytes |
| POST-001 | journal.append_journaled(&cancel_event) | cli_cancel_with_reason_persists_to_journal |
| POST-004 | idempotent counter (shard-level) | shard_cancel_with_reason_persists_reason_to_journal |
| POST-005 | empty run check in cmd_cancel | cli_cancel_nonexistent_run_returns_success_idempotent |
| POST-006 | JSON output in cmd_cancel | cli_cancel_json_output_contains_success_and_status |
| INV-003 | terminal state check before write | cli_cancel_with_reason_persists_to_journal (re-cancel) |
| INV-004 | reason cloned into event | cli_cancel_with_reason_persists_to_journal |
| INV-005 | idempotent at CLI + shard + journal | cli_cancel_nonexistent_run_returns_success_idempotent |
