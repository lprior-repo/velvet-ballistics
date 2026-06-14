//! Properties of `ActionTicket` that hold regardless of feature flags.
//!
//! Covers: Copy trait, equality across all 7 fields, hash stability,
//! and the hash inclusion of all fields.

#![forbid(unsafe_code)]

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use vb_core::action::ActionTicket;
use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};

// ---------------------------------------------------------------------------
// Copy trait tests
// ---------------------------------------------------------------------------

#[test]
fn test_action_ticket_copy_trait() {
    let ticket = ActionTicket {
        run: RunId::new(42),
        step: StepIdx::new(7),
        seq: SeqNo::new(99),
        action: ActionId::new(5),
        attempt: 3,
        idempotency_key: 0xDEADBEEF,
        capacity: 10,
        ..Default::default()
    };

    // Copy: ticket should be usable after being "moved".
    let _copied = ticket;
    assert_eq!(ticket.run.get(), 42, "ticket must remain usable after Copy");
    assert_eq!(ticket.step.get(), 7, "ticket must remain usable after Copy");
    assert_eq!(ticket.seq.get(), 99, "ticket must remain usable after Copy");
    assert_eq!(
        ticket.action.get(),
        5,
        "ticket must remain usable after Copy"
    );
    assert_eq!(ticket.attempt, 3, "ticket must remain usable after Copy");
    assert_eq!(
        ticket.idempotency_key, 0xDEADBEEF,
        "ticket must remain usable after Copy"
    );
    assert_eq!(ticket.capacity, 10, "ticket must remain usable after Copy");
}

#[test]
fn test_action_ticket_clone_trait() {
    let ticket = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(2),
        seq: SeqNo::new(3),
        action: ActionId::new(4),
        attempt: 5,
        idempotency_key: 6,
        capacity: 7,
        ..Default::default()
    };

    // Clone should work and produce an identical ticket.
    let cloned = ticket.clone();
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

// ---------------------------------------------------------------------------
// Equality tests
// ---------------------------------------------------------------------------

#[test]
fn test_action_ticket_equality_all_fields_equal() {
    let t1 = ActionTicket {
        run: RunId::new(100),
        step: StepIdx::new(200),
        seq: SeqNo::new(300),
        action: ActionId::new(400),
        attempt: 10,
        idempotency_key: 0x1234_5678_9ABC_DEF0,
        capacity: 42,
        ..Default::default()
    };
    let t2 = ActionTicket {
        run: RunId::new(100),
        step: StepIdx::new(200),
        seq: SeqNo::new(300),
        action: ActionId::new(400),
        attempt: 10,
        idempotency_key: 0x1234_5678_9ABC_DEF0,
        capacity: 42,
        ..Default::default()
    };

    assert_eq!(t1, t2, "tickets with identical fields must be equal");
}

#[test]
fn test_action_ticket_equality_run_differs() {
    let t1 = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(2),
        seq: SeqNo::new(3),
        action: ActionId::new(4),
        attempt: 5,
        idempotency_key: 6,
        capacity: 7,
        ..Default::default()
    };
    let t2 = ActionTicket {
        run: RunId::new(2),
        step: StepIdx::new(2),
        seq: SeqNo::new(3),
        action: ActionId::new(4),
        attempt: 5,
        idempotency_key: 6,
        capacity: 7,
        ..Default::default()
    };

    assert_ne!(t1, t2, "tickets with different run must not be equal");
}

#[test]
fn test_action_ticket_equality_step_differs() {
    let t1 = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(2),
        seq: SeqNo::new(3),
        action: ActionId::new(4),
        attempt: 5,
        idempotency_key: 6,
        capacity: 7,
        ..Default::default()
    };
    let t2 = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(3),
        seq: SeqNo::new(3),
        action: ActionId::new(4),
        attempt: 5,
        idempotency_key: 6,
        capacity: 7,
        ..Default::default()
    };

    assert_ne!(t1, t2, "tickets with different step must not be equal");
}

#[test]
fn test_action_ticket_equality_seq_differs() {
    let t1 = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(2),
        seq: SeqNo::new(3),
        action: ActionId::new(4),
        attempt: 5,
        idempotency_key: 6,
        capacity: 7,
        ..Default::default()
    };
    let t2 = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(2),
        seq: SeqNo::new(4),
        action: ActionId::new(4),
        attempt: 5,
        idempotency_key: 6,
        capacity: 7,
        ..Default::default()
    };

    assert_ne!(t1, t2, "tickets with different seq must not be equal");
}

#[test]
fn test_action_ticket_equality_action_differs() {
    let t1 = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(2),
        seq: SeqNo::new(3),
        action: ActionId::new(4),
        attempt: 5,
        idempotency_key: 6,
        capacity: 7,
        ..Default::default()
    };
    let t2 = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(2),
        seq: SeqNo::new(3),
        action: ActionId::new(5),
        attempt: 5,
        idempotency_key: 6,
        capacity: 7,
        ..Default::default()
    };

    assert_ne!(t1, t2, "tickets with different action must not be equal");
}

#[test]
fn test_action_ticket_equality_attempt_differs() {
    let t1 = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(2),
        seq: SeqNo::new(3),
        action: ActionId::new(4),
        attempt: 5,
        idempotency_key: 6,
        capacity: 7,
        ..Default::default()
    };
    let t2 = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(2),
        seq: SeqNo::new(3),
        action: ActionId::new(4),
        attempt: 6,
        idempotency_key: 6,
        capacity: 7,
        ..Default::default()
    };

    assert_ne!(t1, t2, "tickets with different attempt must not be equal");
}

#[test]
fn test_action_ticket_equality_idempotency_key_differs() {
    let t1 = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(2),
        seq: SeqNo::new(3),
        action: ActionId::new(4),
        attempt: 5,
        idempotency_key: 0xBEEF,
        capacity: 7,
        ..Default::default()
    };
    let t2 = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(2),
        seq: SeqNo::new(3),
        action: ActionId::new(4),
        attempt: 5,
        idempotency_key: 0xDEAD,
        capacity: 7,
        ..Default::default()
    };

    assert_ne!(
        t1, t2,
        "tickets with different idempotency_key must not be equal"
    );
}

#[test]
fn test_action_ticket_equality_capacity_differs() {
    let t1 = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(2),
        seq: SeqNo::new(3),
        action: ActionId::new(4),
        attempt: 5,
        idempotency_key: 6,
        capacity: 7,
        ..Default::default()
    };
    let t2 = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(2),
        seq: SeqNo::new(3),
        action: ActionId::new(4),
        attempt: 5,
        idempotency_key: 6,
        capacity: 8,
        ..Default::default()
    };

    assert_ne!(t1, t2, "tickets with different capacity must not be equal");
}

#[test]
fn test_action_ticket_equality_zero_values() {
    let t1 = ActionTicket {
        run: RunId::new(0),
        step: StepIdx::new(0),
        seq: SeqNo::new(0),
        action: ActionId::new(0),
        attempt: 0,
        idempotency_key: 0,
        capacity: 0,
        ..Default::default()
    };
    let t2 = ActionTicket {
        run: RunId::new(0),
        step: StepIdx::new(0),
        seq: SeqNo::new(0),
        action: ActionId::new(0),
        attempt: 0,
        idempotency_key: 0,
        capacity: 0,
        ..Default::default()
    };

    assert_eq!(t1, t2, "tickets with all-zero fields must be equal");
}

// ---------------------------------------------------------------------------
// Hash tests
// ---------------------------------------------------------------------------

fn hash_of(ticket: &ActionTicket) -> u64 {
    let mut hasher = DefaultHasher::new();
    ticket.hash(&mut hasher);
    hasher.finish()
}

#[test]
fn test_action_ticket_hash_consistency() {
    let t1 = ActionTicket {
        run: RunId::new(42),
        step: StepIdx::new(7),
        seq: SeqNo::new(99),
        action: ActionId::new(5),
        attempt: 3,
        idempotency_key: 0xDEADBEEF,
        capacity: 10,
        ..Default::default()
    };
    let t2 = ActionTicket {
        run: RunId::new(42),
        step: StepIdx::new(7),
        seq: SeqNo::new(99),
        action: ActionId::new(5),
        attempt: 3,
        idempotency_key: 0xDEADBEEF,
        capacity: 10,
        ..Default::default()
    };

    assert_eq!(t1, t2, "tickets must be equal for hash consistency test");
    assert_eq!(
        hash_of(&t1),
        hash_of(&t2),
        "equal tickets must produce equal hashes"
    );
}

#[test]
fn test_action_ticket_hash_stability() {
    let t1 = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(2),
        seq: SeqNo::new(3),
        action: ActionId::new(4),
        attempt: 5,
        idempotency_key: 6,
        capacity: 7,
        ..Default::default()
    };

    let h1 = hash_of(&t1);
    let h2 = hash_of(&t1);
    let h3 = hash_of(&t1);

    assert_eq!(h1, h2, "hash must be stable across invocations (1 vs 2)");
    assert_eq!(h2, h3, "hash must be stable across invocations (2 vs 3)");
}

#[test]
fn test_action_ticket_hash_differs_when_run_differs() {
    let t1 = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(2),
        seq: SeqNo::new(3),
        action: ActionId::new(4),
        attempt: 5,
        idempotency_key: 6,
        capacity: 7,
        ..Default::default()
    };
    let t2 = ActionTicket {
        run: RunId::new(999),
        step: StepIdx::new(2),
        seq: SeqNo::new(3),
        action: ActionId::new(4),
        attempt: 5,
        idempotency_key: 6,
        capacity: 7,
        ..Default::default()
    };

    assert_ne!(
        hash_of(&t1),
        hash_of(&t2),
        "tickets differing only in run must hash differently"
    );
}

#[test]
fn test_action_ticket_hash_differs_when_step_differs() {
    let t1 = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(2),
        seq: SeqNo::new(3),
        action: ActionId::new(4),
        attempt: 5,
        idempotency_key: 6,
        capacity: 7,
        ..Default::default()
    };
    let t2 = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(999),
        seq: SeqNo::new(3),
        action: ActionId::new(4),
        attempt: 5,
        idempotency_key: 6,
        capacity: 7,
        ..Default::default()
    };

    assert_ne!(
        hash_of(&t1),
        hash_of(&t2),
        "tickets differing only in step must hash differently"
    );
}

#[test]
fn test_action_ticket_hash_differs_when_seq_differs() {
    let t1 = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(2),
        seq: SeqNo::new(3),
        action: ActionId::new(4),
        attempt: 5,
        idempotency_key: 6,
        capacity: 7,
        ..Default::default()
    };
    let t2 = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(2),
        seq: SeqNo::new(999),
        action: ActionId::new(4),
        attempt: 5,
        idempotency_key: 6,
        capacity: 7,
        ..Default::default()
    };

    assert_ne!(
        hash_of(&t1),
        hash_of(&t2),
        "tickets differing only in seq must hash differently"
    );
}

#[test]
fn test_action_ticket_hash_differs_when_action_differs() {
    let t1 = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(2),
        seq: SeqNo::new(3),
        action: ActionId::new(4),
        attempt: 5,
        idempotency_key: 6,
        capacity: 7,
        ..Default::default()
    };
    let t2 = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(2),
        seq: SeqNo::new(3),
        action: ActionId::new(999),
        attempt: 5,
        idempotency_key: 6,
        capacity: 7,
        ..Default::default()
    };

    assert_ne!(
        hash_of(&t1),
        hash_of(&t2),
        "tickets differing only in action must hash differently"
    );
}

#[test]
fn test_action_ticket_hash_differs_when_attempt_differs() {
    let t1 = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(2),
        seq: SeqNo::new(3),
        action: ActionId::new(4),
        attempt: 5,
        idempotency_key: 6,
        capacity: 7,
        ..Default::default()
    };
    let t2 = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(2),
        seq: SeqNo::new(3),
        action: ActionId::new(4),
        attempt: 999,
        idempotency_key: 6,
        capacity: 7,
        ..Default::default()
    };

    assert_ne!(
        hash_of(&t1),
        hash_of(&t2),
        "tickets differing only in attempt must hash differently"
    );
}

#[test]
fn test_action_ticket_hash_differs_when_idempotency_key_differs() {
    let t1 = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(2),
        seq: SeqNo::new(3),
        action: ActionId::new(4),
        attempt: 5,
        idempotency_key: 0x1111_1111_1111_1111,
        capacity: 7,
        ..Default::default()
    };
    let t2 = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(2),
        seq: SeqNo::new(3),
        action: ActionId::new(4),
        attempt: 5,
        idempotency_key: 0x2222_2222_2222_2222,
        capacity: 7,
        ..Default::default()
    };

    assert_ne!(
        hash_of(&t1),
        hash_of(&t2),
        "tickets differing only in idempotency_key must hash differently"
    );
}

#[test]
fn test_action_ticket_hash_differs_when_capacity_differs() {
    let t1 = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(2),
        seq: SeqNo::new(3),
        action: ActionId::new(4),
        attempt: 5,
        idempotency_key: 6,
        capacity: 7,
        ..Default::default()
    };
    let t2 = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(2),
        seq: SeqNo::new(3),
        action: ActionId::new(4),
        attempt: 5,
        idempotency_key: 6,
        capacity: 999,
        ..Default::default()
    };

    assert_ne!(
        hash_of(&t1),
        hash_of(&t2),
        "tickets differing only in capacity must hash differently"
    );
}
