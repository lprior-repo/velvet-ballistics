#![allow(clippy::expect_used, clippy::unwrap_used, clippy::as_conversions, clippy::panic)]
//! Postcard serialization roundtrip for `ActionTicket`.
//!
//! Verifies that all 7 fields are preserved through serialization and
//! deserialization.

#![forbid(unsafe_code)]

use vb_core::action::ActionTicket;
use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};

#[test]
fn test_action_ticket_postcard_roundtrip_all_fields() {
    let ticket = ActionTicket {
        run: RunId::new(0x0102_0304_0506_0708),
        step: StepIdx::new(0x1011),
        seq: SeqNo::new(0x2021_2223_2425_2627),
        action: ActionId::new(0x3031),
        attempt: 42,
        idempotency_key: 0xAABB_CCDD_0011_2233_4455_6677_8899_AABB,
        capacity: 99,
        ..Default::default()
    };

    let buf = postcard::to_allocvec(&ticket).expect("ActionTicket serialization must succeed");
    let ticket2: ActionTicket =
        postcard::from_bytes(&buf).expect("ActionTicket deserialization must succeed");

    assert_eq!(
        ticket2.run.get(),
        ticket.run.get(),
        "run must be preserved through serialization"
    );
    assert_eq!(
        ticket2.step.get(),
        ticket.step.get(),
        "step must be preserved through serialization"
    );
    assert_eq!(
        ticket2.seq.get(),
        ticket.seq.get(),
        "seq must be preserved through serialization"
    );
    assert_eq!(
        ticket2.action.get(),
        ticket.action.get(),
        "action must be preserved through serialization"
    );
    assert_eq!(
        ticket2.attempt, ticket.attempt,
        "attempt must be preserved through serialization"
    );
    assert_eq!(
        ticket2.idempotency_key, ticket.idempotency_key,
        "idempotency_key must be preserved through serialization"
    );
    assert_eq!(
        ticket2.capacity, ticket.capacity,
        "capacity must be preserved through serialization"
    );
}

#[test]
fn test_action_ticket_postcard_roundtrip_zero_values() {
    let ticket = ActionTicket {
        run: RunId::new(0),
        step: StepIdx::new(0),
        seq: SeqNo::new(0),
        action: ActionId::new(0),
        attempt: 0,
        idempotency_key: 0,
        capacity: 0,
        ..Default::default()
    };

    let buf = postcard::to_allocvec(&ticket).expect("ActionTicket serialization must succeed");
    let ticket2: ActionTicket =
        postcard::from_bytes(&buf).expect("ActionTicket deserialization must succeed");

    assert_eq!(
        ticket2, ticket,
        "zero-value ticket must roundtrip identically"
    );
}

#[test]
fn test_action_ticket_postcard_roundtrip_max_values() {
    let ticket = ActionTicket {
        run: RunId::new(u64::MAX),
        step: StepIdx::new(u16::MAX),
        seq: SeqNo::new(u64::MAX),
        action: ActionId::new(u16::MAX),
        attempt: u16::MAX,
        idempotency_key: u128::MAX,
        capacity: u16::MAX,
        ..Default::default()
    };

    let buf = postcard::to_allocvec(&ticket).expect("ActionTicket serialization must succeed");
    let ticket2: ActionTicket =
        postcard::from_bytes(&buf).expect("ActionTicket deserialization must succeed");

    assert_eq!(
        ticket2.run.get(),
        u64::MAX,
        "max run must be preserved through serialization"
    );
    assert_eq!(
        ticket2.step.get(),
        u16::MAX,
        "max step must be preserved through serialization"
    );
    assert_eq!(
        ticket2.seq.get(),
        u64::MAX,
        "max seq must be preserved through serialization"
    );
    assert_eq!(
        ticket2.action.get(),
        u16::MAX,
        "max action must be preserved through serialization"
    );
    assert_eq!(
        ticket2.attempt,
        u16::MAX,
        "max attempt must be preserved through serialization"
    );
    assert_eq!(
        ticket2.idempotency_key,
        u128::MAX,
        "max idempotency_key must be preserved through serialization"
    );
    assert_eq!(
        ticket2.capacity,
        u16::MAX,
        "max capacity must be preserved through serialization"
    );
}

#[test]
fn test_action_ticket_postcard_roundtrip_determinism() {
    // Two tickets with different values serialize to different sizes (postcard
    // uses varint), but both must roundtrip correctly and produce deterministic
    // output for the same input.
    let small = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(2),
        seq: SeqNo::new(3),
        action: ActionId::new(4),
        attempt: 5,
        idempotency_key: 6,
        capacity: 7,
        ..Default::default()
    };
    let large = ActionTicket {
        run: RunId::new(u64::MAX),
        step: StepIdx::new(u16::MAX),
        seq: SeqNo::new(u64::MAX),
        action: ActionId::new(u16::MAX),
        attempt: u16::MAX,
        idempotency_key: u128::MAX,
        capacity: u16::MAX,
        ..Default::default()
    };

    let buf_small = postcard::to_allocvec(&small).expect("small ticket serialization must succeed");
    let buf_large = postcard::to_allocvec(&large).expect("large ticket serialization must succeed");

    // Postcard uses variable-length encoding; small values serialize smaller.
    assert!(
        buf_small.len() <= buf_large.len(),
        "small ticket must serialize to <= bytes as large ticket"
    );
    assert!(
        buf_small.len() > 0,
        "serialization must produce non-empty output"
    );

    // Both must roundtrip correctly.
    let restored_small: ActionTicket =
        postcard::from_bytes(&buf_small).expect("small roundtrip must succeed");
    let restored_large: ActionTicket =
        postcard::from_bytes(&buf_large).expect("large roundtrip must succeed");

    assert_eq!(
        restored_small, small,
        "small ticket must roundtrip faithfully"
    );
    assert_eq!(
        restored_large, large,
        "large ticket must roundtrip faithfully"
    );

    // Determinism: same input always produces same output.
    let buf_small_again =
        postcard::to_allocvec(&small).expect("determinism serialization must succeed");
    assert_eq!(
        buf_small, buf_small_again,
        "serialization must be deterministic (same input → same bytes)"
    );
}

#[test]
fn test_action_ticket_postcard_roundtrip_mixed_values() {
    // Test a variety of mixed values to catch encoding edge cases.
    let ticket = ActionTicket {
        run: RunId::new(0xDEAD_BEEF_CAFE_BABE),
        step: StepIdx::new(0x1234),
        seq: SeqNo::new(0x0000_0000_FFFF_FFFF),
        action: ActionId::new(0x00FF),
        attempt: 1,
        idempotency_key: 0x0000_0000_0000_0000_DEAD_BEEF_DEAD_BEEF,
        capacity: 1,
        ..Default::default()
    };

    let buf = postcard::to_allocvec(&ticket).expect("ActionTicket serialization must succeed");
    let ticket2: ActionTicket =
        postcard::from_bytes(&buf).expect("ActionTicket deserialization must succeed");

    assert_eq!(ticket2.run.get(), ticket.run.get(), "run must match");
    assert_eq!(ticket2.step.get(), ticket.step.get(), "step must match");
    assert_eq!(ticket2.seq.get(), ticket.seq.get(), "seq must match");
    assert_eq!(
        ticket2.action.get(),
        ticket.action.get(),
        "action must match"
    );
    assert_eq!(ticket2.attempt, ticket.attempt, "attempt must match");
    assert_eq!(
        ticket2.idempotency_key, ticket.idempotency_key,
        "idempotency_key must match"
    );
    assert_eq!(ticket2.capacity, ticket.capacity, "capacity must match");
}
