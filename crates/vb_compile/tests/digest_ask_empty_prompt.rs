// Behavior test: Ask empty prompt produces valid distinct digest (B4)
// Bead: vb-xi2f.33 / P1: digest covers ask semantics
//
// Verifies INV-ASK-004: Empty prompt produces well-defined digest
// distinct from any non-empty prompt. Also verifies POST-004.

#![forbid(unsafe_code)]

mod common;
use common::ask_source;
use vb_compile::canonical_digest;

// ── B4: Empty prompt produces distinct, valid digest ──

#[test]
fn canonical_digest_produces_distinct_digest_when_ask_prompt_is_empty_vs_nonempty() {
    // Given: source with empty prompt vs non-empty
    let source_a = ask_source("", None);
    let source_b = ask_source("hello", None);
    // When
    let digest_a = canonical_digest(&source_a);
    let digest_b = canonical_digest(&source_b);
    // Then: INV-ASK-004 — empty prompt must produce distinct digest
    assert_ne!(
        digest_a, digest_b,
        "INV-ASK-004: empty prompt must produce distinct digest from non-empty prompt"
    );
}

#[test]
fn canonical_digest_returns_valid_hash_when_ask_prompt_is_empty() {
    // Given: source with empty prompt
    let source = ask_source("", None);
    // When
    let digest = canonical_digest(&source);
    let bytes = digest.as_bytes();
    // Then: digest is a valid 32-byte hash, not all zeros
    assert_eq!(
        bytes.len(),
        32,
        "POST-004: empty prompt must produce valid 32-byte digest"
    );
    let all_zero = bytes.iter().all(|b| *b == 0);
    assert!(
        !all_zero,
        "POST-004: empty prompt must not produce all-zero digest"
    );
}

#[test]
fn canonical_digest_produces_identical_digests_when_both_prompts_are_empty() {
    // Given: two sources both with empty prompt
    let source_a = ask_source("", None);
    let source_b = ask_source("", None);
    // When
    let digest_a = canonical_digest(&source_a);
    let digest_b = canonical_digest(&source_b);
    // Then: identical sources produce identical digests (determinism holds)
    assert_eq!(
        digest_a, digest_b,
        "Two sources with empty prompts must produce identical digests"
    );
}

#[test]
fn canonical_digest_empty_prompt_distinct_from_single_char() {
    // Given: empty prompt "" vs single character "x"
    let source_a = ask_source("", None);
    let source_b = ask_source("x", None);
    // When
    let digest_a = canonical_digest(&source_a);
    let digest_b = canonical_digest(&source_b);
    // Then: empty vs single char must be distinct
    assert_ne!(
        digest_a, digest_b,
        "Empty prompt must produce distinct digest from single-character prompt"
    );
}
