# Proof Repair Guide — vb-qi37.5.3

## Rejected Findings

### LETHAL-1 (vb_runtime_admission_proofs.rs:167)
**Problem**: Type mismatch — `&Box::new([])` is `&Box<[_; 0]>` (empty array literal infers `[i32; 0]`), but `spec_field_type_is_boxed_slice` expects `&Box<[ActionId]>` where `ActionId = u128`. Rust does not coerce between distinct boxed slice element types.

**Fix**: Replace `&Box::new([])` in the `ensures` clause with an explicitly typed empty boxed slice:

```rust
// BEFORE (line 167):
ensures
    spec_field_type_is_boxed_slice(&Box::new([])),

// AFTER — use cast to explicit type:
ensures
    spec_field_type_is_boxed_slice(&(Box::new([]) as Box<[ActionId]>)),
```

Or define a helper function and call it:
```rust
fn empty_action_id_box() -> Box<[ActionId]> { Box::new([]) }

pub proof fn proof_field_type_match()
    ensures
        spec_field_type_is_boxed_slice(&empty_action_id_box()),
{
}
```

### LETHAL-2 (vb_runtime_idempotency_proofs.rs:71)
**Problem**: Named return parameters in `pub open spec fn` signature — `-> (new_completed_len: int, evicted_key: Option<u128>)` is not valid Verus syntax. Verus spec functions do not support named return parameters in the signature.

**Fix**: Change the return type to remove named parameters and restructure the ensures clause:

```rust
// BEFORE (lines 66-90):
pub open spec fn spec_insert_with_eviction(
    old_completed_len: int,
    capacity: int,
    new_key: u128,
    order: &[u128],
) -> (new_completed_len: int, evicted_key: Option<u128>)
    requires
        old_completed_len >= 0,
        capacity >= 1,
        old_completed_len <= capacity + 1,
    ensures
        new_completed_len >= 0,
        new_completed_len <= capacity,
        if old_completed_len > capacity {
            new_completed_len == old_completed_len - 1
        } else {
            new_completed_len == old_completed_len
        },

// AFTER — use anonymous return type:
pub open spec fn spec_insert_with_eviction(
    old_completed_len: int,
    capacity: int,
    new_key: u128,
    order: &[u128],
) -> (int, Option<u128>)
    requires
        old_completed_len >= 0,
        capacity >= 1,
        old_completed_len <= capacity + 1,
    ensures
        // Use result.0 and result.1 to reference return values in ensures
        // Or use a named spec variable approach
        old_completed_len > capacity ==> (old_completed_len - 1) >= 0
            && (old_completed_len - 1) <= capacity,
```

Note: The ensures clause also needs to be updated since it references named return values. The ensures should express the same guarantees without named parameters — use a spec variable or restructure.

### ADDITIONAL: verification-layers.md missing
**Problem**: The contract-verification-reviewer requires `verification-layers.md` as a mandatory artifact.

**Fix**: Create `verification/verification-layers.md` (or `.beads/vb-qi37.5.3/verification-layers.md`) that documents:
- The verification layers used (verus, kani, miri, loom, proptest, cargo-test)
- Which obligations belong to which layer
- The dependency chain: vb_runtime must compile (DEFERRED_GLOBAL) before vb_runtime proofs can run
- Kani on vb_storage is independent and can proceed now

## Rerun Targets

After fixes, rerun:
1. `verus verification/verus/vb_runtime_admission_proofs.rs` — must exit 0
2. `verus verification/verus/vb_runtime_idempotency_proofs.rs` — must exit 0
3. `cargo kani --harness verification_proof_flags_harness -p vb_storage` — must complete with no failures

## Exit Criteria

- verus exits 0 on both files → Verus obligations PASS
- cargo kani completes with no failures → Kani obligations PASS
- verification-layers.md exists → Contract artifact completeness PASS
