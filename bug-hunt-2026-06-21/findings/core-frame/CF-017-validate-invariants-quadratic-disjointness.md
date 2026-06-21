# CF-017: `PartitionPlan::validate_invariants` is O(N²) for the disjointness check

- **Severity**: Low
- **Category**: perf
- **Location**: `crates/vb_core/src/shard/partition/mod.rs:389`
- **Confidence**: confirmed

## Description

`validate_invariants` checks that all `ranges` are pairwise disjoint
with a nested loop: `for (index, left) in ranges.iter().enumerate() {
for right in ranges.iter().skip(index.saturating_add(1)) { ... } }`.
For N ranges this is N²/2 `is_disjoint` calls, each of which is itself
O(1) (`intersection`). At `MAX_SHARD_COUNT = 65_536`, this is roughly
2.1 billion comparisons on every call.

## Evidence

```rust
for (index, left) in ranges.iter().enumerate() {
    for right in ranges.iter().skip(index.saturating_add(1)) {
        if !left.is_disjoint(*right) {
            return Err("ranges overlap");
        }
    }
}
```

(`crates/vb_core/src/shard/partition/mod.rs:389-395`)

## Adversarial Check

A defender might argue "this only runs once at construction, not in the
hot path." But construction runs on every shard-plan reload, and the
preceding loop at lines 375-388 already verifies contiguity in O(N)
(`windows(2)`). Contiguity implies disjointness — adjacent non-overlapping
ranges that together cover the full keyspace are by construction
pairwise disjoint. The O(N²) check is redundant.

## Suggested Fix

Delete the O(N²) disjointness loop. The contiguity check at lines 375-388
+ the start <= end check at lines 370-374 together imply disjointness
and exhaustiveness. If redundant safety is desired, document why.
