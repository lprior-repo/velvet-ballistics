// Behavior tests: Set and Finish regression verification (B7, B13, B14)
// Bead: vb-xi2f.33 / P1: digest covers ask semantics
//
// Verifies INV-ASK-007: Set and Finish digest behavior is unchanged after Ask fix.
// Verifies POS-007: No regression for existing primitives.
// Verifies TC-005: Finish and Set arms continue to hash correctly.

#![forbid(unsafe_code)]

mod common;
use common::{finish_source_integer, finish_source_string, set_finish_source, set_source};
use vb_compile::canonical_digest;

// ── B7 / B14: Set primitive regression ──

#[test]
fn canonical_digest_produces_valid_digest_for_set_only_source() {
    // Given: source with only a Set step
    let source = set_source("x", "1");
    // When
    let digest = canonical_digest(&source).expect("valid test input");
    // Then: valid 32-byte digest, no panic
    assert_eq!(
        digest.as_bytes().len(),
        32,
        "INV-ASK-007: Set-only source must produce valid 32-byte digest"
    );
}

#[test]
fn canonical_digest_is_deterministic_for_set_only_source() {
    // Given: Set-only source
    let source = set_source("x", "1");
    // When: called twice
    let digest_a = canonical_digest(&source).expect("valid test input");
    let digest_b = canonical_digest(&source).expect("valid test input");
    // Then: deterministic
    assert_eq!(
        digest_a, digest_b,
        "INV-ASK-007: Set-only digest must be deterministic"
    );
}

#[test]
fn canonical_digest_produces_distinct_digests_when_set_output_differs() {
    // Given: Set sources with different output names
    let source_a = set_source("x", "1");
    let source_b = set_source("y", "1");
    // When
    let digest_a = canonical_digest(&source_a).expect("valid test input");
    let digest_b = canonical_digest(&source_b).expect("valid test input");
    // Then: different output → different digest
    assert_ne!(
        digest_a, digest_b,
        "TC-005: different Set output names must produce distinct digests"
    );
}

#[test]
fn canonical_digest_produces_distinct_digests_when_set_value_differs() {
    // Given: Set sources with same output, different values
    let source_a = set_source("x", "1");
    let source_b = set_source("x", "2");
    // When
    let digest_a = canonical_digest(&source_a).expect("valid test input");
    let digest_b = canonical_digest(&source_b).expect("valid test input");
    // Then: different value → different digest
    assert_ne!(
        digest_a, digest_b,
        "TC-005: different Set values must produce distinct digests"
    );
}

// ── B7 / B13: Finish primitive regression ──

#[test]
fn canonical_digest_produces_valid_digest_for_finish_string_only_source() {
    // Given: source with only a Finish(String) step
    let source = finish_source_string("done");
    // When
    let digest = canonical_digest(&source).expect("valid test input");
    // Then: valid 32-byte digest
    assert_eq!(
        digest.as_bytes().len(),
        32,
        "INV-ASK-007: Finish(String)-only source must produce valid 32-byte digest"
    );
}

#[test]
fn canonical_digest_produces_valid_digest_for_finish_integer_only_source() {
    // Given: source with only a Finish(Integer) step
    let source = finish_source_integer(0);
    // When
    let digest = canonical_digest(&source).expect("valid test input");
    // Then: valid 32-byte digest
    assert_eq!(
        digest.as_bytes().len(),
        32,
        "INV-ASK-007: Finish(Integer)-only source must produce valid 32-byte digest"
    );
}

#[test]
fn canonical_digest_is_deterministic_for_finish_source() {
    // Given: Finish source
    let source = finish_source_string("result");
    // When: called twice
    let digest_a = canonical_digest(&source).expect("valid test input");
    let digest_b = canonical_digest(&source).expect("valid test input");
    // Then: deterministic
    assert_eq!(
        digest_a, digest_b,
        "INV-ASK-007: Finish digest must be deterministic"
    );
}

#[test]
fn canonical_digest_produces_distinct_digests_when_finish_result_string_differs() {
    // Given: Finish(String) with different values
    let source_a = finish_source_string("done_a");
    let source_b = finish_source_string("done_b");
    // When
    let digest_a = canonical_digest(&source_a).expect("valid test input");
    let digest_b = canonical_digest(&source_b).expect("valid test input");
    // Then: different results → different digests
    assert_ne!(
        digest_a, digest_b,
        "TC-005: different Finish(String) results must produce distinct digests"
    );
}

#[test]
fn canonical_digest_produces_distinct_digests_when_finish_result_integer_differs() {
    // Given: Finish(Integer) with different values
    let source_a = finish_source_integer(0);
    let source_b = finish_source_integer(1);
    // When
    let digest_a = canonical_digest(&source_a).expect("valid test input");
    let digest_b = canonical_digest(&source_b).expect("valid test input");
    // Then: different results → different digests
    assert_ne!(
        digest_a, digest_b,
        "TC-005: different Finish(Integer) results must produce distinct digests"
    );
}

#[test]
fn canonical_digest_finish_string_vs_integer_produce_distinct_digests() {
    // Given: Finish(String) vs Finish(Integer) with different semantic type
    let source_a = finish_source_string("0");
    let source_b = finish_source_integer(0);
    // When
    let digest_a = canonical_digest(&source_a).expect("valid test input");
    let digest_b = canonical_digest(&source_b).expect("valid test input");
    // Then: different ScalarValue types → different digests
    // Note: Finish hashing differs for String (as_bytes) vs Integer (to_le_bytes)
    assert_ne!(
        digest_a, digest_b,
        "TC-005: Finish(String) and Finish(Integer) must produce distinct digests"
    );
}

#[test]
fn canonical_digest_is_deterministic_for_set_finish_source() {
    // Given: Set+Finish source
    let source = set_finish_source();
    // When: called three times
    let d1 = canonical_digest(&source).expect("valid test input");
    let d2 = canonical_digest(&source).expect("valid test input");
    let d3 = canonical_digest(&source).expect("valid test input");
    // Then: all identical
    assert_eq!(d1, d2);
    assert_eq!(d1, d3);
}

// ── B7: Set → Finish order sensitivity ──

#[test]
fn canonical_digest_produces_distinct_digests_when_step_order_differs_set_vs_finish() {
    // Given: source with Set then Finish
    let source_a = set_finish_source();
    // And: source with Finish then Set (reversed order)
    use vb_compile::{
        ScalarValue, StepAst, StepPrimitive, TriggerAst, WorkflowSource, WorkflowSourceParts,
    };
    let source_b = WorkflowSource::new(WorkflowSourceParts {
        version: "velvet-ballistics/v1".to_string(),
        name: "test_set_finish_workflow".to_string(),
        trigger: TriggerAst::Manual,
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![
            StepAst {
                id: "finish_1".to_string(),
                name: None,
                condition: None,
                primitive: StepPrimitive::Finish {
                    result: ScalarValue::String("done".to_string()),
                },
                with: None,
                retry: None,
                on_error: None,
                then: None,
            },
            StepAst {
                id: "set_1".to_string(),
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
            },
        ],
        result: None,
        examples: vec![],
    });
    // When
    let digest_a = canonical_digest(&source_a).expect("valid test input");
    let digest_b = canonical_digest(&source_b).expect("valid test input");
    // Then: different order → different digest (step order matters for hash)
    assert_ne!(
        digest_a, digest_b,
        "Step order matters: Set→Finish must produce different digest than Finish→Set"
    );
}
