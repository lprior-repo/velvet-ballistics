# Contract: vb-b8i8f Cancel/Kill Lattice Recovery

## Acceptance Contract

Downstream states must implement and verify the following behavior-affecting requirements.

### C1 Public Kill API

- `Runtime::kill_run(&self, run: RunId) -> RuntimeResult<()>` is required.
- It must route to the owning shard and use `ShardCommand::Kill` or an equivalent typed command.
- It must not require YAML, JSON, HTTP, or text command routing.

### C2 Cancel/Kill Missing and Already-Terminal Semantics

- Cancel/kill for a missing run returns typed error.
- Cancel/kill for an already terminal run returns typed error.
- Either case appends no terminal journal event, emits no terminal trace, increments no terminal counter, and does not corrupt per-run sequence state.
- Returning `Ok(())` for those cases violates this contract.

### C3 Single Terminal Journal Event

- A live run may append exactly one terminal event.
- Terminal events are mutually exclusive: `RunFinished`, `RunFailed`, `RunCancelled`, `RunKilled`.
- Once a terminal event wins, every later terminalization attempt is rejected.

### C4 Stale Action/Timer Cleanup

- Successful cancel/kill removes pending timers for the run.
- Successful cancel/kill invalidates live action/ask/timer authority by removing live run state and marking terminal.
- Stale timer fires, action completions/failures, ask answers, and resumes after terminalization must not mutate slots, frames, journal, trace, or counters.

### C5 Durable Kill Storage Admission

- `RecordKind::RunKilled.id()` remains `28`.
- `is_known_record_kind(28)` must be true.
- `validate_kind_family(MAGIC_JOURNAL_EVENT, 28)` must return `Ok(())`.
- Runtime mapping from `RuntimeJournalEvent::RunKilled` to `JournalEvent::RunKilled` must encode and decode successfully under the journal event envelope.
- Prior `RecordKindFamilyMismatch { kind: 28 }` evidence must become impossible for valid `RunKilled` journal records.

### C6 Replay Integrity

- Run event replay remains contiguous per `EventSeq`.
- Adding kind 28 must not weaken unknown-kind or wrong-family rejection for other values.
- Killed terminal events replay as terminal and do not permit later side-effect re-execution.

## Non-Goals

- No implementation in State 3.
- No behavior tests or verifier harnesses in State 3.
- No broad storage migration beyond admitting the already-declared `RunKilled=28` kind.
- No distributed/runtime replication semantics.

## Bridge Pointers for Later States

- Public API: `crates/vb_runtime/src/runtime.rs`.
- Shard lifecycle: `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs`.
- Commands/private state: `crates/vb_runtime/src/shard/types.rs`.
- Runtime-to-storage event mapping: `crates/vb_runtime/src/journal/chunk_002.rs`.
- Storage validation: `crates/vb_storage/src/codec/validation.rs`.
- Storage record/event definitions: `crates/vb_storage/src/records.rs`, `crates/vb_storage/src/events.rs`.
- Registered workspace target: `crates/workspace_tests/tests/cancel_kill_lattice_tests.rs`.
