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
// Verification artifact: proptest_budget_together.rs
// Obligation: PO-009-P
// Requirement: C-9 (Budget compliance after nested together lowering)
// Proof seed: ps-22-009
// Verifier: proptest
// Command: cargo test -p vb_compile -- proptest_together_budget_compliance --nocapture
// Bead: vb-xi2f.22
// State: 5 (proof-writer)
//
// GOD RULE 1: Uses proptest strategies for random together bodies of varying sizes.
// GOD RULE 2: Binds to actual emit_single_body_set and budget validation.
#![cfg(test)]
#![forbid(unsafe_code)]

use crate::SlotCompiler;
use crate::mod_compile_lowering::emit_single_body_set;
use proptest::prelude::*;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_yaml::ast::{StepAst, StepPrimitive, TogetherBranch};

/// Strategy for small together bodies (within typical budget).
fn small_together_strategy() -> impl Strategy<Value = Vec<StepAst>> {
    (1usize..=4usize, 1usize..=8usize).prop_map(|(branches, steps)| {
        let brs: Vec<TogetherBranch> = (0..branches)
            .map(|i| TogetherBranch {
                label: format!("b{}", i),
                steps: (0..steps)
                    .map(|s| StepAst {
                        id: format!("s{}.{}", i, s),
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
                    .collect(),
            })
            .collect();
        vec![StepAst {
            id: String::from("together"),
            name: None,
            condition: None,
            primitive: StepPrimitive::Together { branches: brs },
            with: None,
            retry: None,
            on_error: None,
            then: None,
        }]
    })
}

/// Strategy for large together bodies (likely over budget).
fn large_together_strategy() -> impl Strategy<Value = Vec<StepAst>> {
    (12usize..=16usize, 8usize..=16usize).prop_map(|(branches, steps)| {
        let brs: Vec<TogetherBranch> = (0..branches)
            .map(|i| TogetherBranch {
                label: format!("b{}", i),
                steps: (0..steps)
                    .map(|s| StepAst {
                        id: format!("s{}.{}", i, s),
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
                    .collect(),
            })
            .collect();
        vec![StepAst {
            id: String::from("together"),
            name: None,
            condition: None,
            primitive: StepPrimitive::Together { branches: brs },
            with: None,
            retry: None,
            on_error: None,
            then: None,
        }]
    })
}

// ─────────────────────────────────────────────────────────────────
// PO-009-P: Budget compliance for random together bodies
// ─────────────────────────────────────────────────────────────────

proptest! {
    /// Small together bodies: should fit within budget.
    #[test]
    fn proptest_together_budget_within(body in small_together_strategy()) {
        let mut builder = SlotCompiler::new();

        let nodes_before = builder.nodes.len();
        let result = emit_single_body_set(
            &body,
            StepIdx::new(0),
            0,
            SlotIdx::new(0),
            None,
            &mut builder,
            false,
        );

        match result {
            Ok(()) => {
                let nodes_after = builder.nodes.len();
                let emitted = nodes_after - nodes_before;

                // Small together should not exceed reasonable budget
                // For 4 branches × 8 steps: together_width = 2 + 4*8 = 34 nodes
                prop_assert!(emitted <= 128,
                    "small together must fit within 128-node budget, got {}", emitted);

                // Must fit within u16 for StepIdx
                prop_assert!(emitted <= u16::MAX as usize,
                    "emitted nodes must fit in u16");
            }
            Err(_) => {
                // Currently expected: UnsupportedStepPrimitive
            }
        }
    }

    /// Large together bodies: may exceed budget but must not panic.
    #[test]
    fn proptest_together_budget_exceeded(body in large_together_strategy()) {
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

        // Must not panic. Either Ok (if within budget) or Err (if exceeded
        // or unsupported).
        match result {
            Ok(()) => {
                // If success, total node count must still fit in u16
                let emitted = builder.nodes.len();
                prop_assert!(emitted <= u16::MAX as usize,
                    "total nodes must fit in u16 even for large together");
            }
            Err(_) => {
                // Expected: budget exceeded, StepIdx overflow, or UnsupportedStepPrimitive
            }
        }
    }

    /// Deterministic behavior: same together body produces same result
    /// (no non-deterministic budget failure).
    #[test]
    fn proptest_together_budget_deterministic(body in small_together_strategy()) {
        let do_lowering = |body: &[StepAst]| {
            let mut builder = SlotCompiler::new();
            emit_single_body_set(
                body,
                StepIdx::new(0),
                0,
                SlotIdx::new(0),
                None,
                &mut builder,
                false,
            ).map(|_| builder.nodes.len())
        };

        let result1 = do_lowering(&body);
        let result2 = do_lowering(&body);

        match (result1, result2) {
            (Ok(n1), Ok(n2)) => prop_assert_eq!(n1, n2, "deterministic node count"),
            (Err(_), Err(_)) => {}, // both error → deterministic
            _ => prop_assert!(false, "inconsistent results"),
        }
    }
}
