# RE-007: `TraceRing::push` clones the event unconditionally, even when the ring is full

- **Severity**: Low
- **Category**: perf
- **Location**: `crates/vb_runtime/src/trace/ring.rs:86-109`
- **Confidence**: confirmed

## Description

`TraceRing::push` clones every incoming `TraceEvent` so that one copy can live in the SPSC pending queue and another in the in-process history `VecDeque`. The clone happens before the capacity check, so events that get dropped (because `rtrb` is full) still pay for the clone. For `SlotWritten` events — which carry a `Vec<u8>` payload — this clone allocates.

## Evidence

`crates/vb_runtime/src/trace/ring.rs:86-97`:

```rust
pub fn push(&mut self, event: TraceEvent) -> bool {
    #[cfg(not(kani))]
    {
        let remembered = event.clone();      // <-- clone always
        if self.push_pending(event) {
            self.remember(remembered);
            true
        } else {
            self.dropped = self.dropped.saturating_add(1);
            false
        }
    }
    ...
}
```

`TraceEvent::SlotWritten { value: Vec<u8> }` (event.rs:28-35) — cloning that variant allocates and copies the entire byte payload.

When the SPSC queue is full, the clone was wasted work: we copy the payload only to immediately throw it away. Under sustained overflow (a slow trace consumer), every dropped `SlotWritten` event costs an allocation.

## Adversarial Check

1. *"Rust clone elision may save us."* — No; the value is moved into `push_pending` unconditionally. The `remembered` clone is computed regardless of whether `push_pending` will succeed.
2. *"Trace events are small."* — `SlotWritten` carries a postcard-encoded slot value, which can be hundreds of bytes for list/slot-heavy values. Clone cost is O(payload).
3. *"This is the cold path."* — Trace push is the per-step emission path. It runs for every `SlotWritten` the engine emits. For a workflow that runs 10 000 steps, this is 10 000 potential allocations.

Severity Low: in steady state (no overflow) the clone is unavoidable given the design (two stores). The waste only manifests under overflow. Still, the optimization is trivial.

## Suggested Fix

```rust
pub fn push(&mut self, event: TraceEvent) -> bool {
    if self.push_pending(event) {
        // pull from the consumer side to seed history, avoiding a clone
        // OR: re-design so that history is the single source of truth
        true
    } else {
        self.dropped = self.dropped.saturating_add(1);
        false
    }
}
```

A cleaner architectural fix: collapse `pending` and `history` into a single store. The current design keeps an `rtrb` SPSC (for cross-thread handoff) *plus* a `VecDeque` history (for in-process snapshots). The `rtrb` is single-producer/single-consumer, but `TraceRing::push` takes `&mut self`, so the ring is not actually shared across threads today — the SPSC machinery is unused. Replacing both with a single `VecDeque<TraceEvent>` and exposing `drain_into` would remove the clone entirely.
