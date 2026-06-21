# RS-105-life: Action failure handling mutates retry/handler state before journaling

- **Severity**: High
- **Category**: correctness
- **Location**: `crates/vb_runtime/src/shard/lifecycle/chunk_001_action.rs:78`
- **Confidence**: confirmed

## Description

`handle_action_failure` applies retry or error-handler mutations before appending the `ActionFailed` journal event. If the append fails, the run has already consumed retry state or jumped to an error handler without a durable failure record.

## Evidence

The mutable state transition happens before journaling:

```rust
let ticket = self.ticket_with_retry_capacity(ticket, failure.retry_policy)?;
let outcome = self.apply_action_failure_to_state(ticket, failure)?;
...
self.append_journal_event(RuntimeJournalEvent::ActionFailed {
    run,
    step: ticket.step,
    action: ticket.action,
    attempt: ticket.attempt,
})?;
```

The retry path records an attempt before returning:

```rust
crate::shard::helpers::record_retry_attempt(state, ticket, policy)
```

The handler path marks the failed step, writes the error slot, and sets the program counter before returning:

```rust
state.frame.mark_failed(ticket.step)?;
write_failure_slot(state, ticket.step, error_slot)?;
state.frame.set_pc(handler)?;
```

## Adversarial Check

This is not merely a trace-order issue. `apply_action_failure_to_state` takes `&mut RunState` and performs real frame/retry mutations before the fallible journal append. The later `drive_run` is not reached if `ActionFailed` cannot be appended, and no rollback is attempted.

## Suggested Fix

Split failure handling into a pure preflight that determines retry/handler outcome, append the `ActionFailed` record first, then apply the retry or handler mutation. If the handler writes an error slot, make that slot write durable with an explicit journal record or derive it deterministically during replay from the failure event.
