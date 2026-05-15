bead_id: vb-qi37.16.1
bead_title: cli/runtime: Implement durable cancel transition
phase: 3
updated_at: 2026-05-09T00:00:00Z

# Contract Specification: Durable Cancel Transition

## Context
- Feature: Add `cancel` CLI command with full runtime state transitions, journal evidence, cancellation reason, and idempotent repeated-cancel behavior.
- Domain terms:
  - **Cancel**: Transition a run from active/queued/suspended to terminal cancelled state.
  - **Cancellation reason**: Optional human-readable or programmatic string explaining why the run was cancelled.
  - **Journal evidence**: A persistent `RunCancelled` journal event recorded in Fjall storage.
  - **Idempotent cancel**: Repeated cancellation of the same run ID must succeed silently without side effects (no duplicate journal events, no counter increments).
- Assumptions:
  - The runtime shard already supports `ShardCommand::Cancel` with idempotent semantics.
  - The storage layer already has `JournalEvent::RunCancelled` but without a reason field.
  - The IPC layer already has `IpcPayload::CancelRun` but without a reason field.
- Open questions: None.

## Preconditions
- PRE-001: The CLI `--db` path must point to a valid Fjall journal directory if `--durability` is not `none`.
- PRE-002: The `run_id` argument must be a parseable non-zero `RunId`.
- PRE-003: If `--reason` is provided, its length must not exceed 256 UTF-8 bytes.

## Postconditions
- POST-001: If the run exists in the shard, a `RunCancelled` journal event is persisted with the run ID, sequence number, attempt number, and optional reason.
- POST-002: If the run exists, the run is removed from the shard's active run map and its frame is returned to the pool.
- POST-003: If the run exists, a `TraceEvent::RunCancelled` is pushed to the shard's trace ring.
- POST-004: If the run exists, the shard's failed counter is incremented exactly once per distinct run cancellation.
- POST-005: If the run does not exist (already cancelled, finished, or never submitted), the command returns success with no journal event, no trace event, and no counter increment.
- POST-006: The CLI outputs structured JSON/JSONL when requested, including `success: true`, `run_id`, and `status: "cancelled"`.
- POST-007: The CLI outputs human-readable text by default confirming cancellation.

## Invariants
- INV-001: A run can transition to cancelled from any non-terminal state (active, queued, suspended, waiting).
- INV-002: A run in a terminal state (finished, failed, already cancelled) cannot be affected by cancel; the operation is a no-op.
- INV-003: The journal never contains duplicate `RunCancelled` events for the same run ID from the same execution.
- INV-004: The cancellation reason, if provided, is preserved exactly in the journal event without truncation or mutation.
- INV-005: Cancel is always safe to retry (idempotent at all layers: CLI, runtime, shard, journal).

## Error Taxonomy
- `CliError::InvalidRunId` — When the provided run_id cannot be parsed as a non-zero RunId.
- `CliError::StorageOpenFailed` — When the Fjall journal cannot be opened at the given `--db` path.
- `CliError::RuntimeEnqueueFailed` — When the runtime shard queue is full and the cancel command cannot be enqueued.
- `CliError::ReasonTooLong` — When `--reason` exceeds 256 UTF-8 bytes.

## Contract Signatures

```rust
// CLI layer
fn cmd_cancel(
    run_id: &str,
    db: &Path,
    reason: Option<&str>,
    output: OutputFormat,
) -> ExitCode;

// Args layer
fn parse_cancel(args: &[OsString]) -> Result<Command, ParseError>;

// Runtime layer (existing, extended)
pub fn cancel_run(&self, run: RunId, reason: Option<String>) -> RuntimeResult<()>;

// Shard layer (existing, extended)
pub(crate) fn handle_cancel(&mut self, run: RunId, reason: Option<String>) -> RuntimeResult<()>;

// Journal event (extended)
RuntimeJournalEvent::RunCancelled { run: RunId, reason: Option<String> }

// Storage event (extended)
JournalEvent::RunCancelled { run: RunId, seq: EventSeq, attempt: u16, reason: Option<String> }

// IPC payload (extended)
IpcPayload::CancelRun { run_id: RunId, reason: Option<String> }
```

## Non-goals
- Do NOT implement async cancellation or in-flight action interruption.
- Do NOT implement batch/multi-run cancellation in this bead.
- Do NOT implement cancellation rollback or undo.
- Do NOT change the semantics of existing cancel behavior in the shard (idempotency must be preserved).
