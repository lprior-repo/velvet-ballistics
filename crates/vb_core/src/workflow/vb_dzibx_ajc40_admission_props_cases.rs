#![forbid(unsafe_code)]
//! RPO-AJC40-002 boundary cases for direct production AJC40 validators.

use super::*;

#[test]
fn vb_dzibx_ajc40_admission_bridge_count_boundaries_are_exact() {
    assert_eq!(validate_compiled_slug_count(MAX_SLUGS_PER_WORKFLOW), Ok(()));
    assert_eq!(
        validate_compiled_slug_count(COUNT_OVER_LIMIT),
        Err(slug_too_many(COUNT_OVER_LIMIT))
    );
    assert_eq!(
        validate_compiled_query_count(MAX_QUERIES_PER_WORKFLOW),
        Ok(())
    );
    assert_eq!(
        validate_compiled_query_count(COUNT_OVER_LIMIT),
        Err(query_too_many(COUNT_OVER_LIMIT))
    );
}

#[test]
fn vb_dzibx_ajc40_admission_bridge_depth_boundaries_are_exact() {
    assert_eq!(
        validate_compiled_slug_summary(0, 0, 0, MAX_SLUG_PATH_SEGMENTS, 0),
        Ok(0)
    );
    assert_eq!(
        validate_compiled_slug_summary(0, 0, 0, PATH_DEPTH_OVER_LIMIT, 0),
        Err(slug_too_deep(PATH_DEPTH_OVER_LIMIT))
    );
    assert_eq!(
        validate_compiled_query_summary(0, 0, 0, MAX_QUERY_PATH_SEGMENTS, 0),
        Ok(0)
    );
    assert_eq!(
        validate_compiled_query_summary(0, 0, 0, PATH_DEPTH_OVER_LIMIT, 0),
        Err(query_too_deep(PATH_DEPTH_OVER_LIMIT))
    );
}

#[test]
fn vb_dzibx_ajc40_admission_bridge_error_order_is_count_depth_total_budget() {
    assert_eq!(
        validate_compiled_slug_summary(COUNT_OVER_LIMIT, 5, 6, PATH_DEPTH_OVER_LIMIT, 0),
        Err(slug_too_many(COUNT_OVER_LIMIT))
    );
    assert_eq!(
        validate_compiled_slug_summary(0, 5, 6, PATH_DEPTH_OVER_LIMIT, 0),
        Err(slug_too_deep(PATH_DEPTH_OVER_LIMIT))
    );
    assert_eq!(
        validate_compiled_slug_summary(0, 5, 6, MAX_SLUG_PATH_SEGMENTS, 0),
        Err(slug_total_mismatch(6, 5))
    );
    assert_eq!(
        validate_compiled_query_summary(COUNT_OVER_LIMIT, 5, 6, PATH_DEPTH_OVER_LIMIT, 0),
        Err(query_too_many(COUNT_OVER_LIMIT))
    );
    assert_eq!(
        validate_compiled_query_summary(0, 5, 6, PATH_DEPTH_OVER_LIMIT, 0),
        Err(query_too_deep(PATH_DEPTH_OVER_LIMIT))
    );
    assert_eq!(
        validate_compiled_query_summary(0, 5, 6, MAX_QUERY_PATH_SEGMENTS, 0),
        Err(query_total_mismatch(6, 5))
    );
}

#[test]
fn vb_dzibx_ajc40_admission_bridge_budget_and_checked_add_boundaries_are_exact() {
    assert_eq!(validate_compiled_slug_summary(0, 7, 7, 0, 9), Ok(2));
    assert_eq!(
        validate_compiled_slug_summary(0, 7, 7, 0, 6),
        Err(SlugParseError::YbBudgetExceeded { total: 7, max: 6 })
    );
    assert_eq!(validate_compiled_query_summary(0, 7, 7, 0, 9), Ok(2));
    assert_eq!(
        validate_compiled_query_summary(0, 7, 7, 0, 6),
        Err(QueryParseError::YbBudgetExceeded { total: 7, max: 6 })
    );
    assert_eq!(
        accumulate_yield_cost(u64::MAX.saturating_sub(1), 1),
        Ok(u64::MAX)
    );
    assert_eq!(
        accumulate_yield_cost(u64::MAX, 1),
        Err(AdmissionKernelError::YieldBudgetExceeded)
    );
}
