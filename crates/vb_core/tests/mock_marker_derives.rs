//! MockMarker derive trait tests.
//!
//! Verifies that all required derives compile and behave correctly.
//! Gated behind `vb-rxru0-mock-marker` feature.
//!
//! Obligations: RFO-003
//! Contract clauses: C-MOCK-2 (Copy, PartialEq, Eq)

#![forbid(unsafe_code)]
#![cfg(feature = "vb-rxru0-mock-marker")]

use std::hash::{DefaultHasher, Hash, Hasher};

use vb_core::action::MockMarker;

#[test]
fn test_mock_marker_derives() {
    // PartialEq + Eq
    assert_eq!(
        MockMarker::GithubIssueCreate,
        MockMarker::GithubIssueCreate,
        "same variant must be equal"
    );
    assert_ne!(
        MockMarker::GithubIssueCreate,
        MockMarker::AiClassifyTicket,
        "different variants must not be equal"
    );
    assert_ne!(
        MockMarker::AiClassifyTicket,
        MockMarker::HttpGet,
        "different variants must not be equal"
    );
    assert_ne!(
        MockMarker::GithubIssueCreate,
        MockMarker::HttpGet,
        "different variants must not be equal"
    );

    // Debug
    let s = format!("{:?}", MockMarker::AiClassifyTicket);
    assert!(
        s.contains("AiClassifyTicket"),
        "Debug display must contain variant name: {s}"
    );

    let s = format!("{:?}", MockMarker::GithubIssueCreate);
    assert!(
        s.contains("GithubIssueCreate"),
        "Debug display must contain variant name: {s}"
    );

    let s = format!("{:?}", MockMarker::HttpGet);
    assert!(
        s.contains("HttpGet"),
        "Debug display must contain variant name: {s}"
    );
}

#[test]
fn test_mock_marker_hash_basic() {
    // Hash of same variant must be same.
    assert_eq!(
        hash_of(MockMarker::GithubIssueCreate),
        hash_of(MockMarker::GithubIssueCreate),
        "same variant must have same hash"
    );

    // Hash of different variants must differ.
    let h_github = hash_of(MockMarker::GithubIssueCreate);
    let h_ai = hash_of(MockMarker::AiClassifyTicket);
    let h_http = hash_of(MockMarker::HttpGet);

    assert_ne!(
        h_github, h_ai,
        "GithubIssueCreate and AiClassifyTicket must hash differently"
    );
    assert_ne!(
        h_ai, h_http,
        "AiClassifyTicket and HttpGet must hash differently"
    );
    assert_ne!(
        h_github, h_http,
        "GithubIssueCreate and HttpGet must hash differently"
    );
}

fn hash_of(m: MockMarker) -> u64 {
    let mut h = DefaultHasher::new();
    m.hash(&mut h);
    h.finish()
}
