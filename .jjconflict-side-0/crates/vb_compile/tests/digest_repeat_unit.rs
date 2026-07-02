//! Unit tests for Repeat digest coverage (bead vb-xi2f.31).
//!
//! Proof obligations: PO-008, PO-009, PO-010.
//!
//! These tests verify:
//! - PO-008: Different max_attempts → different WorkflowDigest
//! - PO-009: Different body steps → different WorkflowDigest
//! - PO-010: Same config → same WorkflowDigest (idempotency)
//!
//! All tests use the public `compile_workflow` and `compile_source` APIs.

use vb_compile::parse_workflow_source;
use vb_compile::{compile_source, compile_workflow};

const HEADER: &str =
    "version: velvet-ballistics/v1\nname: repeat_digest_unit\nwhen:\n  manual: {}\nsteps:\n";

fn workflow_yaml(steps: &str) -> String {
    let mut yaml = String::from(HEADER);
    yaml.push_str(steps);
    yaml
}

fn compile_workflow_from_steps(steps: &str) -> Result<vb_core::CompiledWorkflow, String> {
    let yaml = workflow_yaml(steps);
    compile_workflow(yaml.as_bytes()).map_err(|errors| {
        errors
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("; ")
    })
}

fn compile_source_from_steps(steps: &str) -> Result<vb_core::CompiledWorkflow, String> {
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
// PO-008: test_repeat_max_attempts_changes_digest
// =========================================================================

/// PO-008: Explicit test: repeat(3, bodyA) vs repeat(5, bodyA) produce
/// different WorkflowDigest values.
///
/// Non-vacuous: asserts inequality, not just that both compile.
#[test]
fn test_repeat_max_attempts_changes_digest() {
    let steps_3 = "  - id: retry\n    repeat:\n      max_attempts: 3\n      steps:\n        - id: attempt\n          set:\n            output: attempted\n            value: \"1\"\n  - id: done\n    finish:\n      result: 0\n";

    let steps_5 = "  - id: retry\n    repeat:\n      max_attempts: 5\n      steps:\n        - id: attempt\n          set:\n            output: attempted\n            value: \"1\"\n  - id: done\n    finish:\n      result: 0\n";

    let wf3 = compile_workflow_from_steps(steps_3).expect("repeat max_attempts=3 should compile");
    let wf5 = compile_workflow_from_steps(steps_5).expect("repeat max_attempts=5 should compile");

    assert_ne!(
        wf3.digest(),
        wf5.digest(),
        "repeat max_attempts 3 vs 5 must produce different WorkflowDigest"
    );
}

/// PO-008 extended: Same test through compile_source path.
#[test]
fn test_repeat_max_attempts_changes_digest_compile_source() {
    let steps_3 = "  - id: retry\n    repeat:\n      max_attempts: 3\n      steps:\n        - id: attempt\n          set:\n            output: attempted\n            value: \"1\"\n  - id: done\n    finish:\n      result: 0\n";

    let steps_5 = "  - id: retry\n    repeat:\n      max_attempts: 5\n      steps:\n        - id: attempt\n          set:\n            output: attempted\n            value: \"1\"\n  - id: done\n    finish:\n      result: 0\n";

    let wf3 = compile_source_from_steps(steps_3)
        .expect("repeat max_attempts=3 should compile via compile_source");
    let wf5 = compile_source_from_steps(steps_5)
        .expect("repeat max_attempts=5 should compile via compile_source");

    assert_ne!(
        wf3.digest(),
        wf5.digest(),
        "repeat max_attempts 3 vs 5 must produce different WorkflowDigest (compile_source)"
    );
}

// =========================================================================
// PO-009: test_repeat_body_changes_digest
// =========================================================================

/// PO-009: Explicit test: repeat(3, bodyA) vs repeat(3, bodyB) produce
/// different WorkflowDigest values when inner Set step values differ.
///
/// Both bodies contain a single Set step (required by lowering validation),
/// but with different output/value fields.
/// Non-vacuous: asserts inequality.
#[test]
fn test_repeat_body_changes_digest() {
    // Single Set: output=seen, value="1"
    let steps_set = "  - id: retry\n    repeat:\n      max_attempts: 3\n      steps:\n        - id: a_set\n          set:\n            output: seen\n            value: \"1\"\n  - id: done\n    finish:\n      result: 0\n";

    // Single Set: output=out99, value="99" — different Set content
    let steps_diff = "  - id: retry\n    repeat:\n      max_attempts: 3\n      steps:\n        - id: a_set\n          set:\n            output: out99\n            value: \"99\"\n  - id: done\n    finish:\n      result: 0\n";

    let wf_set = compile_workflow_from_steps(steps_set)
        .expect("repeat with single Set body (value=1) should compile");
    let wf_diff = compile_workflow_from_steps(steps_diff)
        .expect("repeat with single Set body (value=99) should compile");

    assert_ne!(
        wf_set.digest(),
        wf_diff.digest(),
        "repeat body with different Set values must produce different WorkflowDigest"
    );
}

/// PO-009 extended: Same test through compile_source path.
#[test]
fn test_repeat_body_changes_digest_compile_source() {
    let steps_set = "  - id: retry\n    repeat:\n      max_attempts: 3\n      steps:\n        - id: a_set\n          set:\n            output: seen\n            value: \"1\"\n  - id: done\n    finish:\n      result: 0\n";

    let steps_diff = "  - id: retry\n    repeat:\n      max_attempts: 3\n      steps:\n        - id: a_set\n          set:\n            output: out99\n            value: \"99\"\n  - id: done\n    finish:\n      result: 0\n";

    let wf_set = compile_source_from_steps(steps_set)
        .expect("repeat with single Set body (value=1) should compile via compile_source");
    let wf_diff = compile_source_from_steps(steps_diff)
        .expect("repeat with single Set body (value=99) should compile via compile_source");

    assert_ne!(
        wf_set.digest(),
        wf_diff.digest(),
        "repeat body with different Set values must produce different WorkflowDigest (compile_source)"
    );
}

// =========================================================================
// PO-008b: Boundary tests for max_attempts
// =========================================================================

/// PO-008 boundary: max_attempts=2 vs max_attempts=1 produce different digests.
///
/// Proves that distinct low-attempt-count repeats are digest-distinct.
/// Note: max_attempts=0 is rejected at the YAML validation layer as non-empty,
/// so the minimum valid value is 1.
#[test]
fn test_repeat_max_attempts_two_differs_from_one() {
    let steps_two = "  - id: retry\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: attempt\n          set:\n            output: attempted\n            value: \"1\"\n  - id: done\n    finish:\n      result: 0\n";

    let steps_one = "  - id: retry\n    repeat:\n      max_attempts: 1\n      steps:\n        - id: attempt\n          set:\n            output: attempted\n            value: \"1\"\n  - id: done\n    finish:\n      result: 0\n";

    let wf2 = compile_workflow_from_steps(steps_two).expect("repeat max_attempts=2 should compile");
    let wf1 = compile_workflow_from_steps(steps_one).expect("repeat max_attempts=1 should compile");

    assert_ne!(
        wf2.digest(),
        wf1.digest(),
        "repeat max_attempts 2 vs 1 must produce different WorkflowDigest"
    );
}

/// PO-008 boundary: max_attempts=u16::MAX vs max_attempts=1 produce
/// different digests.
#[test]
fn test_repeat_max_attempts_max_differs_from_one() {
    let steps_max = "  - id: retry\n    repeat:\n      max_attempts: 65535\n      steps:\n        - id: attempt\n          set:\n            output: attempted\n            value: \"1\"\n  - id: done\n    finish:\n      result: 0\n";

    let steps_one = "  - id: retry\n    repeat:\n      max_attempts: 1\n      steps:\n        - id: attempt\n          set:\n            output: attempted\n            value: \"1\"\n  - id: done\n    finish:\n      result: 0\n";

    let wf_max =
        compile_workflow_from_steps(steps_max).expect("repeat max_attempts=65535 should compile");
    let wf1 = compile_workflow_from_steps(steps_one).expect("repeat max_attempts=1 should compile");

    assert_ne!(
        wf_max.digest(),
        wf1.digest(),
        "repeat max_attempts u16::MAX vs 1 must produce different WorkflowDigest"
    );
}

/// PO-008 boundary: max_attempts=2 vs max_attempts=u16::MAX produce
/// different digests (extremal pair with valid min).
#[test]
fn test_repeat_max_attempts_two_differs_from_max() {
    let steps_two = "  - id: retry\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: attempt\n          set:\n            output: attempted\n            value: \"1\"\n  - id: done\n    finish:\n      result: 0\n";

    let steps_max = "  - id: retry\n    repeat:\n      max_attempts: 65535\n      steps:\n        - id: attempt\n          set:\n            output: attempted\n            value: \"1\"\n  - id: done\n    finish:\n      result: 0\n";

    let wf2 = compile_workflow_from_steps(steps_two).expect("repeat max_attempts=2 should compile");
    let wf_max =
        compile_workflow_from_steps(steps_max).expect("repeat max_attempts=65535 should compile");

    assert_ne!(
        wf2.digest(),
        wf_max.digest(),
        "repeat max_attempts 2 vs u16::MAX must produce different WorkflowDigest"
    );
}

// =========================================================================
// PO-009b: Multi-step body rejection test
// =========================================================================

/// PO-009 extended: Multi-step Repeat body must be rejected by lowering
/// with StepFieldShape error (exactly one set step required).
///
/// The compiler validates that Repeat body has exactly one Set step.
/// Multi-step bodies are rejected at lowering time.
#[test]
fn test_repeat_multi_step_body_rejected() {
    let steps_multi = "  - id: retry\n    repeat:\n      max_attempts: 3\n      steps:\n        - id: a\n          set:\n            output: out1\n            value: \"10\"\n        - id: b\n          set:\n            output: out2\n            value: \"20\"\n        - id: c\n          set:\n            output: out3\n            value: \"30\"\n  - id: done\n    finish:\n      result: 0\n";

    let result = compile_workflow_from_steps(steps_multi);

    assert!(
        result.is_err(),
        "multi-step repeat body must be rejected by lowering validation"
    );
    let err = result.unwrap_err();
    assert!(
        err.contains("exactly one set step") || err.contains("step"),
        "multi-step body rejection error must mention step constraint, got: {err}"
    );
}

// =========================================================================
// PO-010: test_repeat_same_config_same_digest
// =========================================================================

/// PO-010: Explicit test: repeat(3, bodyA) vs repeat(3, bodyA) produce
/// same WorkflowDigest (idempotency preservation).
#[test]
fn test_repeat_same_config_same_digest() {
    let steps = "  - id: retry\n    repeat:\n      max_attempts: 3\n      steps:\n        - id: attempt\n          set:\n            output: attempted\n            value: \"1\"\n  - id: done\n    finish:\n      result: 0\n";

    let wf1 = compile_workflow_from_steps(steps).expect("first compile should succeed");
    let wf2 = compile_workflow_from_steps(steps).expect("second compile should succeed");

    assert_eq!(
        wf1.digest(),
        wf2.digest(),
        "identical repeat config must produce identical WorkflowDigest"
    );
}

/// PO-010 extended: Same test through compile_source path.
#[test]
fn test_repeat_same_config_same_digest_compile_source() {
    let steps = "  - id: retry\n    repeat:\n      max_attempts: 3\n      steps:\n        - id: attempt\n          set:\n            output: attempted\n            value: \"1\"\n  - id: done\n    finish:\n      result: 0\n";

    let wf1 =
        compile_source_from_steps(steps).expect("first compile via compile_source should succeed");
    let wf2 =
        compile_source_from_steps(steps).expect("second compile via compile_source should succeed");

    assert_eq!(
        wf1.digest(),
        wf2.digest(),
        "identical repeat config must produce identical WorkflowDigest (compile_source)"
    );
}

// =========================================================================
// PO-009: B-008 body content sensitivity
// =========================================================================

/// PO-009 / B-008: Different single-step Set output fields produce
/// different digests even with same max_attempts.
///
/// Repeat configs with same max_attempts but different Set output names
/// must produce different WorkflowDigest values.
#[test]
fn test_repeat_different_set_output_changes_digest() {
    // Set output=out1, value="10"
    let steps_out1 = "  - id: retry\n    repeat:\n      max_attempts: 3\n      steps:\n        - id: step_a\n          set:\n            output: out1\n            value: \"10\"\n  - id: done\n    finish:\n      result: 0\n";

    // Set output=out2, value="10" — only output field differs
    let steps_out2 = "  - id: retry\n    repeat:\n      max_attempts: 3\n      steps:\n        - id: step_a\n          set:\n            output: out2\n            value: \"10\"\n  - id: done\n    finish:\n      result: 0\n";

    let wf_out1 =
        compile_workflow_from_steps(steps_out1).expect("repeat body [out1] should compile");
    let wf_out2 =
        compile_workflow_from_steps(steps_out2).expect("repeat body [out2] should compile");

    assert_ne!(
        wf_out1.digest(),
        wf_out2.digest(),
        "different Set output fields in repeat body must produce different WorkflowDigest"
    );
}

// =========================================================================
// PO-008 / B-007: Genuine empty repeat body (zero body steps)
// =========================================================================

/// PO-008 / B-007: Repeat with empty body (body: []) is rejected at
/// lowering with StepFieldShape because `emit_single_body_set` requires
/// at least one Set step.
#[test]
fn test_repeat_empty_body_rejected_with_step_field_shape() {
    let steps_empty = "  - id: retry\n    repeat:\n      max_attempts: 3\n      steps: []\n  - id: done\n    finish:\n      result: 0\n";

    let result = compile_workflow_from_steps(steps_empty);

    assert!(
        result.is_err(),
        "repeat with empty body (body: []) must fail to compile"
    );
    let err = result.unwrap_err();
    assert!(
        err.contains("step"),
        "empty repeat body error must mention 'step', got: {err}"
    );
}

/// PO-010 extended: Same config cross-path (compile_workflow vs compile_source).
#[test]
fn test_repeat_same_config_same_digest_cross_path() {
    let steps = "  - id: retry\n    repeat:\n      max_attempts: 3\n      steps:\n        - id: attempt\n          set:\n            output: attempted\n            value: \"1\"\n  - id: done\n    finish:\n      result: 0\n";

    let wf_wf = compile_workflow_from_steps(steps).expect("compile_workflow should succeed");
    let wf_src = compile_source_from_steps(steps).expect("compile_source should succeed");

    assert_eq!(
        wf_wf.digest(),
        wf_src.digest(),
        "compile_workflow and compile_source must produce identical digest for same input"
    );
}
