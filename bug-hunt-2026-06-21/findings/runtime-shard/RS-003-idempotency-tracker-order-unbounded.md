# RS-003: IdempotencyTracker `order` Vec grows unboundedly while `completed` stays bounded

- **Severity**: High
- **Category**: bug / memory leak
- **Location**: `crates/vb_runtime/src/idempotency.rs:108-116, 207-231`
- **Confidence**: confirmed

## Description

`IdempotencyTracker` advertises a "bounded" tracker (`idempotency.rs:5-9, 17-23`): `completed` is bounded by `capacity` via FIFO eviction. But every `mark_completed` call *unconditionally* `push`es the key onto `order` (`idempotency.rs:114`), and `evict_if_full` only walks the existing `order` to find a slot to free — it never *truncates* `order`. Over time `order.len()` grows without bound while `completed.len()` stays at `capacity`, producing a slow memory leak and degrading eviction to O(capacity) skips through dead entries.

## Evidence

```rust
// idempotency.rs:108-116
pub fn mark_completed(&mut self, ticket: &ActionTicket) -> Result<(), ActionError> {
    if self.completed.contains_key(&ticket.idempotency_key) {
        return Err(ActionError::CompletionAlreadyRecorded);
    }
    self.evict_if_full();
    self.completed.insert(ticket.idempotency_key, *ticket);
    self.order.push(ticket.idempotency_key);   // ← unbounded push
    Ok(())
}
```

```rust
// idempotency.rs:207-231
fn evict_if_full(&mut self) {
    if self.completed.len() < self.capacity { return; }
    let max_attempts = self.order.len();
    let mut attempts = 0;
    while attempts < max_attempts {
        attempts = attempts.checked_add(1)…;
        let Some(&key) = self.order.get(self.cursor) else { break; };
        let removed = self.completed.remove(&key);   // ← may be None (already gone)
        let next = self.cursor.saturating_add(1);
        self.cursor = if next >= self.order.len() { 0 } else { next };
        if removed.is_some() { return; }
    }
}
```

Trace with `capacity = 2`:

| Step | `mark_completed(x)` | `completed` after | `order` after | `cursor` |
|------|---------------------|-------------------|---------------|----------|
| 1    | a                   | {a}               | [a]           | 0        |
| 2    | b                   | {a,b}             | [a,b]         | 0        |
| 3    | c (evict a)         | {b,c}             | [a,b,c]       | 1        |
| 4    | d (evict b)         | {c,d}             | [a,b,c,d]     | 2        |
| 5    | e (scan: a, b, c)   | {d,e}             | [a,b,c,d,e]   | 0        |

`order.len()` grows by 1 per `mark_completed` call. After N insertions, `order.len() == N` regardless of `capacity`. The advertised "bounded" property is violated.

Eviction degrades too: at step 5, two cursor slots are dead (a and b already removed) before the live c is found, so eviction touches `cursor + 2` slots. After 10⁶ insertions at capacity 2, eviction may scan up to 10⁶ dead entries per call (the `while attempts < max_attempts` bound only stops at `order.len()`, which is the leak size itself).

## Adversarial Check

A defender might claim that `order` mirrors the FIFO insertion order and "morally" stays at capacity. The trace above disproves this — `order.push` is unconditional. The defender might also argue that `idempotency_key` is `u128` (16 bytes) so the leak is "small". Even at 16 bytes/entry, a 1000-action-per-second runtime accumulates ~1.4 GB/day of `order` growth per shard. The eviction scan complexity is the worse problem: an attacker (or just a long-running workload) can make every `mark_completed` O(N) in the total number of historical completions.

## Suggested Fix

Use a proper ring buffer (`VecDeque` with `truncate` after capacity, or `std::collections::VecDeque` with `pop_front`) and remove dead entries eagerly. Better: store `(key, ticket)` directly in a `VecDeque` of size `capacity`, evicting the front:

```rust
pub struct IdempotencyTracker {
    entries: VecDeque<(u128, ActionTicket)>,
    index: HashMap<u128, usize>,    // key -> index in entries
    capacity: usize,
    …
}
```

Or, simpler, replace the cursor logic with `self.order.remove(0)` (or use `VecDeque::pop_front`) on each eviction, so `order.len()` stays at `capacity`.
