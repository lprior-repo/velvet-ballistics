//! Properties of `compute_action_idempotency_key`.
//!
//! Verifies determinism, boundary value safety, non-zero output, and
//! the relationship with `action_ticket_has_valid_key`.

#![forbid(unsafe_code)]

use vb_core::action::{action_ticket_has_valid_key, compute_action_idempotency_key, issue_action_ticket};
use vb_core::ids::{ActionId, RunId, SeqNo};

// ---------------------------------------------------------------------------
// Determinism
// ---------------------------------------------------------------------------

#[test]
fn test_compute_key_deterministic() {
    let run = RunId::new(100);
    let seq = SeqNo::new(200);
    let action = ActionId::new(300);

    let key1 = compute_action_idempotency_key(run, seq, action);
    let key2 = compute_action_idempotency_key(run, seq, action);

    assert_eq!(key1, key2, "same inputs must produce the same key");
}

#[test]
fn test_compute_key_deterministic_across_many_calls() {
    let run = RunId::new(42);
    let seq = SeqNo::new(84);
    let action = ActionId::new(168);

    let first = compute_action_idempotency_key(run, seq, action);
    let mut all_match = true;
    for _ in 0..1000 {
        let next = compute_action_idempotency_key(run, seq, action);
        if next != first {
            all_match = false;
            break;
        }
    }
    assert!(
        all_match,
        "compute_action_idempotency_key must be deterministic across 1000 calls"
    );
}

// ---------------------------------------------------------------------------
// Boundary values
// ---------------------------------------------------------------------------

#[test]
fn test_compute_key_boundary_run_zero() {
    let key = compute_action_idempotency_key(RunId::new(0), SeqNo::new(1), ActionId::new(1));
    // Should not panic and should produce some value.
    let _ = key;
}

#[test]
fn test_compute_key_boundary_seq_zero() {
    let key = compute_action_idempotency_key(RunId::new(1), SeqNo::new(0), ActionId::new(1));
    let _ = key;
}

#[test]
fn test_compute_key_boundary_action_zero() {
    let key = compute_action_idempotency_key(RunId::new(1), SeqNo::new(1), ActionId::new(0));
    let _ = key;
}

#[test]
fn test_compute_key_boundary_all_zero() {
    let key = compute_action_idempotency_key(RunId::new(0), SeqNo::new(0), ActionId::new(0));
    let _ = key;
}

#[test]
fn test_compute_key_boundary_run_max() {
    let key = compute_action_idempotency_key(
        RunId::new(u64::MAX),
        SeqNo::new(1),
        ActionId::new(1),
    );
    let _ = key;
}

#[test]
fn test_compute_key_boundary_seq_max() {
    let key = compute_action_idempotency_key(
        RunId::new(1),
        SeqNo::new(u64::MAX),
        ActionId::new(1),
    );
    let _ = key;
}

#[test]
fn test_compute_key_boundary_action_max() {
    let key = compute_action_idempotency_key(
        RunId::new(1),
        SeqNo::new(1),
        ActionId::new(u16::MAX),
    );
    let _ = key;
}

#[test]
fn test_compute_key_boundary_all_max() {
    let key = compute_action_idempotency_key(
        RunId::new(u64::MAX),
        SeqNo::new(u64::MAX),
        ActionId::new(u16::MAX),
    );
    let _ = key;
}

// ---------------------------------------------------------------------------
// Non-zero output
// ---------------------------------------------------------------------------

#[test]
fn test_compute_key_non_trivial_produces_nonzero() {
    let key = compute_action_idempotency_key(RunId::new(1), SeqNo::new(1), ActionId::new(1));
    assert_ne!(key, 0, "non-trivial key must be non-zero");
}

#[test]
fn test_compute_key_trivial_all_zero_is_zero() {
    let key = compute_action_idempotency_key(RunId::new(0), SeqNo::new(0), ActionId::new(0));
    assert_eq!(key, 0, "all-zero inputs should produce zero key");
}

// ---------------------------------------------------------------------------
// Valid key relationship
// ---------------------------------------------------------------------------

#[test]
fn test_ticket_valid_key_true_when_canonical() {
    let run = RunId::new(42);
    let seq = SeqNo::new(100);
    let action = ActionId::new(7);
    let canonical_key = compute_action_idempotency_key(run, seq, action);

    let ticket = issue_action_ticket(
        run,
        vb_core::ids::StepIdx::new(0),
        seq,
        action,
        1,
        canonical_key,
        1,
    );

    assert!(
        action_ticket_has_valid_key(ticket),
        "ticket with canonical key must have valid key"
    );
}

#[test]
fn test_ticket_valid_key_false_when_wrong() {
    let ticket = issue_action_ticket(
        RunId::new(42),
        vb_core::ids::StepIdx::new(0),
        SeqNo::new(100),
        ActionId::new(7),
        1,
        0xDEAD_BEEF, // wrong key
        1,
    );

    assert!(
        !action_ticket_has_valid_key(ticket),
        "ticket with wrong key must not have valid key"
    );
}

#[test]
fn test_ticket_valid_key_false_when_zero() {
    let ticket = issue_action_ticket(
        RunId::new(42),
        vb_core::ids::StepIdx::new(0),
        SeqNo::new(100),
        ActionId::new(7),
        1,
        0, // zero key
        1,
    );

    assert!(
        !action_ticket_has_valid_key(ticket),
        "ticket with zero key must not have valid key (for non-trivial run/seq/action)"
    );
}

#[test]
fn test_compute_key_different_inputs_produce_different_keys() {
    let key_a = compute_action_idempotency_key(RunId::new(1), SeqNo::new(1), ActionId::new(1));
    let key_b = compute_action_idempotency_key(RunId::new(2), SeqNo::new(1), ActionId::new(1));

    assert_ne!(
        key_a,
        key_b,
        "different run values must produce different keys"
    );
}

#[test]
fn test_compute_key_different_seq_produces_different_key() {
    let key_a = compute_action_idempotency_key(RunId::new(1), SeqNo::new(1), ActionId::new(1));
    let key_b = compute_action_idempotency_key(RunId::new(1), SeqNo::new(2), ActionId::new(1));

    assert_ne!(
        key_a,
        key_b,
        "different seq values must produce different keys"
    );
}

#[test]
fn test_compute_key_different_action_produces_different_key() {
    let key_a = compute_action_idempotency_key(RunId::new(1), SeqNo::new(1), ActionId::new(1));
    let key_b = compute_action_idempotency_key(RunId::new(1), SeqNo::new(1), ActionId::new(2));

    assert_ne!(
        key_a,
        key_b,
        "different action values must produce different keys"
    );
}
