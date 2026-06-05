#![cfg(kani)]
#![forbid(unsafe_code)]
//! Kani harness documenting repaired vb-ajc40 total_yield_cost mismatch rejection.
//!
//! Obligation IDs: PO-020 with blocker propagation to PO-011..PO-022 and
//! PO-039..PO-043.

use crate::workflow::compiled_query::{
    CompiledQueries, QueryOutputType, QueryParseError, YbBoundedQuery, validate_compiled_queries,
};
use crate::workflow::compiled_slug::{
    CompiledSlugs, SlugParseError, YbBoundedSlug, validate_compiled_slugs,
};

fn empty_path_slug(yield_cost: u64) -> YbBoundedSlug {
    YbBoundedSlug {
        path: Vec::new().into_boxed_slice(),
        yield_cost,
    }
}

fn empty_path_query(yield_cost: u64) -> YbBoundedQuery {
    YbBoundedQuery {
        path: Vec::new().into_boxed_slice(),
        output_type: QueryOutputType::Boolean,
        yield_cost,
    }
}

/// PO-020: required behavior is mismatch rejection. The repaired production
/// implementation recomputes item yield-cost totals and rejects serialized-total
/// mismatches before admission.
#[kani::proof]
#[kani::unwind(8)]
fn vb_ajc40_total_cost_mismatch_rejected() {
    match validate_compiled_slugs(
        CompiledSlugs {
            slugs: vec![empty_path_slug(1)].into_boxed_slice(),
            total_yield_cost: 0,
        },
        1,
    ) {
        Err(SlugParseError::TotalYieldCostMismatch {
            declared,
            recomputed,
        }) => {
            assert_eq!(declared, 0);
            assert_eq!(recomputed, 1);
        }
        _ => assert!(false),
    }

    match validate_compiled_queries(
        CompiledQueries {
            queries: vec![empty_path_query(1)].into_boxed_slice(),
            total_yield_cost: 0,
        },
        1,
    ) {
        Err(QueryParseError::TotalYieldCostMismatch {
            declared,
            recomputed,
        }) => {
            assert_eq!(declared, 0);
            assert_eq!(recomputed, 1);
        }
        _ => assert!(false),
    }
}
