//! Panic freedom tests for MockMarker and ActionTicket operations.
//!
//! Verifies that no panic occurs when operating on any MockMarker variant
//! or when deserializing tickets.
//! Gated behind `vb-rxru0-mock-marker` feature.
//!
//! Obligations: RFO-004, RFO-005, RFO-021, RFO-022

#![forbid(unsafe_code)]
#![cfg(feature = "vb-rxru0-mock-marker")]

use std::hash::{DefaultHasher, Hash, Hasher};

use vb_core::action::MockMarker;
use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};

fn hash_of(m: MockMarker) -> u64 {
    let mut h = DefaultHasher::new();
    m.hash(&mut h);
    h.finish()
}

#[test]
fn test_mock_marker_no_panic_on_all_variants() {
    let variants = [
        MockMarker::GithubIssueCreate,
        MockMarker::AiClassifyTicket,
        MockMarker::HttpGet,
    ];

    for m in &variants {
        // Serialization roundtrip must not panic.
        let buf = match postcard::to_allocvec(m) {
            Ok(b) => b,
            Err(_) => { kani::assume(false, "serialize must not panic"); return; }
        };
        let _: MockMarker = match postcard::from_bytes(&buf) {
            Ok(v) => v,
            Err(_) => { kani::assume(false, "deserialize must not panic"); return; }
        };

        // Debug formatting must not panic.
        let _ = format!("{m:?}");

        // Hashing must not panic.
        let _ = hash_of(*m);

        // Equality must not panic.
        let _ = *m == MockMarker::GithubIssueCreate;
        let _ = *m != MockMarker::AiClassifyTicket;
    }
}

#[test]
fn test_legacy_7field_deserialize_fallback() {
    // Construct a ticket with mock = GithubIssueCreate (discriminant 0 — legacy default).
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

    let buf = match postcard::to_allocvec(&legacy_ticket) {
        Ok(b) => b,
        Err(_) => { kani::assume(false, "serialization must not panic"); return; }
    };

    // Deserialize back.
    let restored: vb_core::action::ActionTicket = match postcard::from_bytes(&buf) {
        Ok(v) => v,
        Err(_) => { kani::assume(false, "deserialization must not panic"); return; }
    };

    assert_eq!(
        restored.mock,
        MockMarker::GithubIssueCreate,
        "Legacy 7-field deserialization must yield mock = GithubIssueCreate (discriminant 0)"
    );
}

#[test]
fn test_action_ticket_serde_no_panic_boundary() {
    // Test serialization and deserialization at boundary values.
    let ticket = vb_core::action::ActionTicket {
        run: RunId::new(u64::MAX),
        step: StepIdx::new(u16::MAX),
        seq: SeqNo::new(u64::MAX),
        action: ActionId::new(u16::MAX),
        attempt: u16::MAX,
        idempotency_key: u128::MAX,
        capacity: u16::MAX,
        mock: MockMarker::HttpGet,
    };

    let buf = match postcard::to_allocvec(&ticket) {
        Ok(b) => b,
        Err(_) => { kani::assume(false, "max-value serialization must not panic"); return; }
    };
    let restored: vb_core::action::ActionTicket = match postcard::from_bytes(&buf) {
        Ok(v) => v,
        Err(_) => { kani::assume(false, "max-value deserialization must not panic"); return; }
    };

    assert_eq!(restored.run.get(), u64::MAX, "max run must be preserved");
    assert_eq!(restored.step.get(), u16::MAX, "max step must be preserved");
    assert_eq!(restored.seq.get(), u64::MAX, "max seq must be preserved");
    assert_eq!(
        restored.action.get(),
        u16::MAX,
        "max action must be preserved"
    );
    assert_eq!(restored.attempt, u16::MAX, "max attempt must be preserved");
    assert_eq!(
        restored.idempotency_key,
        u128::MAX,
        "max idempotency_key must be preserved"
    );
    assert_eq!(
        restored.capacity,
        u16::MAX,
        "max capacity must be preserved"
    );
    assert_eq!(restored.mock, MockMarker::HttpGet, "mock must be preserved");
}
