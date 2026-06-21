# RS-210-core-lru-ring-clear-free-list-leak: `clear` strands free slots and lets the arena grow after every reuse

- **Severity**: Medium
- **Category**: bug
- **Location**: `crates/vb_runtime/src/shard/lru_ring.rs:210`
- **Confidence**: confirmed

## Description
`LruRing::clear` marks existing slots as `None` but clears the free list. The next insert sees an empty free list and appends a new slot, so repeated `clear` plus insert cycles grow `nodes` even when live length stays within capacity.

## Evidence
```rust
210:     pub fn clear(&mut self) {
211:         self.head = None;
212:         self.tail = None;
213:         self.free.clear();
214:         for slot in self.nodes.iter_mut() {
215:             *slot = None;
216:         }
217:         self.position.clear();
218:     }
...
352:     fn push_tail(&mut self, item: T, now: TimerTick) {
353:         let slot = match self.free.pop() {
354:             Some(free_slot) => free_slot,
355:             None => {
356:                 let new_slot = self.nodes.len();
357:                 self.nodes.push(None);
358:                 new_slot
359:             }
```

For capacity `1`, insert once, call `clear`, then insert again. Slot `0` is `None` but not on `free`, so `push_tail` appends slot `1`. Repeating the cycle keeps appending slots while `position.len()` remains at most `1`.

## Adversarial Check
The bug does not require internal corruption or invalid input. It follows from the public `clear` method and the normal `push_tail` allocation path. The method documentation says clear removes entries without changing capacity or TTL; it does not say the arena becomes unreusable.

## Suggested Fix
Either call `self.nodes.clear()` so the next insert reuses the vector allocation from index zero, or rebuild `free` with every cleared slot index. The simpler fix is to clear `nodes` length while preserving vector capacity.
