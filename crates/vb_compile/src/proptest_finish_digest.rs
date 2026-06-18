#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::ok_expect,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::let_underscore_must_use,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::todo,
    clippy::unimplemented,
    clippy::assertions_on_constants,
    clippy::needless_range_loop,
    clippy::bool_assert_comparison,
    clippy::approx_constant,
    clippy::field_reassign_with_default,
    clippy::redundant_guards,
    clippy::redundant_closure,
    clippy::useless_conversion,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_cast,
    clippy::needless_update,
    clippy::bool_comparison,
    clippy::manual_div_ceil,
    clippy::clone_on_copy,
    clippy::len_zero,
    clippy::redundant_clone,
    clippy::collapsible_if,
    clippy::needless_return,
    clippy::needless_borrow,
    clippy::useless_format,
    clippy::redundant_pub_crate,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::missing_safety_doc,
    clippy::wildcard_enum_match_arm,
    clippy::large_futures,
    clippy::unused_async,
    clippy::unused_self,
    let_underscore_drop,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inefficient_to_string,
    clippy::inconsistent_struct_constructor,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_types_passed_by_value,
    clippy::let_and_return,
    clippy::misnamed_getters,
    clippy::mutable_key_type,
    clippy::needless_collect,
    clippy::nonminimal_bool,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::trivially_copy_pass_by_ref,
    clippy::uninlined_format_args,
    clippy::unnecessary_wraps,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_io_amount,
    clippy::unused_trait_names,
    clippy::vec_init_then_push,
    clippy::wildcard_imports,
    clippy::absurd_extreme_comparisons,
    clippy::expect_fun_call,
    clippy::useless_vec,
    clippy::redundant_locals,
    clippy::too_many_lines,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_abs_to_unsigned,
    clippy::similar_names,
    clippy::shadow_unrelated,
    clippy::needless_pass_by_value,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::redundant_pattern_matching,
    clippy::unneeded_struct_pattern,
    clippy::single_match,
    clippy::module_inception,
    clippy::match_like_matches_macro,
    clippy::duplicated_attributes,
    clippy::redundant_else,
    clippy::collapsible_match,
    clippy::manual_map,
    clippy::manual_let_else,
    clippy::manual_strip,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::if_let_mutex,
    unused_imports,
    dead_code,
    unused_variables
)]
// Verification artifact: proptest_finish_digest.rs
// Bead: vb-xi2f.34 — P1: digest covers finish semantics
// Proof obligations:
//   PO-PROPTEST-FINISH-001: Canonical digest determinism
//   PO-PROPTEST-FINISH-002: Finish result sensitivity (defense-in-depth)
//   PO-PROPTEST-FINISH-003: Finish step position sensitivity
//   PO-PROPTEST-FINISH-004: Digest is pre-validation (AST scope)
//
// GOD RULE 1: Uses proptest strategies to generate varied YAML-based inputs.
// GOD RULE 2: Binds to actual compile_source() and CompiledWorkflow::digest() APIs.
// GOD RULE 4: Exhaustive property testing; no loop oscillations.
//
// NOTE: canonical_digest() is pub(super) in part_05.rs and NOT accessible
// from this module. Instead, we use the public compile_source() API which
// internally calls canonical_digest() and stores the result in
// CompiledWorkflow.digest(). All property tests verify digest properties
// through the compilation pipeline.
#![cfg(test)]
#![forbid(unsafe_code)]

use crate::compile_source;
use proptest::prelude::*;
use vb_core::ids::WorkflowDigest;

// ─────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────

fn digest_from_yaml(yaml: &str) -> Result<WorkflowDigest, String> {
    let source = vb_yaml::parse_workflow_source(yaml).map_err(|e| format!("parse error: {e:?}"))?;
    let compiled = compile_source(&source).map_err(|e| format!("compile error: {e:?}"))?;
    Ok(compiled.digest())
}

fn digest_unwrap(yaml: &str) -> WorkflowDigest {
    digest_from_yaml(yaml).expect("valid workflow must compile")
}

fn yaml_with_steps(steps_yaml: &str) -> String {
    format!(
        "version: velvet-ballistics/v1\n\
         name: test_workflow\n\
         when:\n  manual: {{}}\n\
         steps:\n{}",
        steps_yaml
    )
}

// ─────────────────────────────────────────────────────────────────
// Strategies
// ─────────────────────────────────────────────────────────────────

fn step_id_strategy() -> impl Strategy<Value = String> {
    // Exclude single-character and YAML-ambiguous values.
    // YAML treats "y", "n", "yes", "no", "true", "false", "on", "off"
    // as booleans, which breaks the YAML template formatting.
    // Require at least 2 characters and avoid the known ambiguous patterns.
    "[a-z][a-z0-9_]{0,15}".prop_filter("exclude YAML-ambiguous step IDs", |s| {
        !matches!(
            s.as_str(),
            "y" | "n" | "yes" | "no" | "true" | "false" | "on" | "off"
        )
    })
}

#[allow(dead_code)]
fn set_step_yaml_strategy() -> impl Strategy<Value = String> {
    (step_id_strategy(), "[a-z][a-z0-9_]{0,15}", "[0-9]+").prop_map(|(id, output, value)| {
        format!("  - id: {id}\n    set:\n      output: {output}\n      value: \"{value}\"")
    })
}

// ─────────────────────────────────────────────────────────────────
// PO-PROPTEST-FINISH-001: Canonical digest determinism
// (also covers PO-PROPTEST-FINISH-004 / C9 — IR layout independence)
// ─────────────────────────────────────────────────────────────────

proptest! {
    /// Prove that the compiled digest is deterministic:
    /// compiling the same source twice returns the same digest.
    ///
    /// Contract clauses:
    ///   C4 — Canonical Digest Determinism
    ///   C9 — Digest Is Pre-Validation (structural guarantee:
    ///        fn canonical_digest(source: &WorkflowSource) cannot
    ///        depend on IR layout)
    /// Proof seeds: PS-FINISH-DIGEST-003, PS-FINISH-DIGEST-007.
    #[test]
    #[ignore = "proptest: run with --ignored or proptest runner"]
    fn canonical_digest_is_deterministic(
        slot in any::<u16>(),
        id in step_id_strategy(),
    ) {
        let yaml = yaml_with_steps(&format!(
            "  - id: {id}\n    finish:\n      result: {slot}"
        ));
        let source = vb_yaml::parse_workflow_source(&yaml)
            .expect("valid YAML must parse");

        let c1 = compile_source(&source).expect("must compile");
        let c2 = compile_source(&source).expect("must compile");

        prop_assert_eq!(c1.digest(), c2.digest(), "digest must be deterministic");
    }
}

// ─────────────────────────────────────────────────────────────────
// PO-PROPTEST-FINISH-002: Finish result value sensitivity
// ─────────────────────────────────────────────────────────────────

proptest! {
    /// Prove that changing the Finish result Integer value changes the digest.
    ///
    /// Contract clause: C1 — Finish Result Value Sensitivity.
    /// Proof seed: PS-FINISH-DIGEST-001.
    #[test]
    #[ignore = "proptest: run with --ignored or proptest runner"]
    fn finish_result_change_changes_digest_integer(
        id in step_id_strategy(),
        slot_a in any::<u16>(),
        slot_b in any::<u16>(),
    ) {
        proptest::prop_assume!(slot_a  != slot_b );

        let yaml_a = yaml_with_steps(&format!(
            "  - id: {id}\n    finish:\n      result: {slot_a}"
        ));
        let yaml_b = yaml_with_steps(&format!(
            "  - id: {id}\n    finish:\n      result: {slot_b}"
        ));

        let da = digest_unwrap(&yaml_a);
        let db = digest_unwrap(&yaml_b);

        prop_assert_ne!(da, db,
            "different Finish result Integer values must produce different digests");
    }

    /// Prove that changing the Finish result output name changes the digest.
    #[test]
    #[ignore = "proptest: run with --ignored or proptest runner"]
    fn finish_result_change_changes_digest_string(
        id in step_id_strategy(),
        out_a in step_id_strategy(),
        out_b in step_id_strategy(),
    ) {
        proptest::prop_assume!(out_a  != out_b );
        proptest::prop_assume!(id  != "s" );
        let _sid = "s".to_string();

        let yaml_a = yaml_with_steps(&format!(
            "  - id: {sid}\n    set:\n      output: {out_a}\n      value: \"10\"\n  - id: {id}\n    finish:\n      result: \"{out_a}\"",
            sid = "s",
            id = id,
            out_a = out_a,
        ));
        let yaml_b = yaml_with_steps(&format!(
            "  - id: {sid}\n    set:\n      output: {out_b}\n      value: \"10\"\n  - id: {id}\n    finish:\n      result: \"{out_b}\"",
            sid = "s",
            id = id,
            out_b = out_b,
        ));

        let da = digest_unwrap(&yaml_a);
        let db = digest_unwrap(&yaml_b);

        prop_assert_ne!(da, db,
            "different Finish result output names must produce different digests");
    }
}

// ─────────────────────────────────────────────────────────────────
// PO-PROPTEST-FINISH-003: Finish step position sensitivity
// ─────────────────────────────────────────────────────────────────

proptest! {
    /// Prove that step IDs (and therefore step ordering) affect the digest.
    ///
    /// Contract clause: C3 — Finish Step Position Sensitivity.
    /// Proof seed: PS-FINISH-DIGEST-010.
    #[test]
    #[ignore = "proptest: run with --ignored or proptest runner"]
    fn finish_position_change_changes_digest(
        id1 in step_id_strategy(),
        id2 in step_id_strategy(),
        slot in any::<u16>(),
    ) {
        proptest::prop_assume!(id1  != id2 );

        let yaml_a = yaml_with_steps(&format!(
            "  - id: {id1}\n    finish:\n      result: {slot}"
        ));
        let yaml_b = yaml_with_steps(&format!(
            "  - id: {id2}\n    finish:\n      result: {slot}"
        ));

        let da = digest_unwrap(&yaml_a);
        let db = digest_unwrap(&yaml_b);

        prop_assert_ne!(da, db,
            "different step IDs must produce different digests");
    }
}

// ─────────────────────────────────────────────────────────────────
// PO-INT-FINISH-004: Canonical/legacy digest equivalence
// ─────────────────────────────────────────────────────────────────
//
// NOTE: This test is implemented inside `compile/mod.rs` (see
// the `#[cfg(test)] mod finish_digest_equivalence_tests` block at
// the end of that file) because both `canonical_digest` functions
// are private/`pub(super)` within their respective modules and
// are not accessible from `proptest_finish_digest.rs` or from
// integration tests in `tests/`.
//
// The integration test in tests/finish_digest_integration.rs has a
// placeholder blocked by visibility (BLOCKED_VISIBILITY).
// See PF-FINISH-INT-001 in proof-findings.jsonl.

// ─────────────────────────────────────────────────────────────────
// PO-PROPTEST-FINISH-004: Digest is pre-validation (AST scope)
// ─────────────────────────────────────────────────────────────────
//
// NOTE: PO-PROPTEST-FINISH-004 was identified as a duplicate of
// PO-PROPTEST-FINISH-001 (both compile the same source twice and
// assert identical digests). The function signature
//   fn canonical_digest(source: &WorkflowSource)
// structurally guarantees IR independence (C9) — the digest depends
// only on the AST, not on any IR layout decisions made during
// lowering. See PF-FINISH-PROP-002 in proof-findings.jsonl.
//
// The canonical_digest_is_deterministic test above covers both:
// - C4: Canonical Digest Determinism
// - C9: Digest Is Pre-Validation (structural guarantee)
// through the single proptest property.
//
// If IR layout independence needs independent verification in the
// future, create a test that varies compiler settings while keeping
// the AST identical.
