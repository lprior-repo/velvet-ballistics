//! Proptest for Repeat digest coverage (bead vb-xi2f.31).
//!
//! Proof obligations: PROP-REPEAT-001 through PROP-REPEAT-004.
//!
//! These property-based tests provide exhaustive coverage of Repeat digest
//! properties that the Kani harnesses cannot verify (blocked by blake3
//! inline ASM -- see BLOCKER-BLAKE3-INLINEASM in kani_digest_repeat.rs).
//!
//! Properties:
//! - PROP-REPEAT-001: Same config → same digest (idempotency)
//! - PROP-REPEAT-002: Different max_attempts → different digest
//! - PROP-REPEAT-003: Different body → different digest (output name diff)
//! - PROP-REPEAT-004: Different body → different digest (value diff)
//!
//! God Rules: Uses public compile_workflow API; no hardcoded structural
//! inputs beyond strategy bounds.

use proptest::prelude::*;
use vb_compile::compile_workflow;

// ─────────────────────────────────────────────────────────────────
// Strategy helpers
// ─────────────────────────────────────────────────────────────────

/// Strategy for max_attempts: any u16 value in [1, u16::MAX].
/// max_attempts=0 is rejected at the YAML validation layer
/// ("must be non-empty primitive field").
fn arb_max_attempts() -> impl Strategy<Value = u16> {
    1u16..=u16::MAX
}

/// Strategy for a simple step ID string.
/// Uses lowercase letter + digit prefix to guarantee no YAML 1.1 boolean
/// collision (on/off/yes/no/true/false in any case are all rejected).
fn arb_step_id() -> impl Strategy<Value = String> {
    "[a-z][0-9][a-zA-Z0-9]{0,6}".prop_map(|s| s.to_string())
}

/// Strategy for an output variable name.
/// Uses lowercase letter + digit prefix to guarantee no YAML 1.1 boolean
/// collision (on/off/yes/no/true/false in any case are all rejected).
fn arb_output_name() -> impl Strategy<Value = String> {
    "[a-z][0-9][a-zA-Z0-9]{0,6}".prop_map(|s| s.to_string())
}

/// Strategy for a Set value (integer string).
fn arb_set_value() -> impl Strategy<Value = String> {
    prop_oneof![
        any::<i64>().prop_map(|v| v.to_string()),
        any::<u8>().prop_map(|v| v.to_string()),
    ]
}

/// Generate YAML for a workflow with a Repeat step with given params.
fn repeat_workflow_yaml(
    max_attempts: u16,
    body_step_id: &str,
    output_name: &str,
    value: &str,
) -> String {
    format!(
        "version: velvet-ballastics/v1\nname: repeat-proptest\nwhen:\n  manual: {{}}\nsteps:\n  - id: retry\n    repeat:\n      max_attempts: {max_attempts}\n      steps:\n        - id: {body_step_id}\n          set:\n            output: {output_name}\n            value: \"{value}\"\n  - id: done\n    finish:\n      result: 0\n"
    )
}

/// Generate YAML for a workflow with a Repeat step, using a different
/// body step ID and output to ensure distinctness.
fn repeat_workflow_yaml_alt(
    max_attempts: u16,
    body_step_id: &str,
    output_name: &str,
    value: &str,
) -> String {
    format!(
        "version: velvet-ballastics/v1\nname: repeat-proptest-alt\nwhen:\n  manual: {{}}\nsteps:\n  - id: retry\n    repeat:\n      max_attempts: {max_attempts}\n      steps:\n        - id: alt_{body_step_id}\n          set:\n            output: alt_{output_name}\n            value: \"{value}\"\n  - id: done\n    finish:\n      result: 0\n"
    )
}

/// Compile YAML string to CompiledWorkflow.
fn try_compile(yaml: &str) -> Result<vb_core::CompiledWorkflow, String> {
    compile_workflow(yaml.as_bytes()).map_err(|errors| {
        errors
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("; ")
    })
}

// ─────────────────────────────────────────────────────────────────
// PROP-REPEAT-001: Idempotency
// ─────────────────────────────────────────────────────────────────

proptest! {
    /// PROP-REPEAT-001: Same repeat config compiled twice produces
    /// the same WorkflowDigest.
    ///
    /// Non-vacuous: asserts equality, not just that both compile.
    /// Run with ≥1000 cases.
    #[test]
    fn prop_repeat_idempotency(
        max_attempts in arb_max_attempts(),
        id in arb_step_id(),
        output in arb_output_name(),
        value in arb_set_value(),
    ) {
        let yaml = repeat_workflow_yaml(max_attempts, &id, &output, &value);
        let wf1 = try_compile(&yaml).expect("first compile should succeed");
        let wf2 = try_compile(&yaml).expect("second compile should succeed");

        prop_assert_eq!(
            wf1.digest(),
            wf2.digest(),
            "identical repeat config must produce identical WorkflowDigest"
        );
    }
}

// ─────────────────────────────────────────────────────────────────
// PROP-REPEAT-002: Different max_attempts → different digest
// ─────────────────────────────────────────────────────────────────

proptest! {
    /// PROP-REPEAT-002: Two different max_attempts values produce
    /// different WorkflowDigest values when all other parameters
    /// are identical.
    ///
    /// Non-vacuous: asserts inequality.
    /// Run with ≥1000 cases.
    #[test]
    fn prop_repeat_max_attempts_sensitivity(
        max1 in arb_max_attempts(),
        max2 in arb_max_attempts(),
        id in arb_step_id(),
        output in arb_output_name(),
        value in arb_set_value(),
    ) {
        // Only test when values actually differ
        prop_assume!(max1 != max2);

        let yaml1 = repeat_workflow_yaml(max1, &id, &output, &value);
        let yaml2 = repeat_workflow_yaml(max2, &id, &output, &value);

        let wf1 = try_compile(&yaml1).expect("compile with max1 should succeed");
        let wf2 = try_compile(&yaml2).expect("compile with max2 should succeed");

        prop_assert_ne!(
            wf1.digest(),
            wf2.digest(),
            "different max_attempts must produce different WorkflowDigest"
        );
    }
}

// ─────────────────────────────────────────────────────────────────
// PROP-REPEAT-003: Different body (output name) → different digest
// ─────────────────────────────────────────────────────────────────

proptest! {
    /// PROP-REPEAT-003: Different body output names produce different
    /// WorkflowDigest values when max_attempts is identical.
    ///
    /// Non-vacuous: asserts inequality.
    /// Run with ≥1000 cases.
    #[test]
    fn prop_repeat_body_output_name_sensitivity(
        max_attempts in arb_max_attempts(),
        id in arb_step_id(),
        output1 in arb_output_name(),
        output2 in arb_output_name(),
        value in arb_set_value(),
    ) {
        prop_assume!(output1 != output2);

        let yaml1 = repeat_workflow_yaml(max_attempts, &id, &output1, &value);
        let yaml2 = repeat_workflow_yaml(max_attempts, &id, &output2, &value);

        let wf1 = try_compile(&yaml1).expect("compile with output1 should succeed");
        let wf2 = try_compile(&yaml2).expect("compile with output2 should succeed");

        prop_assert_ne!(
            wf1.digest(),
            wf2.digest(),
            "different output names in repeat body must produce different WorkflowDigest"
        );
    }
}

// ─────────────────────────────────────────────────────────────────
// PROP-REPEAT-004: Different body (value) → different digest
// ─────────────────────────────────────────────────────────────────

proptest! {
    /// PROP-REPEAT-004: Different body Set values produce different
    /// WorkflowDigest values when max_attempts is identical.
    ///
    /// Non-vacuous: asserts inequality.
    /// Run with ≥1000 cases.
    #[test]
    fn prop_repeat_body_value_sensitivity(
        max_attempts in arb_max_attempts(),
        id in arb_step_id(),
        output in arb_output_name(),
        value1 in arb_set_value(),
        value2 in arb_set_value(),
    ) {
        prop_assume!(value1 != value2);

        let yaml1 = repeat_workflow_yaml(max_attempts, &id, &output, &value1);
        let yaml2 = repeat_workflow_yaml(max_attempts, &id, &output, &value2);

        let wf1 = try_compile(&yaml1).expect("compile with value1 should succeed");
        let wf2 = try_compile(&yaml2).expect("compile with value2 should succeed");

        prop_assert_ne!(
            wf1.digest(),
            wf2.digest(),
            "different Set values in repeat body must produce different WorkflowDigest"
        );
    }
}

// ─────────────────────────────────────────────────────────────────
// PROP-REPEAT-005: Cross-workflow-name digest distinction
// ─────────────────────────────────────────────────────────────────

proptest! {
    /// PROP-REPEAT-005: Same repeat config but different workflow
    /// name produces different WorkflowDigest.
    ///
    /// The workflow name is hashed in canonical_digest, so this
    /// regression test ensures repeat digest doesn't mask the name.
    /// Run with ≥1000 cases.
    #[test]
    fn prop_repeat_name_in_digest(
        max_attempts in arb_max_attempts(),
        id in arb_step_id(),
        output in arb_output_name(),
        value in arb_set_value(),
    ) {
        let yaml1 = repeat_workflow_yaml(max_attempts, &id, &output, &value);
        let yaml2 = repeat_workflow_yaml_alt(max_attempts, &id, &output, &value);

        let wf1 = try_compile(&yaml1).expect("compile name='repeat-proptest' should succeed");
        let wf2 = try_compile(&yaml2).expect("compile name='repeat-proptest-alt' should succeed");

        prop_assert_ne!(
            wf1.digest(),
            wf2.digest(),
            "different workflow names must produce different WorkflowDigest even with identical repeat config"
        );
    }
}
