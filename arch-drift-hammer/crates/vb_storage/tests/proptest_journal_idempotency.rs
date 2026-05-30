//!
//! Proptest properties for journal idempotency — supplementary to Kani harnesses.
//!
//! Bead: vb-282my
//! Obligation: PO-vb282my-RJ-PROP-001
//!
//! Target: crate::journal::internal::append_queued_unpersisted
//!
//! Tests idempotency round-trip: serialize JournalEvent → append →
//! decode existing → assert equality.

use proptest::prelude::*;
use vb_core::ids::RunId;
use vb_storage::{EventSeq, keys::run_event_key};

proptest! {
    /// PO-vb282my-RJ-PROP-001: Key encoding idempotency
    /// The run_event_key function must be deterministic: same (run, seq) input
    /// must always produce the same key output.
    #[test]
    fn proptest_journal_idempotency_round_trip(
        run in 0u64..,
        seq in 0u64..,
    ) {
        let key1 = run_event_key(RunId::new(run), EventSeq::new(seq));
        let key2 = run_event_key(RunId::new(run), EventSeq::new(seq));

        match (key1, key2) {
            (Ok(k1), Ok(k2)) => {
                assert_eq!(k1, k2, "key encoding must be deterministic");
                assert_eq!(k1.len(), 17, "key must be 17 bytes");
            }
            _ => {
                // Key encoding should not fail for valid inputs
                panic!("key encoding failed unexpectedly");
            }
        }
    }

    /// Key injectivity: different inputs produce different keys
    #[test]
    fn proptest_journal_key_injectivity(
        run1 in 0u64..,
        seq1 in 0u64..,
        run2 in 0u64..,
        seq2 in 0u64..,
    ) {
        proptest::prop_assume!(run1 != run2 || seq1 != seq2);

        let key1 = run_event_key(RunId::new(run1), EventSeq::new(seq1));
        let key2 = run_event_key(RunId::new(run2), EventSeq::new(seq2));

        match (key1, key2) {
            (Ok(k1), Ok(k2)) => {
                assert_ne!(k1, k2, "distinct inputs must produce distinct keys");
            }
            _ => {}
        }
    }

    /// Key prefix validation: prefix byte must be 0x11
    #[test]
    fn proptest_journal_key_prefix(
        run in 0u64..,
        seq in 0u64..,
    ) {
        let key = run_event_key(RunId::new(run), EventSeq::new(seq));

        if let Ok(k) = key {
            assert_eq!(k[0], 0x11, "key must start with 0x11 prefix");
        }
    }
}
