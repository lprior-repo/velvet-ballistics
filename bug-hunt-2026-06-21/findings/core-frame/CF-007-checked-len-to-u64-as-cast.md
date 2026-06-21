# CF-007: `as u64` cast in `checked_len_to_u64` violates the no-`as`-casts rule

- **Severity**: Medium
- **Category**: correctness
- **Location**: `crates/vb_core/src/value_store.rs:333` (and `crates/vb_core/src/value_store/id_gen.rs:6`)
- **Confidence**: confirmed

## Description

Both copies of `checked_len_to_u64` use `len as u64` and silence clippy
with `#[allow(clippy::as_conversions)]`. The repo's Holzman / functional-rust
rules forbid `as` numeric casts in production source. Even though the cast
is lossless on supported targets (32-bit and 64-bit `usize`), the rule is
absolute and exists precisely to prevent "it's safe today" drift.

## Evidence

```rust
#[allow(clippy::as_conversions)]
fn checked_len_to_u64(len: usize) -> u64 {
    // Lossless on all Rust targets: usize is either 32-bit or 64-bit.
    // Both fit in u64, so this cast never overflows or truncates.
    len as u64
}
```

(`crates/vb_core/src/value_store.rs:332-337`)

Identical body in `crates/vb_core/src/value_store/id_gen.rs:6-10`.

Both call sites are in production code paths
(`total_arena_count` / `value_store.rs:301-308`).

## Adversarial Check

The comment is technically correct: `usize` is implementation-defined but
is 32-bit or 64-bit on every Tier-1 Rust target. So the cast is lossless.
But the rule is not "no lossy casts" — it is "no `as` casts" period, and
the AGENTS.md engineering rule (line 76) explicitly lists "no `as` numeric
casts" alongside "no unchecked indexing/slicing." Opening an exception
here sets a precedent that future, less-safe casts can hide behind. The
safe alternative is one line longer and unambiguously correct.

## Suggested Fix

```rust
fn checked_len_to_u64(len: usize) -> u64 {
    u64::try_from(len).map_or(u64::MAX, core::convert::identity)
}
```

This is the same shape `usize_to_u64` already uses in
`shard/partition/mod.rs:26-28`.
