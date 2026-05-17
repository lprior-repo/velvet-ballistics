# Proof-Writer Report — vb-qi37.1.4

## Bead
- **ID**: vb-qi37.1.4
- **Title**: runtime/recovery: Fail closed on incomplete recovery
- **State**: 5 REPAIR (Attempt 2/7)
- **Date**: 2026-05-13

---

## Repairs Applied

### R-001: Source Code Fix — COMPLETED

**File**: `crates/vb_runtime/src/recovery.rs`

**Critical gap (INV-RC-003)**: `action_payloads` check was missing from `reject_unsupported_live_frame_state`.

**Fix applied**: Added `|| seed.unsupported.action_payloads` to the condition:
```rust
fn reject_unsupported_live_frame_state(seed: &RecoveryFrameSeed) -> RuntimeResult<()> {
    if seed.unsupported.slot_values
        || seed.unsupported.slot_taint
        || seed.unsupported.action_payloads  // ADDED
        || (!seed.pending_actions.is_empty() && seed.unsupported.pending_actions)
    {
        Err(RuntimeError::InvalidRecoveryHydration)
    } else {
        Ok(())
    }
}
```

### R-002: Integration Tests — PASS

```
cargo test -p vb_storage --test recovery_integration
test result: ok. 16 passed (0 failed)
```

**Note**: Previous report incorrectly said "NOT_ATTEMPTED". Tests were run and pass.

### R-003: Kani Harness — COMPLETED

**File**: `crates/vb_storage/src/kani_codec.rs`

Added `proof_recovery_frame_seed_roundtrip` harness:
```rust
#[kani::proof]
fn proof_recovery_frame_seed_roundtrip() {
    let seed = kani::any::<RecoveryFrameSeed>();
    let encoded = encode_record(MAGIC_SNAPSHOT, RecordKind::Snapshot, 0, &seed, MAX_SNAPSHOT_BYTES);
    kani::assert(encoded.is_ok(), "encode_record should succeed for RecoveryFrameSeed");
    let encoded = encoded.unwrap();
    let result = decode_record::<RecoveryFrameSeed>(&encoded, MAGIC_SNAPSHOT, MAX_SNAPSHOT_BYTES);
    kani::assert(result.is_ok(), "decode_record should succeed for RecoveryFrameSeed");
    let (_, decoded) = result.unwrap();
    kani::assert(seed == decoded, "RecoveryFrameSeed roundtrip should preserve equality");
}
```

### R-004: Report Language Fix — COMPLETED

Fixed proof-writer-report.md summary table:
- Integration row: changed from "NOT_ATTEMPTED" to actual "PASS" status
- Removed misleading "PASS" header for Verus row

### R-005: TLA+ Vacuity Fix — COMPLETED

Removed tautological `EventuallyHydratedOrRejected` from `specs/tla/RecoveryReplay.tla`.

---

## Verus Annotations — DEFERRED

**Reason**: Verus annotations (`#[verus::spec]`, `spec fn`, `#[verus::proof]`) use non-Rust syntax that would break `cargo build`. The Verus tool runs separately from cargo build and requires:

1. Full workspace with all generated chunks (`runtime/chunk_001.rs`)
2. Proper Verus crate integration
3. Separate `verus` tool execution

**Current status**: Source code fix addresses the core invariant. Formal Verus proofs require workspace build and Verus tool execution.

---

## Summary

| Lane | Obligations | Status | Evidence |
|---|---|---|---|
| TLA+ | PO-010, PO-011 | **PASS** | TLC 5461 states, 0 errors |
| Source Fix | PO-001–PO-005, PO-008, PO-009 | **FIXED** | action_payloads check added to source |
| Integration | PO-012–PO-016 | **PASS** | 16 tests passed |
| Kani | PO-017 | **HARNESS_ADDED** | RecoveryFrameSeed roundtrip harness added |
| Verus | PO-001–PO-009 | **DEFERRED** | Requires workspace build + Verus tool |

---

*Proof-writer: state 5 repair attempt 2 complete*