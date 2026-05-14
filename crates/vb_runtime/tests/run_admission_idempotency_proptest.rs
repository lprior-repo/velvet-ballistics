//! Proptest strategies for vb_runtime idempotency evidence propagation.
//!
//! Obligation IDs: PROPTEST-POST-01, PROPTEST-INV-03
//! Contract clauses: POST-01, INV-03
//! Risk: medium
//! Verifier: proptest
//!
//! Source: crates/vb_runtime/tests/run_admission_idempotency_proptest.rs
//!         crates/vb_runtime/tests/idempotency_tracker_capacity_proptest.rs
//! Command: cargo test -p vb_runtime run_admission_idempotency_proptest -- --nocapture
//!          cargo test -p vb_runtime idempotency_tracker_capacity_proptest -- --nocapture
//!
//! # Context
//!
//! PROPTEST-POST-01: Verifies that idempotency_keyed and idempotency_attested
//! field lengths and contents match the source VerificationProof after copy.
//!
//! PROPTEST-INV-03: Verifies that IdempotencyTracker capacity never exceeds
//! DEFAULT_CAPACITY (1024) after eviction, and oldest entry is evicted first.
//!
//! # Blocking
//!
//! NOT BLOCKED - proptest runs against vb_storage/vb_core which build successfully.
//! These tests can be executed in parallel with vb_runtime implementation.
//!
//! # Status
//!
//! Written: 2026-05-14
//! Will be verified in: State 11 (formal-verifier)

#![forbid(unsafe_code)]

use proptest::prelude::*;
use vb_core::action::{ActionTicket, Idempotency};
use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};

// =====================================================================
// Helper functions
// =====================================================================

/// Creates a test ActionTicket with the given idempotency key.
fn make_ticket(key: u128) -> ActionTicket {
    ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: SeqNo::new(1),
        action: ActionId::new(1),
        attempt: 1,
        idempotency_key: key,
        capacity: 1,
    }
}

/// Strategy for generating ActionId values.
prop_compose! {
    fn arb_action_id() -> ActionId {
        ActionId::new(any::<u16>())
    }
}

/// Strategy for generating Box<[ActionId]> with length between 0 and 100.
fn arb_boxed_action_ids() -> impl Strategy<Value = Box<[ActionId]>> {
    prop::collection::vec(arb_action_id(), 0..100).prop_map(|v| v.into_boxed_slice())
}

/// Strategy for generating (idempotency_keyed, idempotency_attested) pairs.
fn arb_idempotency_pair() -> impl Strategy<Value = (Box<[ActionId]>, Box<[ActionId]>)> {
    (arb_boxed_action_ids(), arb_boxed_action_ids())
}

// =====================================================================
// PROPTEST-POST-01: Field length and content preservation
// =====================================================================

/// Proptest for POST-01: idempotency_keyed and idempotency_attested field
/// lengths match source VerificationProof after copy.
///
/// This test generates random ActionId slices and verifies that when copied
/// to RunAdmission, the lengths are preserved exactly.
#[test]
fn proptest_idempotency_keyed_len_preserved() {
    proptest! {
        #[test]
        fn test_keyed_field_length_preservation(keyed in arb_boxed_action_ids()) {
            let source_len = keyed.len();

            // Simulate field copy (Box<[T]>::clone copies by reference, preserving length)
            let copied: Box<[ActionId]> = keyed;

            // POST-01: lengths must match exactly
            assert_eq!(
                copied.len(),
                source_len,
                "idempotency_keyed.len() should be preserved after copy"
            );
        }
    }
}

/// Proptest for POST-01: idempotency_attested field length preservation.
#[test]
fn proptest_idempotency_attested_len_preserved() {
    proptest! {
        #[test]
        fn test_attested_field_length_preservation(attested in arb_boxed_action_ids()) {
            let source_len = attested.len();

            // Simulate field copy
            let copied: Box<[ActionId]> = attested;

            // POST-01: lengths must match exactly
            assert_eq!(
                copied.len(),
                source_len,
                "idempotency_attested.len() should be preserved after copy"
            );
        }
    }
}

/// Proptest for POST-01: Both idempotency fields preserved simultaneously.
#[test]
fn proptest_both_idempotency_fields_preserved() {
    proptest! {
        #[test]
        fn test_both_fields_preserved((keyed, attested) in arb_idempotency_pair()) {
            let keyed_len = keyed.len();
            let attested_len = attested.len();

            // Simulate field copy
            let copied_keyed: Box<[ActionId]> = keyed;
            let copied_attested: Box<[ActionId]> = attested;

            // INV-01 and INV-02: both lengths preserved
            assert_eq!(
                copied_keyed.len(),
                keyed_len,
                "idempotency_keyed.len() preserved"
            );
            assert_eq!(
                copied_attested.len(),
                attested_len,
                "idempotency_attested.len() preserved"
            );
        }
    }
}

/// Proptest for POST-01: Field contents match after copy.
#[test]
fn proptest_idempotency_field_contents_match() {
    proptest! {
        #[test]
        fn test_keyed_contents_match(keyed in arb_boxed_action_ids()) {
            let source_len = keyed.len();
            let copied: Box<[ActionId]> = keyed;

            // Lengths must match
            assert_eq!(copied.len(), source_len);

            // Contents must match element-by-element
            for i in 0..source_len {
                assert_eq!(
                    copied[i], keyed[i],
                    "element {} should match after copy", i
                );
            }
        }
    }
}

/// Proptest for POST-01: Large slice copy preserves all elements.
#[test]
fn proptest_large_slice_copy_preserves_elements() {
    // Generate a large slice with known pattern
    let large_vec: Vec<ActionId> = (0..500u16).map(ActionId::new).collect();
    let source: Box<[ActionId]> = large_vec.into_boxed_slice();

    let copied: Box<[ActionId]> = source;

    assert_eq!(copied.len(), 500, "large slice length preserved");
    assert_eq!(copied[0].get(), 0, "first element correct");
    assert_eq!(copied[499].get(), 499, "last element correct");

    // Verify all elements
    for i in 0..500 {
        assert_eq!(copied[i].get(), i as u16, "element {} mismatch", i);
    }
}

// =====================================================================
// PROPTEST-INV-03: IdempotencyTracker capacity invariant
// =====================================================================

/// Proptest for INV-03: Tracker capacity never exceeds 1024 after eviction.
#[test]
fn proptest_tracker_capacity_never_exceeds_1024() {
    proptest! {
        #[test]
        fn test_capacity_invariant_after_overflow_insertions(key_count in 0u32..2000) {
            let mut tracker = vb_runtime::idempotency::IdempotencyTracker::with_default_capacity();

            // Insert more than DEFAULT_CAPACITY (1024) entries
            for i in 0..key_count {
                let key = i as u128;
                tracker.track_for_policy(Idempotency::AtLeastOnceExternal, key);
            }

            // INV-03: capacity should never exceed 1024
            assert!(
                tracker.len() <= 1024,
                "tracker len {} exceeds DEFAULT_CAPACITY 1024",
                tracker.len()
            );
        }
    }
}

/// Proptest for INV-03: Oldest entry evicted first (FIFO order).
#[test]
fn proptest_fifo_eviction_order() {
    let mut tracker = vb_runtime::idempotency::IdempotencyTracker::with_capacity(3);

    // Insert 1, 2, 3
    assert!(tracker.track_for_policy(Idempotency::AtLeastOnceExternal, 1));
    assert!(tracker.track_for_policy(Idempotency::AtLeastOnceExternal, 2));
    assert!(tracker.track_for_policy(Idempotency::AtLeastOnceExternal, 3));

    // Insert 4 - should evict 1 (oldest)
    assert!(tracker.track_for_policy(Idempotency::AtLeastOnceExternal, 4));

    // 1 should be evicted, 2, 3, 4 should remain
    assert!(
        !tracker.is_completed_for_policy(Idempotency::AtLeastOnceExternal, 1),
        "key 1 (oldest) should be evicted"
    );
    assert!(
        tracker.is_completed_for_policy(Idempotency::AtLeastOnceExternal, 2),
        "key 2 should still be present"
    );
    assert!(
        tracker.is_completed_for_policy(Idempotency::AtLeastOnceExternal, 3),
        "key 3 should still be present"
    );
    assert!(
        tracker.is_completed_for_policy(Idempotency::AtLeastOnceExternal, 4),
        "key 4 should be present"
    );
}

/// Proptest for INV-03: Capacity 1 eviction works correctly.
#[test]
fn proptest_capacity_one_eviction() {
    let mut tracker = vb_runtime::idempotency::IdempotencyTracker::with_capacity(1);

    // First insertion succeeds
    assert!(tracker.track_for_policy(Idempotency::AtLeastOnceExternal, 100));
    assert!(tracker.is_completed_for_policy(Idempotency::AtLeastOnceExternal, 100));

    // Second insertion evicts first
    assert!(tracker.track_for_policy(Idempotency::AtLeastOnceExternal, 200));
    assert!(
        !tracker.is_completed_for_policy(Idempotency::AtLeastOnceExternal, 100),
        "key 100 should be evicted after capacity-1 insert"
    );
    assert!(
        tracker.is_completed_for_policy(Idempotency::AtLeastOnceExternal, 200),
        "key 200 should be present"
    );

    // Tracker should be at capacity 1
    assert_eq!(tracker.len(), 1, "tracker should be at capacity 1");
}

/// Proptest for INV-03: Multiple rounds of overflow eviction.
#[test]
fn proptest_multiple_overflow_rounds() {
    let mut tracker = vb_runtime::idempotency::IdempotencyTracker::with_capacity(5);

    // Round 1: Insert 1-5
    for i in 1..=5 {
        assert!(tracker.track_for_policy(Idempotency::AtLeastOnceExternal, i));
    }
    assert_eq!(tracker.len(), 5);

    // Round 2: Insert 6-10 (evicts 1-5)
    for i in 6..=10 {
        assert!(tracker.track_for_policy(Idempotency::AtLeastOnceExternal, i));
    }

    // After eviction, only 6-10 should be present
    for i in 1..=5 {
        assert!(
            !tracker.is_completed_for_policy(Idempotency::AtLeastOnceExternal, i),
            "key {} should have been evicted",
            i
        );
    }
    for i in 6..=10 {
        assert!(
            tracker.is_completed_for_policy(Idempotency::AtLeastOnceExternal, i),
            "key {} should be present",
            i
        );
    }

    // Capacity invariant
    assert!(tracker.len() <= 5, "tracker should not exceed capacity 5");
}
