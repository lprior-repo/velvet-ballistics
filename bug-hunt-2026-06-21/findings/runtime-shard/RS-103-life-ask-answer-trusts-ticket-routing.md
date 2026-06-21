# RS-103-life: Ask answers can choose their own output slot and resume step

- **Severity**: High
- **Category**: correctness
- **Location**: `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs:23`
- **Confidence**: confirmed

## Description

`handle_ask_answer` validates only the pending timer's run, ask step, and kind. It then trusts `answer.answer_slot` and `answer.ticket.resume_step` from the incoming answer to mutate the frame and program counter.

## Evidence

The authority check only compares the pending timer against `ask_step` and `PendingTimerKind::Ask`:

```rust
let pending_timer = self
    .pending_timer_get(run)
    .ok_or(RuntimeError::InvalidActionCompletion)?;
if pending_timer.step != answer.ticket.ask_step
    || pending_timer.kind != PendingTimerKind::Ask
{
    return Err(RuntimeError::InvalidActionCompletion);
}
```

After that, the handler writes and resumes using fields supplied by the answer:

```rust
state
    .frame
    .write_slot_with_taint(answer.answer_slot, answer.value, answer.taint)
    .map_err(|_| RuntimeError::RunNotFound)?;
state
    .frame
    .set_pc(answer.ticket.resume_step)
    .map_err(|_| RuntimeError::RunNotFound)?;
```

There is no validation that the answer slot or resume step matches the workflow node that suspended.

## Adversarial Check

This is not protected by the pending timer: `PendingTimer` is checked only for `step` and `kind` here, and it does not constrain `answer_slot` or `resume_step` in this function. The action completion path has a separate preflight that derives and checks expected slots; the ask answer path lacks an equivalent guard.

## Suggested Fix

Store the ask answer slot and resume step in the pending ask authority when the ask suspends, or derive them from the workflow during answer handling. Reject answers whose supplied ticket fields do not exactly match the stored authority. Prefer an opaque generation/key over trusting routable fields from the answer payload.
