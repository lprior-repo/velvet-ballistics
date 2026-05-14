# Contract Specification — vb-qi37.16.4

## Context
- **Bead ID:** vb-qi37.16.4
- **Title:** cli/runtime: Implement durable answer command
- **State:** GoMasterOrchestrator State 3 (contract synthesis only, no implementation)
- **Touched crates:** `velvet_ballastics`, `vb_runtime`, `vb_storage`
- **Domain terms:** AskTicket, AskAnswer, AskAnswered journal event, SlotWritten, answer routing, secret redaction, journal replay, durable answer

## Assumptions
- The `Command::Answer { run_id, step, value_file, db, output }` CLI variant is already declared in `args.rs`.
- `AskTicket` and `AskAnswer` types are already declared in `vb_runtime/src/shard/types.rs`.
- `handle_ask_answer` exists in `Shard` with journal events `AskAnswered` and `SlotWritten`.
- Runtime trace events for ask-answer diagnostics are declared in `vb_runtime/src/trace.rs`.
- Answer routing lives under `crates/velvet_ballastics/src/storage.rs`.
- Integration tests live under `crates/velvet_ballastics/tests/cli_integration.rs`.

## Open Questions
- Whether the answer value is loaded from `value_file` as raw bytes or as a pre-parsed `SlotValue` handle — **resolve before implementation**.
- Whether `output` file path is optional or mandatory for CLI surface — **confirm before implementation**.
- Whether the durable answer command requires a new IPC command variant or reuses `AnswerAsk` from the IPC protocol — **confirm before implementation**.

---

## Preconditions
- **PRE-001:** The `run_id` refers to an active or suspended run that is in `AwaitingAsk` state.
- **PRE-002:** The `step` index matches the suspended `Ask` step in the run's frame.
- **PRE-003:** The `value_file` (if provided) exists, is readable, and its size is <= `max_ipc_payload_bytes` from the run's `ResourceContract`.
- **PRE-004:** The answer ticket presented by the caller matches the stored `AskTicket` for the suspended step (run_id, step, seq).
- **PRE-005:** The run is not already answered (no duplicate `AskAnswered` for the same ticket seq).
- **PRE-006:** The caller has validated that no secret-tainted payload enters diagnostics without redaction.

## Postconditions
- **POST-001:** The answer value is written to the run's slot via `SlotWritten` journal event before `AskAnswered` is acknowledged.
- **POST-002:** `AskAnswered` journal record is emitted with the answer value (or blob reference) and taint classification.
- **POST-003:** The run transitions from `AwaitingAsk` to the next step index after the answer is applied.
- **POST-004:** The answer is durable: it survives process restart if `journaled` or `strict` durability is active.
- **POST-005:** Diagnostics emitted during answer processing redact any secret-tainted values.

## Invariants
- **INV-001:** No two `AskAnswered` events with the same `(run_id, step, seq)` ticket can be recorded in the journal.
- **INV-002:** The slot value written by an answer must not be `Secret`-tainted unless the workflow's `ResourceContract` explicitly allows secret results.
- **INV-003:** The journal sequence numbers remain monotonic per run before and after the answer is recorded.
- **INV-004:** On journal replay, an already-answered ask ticket is skipped without error (idempotent replay).

## Error Taxonomy
- `Error::RunNotFound` — `run_id` does not exist in the runtime or storage.
- `Error::StepNotAwaitingAsk` — the step is not in `AwaitingAsk` state.
- `Error::TicketMismatch` — presented ticket does not match stored `AskTicket`.
- `Error::DuplicateAnswer` — an `AskAnswered` record already exists for this ticket.
- `Error::PayloadTooLarge` — `value_file` contents exceed `max_ipc_payload_bytes`.
- `Error::ValueFileUnreadable` — `value_file` cannot be read (permission or path).
- `Error::SlotOutOfBounds` — target output slot index is invalid for this run's frame.
- `Error::SecretLeak` — a secret-tainted value would appear in diagnostics without redaction.

## Contract Signatures
```rust
// CLI surface (velvet_ballastics/src/args.rs already declares this variant)
Command::Answer { run_id, step, value_file, db, output }

// Runtime journal event
RuntimeJournalEvent::AskAnswered {
    run: RunId,
    step: StepIdx,
    seq: SeqNo,
    value: SlotValue,
    taint: Taint,
    encoded_len: u32,
}

// Shard command
ShardCommand::AskAnswered {
    run: RunId,
    step: StepIdx,
    ticket: AskTicket,
    answer: AskAnswer,
}

// Result type for all fallible operations
type AnswerResult<T> = Result<T, AnswerError>;
```

## TLA+-Owned Clauses
- **INV-001** (no duplicate AskAnswered) — temporal safety over journal append discipline
- **INV-003** (monotonic seqno) — lifecycle state machine invariant
- **POST-003** (state transition) — explicit next-step advancement after answer
- **INV-004** (idempotent replay) — replay determinism over already-answered tickets

## Verus-Owned Clauses
- **INV-002** (taint enforcement on slot write) — Rust-local pure invariant on `SlotValue` write path
- **PRE-004** (ticket equality check) — pure deterministic equality on `AskTicket` fields
- **PRE-005** (duplicate detection) — pure deterministic dedup check before journal write
- **PRE-003** (payload size bound) — checked arithmetic against `max_ipc_payload_bytes`

## Non-goals
- Workflow compilation or IR validation (separate beads).
- Action completion path (`ActionCompleted`) — separate concern from ask-answer.
- The `ask` step suspension mechanism itself — only the answer command that resumes it.
