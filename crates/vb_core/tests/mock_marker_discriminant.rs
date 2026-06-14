//! MockMarker discriminant tests.
//!
//! These tests describe the expected behavior of MockMarker once implemented.
//! Gated behind `vb-rxru0-mock-marker` feature.
//!
//! Obligations: RFO-001, RFO-002
//! Contract clauses: C-MOCK-1 (variant count), C-MOCK-2 (unit variants)

#![forbid(unsafe_code)]
#![cfg(feature = "vb-rxru0-mock-marker")]

use vb_core::action::MockMarker;

#[test]
fn test_mock_marker_three_variants() {
    // Exhaustiveness: every MockMarker value matches one of the three variants.
    fn exhaustiveness(m: MockMarker) {
        match m {
            MockMarker::GithubIssueCreate => {}
            MockMarker::AiClassifyTicket => {}
            MockMarker::HttpGet => {}
        }
    }

    exhaustiveness(MockMarker::GithubIssueCreate);
    exhaustiveness(MockMarker::AiClassifyTicket);
    exhaustiveness(MockMarker::HttpGet);

    // Discriminant uniqueness: each variant has a distinct discriminant.
    assert_ne!(
        MockMarker::GithubIssueCreate as u8,
        MockMarker::AiClassifyTicket as u8,
        "GithubIssueCreate and AiClassifyTicket must have different discriminants"
    );
    assert_ne!(
        MockMarker::AiClassifyTicket as u8,
        MockMarker::HttpGet as u8,
        "AiClassifyTicket and HttpGet must have different discriminants"
    );
    assert_ne!(
        MockMarker::GithubIssueCreate as u8,
        MockMarker::HttpGet as u8,
        "GithubIssueCreate and HttpGet must have different discriminants"
    );

    // Discriminant values are 0, 1, 2 in declaration order.
    assert_eq!(
        MockMarker::GithubIssueCreate as u8,
        0,
        "GithubIssueCreate must have discriminant 0"
    );
    assert_eq!(
        MockMarker::AiClassifyTicket as u8,
        1,
        "AiClassifyTicket must have discriminant 1"
    );
    assert_eq!(
        MockMarker::HttpGet as u8,
        2,
        "HttpGet must have discriminant 2"
    );
}

#[test]
fn test_mock_marker_copy_trait() {
    // Compile-time: Copy trait must exist.
    fn compile_copy(m: MockMarker) -> MockMarker {
        let _copied = m; // Copy: m is moved, not borrowed
        m // m can be returned because it was copied
    }

    let m = MockMarker::GithubIssueCreate;
    let _returned = compile_copy(m);

    // Size check: all variants are zero-sized (unit), repr(u8).
    assert_eq!(
        std::mem::size_of::<MockMarker>(),
        1,
        "MockMarker size must be 1 byte (repr(u8) with 3 unit variants)"
    );

    // Copy implies Clone.
    let m = MockMarker::GithubIssueCreate;
    let _cloned = m.clone();
    let _copied2 = m; // m not moved — Copy
}
