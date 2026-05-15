# Contract Specification: vb-qi37.16.3 State 3

## Context

- **Feature**: Durable retry transition for CLI/runtime
- **Bead ID**: vb-qi37.16.3
- **Phase**: State 3 - Contract/Verification
- **Domain terms**:
  - `ActionTicket` - opaque handle tracking run, step, seq, action, attempt, idempotency_key, capacity
  - `ActionFailure` - failure code, retry_policy (Retryable | NonRetryable), taint, detail
  - `RetryPolicy` - max_attempts (u16), base_delay_ms (u32), exponential_backoff (bool)
  - `ActionFailureOutcome` - RetryNow | DriveHandler | FailRun
  - `RuntimeJournalEvent::ActionFailed` - evidence event for durable journal
  - `ShardCommand::ActionFailed` - CLI-facing command variant for `velvet-ballastics retry`
- **Assumptions**:
  - Retry is only applicable to steps with a succeeding `RetryCheck` node in the workflow graph
  - `VbCoreRetryPolicy::Retryable` gates retry; `NonRetryable` bypasses retry logic
  - `action_attempts[step]` tracks the current attempt counter for each step (0 = unscheduled)
  - Journal replay is deterministic and idempotent: duplicate `ActionFailed` events do not corrupt state
- **Open questions**: None for State 3 scope

## Preconditions

- **PRE-001**: `handle_action_failure` caller must hold a valid `ActionTicket` for an active run.
- **PRE-002**: `ticket.attempt >= 1` and `ticket.capacity >= 1`; `ticket.attempt <= ticket.capacity`.
- **PRE-003**: The run referenced by `ticket.run` must exist in `self.runs`.
- **PRE-004**: `retry_is_available` requires `VbCoreRetryPolicy::Retryable` and `retry_metadata_exists(state, ticket.step) == true`.

## Postconditions

- **POST-001**: When `retry_is_available` returns `true`, `apply_action_failure_to_state` sets PC to `ticket.step` and returns `ActionFailureOutcome::RetryNow`.
- **POST-002**: When `retry_is_available` returns `false` and an error handler exists, `apply_error_handler` writes the error slot and sets PC to the handler step, returning `ActionFailureOutcome::DriveHandler`.
- **POST-003**: When `retry_is_available` returns `false` and no error handler exists, `apply_error_handler` returns `ActionFailureOutcome::FailRun`.
- **POST-004**: `handle_action_failure` emits exactly one `RuntimeJournalEvent::ActionFailed { run, step, action }` to the journal before returning.
- **POST-005**: `ticket_with_retry_capacity` returns the original ticket unchanged when `retry_metadata_exists` is false; otherwise returns a ticket with `capacity = max(ticket.capacity, policy.max_attempts)`.
- **POST-006**: After `record_retry_attempt`, `action_attempts[ticket.step] >= ticket.attempt`; when `attempt >= max_attempts`, `record_retry_attempt` returns `Ok(false)`.
- **POST-007**: Stale attempt completions (incoming attempt < current attempt) are rejected with `RuntimeError::StaleAttempt` and leave run state (frame, counters, journal) unchanged.

## Invariants

- **INV-001**: `action_attempts[step]` is monotonically non-decreasing within a run.
- **INV-002**: A step that has emitted `ActionFailed` with `Retryable` policy may emit `ActionFailed` again only if `attempt < max_attempts`.
- **INV-003**: `RuntimeJournalEvent::ActionFailed` events for a given `(run, step)` are append-only and replay-deterministic.
- **INV-004**: `handle_action_failure` never modifies the slot values written by a prior successful `ActionCompleted` for the same step.
- **INV-005**: When `retry_is_available` is true, the frame PC is reset to the failed step (not advanced), enabling retry.

## Error Taxonomy

- `RuntimeError::RunNotFound` - ticket.run is not in self.runs
- `RuntimeError::InvalidActionCompletion` - ticket validation fails (step not Running, action mismatch, out-of-bounds)
- `RuntimeError::AttemptBeyondMax { attempt, max }` - ticket.attempt == 0, capacity == 0, or attempt > capacity
- `RuntimeError::StaleAttempt { incoming, current }` - incoming attempt < current recorded attempt
- `RuntimeError::UnsupportedOperation { operation }` - retry_policy_attempts_zero, retry_metadata_missing, retry_policy_slot_unreadable, retry_attempt_overflow

## Contract Signatures

```rust
// vb_runtime/src/shard/lifecycle.rs

fn retry_is_available(
    state: &mut RunState,
    ticket: ActionTicket,
    retry_policy: VbCoreRetryPolicy,
) -> RuntimeResult<bool>

fn apply_error_handler(
    state: &mut RunState,
    ticket: ActionTicket,
) -> RuntimeResult<ActionFailureOutcome>

fn apply_action_failure_to_state(
    &mut self,
    ticket: ActionTicket,
    failure: ActionFailure,
) -> RuntimeResult<ActionFailureOutcome>

fn ticket_with_retry_capacity(
    &self,
    ticket: ActionTicket,
    retry_policy: VbCoreRetryPolicy,
) -> RuntimeResult<ActionTicket>

pub(crate) fn handle_action_failure(
    &mut self,
    ticket: ActionTicket,
    failure: ActionFailure,
) -> RuntimeResult<()>
```

## Verus-Owned Clauses

- **INV-001**: `action_attempts[step]` monotonicity proven by Verus in `vb_runtime/src/shard/helpers.rs::record_retry_attempt`
- **INV-005**: PC reset correctness proven by Verus for `apply_action_failure_to_state`
- **PRE-002**: Ticket attempt/capacity bounds proven by Verus in `validate_ticket_attempt`

## TLA+-Owned Clauses

- **INV-002**: Retry exhaustion finite-state machine model-checked by TLA+
- **INV-003**: Journal idempotency and replay determinism model-checked by TLA+
- **POST-004**: ActionFailed event emission ordering model-checked by TLA+

## Non-goals

- End-to-end durable storage integration (Fjall/vb_storage) is out of scope for this contract
- Network delivery guarantees for action boundary are out of scope