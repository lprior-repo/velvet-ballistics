//! ActionTicket hash includes mock field.
//!
//! Verifies that hash differs when only mock differs, and that hash
//! is stable across invocations.
//! Gated behind `vb-rxru0-mock-marker` feature.
//!
//! Obligations: RFO-007, RFO-017
//! Contract clauses: C-TICKET-2 (hash includes mock)

#![forbid(unsafe_code)]
#![cfg(feature = "vb-rxru0-mock-marker")]

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

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

fn hash_of(ticket: &vb_core::action::ActionTicket) -> u64 {
    let mut h = DefaultHasher::new();
    ticket.hash(&mut h);
    h.finish()
}

#[test]
fn test_action_ticket_hash_includes_mock() {
    let t0 = make_ticket(MockMarker::GithubIssueCreate);
    let t1 = make_ticket(MockMarker::AiClassifyTicket);
    let t2 = make_ticket(MockMarker::HttpGet);
    let t0h = hash_of(&t0);
    let t1h = hash_of(&t1);
    let t2h = hash_of(&t2);

    assert_ne!(
        t0h, t1h,
        "Tickets differing only in mock must hash differently (GithubIssueCreate vs AiClassifyTicket)"
    );
    assert_ne!(
        t1h, t2h,
        "Tickets differing only in mock must hash differently (AiClassifyTicket vs HttpGet)"
    );
    assert_ne!(
        t0h, t2h,
        "Tickets differing only in mock must hash differently (GithubIssueCreate vs HttpGet)"
    );
}

#[test]
fn test_action_ticket_hash_consistency_with_mock() {
    let t1 = make_ticket(MockMarker::GithubIssueCreate);
    let t2 = make_ticket(MockMarker::GithubIssueCreate);

    assert_eq!(t1, t2, "tickets must be equal for hash consistency test");
    assert_eq!(
        hash_of(&t1),
        hash_of(&t2),
        "equal tickets must produce equal hashes"
    );
}

#[test]
fn test_action_ticket_hash_stability_with_mock() {
    let t = make_ticket(MockMarker::AiClassifyTicket);

    let h1 = hash_of(&t);
    let h2 = hash_of(&t);
    let h3 = hash_of(&t);

    assert_eq!(h1, h2, "hash must be stable across invocations (1 vs 2)");
    assert_eq!(h2, h3, "hash must be stable across invocations (2 vs 3)");
}
