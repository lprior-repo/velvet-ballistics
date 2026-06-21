# RS-026: `SlotSet::ensure_insert_slot` trusts caller-provided generation when growing the arena

- **Severity**: Low
- **Category**: correctness / encapsulation
- **Location**: `crates/vb_runtime/src/shard/arena/slot_set.rs:33-45, 47-72`
- **Confidence**: confirmed

## Description

When `SlotSet::insert` is called with a `SlotHandle` whose slot id equals the current arena length (i.e. the caller is appending a new slot), the function pushes the slot and writes the *caller-claimed* generation directly into `arena.generations`. There is no validation that the claimed generation matches what the Arena would have assigned (`Generation::INITIAL` for a fresh slot). A caller can install arbitrary generation tokens and later reject legitimate operations via generation mismatch.

## Evidence

```rust
// slot_set.rs:33-45
fn ensure_insert_slot(&mut self, handle: SlotHandle) -> Result<(usize, bool), ArenaError> {
    let idx = Arena::<()>::slot_index(handle.slot_id())?;
    if idx > self.arena.slots.len() {
        return Err(ArenaError::InvalidSlotId);
    }
    if idx == self.arena.slots.len() {
        self.arena.slots.push(None);
        self.arena.generations.push(handle.generation());   // ← caller-claimed gen
        Ok((idx, true))
    } else {
        Ok((idx, false))
    }
}
```

```rust
// slot_set.rs:47-72
fn insert_at(
    &mut self, idx: usize, handle: SlotHandle, new_slot: bool,
) -> Result<(), ArenaError> {
    let generation = self
        .arena
        .generations
        .get_mut(idx)
        .ok_or(ArenaError::InvalidSlotId)?;
    if !new_slot && *generation != handle.generation() {
        return Err(ArenaError::GenerationMismatch);     // ← checked only for existing slots
    }
    …
}
```

Compare with `Arena::push_new_slot` (`arena.rs:92-102`), which always assigns `Generation::INITIAL`:

```rust
fn push_new_slot(&mut self) -> Result<SlotId, ArenaError> {
    let id = self.slots.len();
    …
    self.slots.push(None);
    self.generations.push(Generation::INITIAL);          // ← always INITIAL
    …
}
```

The Arena enforces its invariant; the SlotSet bypasses it for the new-slot path.

## Adversarial Check

A defender might argue "SlotSet is membership-only, so the generation is just a tag." But SlotSet uses the same `Arena` and the same `SlotHandle` types — callers will reasonably expect generation semantics consistent with Arena. The doc on `Generation` (`types.rs:47-51`) says it is for ABA prevention — and ABA prevention requires the generation to be assigned by the allocator, not by the caller.

Concrete exploit: a caller that constructs `SlotHandle { slot_id: SlotId::new(N), generation: Generation(u64::MAX) }` and inserts into a SlotSet of length N installs a "terminal" generation (per `Generation::is_terminal`, `types.rs:73-76`). When `Arena::deallocate` later tries to push this slot onto the free list (`arena.rs:143-146`), the terminal-generation check skips the push, permanently retiring the slot. The SlotSet has now shrunk the Arena's effective capacity.

## Adversarial Check (continued)

A second defender argument: "SlotSet is only used for `terminal_runs`, and the only inserter is `terminal_runs_insert` which never exposes generation control to external callers." True today, but `SlotSet::insert` is `pub` and `SlotHandle::new` is `pub const fn` (`types.rs:96-102`) — any future caller can construct handles with arbitrary generations. The encapsulation gap is real.

## Suggested Fix

```rust
fn ensure_insert_slot(&mut self, handle: SlotHandle) -> Result<(usize, bool), ArenaError> {
    let idx = Arena::<()>::slot_index(handle.slot_id())?;
    if idx > self.arena.slots.len() {
        return Err(ArenaError::InvalidSlotId);
    }
    if idx == self.arena.slots.len() {
        self.arena.slots.push(None);
        self.arena.generations.push(Generation::INITIAL);   // ← always INITIAL
        Ok((idx, true))
    } else {
        Ok((idx, false))
    }
}
```

If the caller really does need to install a specific generation (e.g. for recovery replay), expose that as a separate `insert_with_generation` method that documents the trust assumption.
