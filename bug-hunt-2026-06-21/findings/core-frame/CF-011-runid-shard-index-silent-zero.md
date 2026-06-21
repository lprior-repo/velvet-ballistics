# CF-011: `RunId::shard_index` silently returns 0 when `shard_count == 0`

- **Severity**: Medium
- **Category**: correctness
- **Location**: `crates/vb_core/src/ids/workflow_ids.rs:107`
- **Confidence**: confirmed

## Description

`RunId::shard_index` uses `checked_rem(shard_count)` and on `None`
(shard_count == 0) returns `0`. A misconfigured shard_count of zero
silently routes every run to shard 0 instead of producing an error, which
both defeats the purpose of sharding and hides the configuration bug from
operators.

## Evidence

```rust
/// Returns the shard index for this run.
///
/// Uses `checked_rem` to handle the degenerate case where
/// `shard_count` is 0, returning 0 in that case.
#[must_use]
pub const fn shard_index(self, shard_count: u64) -> u64 {
    match self.0.checked_rem(shard_count) {
        Some(index) => index,
        None => 0,
    }
}
```

(`crates/vb_core/src/ids/workflow_ids.rs:102-112`)

The function is `const fn`, so it cannot return a `Result`. The docstring
even spells out the degenerate behavior, normalizing it.

## Adversarial Check

A defender might say "this is a `const fn`, it has no choice but to
return a value, and `0` is the safest default." But the choice between
"silently route to shard 0" and "make the caller handle the degenerate
case" is significant: in production, silent shard-0 routing means every
run lands on a single server while the others sit idle, and the
misconfiguration produces no error log. The `ShardCount` type in
`shard/partition/mod.rs:164` already enforces `raw >= 1` at construction;
`shard_index` should take a `ShardCount` and skip the `checked_rem` path
entirely.

## Suggested Fix

Change the signature to take `shard_count: ShardCount` (which is
guaranteed `>= 1` by construction), then `self.0 %
shard_count.as_u64()` cannot divide by zero and needs no `checked_rem`.
