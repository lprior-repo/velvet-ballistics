//! Unit tests for ForEach digest coverage.
//!
//! Bead: vb-xi2f.28 | State: 9 (test-writer)
//!
//! Tests the `digest_step_primitive` and `canonical_digest` functions
//! for ForEach field sensitivity, semantic equivalence, and edge cases.
//!
//! BDD scenarios covered:
//!   B7:  at_once=None vs Some(1) equivalence (AC-FE-07)
//!   B8:  at_once=None vs Some(0) inequivalence (AC-FE-07 inverse)
//!   B10: ForEach arm hit (not catch-all fallthrough) (INV-FE-01)
//!   B13: Empty body produces deterministic digest (G-FE-06)
//!   B14: Body step ID sensitivity (Contract §2.3)
//!   B17: at_once=Some(0) distinct from None and Some(1)
//!   +    Edge cases: u32::MAX, empty variable, non-ASCII, Finish body

use crate::{canonical_digest_part05, digest_step_primitive_part05};
use vb_yaml::ast::{
    ScalarValue, StepAst, StepPrimitive, TriggerAst, WorkflowSource, WorkflowSourceParts,
};

/// Build a minimal ForEach step for step-level testing with `digest_step_primitive`.
fn foreach_step(
    variable: &str,
    input: &str,
    at_once: Option<u32>,
    body: Vec<StepAst>,
) -> StepPrimitive {
    StepPrimitive::ForEach {
        variable: variable.to_string(),
        input: input.to_string(),
        at_once,
        body,
    }
}

/// Build a simple Set body step with the given id, output, and value.
fn set_body_step(id: &str, output: &str, value: &str) -> StepAst {
    StepAst {
        id: id.to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Set {
            output: output.to_string(),
            value: value.to_string(),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }
}

/// Build a simple Finish body step with the given id and integer result.
fn finish_body_step(id: &str, result: i64) -> StepAst {
    StepAst {
        id: id.to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Finish {
            result: ScalarValue::Integer(result),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }
}

/// Build a WorkflowSource with a single ForEach step for `canonical_digest` testing.
fn foreach_workflow(
    variable: String,
    input: String,
    at_once: Option<u32>,
    body: Vec<StepAst>,
) -> WorkflowSource {
    let steps = vec![StepAst {
        id: "step1".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::ForEach {
            variable,
            input,
            at_once,
            body,
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }];
    WorkflowSource::new(WorkflowSourceParts {
        version: "velvet-ballastics/v1".to_string(),
        name: "test".to_string(),
        trigger: TriggerAst::Manual,
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps,
        result: None,
        examples: vec![],
    })
}

/// Compute digest bytes from `digest_step_primitive` on a standalone hasher.
fn step_digest_bytes(primitive: &StepPrimitive) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    digest_step_primitive_part05(&mut hasher, primitive);
    *hasher.finalize().as_bytes()
}

/// Compute final digest bytes from a standalone hasher fed only the string `s`.
fn name_only_bytes(s: &str) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(s.as_bytes());
    *hasher.finalize().as_bytes()
}

// ═══════════════════════════════════════════════════════════════════════════
// B7: at_once=None vs Some(1) equivalence (AC-FE-07)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn foreach_at_once_none_some1_produces_identical_step_digest() {
    // Given: Two ForEach steps identical except at_once=None vs Some(1)
    let body = vec![set_body_step("s1", "x", "42")];
    let step_none = foreach_step("item", "items", None, body.clone());
    let step_some1 = foreach_step("item", "items", Some(1), body);

    // When: digest_step_primitive is called on each with independent hashers
    let digest_none = step_digest_bytes(&step_none);
    let digest_some1 = step_digest_bytes(&step_some1);

    // Then: Both hashers produce identical bytes
    // (None → 1u32, Some(1) → 1u32 — semantic equivalence)
    assert_eq!(
        digest_none, digest_some1,
        "at_once=None and at_once=Some(1) must produce identical digest contributions"
    );
}

#[test]
fn foreach_at_once_none_some1_produces_identical_workflow_digest() {
    // Given: Two WorkflowSources differing only in at_once=None vs Some(1)
    let body = vec![set_body_step("s1", "x", "42")];
    let source_none = foreach_workflow("item".to_string(), "items".to_string(), None, body.clone());
    let source_some1 = foreach_workflow("item".to_string(), "items".to_string(), Some(1), body);

    // When: canonical_digest is called on each
    let digest_none = canonical_digest_part05(&source_none);
    let digest_some1 = canonical_digest_part05(&source_some1);

    // Then: Both produce identical WorkflowDigest values
    assert_eq!(
        digest_none, digest_some1,
        "at_once=None and at_once=Some(1) must produce identical workflow digests"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// B8: at_once=None vs Some(0) inequivalence (AC-FE-07 inverse)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn foreach_at_once_none_produces_different_step_digest_than_some0() {
    // Given: Two ForEach steps identical except at_once=None vs Some(0)
    let body = vec![set_body_step("s1", "x", "42")];
    let step_none = foreach_step("item", "items", None, body.clone());
    let step_some0 = foreach_step("item", "items", Some(0), body);

    // When: digest_step_primitive is called on each
    let digest_none = step_digest_bytes(&step_none);
    let digest_some0 = step_digest_bytes(&step_some0);

    // Then: Produces different digest bytes
    // (None → 1u32, Some(0) → 0u32 — different resolved values)
    assert_ne!(
        digest_none, digest_some0,
        "at_once=None (→1u32) and at_once=Some(0) (→0u32) must produce different step digests"
    );
}

#[test]
fn foreach_at_once_some1_produces_different_step_digest_than_some0() {
    // Given: at_once=Some(1) vs Some(0)
    let body = vec![set_body_step("s1", "x", "42")];
    let step_some1 = foreach_step("item", "items", Some(1), body.clone());
    let step_some0 = foreach_step("item", "items", Some(0), body);

    // When: digest_step_primitive is called
    let digest_some1 = step_digest_bytes(&step_some1);
    let digest_some0 = step_digest_bytes(&step_some0);

    // Then: Different digests
    assert_ne!(
        digest_some1, digest_some0,
        "at_once=Some(1) and at_once=Some(0) must produce different step digests"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// B10: ForEach arm hit (not catch-all fallthrough) (INV-FE-01, PRF-FE-02)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn foreach_step_digest_contains_more_than_just_primitive_name() {
    // Given: A ForEach step with all fields populated
    let body = vec![set_body_step("s1", "x", "value")];
    let foreach = foreach_step("item", "items", Some(3), body);

    // When: digest_step_primitive is called
    let foreach_digest = step_digest_bytes(&foreach);
    // A second hasher fed only the primitive name "for_each" (the catch-all path)
    let name_only_digest = name_only_bytes("for_each");

    // Then: The ForEach digest differs from name-only digest
    // (Proves the explicit ForEach arm runs, not the catch-all)
    assert_ne!(
        foreach_digest, name_only_digest,
        "ForEach digest must differ from name-only 'for_each' hash (catch-all test)"
    );
}

#[test]
fn foreach_arm_produces_distinct_bytes_from_same_body_size_set() {
    // Given: A ForEach with body content
    // And: A Set step — which also uses an explicit arm
    let body = vec![set_body_step("s1", "x", "value")];
    let foreach = foreach_step("item", "items", Some(3), body);
    let set = StepPrimitive::Set {
        output: "x".to_string(),
        value: "value".to_string(),
    };

    // When: digests are computed
    let foreach_digest = step_digest_bytes(&foreach);
    let set_digest = step_digest_bytes(&set);

    // Then: Different (ForEach arm != Set arm)
    // The catch-all for Set would also produce "for_each" if it fell through
    assert_ne!(
        foreach_digest, set_digest,
        "ForEach digest must differ from equivalent Set digest (arm specificity)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// B13: Empty body produces valid, deterministic digest (G-FE-06)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn foreach_empty_body_produces_deterministic_step_digest() {
    // Given: A ForEach with empty body
    let empty_body: Vec<StepAst> = vec![];
    let foreach = foreach_step("x", "items", None, empty_body);

    // When: digest_step_primitive is called twice
    let d1 = step_digest_bytes(&foreach);
    let d2 = step_digest_bytes(&foreach);

    // Then: Both calls produce identical digests
    assert_eq!(
        d1, d2,
        "Empty-body ForEach must produce deterministic digests across calls"
    );
}

#[test]
fn foreach_empty_body_digest_differs_from_nonempty_body_digest() {
    // Given: Two ForEach steps — one empty body, one with a Set body step
    let empty_step = foreach_step("x", "items", None, vec![]);
    let nonempty_step = foreach_step("x", "items", None, vec![set_body_step("s1", "x", "42")]);

    // When: digest_step_primitive is called
    let empty_digest = step_digest_bytes(&empty_step);
    let nonempty_digest = step_digest_bytes(&nonempty_step);

    // Then: Digests differ (empty body is distinguishable)
    assert_ne!(
        empty_digest, nonempty_digest,
        "Empty-body ForEach digest must differ from non-empty body ForEach digest"
    );
}

#[test]
fn foreach_empty_body_workflow_digest_is_deterministic() {
    // Given: A WorkflowSource with empty-body ForEach
    let source = foreach_workflow("x".to_string(), "items".to_string(), None, vec![]);

    // When: canonical_digest is called twice
    let d1 = canonical_digest_part05(&source);
    let d2 = canonical_digest_part05(&source);

    // Then: Both calls produce identical digests
    assert_eq!(
        d1, d2,
        "Empty-body ForEach workflow digest must be deterministic"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// B14: Body step ID sensitivity (Contract §2.3, domain decision DD-02)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn foreach_body_step_id_variation_changes_step_digest() {
    // Given: Two ForEach steps identical except body[0].id
    let body_a = vec![set_body_step("step_a", "x", "1")];
    let body_b = vec![set_body_step("step_b", "x", "1")];
    let foreach_a = foreach_step("item", "items", None, body_a);
    let foreach_b = foreach_step("item", "items", None, body_b);

    // When: digest_step_primitive is called on each
    let digest_a = step_digest_bytes(&foreach_a);
    let digest_b = step_digest_bytes(&foreach_b);

    // Then: Digests differ (step ID is hashed)
    assert_ne!(
        digest_a, digest_b,
        "Changing body step ID must change the ForEach digest"
    );
}

#[test]
fn foreach_body_step_id_variation_changes_workflow_digest() {
    // Given: Two WorkflowSources with body steps differing only in id
    let body_a = vec![set_body_step("alpha", "x", "1")];
    let body_b = vec![set_body_step("beta", "x", "1")];
    let source_a = foreach_workflow("item".to_string(), "items".to_string(), None, body_a);
    let source_b = foreach_workflow("item".to_string(), "items".to_string(), None, body_b);

    // When: canonical_digest is called
    let digest_a = canonical_digest_part05(&source_a);
    let digest_b = canonical_digest_part05(&source_b);

    // Then: Different
    assert_ne!(
        digest_a, digest_b,
        "Changing body step ID must change the workflow-level digest"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// B17: at_once=Some(0) distinct from both None and Some(1)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn foreach_at_once_zero_step_digest_differs_from_none() {
    // Given: at_once=Some(0) vs None
    let body = vec![set_body_step("s1", "x", "1")];
    let step_zero = foreach_step("item", "items", Some(0), body.clone());
    let step_none = foreach_step("item", "items", None, body);

    // When: digest computed
    let digest_zero = step_digest_bytes(&step_zero);
    let digest_none = step_digest_bytes(&step_none);

    // Then: Different (Some(0)→0u32 != None→1u32)
    assert_ne!(
        digest_zero, digest_none,
        "at_once=Some(0) must produce different digest than at_once=None"
    );
}

#[test]
fn foreach_at_once_zero_step_digest_differs_from_some1() {
    // Given: at_once=Some(0) vs Some(1)
    let body = vec![set_body_step("s1", "x", "1")];
    let step_zero = foreach_step("item", "items", Some(0), body.clone());
    let step_one = foreach_step("item", "items", Some(1), body);

    // When: digest computed
    let digest_zero = step_digest_bytes(&step_zero);
    let digest_one = step_digest_bytes(&step_one);

    // Then: Different
    assert_ne!(
        digest_zero, digest_one,
        "at_once=Some(0) must produce different digest than at_once=Some(1)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Edge case: at_once=Some(u32::MAX) — maximum boundary
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn foreach_at_once_max_boundary_produces_distinct_step_digest() {
    // Given: at_once=Some(u32::MAX) vs None
    let body = vec![set_body_step("s1", "x", "1")];
    let step_max = foreach_step("item", "items", Some(u32::MAX), body.clone());
    let step_none = foreach_step("item", "items", None, body);

    // When: digests computed
    let digest_max = step_digest_bytes(&step_max);
    let digest_none = step_digest_bytes(&step_none);

    // Then: Different (u32::MAX != 1)
    assert_ne!(
        digest_max, digest_none,
        "at_once=Some(u32::MAX) must produce different digest than at_once=None"
    );
}

#[test]
fn foreach_at_once_max_vs_one_produces_distinct_step_digests() {
    // Given: at_once=Some(u32::MAX) vs Some(1)
    let body = vec![set_body_step("s1", "x", "1")];
    let step_max = foreach_step("item", "items", Some(u32::MAX), body.clone());
    let step_one = foreach_step("item", "items", Some(1), body);

    // When: digests computed
    let digest_max = step_digest_bytes(&step_max);
    let digest_one = step_digest_bytes(&step_one);

    // Then: Different
    assert_ne!(
        digest_max, digest_one,
        "at_once=Some(u32::MAX) must produce different digest than at_once=Some(1)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Edge case: Empty variable name
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn foreach_empty_variable_produces_deterministic_step_digest() {
    // Given: A ForEach step with empty variable name
    let foreach = foreach_step("", "items", None, vec![]);

    // When: digest_step_primitive is called twice
    let d1 = step_digest_bytes(&foreach);
    let d2 = step_digest_bytes(&foreach);

    // Then: Deterministic (empty byte sequence is valid)
    assert_eq!(
        d1, d2,
        "ForEach with empty variable must produce deterministic digests"
    );
}

#[test]
fn foreach_empty_variable_differs_from_nonempty_variable() {
    // Given: empty variable vs "item" variable
    let step_empty = foreach_step("", "items", None, vec![]);
    let step_nonempty = foreach_step("item", "items", None, vec![]);

    // When: digests computed
    let digest_empty = step_digest_bytes(&step_empty);
    let digest_nonempty = step_digest_bytes(&step_nonempty);

    // Then: Different (variable is hashed)
    assert_ne!(
        digest_empty, digest_nonempty,
        "ForEach with empty variable must produce different digest than non-empty variable"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Edge case: Non-ASCII variable name (e.g. "café")
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn foreach_non_ascii_variable_produces_deterministic_step_digest() {
    // Given: A ForEach with non-ASCII variable name "café" (UTF-8 bytes)
    let foreach = foreach_step("café", "items", None, vec![]);

    // When: digest_step_primitive is called twice
    let d1 = step_digest_bytes(&foreach);
    let d2 = step_digest_bytes(&foreach);

    // Then: Deterministic
    assert_eq!(
        d1, d2,
        "ForEach with non-ASCII variable must produce deterministic digests"
    );
}

#[test]
fn foreach_non_ascii_variable_differs_from_ascii_variable() {
    // Given: "café" (5 bytes: c a f 0xC3 0XA9) vs "cafe" (4 bytes)
    let step_accent = foreach_step("café", "items", None, vec![]);
    let step_ascii = foreach_step("cafe", "items", None, vec![]);

    // When: digests computed
    let digest_accent = step_digest_bytes(&step_accent);
    let digest_ascii = step_digest_bytes(&step_ascii);

    // Then: Different (UTF-8 bytes differ)
    assert_ne!(
        digest_accent, digest_ascii,
        "ForEach with non-ASCII variable must produce different digest than ASCII variant"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Body step primitive type diversity: Set vs Finish body
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn foreach_body_set_vs_finish_produces_different_step_digest() {
    // Given: ForEach with Set body vs Finish body
    let body_set = vec![set_body_step("s1", "x", "1")];
    let body_finish = vec![finish_body_step("s1", 42)];
    let foreach_set = foreach_step("item", "items", None, body_set);
    let foreach_finish = foreach_step("item", "items", None, body_finish);

    // When: digests computed
    let digest_set = step_digest_bytes(&foreach_set);
    let digest_finish = step_digest_bytes(&foreach_finish);

    // Then: Different (primitive type is hashed via recursive digest_step_primitive)
    assert_ne!(
        digest_set, digest_finish,
        "ForEach with Set body must produce different digest than ForEach with Finish body"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Body content sensitivity: changing Set output changes digest
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn foreach_body_set_output_variation_changes_step_digest() {
    // Given: Two ForEach steps with body Set differing only in output name
    let body_a = vec![set_body_step("s1", "alpha", "value")];
    let body_b = vec![set_body_step("s1", "beta", "value")];
    let foreach_a = foreach_step("item", "items", None, body_a);
    let foreach_b = foreach_step("item", "items", None, body_b);

    // When: digests computed
    let digest_a = step_digest_bytes(&foreach_a);
    let digest_b = step_digest_bytes(&foreach_b);

    // Then: Different (Set output is hashed recursively)
    assert_ne!(
        digest_a, digest_b,
        "Changing body Set.output must change the ForEach digest"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Input field sensitivity (B1 style unit test)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn foreach_input_variation_changes_step_digest() {
    // Given: Two ForEach steps identical except input field
    let step_items = foreach_step("item", "items_list", None, vec![]);
    let step_other = foreach_step("item", "other_list", None, vec![]);

    // When: digest_step_primitive called
    let digest_items = step_digest_bytes(&step_items);
    let digest_other = step_digest_bytes(&step_other);

    // Then: Different (input is hashed)
    assert_ne!(
        digest_items, digest_other,
        "Changing ForEach.input must change the step digest"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Variable field sensitivity (B3 style unit test)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn foreach_variable_variation_changes_step_digest() {
    // Given: Two ForEach steps identical except variable field
    let step_x = foreach_step("x", "items", None, vec![]);
    let step_y = foreach_step("y", "items", None, vec![]);

    // When: digest_step_primitive called
    let digest_x = step_digest_bytes(&step_x);
    let digest_y = step_digest_bytes(&step_y);

    // Then: Different (variable is hashed)
    assert_ne!(
        digest_x, digest_y,
        "Changing ForEach.variable must change the step digest"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Determinism: digest_step_primitive is deterministic (B5 unit-level)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn foreach_step_digest_is_deterministic_across_multiple_calls() {
    // Given: A ForEach step with all fields
    let body = vec![set_body_step("s1", "x", "42"), finish_body_step("s2", 0)];
    let foreach = foreach_step("item", "items", Some(5), body);

    // When: digest_step_primitive is called 5 times
    let digests: Vec<[u8; 32]> = (0..5).map(|_| step_digest_bytes(&foreach)).collect();

    // Then: All 5 digests are identical
    let first = digests[0];
    for (i, d) in digests.iter().enumerate().skip(1) {
        assert_eq!(
            first,
            *d,
            "Call {} produced different step digest than call 1",
            i + 1
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Multi-field sensitivity: all ForEach fields contribute to digest
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn foreach_all_fields_contribute_to_step_digest() {
    // Given: A baseline ForEach and 4 variants each changing one field
    let body_base = vec![set_body_step("s1", "x", "1")];
    let body_diff = vec![set_body_step("s1", "x", "999")];

    let base = foreach_step("item", "items", None, body_base);

    let var_diff = foreach_step("other", "items", None, vec![set_body_step("s1", "x", "1")]);
    let inp_diff = foreach_step(
        "item",
        "other_list",
        None,
        vec![set_body_step("s1", "x", "1")],
    );
    let ato_diff = foreach_step(
        "item",
        "items",
        Some(7),
        vec![set_body_step("s1", "x", "1")],
    );
    let bdy_diff = foreach_step("item", "items", None, body_diff);

    let base_digest = step_digest_bytes(&base);

    // When/Then: Each field variation produces a different digest
    let var_digest = step_digest_bytes(&var_diff);
    let inp_digest = step_digest_bytes(&inp_diff);
    let ato_digest = step_digest_bytes(&ato_diff);
    let bdy_digest = step_digest_bytes(&bdy_diff);

    assert_ne!(base_digest, var_digest, "Variable field must change digest");
    assert_ne!(base_digest, inp_digest, "Input field must change digest");
    assert_ne!(base_digest, ato_digest, "at_once field must change digest");
    assert_ne!(base_digest, bdy_digest, "Body field must change digest");
}

// ═══════════════════════════════════════════════════════════════════════════
// Multiple body steps: count and content sensitivity
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn foreach_body_step_count_changes_step_digest() {
    // Given: 1 body step vs 2 body steps
    let body_1 = vec![set_body_step("s1", "x", "1")];
    let body_2 = vec![set_body_step("s1", "x", "1"), set_body_step("s2", "y", "2")];
    let step_1 = foreach_step("item", "items", None, body_1);
    let step_2 = foreach_step("item", "items", None, body_2);

    // When: digests computed
    let digest_1 = step_digest_bytes(&step_1);
    let digest_2 = step_digest_bytes(&step_2);

    // Then: Different (body step count is hashed)
    assert_ne!(
        digest_1, digest_2,
        "Different body step counts must produce different digests"
    );
}

#[test]
fn foreach_body_step_order_changes_step_digest() {
    // Given: Two body steps in different order
    let body_ab = vec![set_body_step("a", "x", "1"), finish_body_step("b", 2)];
    let body_ba = vec![finish_body_step("b", 2), set_body_step("a", "x", "1")];
    let step_ab = foreach_step("item", "items", None, body_ab);
    let step_ba = foreach_step("item", "items", None, body_ba);

    // When: digests computed
    let digest_ab = step_digest_bytes(&step_ab);
    let digest_ba = step_digest_bytes(&step_ba);

    // Then: Different (step order is hashed)
    assert_ne!(
        digest_ab, digest_ba,
        "Different body step order must produce different digests"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Field separator / delimiter collision prevention
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn foreach_variable_containing_colon_does_not_cause_delimiter_collision() {
    // Given: A variable containing ":" (e.g., "var:iable") and another without
    // The delimiter bytes (b":variable:") use `:` to prevent boundary collisions.
    // Variables containing `:` should still produce correct digests.
    let step_with_colon = foreach_step("var:iable", "items", None, vec![]);
    let step_no_colon = foreach_step("variable", "items", None, vec![]);

    // When: digests computed
    let digest_with = step_digest_bytes(&step_with_colon);
    let digest_without = step_digest_bytes(&step_no_colon);

    // Then: Digests differ AND both are valid (no collision)
    assert_ne!(
        digest_with, digest_without,
        "Variable containing ':' must produce digest distinct from similar variable without ':'"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Canonical digest: ForEach step position matters (first vs last step)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn foreach_step_position_changes_workflow_digest() {
    // Given: Two WorkflowSources with ForEach at different positions
    let foreach_step = StepAst {
        id: "fe".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::ForEach {
            variable: "x".to_string(),
            input: "items".to_string(),
            at_once: None,
            body: vec![],
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    };
    let set_step = StepAst {
        id: "s1".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Set {
            output: "x".to_string(),
            value: "1".to_string(),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    };

    // ForEach first, Set second
    let source_fe_first = WorkflowSource::new(WorkflowSourceParts {
        version: "velvet-ballastics/v1".to_string(),
        name: "test".to_string(),
        trigger: TriggerAst::Manual,
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![foreach_step.clone(), set_step.clone()],
        result: None,
        examples: vec![],
    });
    // Set first, ForEach second
    let source_fe_last = WorkflowSource::new(WorkflowSourceParts {
        version: "velvet-ballastics/v1".to_string(),
        name: "test".to_string(),
        trigger: TriggerAst::Manual,
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![set_step, foreach_step],
        result: None,
        examples: vec![],
    });

    // When: canonical_digest called
    let digest_first = canonical_digest_part05(&source_fe_first);
    let digest_last = canonical_digest_part05(&source_fe_last);

    // Then: Different (step order matters via step.id hashing)
    assert_ne!(
        digest_first, digest_last,
        "ForEach step position (first vs last) must produce different workflow digests"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// B15: Nested ForEach body recursion — workflow-level (canonical_digest)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn foreach_nested_body_content_changes_workflow_digest() {
    // Given: Two WorkflowSources with ForEach whose body contains a nested ForEach
    // The nested ForEach content differs between the two sources.

    // Inner ForEach step for source A — simple Set body
    let inner_a = StepAst {
        id: "inner_fe".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::ForEach {
            variable: "nested_item".to_string(),
            input: "nested_list".to_string(),
            at_once: None,
            body: vec![StepAst {
                id: "nested_s".to_string(),
                name: None,
                condition: None,
                primitive: StepPrimitive::Set {
                    output: "a_val".to_string(),
                    value: "1".to_string(),
                },
                with: None,
                retry: None,
                on_error: None,
                then: None,
            }],
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    };

    // Inner ForEach step for source B — same structure, different inner value
    let inner_b = StepAst {
        id: "inner_fe".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::ForEach {
            variable: "nested_item".to_string(),
            input: "nested_list".to_string(),
            at_once: None,
            body: vec![StepAst {
                id: "nested_s".to_string(),
                name: None,
                condition: None,
                primitive: StepPrimitive::Set {
                    output: "b_val".to_string(),
                    value: "999".to_string(),
                },
                with: None,
                retry: None,
                on_error: None,
                then: None,
            }],
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    };

    // Outer ForEach — contains the inner ForEach as body
    let outer_step_a = StepAst {
        id: "outer_fe".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::ForEach {
            variable: "item".to_string(),
            input: "items".to_string(),
            at_once: None,
            body: vec![inner_a],
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    };
    let outer_step_b = StepAst {
        id: "outer_fe".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::ForEach {
            variable: "item".to_string(),
            input: "items".to_string(),
            at_once: None,
            body: vec![inner_b],
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    };

    let source_a = WorkflowSource::new(WorkflowSourceParts {
        version: "velvet-ballastics/v1".to_string(),
        name: "test".to_string(),
        trigger: TriggerAst::Manual,
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![outer_step_a],
        result: None,
        examples: vec![],
    });
    let source_b = WorkflowSource::new(WorkflowSourceParts {
        version: "velvet-ballastics/v1".to_string(),
        name: "test".to_string(),
        trigger: TriggerAst::Manual,
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![outer_step_b],
        result: None,
        examples: vec![],
    });

    // When: canonical_digest is called on each
    let digest_a = canonical_digest_part05(&source_a);
    let digest_b = canonical_digest_part05(&source_b);

    // Then: Digests differ (recursive body hashing detects nested ForEach content change)
    assert_ne!(
        digest_a, digest_b,
        "Nested ForEach content change must change outer ForEach workflow digest"
    );
}

#[test]
fn foreach_nested_foreach_vs_flat_set_body_produces_different_workflow_digest() {
    // Given: Workflow A — outer ForEach with nested ForEach in body
    //        Workflow B — outer ForEach with flat Set in body
    let nested_step = StepAst {
        id: "fe".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::ForEach {
            variable: "item".to_string(),
            input: "items".to_string(),
            at_once: None,
            body: vec![StepAst {
                id: "inner_fe".to_string(),
                name: None,
                condition: None,
                primitive: StepPrimitive::ForEach {
                    variable: "nested_item".to_string(),
                    input: "nested_list".to_string(),
                    at_once: None,
                    body: vec![set_body_step("ns1", "x", "1")],
                },
                with: None,
                retry: None,
                on_error: None,
                then: None,
            }],
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    };
    let flat_step = StepAst {
        id: "fe".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::ForEach {
            variable: "item".to_string(),
            input: "items".to_string(),
            at_once: None,
            body: vec![set_body_step("inner_s", "x", "1")],
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    };

    let source_nested = WorkflowSource::new(WorkflowSourceParts {
        version: "velvet-ballastics/v1".to_string(),
        name: "test".to_string(),
        trigger: TriggerAst::Manual,
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![nested_step],
        result: None,
        examples: vec![],
    });
    let source_flat = WorkflowSource::new(WorkflowSourceParts {
        version: "velvet-ballastics/v1".to_string(),
        name: "test".to_string(),
        trigger: TriggerAst::Manual,
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![flat_step],
        result: None,
        examples: vec![],
    });

    // When: canonical_digest is called
    let digest_nested = canonical_digest_part05(&source_nested);
    let digest_flat = canonical_digest_part05(&source_flat);

    // Then: Different (nested ForEach has different digest structure than flat Set)
    assert_ne!(
        digest_nested, digest_flat,
        "Nested ForEach body must produce different digest than flat Set body"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// B16: Body step primitive type diversity at workflow level (canonical_digest)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn foreach_body_set_vs_finish_produces_different_workflow_digest() {
    // Given: Two WorkflowSources — ForEach with Set body vs Finish body
    let body_set = vec![set_body_step("s1", "x", "1")];
    let body_finish = vec![finish_body_step("s1", 42)];

    let source_set = foreach_workflow("item".to_string(), "items".to_string(), None, body_set);
    let source_finish =
        foreach_workflow("item".to_string(), "items".to_string(), None, body_finish);

    // When: canonical_digest is called
    let digest_set = canonical_digest_part05(&source_set);
    let digest_finish = canonical_digest_part05(&source_finish);

    // Then: Different (primitive type difference propagates to workflow digest)
    assert_ne!(
        digest_set, digest_finish,
        "ForEach with Set body must produce different workflow digest than Finish body"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Additional edge case: ForEach body with Finish using String result
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn foreach_body_finish_string_result_differs_from_integer_result() {
    // Given: Two ForEach steps — body Finish with String(42) vs Integer(42)
    let body_str = vec![StepAst {
        id: "s1".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Finish {
            result: ScalarValue::String("42".to_string()),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }];
    let body_int = vec![finish_body_step("s1", 42)];

    let foreach_str = foreach_step("item", "items", None, body_str);
    let foreach_int = foreach_step("item", "items", None, body_int);

    // When: digests computed
    let digest_str = step_digest_bytes(&foreach_str);
    let digest_int = step_digest_bytes(&foreach_int);

    // Then: Different (String hashes .as_bytes(), Integer hashes .to_le_bytes())
    assert_ne!(
        digest_str, digest_int,
        "ForEach body Finish with String result must differ from Integer result"
    );
}
