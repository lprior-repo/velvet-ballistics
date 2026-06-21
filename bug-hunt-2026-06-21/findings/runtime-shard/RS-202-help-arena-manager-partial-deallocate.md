# RS-202-help: `ArenaManager::deallocate_all` mutates arenas even when the aggregate deallocation fails

- **Severity**: Medium
- **Category**: correctness
- **Location**: `crates/vb_runtime/src/shard/arena/mod.rs:53`
- **Confidence**: confirmed

## Description

`ArenaManager::deallocate_all` is documented as an atomic synchronized deallocation, but it deallocates each arena immediately and only returns the first error after the fact. If any later arena rejects the handle, earlier arenas have already lost their state and the caller receives an error with no rollback path.

## Evidence

```rust
// mod.rs:53-66
/// Deallocate all state associated with a given slot handle from all arenas.
/// This is the synchronized deallocation operation — all 4 per-run arenas
/// are freed together atomically.
pub fn deallocate_all(&mut self, handle: SlotHandle) -> Result<(), ArenaError> {
    let r1 = self.frame_pools.deallocate(handle);
    let r2 = self.pending_timers.deallocate(handle);
    let r3 = self.journal_sequences.deallocate(handle);
    let r4 = self.runtime_states.deallocate(handle);
    let r5 = self.run_states.deallocate(handle);
    let r6 = self.terminal_runs.remove(handle);
    r1.or(r2).or(r3).or(r4).or(r5).or(r6)
}
```

Each `deallocate` call performs mutation before the final `or` chain decides whether the whole operation succeeded. A missing pending timer or terminal-run membership is enough to produce an error after other arenas were already deallocated.

## Adversarial Check

The comment could be dismissed as stale wording, but the function returns a single `Result`, not a best-effort report, and the doc says the operation is atomic. The code also includes arenas whose membership can naturally differ by lifecycle phase, such as `pending_timers` and `terminal_runs`, making partial failure a realistic state transition risk rather than only memory-corruption fallout.

## Suggested Fix

Use a two-phase operation. First validate every required arena and explicitly classify optional arenas. Only after validation succeeds should the function mutate. If best-effort cleanup is intended, rename the method and return a structured report that lists which arenas were actually deallocated and which were absent.
