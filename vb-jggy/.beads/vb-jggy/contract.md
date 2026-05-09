# Contract Specification: vb-jggy

## Context

- **Feature**: Persist execution attempt numbers and reject stale completions
- **Domain terms**:
  - `RunState`: Shard-owned mutable run state; holds `action_attempts: Box<[u16]>` — per-step monotonic attempt counters
  - `ActionTicket`: Issued by engine per action dispatch; carries `attempt: u16` and `capacity: u16`
  - `RuntimeJournalEvent`: Durable event written to Fjall journal; completion/failure variants must carry attempt
  - `RuntimeError::StaleAttempt { incoming, current }`: Existing typed error for rejected stale completions
  - `RuntimeError::AttemptBeyondMax { attempt, max }`: Existing typed error for policy exhaustion
  - `validate_ticket_attempt(state, ticket)`: Pure helper that rejects stale attempt before journal mutation
  - `record_scheduled_attempt(state, ticket)`: Updates per-step counter to max(current, ticket.attempt)
- **Assumptions**:
  - `RuntimeError::StaleAttempt` and `RuntimeError::AttemptBeyondMax` already exist in `vb_runtime::RuntimeError`
  - `validate_ticket_attempt` is already implemented and correct
  - `ShardCommand::ActionCompleted` and `ShardCommand::ActionFailed` are the two completion paths
  - `RuntimeJournalEvent::StepSucceeded` and `RuntimeJournalEvent::StepFailed` (or equivalent) are the durable event types
  - Run admission creates `RunState` with zero-initialized `action_attempts`
- **Open questions**: None

---

## Preconditions

- **PRE-001**: Run admission path (`handle_submit_with_inputs`) must have located and read the master-doc Section 72 execution attempt contract before implementation.
- **PRE-002**: Action completion path (`handle_action_completion`) must have located the `validate_ticket_attempt` gate before any journal mutation.
- **PRE-003**: Action failure path (`handle_action_failure`) must have the same stale-attempt gate as completion.
- **PRE-004**: A typed `StaleAttempt` error variant exists in `RuntimeError` and is used as the rejection type.

---

## Postconditions

- **POST-001**: `RunState::action_attempts` is zero-initialized at admission and persists as the authoritative latest-attempt counter per step.
- **POST-002**: Every `ActionTicket` issued carries `attempt = 1` for the first dispatch, and incremented attempts for retries.
- **POST-003**: Every `RuntimeJournalEvent::StepSucceeded` and `RuntimeJournalEvent::StepFailed` written to durable storage carries the `attempt` number from the originating ticket.
- **POST-004**: `validate_ticket_attempt` is called **before** any `journal.append(...)` call in the completion and failure paths.
- **POST-005**: A stale completion (attempt < current per-step counter) returns `Err(RuntimeError::StaleAttempt { .. })` **before** any state mutation or journal write.
- **POST-006**: `record_scheduled_attempt` is called when a ticket is issued, advancing the per-step counter monotonically.

---

## Invariants

- **INV-001**: A run has exactly one latest accepted attempt per step at any time; `action_attempts[step] >= 1` after first dispatch.
- **INV-002**: Older attempts cannot win after a newer attempt is admitted: if `action_attempts[step] = N`, all events with attempt < N are rejected.
- **INV-003**: Attempt checks (`validate_ticket_attempt`) happen before durable mutation (journal append or state frame mutation).
- **INV-004**: The monotonicity property: `action_attempts[step]` never decreases across the lifetime of a run.

---

## Error Taxonomy

- `RuntimeError::StaleAttempt { incoming: u16, current: u16 }` — arrival attempt is older than the currently recorded attempt; rejected before mutation.
- `RuntimeError::AttemptBeyondMax { attempt: u16, max: u16 }` — ticket attempt exceeds retry policy capacity; rejected at scheduling time.
- `RuntimeError::InvalidActionCompletion` — step not in `Running` state, or step/action mismatch.
- `RuntimeError::RunNotFound` — run does not exist in shard's `runs` map.
- `RuntimeError::EncodeFailed` — postcard serialization failure for journal payload.

---

## Contract Signatures

```rust
// helpers.rs — pure validation kernel
fn validate_ticket_attempt(state: &RunState, ticket: ActionTicket) -> RuntimeResult<()>;

fn record_scheduled_attempt(state: &mut RunState, ticket: ActionTicket);

// lifecycle.rs — run admission
pub(crate) fn handle_submit_with_inputs(
    &mut self,
    run: RunId,
    workflow: CompiledWorkflow,
    inputs: &[(SlotIdx, SlotValue)],
    caps: CapabilitySet,
) -> RuntimeResult<()>;

// lifecycle.rs — action completion (stale check before journal)
pub(crate) fn handle_action_completion(
    &mut self,
    ticket: ActionTicket,
    output: ActionOutputReady,
) -> RuntimeResult<()>;

// lifecycle.rs — action failure (stale check before journal)
pub(crate) fn handle_action_failure(
    &mut self,
    ticket: ActionTicket,
    failure: ActionFailure,
) -> RuntimeResult<()>;

// journal.rs — event types carrying attempt
enum RuntimeJournalEvent {
    StepSucceeded { run: RunId, step: StepIdx, attempt: u16 },
    StepFailed    { run: RunId, step: StepIdx, attempt: u16, code: u32 },
    // ...
}
```

---

## Non-goals

- Modifying the Fjall keyspace layout or record format
- Changing `ActionTicket` wire encoding or IPC protocol
- Implementing attempt-based replay/recovery (future phase)
- Adding new unsafe code or relaxing existing unsafe prohibitions
