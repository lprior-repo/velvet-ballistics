# Contract: TraceRing zero-capacity clamping

## Scope

Production: `crates/vb_runtime/src/trace.rs` `TraceRing::new`.

## Current Behavior (Bug)

When `capacity = 0` is passed to `TraceRing::new`:
- `RingBuffer::new(capacity.max(1))` → ring buffer has 1 slot (OK)
- `self.capacity` stores original 0 (BUG)
- `self.history` gets `VecDeque::with_capacity(0)` (BUG)
- `drain()` allocates 0 and passes limit 0 → returns empty even if event exists
- `remember()` checks `while self.history.len() >= self.capacity` → `0 >= 0` is true, pops nothing, returns; event never remembered
- `drain_for_run` short-circuits with `bounded_limit == 0`

## Root Cause

Constructor clamps for `RingBuffer::new` but does not propagate the clamped value to the stored `capacity` field and `history` allocation. The clamped value is used only for the underlying ring buffer, leaving capacity-0 semantics broken for all higher-level operations.

## Expected Behavior (Fix)

The clamped value must be used consistently throughout `new`:

```rust
pub fn new(capacity: usize) -> Self {
    let capacity = capacity.max(1);
    let (producer, consumer) = RingBuffer::new(capacity);
    Self {
        producer,
        consumer,
        capacity,
        dropped: 0,
        history: VecDeque::with_capacity(capacity),
    }
}
```

No capacity-0 "disable" mode exists or is documented; capacity is always a positive bounded ring.

## Evidence Strategy

Integration test (new file) exercising capacity-0 construction → push → drain → history, plus overflow behavior. Baseline fails on UNCHANGED source; passes after fix. Unit test suite runs as regression.

## Non-Goals

No benchmark claims. No proof harnesses. No API changes. No test suite additions beyond the single integration test file.

## Oracle Amendments (Sol test-review v26dd4772)

1. `trace_ring_zero_capacity_refill_after_drain` now asserts exact event identity via `drain() == vec![event2]` (the second payload is observable), `pending_is_empty() == true`, and `snapshot_for_run(RunId::new(2), 1) == vec![event2]`. This proves the refilled event is not merely counted but actually retrievable by value, not just by count.

2. `trace_ring_zero_capacity_overflow_drops_second_event` now asserts exact event identity via `drain() == vec![event1]` (only the first event survives), `snapshot_for_run(RunId::new(1), 1) == vec![event1]`, and `snapshot_for_run(RunId::new(2), 1) == []`. This proves the dropped event never reached history and the surviving event is the correct first payload.