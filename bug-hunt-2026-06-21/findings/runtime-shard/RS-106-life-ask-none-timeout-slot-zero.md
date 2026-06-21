# RS-106-life: Ask without timeout collapses to slot zero

- **Severity**: High
- **Category**: correctness
- **Location**: `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs:222`
- **Confidence**: confirmed

## Description

`RuntimeSignal::AwaitingAsk(timeout_slot)` carries an optional timeout slot, but the lifecycle code erases `None` by substituting `SlotIdx::ZERO`. A no-timeout ask is therefore represented as either a timer derived from slot zero or, if timer registration is skipped elsewhere, an ask with no pending authority even though answers require one.

## Evidence

The `Option<SlotIdx>` is collapsed before entering the timer/ask suspension path:

```rust
Ok(RuntimeSignal::AwaitingAsk(timeout_slot)) => {
    self.apply_awaiting_timer(run, state, PendingTimerKind::Ask, timeout_slot.unwrap_or(vb_core::ids::SlotIdx::ZERO))
}
```

`await_timer` then computes a deadline from the supplied slot when registration is required:

```rust
let deadline_ms = compute_deadline_ms_from_slot(&state, deadline_slot);
```

The answer path requires a pending ask timer to exist:

```rust
let pending_timer = self
    .pending_timer_get(run)
    .ok_or(RuntimeError::InvalidActionCompletion)?;
```

## Adversarial Check

This is not just a style issue around `unwrap_or`. The source has no explicit `None` branch preserving the semantic difference between “no timeout” and “timeout value read from slot zero.” If registration proceeds, slot zero controls the timeout. If registration does not proceed, `handle_ask_answer` rejects the answer because there is no pending ask authority.

## Suggested Fix

Represent pending asks separately from timed waits, or make `await_timer` accept `Option<SlotIdx>` for ask deadlines. For `None`, register an ask authority without a deadline and allow `handle_ask_answer` to validate against that authority without reading slot zero.
