#![forbid(unsafe_code)]
//! Post-decode validation pipeline for compiled queries.
//!
//! Functions in this module accept already-decoded data and verify the
//! structural and yield-budget admission contract.  They delegate to
//! `admission_kernel` for scalar checks and expose typed errors.

use super::domain::{
    CompiledQueries, MAX_QUERIES_PER_WORKFLOW, MAX_QUERY_PATH_SEGMENTS, YbBoundedQueries,
    YbBoundedQuery,
};
use super::errors::QueryParseError;
use crate::workflow::admission_kernel::{
    AdmissionKernelError, accumulate_yield_cost, validate_admission_summary,
};

fn checked_total_yield_cost(queries: &[YbBoundedQuery]) -> Result<u64, QueryParseError> {
    let mut total = 0_u64;
    for query in queries {
        total = accumulate_yield_cost(total, query.yield_cost)
            .map_err(|_| QueryParseError::YieldCostOverflow)?;
    }
    Ok(total)
}

fn max_query_path_depth(queries: &[YbBoundedQuery]) -> usize {
    let mut max_depth = 0_usize;
    for query in queries {
        max_depth = max_depth.max(query.path_depth());
    }
    max_depth
}

/// Validates a decoded query count against the workflow admission limit.
///
/// # Errors
///
/// Returns `QueryParseError::TooManyQueries` when `count` exceeds
/// `MAX_QUERIES_PER_WORKFLOW`.
pub fn validate_compiled_query_count(count: usize) -> Result<(), QueryParseError> {
    if count > MAX_QUERIES_PER_WORKFLOW {
        return Err(QueryParseError::TooManyQueries {
            count,
            max: MAX_QUERIES_PER_WORKFLOW,
        });
    }

    Ok(())
}

/// Validates the summarized post-decode query admission facts.
///
/// # Errors
///
/// Returns the same count, path-depth, total-mismatch, and budget errors as the
/// full decoded-query validator after total recomputation succeeds.
pub fn validate_compiled_query_summary(
    count: usize,
    recomputed_total: u64,
    declared_total_yield_cost: u64,
    max_path_depth: usize,
    max_yield_budget: u64,
) -> Result<u64, QueryParseError> {
    validate_query_admission_kernel(
        count,
        recomputed_total,
        declared_total_yield_cost,
        max_path_depth,
        max_yield_budget,
    )
    .map_err(|error| {
        query_summary_error(
            error,
            count,
            recomputed_total,
            declared_total_yield_cost,
            max_path_depth,
            max_yield_budget,
        )
    })
}

fn validate_query_admission_kernel(
    count: usize,
    recomputed_total: u64,
    declared_total_yield_cost: u64,
    max_path_depth: usize,
    max_yield_budget: u64,
) -> Result<u64, AdmissionKernelError> {
    validate_admission_summary(
        count,
        MAX_QUERIES_PER_WORKFLOW,
        max_path_depth,
        MAX_QUERY_PATH_SEGMENTS,
        recomputed_total,
        declared_total_yield_cost,
        max_yield_budget,
    )
}

fn query_summary_error(
    error: AdmissionKernelError,
    count: usize,
    recomputed_total: u64,
    declared_total_yield_cost: u64,
    max_path_depth: usize,
    max_yield_budget: u64,
) -> QueryParseError {
    match error {
        AdmissionKernelError::TooManyItems => QueryParseError::TooManyQueries {
            count,
            max: MAX_QUERIES_PER_WORKFLOW,
        },
        AdmissionKernelError::PathTooDeep => QueryParseError::QueryPathTooDeep {
            depth: max_path_depth,
            max: MAX_QUERY_PATH_SEGMENTS,
        },
        AdmissionKernelError::TotalYieldCostMismatch => QueryParseError::TotalYieldCostMismatch {
            declared: declared_total_yield_cost,
            recomputed: recomputed_total,
        },
        AdmissionKernelError::YieldBudgetExceeded => QueryParseError::YbBudgetExceeded {
            total: recomputed_total,
            max: max_yield_budget,
        },
    }
}

/// Validates decoded query parts and returns the remaining budget.
///
/// This lower-level seam avoids ownership transfer so Kani can verify the
/// post-decode admission contract over bounded fixed arrays.
///
/// # Errors
///
/// Returns the same structural, total, and budget errors as
/// `validate_compiled_queries`.
pub fn validate_compiled_query_parts(
    queries: &[YbBoundedQuery],
    declared_total_yield_cost: u64,
    max_yield_budget: u64,
) -> Result<u64, QueryParseError> {
    validate_compiled_query_count(queries.len())?;
    let max_path_depth = max_query_path_depth(queries);
    let recomputed_total = checked_total_yield_cost(queries)?;
    validate_compiled_query_summary(
        queries.len(),
        recomputed_total,
        declared_total_yield_cost,
        max_path_depth,
        max_yield_budget,
    )
}

/// Validates an already-decoded compiled query payload against structural and
/// yield-budget admission rules.
///
/// This seam is intentionally post-decode and side-effect free so verification
/// tools can prove admission behavior without symbolically executing postcard.
/// `from_bytes_compiled_queries` delegates here after successful deserialization.
///
/// # Errors
///
/// Returns `QueryParseError::TooManyQueries` if `compiled` exceeds
/// `MAX_QUERIES_PER_WORKFLOW`, `QueryParseError::QueryPathTooDeep` if any path
/// exceeds `MAX_QUERY_PATH_SEGMENTS`, `QueryParseError::YieldCostOverflow` if
/// recomputing totals overflows, `QueryParseError::TotalYieldCostMismatch` if
/// the declared total differs from the recomputed total, and
/// `QueryParseError::YbBudgetExceeded` if the recomputed total exceeds
/// `max_yield_budget`.
pub fn validate_compiled_queries(
    compiled: CompiledQueries,
    max_yield_budget: u64,
) -> Result<YbBoundedQueries, QueryParseError> {
    let remaining = validate_compiled_query_parts(
        &compiled.queries,
        compiled.total_yield_cost,
        max_yield_budget,
    )?;

    Ok(YbBoundedQueries::new(compiled.queries, remaining))
}
