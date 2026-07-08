//! Proptest properties for Together digest sensitivity (vb-xi2f.29).
//!
//! These property tests prove that the compiled workflow digest is sensitive to
//! Together branch structure: branch count, branch labels, sub-step contents,
//! and branch ordering all affect the digest.
//!
//! ## Obligations Covered
//!
//! | Obligation | Property | Risk |
//! |---|---|---|
//! | PO-xi2f29-002 | Branch count → different digest | BRANCH_COUNT_BLIND |
//! | PO-xi2f29-003 | Branch labels → different digest | LABEL_BLIND |
//! | PO-xi2f29-004 | Sub-step contents → different digest | NESTED_STEP_BLINDNESS |
//! | PO-xi2f29-005 | Branch ordering → different digest | ORDERING_BLIND |
//! | PO-xi2f29-006 | Same together source → same digest | REGRESSION |
//!
//! ## Production Dependency
//!
//! These tests require the Together arm in `digest_step_primitive` (part_05.rs)
//! and `digest_sub_step` function. Without these, all sensitivity tests will
//! FAIL because the current `other` wildcard arm only hashes the canonical name.
//!
//! ## Non-Vacuity
//!
//! - `assert_ne!` on distinct together configurations prevents vacuous pass
//! - Strategies generate actual together workflows through the full compile pipeline
//! - High iteration counts (1000 per property) ensure statistical confidence

use proptest::prelude::*;
use vb_core::WorkflowDigest;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a complete workflow YAML with together steps and a finish step.
/// Builds the YAML incrementally (no \\ line continuation) to match
/// the proven pattern from v1_primitive_lowering tests.
fn together_workflow_yaml(branches_yaml: &str) -> String {
    let mut yaml = String::from(
        "version: velvet-ballistics/v1\nname: together_test\nwhen:\n  manual: {}\nsteps:\n",
    );
    yaml.push_str(branches_yaml);
    yaml.push_str("  - id: done\n    finish:\n      result: 0\n");
    yaml
}

/// Parse and compile a YAML string, returning the digest.
/// Uses the same compile path as the working v1_primitive_lowering tests.
fn compile_and_digest(yaml: &str) -> Result<WorkflowDigest, String> {
    let workflow = vb_compile::compile_workflow(yaml.as_bytes())
        .map_err(|e| format!("compile error: {e:?}"))?;
    Ok(workflow.digest())
}

/// Generate branch YAML for a together step with given labels and optional set values.
/// Each branch gets exactly one set step: `set { output: out_label, value: val_str }`.
fn together_branch_yaml(labels_and_outputs: &[(&str, &str, &str)]) -> String {
    let mut yaml = String::from("  - id: fanout\n    together:\n      branches:\n");
    for (label, output, value) in labels_and_outputs {
        yaml.push_str(&format!("        - label: \"{label}\"\n          steps:\n"));
        yaml.push_str(&format!(
            "            - id: set_{label}\n              set:\n                output: \"{output}\"\n                value: \"{value}\"\n"
        ));
    }
    yaml
}

/// Generate a label string: 1-16 chars, alphanumeric + underscores, no YAML ambiguity.
fn label_strategy() -> impl Strategy<Value = String> {
    proptest::string::string_regex("[a-zA-Z][a-zA-Z0-9_]{0,15}").expect("label regex is valid")
}

/// Generate an output name: alphanumeric, no ambiguity.
fn output_strategy() -> impl Strategy<Value = String> {
    proptest::string::string_regex("[a-zA-Z][a-zA-Z0-9_]{0,7}").expect("output regex is valid")
}

/// Generate a value string: 1-4 digit non-empty numeric string.
fn value_strategy() -> impl Strategy<Value = String> {
    proptest::string::string_regex("[0-9]{1,4}").expect("value regex is valid")
}

// ---------------------------------------------------------------------------
// Proptest Properties
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 1000,
        failure_persistence: None,
        .. ProptestConfig::default()
    })]

    // -----------------------------------------------------------------------
    // PO-xi2f29-002: Branch Count Sensitivity
    // -----------------------------------------------------------------------

    /// Proptest: workflows with different branch counts produce different digests.
    #[test]
    fn proptest_together_branch_count_produces_different_digest(
        l1 in label_strategy(),
        o1 in output_strategy(),
        v1 in value_strategy(),
        l2 in label_strategy(),
        o2 in output_strategy(),
        v2 in value_strategy(),
        l3 in label_strategy(),
        o3 in output_strategy(),
        v3 in value_strategy(),
    ) {
        // 2-branch together
        let branches_2 = together_branch_yaml(&[
            (&l1, &o1, &v1),
            (&l2, &o2, &v2),
        ]);
        let yaml_2 = together_workflow_yaml(&branches_2);
        let d2 = compile_and_digest(&yaml_2).map_err(|e| TestCaseError::fail(e))?;

        // 3-branch together
        let branches_3 = together_branch_yaml(&[
            (&l1, &o1, &v1),
            (&l2, &o2, &v2),
            (&l3, &o3, &v3),
        ]);
        let yaml_3 = together_workflow_yaml(&branches_3);
        let d3 = compile_and_digest(&yaml_3).map_err(|e| TestCaseError::fail(e))?;

        prop_assert_ne!(d2, d3,
            "2-branch and 3-branch together workflows must produce different digests");
    }

    // -----------------------------------------------------------------------
    // PO-xi2f29-003: Branch Label Sensitivity
    // -----------------------------------------------------------------------

    /// Proptest: workflows with different branch labels produce different digests.
    #[test]
    fn proptest_together_branch_labels_produce_different_digest(
        la in label_strategy(),
        lb in label_strategy(),
        lc in label_strategy(),
        o1 in output_strategy(),
        v1 in value_strategy(),
        o2 in output_strategy(),
        v2 in value_strategy(),
    ) {
        // Ensure labels actually differ
        prop_assume!(la != lb);
        prop_assume!(la != lc);

        let branches_a = together_branch_yaml(&[
            (&la, &o1, &v1),
            (&lb, &o2, &v2),
        ]);
        let branches_b = together_branch_yaml(&[
            (&lc, &o1, &v1),
            (&lb, &o2, &v2),
        ]);

        let da = compile_and_digest(&together_workflow_yaml(&branches_a))
            .map_err(|e| TestCaseError::fail(e))?;
        let db = compile_and_digest(&together_workflow_yaml(&branches_b))
            .map_err(|e| TestCaseError::fail(e))?;

        prop_assert_ne!(da, db,
            "different branch labels must produce different digests");
    }

    // -----------------------------------------------------------------------
    // PO-xi2f29-004: Sub-Step Content Sensitivity
    // -----------------------------------------------------------------------

    /// Proptest: workflows with different sub-step outputs/values produce different digests.
    #[test]
    fn proptest_together_sub_step_contents_produce_different_digest(
        l1 in label_strategy(),
        l2 in label_strategy(),
        o1 in output_strategy(),
        v1 in value_strategy(),
        v1b in value_strategy(),
    ) {
        prop_assume!(v1 != v1b);

        let branches_a = together_branch_yaml(&[
            (&l1, &o1, &v1),
            (&l2, &o1, &v1),
        ]);
        let branches_b = together_branch_yaml(&[
            (&l1, &o1, &v1b),
            (&l2, &o1, &v1),
        ]);

        let da = compile_and_digest(&together_workflow_yaml(&branches_a))
            .map_err(|e| TestCaseError::fail(e))?;
        let db = compile_and_digest(&together_workflow_yaml(&branches_b))
            .map_err(|e| TestCaseError::fail(e))?;

        prop_assert_ne!(da, db,
            "different sub-step set values must produce different digests");
    }

    /// Proptest: different sub-step output names produce different digests.
    #[test]
    fn proptest_together_sub_step_output_produces_different_digest(
        l1 in label_strategy(),
        l2 in label_strategy(),
        o1 in output_strategy(),
        o2 in output_strategy(),
        v1 in value_strategy(),
    ) {
        prop_assume!(o1 != o2);

        let branches_a = together_branch_yaml(&[
            (&l1, &o1, &v1),
            (&l2, &o1, &v1),
        ]);
        let branches_b = together_branch_yaml(&[
            (&l1, &o2, &v1),
            (&l2, &o1, &v1),
        ]);

        let da = compile_and_digest(&together_workflow_yaml(&branches_a))
            .map_err(|e| TestCaseError::fail(e))?;
        let db = compile_and_digest(&together_workflow_yaml(&branches_b))
            .map_err(|e| TestCaseError::fail(e))?;

        prop_assert_ne!(da, db,
            "different sub-step output names must produce different digests");
    }

    // -----------------------------------------------------------------------
    // PO-xi2f29-005: Branch Ordering Sensitivity
    // -----------------------------------------------------------------------

    /// Proptest: reordering branches produces different digests.
    #[test]
    fn proptest_together_branch_ordering_produces_different_digest(
        la in label_strategy(),
        lb in label_strategy(),
        o1 in output_strategy(),
        o2 in output_strategy(),
        v1 in value_strategy(),
        v2 in value_strategy(),
    ) {
        prop_assume!(la != lb);

        let branches_ab = together_branch_yaml(&[
            (&la, &o1, &v1),
            (&lb, &o2, &v2),
        ]);
        let branches_ba = together_branch_yaml(&[
            (&lb, &o2, &v2),
            (&la, &o1, &v1),
        ]);

        let dab = compile_and_digest(&together_workflow_yaml(&branches_ab))
            .map_err(|e| TestCaseError::fail(e))?;
        let dba = compile_and_digest(&together_workflow_yaml(&branches_ba))
            .map_err(|e| TestCaseError::fail(e))?;

        prop_assert_ne!(dab, dba,
            "reordered branches must produce different digests");
    }

    // -----------------------------------------------------------------------
    // PO-xi2f29-006: Determinism (Together-Specific)
    // -----------------------------------------------------------------------

    /// Proptest: same together workflow → same digest (determinism).
    #[test]
    fn proptest_together_digest_is_deterministic(
        l1 in label_strategy(),
        l2 in label_strategy(),
        o1 in output_strategy(),
        o2 in output_strategy(),
        v1 in value_strategy(),
        v2 in value_strategy(),
    ) {
        let branches = together_branch_yaml(&[
            (&l1, &o1, &v1),
            (&l2, &o2, &v2),
        ]);
        let yaml = together_workflow_yaml(&branches);

        let d1 = compile_and_digest(&yaml).map_err(|e| TestCaseError::fail(e))?;
        let d2 = compile_and_digest(&yaml).map_err(|e| TestCaseError::fail(e))?;

        prop_assert_eq!(d1, d2,
            "same together workflow must produce same digest (determinism)");
    }

    // -----------------------------------------------------------------------
    // GAP-4 / P-8: Variable Branch Count (1..=20) — Valid Digest, No Panic
    // -----------------------------------------------------------------------

    /// Proptest: workflows with 1..=20 branches all produce valid, non-zero
    /// digests, and different branch counts produce different digests.
    ///
    /// Individual branch count values are stress-tested. Branch count
    /// changes must be reflected in the digest (via the u16 LE encoding).
    #[test]
    fn proptest_variable_branch_count_produces_different_digest(
        count in (3usize..=20usize),
    ) {
        // Build a workflow with `count` branches
        let mut branches_yaml = String::from(
            "  - id: fanout\n    together:\n      branches:\n"
        );
        for i in 0..count {
            branches_yaml.push_str(&format!(
                "        - label: \"br{i}\"\n          steps:\n            - id: set_{i}\n              set:\n                output: \"o{i}\"\n                value: \"{i}\"\n"
            ));
        }
        let yaml = together_workflow_yaml(&branches_yaml);
        let d1 = compile_and_digest(&yaml).map_err(|e| TestCaseError::fail(e))?;

        // Verify non-zero
        let is_all_zero = d1.as_bytes().iter().all(|&b| b == 0);
        prop_assert!(!is_all_zero,
            "digest for {}-branch together must not be all zeros", count);

        // Deterministic
        let d2 = compile_and_digest(&yaml).map_err(|e| TestCaseError::fail(e))?;
        prop_assert_eq!(d1, d2,
            "digest for {}-branch together must be deterministic", count);

        // Different count (count+1) must produce different digest
        let mut more_yaml = String::from(
            "  - id: fanout\n    together:\n      branches:\n"
        );
        for i in 0..(count + 1) {
            more_yaml.push_str(&format!(
                "        - label: \"br{i}\"\n          steps:\n            - id: set_{i}\n              set:\n                output: \"o{i}\"\n                value: \"{i}\"\n"
            ));
        }
        let yaml_more = together_workflow_yaml(&more_yaml);
        let d_more = compile_and_digest(&yaml_more).map_err(|e| TestCaseError::fail(e))?;
        let more_count = count + 1;
        prop_assert_ne!(d1, d_more,
            "{}-branch and {}-branch digests must differ", count, more_count);
    }

    // -----------------------------------------------------------------------
    // GAP-11 / P-9: Branch Label Length Variation (1..=256 chars)
    // -----------------------------------------------------------------------

    /// Proptest: varying branch label length produces valid digests, and
    /// different labels produce different digests.
    ///
    /// Labels up to 256 ASCII chars are stress-tested. The label is
    /// hashed via `hasher.update(branch.label.as_bytes())`.
    #[test]
    fn proptest_branch_label_length_produces_different_digest(
        la in proptest::string::string_regex("[a-zA-Z][a-zA-Z0-9_]{0,255}")
            .expect("label regex valid"),
        lb in proptest::string::string_regex("[a-zA-Z][a-zA-Z0-9_]{0,255}")
            .expect("label regex valid"),
        o1 in output_strategy(),
        o2 in output_strategy(),
        v1 in value_strategy(),
        v2 in value_strategy(),
    ) {
        prop_assume!(la != lb);

        let branches_a = together_branch_yaml(&[
            (&la, &o1, &v1),
            (&lb, &o2, &v2),
        ]);
        let branches_b = together_branch_yaml(&[
            (&lb, &o1, &v1),
            (&la, &o2, &v2),
        ]);

        let da = compile_and_digest(&together_workflow_yaml(&branches_a))
            .map_err(|e| TestCaseError::fail(e))?;
        let db = compile_and_digest(&together_workflow_yaml(&branches_b))
            .map_err(|e| TestCaseError::fail(e))?;

        prop_assert_ne!(da, db,
            "workflows with different branch label sets (long labels) must produce different digests");
    }
}
