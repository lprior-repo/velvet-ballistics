use super::*;

// ─────────────────────────────────────────────────────────────────
// Variable field
// ─────────────────────────────────────────────────────────────────
#[test]
fn direct_digest_collect_variable_field() {
    // Two Collect primitives identical except variable field differs.
    let collect_a = make_collect("x", "items", Some(10), Some(50), vec![]);
    let collect_b = make_collect("y", "items", Some(10), Some(50), vec![]);

    let digest_a = digest_primitive(&collect_a);
    let digest_b = digest_primitive(&collect_b);

    // Different variable fields MUST produce different digests
    assert_ne!(
        digest_a, digest_b,
        "different variable fields must produce different digests via digest_step_primitive"
    );
}

// ─────────────────────────────────────────────────────────────────
// Source field
// ─────────────────────────────────────────────────────────────────
#[test]
fn direct_digest_collect_source_field() {
    // Two Collect primitives identical except source field differs.
    let collect_a = make_collect("x", "items_a", Some(10), Some(50), vec![]);
    let collect_b = make_collect("x", "items_b", Some(10), Some(50), vec![]);

    let digest_a = digest_primitive(&collect_a);
    let digest_b = digest_primitive(&collect_b);

    // Different source fields MUST produce different digests
    assert_ne!(
        digest_a, digest_b,
        "different source fields must produce different digests via digest_step_primitive"
    );
}

// ─────────────────────────────────────────────────────────────────
// Pages field (Some different values)
// ─────────────────────────────────────────────────────────────────
#[test]
fn direct_digest_collect_pages_field() {
    // Two Collect primitives identical except pages field differs.
    let collect_a = make_collect("x", "items", Some(10), Some(50), vec![]);
    let collect_b = make_collect("x", "items", Some(999), Some(50), vec![]);

    let digest_a = digest_primitive(&collect_a);
    let digest_b = digest_primitive(&collect_b);

    // Different pages fields MUST produce different digests
    assert_ne!(
        digest_a, digest_b,
        "different pages fields must produce different digests via digest_step_primitive"
    );
}

// ─────────────────────────────────────────────────────────────────
// Pages field (None vs Some)
// ─────────────────────────────────────────────────────────────────
#[test]
fn direct_digest_collect_pages_none_equiv_some1() {
    // pages: None vs pages: Some(1)
    let collect_a = make_collect("x", "items", None, Some(50), vec![]);
    let collect_b = make_collect("x", "items", Some(1), Some(50), vec![]);

    let digest_a = digest_primitive(&collect_a);
    let digest_b = digest_primitive(&collect_b);

    // None and Some(1) both lower to `limit = pages.unwrap_or(1)` and must
    // share a digest (canonical equivalence).
    assert_eq!(
        digest_a, digest_b,
        "pages None and Some(1) lower to limit=1 and must share a digest"
    );
}

// ─────────────────────────────────────────────────────────────────
// Items field (Some different values)
// ─────────────────────────────────────────────────────────────────
#[test]
fn direct_digest_collect_items_field() {
    // Two Collect primitives identical except items field differs.
    let collect_a = make_collect("x", "items", Some(10), Some(50), vec![]);
    let collect_b = make_collect("x", "items", Some(10), Some(999), vec![]);

    let digest_a = digest_primitive(&collect_a);
    let digest_b = digest_primitive(&collect_b);

    // Different items fields MUST produce different digests
    assert_ne!(
        digest_a, digest_b,
        "different items fields must produce different digests via digest_step_primitive"
    );
}

// ─────────────────────────────────────────────────────────────────
// Items field (None vs Some)
// ─────────────────────────────────────────────────────────────────
#[test]
fn direct_digest_collect_items_none_equiv_some1() {
    // items: None vs items: Some(1)
    let collect_a = make_collect("x", "items", Some(10), None, vec![]);
    let collect_b = make_collect("x", "items", Some(10), Some(1), vec![]);

    let digest_a = digest_primitive(&collect_a);
    let digest_b = digest_primitive(&collect_b);

    // None and Some(1) both lower to `page_size = items.unwrap_or(1)` and
    // must share a digest (canonical equivalence).
    assert_eq!(
        digest_a, digest_b,
        "items None and Some(1) lower to page_size=1 and must share a digest"
    );
}

#[test]
fn direct_digest_collect_body_recursive() {
    // Two Collect primitives identical except body content differs.
    let body_a = vec![make_set_step("step_a", "a", "1")];
    let body_b = vec![make_set_step(
        "step_b", // different id
        "b",      // different output
        "1",
    )];

    let collect_a = make_collect("x", "items", Some(10), Some(50), body_a);
    let collect_b = make_collect("x", "items", Some(10), Some(50), body_b);

    let digest_a = digest_primitive(&collect_a);
    let digest_b = digest_primitive(&collect_b);

    // Different body step content MUST produce different digests
    assert_ne!(
        digest_a, digest_b,
        "different body step content must produce different digests via digest_step_primitive"
    );
}

// ─────────────────────────────────────────────────────────────────
// Idempotence: same input produces same digest
// ─────────────────────────────────────────────────────────────────
#[test]
fn direct_digest_collect_idempotence() {
    let collect = make_collect("x", "items", Some(10), Some(50), vec![]);

    let digest_a = digest_primitive(&collect);
    let digest_b = digest_primitive(&collect);

    assert_eq!(
        digest_a, digest_b,
        "same StepPrimitive::Collect must produce same digest (idempotence)"
    );
}

// ─────────────────────────────────────────────────────────────────
// Determinism: repeated calls produce same digest
// ─────────────────────────────────────────────────────────────────
#[test]
fn direct_digest_collect_repeated_calls_same_digest() {
    let collect = make_collect("x", "items", Some(10), Some(50), vec![]);

    // Call 1
    let d1 = digest_primitive(&collect);
    // Call 2
    let d2 = digest_primitive(&collect);
    // Call 3
    let d3 = digest_primitive(&collect);

    assert_eq!(d1, d2, "first and second call must match");
    assert_eq!(d2, d3, "second and third call must match");
}

// ─────────────────────────────────────────────────────────────────
// Empty body (zero steps) is distinct from non-empty body
// ─────────────────────────────────────────────────────────────────
#[test]
fn direct_digest_collect_empty_vs_nonempty_body() {
    let body = vec![make_set_step("inner", "x", "1")];

    let collect_empty = make_collect("x", "items", Some(10), Some(50), vec![]);
    let collect_nonempty = make_collect("x", "items", Some(10), Some(50), body);

    let digest_empty = digest_primitive(&collect_empty);
    let digest_nonempty = digest_primitive(&collect_nonempty);

    assert_ne!(
        digest_empty, digest_nonempty,
        "empty body vs non-empty body must produce different digests via digest_step_primitive"
    );
}
