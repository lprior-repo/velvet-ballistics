//! Kani harnesses for RunId::shard_index bounds verification.
//!
//! **Obligations:**
//! - PO-004: shard_index returns value in [0, shard_count) when shard_count > 0
//! - PO-014: u64::MAX key produces valid shard index without overflow
//!
//! **Target:** `RunId::shard_index` (production const fn, `ids/mod.rs:238`)
//! **Domain:** RunId ∈ [0, u64::MAX], shard_count ∈ [0, u64::MAX]
//! **Trust Boundary (TB-004):** `u64::checked_rem` assumed correct as stdlib arithmetic.

#![forbid(unsafe_code)]

use super::RunId;

// ============================================================================
// PO-004: Shard index bounds for all u64 inputs
// ============================================================================

#[kani::proof]
fn shard_index_bounded() {
    let run: u64 = kani::any();
    let shard_count: u64 = kani::any();
    let run_id = RunId::new(run);

    let result = run_id.shard_index(shard_count);

    if shard_count > 0 {
        kani::assert(
            result < shard_count,
            "shard_index must be strictly less than shard_count",
        );
    } else {
        kani::assert(result == 0, "shard_index must be 0 when shard_count == 0");
    }

    // Non-vacuity: prove domain includes boundary values
    kani::cover!(run == 0 && shard_count == 1, "min key, 1 shard");
    kani::cover!(run == u64::MAX && shard_count == 1, "max key, 1 shard");
    kani::cover!(shard_count == 0, "zero shard_count reachable");
}

// ============================================================================
// PO-014: u64::MAX key edge case — exhaustive for all shard_count
// ============================================================================

#[kani::proof]
fn shard_index_u64_max() {
    let run_id = RunId::new(u64::MAX);
    let shard_count: u64 = kani::any();

    let result = run_id.shard_index(shard_count);

    if shard_count > 0 {
        kani::assert(
            result < shard_count,
            "u64::MAX key must route to valid shard index",
        );
        // Verify mathematical consistency
        if let Some(expected) = u64::MAX.checked_rem(shard_count) {
            kani::assert(
                result == expected,
                "shard_index must equal u64::MAX % shard_count",
            );
        }
    } else {
        kani::assert(result == 0, "zero shard_count fallback is 0");
    }

    kani::cover!(shard_count == 1, "u64::MAX with 1 shard");
    kani::cover!(shard_count == 2, "u64::MAX with 2 shards");
    kani::cover!(shard_count == u64::MAX, "u64::MAX with max shard_count");
}

// ============================================================================
// Non-vacuity: boundary coverage harness
// ============================================================================

#[kani::proof]
fn shard_index_cover_boundaries() {
    let run: u64 = kani::any();
    let shard_count: u64 = kani::any();
    let run_id = RunId::new(run);

    let _ = run_id.shard_index(shard_count);

    kani::cover!(run == 0, "run_id 0 reachable");
    kani::cover!(run == u64::MAX, "run_id u64::MAX reachable");
    kani::cover!(shard_count == 0, "shard_count 0 reachable");
    kani::cover!(shard_count == u64::MAX, "shard_count u64::MAX reachable");
}
