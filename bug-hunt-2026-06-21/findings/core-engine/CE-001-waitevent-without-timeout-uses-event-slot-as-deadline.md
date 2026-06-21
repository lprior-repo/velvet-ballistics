# CE-001: WaitEvent without a timeout reports the event slot as a deadline

- **Severity**: High
- **Category**: correctness
- **Location**: `crates/vb_core/src/engine/step.rs:123`
- **Confidence**: confirmed

## Description

`WaitEvent` nodes with no `timeout_slot` are converted into `EngineSignal::AwaitingWait` with `deadline_slot` set to the event slot. A caller following the signal contract will read event data as a deadline and arm the wrong wait behavior.

## Evidence

`execute_suspension_node` handles `WaitEvent` by falling back from `timeout_slot` to `event`:

```rust
CompiledNodeKind::WaitEvent {
    event,
    timeout_slot,
} => Ok(EngineSignal::AwaitingWait {
    deadline_slot: timeout_slot.unwrap_or(*event),
}),
```

The signal type documents `AwaitingWait.deadline_slot` as the slot the runtime reads for a concrete deadline, not an event identifier:

```rust
AwaitingWait {
    /// Slot the wait primitive read its deadline from.
    deadline_slot: SlotIdx,
}
```

## Adversarial Check

This is not just a naming nit. The IR carries separate `event` and `timeout_slot` fields, so a timeout-less event wait has no deadline slot to report. The current enum shape forces `WaitEvent` into a deadline-only signal and silently substitutes the event slot, which is only valid if event payloads and deadlines share a slot and type. Nothing in this function validates that impossible assumption.

## Suggested Fix

Add an explicit event-wait signal shape, for example `AwaitingEvent { event: SlotIdx, timeout_slot: Option<SlotIdx> }`, and emit that for `WaitEvent`. If the runtime needs a deadline only when a timeout exists, keep deadline extraction out of the no-timeout path.
