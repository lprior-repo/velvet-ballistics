//! MockMarker hash consistency tests.
//!
//! Verifies that hash behavior is consistent with PartialEq.
//! Gated behind `vb-rxru0-mock-marker` feature.
//!
//! Obligations: RFO-006
//! Contract clauses: C-MOCK-4 (hash consistency)

#![forbid(unsafe_code)]
#![cfg(feature = "vb-rxru0-mock-marker")]

use std::hash::{Hash, Hasher, DefaultHasher};

use vb_core::action::MockMarker;

fn hash_of(m: MockMarker) -> u64 {
    let mut h = DefaultHasher::new();
    m.hash(&mut h);
    h.finish()
}

#[test]
fn test_mock_marker_hash_consistency() {
    // Consistency: equal values produce equal hashes.
    let m1 = MockMarker::GithubIssueCreate;
    let m2 = MockMarker::GithubIssueCreate;
    assert_eq!(m1, m2, "setup: values must be equal");
    assert_eq!(hash_of(m1), hash_of(m2), "equal values must produce equal hashes");

    let m1 = MockMarker::AiClassifyTicket;
    let m2 = MockMarker::AiClassifyTicket;
    assert_eq!(m1, m2, "setup: values must be equal");
    assert_eq!(hash_of(m1), hash_of(m2), "equal values must produce equal hashes");

    let m1 = MockMarker::HttpGet;
    let m2 = MockMarker::HttpGet;
    assert_eq!(m1, m2, "setup: values must be equal");
    assert_eq!(hash_of(m1), hash_of(m2), "equal values must produce equal hashes");
}

#[test]
fn test_mock_marker_hash_anti_consistency() {
    // Different values must differ in hash (deterministically — only 3 variants).
    let h0 = hash_of(MockMarker::GithubIssueCreate);
    let h1 = hash_of(MockMarker::AiClassifyTicket);
    let h2 = hash_of(MockMarker::HttpGet);

    assert_ne!(
        h0, h1,
        "Different MockMarker variants must produce different hashes"
    );
    assert_ne!(
        h1, h2,
        "Different MockMarker variants must produce different hashes"
    );
    assert_ne!(
        h0, h2,
        "Different MockMarker variants must produce different hashes"
    );
}

#[test]
fn test_mock_marker_hash_stability() {
    // Hash is stable across invocations.
    assert_eq!(
        hash_of(MockMarker::GithubIssueCreate),
        hash_of(MockMarker::GithubIssueCreate),
        "GithubIssueCreate hash must be stable"
    );
    assert_eq!(
        hash_of(MockMarker::AiClassifyTicket),
        hash_of(MockMarker::AiClassifyTicket),
        "AiClassifyTicket hash must be stable"
    );
    assert_eq!(
        hash_of(MockMarker::HttpGet),
        hash_of(MockMarker::HttpGet),
        "HttpGet hash must be stable"
    );
}
