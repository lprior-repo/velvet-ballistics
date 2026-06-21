# RS-009: CompletionWatermark `drain_prefix` is O(n²) — `Vec<u64>` linear scan + `waiters.retain` per drained seq

- **Severity**: Medium
- **Category**: perf
- **Location**: `crates/vb_runtime/src/shard/completion_watermark.rs:177-198`
- **Confidence**: confirmed

## Description

`drain_prefix` repeatedly calls `remove_pending` (linear scan over unsorted `Vec<u64>`) and `waiters.retain` (linear scan over unsorted `Vec<u64>`) for *each* drained sequence. If the pending set contains a long contiguous prefix that becomes drainable in one `complete` call, the cost is O(n²) in the prefix length.

## Evidence

```rust
// completion_watermark.rs:177-198
fn drain_prefix(&mut self) -> Vec<u64> {
    std::iter::from_fn(|| self.drain_next()).collect()
}

fn drain_next(&mut self) -> Option<u64> {
    let next = self.boundary.checked_add(1)?;
    self.remove_pending(next).then(|| {
        self.boundary = next;
        self.waiters.retain(|waiter| *waiter != next);   // ← O(waiters.len()) per drain
        next
    })
}

fn remove_pending(&mut self, seq: u64) -> bool {
    match self.pending.iter().position(|candidate| *candidate == seq) {  // ← O(pending.len())
        Some(position) => {
            self.pending.swap_remove(position);
            true
        }
        None => false,
    }
}
```

Per `drain_next` call: O(`pending.len()`) for the position scan, plus O(`waiters.len()`) for retain. To drain a contiguous prefix of length k from a queue of size n, total cost is O(k·(n+k)) ≈ O(k·n) when waiters grow with completed sequences.

Hot path: this runs on every `complete()` call. Under bursty traffic (e.g. a large batch of action completions arriving in seq order), `pending.len()` is bounded by `max_pending` (default unspecified but typically 1024+), and a single `complete` call can drain up to `pending.len()` sequences in `drain_prefix`. With `max_pending = 1024` and a full contiguous prefix, that is ~10⁶ element comparisons per call.

## Adversarial Check

A defender might argue "completions arrive one at a time, so each `complete` drains at most 1 sequence." That is true only when completions arrive in order. The entire point of the watermark is to handle *out-of-order* arrivals — that is what the `pending` set exists for. A workload that bursts N out-of-order completions, then sends the missing head of the prefix, drains all N pending entries in one `complete` call. The data structure is specifically designed for this case, and the data structure specifically mis-handles it.

A second defender argument: "max_pending bounds n." True, but max_pending defaults are typically 1024-4096; O(n²) at that size is 10⁶-10⁷ operations on the hot path. A functional-rust rewrite using a `BTreeSet<u64>` for `pending` and a `BTreeSet<u64>` for `waiters` makes both operations O(log n).

## Suggested Fix

Replace `Vec<u64>` for `pending` with a `BTreeSet<u64>` (or `BinaryHeap<Reverse<u64>>`). `drain_prefix` then walks the prefix in order without scanning:

```rust
fn drain_prefix(&mut self) -> Vec<u64> {
    let mut drained = Vec::new();
    while self.pending.remove(&(self.boundary + 1)) {
        self.boundary += 1;
        self.waiters.remove(&self.boundary);
        drained.push(self.boundary);
    }
    drained
}
```

`BTreeSet::remove` is O(log n). Total drain cost is O(k log n). `waiters` can also be a `BTreeSet` for the same reason.
