# RA-026: `Runtime::shard_index` falls back to shard 0 on overflow, silently concentrating load

- **Severity**: Low
- **Category**: correctness (silent fallback)
- **Location**: `crates/vb_runtime/src/runtime/mod.rs:32-41`
- **Confidence**: confirmed

## Description

`shard_index` has two `return 0` fallbacks: one for `u64::try_from(self.shard_count)` failure (impossible on 64-bit), one for `checked_rem` returning `None` (impossible since `count != 0`), and a final `usize::try_from(remainder).unwrap_or_default()` that returns 0 if the u64 remainder does not fit in usize (impossible on 64-bit, possible on 32-bit if `shard_count > u32::MAX`). All three fallbacks silently route to shard 0 with no log, metric, or error.

## Evidence

```rust
pub fn shard_index(&self, run: RunId) -> usize {
    let Ok(count) = u64::try_from(self.shard_count) else {
        return 0;
    };
    let Some(remainder) = run.get().checked_rem(count) else {
        return 0;
    };
    usize::try_from(remainder).unwrap_or_default()
}
```

The fallbacks exist for arithmetic safety. But on a 32-bit target with `shard_count > u32::MAX` (impossible in practice — that would require > 4 billion shards), all runs whose `run_id % count` exceeds `u32::MAX` are routed to shard 0, creating a hot shard.

More importantly, the `RunId -> shard` mapping is a public `#[must_use]` function. The silent fallback makes it impossible for the caller to detect that routing has degenerated.

## Adversarial Check

One could argue the fallbacks are unreachable in practice — `shard_count` comes from `NonZeroUsize`, so `count >= 1` and `checked_rem` always returns `Some`. On 64-bit (the only platform the project targets per AGENTS.md), `usize::try_from(remainder)` always succeeds because `remainder < count ≤ usize::MAX as u64`. So all three fallbacks are dead code on the supported platform. The issue is that the function shape obscures the invariant: a future port to a 32-bit target (or a `NonZeroU64` shard count change) would silently fall back to shard 0 instead of failing to compile. Documenting the invariant or using `expect`-free typed conversions makes the assumption explicit.

## Suggested Fix

Replace the silent fallbacks with `const`-shaped invariant checks that fail to compile if the assumption breaks, e.g.:

```rust
pub fn shard_index(&self, run: RunId) -> usize {
    const _: () = assert!(std::mem::size_of::<usize>() >= std::mem::size_of::<u64>());
    let count = u64::try_from(self.shard_count).unwrap_or(u64::MAX);
    let remainder = run.get().checked_rem(count).unwrap_or(0);
    usize::try_from(remainder).unwrap_or(0)
}
```

Or simply `(run.get() % self.shard_count as u64) as usize` with a const-assert that `usize` is at least 64 bits. The current shape is correct on 64-bit but obscures the platform assumption behind three layers of defensive code.
