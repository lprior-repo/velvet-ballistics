use super::*;

// ─────────────────────────────────────────────────────────────────
// PO-R1: Variable (accumulator) field hashing
// ─────────────────────────────────────────────────────────────────
#[test]
fn direct_digest_reduce_variable_field() {
    let reduce_a = make_reduce("acc", "items", "0", vec![]);
    let reduce_b = make_reduce("result", "items", "0", vec![]);

    let digest_a = digest_primitive(&reduce_a);
    let digest_b = digest_primitive(&reduce_b);

    assert_ne!(
        digest_a, digest_b,
        "different variable (accumulator) fields must produce different digests via digest_step_primitive"
    );
}

// ─────────────────────────────────────────────────────────────────
// PO-R2: Input collection field hashing
// ─────────────────────────────────────────────────────────────────
#[test]
fn direct_digest_reduce_input_field() {
    let reduce_a = make_reduce("acc", "items_a", "0", vec![]);
    let reduce_b = make_reduce("acc", "items_b", "0", vec![]);

    let digest_a = digest_primitive(&reduce_a);
    let digest_b = digest_primitive(&reduce_b);

    assert_ne!(
        digest_a, digest_b,
        "different input collection fields must produce different digests via digest_step_primitive"
    );
}

// ─────────────────────────────────────────────────────────────────
// PO-R3: Initial accumulator value hashing
// ─────────────────────────────────────────────────────────────────
#[test]
fn direct_digest_reduce_initial_field() {
    let reduce_a = make_reduce("acc", "items", "0", vec![]);
    let reduce_b = make_reduce("acc", "items", "1", vec![]);

    let digest_a = digest_primitive(&reduce_a);
    let digest_b = digest_primitive(&reduce_b);

    assert_ne!(
        digest_a, digest_b,
        "different initial accumulator values must produce different digests via digest_step_primitive"
    );
}

// ─────────────────────────────────────────────────────────────────
// PO-R4: Body steps hashing
// ─────────────────────────────────────────────────────────────────
#[test]
fn direct_digest_reduce_body_steps() {
    let body_a = vec![make_set_step("step_a", "x", "1")];
    let body_b = vec![make_set_step("step_b", "y", "2")];

    let reduce_a = make_reduce("acc", "items", "0", body_a);
    let reduce_b = make_reduce("acc", "items", "0", body_b);

    let digest_a = digest_primitive(&reduce_a);
    let digest_b = digest_primitive(&reduce_b);

    assert_ne!(
        digest_a, digest_b,
        "different body steps must produce different digests via digest_step_primitive"
    );
}

// ─────────────────────────────────────────────────────────────────
// PO-R5: Empty body vs non-empty body
// ─────────────────────────────────────────────────────────────────
#[test]
fn direct_digest_reduce_empty_vs_nonempty_body() {
    let body = vec![make_set_step("inner", "x", "1")];

    let reduce_empty = make_reduce("acc", "items", "0", vec![]);
    let reduce_nonempty = make_reduce("acc", "items", "0", body);

    let digest_empty = digest_primitive(&reduce_empty);
    let digest_nonempty = digest_primitive(&reduce_nonempty);

    assert_ne!(
        digest_empty, digest_nonempty,
        "empty body vs non-empty body must produce different digests via digest_step_primitive"
    );
}

// ─────────────────────────────────────────────────────────────────
// PO-R6: Idempotence — same input produces same digest
// ─────────────────────────────────────────────────────────────────
#[test]
fn direct_digest_reduce_idempotence() {
    let reduce = make_reduce("acc", "items", "0", vec![]);

    let digest_a = digest_primitive(&reduce);
    let digest_b = digest_primitive(&reduce);

    assert_eq!(
        digest_a, digest_b,
        "same StepPrimitive::Aggregate must produce same digest (idempotence)"
    );
}

// ─────────────────────────────────────────────────────────────────
// PO-R7: Determinism — repeated calls produce same digest
// ─────────────────────────────────────────────────────────────────
#[test]
fn direct_digest_reduce_repeated_calls_same_digest() {
    let reduce = make_reduce("acc", "items", "0", vec![]);

    let d1 = digest_primitive(&reduce);
    let d2 = digest_primitive(&reduce);
    let d3 = digest_primitive(&reduce);

    assert_eq!(d1, d2, "first and second call must match");
    assert_eq!(d2, d3, "second and third call must match");
}

// ─────────────────────────────────────────────────────────────────
// PO-R8: Collect vs Aggregate cross-variant collision resistance
// ─────────────────────────────────────────────────────────────────
#[test]
fn direct_digest_collect_vs_aggregate_different_sentinels() {
    let collect = StepPrimitive::Collect {
        variable: "x".to_string(),
        source: "items".to_string(),
        pages: None,
        items: None,
        body: vec![],
    };
    let aggregate = StepPrimitive::Aggregate {
        variable: "x".to_string(),
        input: "items".to_string(),
        initial: "0".to_string(),
        body: vec![],
    };

    let digest_collect = digest_primitive(&collect);
    let digest_aggregate = digest_primitive(&aggregate);

    assert_ne!(
        digest_collect, digest_aggregate,
        "Collect and Aggregate with similar fields must produce different digests (different sentinels)"
    );
}
