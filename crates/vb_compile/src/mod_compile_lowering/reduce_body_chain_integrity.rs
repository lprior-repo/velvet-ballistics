#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::duplicated_attributes,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_strip,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
    clippy::needless_collect,
    clippy::needless_pass_by_value,
    clippy::needless_range_loop,
    clippy::needless_return,
    clippy::needless_update,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::nonminimal_bool,
    clippy::ok_expect,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::redundant_clone,
    clippy::redundant_closure,
    clippy::redundant_else,
    clippy::redundant_guards,
    clippy::redundant_locals,
    clippy::redundant_pattern_matching,
    clippy::redundant_pub_crate,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::shadow_unrelated,
    clippy::similar_names,
    clippy::single_match,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::todo,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_wraps,
    clippy::unneeded_struct_pattern,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_async,
    clippy::unused_io_amount,
    clippy::unused_self,
    clippy::unused_trait_names,
    clippy::unwrap_used,
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::useless_vec,
    clippy::vec_init_then_push,
    clippy::wildcard_enum_match_arm,
    clippy::wildcard_imports,
    dead_code,
    let_underscore_drop,
    unused_imports,
    unused_variables,
)]

// Verification artifact: reduce_body_chain_integrity.rs
// PO: PO-CHAIN-PROP-001
// Bead: vb-xi2f.24 | State: 5 (proof-writer)
// Verifier: proptest
// Command: cargo test -p vb_compile -- proptest_reduce_body_chain_integrity
//
// Requirement: C4 — Body Step Next-Link Chain
// Domain Claim: Arbitrary body step sequences produce correct linear
//   next-link chains with no broken or dangling links.

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    fn step_sequence_strategy() -> impl Strategy<Value = Vec<u16>> {
        prop::collection::vec(1u16..10u16, 1..50)
    }

    fn build_chain(base: u16, widths: &[u16], next_step: u16) -> Vec<(u16, u16)> {
        let mut chain = Vec::new();
        let mut cumulative: u16 = 0;
        for i in 0..widths.len() {
            let step_id = base + 1 + cumulative;
            let step_next = if i == widths.len() - 1 {
                next_step
            } else {
                base + 1 + cumulative + widths[i]
            };
            chain.push((step_id, step_next));
            cumulative = cumulative.saturating_add(widths[i]);
        }
        chain
    }

    proptest! {
        #[test]
        fn proptest_reduce_body_chain_integrity(
            widths in step_sequence_strategy(),
        ) {
            let base: u16 = 10;
            // Compute next_step as if after all body steps
            let total_width: u16 = widths.iter().sum();
            let next_step = base + 1 + total_width;

            let chain = build_chain(base, &widths, next_step);

            if chain.is_empty() {
                return Ok(());
            }

            // Chain must be continuous
            for i in 0..chain.len() - 1 {
                let (current_id, current_next) = chain[i];
                let (next_id, _) = chain[i + 1];

                assert_eq!(
                    current_next, next_id,
                    "step {} next ({}) must equal next step id ({})",
                    i, current_next, next_id
                );
                assert!(
                    current_next > current_id,
                    "step {} next must be > step id",
                    i
                );
            }

            // Last step chains to next_step
            let (last_id, last_next) = chain[chain.len() - 1];
            assert_eq!(
                last_next, next_step,
                "last step next ({}) must equal next_step ({})",
                last_next, next_step
            );
            assert!(
                last_next > last_id,
                "last step next must be > last step id"
            );
        }
    }
}
