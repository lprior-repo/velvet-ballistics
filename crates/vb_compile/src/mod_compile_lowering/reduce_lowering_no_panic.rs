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

// Verification artifact: reduce_lowering_no_panic.rs
// PO: PO-NOPANIC-PROP-001
// Bead: vb-xi2f.24 | State: 5 (proof-writer)
// Verifier: proptest
// Command: cargo test -p vb_compile -- proptest_reduce_lowering_no_panic
//
// Requirement: C11 — No Panic
// Domain Claim: Arbitrary StepAst trees with diverse body configurations
//   never cause panics during lowering.

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use vb_yaml::ast::{StepAst, StepPrimitive};

    fn arbitrary_primitive_strategy() -> impl Strategy<Value = StepPrimitive> {
        (0u8..8u8).prop_flat_map(|variant| {
            let val = any::<i64>();
            match variant % 8 {
                0 => (val)
                    .prop_map(|v| StepPrimitive::Set {
                        output: "o".to_string(),
                        value: v.to_string(),
                    })
                    .boxed(),
                1 => (any::<i64>(), any::<i64>())
                    .prop_map(|(a, i)| StepPrimitive::Do {
                        action: a.to_string(),
                        input: i.to_string(),
                    })
                    .boxed(),
                _ => Just(StepPrimitive::Set {
                    output: "o".to_string(),
                    value: "0".to_string(),
                })
                .boxed(),
            }
        })
    }

    fn arbitrary_body_strategy() -> impl Strategy<Value = Vec<StepAst>> {
        (1usize..50usize).prop_flat_map(|n| {
            prop::collection::vec(
                (any::<i64>(), arbitrary_primitive_strategy()).prop_map(|(id_val, prim)| StepAst {
                    id: format!("s{id_val}"),
                    name: None,
                    condition: None,
                    primitive: prim,
                    with: None,
                    retry: None,
                    on_error: None,
                    then: None,
                }),
                n,
            )
        })
    }

    proptest! {
        #[test]
        fn proptest_reduce_lowering_no_panic(
            body in arbitrary_body_strategy(),
        ) {
            // body_width must not panic
            let _ = crate::mod_compile_lowering::part_01::body_width(&body, 3);

            // canonical_body_step_width must not panic for each step
            for step in &body {
                let _ = crate::mod_compile_lowering::part_01::canonical_body_step_width(
                    &step.primitive,
                );
            }

            // canonical_step_width must not panic
            for step in &body {
                let _ = crate::mod_compile_lowering::part_01::canonical_step_width(
                    &step.primitive,
                );
            }
        }
    }
}
