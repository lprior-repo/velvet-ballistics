// Behavior tests: explicit Ask arm, panic-freedom, sentinel distinction (B8, B10, B11, B12)
// Bead: vb-xi2f.33 / P1: digest covers ask semantics
//
// B8: digest_step_primitive handles Ask via explicit arm, not catch-all.
// B10: Empty prompt is valid hash input - does not panic or degenerate.
// B11: Timeout sentinel distinction (b"no_timeout" vs b"timeout").
// B12: digest_step_primitive never panics, unwraps, or expects on any valid primitive.
//
// Verifies TC-001, TC-003, TC-004, TC-007.

#![forbid(unsafe_code)]
#![allow(clippy::expect_used)]

mod common;
use common::{
    ask_source, empty_source, finish_source_integer, finish_source_string, set_finish_source,
    set_source,
};
use vb_compile::canonical_digest;
use vb_yaml::ast::StepPrimitive;

// ── B8: Explicit Ask arm verification (runtime) ──

/// Verifies that `digest_step_primitive` with an Ask primitive produces different
/// hash output than what the catch-all would produce (which only hashes the primitive name).
///
/// Strategy: Compare canonical_digest of a source with one Ask step to a source
/// that would produce an identical digest if only the primitive name were hashed.
/// If the Ask arm hashes prompt+timeout, the digests MUST differ.
#[test]
fn digest_step_primitive_ask_produces_different_result_than_catch_all_would() {
    // Given: A source with Ask(prompt="hello", timeout=Some("30s"))
    let _ask_source = ask_source("hello", Some("30s"));
    // And: A source with a Do step using the same step ID (Do falls to catch-all)
    // Since catch-all only hashes the primitive name, if Ask were catch-all it would
    // just hash b"ask" and produce the same result as a Do step with ID "ask_1"
    // that hashes b"do". These are different names, so we use a different approach.

    // Instead, verify: two different Ask prompts produce different digests through
    // the explicit arm. If Ask fell to catch-all, both would hash only "ask" and
    // produce identical digests (since step IDs match).
    let source_a = ask_source("hello", None);
    let source_b = ask_source("world", None);

    let digest_a = canonical_digest(&source_a).expect("valid test input");
    let digest_b = canonical_digest(&source_b).expect("valid test input");

    // If Ask used catch-all, both digests would be identical (only "ask" name hashed).
    // With explicit arm, prompt differences change the digest.
    assert_ne!(
        digest_a, digest_b,
        "TC-001: Ask must use explicit arm that hashes prompt — different prompts must yield different digests"
    );
}

/// Verify that Ask primitive reaches the explicit arm and not the catch-all
/// by testing that prompt content affects the digest.
#[test]
fn digest_step_primitive_ask_hashes_prompt_proving_explicit_arm() {
    // Given: two Ask sources with same timeout but different prompts
    let source_a = ask_source("short_prompt", Some("30s"));
    let source_b = ask_source("a_different_prompt", Some("30s"));

    let digest_a = canonical_digest(&source_a).expect("valid test input");
    let digest_b = canonical_digest(&source_b).expect("valid test input");

    // If prompt content doesn't affect digest, Ask fell through to catch-all
    assert_ne!(
        digest_a, digest_b,
        "TC-001: prompt content must affect digest (explicit arm), not catch-all which ignores prompt"
    );
}

// ── B10: Empty prompt input validity ──

#[test]
fn digest_step_primitive_accepts_empty_prompt_without_panic_none_timeout() {
    // Given: Ask with empty prompt, No timeout
    let source = ask_source("", None);
    // When/Then: must not panic
    let digest = canonical_digest(&source).expect("valid test input");
    // And: must produce valid 32-byte digest
    assert_eq!(
        digest.as_bytes().len(),
        32,
        "TC-003: empty prompt with None timeout must produce valid 32-byte digest"
    );
}

#[test]
fn digest_step_primitive_accepts_empty_prompt_without_panic_some_empty_timeout() {
    // Given: Ask with empty prompt, Some("") timeout
    let source = ask_source("", Some(""));
    // When/Then: must not panic
    let digest = canonical_digest(&source).expect("valid test input");
    assert_eq!(
        digest.as_bytes().len(),
        32,
        "TC-003: empty prompt with Some(\"\") timeout must produce valid 32-byte digest"
    );
}

#[test]
fn digest_step_primitive_accepts_empty_prompt_without_panic_some_nonempty_timeout() {
    // Given: Ask with empty prompt, Some("30s") timeout
    let source = ask_source("", Some("30s"));
    // When/Then: must not panic
    let digest = canonical_digest(&source).expect("valid test input");
    assert_eq!(
        digest.as_bytes().len(),
        32,
        "TC-003: empty prompt with Some(30s) timeout must produce valid 32-byte digest"
    );
}

// ── B11: Timeout sentinel distinction ──

/// Verifies TC-004: `None` timeout uses sentinel `b"no_timeout"`,
/// `Some("")` uses `b"timeout"` + `b""`.
/// Since we cannot inspect hasher internals, we verify indirectly:
/// None vs Some("") produce distinct digests (the sentinels are different).
#[test]
fn digest_step_primitive_uses_distinct_sentinel_for_none_timeout() {
    // Given: same prompt, None vs Some("")
    let source_none = ask_source("prompt", None);
    let source_some_empty = ask_source("prompt", Some(""));

    let digest_none = canonical_digest(&source_none).expect("valid test input");
    let digest_some_empty = canonical_digest(&source_some_empty).expect("valid test input");

    // If same sentinel were used for both, digests would be identical
    assert_ne!(
        digest_none, digest_some_empty,
        "TC-004: None timeout sentinel (b\"no_timeout\") must differ from Some(\"\") (b\"timeout\" + b\"\")"
    );
}

// ── B12: Panic freedom for all valid variants ──

#[test]
fn digest_step_primitive_does_not_panic_for_ask_normal_variant() {
    let source = ask_source("hello", Some("30s"));
    // Must not panic
    let _ = canonical_digest(&source).expect("valid test input");
}

#[test]
fn digest_step_primitive_does_not_panic_for_ask_empty_prompt_none_timeout() {
    let source = ask_source("", None);
    let _ = canonical_digest(&source).expect("valid test input");
}

#[test]
fn digest_step_primitive_does_not_panic_for_ask_large_prompt() {
    let large_prompt = "a".repeat(10240);
    let source = ask_source(&large_prompt, None);
    let _ = canonical_digest(&source).expect("valid test input");
}

#[test]
fn digest_step_primitive_does_not_panic_for_ask_prompt_with_all_visible_controls() {
    // Test all printable ASCII including control-looking chars
    let prompt: String = (0u8..=127)
        .map(|c| char::from_u32(c.into()).expect("0u8..=127 are valid Unicode scalar values"))
        .collect();
    let source = ask_source(&prompt, None);
    let _ = canonical_digest(&source).expect("valid test input");
}

#[test]
fn digest_step_primitive_does_not_panic_for_set_primitive() {
    let source = set_source("x", "1");
    let _ = canonical_digest(&source).expect("valid test input");
}

#[test]
fn digest_step_primitive_does_not_panic_for_finish_string_primitive() {
    let source = finish_source_string("done");
    let _ = canonical_digest(&source).expect("valid test input");
}

#[test]
fn digest_step_primitive_does_not_panic_for_finish_integer_primitive() {
    let source = finish_source_integer(42);
    let _ = canonical_digest(&source).expect("valid test input");
}

#[test]
fn digest_step_primitive_does_not_panic_for_empty_source() {
    let source = empty_source();
    let _ = canonical_digest(&source).expect("valid test input");
}

#[test]
fn digest_step_primitive_does_not_panic_for_do_primitive_catch_all() {
    // Do falls through to catch-all arm — must not panic
    use vb_yaml::ast::{StepAst, TriggerAst, WorkflowSource, WorkflowSourceParts};
    let source = WorkflowSource::new(WorkflowSourceParts {
        version: "velvet-ballistics/v1".to_string(),
        name: "test_do".to_string(),
        trigger: TriggerAst::Manual,
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![StepAst {
            id: "do_1".to_string(),
            name: None,
            condition: None,
            primitive: StepPrimitive::Do {
                action: "test_action".to_string(),
                input: "test_input".to_string(),
            },
            with: None,
            retry: None,
            on_error: None,
            then: None,
        }],
        result: None,
        examples: vec![],
    });
    let _ = canonical_digest(&source).expect("valid test input");
}

#[test]
fn canonical_digest_does_not_panic_for_ask_with_empty_timeout() {
    let source = ask_source("hello", Some(""));
    let _ = canonical_digest(&source).expect("valid test input");
}

#[test]
fn canonical_digest_does_not_panic_for_set_finish_source() {
    let source = set_finish_source();
    let _ = canonical_digest(&source).expect("valid test input");
}
