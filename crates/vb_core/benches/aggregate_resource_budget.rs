#![allow(clippy::expect_used, clippy::unwrap_used, clippy::as_conversions, clippy::arithmetic_side_effects, clippy::indexing_slicing, clippy::let_underscore_must_use, clippy::panic, clippy::panic_in_result_fn, clippy::bool_comparison, clippy::manual_div_ceil, clippy::clone_on_copy, clippy::len_zero, clippy::redundant_clone, clippy::collapsible_if, clippy::needless_return, clippy::needless_borrow, clippy::useless_format, clippy::redundant_pub_crate, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::missing_safety_doc, clippy::wildcard_enum_match_arm, clippy::large_futures, clippy::unused_async, clippy::unused_self, clippy::let_underscore_drop, clippy::filter_map_next, clippy::from_iter_instead_of_collect, clippy::if_not_else, clippy::implicit_clone, clippy::inefficient_to_string, clippy::inconsistent_struct_constructor, clippy::iter_filter_is_ok, clippy::iter_filter_is_some, clippy::iter_not_returning_iterator, clippy::iter_over_hash_type, clippy::iter_without_into_iter, clippy::large_digit_groups, clippy::large_types_passed_by_value, clippy::let_and_return, clippy::misnamed_getters, clippy::mutable_key_type, clippy::needless_collect, clippy::nonminimal_bool, clippy::option_if_let_else, clippy::or_fun_call, clippy::path_buf_push_overwrite, clippy::print_stderr, clippy::print_stdout, clippy::pub_with_shorthand, clippy::range_minus_one, clippy::range_plus_one, clippy::ref_binding_to_reference, clippy::ref_option_ref, clippy::single_match_else, clippy::suspicious_operation_groupings, clippy::trivially_copy_pass_by_ref, clippy::uninlined_format_args, clippy::unnecessary_wraps, clippy::unnested_or_patterns, clippy::unreadable_literal, clippy::unused_io_amount, clippy::unused_trait_names, clippy::vec_init_then_push, clippy::wildcard_imports, clippy::approximate_const, clippy::absurd_extreme_comparisons, clippy::expect_fun_call)]


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
