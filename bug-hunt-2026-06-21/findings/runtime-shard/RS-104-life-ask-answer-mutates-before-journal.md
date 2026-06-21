# RS-104-life: Ask answer mutates frame and timer state before journaling

- **Severity**: High
- **Category**: correctness
- **Location**: `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs:42`
- **Confidence**: confirmed

## Description

`handle_ask_answer` writes the answer slot, advances the program counter, and removes the pending timer before appending the `SlotWritten`, `AskAnswered`, and `StepSucceeded` journal events. A journal failure leaves unjournaled in-memory progress and no pending ask authority.

## Evidence

The state mutation and timer removal precede all three journal appends:

```rust
state
    .frame
    .write_slot_with_taint(answer.answer_slot, answer.value, answer.taint)
    .map_err(|_| RuntimeError::RunNotFound)?;
state
    .frame
    .set_pc(answer.ticket.resume_step)
    .map_err(|_| RuntimeError::RunNotFound)?;
self.pending_timer_remove(run);
```

The first durable write happens later:

```rust
self.append_journal_event(RuntimeJournalEvent::SlotWritten { ... })?;
```

`AskAnswered` and `StepSucceeded` are appended after that at `chunk_002.rs:67-77`.

## Adversarial Check

This is a real partial-commit path because each append uses `?` and there is no rollback after any failure. Even if the first append succeeds, failure on `AskAnswered` or `StepSucceeded` still leaves the frame advanced and the timer removed without a complete durable answer sequence.

## Suggested Fix

Pre-encode and validate the answer, append the durable answer events before mutating the frame, or introduce a transaction helper that can roll back the slot write, program counter, and pending timer on journal failure. Remove the pending timer only after the answer sequence is durable.
