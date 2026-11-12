use super::*;

// ─────────────────────────────────────────────────────────────────
// PO-003: Collect variable field hashing
// ─────────────────────────────────────────────────────────────────
#[test]
fn digest_collect_variable_field() {
    // Two Collect primitives identical except variable field differs.
    let yaml_a = collect_yaml_with_field("x", "items", Some(10), Some(50), None);
    let yaml_b = collect_yaml_with_field("y", "items", Some(10), Some(50), None);

    let digest_a = compute_compiled_digest(&yaml_a);
    let digest_b = compute_compiled_digest(&yaml_b);

    // Different variable fields MUST produce different digests
    assert_ne!(
        digest_a, digest_b,
        "different variable fields must produce different digests"
    );
}

// ─────────────────────────────────────────────────────────────────
// PO-004: Collect source field hashing
// ─────────────────────────────────────────────────────────────────
#[test]
fn digest_collect_source_field() {
    // Two Collect primitives identical except source field differs.
    let yaml_a = collect_yaml_with_field("x", "items_a", Some(10), Some(50), None);
    let yaml_b = collect_yaml_with_field("x", "items_b", Some(10), Some(50), None);

    let digest_a = compute_compiled_digest(&yaml_a);
    let digest_b = compute_compiled_digest(&yaml_b);

    // Different source fields MUST produce different digests
    assert_ne!(
        digest_a, digest_b,
        "different source fields must produce different digests"
    );
}

// ─────────────────────────────────────────────────────────────────
// PO-005: Collect pages field hashing
// ─────────────────────────────────────────────────────────────────
#[test]
fn digest_collect_pages_field() {
    // Two Collect primitives identical except pages field differs.
    let yaml_a = collect_yaml_with_field("x", "items", Some(10), Some(50), None);
    let yaml_b = collect_yaml_with_field("x", "items", Some(999), Some(50), None);

    let digest_a = compute_compiled_digest(&yaml_a);
    let digest_b = compute_compiled_digest(&yaml_b);

    // Different pages fields MUST produce different digests
    assert_ne!(
        digest_a, digest_b,
        "different pages fields must produce different digests"
    );
}

// ─────────────────────────────────────────────────────────────────
// PO-005b: Collect pages None vs Some
// ─────────────────────────────────────────────────────────────────
#[test]
fn digest_collect_pages_none_vs_some() {
    // pages: None vs pages: Some(1)
    let yaml_a = collect_yaml_with_field("x", "items", None, Some(50), None);
    let yaml_b = collect_yaml_with_field("x", "items", Some(1), Some(50), None);

    let digest_a = compute_compiled_digest(&yaml_a);
    let digest_b = compute_compiled_digest(&yaml_b);

    assert_ne!(
        digest_a, digest_b,
        "pages None vs Some(1) must produce different digests"
    );
}

// ─────────────────────────────────────────────────────────────────
// PO-006: Collect items field hashing
// ─────────────────────────────────────────────────────────────────
#[test]
fn digest_collect_items_field() {
    // Two Collect primitives identical except items field differs.
    let yaml_a = collect_yaml_with_field("x", "items", Some(10), Some(50), None);
    let yaml_b = collect_yaml_with_field("x", "items", Some(10), Some(999), None);

    let digest_a = compute_compiled_digest(&yaml_a);
    let digest_b = compute_compiled_digest(&yaml_b);

    // Different items fields MUST produce different digests
    assert_ne!(
        digest_a, digest_b,
        "different items fields must produce different digests"
    );
}

// ─────────────────────────────────────────────────────────────────
// PO-006b: Collect items None vs Some
// ─────────────────────────────────────────────────────────────────
#[test]
fn digest_collect_items_none_vs_some() {
    // items: None vs items: Some(1)
    let yaml_a = collect_yaml_with_field("x", "items", Some(10), None, None);
    let yaml_b = collect_yaml_with_field("x", "items", Some(10), Some(1), None);

    let digest_a = compute_compiled_digest(&yaml_a);
    let digest_b = compute_compiled_digest(&yaml_b);

    assert_ne!(
        digest_a, digest_b,
        "items None vs Some(1) must produce different digests"
    );
}

// ─────────────────────────────────────────────────────────────────
// PO-007: Collect body recursive hashing
// ─────────────────────────────────────────────────────────────────
#[test]
fn digest_collect_body_recursive() {
    // Two Collect primitives identical except body content differs.
    let yaml_a = collect_yaml_with_field("x", "items", Some(10), Some(50), Some("a"));
    let yaml_b = collect_yaml_with_field("x", "items", Some(10), Some(50), Some("b"));

    let digest_a = compute_compiled_digest(&yaml_a);
    let digest_b = compute_compiled_digest(&yaml_b);

    // Different body step content MUST produce different digests
    assert_ne!(
        digest_a, digest_b,
        "different body step content must produce different digests"
    );
}

// ─────────────────────────────────────────────────────────────────
// PO-014: Property-based digest equality
// ─────────────────────────────────────────────────────────────────
#[test]
fn collect_digest_equality_property() {
    // Same source MUST produce same digest (idempotence)
    let yaml = collect_yaml_with_field("x", "items", Some(10), Some(50), None);
    let digest_a = compute_compiled_digest(&yaml);
    let digest_b = compute_compiled_digest(&yaml);
    assert_eq!(
        digest_a, digest_b,
        "same source must produce same digest (idempotence)"
    );

    // Different sources MUST produce different digests (collision resistance)
    let yaml_other = collect_yaml_with_field("y", "items", Some(10), Some(50), None);
    let digest_other = compute_compiled_digest(&yaml_other);
    assert_ne!(
        digest_a, digest_other,
        "different sources must produce different digests"
    );
}

// ─────────────────────────────────────────────────────────────────
// Determinism: repeated calls produce same digest
// ─────────────────────────────────────────────────────────────────
#[test]
fn digest_collect_repeated_calls_same_digest() {
    let yaml = collect_yaml_with_field("x", "items", Some(10), Some(50), None);

    // Call 1
    let d1 = compute_compiled_digest(&yaml);
    // Call 2
    let d2 = compute_compiled_digest(&yaml);
    // Call 3
    let d3 = compute_compiled_digest(&yaml);

    assert_eq!(d1, d2, "first and second call must match");
    assert_eq!(d2, d3, "second and third call must match");
}
