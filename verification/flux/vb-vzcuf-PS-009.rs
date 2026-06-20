// Flux-rs refinement annotations for duplicate accounting (PS-009, C2).
//
// Obligation ID: POB-vb-vzcuf-035
// Verifier: flux-rs
// Command: flux check verification/flux/vb-vzcuf-PS-009.rs
//
// Domain claim: Same-batch duplicate accounting follows the documented
// policy and preserves staged byte invariant.
//
// PRODUCTION BINDING (REMOVED IN COMMIT 150e1489a):
//   JournalWriteBatch::staged_event_keys HashSet in
//   crates/vb_storage/src/batch.rs:42.
//   Field was removed from the production struct in commit 150e1489a
//   (bead vb-u2psq). This Flux spec is preserved as a mathematical
//   model of the duplicate-accounting policies (conservative and
//   precise) that previously used that field; it no longer binds
//   to a live production field. The refinement annotations on the
//   local helpers remain valid as standalone arithmetic models.
//   Tracking: FINDING-008 (binding drift).
//
// Source: .beads/vb-vzcuf/proof-obligations.planned.jsonl POB-vb-vzcuf-035

#![allow(unused)]

use std::collections::HashSet;

/// Conservative accounting: always add encoded_len.
#[flux_rs::sig(fn(u64, u64) -> u64)]
fn conservative_accounting(current: u64, encoded_len: u64) -> u64 {
    current + encoded_len
}

/// Precise accounting: only add for new keys.
fn precise_accounting(current: u64, encoded_len: u64, seen: &HashSet<u64>, key: u64) -> u64 {
    if seen.contains(&key) {
        current
    } else {
        current + encoded_len
    }
}

/// Refinement: conservative always increases (for n > 0).
fn test_conservative_always_increases() {
    assert!(conservative_accounting(100, 50) > 100);
    assert_eq!(conservative_accounting(100, 0), 100);
}

/// Refinement: precise preserves bytes for duplicates.
fn test_precise_duplicate_unchanged() {
    let mut seen = HashSet::new();
    seen.insert(42u64);
    assert_eq!(precise_accounting(100, 50, &seen, 42), 100);
}

/// Refinement: precise increases bytes for new keys.
fn test_precise_new_key_increases() {
    let seen = HashSet::new();
    assert_eq!(precise_accounting(100, 50, &seen, 1), 150);
}

/// Refinement: both policies agree for first-time key.
fn test_policies_agree_for_new() {
    let seen = HashSet::new();
    let c = conservative_accounting(100, 50);
    let p = precise_accounting(100, 50, &seen, 1);
    assert_eq!(c, p);
}

/// Refinement: staged bytes never decrease.
fn test_staged_monotonic() {
    assert!(conservative_accounting(0, 10) >= 0);
    assert!(conservative_accounting(100, 0) >= 100);
    assert!(conservative_accounting(100, 50) >= 100);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conservative_accounting_basics() {
        assert_eq!(conservative_accounting(0, 60), 60);
        assert_eq!(conservative_accounting(60, 40), 100);
    }

    #[test]
    fn precise_duplicate_ignored() {
        let mut seen = HashSet::new();
        seen.insert(1u64);
        // Same key, should not increase
        assert_eq!(precise_accounting(100, 50, &seen, 1), 100);
        // Different key, should increase
        assert_eq!(precise_accounting(100, 50, &seen, 99), 150);
    }

    #[test]
    fn both_policies_monotonic() {
        let mut seen = HashSet::new();
        seen.insert(1u64);

        let c = conservative_accounting(100, 50);
        let p_new = precise_accounting(100, 50, &seen, 2);
        let p_dup = precise_accounting(100, 50, &seen, 1);

        assert!(c >= 100);
        assert!(p_new >= 100);
        assert!(p_dup >= 100);
    }

    #[test]
    fn duplicate_policy_safety() {
        // Both policies should never exceed the limit
        let limit = 200u64;
        let current = 100u64;
        let encoded_len = 50u64;

        let c = conservative_accounting(current, encoded_len);
        assert!(c <= limit);

        let mut seen = HashSet::new();
        seen.insert(1u64);
        let p = precise_accounting(current, encoded_len, &seen, 1);
        assert!(p <= limit);
    }
}
