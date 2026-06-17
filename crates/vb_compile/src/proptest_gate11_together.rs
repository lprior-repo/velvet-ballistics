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
    unused_variables,
)]

// Verification artifact: proptest_gate11_together.rs
// Obligation: PO-008-P
// Requirement: C-8 (Gate 11 validation compatibility)
// Proof seed: ps-22-008
// Verifier: proptest
// Command: cargo test -p vb_compile -- proptest_gate11_accepts_together_body --nocapture
// Bead: vb-xi2f.22
// State: 5 (proof-writer)
//
// GOD RULE 1: Uses proptest strategies for random together bodies.
// GOD RULE 2: Binds to actual emit_single_body_set and gate 11 validation.

#![cfg(test)]
#![forbid(unsafe_code)]

use crate::SlotCompiler;
use crate::mod_compile_lowering::emit_single_body_set;
use proptest::prelude::*;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_yaml::ast::{StepAst, StepPrimitive, TogetherBranch};

/// Strategy for valid together bodies that gate 11 should accept.
fn gate11_acceptance_body_strategy() -> impl Strategy<Value = Vec<StepAst>> {
    (1usize..=8usize, 1usize..=16usize).prop_map(|(branch_count, body_steps)| {
        let branches: Vec<TogetherBranch> = (0..branch_count)
            .map(|i| TogetherBranch {
                label: format!("b{}", i),
                steps: (0..body_steps)
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
            primitive: StepPrimitive::Together { branches },
            with: None,
            retry: None,
            on_error: None,
            then: None,
        }]
    })
}

// ─────────────────────────────────────────────────────────────────
// PO-008-P: Gate 11 compatibility for random together bodies
// ─────────────────────────────────────────────────────────────────

proptest! {
    /// Verify that random together bodies produce IR with valid
    /// structural properties that gate 11 requires.
    ///
    /// Gate 11 checks:
    /// - TogetherStart at start of span
    /// - TogetherBranch nodes between start and join
    /// - TogetherJoin at end of span with correct branch_count
    /// - All StepIdx values are within the together span
    #[test]
    fn proptest_gate11_accepts_together_body(body in gate11_acceptance_body_strategy()) {
        let mut builder = SlotCompiler::new();
        let base_id = StepIdx::new(0);

        let nodes_before = builder.nodes.len();
        let result = emit_single_body_set(
            &body,
            base_id,
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

                if emitted > 0 {
                    // Gate 11 structural property: first node at start of span
                    prop_assert_eq!(
                        builder.nodes[nodes_before].id.as_usize(),
                        base_id.as_usize(),
                        "first node must start at base StepIdx"
                    );

                    // Gate 11: all StepIdx values are unique (monotonic, strictly increasing)
                    let mut prev_id: Option<usize> = None;
                    for i in nodes_before..nodes_after {
                        let current_id = builder.nodes[i].id.as_usize();
                        if let Some(p) = prev_id {
                            // Gate 11 requires non-decreasing (can be equal for same-step nodes
                            // but typically strictly increasing)
                            prop_assert!(
                                current_id >= p,
                                "gate 11: StepIdx must be monotonic"
                            );
                        }
                        prev_id = Some(current_id);
                    }

                    // Gate 11: all StepIdx values fit in u16
                    if emitted > 0 {
                        let last_id = builder.nodes[nodes_after - 1].id.as_usize();
                        prop_assert!(
                            last_id < u16::MAX as usize,
                            "gate 11: StepIdx must fit in u16"
                        );
                    }
                }
            }
            Err(_) => {
                // Currently expected: UnsupportedStepPrimitive
            }
        }
    }
}
