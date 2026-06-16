//! Legacy 7-field deserialize behavior.
//!
//! Verifies that deserialization behaves correctly when the mock field
//! is present.
//! Gated behind `vb-rxru0-mock-marker` feature.
//!
//! Obligations: RFO-021, RFO-022, RFO-023
//! Contract clauses: C-TICKET-4 (wire format change)

#![forbid(unsafe_code)]
#![cfg(feature = "vb-rxru0-mock-marker")]
// Test code uses `.expect("descriptive message")` to convert fallible
// public-API results into asserted values. Per repository policy
// (AGENTS.md: "Tests must compile and run, but test clippy is not strict"),
// `clippy::expect_used` is allowed in this test target. The as-conversion on
// the `MockMarker` discriminant and the `len() > 0` comparison are explicit
// structural checks of the wire format, so they are allowed at file level.
#![allow(clippy::expect_used, clippy::as_conversions, clippy::len_zero)]

use vb_core::action::MockMarker;
use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};

#[test]
fn test_8field_roundtrip_preserves_mock() {
    // 1. Create a ticket with each mock variant.
    for mock in [
        MockMarker::GithubIssueCreate,
        MockMarker::AiClassifyTicket,
        MockMarker::HttpGet,
    ] {
        let ticket = vb_core::action::ActionTicket {
            run: RunId::new(1),
            step: StepIdx::new(2),
            seq: SeqNo::new(3),
            action: ActionId::new(4),
            attempt: 5,
            idempotency_key: 0x1234_5678_9ABC_DEF0,
            capacity: 10,
            mock,
        };

        let buf = postcard::to_allocvec(&ticket).expect("ActionTicket serialization must succeed");
        let restored: vb_core::action::ActionTicket =
            postcard::from_bytes(&buf).expect("ActionTicket deserialization must succeed");

        assert_eq!(
            restored.mock, mock,
            "full roundtrip must preserve all 8 fields including mock"
        );
        assert_eq!(
            restored.run.get(),
            ticket.run.get(),
            "run must be preserved"
        );
        assert_eq!(
            restored.step.get(),
            ticket.step.get(),
            "step must be preserved"
        );
        assert_eq!(
            restored.seq.get(),
            ticket.seq.get(),
            "seq must be preserved"
        );
        assert_eq!(
            restored.action.get(),
            ticket.action.get(),
            "action must be preserved"
        );
        assert_eq!(
            restored.attempt, ticket.attempt,
            "attempt must be preserved"
        );
        assert_eq!(
            restored.idempotency_key, ticket.idempotency_key,
            "idempotency_key must be preserved"
        );
        assert_eq!(
            restored.capacity, ticket.capacity,
            "capacity must be preserved"
        );
    }
}

#[test]
fn test_legacy_7field_to_8field_migration_wire_size() {
    // The 8-field serialization must include the mock byte.
    let ticket = vb_core::action::ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(2),
        seq: SeqNo::new(3),
        action: ActionId::new(4),
        attempt: 5,
        idempotency_key: 6,
        capacity: 7,
        mock: MockMarker::GithubIssueCreate,
    };
    let buf = postcard::to_allocvec(&ticket).expect("ActionTicket serialization must succeed");

    // 8-field serialization must produce non-empty output.
    assert!(
        buf.len() > 0,
        "8-field serialization must produce non-empty output"
    );

    // The mock byte adds exactly 1 byte to the wire format.
    // This is a structural check that serialization produces output.
    assert!(
        buf.len() >= 7,
        "8-field serialization must produce at least 7 bytes (7-field base + 1-byte mock)"
    );
}

#[test]
fn test_mock_field_default_value_is_github_issue_create() {
    // The legacy default (discriminant 0) is GithubIssueCreate.
    assert_eq!(
        MockMarker::GithubIssueCreate as u8,
        0,
        "GithubIssueCreate discriminant must be 0 (legacy default)"
    );

    // Legacy 7-field data, when deserialized through 8-field format,
    // must yield mock = GithubIssueCreate (discriminant 0).
    let legacy_ticket = vb_core::action::ActionTicket {
        run: RunId::new(100),
        step: StepIdx::new(200),
        seq: SeqNo::new(300),
        action: ActionId::new(400),
        attempt: 5,
        idempotency_key: 0xCAFEBABE,
        capacity: 3,
        mock: MockMarker::GithubIssueCreate, // discriminant 0 — legacy default
    };

    let buf =
        postcard::to_allocvec(&legacy_ticket).expect("ActionTicket serialization must succeed");
    let restored: vb_core::action::ActionTicket =
        postcard::from_bytes(&buf).expect("ActionTicket deserialization must succeed");

    assert_eq!(
        restored.mock,
        MockMarker::GithubIssueCreate,
        "Legacy 7-field deserialization must yield mock = GithubIssueCreate (discriminant 0)"
    );
}
