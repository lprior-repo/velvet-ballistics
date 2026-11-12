# TLA+ Temporal Model Plan: vb-qi37.16.3 State 3 Durable Retry Transition

## Boundary

- **Temporal/workflow behavior**:
  - Retry state machine: `Pending` → `Running` → `Failed` → (`RetryNow` | `Exhausted`) → `Running` → ...
  - Action failure handling and retry decision
  - Journal event emission ordering guarantees
  - Stale completion rejection
- **Rust/core behavior excluded from TLA+ and handled by Verus/tests**:
  - `validate_ticket_attempt` arithmetic bounds (Verus)
  - `retry_policy_after_action` slot reading (Verus + unit tests)
  - `record_retry_attempt` monotonic counter update (Verus)
  - Frame slot write semantics (unit tests)
- **External systems abstracted**:
  - External action boundary: emits `ActionFailure` with retry_policy
  - Journal writer: append-only, deterministic replay
  - Deterministic engine: step execution, PC manipulation
- **Non-applicability rationale**: N/A - temporal model applies

## TLA+-Owned Clauses

- `INV-002` -> `specs/RetryFSM.tla::NoDoubleRetryAfterExhaustion`
- `INV-003` -> `specs/RetryJournal.tla::JournalIdempotency`
- `POST-004` -> `specs/RetryJournal.tla::ActionFailedEventOrder`

## Model Shape

- **Module/model path**: `specs/RetryFSM.tla`, `specs/RetryJournal.tla`
- **Variables**:
  - `runs`: set of active RunId
  - `actionAttempts`: function from (RunId, StepIdx) to current attempt counter (Nat)
  - `framePC`: function from RunId to StepIdx (current program counter)
  - `stepState`: function from (RunId, StepIdx) to {Pending, Running, Succeeded, Failed}
  - `journal`: sequence of RuntimeJournalEvent
  - `maxAttempts`: constant per-step max_attempts from workflow
  - `retryPolicy`: constant per-step {Retryable, NonRetryable}
  - `stepHasRetryCheck`: boolean constant per step

- **Init action**:
  ```
  Init ==
      /\ runs = {}
      /\ actionAttempts = [run \in runs |-> [step \in Steps |-> 0]]
      /\ framePC = [run \in runs |-> entry]
      /\ stepState = [run \in runs |-> [step \in Steps |-> Pending]]
      /\ journal = <<>>
  ```

- **Next/actions**:
  ```
  ActionFailed(run, step, attempt, failure) ==
      /\ run \in runs
      /\ stepState[run][step] = Running
      /\ IF failure.retry_policy = NonRetryable \/ ~stepHasRetryCheck[run][step]
         THEN \* No retry - either exhausted or non-retryable
              /\ stepState' = [stepState EXCEPT ![run][step] = Failed]
              /\ journal' = Append(journal, ActionFailedEvent(run, step))
              /\ actionAttempts' = actionAttempts
         ELSE \* Retryable and retry metadata exists
              LET available == actionAttempts[run][step] < maxAttempts[run][step]
              IN
              IF available
              THEN \* Retry allowed
                   /\ framePC' = [framePC EXCEPT ![run] = step]
                   /\ stepState' = [stepState EXCEPT ![run][step] = Running]
                   /\ actionAttempts' = [actionAttempts EXCEPT ![run][step] = actionAttempts[run][step] + 1]
                   /\ journal' = Append(journal, ActionFailedEvent(run, step))
              ELSE \* Exhausted - fail the run
                   /\ stepState' = [stepState EXCEPT ![run][step] = Failed]
                   /\ journal' = Append(journal, ActionFailedEvent(run, step))

  StaleCompletionRejected(run, step, staleAttempt, currentAttempt) ==
      /\ staleAttempt < currentAttempt
      /\ stepState[run][step] = Running
      /\ journal' = journal  \* No journal modification
      /\ UNCHANGED <<runs, actionAttempts, framePC, stepState>>
  ```

- **State constraints**:
  - `Len(journal) <= 1000` for TLC bounded model
  - `actionAttempts[run][step] <= maxAttempts[run][step] + 1` (allows one beyond for Exhausted detection)
  - `runs` finite for model checking

- **Symmetry sets**:
  - `runs` symm set (model-specific)
  - `Steps` symm set (model-specific)

- **Bounded model limits**:
  - 3 runs max
  - 5 steps per run
  - 3 max_attempts per retry step

## Properties

### Safety Invariants

- **NoDoubleRetryAfterExhaustion**: For any (run, step), once `actionAttempts[run][step] >= maxAttempts[run][step]`, no further retry transitions are allowed; the next ActionFailed must result in Failed state.
- **NoStaleCompletion**: A completion with attempt N is only accepted when `actionAttempts[run][step] = N-1` (or N when N=1).
- **JournalIdempotency**: Appending the same `ActionFailed` event twice does not change observable state beyond the duplicate event in the journal.
- **FramePCResetOnRetry**: When retry is available, `framePC[run]` is reset to the failed step, not advanced.

### Liveness/Eventuality

- **EventuallyTerminalOrExhausted**: Every retryable failed action eventually reaches either a successful completion or exhaustion (Failed state with no further retries).
- **EventuallyJournalAppended**: Every `ActionFailed` call results in a journal append before the handler returns.

### Fairness Assumptions

- Weak fairness on `ActionFailed` transitions when retry is available
- No fairness required for `StaleCompletionRejected` (error path)

### Deadlock Freedom

- No deadlocks in retry state machine (Pending → Running → Failed → RetryNow → Running is acyclic within attempt bounds)

## Evidence Command

```bash
# TLC model check for RetryFSM - NoDoubleRetryAfterExhaustion invariant
tlc -config specs/RetryFSM.cfg specs/RetryFSM.tla

# TLC model check for RetryJournal - JournalIdempotency and ActionFailedEventOrder invariants
tlc -config specs/RetryJournal.cfg specs/RetryJournal.tla
```

## Implementation Status

**Created**: `specs/RetryFSM.tla`, `specs/RetryJournal.tla`, `specs/RetryFSM.cfg`, `specs/RetryJournal.cfg`

These specs were missing at State 12 (formal-verifier rejected obligations). Repair created these files at the exact paths referenced in `proof-obligations.jsonl`.

## Refinement to Rust/runtime Behavior

- **Rust `handle_action_failure` refines TLA+ `ActionFailed` action**:
  - Rust run lookup corresponds to `run \in runs` guard
  - Rust `retry_metadata_exists` corresponds to `stepHasRetryCheck` constant
  - Rust `retry_is_available` corresponds to `actionAttempts[run][step] < maxAttempts[run][step]` guard
  - Rust `action_attempts[step]` counter refines `actionAttempts` variable
  - Rust PC reset (`frame.set_pc(ticket.step)`) refines `framePC' = [framePC EXCEPT ![run] = step]`
  - Rust journal append refines `journal' = Append(journal, ActionFailedEvent(...))`

- **Rust `validate_ticket_attempt` refines TLA+ `StaleCompletionRejected` guard**:
  - Rust `ticket.attempt < current` returns `RuntimeError::StaleAttempt` refines TLA+ `staleAttempt < currentAttempt` condition
  - Error return leaves all state unchanged, matching TLA+ `UNCHANGED <<...>>`

## Waivers

None - temporal model applies to all retry-related temporal behavior in this bead.