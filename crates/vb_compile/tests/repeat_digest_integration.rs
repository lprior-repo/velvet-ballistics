#![allow(clippy::expect_used)]
//! Integration tests for Repeat digest coverage (bead vb-xi2f.31).
//!
//! Proof obligations: PO-011, PO-012.
//!
//! These tests verify end-to-end that both compile paths produce correct
//! repeat-aware digests:
//! - PO-011: compile_workflow path (YamlCompiler::compile → compile_source)
//! - PO-012: compile_source path (direct AST → IR)
//!
//! Each test uses real YAML fixtures and asserts that different repeat
//! configurations produce different WorkflowDigest values.

use vb_compile::{compile_source, compile_workflow};
use vb_yaml::parse_workflow_source;

const HEADER: &str =
    "version: velvet-ballastics/v1\nname: repeat-digest-integration\nwhen:\n  manual: {}\nsteps:\n";

fn workflow_yaml(steps: &str) -> String {
    let mut yaml = String::from(HEADER);
    yaml.push_str(steps);
    yaml
}

fn compile_workflow_from_yaml(steps: &str) -> Result<vb_core::CompiledWorkflow, String> {
    let yaml = workflow_yaml(steps);
    compile_workflow(yaml.as_bytes()).map_err(|errors| {
        errors
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("; ")
    })
}

fn compile_source_from_yaml(steps: &str) -> Result<vb_core::CompiledWorkflow, String> {
    let yaml = workflow_yaml(steps);
    let source = parse_workflow_source(&yaml).map_err(|e| e.to_string())?;
    compile_source(&source).map_err(|errors| {
        errors
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("; ")
    })
}

// =========================================================================
// PO-011: test_compile_workflow_repeat_digest
// =========================================================================

/// PO-011 / INTEGRATION-REPEAT-001: compile_workflow produces correct
/// repeat-aware digest.
///
/// End-to-end verification that the compile_workflow path embeds correct
/// repeat digest, with both max_attempts and body hashed.
#[test]
fn test_compile_workflow_repeat_digest() {
    // Workflow A: repeat max_attempts=3 with Set body
    let steps_a = "  - id: retry\n    repeat:\n      max_attempts: 3\n      steps:\n        - id: attempt_a\n          set:\n            output: attempted\n            value: \"42\"\n  - id: done\n    finish:\n      result: 0\n";

    // Workflow B: repeat max_attempts=7 with same Set body
    let steps_b = "  - id: retry\n    repeat:\n      max_attempts: 7\n      steps:\n        - id: attempt_b\n          set:\n            output: attempted\n            value: \"42\"\n  - id: done\n    finish:\n      result: 0\n";

    // Workflow C: repeat max_attempts=3 with different Set body value
    // (single-step body as required by lowering validation)
    let steps_c = "  - id: retry\n    repeat:\n      max_attempts: 3\n      steps:\n        - id: attempt_c\n          set:\n            output: attempted\n            value: \"99\"\n  - id: done\n    finish:\n      result: 0\n";

    let wf_a = compile_workflow_from_yaml(steps_a).expect("workflow A should compile");
    let wf_b = compile_workflow_from_yaml(steps_b).expect("workflow B should compile");
    let wf_c = compile_workflow_from_yaml(steps_c).expect("workflow C should compile");

    // Different max_attempts → different digest
    assert_ne!(
        wf_a.digest(),
        wf_b.digest(),
        "compile_workflow: different max_attempts (3 vs 7) must produce different digests"
    );

    // Different body value → different digest (same max_attempts)
    assert_ne!(
        wf_a.digest(),
        wf_c.digest(),
        "compile_workflow: different body Set value (42 vs 99) must produce different digests"
    );

    // Different max_attempts AND different body → different digest
    assert_ne!(
        wf_b.digest(),
        wf_c.digest(),
        "compile_workflow: both max_attempts and body differ must produce different digests"
    );
}

/// PO-011 extended: compile_workflow with minimal repeat body.
#[test]
fn test_compile_workflow_repeat_digest_empty_body() {
    let steps_max3 = "  - id: retry\n    repeat:\n      max_attempts: 3\n      steps:\n        - id: inner\n          set:\n            output: a\n            value: \"1\"\n  - id: done\n    finish:\n      result: 0\n";

    let steps_max5 = "  - id: retry\n    repeat:\n      max_attempts: 5\n      steps:\n        - id: inner\n          set:\n            output: a\n            value: \"1\"\n  - id: done\n    finish:\n      result: 0\n";

    let wf3 = compile_workflow_from_yaml(steps_max3)
        .expect("repeat max_attempts=3 empty body should compile");
    let wf5 = compile_workflow_from_yaml(steps_max5)
        .expect("repeat max_attempts=5 empty body should compile");

    assert_ne!(
        wf3.digest(),
        wf5.digest(),
        "compile_workflow: different max_attempts with empty body must produce different digests"
    );
}

/// PO-011 extended: compile_workflow idempotency (same config → same digest).
#[test]
fn test_compile_workflow_repeat_digest_idempotent() {
    let steps = "  - id: retry\n    repeat:\n      max_attempts: 3\n      steps:\n        - id: attempt\n          set:\n            output: attempted\n            value: \"1\"\n  - id: done\n    finish:\n      result: 0\n";

    let wf1 = compile_workflow_from_yaml(steps).expect("first compile_workflow should succeed");
    let wf2 = compile_workflow_from_yaml(steps).expect("second compile_workflow should succeed");

    assert_eq!(
        wf1.digest(),
        wf2.digest(),
        "compile_workflow: identical repeat config must produce identical digest"
    );
}

// =========================================================================
// PO-012: test_compile_source_repeat_digest
// =========================================================================

/// PO-012 / INTEGRATION-REPEAT-002: compile_source produces correct
/// repeat-aware digest.
///
/// End-to-end verification that the compile_source path (via part_01.rs →
/// part_05.rs) embeds correct repeat digest.
#[test]
fn test_compile_source_repeat_digest() {
    // Workflow A: repeat max_attempts=3 with Set body
    let steps_a = "  - id: retry\n    repeat:\n      max_attempts: 3\n      steps:\n        - id: attempt_a\n          set:\n            output: attempted\n            value: \"42\"\n  - id: done\n    finish:\n      result: 0\n";

    // Workflow B: repeat max_attempts=7 with same Set body
    let steps_b = "  - id: retry\n    repeat:\n      max_attempts: 7\n      steps:\n        - id: attempt_b\n          set:\n            output: attempted\n            value: \"42\"\n  - id: done\n    finish:\n      result: 0\n";

    // Workflow C: repeat max_attempts=3 with different Set body value
    // (single-step body as required by lowering validation)
    let steps_c = "  - id: retry\n    repeat:\n      max_attempts: 3\n      steps:\n        - id: attempt_c\n          set:\n            output: attempted\n            value: \"99\"\n  - id: done\n    finish:\n      result: 0\n";

    let wf_a =
        compile_source_from_yaml(steps_a).expect("workflow A should compile via compile_source");
    let wf_b =
        compile_source_from_yaml(steps_b).expect("workflow B should compile via compile_source");
    let wf_c =
        compile_source_from_yaml(steps_c).expect("workflow C should compile via compile_source");

    // Different max_attempts → different digest
    assert_ne!(
        wf_a.digest(),
        wf_b.digest(),
        "compile_source: different max_attempts (3 vs 7) must produce different digests"
    );

    // Different body value → different digest
    assert_ne!(
        wf_a.digest(),
        wf_c.digest(),
        "compile_source: different body Set value (42 vs 99) must produce different digests"
    );
}

/// PO-012 extended: compile_source with empty repeat body.
#[test]
fn test_compile_source_repeat_digest_empty_body() {
    let steps_max3 = "  - id: retry\n    repeat:\n      max_attempts: 3\n      steps:\n        - id: inner\n          set:\n            output: a\n            value: \"1\"\n  - id: done\n    finish:\n      result: 0\n";

    let steps_max5 = "  - id: retry\n    repeat:\n      max_attempts: 5\n      steps:\n        - id: inner\n          set:\n            output: a\n            value: \"1\"\n  - id: done\n    finish:\n      result: 0\n";

    let wf3 = compile_source_from_yaml(steps_max3)
        .expect("repeat max_attempts=3 empty body should compile via compile_source");
    let wf5 = compile_source_from_yaml(steps_max5)
        .expect("repeat max_attempts=5 empty body should compile via compile_source");

    assert_ne!(
        wf3.digest(),
        wf5.digest(),
        "compile_source: different max_attempts with empty body must produce different digests"
    );
}

/// PO-012 extended: compile_source idempotency.
#[test]
fn test_compile_source_repeat_digest_idempotent() {
    let steps = "  - id: retry\n    repeat:\n      max_attempts: 3\n      steps:\n        - id: attempt\n          set:\n            output: attempted\n            value: \"1\"\n  - id: done\n    finish:\n      result: 0\n";

    let wf1 = compile_source_from_yaml(steps).expect("first compile_source should succeed");
    let wf2 = compile_source_from_yaml(steps).expect("second compile_source should succeed");

    assert_eq!(
        wf1.digest(),
        wf2.digest(),
        "compile_source: identical repeat config must produce identical digest"
    );
}

/// PO-011 extended: compile_workflow with max_attempts at maximum u16 boundary.
#[test]
fn test_compile_workflow_repeat_digest_max_u16_boundary() {
    let steps_max = "  - id: retry\n    repeat:\n      max_attempts: 65535\n      steps:\n        - id: attempt\n          set:\n            output: attempted\n            value: \"1\"\n  - id: done\n    finish:\n      result: 0\n";

    let wf1 =
        compile_workflow_from_yaml(steps_max).expect("repeat max_attempts=65535 should compile");
    let wf2 =
        compile_workflow_from_yaml(steps_max).expect("second compile at u16::MAX should succeed");

    // Idempotency: same boundary value compiled twice → same digest
    assert_eq!(
        wf1.digest(),
        wf2.digest(),
        "compile_workflow: identical max_attempts=u16::MAX configs must produce identical digest"
    );
}

/// PO-011 extended: repeat body with composite step triggers digest change
/// when inner output names differ.
#[test]
fn test_compile_workflow_repeat_digest_identical_body_different_output_names() {
    let steps_a = "  - id: retry\n    repeat:\n      max_attempts: 3\n      steps:\n        - id: s1\n          set:\n            output: alpha\n            value: \"1\"\n  - id: done\n    finish:\n      result: 0\n";

    let steps_b = "  - id: retry\n    repeat:\n      max_attempts: 3\n      steps:\n        - id: s1\n          set:\n            output: beta\n            value: \"1\"\n  - id: done\n    finish:\n      result: 0\n";

    let wf_a = compile_workflow_from_yaml(steps_a).expect("output=alpha should compile");
    let wf_b = compile_workflow_from_yaml(steps_b).expect("output=beta should compile");

    assert_ne!(
        wf_a.digest(),
        wf_b.digest(),
        "compile_workflow: different output names in repeat body must produce different digests"
    );
}

/// PO-012 extended: compile_source with max_attempts at minimum valid value (1).
/// Note: max_attempts=0 is rejected at the YAML validation layer.
#[test]
fn test_compile_source_repeat_digest_min_attempts() {
    let steps_min = "  - id: retry\n    repeat:\n      max_attempts: 1\n      steps:\n        - id: attempt\n          set:\n            output: attempted\n            value: \"1\"\n  - id: done\n    finish:\n      result: 0\n";

    let wf1 = compile_source_from_yaml(steps_min)
        .expect("repeat max_attempts=1 should compile via compile_source");
    let wf2 = compile_source_from_yaml(steps_min)
        .expect("second compile of max_attempts=1 should succeed");

    assert_eq!(
        wf1.digest(),
        wf2.digest(),
        "compile_source: identical max_attempts=1 configs must produce identical digest"
    );
}

/// Cross-path verification: compile_workflow and compile_source must agree.
#[test]
fn test_repeat_digest_cross_path_equivalent() {
    let steps = "  - id: retry\n    repeat:\n      max_attempts: 3\n      steps:\n        - id: attempt\n          set:\n            output: attempted\n            value: \"1\"\n  - id: done\n    finish:\n      result: 0\n";

    let wf_workflow = compile_workflow_from_yaml(steps).expect("compile_workflow should succeed");
    let wf_source = compile_source_from_yaml(steps).expect("compile_source should succeed");

    assert_eq!(
        wf_workflow.digest(),
        wf_source.digest(),
        "compile_workflow and compile_source must produce identical digest"
    );
}
