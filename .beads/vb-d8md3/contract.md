# VB-D8MD3 — Bounded Action Completion Queue Warning Outcome Surfacing

## Scope

Add an honest outcome surface to `BoundedActionCompletionQueue::enqueue` so callers can distinguish whether a backpressure warning was delivered, dropped, or not attempted.

## API Contract

### New Type

```rust
/// Outcome of attempting to send a backpressure warning notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BackpressureWarningOutcome {
    /// A backpressure warning was sent and the channel accepted it.
    Delivered,
    /// A backpressure warning was attempted but the channel was full.
    ChannelFull,
    /// A backpressure warning was attempted but the receiver had been dropped.
    ChannelDisconnected,
}
```

### New Method

```rust
impl BoundedActionCompletionQueue {
    /// Attempts to enqueue an action ticket, surfacing the backpressure
    /// warning send outcome.
    ///
    /// Returns:
    ///   - `Ok(())` if the ticket was enqueued
    ///   - `Err(ActionQueueError::QueueFull)` if the queue is at capacity
    ///
    /// The `Option<BackpressureWarningOutcome>` indicates whether a backpressure
    /// warning was attempted and its result:
    ///   - `None`: No warning was attempted (below threshold, queue full, or no channel).
    ///   - `Some(Delivered)`: Warning sent and channel accepted.
    ///   - `Some(ChannelFull)`: Warning attempted but channel was full.
    ///   - `Some(ChannelDisconnected)`: Warning attempted but receiver dropped.
    pub fn enqueue_with_backpressure_warning_outcome(
        &self,
        ticket: ActionTicket,
    ) -> (Result<(), ActionQueueError>, Option<BackpressureWarningOutcome>);
}
```

### Existing Method Compatibility

`enqueue` remains unchanged and returns `Result<(), ActionQueueError>`. Callers who do not need warning outcome information use `enqueue`. `enqueue_with_backpressure_warning_outcome` is a superset.

## Invariants

1. **Honest outcome:** `Some(Delivered)` means a warning was actually sent to the channel. `None` means no warning was attempted.
2. **Threshold consistency:** Warnings are only attempted when depth >= backpressure threshold (ceiling of 80%).
3. **Queue full:** When queue is full, ticket is not enqueued, no warning is attempted, outcome is `None`.
4. **No channel:** When `backpressure_tx` is `None`, outcome is always `None` (not `Delivered`).
5. **FIFO preserved:** Enqueue order is unchanged.

## Acceptance Criteria

### Correctness

- [ ] Queue full returns `Err(QueueFull)` with outcome `None`
- [ ] Below threshold returns `Ok(())` with outcome `None`
- [ ] No channel returns `Ok(())` with outcome `None`
- [ ] At threshold with live channel returns `Ok(())` with outcome `Some(Delivered)` and receiver gets warning
- [ ] At threshold with full channel returns `Ok(())` with outcome `Some(ChannelFull)`
- [ ] At threshold with dropped receiver returns `Ok(())` with outcome `Some(ChannelDisconnected)`

### Integration

- [ ] Existing `enqueue` method unchanged
- [ ] Existing backpressure threshold calculation unchanged
- [ ] No changes to capacity validation
- [ ] No changes to poison recovery behavior

### Test Requirements

- Real bounded channels (no mocks)
- Deterministic full/disconnected/threshold/no-channel/queue-full scenarios
- No sleeps or timing-dependent assertions
- Exact warning values verified on channel when delivered

## Non-Goals

- Wiring to production shard command queue (separate bead)
- Benchmarking
- Formal verification
- Generated Rust code

## Size

- Method: ~25 lines
- File additions: ~300 lines (including tests)
- Single file: `crates/vb_runtime/src/action_queue.rs`
