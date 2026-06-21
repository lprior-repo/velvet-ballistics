# CF-015: `ShardCount(pub usize)` exposes the inner field, bypassing `try_new` validation

- **Severity**: Medium
- **Category**: correctness
- **Location**: `crates/vb_core/src/shard/partition/mod.rs:161`
- **Confidence**: confirmed

## Description

`pub struct ShardCount(pub usize);` declares the inner field `pub`. Any
caller can construct `ShardCount(0)` or
`ShardCount(usize::MAX)` directly, bypassing the `try_new` validator
that enforces `1 <= inner <= MAX_SHARD_COUNT`. The file's documented
invariant ("ShardCount invariant: 1 <= inner <= MAX_SHARD_COUNT") is
therefore unenforced.

## Evidence

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShardCount(pub usize);

impl ShardCount {
    pub fn try_new(raw: usize) -> Result<Self, PartitionError> {
        if raw == 0 {
            return Err(PartitionError::ZeroShardCount);
        }
        if raw > MAX_SHARD_COUNT {
            return Err(PartitionError::ShardCountExceedsMax { ... });
        }
        Ok(Self(raw))
    }
    ...
}

impl Default for ShardCount {
    fn default() -> Self {
        Self(1)            // <-- also bypasses try_new
    }
}
```

(`crates/vb_core/src/shard/partition/mod.rs:160-195`)

`PartitionPlan::from_config` (line 244) defensively re-checks `n == 0`,
which only makes sense if it cannot trust the input ShardCount — i.e.
the bypass is a real concern.

## Adversarial Check

A defender might say "this is a verification model file, not a production
type." But the module docstring at line 3-5 explicitly says "These types
are verification models. They will be promoted to production types in
State 6/7 (implementation)." Shipping a model with an unenforced
invariant guarantees that the production version inherits the same hole.
The Kani `Arbitrary` impl at lines 418-424 also constructs
`Self(raw)` directly (assuming 1 <= raw <= KANI_MAX_SHARD_COUNT), which
only works because the field is pub.

## Suggested Fix

Make the field `pub(crate)` (or private) and expose construction only
through `try_new` and `Default`. Kani harnesses can use
`try_new(raw).unwrap()` inside `kani::Arbitrary` since the assume makes
it infallible.
