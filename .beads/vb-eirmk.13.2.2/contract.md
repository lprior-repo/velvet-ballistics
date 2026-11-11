## Contract: Kani Partition Plan Safety Verification

**Bead ID:** vb-eirmk.13.2.2

**Obligations:**
- PO-010a: `PartitionPlan::from_config()` is panic-free for N ∈ [1, 32] and any u64 keyspace
- PO-010b: Successful construction satisfies 5 invariants (length, start, end, contiguity, disjointness)
- PO-010c: Undersized keyspaces (span < N) return exact `PartitionError::InvalidKeyRange`

**Inputs:**
- Shard count: N ∈ [1, 32] (bounded for Kani)
- Keyspace: [0, kmax] for any kmax ∈ u64
- No assumptions that exclude undersized keyspaces (span < N)

**Postconditions:**
On success:
1. `ranges.len() == N`
2. `ranges[0].start() == 0`
3. `ranges[N-1].end() == kmax`
4. For all adjacent pairs: `ranges[i].end() + 1 == ranges[i+1].start()` (contiguity)
5. For all pairs i < j: `ranges[i].is_disjoint(ranges[j])` (no overlap)

On failure:
- Returns `Err(PartitionError)` with exact error variant
- No panics, no unwrap/expect/todo

**Scope:**
- File: `crates/vb_core/src/verification/kani/partition_kani_partition_plan_safety.rs`
- Uses real production types: `PartitionPlan`, `PartitionConfig`, `ShardCount`, `KeyRange`, `PartitionError`
- Calls actual `PartitionPlan::from_config(&PartitionConfig::new(...))`
- 3 named harnesses, each with `#[kani::unwind(36)]`