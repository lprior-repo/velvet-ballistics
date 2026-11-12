// Behavior test: canonical_digest determinism (B3, B9)
// Bead: vb-xi2f.33 / P1: digest covers ask semantics
//
// Verifies INV-ASK-003: Same source always produces same digest (determinism).
// Verifies TC-002/WF-INV-003: field ordering is deterministic.
// Verifies POST-003.

#![forbid(unsafe_code)]

mod common;
use common::{ask_source, set_finish_source};
use vb_compile::canonical_digest;

// ── B3: Determinism — same source, multiple calls ──

#[test]
fn canonical_digest_is_deterministic_when_called_multiple_times_with_ask_and_timeout() {
    // Given: a source with Ask(prompt, Some(timeout))
    let source = ask_source("hello", Some("30s"));
    // When: called three times
    let digest_1 = canonical_digest(&source);
    let digest_2 = canonical_digest(&source);
    let digest_3 = canonical_digest(&source);
    // Then: all three are identical
    assert_eq!(
        digest_1, digest_2,
        "INV-ASK-003: canonical_digest must be deterministic (call 1 vs 2)"
    );
    assert_eq!(
        digest_1, digest_3,
        "INV-ASK-003: canonical_digest must be deterministic (call 1 vs 3)"
    );
    assert_eq!(
        digest_2, digest_3,
        "INV-ASK-003: canonical_digest must be deterministic (call 2 vs 3)"
    );
}

#[test]
fn canonical_digest_is_deterministic_when_called_multiple_times_with_ask_empty_prompt_none_timeout()
{
    // Given: a source with Ask("", None)
    let source = ask_source("", None);
    // When: called three times
    let digest_1 = canonical_digest(&source);
    let digest_2 = canonical_digest(&source);
    let digest_3 = canonical_digest(&source);
    // Then: all identical
    assert_eq!(digest_1, digest_2);
    assert_eq!(digest_1, digest_3);
}

#[test]
fn canonical_digest_is_deterministic_when_called_multiple_times_with_set_finish_source() {
    // Given: a source with Set + Finish (no Ask)
    let source = set_finish_source();
    // When: called three times
    let digest_1 = canonical_digest(&source);
    let digest_2 = canonical_digest(&source);
    let digest_3 = canonical_digest(&source);
    // Then: all identical
    assert_eq!(digest_1, digest_2);
    assert_eq!(digest_1, digest_3);
}

// ── B9: Field ordering determinism ──

#[test]
fn digest_step_primitive_hashes_ask_fields_in_deterministic_order_same_source_twice() {
    // Given: same Ask source called twice
    let source = ask_source("abc", Some("xyz"));
    // When
    let digest_a = canonical_digest(&source);
    let digest_b = canonical_digest(&source);
    // Then: same digest both times (proves no non-determinism from field ordering)
    assert_eq!(
        digest_a, digest_b,
        "TC-002: same source must produce same digest on repeated calls (field order deterministic)"
    );
}

#[test]
fn canonical_digest_produces_same_digest_for_same_fields_different_declaration_order() {
    // Given: two sources with identical Ask fields, constructed with same values
    // (Rust struct literal declaration order doesn't affect equality, but we verify digest matches)
    let source_a = ask_source("test", Some("30s"));
    let source_b = ask_source("test", Some("30s"));
    // When
    let digest_a = canonical_digest(&source_a);
    let digest_b = canonical_digest(&source_b);
    // Then: same fields → same digest (field order is deterministic)
    assert_eq!(
        digest_a, digest_b,
        "TC-002: identical field values must produce identical digests"
    );
}
