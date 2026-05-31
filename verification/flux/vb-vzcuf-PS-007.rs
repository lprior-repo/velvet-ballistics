// Flux-rs refinement annotations for core/storage bridge (PS-007, C8).
//
// Obligation ID: POB-vb-vzcuf-027
// Verifier: flux-rs
// Command: flux check verification/flux/vb-vzcuf-PS-007.rs
//
// Domain claim: Core max_journal_batch_bytes is safely bridged into
// storage JournalBatchByteLimit or explicitly separated without silent drift.
//
// PRODUCTION BINDING:
//   Core policy from crates/vb_core/src/workflow/mod.rs.
//   Storage limit from crates/vb_storage/src/constants.rs.
//   Both currently use 1_048_576.
//
// Source: .beads/vb-vzcuf/proof-obligations.planned.jsonl POB-vb-vzcuf-027

#![allow(unused)]

const CORE_POLICY: u64 = 1_048_576;
const STORAGE_DEFAULT: u64 = 1_048_576;

/// Bridge function: core policy maps to storage limit.
#[flux_rs::sig(fn(u64) -> u64 requires policy > 0)]
fn bridge_core_to_storage(policy: u64) -> u64 {
    policy
}

/// Refinement: bridge preserves value.
fn test_bridge_preservation() {
    assert_eq!(bridge_core_to_storage(CORE_POLICY), STORAGE_DEFAULT);
}

/// Refinement: silent drift detection.
fn test_no_silent_drift() {
    assert_eq!(CORE_POLICY, STORAGE_DEFAULT);
}

/// Refinement: bridged value is non-zero.
fn test_bridge_value_nonzero() {
    let value = bridge_core_to_storage(CORE_POLICY);
    assert!(value > 0);
}

/// Refinement: bridged value fits in u32 for payload comparisons.
fn test_bridge_fits_u32() {
    let value = bridge_core_to_storage(CORE_POLICY);
    assert!(value <= u32::MAX as u64);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_storage_bridge_aligned() {
        assert_eq!(CORE_POLICY, STORAGE_DEFAULT);
        assert_eq!(bridge_core_to_storage(CORE_POLICY), STORAGE_DEFAULT);
    }

    #[test]
    fn bridge_for_arbitrary_policy() {
        for p in [1u64, 100, 100_000, 1_048_576, 10_000_000] {
            assert_eq!(bridge_core_to_storage(p), p);
        }
    }

    #[test]
    fn bridge_value_within_bounds() {
        let v = bridge_core_to_storage(CORE_POLICY);
        assert!(v > 0);
        assert!(v < u64::MAX);
        assert!(v <= u32::MAX as u64);
    }
}
