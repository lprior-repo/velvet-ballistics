// Behavior test: canonical_digest timeout sensitivity (B2, B5)
// Bead: vb-xi2f.33 / P1: digest covers ask semantics
//
// Verifies INV-ASK-002: Changing an Ask timeout changes the canonical digest.
// Verifies INV-ASK-005: None vs Some("") timeout produce distinct digests.
// Verifies POST-002 and POST-005.

#![forbid(unsafe_code)]
#![allow(clippy::expect_used)]

mod common;
use common::ask_source;
use vb_compile::canonical_digest;

// ── B2: Different timeouts produce distinct digests ──

#[test]
fn canonical_digest_produces_distinct_digests_when_ask_timeout_none_vs_some() {
    // Given: sources differing only in timeout (None vs Some)
    let source_a = ask_source("prompt", None);
    let source_b = ask_source("prompt", Some("30s"));
    // When
    let digest_a = canonical_digest(&source_a);
    let digest_b = canonical_digest(&source_b);
    // Then
    assert_ne!(
        digest_a, digest_b,
        "INV-ASK-002: None vs Some(30s) timeout must produce distinct digests"
    );
}

#[test]
fn canonical_digest_produces_distinct_digests_when_ask_timeout_two_different_some_values() {
    // Given: sources with different Some(timeout) values
    let source_a = ask_source("prompt", Some("10s"));
    let source_b = ask_source("prompt", Some("30s"));
    // When
    let digest_a = canonical_digest(&source_a);
    let digest_b = canonical_digest(&source_b);
    // Then
    assert_ne!(
        digest_a, digest_b,
        "INV-ASK-002: different timeout values (10s vs 30s) must produce distinct digests"
    );
}

#[test]
fn canonical_digest_produces_identical_digests_when_ask_timeout_same_value() {
    // Given: sources with same timeout value
    let source_a = ask_source("prompt", Some("30s"));
    let source_b = ask_source("prompt", Some("30s"));
    // When
    let digest_a = canonical_digest(&source_a);
    let digest_b = canonical_digest(&source_b);
    // Then
    assert_eq!(
        digest_a, digest_b,
        "Identical timeout values must produce identical digests"
    );
}

#[test]
fn canonical_digest_produces_distinct_digests_when_ask_timeout_special_chars() {
    // Given: timeouts with different special characters
    let source_a = ask_source("prompt", Some("10s"));
    let source_b = ask_source("prompt", Some("10\ns"));
    // When
    let digest_a = canonical_digest(&source_a);
    let digest_b = canonical_digest(&source_b);
    // Then
    assert_ne!(
        digest_a, digest_b,
        "Timeout values with different special characters must produce distinct digests"
    );
}

// ── B5: None vs Some("") timeout distinction ──

#[test]
fn canonical_digest_produces_distinct_digests_when_timeout_none_vs_some_empty() {
    // Given: same prompt, one with None timeout, one with Some("")
    let source_a = ask_source("prompt", None);
    let source_b = ask_source("prompt", Some(""));
    // When
    let digest_a = canonical_digest(&source_a);
    let digest_b = canonical_digest(&source_b);
    // Then: INV-ASK-005 — None vs Some("") must produce distinct digests
    assert_ne!(
        digest_a, digest_b,
        "INV-ASK-005: None vs Some(\"\") timeout must produce distinct digests"
    );
}

#[test]
fn canonical_digest_produces_distinct_digests_when_timeout_some_empty_vs_some_value() {
    // Given: Some("") vs Some("30s") — both are Some, but values differ
    let source_a = ask_source("prompt", Some(""));
    let source_b = ask_source("prompt", Some("30s"));
    // When
    let digest_a = canonical_digest(&source_a);
    let digest_b = canonical_digest(&source_b);
    // Then
    assert_ne!(
        digest_a, digest_b,
        "Some(\"\") vs Some(30s) timeout must produce distinct digests"
    );
}
