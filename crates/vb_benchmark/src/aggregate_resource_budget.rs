#![allow(clippy::expect_used, clippy::unwrap_used, clippy::as_conversions, clippy::arithmetic_side_effects, clippy::indexing_slicing, clippy::let_underscore_must_use, clippy::panic, clippy::panic_in_result_fn, clippy::bool_comparison, clippy::manual_div_ceil, clippy::clone_on_copy, clippy::len_zero, clippy::redundant_clone, clippy::collapsible_if, clippy::needless_return, clippy::needless_borrow, clippy::useless_format, clippy::redundant_pub_crate, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::missing_safety_doc, clippy::wildcard_enum_match_arm, clippy::large_futures, clippy::unused_async, clippy::unused_self, let_underscore_drop, clippy::filter_map_next, clippy::from_iter_instead_of_collect, clippy::if_not_else, clippy::implicit_clone, clippy::inefficient_to_string, clippy::inconsistent_struct_constructor, clippy::iter_filter_is_ok, clippy::iter_filter_is_some, clippy::iter_not_returning_iterator, clippy::iter_over_hash_type, clippy::iter_without_into_iter, clippy::large_digit_groups, clippy::large_types_passed_by_value, clippy::let_and_return, clippy::misnamed_getters, clippy::mutable_key_type, clippy::needless_collect, clippy::nonminimal_bool, clippy::option_if_let_else, clippy::or_fun_call, clippy::path_buf_push_overwrite, clippy::print_stderr, clippy::print_stdout, clippy::pub_with_shorthand, clippy::range_minus_one, clippy::range_plus_one, clippy::ref_binding_to_reference, clippy::ref_option_ref, clippy::single_match_else, clippy::suspicious_operation_groupings, clippy::trivially_copy_pass_by_ref, clippy::uninlined_format_args, clippy::unnecessary_wraps, clippy::unnested_or_patterns, clippy::unreadable_literal, clippy::unused_io_amount, clippy::unused_trait_names, clippy::vec_init_then_push, clippy::wildcard_imports, clippy::approx_constant, clippy::absurd_extreme_comparisons, clippy::expect_fun_call)]

//! Aggregate per-run resource metrics into a single resource budget report.
//!
//! The aggregator is a hot-path helper called by benchmark harnesses after each
//! sample run. It folds [`RunMetrics`] over a slice and returns a
//! [`ResourceBudgetReport`] with totals that downstream budget enforcement
//! compares against the configured ceiling.
//!
//! All accumulators use [`u64::saturating_add`], so an overflowed input
//! contribution clamps to [`u64::MAX`] instead of panicking in release builds.
//! The `run_count` field is the slice length, which the caller is expected to
//! have measured; it is copied verbatim.

#![forbid(unsafe_code)]

/// Resource usage data for a single benchmark run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RunMetrics {
    /// CPU time consumed by the run, in microseconds.
    pub cpu_us: u64,
    /// Peak resident memory used by the run, in bytes.
    pub memory_bytes: u64,
    /// Number of iterations executed during the run.
    pub iterations: u64,
}

/// Aggregated resource budget view across a sequence of [`RunMetrics`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ResourceBudgetReport {
    /// Total CPU time consumed across all runs, in microseconds.
    pub total_cpu_us: u64,
    /// Total peak memory used across all runs, in bytes.
    pub total_memory_bytes: u64,
    /// Total iterations executed across all runs.
    pub total_iterations: u64,
    /// Number of runs aggregated (the input slice length).
    pub run_count: usize,
}

/// Aggregate a slice of per-run metrics into a single resource budget report.
///
/// Each accumulator uses saturating arithmetic so that an overflowing input
/// contribution clamps to [`u64::MAX`] rather than panicking. The `run_count`
/// is taken from `runs.len()`.
///
/// # Example
///
/// ```
/// use vb_benchmark::aggregate_resource_budget::{aggregate_resource_budget, RunMetrics};
///
/// let runs = [
///     RunMetrics { cpu_us: 10, memory_bytes: 100, iterations: 1 },
///     RunMetrics { cpu_us: 20, memory_bytes: 200, iterations: 2 },
/// ];
/// let report = aggregate_resource_budget(&runs);
/// assert_eq!(report.total_cpu_us, 30);
/// assert_eq!(report.total_memory_bytes, 300);
/// assert_eq!(report.total_iterations, 3);
/// assert_eq!(report.run_count, 2);
/// ```
#[inline]
#[must_use]
pub fn aggregate_resource_budget(runs: &[RunMetrics]) -> ResourceBudgetReport {
    let mut total_cpu_us: u64 = 0;
    let mut total_memory_bytes: u64 = 0;
    let mut total_iterations: u64 = 0;
    for run in runs {
        total_cpu_us = total_cpu_us.saturating_add(run.cpu_us);
        total_memory_bytes = total_memory_bytes.saturating_add(run.memory_bytes);
        total_iterations = total_iterations.saturating_add(run.iterations);
    }
    ResourceBudgetReport {
        total_cpu_us,
        total_memory_bytes,
        total_iterations,
        run_count: runs.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_slice_returns_zero_report() {
        let report = aggregate_resource_budget(&[]);
        assert_eq!(report.total_cpu_us, 0);
        assert_eq!(report.total_memory_bytes, 0);
        assert_eq!(report.total_iterations, 0);
        assert_eq!(report.run_count, 0);
    }

    #[test]
    fn single_run_reports_input_verbatim() {
        let runs = [RunMetrics {
            cpu_us: 12_345,
            memory_bytes: 67_890,
            iterations: 7,
        }];
        let report = aggregate_resource_budget(&runs);
        assert_eq!(report.total_cpu_us, 12_345);
        assert_eq!(report.total_memory_bytes, 67_890);
        assert_eq!(report.total_iterations, 7);
        assert_eq!(report.run_count, 1);
    }

    #[test]
    fn one_hundred_runs_sum_in_order() {
        let mut runs = [RunMetrics {
            cpu_us: 0,
            memory_bytes: 0,
            iterations: 0,
        }; 100];
        let mut expected_cpu: u64 = 0;
        let mut expected_mem: u64 = 0;
        let mut expected_iters: u64 = 0;
        for (idx, run) in runs.iter_mut().enumerate() {
            run.cpu_us = idx as u64;
            run.memory_bytes = (idx as u64) * 2;
            run.iterations = (idx as u64) + 1;
            expected_cpu = expected_cpu.saturating_add(run.cpu_us);
            expected_mem = expected_mem.saturating_add(run.memory_bytes);
            expected_iters = expected_iters.saturating_add(run.iterations);
        }
        let report = aggregate_resource_budget(&runs);
        assert_eq!(report.total_cpu_us, expected_cpu);
        assert_eq!(report.total_memory_bytes, expected_mem);
        assert_eq!(report.total_iterations, expected_iters);
        assert_eq!(report.run_count, 100);
    }

    #[test]
    fn saturates_on_overflow_instead_of_panicking() {
        let runs = [
            RunMetrics {
                cpu_us: u64::MAX,
                memory_bytes: u64::MAX,
                iterations: u64::MAX,
            },
            RunMetrics {
                cpu_us: 1,
                memory_bytes: 1,
                iterations: 1,
            },
        ];
        let report = aggregate_resource_budget(&runs);
        assert_eq!(report.total_cpu_us, u64::MAX);
        assert_eq!(report.total_memory_bytes, u64::MAX);
        assert_eq!(report.total_iterations, u64::MAX);
        assert_eq!(report.run_count, 2);
    }
}
