#![allow(clippy::expect_used, clippy::unwrap_used, clippy::as_conversions, clippy::panic)]
//! MockMarker serialization tests.
//!
//! Verifies postcard serialization roundtrip for MockMarker.
//! Wire format is exactly 1 byte per variant.
//! Gated behind `vb-rxru0-mock-marker` feature.
//!
//! Obligations: RFO-004, RFO-005, RFO-006 (proptest portion)
//! Contract clauses: C-MOCK-3 (serde roundtrip, 1-byte wire format)

#![forbid(unsafe_code)]
#![cfg(feature = "vb-rxru0-mock-marker")]

use proptest::prelude::{any, prop_assert, prop_assert_eq, prop_oneof, proptest};
use proptest::strategy::Strategy;
use vb_core::action::MockMarker;

fn arb_mock_marker() -> impl proptest::strategy::Strategy<Value = MockMarker> {
    prop_oneof![
        1 => any::<u8>().prop_map(|_v| MockMarker::GithubIssueCreate),
        1 => any::<u8>().prop_map(|_v| MockMarker::AiClassifyTicket),
        1 => any::<u8>().prop_map(|_v| MockMarker::HttpGet),
    ]
}

#[test]
fn test_mock_marker_serde_roundtrip() {
    let variants = [
        MockMarker::GithubIssueCreate,
        MockMarker::AiClassifyTicket,
        MockMarker::HttpGet,
    ];

    for m in &variants {
        // Serialization produces exactly 1 byte.
        let buf = postcard::to_allocvec(m).expect("MockMarker serialization must succeed");
        assert_eq!(
            buf.len(),
            1,
            "MockMarker wire format must be exactly 1 byte, got {}",
            buf.len()
        );

        // Roundtrip: deserialize produces the original value.
        let m2: MockMarker =
            postcard::from_bytes(&buf).expect("MockMarker deserialization must succeed");
        assert_eq!(m2, *m, "Roundtrip must preserve MockMarker value");
    }
}

#[test]
fn test_mock_marker_serde_discriminant_bytes() {
    // Discriminant 0 serializes to byte 0, discriminant 1 to byte 1, etc.
    let buf0: Vec<u8> = postcard::to_allocvec(&MockMarker::GithubIssueCreate).unwrap();
    assert_eq!(
        buf0,
        [0_u8],
        "GithubIssueCreate (discriminant 0) must serialize to [0]"
    );

    let buf1: Vec<u8> = postcard::to_allocvec(&MockMarker::AiClassifyTicket).unwrap();
    assert_eq!(
        buf1,
        [1_u8],
        "AiClassifyTicket (discriminant 1) must serialize to [1]"
    );

    let buf2: Vec<u8> = postcard::to_allocvec(&MockMarker::HttpGet).unwrap();
    assert_eq!(
        buf2,
        [2_u8],
        "HttpGet (discriminant 2) must serialize to [2]"
    );
}

#[test]
fn test_mock_marker_serde_all_variants_not_empty() {
    let variants = [
        MockMarker::GithubIssueCreate,
        MockMarker::AiClassifyTicket,
        MockMarker::HttpGet,
    ];

    for m in &variants {
        let buf = postcard::to_allocvec(m).unwrap();
        assert!(
            !buf.is_empty(),
            "MockMarker serialization must produce non-empty output"
        );
        assert!(
            buf.len() <= 4,
            "MockMarker serialization must produce small output (<= 4 bytes)"
        );
    }
}

// ---------------------------------------------------------------------------
// Proptest: property-based MockMarker serialization
// ---------------------------------------------------------------------------

#[cfg(test)]
proptest! {
    /// Property-based serialization roundtrip for all MockMarker values.
    #[test]
    fn test_mock_marker_serde_all(m in arb_mock_marker()) {
        let buf = postcard::to_allocvec(&m).expect("serialize must succeed");
        prop_assert_eq!(buf.len(), 1, "MockMarker wire format must be exactly 1 byte");
        let m2: MockMarker = postcard::from_bytes(&buf).expect("deserialize must succeed");
        prop_assert_eq!(m2, m, "Roundtrip must preserve MockMarker");
    }

    /// Property-based hash consistency for MockMarker.
    #[test]
    fn test_mock_marker_hash_consistent(
        m1 in arb_mock_marker(),
        m2 in arb_mock_marker(),
    ) {
        let hash_of = |m: MockMarker| -> u64 {
            use std::hash::{Hash, Hasher, DefaultHasher};
            let mut h = DefaultHasher::new();
            m.hash(&mut h);
            h.finish()
        };

        prop_assert!(
            if m1 == m2 { hash_of(m1) == hash_of(m2) } else { hash_of(m1) != hash_of(m2) },
            "hash must be consistent with equality"
        );
    }
}
