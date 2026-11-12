# Rust Contract: Run-Attempt Ticket Authority

## Problem

After a durable run failure and explicit retry (`recover_retry`), the runtime re-executes
from the failed step. The old `ActionTicket` from the first attempt is EXACTLY equal to
the new ticket issued for the second attempt — same `run`, `step`, `seq`, `action`,
`attempt` (per-action retry count, always 1 on fresh attempt), `idempotency_key`, and
`capacity`.

This means an old completion or failure callback can be replayed against the new attempt
and accepted, violating the requirement that stale callbacks must be rejected with exact
expected/found values.

## Root Cause

`ActionTicket` has no `run_attempt` field. The `attempt` field is used for the per-action
retry count within a single run attempt. The run-level attempt counter exists in
`RunState.run_attempt` but is never bound to the ticket, so tickets issued for different
run attempts are indistinguishable.

## Design Decision

Add an explicit `run_attempt: u16` field to `ActionTicket`. This separates:
- **Run attempt** (`run_attempt`): increments on each durable run retry. Differentiates
  authority across retry boundaries. Bound to current `RunState.run_attempt`.
- **Per-action attempt** (`attempt`): increments on each retry of the same action within
  the same run attempt. Bound to `state.action_attempts[step]`.

The idempotency key computation MUST include `run_attempt` so that tickets from different
run attempts have different idempotency keys, preventing old callbacks from being accepted
as new.

## API Changes

### `ActionTicket` struct (vb_core/src/action/ticket.rs)

```rust
pub struct ActionTicket {
    pub run: RunId,
    pub step: StepIdx,
    pub seq: SeqNo,
    pub action: ActionId,
    pub run_attempt: u16,      // NEW: durable run attempt number
    pub attempt: u16,          // existing: per-action retry count
    pub idempotency_key: u128,
    pub capacity: u16,
}
```

### `compute_action_idempotency_key`

New signature includes run_attempt:
```rust
pub fn compute_action_idempotency_key(run: RunId, run_attempt: u16, seq: SeqNo, action: ActionId) -> u128
```

### `issue_action_ticket`

New parameter for run_attempt:
```rust
pub fn issue_action_ticket(run: RunId, step: StepIdx, seq: SeqNo, action: ActionId,
                           run_attempt: u16, attempt: u16, idempotency_key: u128,
                           capacity: u16) -> ActionTicket
```

## Propagation Points

Every production issuance, validation, serialization, and recovery path must be updated:

1. **Ticket issuance** (engine action dispatch): pass `state.run_attempt.get()`
2. **Idempotency key computation**: include `run_attempt`
3. **Journal events** (`ActionScheduledTicket`, `ActionCompletedTicket`, `ActionFailedTicket`):
   preserve `run_attempt` in the ticket payload
4. **Action resume validation** (`resolve_action_resume`): validate ticket's `run_attempt`
   matches current `RunState.run_attempt`; reject stale with `StaleAttempt { expected, found }`
5. **Recovery/retry** (`recover_retry`): pass new `run_attempt` through ticket issuance
6. **AskTicket authority**: inspect for same defect (its `boundary_seq` may already distinguish)

## Rejection Semantics

A completion or failure callback with a stale `run_attempt` must be rejected BEFORE any
frame/journal mutation with a typed error carrying exact `expected` and `found` values.
No mutation, no side effects, no state change.

## What Is NOT Changed

- Per-action retry policy (`attempt`/`capacity` semantics remain)
- Max attempts guards
- Successful prefix behavior (completions within same run attempt)
- AskTicket (unless proven to have same defect)
