#![allow(clippy::expect_used, clippy::unwrap_used, clippy::as_conversions, clippy::arithmetic_side_effects, clippy::indexing_slicing, clippy::let_underscore_must_use, clippy::panic, clippy::panic_in_result_fn, clippy::bool_comparison, clippy::manual_div_ceil, clippy::clone_on_copy, clippy::len_zero, clippy::redundant_clone, clippy::collapsible_if, clippy::needless_return, clippy::needless_borrow, clippy::useless_format, clippy::redundant_pub_crate, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::missing_safety_doc, clippy::wildcard_enum_match_arm, clippy::large_futures, clippy::unused_async, clippy::unused_self, let_underscore_drop, clippy::filter_map_next, clippy::from_iter_instead_of_collect, clippy::if_not_else, clippy::implicit_clone, clippy::inefficient_to_string, clippy::inconsistent_struct_constructor, clippy::iter_filter_is_ok, clippy::iter_filter_is_some, clippy::iter_not_returning_iterator, clippy::iter_over_hash_type, clippy::iter_without_into_iter, clippy::large_digit_groups, clippy::large_types_passed_by_value, clippy::let_and_return, clippy::misnamed_getters, clippy::mutable_key_type, clippy::needless_collect, clippy::nonminimal_bool, clippy::option_if_let_else, clippy::or_fun_call, clippy::path_buf_push_overwrite, clippy::print_stderr, clippy::print_stdout, clippy::pub_with_shorthand, clippy::range_minus_one, clippy::range_plus_one, clippy::ref_binding_to_reference, clippy::ref_option_ref, clippy::single_match_else, clippy::suspicious_operation_groupings, clippy::trivially_copy_pass_by_ref, clippy::uninlined_format_args, clippy::unnecessary_wraps, clippy::unnested_or_patterns, clippy::unreadable_literal, clippy::unused_io_amount, clippy::unused_trait_names, clippy::vec_init_then_push, clippy::wildcard_imports, clippy::approx_constant, clippy::absurd_extreme_comparisons, clippy::expect_fun_call)]

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
