// Behavior test: canonical_digest prompt sensitivity (B1)
// Bead: vb-xi2f.33 / P1: digest covers ask semantics
//
// Verifies INV-ASK-001: Changing an Ask prompt changes the canonical digest.
// Verifies POST-001: Different prompts produce semantically distinct digests.

#![forbid(unsafe_code)]
#![allow(clippy::expect_used)]

mod common;
use common::ask_source;
use vb_compile::canonical_digest;

// ── B1: Different prompts produce distinct digests ──

#[test]
fn canonical_digest_produces_distinct_digests_when_ask_prompt_differs_simple() {
    // Given: two sources differing only in Ask prompt
    let source_a = ask_source("hello", Some("30s"));
    let source_b = ask_source("world", Some("30s"));
    // When
    let digest_a = canonical_digest(&source_a);
    let digest_b = canonical_digest(&source_b);
    // Then
    assert_ne!(
        digest_a, digest_b,
        "INV-ASK-001: different prompts (hello vs world) must produce distinct digests"
    );
}

#[test]
fn canonical_digest_produces_distinct_digests_when_ask_prompt_one_empty_one_nonempty() {
    // Given: one source with empty prompt, one with non-empty
    let source_a = ask_source("", Some("30s"));
    let source_b = ask_source("hello", Some("30s"));
    // When
    let digest_a = canonical_digest(&source_a);
    let digest_b = canonical_digest(&source_b);
    // Then
    assert_ne!(
        digest_a, digest_b,
        "INV-ASK-004: empty prompt must produce distinct digest from non-empty prompt"
    );
}

#[test]
fn canonical_digest_produces_identical_digests_when_ask_prompts_match() {
    // Given: two sources with identical prompts
    let source_a = ask_source("same", None);
    let source_b = ask_source("same", None);
    // When
    let digest_a = canonical_digest(&source_a);
    let digest_b = canonical_digest(&source_b);
    // Then: identical prompts produce identical digests
    assert_eq!(
        digest_a, digest_b,
        "Identical prompts must produce identical digests"
    );
}

#[test]
fn canonical_digest_produces_distinct_digests_when_ask_prompt_has_special_chars() {
    // Given: prompts with different special characters
    let source_a = ask_source("hello\nworld", None);
    let source_b = ask_source("hello\tworld", None);
    // When
    let digest_a = canonical_digest(&source_a);
    let digest_b = canonical_digest(&source_b);
    // Then
    assert_ne!(
        digest_a, digest_b,
        "Prompts with different special characters must produce distinct digests"
    );
}

#[test]
fn canonical_digest_produces_distinct_digests_when_ask_prompt_has_unicode() {
    // Given: prompts with different Unicode characters
    let source_a = ask_source("hello", None);
    let source_b = ask_source("hellö", None);
    // When
    let digest_a = canonical_digest(&source_a);
    let digest_b = canonical_digest(&source_b);
    // Then
    assert_ne!(
        digest_a, digest_b,
        "Prompts with different Unicode characters must produce distinct digests"
    );
}

#[test]
fn canonical_digest_produces_distinct_digests_when_ask_prompt_very_long() {
    // Given: two sources with long but different prompts
    let long_prompt_a = "a".repeat(2048);
    let long_prompt_b = "b".repeat(2048);
    let source_a = ask_source(&long_prompt_a, None);
    let source_b = ask_source(&long_prompt_b, None);
    // When
    let digest_a = canonical_digest(&source_a);
    let digest_b = canonical_digest(&source_b);
    // Then: long prompts should still produce distinct digests
    assert_ne!(
        digest_a, digest_b,
        "Long prompts (>1KB) must produce distinct digests when content differs"
    );
}
