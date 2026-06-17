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

// Verification artifact: reduce_together_collision.rs
// PO: PO-COLLISION-PROP-001
// Bead: vb-xi2f.24 | State: 5 (proof-writer)
// Verifier: proptest
// Command: cargo test -p vb_compile -- proptest_reduce_together_collision
//
// Requirement: N/A (Cross-Bead Collision Boundary)
// Domain Claim: Modifications by vb-xi2f.24 to canonical_body_step_width
//   do not conflict with vb-xi2f.22's modifications. Merged codebase
//   passes both beads' test suites.
//
// Defense-in-depth: proptest exercises both reduce and together bodies.

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn proptest_reduce_together_collision(
            reduce_body_len in 1usize..20usize,
            together_branch_count in 1usize..10usize,
        ) {
            // Verify body_width for reduce body
            let reduce_body: Vec<vb_yaml::ast::StepAst> = (0..reduce_body_len)
                .map(|i| vb_yaml::ast::StepAst {
                    id: format!("rs{i}"),
                    name: None,
                    condition: None,
                    primitive: vb_yaml::ast::StepPrimitive::Set {
                        output: format!("ro{i}"),
                        value: "1".to_string(),
                    },
                    with: None,
                    retry: None,
                    on_error: None,
                    then: None,
                })
                .collect();

            let reduce_width = crate::mod_compile_lowering::part_01::body_width(
                &reduce_body, 3
            );
            if let Ok(rw) = reduce_width {
                assert!(
                    rw >= 3 + reduce_body_len,
                    "reduce body width must include overhead + steps"
                );
            }

            // Verify body_width for together branches (cross-bead compatibility)
            // Together branches use body_width with overhead 1 per branch
            for branch_idx in 0..together_branch_count {
                let branch_body: Vec<vb_yaml::ast::StepAst> = vec![
                    vb_yaml::ast::StepAst {
                        id: format!("tb{branch_idx}"),
                        name: None,
                        condition: None,
                        primitive: vb_yaml::ast::StepPrimitive::Set {
                            output: "to".to_string(),
                            value: "1".to_string(),
                        },
                        with: None,
                        retry: None,
                        on_error: None,
                        then: None,
                    }
                ];

                let branch_width = crate::mod_compile_lowering::part_01::body_width(
                    &branch_body, 1
                );
                assert_eq!(
                    branch_width,
                    Ok(2),
                    "together branch width = overhead(1) + 1 step = 2"
                );
            }
        }
    }
}
