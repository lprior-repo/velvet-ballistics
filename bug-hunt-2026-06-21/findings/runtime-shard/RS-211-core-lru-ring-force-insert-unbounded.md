# RS-211-core-lru-ring-force-insert-unbounded: `force_insert` violates the ring capacity contract

- **Severity**: High
- **Category**: bug
- **Location**: `crates/vb_runtime/src/shard/lru_ring.rs:251`
- **Confidence**: confirmed

## Description
`LruRing` is documented as a bounded ring, but `force_insert` appends new entries even when the ring is already at capacity. Under a burst of terminal runs in the same TTL window, the structure grows past its configured capacity and only increments a counter.

## Evidence
```rust
4: //! `LruRing<T>` stores at most `capacity` entries and tags each entry
...
251:     pub fn force_insert(&mut self, item: T, now: TimerTick) {
252:         if self.position.contains_key(&item) {
253:             return;
254:         }
255:         if let Err(error) = self.sweep_expired(now) {
256:             tracing::error!(
...
262:         let before = self.position.len();
263:         self.push_tail(item, now);
264:         if self.position.len() > before && self.position.len() > self.capacity {
265:             self.counters.capacity_overflows = self.counters.capacity_overflows.saturating_add(1);
266:         }
```

When no entry is TTL-expired, `force_insert` calls `push_tail` regardless of `self.position.len() >= self.capacity`. The only consequence is `capacity_overflows += 1`; the live set still grows beyond the configured bound.

## Adversarial Check
This is not a false positive caused by the strict `insert` path, because `force_insert` is a separate public method with its own capacity-bypassing behavior. The file-level contract says the ring stores at most `capacity` entries, and the repository rule requires bounded resources. Counting an overflow after allocation does not bound memory.

## Suggested Fix
Remove the capacity-bypassing path. Make terminal-run insertion use the existing `insert` error path, or implement an explicit bounded eviction policy for legacy callers that evicts the oldest entry before appending.
