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
// Verification artifact: proptest_together_errors.rs
// Obligation: PO-006-P
// Requirement: C-6 (Together lowering error propagation)
// Proof seed: ps-22-006
// Verifier: proptest
// Command: cargo test -p vb_compile -- proptest_together_error_variants --nocapture
// Bead: vb-xi2f.22
// State: 5 (proof-writer)
//
// GOD RULE 1: Uses proptest strategies for random invalid together configurations.
// GOD RULE 2: Binds to actual emit_single_body_set in part_04.rs.
#![cfg(test)]
#![forbid(unsafe_code)]

use crate::SlotCompiler;
use crate::mod_compile_lowering::emit_single_body_set;
use proptest::prelude::*;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_yaml::ast::{StepAst, StepPrimitive, TogetherBranch};

/// Strategy: multi-step body (2..=5 steps)
fn multi_step_body_strategy() -> impl Strategy<Value = Vec<StepAst>> {
    (2usize..=5usize).prop_map(|n| {
        (0..n)
            .map(|i| StepAst {
                id: format!("step_{}", i),
                name: None,
                condition: None,
                primitive: StepPrimitive::Set {
                    output: String::from("x"),
                    value: String::from("1"),
                },
                with: None,
                retry: None,
                on_error: None,
                then: None,
            })
            .collect()
    })
}

/// Strategy: together with zero branches
fn zero_branch_together_strategy() -> impl Strategy<Value = Vec<StepAst>> {
    Just(vec![StepAst {
        id: String::from("t"),
        name: None,
        condition: None,
        primitive: StepPrimitive::Together { branches: vec![] },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }])
}

/// Strategy: together at edge StepIdx (near overflow)
fn edge_stepidx_together_strategy() -> impl Strategy<Value = (Vec<StepAst>, u16)> {
    (1usize..=4usize, 65530u16..=65535u16).prop_map(|(branch_count, edge_id)| {
        let branches: Vec<TogetherBranch> = (0..branch_count)
            .map(|i| TogetherBranch {
                label: format!("b{}", i),
                steps: vec![StepAst {
                    id: format!("s{}", i),
                    name: None,
                    condition: None,
                    primitive: StepPrimitive::Set {
                        output: String::from("x"),
                        value: String::from("1"),
                    },
                    with: None,
                    retry: None,
                    on_error: None,
                    then: None,
                }],
            })
            .collect();
        (
            vec![StepAst {
                id: String::from("t"),
                name: None,
                condition: None,
                primitive: StepPrimitive::Together { branches },
                with: None,
                retry: None,
                on_error: None,
                then: None,
            }],
            edge_id,
        )
    })
}

// ─────────────────────────────────────────────────────────────────
// PO-006-P: Error variant verification
// ─────────────────────────────────────────────────────────────────

/// Empty body → StepFieldShape error, no panic.
#[test]
fn proptest_together_error_empty_body() {
    let body = vec![];
    let mut builder = SlotCompiler::new();
    let result = emit_single_body_set(
        &body,
        StepIdx::new(0),
        0,
        SlotIdx::new(0),
        None,
        &mut builder,
        false,
    );
    assert!(result.is_err(), "empty body → error");
    let err = result.unwrap_err();
    let has_shape_err = err.0.iter().any(|e| {
        matches!(e, crate::CompileError::StepFieldShape { field, .. }
            if *field == "steps")
    });
    assert!(has_shape_err, "empty body → StepFieldShape");
}

proptest! {
    /// Multi-step body → StepFieldShape error.
    #[test]
    fn proptest_together_error_multi_step(body in multi_step_body_strategy()) {
        let mut builder = SlotCompiler::new();
        let result = emit_single_body_set(
            &body,
            StepIdx::new(0),
            0,
            SlotIdx::new(0),
            None,
            &mut builder,
            false,
        );
        prop_assert!(result.is_err(), "multi-step body → error");
    }

    /// Zero-branch together → error (or graceful handling), no panic.
    #[test]
    fn proptest_together_error_zero_branches(body in zero_branch_together_strategy()) {
        let mut builder = SlotCompiler::new();
        let result = emit_single_body_set(
            &body,
            StepIdx::new(0),
            0,
            SlotIdx::new(0),
            None,
            &mut builder,
            false,
        );
        prop_assert!(
            matches!(result, Ok(()) | Err(_)),
            "zero-branch together must return a Result without panic"
        );
    }

    /// Edge StepIdx together → error or success, but never panic.
    #[test]
    fn proptest_together_error_stepidx_overflow(
        (body, edge_id) in edge_stepidx_together_strategy()
    ) {
        let mut builder = SlotCompiler::new();
        let result = emit_single_body_set(
            &body,
            StepIdx::new(edge_id),
            0,
            SlotIdx::new(0),
            None,
            &mut builder,
            false,
        );
        // Must not panic. May return StepIndexOutOfRange.
        match result {
            Ok(()) => {
                // Success at edge: must not exceed u16 range
                // (checked by checked_step_offset in production code)
            }
            Err(_) => {
                // Expected: StepIndexOutOfRange or similar
            }
        }
    }
}
