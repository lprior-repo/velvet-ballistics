# RS-002: LruRing `push_tail` silently overwrites live nodes on free-list regression and proceeds after slot-out-of-bounds

- **Severity**: Critical
- **Category**: correctness / data corruption
- **Location**: `crates/vb_runtime/src/shard/lru_ring.rs:352-399`
- **Confidence**: confirmed

## Description

`push_tail` logs a `tracing::error!` when it detects an internal corruption (free-list regression or slot out-of-bounds) but then **continues the insertion**, overwriting a live node or fabricating a phantom slot reference. This violates the module's own advertised contract ("Mutating operations … surface internal invariant violations through `LruRingError` instead of silently skipping the failed pointer fix-up", `lru_ring.rs:23-26`) and corrupts the doubly-linked list and the `position` HashMap.

## Evidence

Two corruption paths in `push_tail`:

```rust
// lru_ring.rs:367-389
match self.nodes.get_mut(slot) {
    Some(slot_ref @ Some(_)) => {
        // Free-list accounting regression: a live slot ended up on `free`.
        tracing::error!(…);
        *slot_ref = Some(node);            // ← OVERWRITES the live node!
    }
    Some(empty @ None) => *empty = Some(node),
    None => {
        tracing::error!(…);                // ← slot out of bounds
                                           //   no return; falls through
    }
}
if let Some(old_tail) = self.tail {        // ← still runs after the None case
    if let Some(old_tail_node) = self.nodes.get_mut(old_tail).and_then(Option::as_mut) {
        old_tail_node.next = Some(slot);
    }
} else {
    self.head = Some(slot);                 // ← head now points at a non-existent slot
}
self.tail = Some(slot);                     // ← tail points at a non-existent slot
self.position.insert(item, slot);           // ← position map points at a non-existent slot
```

Three concrete corruptions:

1. **Free-list regression (line 368-379):** `*slot_ref = Some(node)` overwrites the existing live node. The old node's item is still in `position` (pointing at this slot), but the slot now holds the new node's item. Both `position[old_item]` and `position[new_item]` resolve to the same slot, returning the wrong item for one of them. The doubly-linked-list pointers (prev/next) of neighbouring nodes still reference this slot, but the node's own `prev/next` are now `self.tail`/`None` from the new node, fragmenting the list.

2. **Slot out of bounds (line 381-388):** When `self.nodes.get_mut(slot)` returns `None`, the function does not return — it falls through to set `head`/`tail`/`position` to a slot index that does not exist in `nodes`. Every subsequent operation touching head/tail/position will fail or further corrupt state.

3. **`old_tail` link fix-up after overwrite (line 390-396):** When the slot existed but was None (the happy path) the prev-link is set correctly. But in both corruption cases the function continues, so the new node's `prev = self.tail` (line 364) is now chained onto a list whose old tail may not have had its `next` set (because `self.nodes.get_mut(old_tail).and_then(Option::as_mut)` short-circuits on `None`), creating a fragmented list.

The `force_insert` caller (`lru_ring.rs:251-267`) explicitly swallows `sweep_expired` errors and then calls `push_tail`, so any prior corruption compounds.

## Adversarial Check

The defenders of this code will say "free-list regression is impossible by construction — `remove` always pushes the slot onto `free` after unlinking, and `push_tail` always pops from `free`." That argument assumes the *current* code path is the only one mutating state, but the module's own doc (`lru_ring.rs:65-93`) declares that `LruRingError` variants exist because regressions *are* possible (e.g. after partial `sweep_expired` failures that leave a slot half-unlinked, see RS-016). The `tracing::error!` log proves the authors knew the path was reachable. Logging then continuing is the worst-of-both-worlds: the corruption is observed but not acted on. Either the path is unreachable (in which case the `unreachable!`/panic is appropriate) or it is reachable (in which case the function must return `LruRingError`).

## Suggested Fix

Make `push_tail` return `Result<(), LruRingError>` and propagate:

```rust
fn push_tail(&mut self, item: T, now: TimerTick) -> Result<(), LruRingError> {
    let slot = match self.free.pop() {
        Some(free_slot) => free_slot,
        None => { let s = self.nodes.len(); self.nodes.push(None); s }
    };
    match self.nodes.get_mut(slot) {
        Some(Some(_)) => return Err(LruRingError::SlotAlreadyLive(slot)),
        Some(empty @ None) => *empty = Some(node),
        None => return Err(LruRingError::SlotOutOfBounds { slot, arena_len: self.nodes.len() }),
    }
    …
}
```

Then `insert` and `force_insert` must propagate the error rather than logging it.
