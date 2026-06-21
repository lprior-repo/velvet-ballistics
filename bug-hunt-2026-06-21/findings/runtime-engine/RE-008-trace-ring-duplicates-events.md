# RE-008: `TraceRing` keeps every event twice (SPSC pending + `VecDeque` history)

- **Severity**: Low
- **Category**: perf
- **Location**: `crates/vb_runtime/src/trace/ring.rs:19-34, 86-109, 222-227`
- **Confidence**: confirmed

## Description

`TraceRing` stores each accepted event in two places: the `rtrb` SPSC pending queue (for cross-thread consumption) and the `VecDeque` history (for in-process snapshots). This doubles memory usage relative to a single-store design, and the two stores are kept coherent by a manual clone on every push (see RE-007).

## Evidence

`crates/vb_runtime/src/trace/ring.rs:20-34`:

```rust
pub struct TraceRing {
    #[cfg(not(kani))]
    producer: rtrb::Producer<TraceEvent>,
    #[cfg(not(kani))]
    consumer: rtrb::Consumer<TraceEvent>,
    #[cfg(kani)]
    pending: KaniTraceQueue,
    capacity: usize,
    dropped: u64,
    #[cfg(not(kani))]
    history: VecDeque<TraceEvent>,
    #[cfg(kani)]
    history: KaniTraceQueue,
}
```

Both `producer`/`consumer` and `history` are present in the non-kani build. Both are populated by `push`:

- `push_pending(event)` pushes into the `rtrb` via `producer`.
- `remember(remembered)` pushes into `history`.

`remember` (ring.rs:222-227) evicts the oldest history entry when `history.len() >= capacity`. The `rtrb` has its own capacity (also `bounded_capacity`). So for capacity `N`, the ring stores up to `2N` events.

Additionally, `push` takes `&mut self`, which means the SPSC pattern (which exists for *single-producer, single-consumer across threads*) is not actually being exercised — both the producer and consumer are owned by the same `&mut self`, so the trace ring is single-threaded today.

## Adversarial Check

1. *"The producer may be sent across threads elsewhere."* — `rtk grep` shows `TraceRing::push` always takes `&mut`. The `producer` field is private and never moved out. There is no API that exposes the producer separately. So the SPSC machinery is overkill for the current design.
2. *"The VecDeque history is needed for snapshot queries."* — Yes (`snapshot_for_run`, `has_terminal_event_for_run`). But those queries could be served from a single `VecDeque` if `drain_into` simply reads from the same buffer.
3. *"Doubling memory is fine because trace capacity is small."* — `MAX_TRACE_RING_CAPACITY` (vb_core::limits) is the cap. Even at modest capacities (e.g., 4 096), 2× storage of `Vec<u8>`-carrying events is real memory.

Severity Low: the duplication is wasteful but bounded. Combined with RE-007 it is a clear design smell, but neither is a correctness bug.

## Suggested Fix

Collapse to a single `VecDeque<TraceEvent>` (or a `Vec<TraceEvent>` treated as a ring). Cross-thread hand-off, if ever needed, should be a separate concern layered on top (e.g., an SPSC queue that drains from the in-process buffer on demand). Removing the duplicate store eliminates the clone in `push` and halves steady-state memory.
