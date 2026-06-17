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

//! Criterion benchmark for [`vb_core::budget::AggregateResourceBudget`].
//!
//! Folds a fixed synthetic workload of 1,000 [`AggregateResourceBudget`]
//! instances through the runtime admission validator
//! ([`vb_core::budget::validate_step_ceilings`]) and reports the
//! per-iteration median wall-clock time. The input budgets are pre-built
//! outside the timed region so the measurement reflects the validator
//! alone, not the synthetic workload generator.

#![forbid(unsafe_code)]

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use vb_core::budget::{AggregateResourceBudget, validate_step_ceilings};

/// Build a deterministic workload of 1,000 [`AggregateResourceBudget`]
/// instances with non-trivial numeric distribution so the optimiser cannot
/// collapse the loop to a constant.
fn build_workload(size: usize) -> Vec<AggregateResourceBudget> {
    (0..size)
        .map(|i| {
            let i = u32::try_from(i).unwrap_or(0);
            let mut budget = AggregateResourceBudget::default();
            budget.max_steps_executable = i.wrapping_mul(2);
            budget.max_action_tickets = i.wrapping_add(1);
            budget.max_run_time_seconds = u64::from(i.wrapping_mul(3));
            budget.max_trace_events = u64::from(i.wrapping_mul(4));
            budget.max_step_budget_per_tick = u64::from(i.wrapping_add(1));
            budget.max_transitions_per_tick = u64::from(i.wrapping_add(1));
            budget
        })
        .collect()
}

fn bench_aggregate_resource_budget(c: &mut Criterion) {
    let budgets = build_workload(1_000);
    c.bench_function("aggregate_resource_budget/1000_runs", |b| {
        b.iter(|| {
            for budget in black_box(&budgets) {
                let result = validate_step_ceilings(budget);
                let _ = black_box(result);
            }
        });
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(100);
    targets = bench_aggregate_resource_budget
);
criterion_main!(benches);
