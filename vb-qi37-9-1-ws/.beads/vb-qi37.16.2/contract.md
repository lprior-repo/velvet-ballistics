# Contract Specification

## Context
- Feature: cli/runtime: Implement durable resume transition
- Bead ID: vb-qi37.16.2
- Phase: state-3 (contract synthesis)
- Touched Crates: velvet_ballastics, vb_runtime, vb_storage
- Public APIs: Command::Resume, ShardCommand::Resume, Shard::handle_resume, RuntimeJournalEvent
- Release Critical: true
- Risk Tags: p0, durability, journal-replay, cli-runtime-boundary, state-transition

## Domain Terms
- **Resumable State**: A RuntimeState variant from which resume is a valid transition (not Initial, not Running, not Failed)
- **Incomplete Hydration**: RuntimeJournalEvent sequence is missing required events for the requested run_id
- **Journal Evidence**: RuntimeJournalEvent appended to journal log before resume success is reported
- **Structured Result Output**: Machine-parseable resume result (run_id, status, message)

## Assumptions
- Journal replay is the source of truth for runtime state hydration
- A run_id uniquely identifies a resumable execution context
- Resume transition is idempotent at the journal layer (replay of Resume event is safe)
- CLI passes run_id through the full stack: Command::Resume -> ShardCommand::Resume -> Shard::handle_resume

## Open Questions
- None

---

## Preconditions
- PRE-001: The caller must provide a run_id that exists in the runtime journal
- PRE-002: The runtime state for the given run_id must be in a Resumable variant (not Initial, not Running, not Failed)
- PRE-003: Journal hydration for run_id must be complete (all prior events are present and reconstructable)

## Postconditions
- POST-001: On successful resume, the runtime transitions from Resumable -> Running and RuntimeJournalEvent::Resumed is appended to the journal before success is returned
- POST-002: On successful resume, structured output is produced containing run_id, status="resumed", and timestamp
- POST-003: On failed resume (due to PRE violation), the runtime remains in the original state and an appropriate Error variant is returned
- POST-004: Journal evidence (RuntimeJournalEvent::Resumed) is append-only and durable before success is reported to the caller

## Invariants
- INV-001: The runtime state machine never transitions to Running except via a valid Resume transition from Resumable
- INV-002: Journal events are never reordered, deleted, or modified after append
- INV-003: Resume result output always contains run_id, status, and timestamp fields
- INV-004: A run_id in Failed state is not resumable (resume from Failed returns Error::NotResumable)

## Error Taxonomy
- Error::RunIdNotFound - run_id does not exist in journal (PRE-001 violation)
- Error::NotResumable - runtime state is not Resumable (PRE-002 violation)
- Error::IncompleteHydration - journal hydration check failed (PRE-003 violation)
- Error::JournalAppendFailed - failed to append RuntimeJournalEvent::Resumed (durability violation)
- Error::StructuredOutputFailed - CLI output formatting failed (non-fatal, returns partial result with error tag)

## Contract Signatures
```rust
// CLI layer
enum Command { Resume { run_id: RunId, db: PathBuf, output: OutputFormat } }

// Runtime shard layer
enum ShardCommand { Resume { run: RunId } }
impl Shard {
    fn handle_resume(run: RunId) -> Result<ResumeResult, ResumeError>;
}

// Journal layer
enum RuntimeJournalEvent { Resumed { run_id: RunId, timestamp: UtcDateTime } }
trait RuntimeJournal {
    fn append(&mut self, event: RuntimeJournalEvent) -> Result<(), JournalError>;
    fn get_state(&self, run_id: RunId) -> Result<RuntimeState, JournalError>;
    fn is_hydration_complete(&self, run_id: RunId) -> bool;
}

// Result types
struct ResumeResult { run_id: RunId, status: ResumeStatus, timestamp: UtcDateTime }
enum ResumeStatus { Resumed, AlreadyRunning }
enum ResumeError {
    RunIdNotFound(RunId),
    NotResumable { run_id: RunId, current_state: RuntimeState },
    IncompleteHydration(RunId),
    JournalAppendFailed(JurnalError),
    StructuredOutputFailed(OutputError),
}
```

## Verus-Owned Clauses
- INV-001: Runtime state machine transition validity (pure state transition logic)
- INV-002: Journal append-only invariant (pure data structure invariant)
- INV-003: ResumeResult field presence (typestate/property)
- PRE-002, PRE-003: Resumability and hydration completeness checks (predicates on RuntimeState and journal)
- POST-001, POST-004: Journal append ordering guarantee (temporal ordering in pure computation)

## TLA+-Owned Clauses
- INV-001: State machine transition safety across the full lifecycle (deadlock freedom, valid state transitions)
- POST-001: Journal append-before-success temporal ordering (eventuality/liveness)
- INV-002: Journal immutability across concurrent access (concurrent journal safety)
- POST-003: Fail-closed behavior on invalid resume requests (error safety)

## Theorem-Owned Clauses
- None. Rust-local proof obligations are handled by Verus. No tiny algebraic kernel extraction required.

## Non-goals
- Persistence backend specific guarantees (FJALL/LSM-tree internals are out of scope for formal proof)
- Network/distributed resume coordination (single-node runtime only)
- Crash recovery from partial journal writes (handled at storage layer, not runtime layer)
