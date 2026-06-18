#![cfg(test)]
#![forbid(unsafe_code)]
//! RPO-AJC40-002: production-bound proptest/unit properties for AJC40
//! admission validators. These tests call `vb_core` production validators and
//! the production admission kernel directly; they do not use the retired
//! `vb_ajc40_flux_contracts.rs` mirror proof.

use proptest::prelude::*;

use crate::workflow::admission_kernel::{
    AdmissionKernelError, accumulate_yield_cost, validate_admission_summary,
};
use crate::workflow::compiled_query::{
    MAX_QUERIES_PER_WORKFLOW, MAX_QUERY_PATH_SEGMENTS, QueryParseError,
    validate_compiled_query_count, validate_compiled_query_summary,
};
use crate::workflow::compiled_slug::{
    MAX_SLUG_PATH_SEGMENTS, MAX_SLUGS_PER_WORKFLOW, SlugParseError, validate_compiled_slug_count,
    validate_compiled_slug_summary,
};

const COUNT_OVER_LIMIT: usize = 65_536;
const PATH_DEPTH_OVER_LIMIT: usize = 17;
const SMALL_TOTAL_LIMIT: u64 = 1_000_000;

fn slug_too_many(count: usize) -> SlugParseError {
    SlugParseError::TooManySlugs {
        count,
        max: MAX_SLUGS_PER_WORKFLOW,
    }
}

fn slug_too_deep(depth: usize) -> SlugParseError {
    SlugParseError::SlugPathTooDeep {
        depth,
        max: MAX_SLUG_PATH_SEGMENTS,
    }
}

fn slug_total_mismatch(declared: u64, recomputed: u64) -> SlugParseError {
    SlugParseError::TotalYieldCostMismatch {
        declared,
        recomputed,
    }
}

fn query_too_many(count: usize) -> QueryParseError {
    QueryParseError::TooManyQueries {
        count,
        max: MAX_QUERIES_PER_WORKFLOW,
    }
}

fn query_too_deep(depth: usize) -> QueryParseError {
    QueryParseError::QueryPathTooDeep {
        depth,
        max: MAX_QUERY_PATH_SEGMENTS,
    }
}

fn query_total_mismatch(declared: u64, recomputed: u64) -> QueryParseError {
    QueryParseError::TotalYieldCostMismatch {
        declared,
        recomputed,
    }
}

fn expected_kernel_result(
    count: usize,
    max_count: usize,
    max_path_depth: usize,
    max_path_segments: usize,
    recomputed_total: u64,
    declared_total_yield_cost: u64,
    max_yield_budget: u64,
) -> Result<u64, AdmissionKernelError> {
    if count > max_count {
        Err(AdmissionKernelError::TooManyItems)
    } else if max_path_depth > max_path_segments {
        Err(AdmissionKernelError::PathTooDeep)
    } else if declared_total_yield_cost != recomputed_total {
        Err(AdmissionKernelError::TotalYieldCostMismatch)
    } else {
        match max_yield_budget.checked_sub(recomputed_total) {
            Some(remaining) => Ok(remaining),
            None => Err(AdmissionKernelError::YieldBudgetExceeded),
        }
    }
}

fn expected_slug_summary_result(
    count: usize,
    recomputed_total: u64,
    declared_total_yield_cost: u64,
    max_path_depth: usize,
    max_yield_budget: u64,
) -> Result<u64, SlugParseError> {
    match expected_kernel_result(
        count,
        MAX_SLUGS_PER_WORKFLOW,
        max_path_depth,
        MAX_SLUG_PATH_SEGMENTS,
        recomputed_total,
        declared_total_yield_cost,
        max_yield_budget,
    ) {
        Ok(remaining) => Ok(remaining),
        Err(AdmissionKernelError::TooManyItems) => Err(slug_too_many(count)),
        Err(AdmissionKernelError::PathTooDeep) => Err(slug_too_deep(max_path_depth)),
        Err(AdmissionKernelError::TotalYieldCostMismatch) => Err(slug_total_mismatch(
            declared_total_yield_cost,
            recomputed_total,
        )),
        Err(AdmissionKernelError::YieldBudgetExceeded) => Err(SlugParseError::YbBudgetExceeded {
            total: recomputed_total,
            max: max_yield_budget,
        }),
    }
}

fn expected_query_summary_result(
    count: usize,
    recomputed_total: u64,
    declared_total_yield_cost: u64,
    max_path_depth: usize,
    max_yield_budget: u64,
) -> Result<u64, QueryParseError> {
    match expected_kernel_result(
        count,
        MAX_QUERIES_PER_WORKFLOW,
        max_path_depth,
        MAX_QUERY_PATH_SEGMENTS,
        recomputed_total,
        declared_total_yield_cost,
        max_yield_budget,
    ) {
        Ok(remaining) => Ok(remaining),
        Err(AdmissionKernelError::TooManyItems) => Err(query_too_many(count)),
        Err(AdmissionKernelError::PathTooDeep) => Err(query_too_deep(max_path_depth)),
        Err(AdmissionKernelError::TotalYieldCostMismatch) => Err(query_total_mismatch(
            declared_total_yield_cost,
            recomputed_total,
        )),
        Err(AdmissionKernelError::YieldBudgetExceeded) => Err(QueryParseError::YbBudgetExceeded {
            total: recomputed_total,
            max: max_yield_budget,
        }),
    }
}

#[path = "vb_dzibx_ajc40_admission_props_cases.rs"]
mod cases;

proptest! {
    #[test]
    fn vb_dzibx_ajc40_admission_bridge_kernel_matches_contract_for_generated_scalars(
        count in any::<usize>(),
        max_count in any::<usize>(),
        max_path_depth in any::<usize>(),
        max_path_segments in any::<usize>(),
        recomputed_total in any::<u64>(),
        declared_total_yield_cost in any::<u64>(),
        max_yield_budget in any::<u64>(),
    ) {
        let actual = validate_admission_summary(
            count,
            max_count,
            max_path_depth,
            max_path_segments,
            recomputed_total,
            declared_total_yield_cost,
            max_yield_budget,
        );
        let expected = expected_kernel_result(
            count,
            max_count,
            max_path_depth,
            max_path_segments,
            recomputed_total,
            declared_total_yield_cost,
            max_yield_budget,
        );
        prop_assert_eq!(actual, expected);
    }

    #[test]
    fn vb_dzibx_ajc40_admission_bridge_slug_summary_matches_contract_for_generated_boundaries(
        count in 0usize..=COUNT_OVER_LIMIT,
        recomputed_total in 0u64..=SMALL_TOTAL_LIMIT,
        declared_total_yield_cost in 0u64..=SMALL_TOTAL_LIMIT,
        max_path_depth in 0usize..=PATH_DEPTH_OVER_LIMIT,
        max_yield_budget in 0u64..=SMALL_TOTAL_LIMIT,
    ) {
        let actual = validate_compiled_slug_summary(
            count,
            recomputed_total,
            declared_total_yield_cost,
            max_path_depth,
            max_yield_budget,
        );
        let expected = expected_slug_summary_result(
            count,
            recomputed_total,
            declared_total_yield_cost,
            max_path_depth,
            max_yield_budget,
        );
        prop_assert_eq!(actual, expected);
    }

    #[test]
    fn vb_dzibx_ajc40_admission_bridge_query_summary_matches_contract_for_generated_boundaries(
        count in 0usize..=COUNT_OVER_LIMIT,
        recomputed_total in 0u64..=SMALL_TOTAL_LIMIT,
        declared_total_yield_cost in 0u64..=SMALL_TOTAL_LIMIT,
        max_path_depth in 0usize..=PATH_DEPTH_OVER_LIMIT,
        max_yield_budget in 0u64..=SMALL_TOTAL_LIMIT,
    ) {
        let actual = validate_compiled_query_summary(
            count,
            recomputed_total,
            declared_total_yield_cost,
            max_path_depth,
            max_yield_budget,
        );
        let expected = expected_query_summary_result(
            count,
            recomputed_total,
            declared_total_yield_cost,
            max_path_depth,
            max_yield_budget,
        );
        prop_assert_eq!(actual, expected);
    }
}
