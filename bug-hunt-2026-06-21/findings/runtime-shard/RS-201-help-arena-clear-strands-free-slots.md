# RS-201-help: `Arena::clear` strands all cleared slots outside the free list

- **Severity**: Medium
- **Category**: correctness / perf
- **Location**: `crates/vb_runtime/src/shard/arena/arena.rs:212`
- **Confidence**: confirmed

## Description

`Arena::clear` marks every slot empty and clears the free list, but it leaves `slots.len()` unchanged. The next allocation therefore appends a brand-new slot instead of reusing the cleared backing storage, so repeated clear/refill cycles grow the arena and can eventually report exhaustion while old cleared slots remain unusable.

## Evidence

```rust
// arena.rs:85-90
fn next_slot_id(&mut self) -> Result<SlotId, ArenaError> {
    match self.free_list.pop() {
        Some(free_id) => Ok(free_id),
        None => self.push_new_slot(),
    }
}

// arena.rs:212-220
pub fn clear(&mut self) {
    for slot in self.slots.iter_mut() {
        *slot = None;
    }
    self.free_list.clear();
    self.live_count = 0;
}
```

After `clear`, `free_list` is empty and every old slot is `None`. Since `next_slot_id` only reuses slots from `free_list`, allocation falls through to `push_new_slot`, which appends at `self.slots.len()` and leaves all old cleared slots stranded.

## Adversarial Check

This is not just a capacity accounting nit. The method explicitly says it resets the arena while not deallocating backing storage, but the implementation preserves the storage without making it reusable. Stale-handle safety does not require stranding slots: `clear` can increment generations for previously live slots before repopulating the free list, preserving ABA protection while allowing reuse.

## Suggested Fix

Make `clear` either truly drop logical capacity by clearing `slots` and `generations`, or preserve capacity by rebuilding `free_list` with every non-terminal slot id after advancing generations for slots that were live. Avoid reusing a cleared slot without a generation bump.
