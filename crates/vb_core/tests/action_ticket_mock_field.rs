//! ActionTicket with mock field tests.
//!
//! Verifies that ActionTicket includes the mock field, derives Copy,
//! and equality includes mock.
//! Gated behind `vb-rxru0-mock-marker` feature.
//!
//! Obligations: RFO-003, RFO-015, RFO-017
//! Contract clauses: C-TICKET-1 (Copy preserved), C-TICKET-3 (equality includes mock)

#![forbid(unsafe_code)]
#![cfg(feature = "vb-rxru0-mock-marker")]

use vb_core::action::MockMarker;
use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};

fn make_ticket(mock: MockMarker) -> vb_core::action::ActionTicket {
    vb_core::action::ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(2),
        seq: SeqNo::new(3),
        action: ActionId::new(4),
        attempt: 5,
        idempotency_key: 0xBEEF,
        capacity: 10,
        mock,
    }
}

#[test]
fn test_action_ticket_has_mock_field() {
    let ticket = make_ticket(MockMarker::AiClassifyTicket);

    // Mock field is accessible.
    assert_eq!(
        ticket.mock,
        MockMarker::AiClassifyTicket,
        "mock field must be accessible and correct"
    );

    // ActionTicket still derives Copy (no move of ticket).
    let _copied = ticket;
    assert_eq!(
        ticket.mock,
        MockMarker::AiClassifyTicket,
        "ticket must be usable after Copy — mock field accessible"
    );
}

#[test]
fn test_action_ticket_equality_includes_mock() {
    let t0 = make_ticket(MockMarker::GithubIssueCreate);
    let t1 = make_ticket(MockMarker::AiClassifyTicket);
    let t2 = make_ticket(MockMarker::HttpGet);
    let t0_clone = make_ticket(MockMarker::GithubIssueCreate);

    // Equal tickets (all fields identical).
    assert_eq!(
        t0, t0_clone,
        "tickets with all identical fields must be equal"
    );

    // Different mock → not equal.
    assert_ne!(
        t0, t1,
        "Tickets with different mock values must not be equal"
    );
    assert_ne!(
        t1, t2,
        "Tickets with different mock values must not be equal"
    );
    assert_ne!(
        t0, t2,
        "Tickets with different mock values must not be equal"
    );
}

#[test]
fn test_action_ticket_copy_preserved_with_mock() {
    let ticket = make_ticket(MockMarker::GithubIssueCreate);

    // Using ticket after "move" proves Copy.
    let _copied = ticket;
    assert_eq!(
        ticket.mock,
        MockMarker::GithubIssueCreate,
        "ticket must be usable after Copy — mock must be accessible"
    );
}

#[test]
fn test_action_ticket_clone_preserved_with_mock() {
    let ticket = make_ticket(MockMarker::HttpGet);
    let cloned = ticket.clone();

    assert_eq!(cloned.mock, MockMarker::HttpGet, "cloned mock must match");
    assert_eq!(cloned.run, ticket.run, "cloned run must match");
    assert_eq!(cloned.step, ticket.step, "cloned step must match");
    assert_eq!(cloned.seq, ticket.seq, "cloned seq must match");
    assert_eq!(cloned.action, ticket.action, "cloned action must match");
    assert_eq!(cloned.attempt, ticket.attempt, "cloned attempt must match");
    assert_eq!(
        cloned.idempotency_key, ticket.idempotency_key,
        "cloned idempotency_key must match"
    );
    assert_eq!(
        cloned.capacity, ticket.capacity,
        "cloned capacity must match"
    );
}
