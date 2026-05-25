# Contract Specification: vb-qi37.16.5

## Context

- **Bead ID**: vb-qi37.16.5
- **Title**: cli/runtime: Add lifecycle integration evidence
- **Phase**: State 3 (rust-contract)
- **Domain terms**:
  - `lifecycle command` — one of {cancel, resume, retry, answer} surfaced via CLI args.rs
  - `RuntimeJournalEvent` — durable event recorded by vb_runtime/journal.rs and vb_storage/journal.rs
  - `recovery replay` — reconstruction of runtime state from journal events on restart
  - `invalid transition` — a lifecycle command issued against a bead in a state where that command is not permitted
  - `duplicate request` — the same lifecycle command issued twice for the same bead in the same state
  - `stale request` — a lifecycle command for a bead whose state has already advanced past the expected prior state
  - `structured diagnostics` — error messages containing {code, context, timestamp, bead_id, command}

## Preconditions

- **PRE-001**: The CLI runtime must have a valid, connected storage backend before any lifecycle command is dispatched.
- **PRE-002**: Lifecycle commands must be validated against the current bead state before journal write.
- **PRE-003**: Recovery replay must start from a clean snapshot or empty journal state.

## Postconditions

- **POST-001**: Every accepted lifecycle command produces exactly one corresponding `RuntimeJournalEvent` written to durable storage.
- **POST-002**: A successful replay of the journal reconstructs the exact same bead states that existed at crash time.
- **POST-003**: Invalid-transition requests return a structured diagnostic with error code `E_INVALID_TRANSITION` and never modify state.
- **POST-004**: Duplicate requests return `E_DUPLICATE_REQUEST` and never double-write to the journal.
- **POST-005**: Stale requests return `E_STALE_REQUEST` and never retroactively modify already-advanced state.

## Invariants

- **INV-001**: At any point in time, each bead has exactly one canonical lifecycle state in storage.
- **INV-002**: The journal append-only log is the single source of truth for bead state transitions; in-memory state is replay-derived.
- **INV-003**: No lifecycle command can skip a required antecedent state (e.g., answer requires the bead to be in a state awaiting answer).
- **INV-004**: Restart/replay produces bit-identical bead states to those that existed before the crash.
- **INV-005**: CLI command surface (cancel, resume, retry, answer) is decoupled from storage and runtime concerns via well-defined API boundaries.

## Error Taxonomy

- `Error::InvalidTransition` — command issued against bead in ineligible state
- `Error::DuplicateRequest` — same command already processed for this bead
- `Error::StaleRequest` — bead state has already advanced past expected prior state
- `Error::JournalWriteFailure` — durable write to storage journal failed
- `Error::ReplayCorruption` — journal replay encounters malformed or missing events
- `Error::StorageUnavailable` — storage backend unreachable or connection lost

## Contract Signatures

```rust
// crates/velvet_ballistics/src/args.rs (CLI surface)
fn cancel(bead_id: BeadId) -> Result<(), LifecycleError>;
fn resume(bead_id: BeadId) -> Result<(), LifecycleError>;
fn retry(bead_id: BeadId) -> Result<(), LifecycleError>;
fn answer(bead_id: BeadId, answer: Answer) -> Result<(), LifecycleError>;

// crates/vb_runtime/src/journal.rs
fn append_event(event: RuntimeJournalEvent) -> Result<(), JournalError>;
fn replay() -> Result<Vec<RuntimeState>, ReplayError>;

// crates/vb_storage/src/journal.rs
fn write_event(event: StorageJournalEvent) -> Result<(), StorageError>;
fn read_journal() -> Result<JournalSlice, StorageError>;
```

## TLA+-Owned Clauses

- **INV-002**, **INV-003**, **INV-004** — temporal properties of the lifecycle state machine: append-only journal, valid state transitions, replay produces identical state, no state leakage on crash.
- See `tla-spec.md` for the formal temporal model.

## Verus-Owned Clauses

- **INV-001** — each bead has exactly one canonical state: proven by Verus typestate on `lifecycle.rs` state machine.
- **PRE-002** — command validation before journal write: proven by Verus preconditions on transition functions.
- **POST-001** — exactly-one-journal-event property: proven by Verus postconditions on `append_event`.
- See `lean-contract.md` for Verus obligations and `verification-layers.md` for exact mapping.

## Non-goals

- Production implementation of storage or runtime code (this bead adds integration test evidence only).
- Formal proof of performance or latency budgets (performance evidence is scope: integration/replay only).
- TLA+ model of the full distributed system (bounded to single-node journal replay for this bead).
