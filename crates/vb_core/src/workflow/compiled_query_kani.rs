#![cfg(kani)]
#![forbid(unsafe_code)]
//! Kani harnesses for vb-ajc40 compiled query admission obligations.
//!
//! Obligation IDs: PO-007, PO-016, PO-028, PO-036.

use crate::ids::SymbolId;
use crate::workflow::PathSegment;
use crate::workflow::compiled_query::{
    CompiledQueries, MAX_QUERIES_PER_WORKFLOW, MAX_QUERY_PATH_SEGMENTS, QueryOutputType,
    QueryParseError, YbBoundedQuery, from_bytes_compiled_queries, validate_compiled_queries,
    validate_compiled_query_count, validate_compiled_query_summary,
};

const SYMBOL_ZERO: SymbolId = SymbolId::new(0);
const SEMANTIC_ITEM_BOUND: usize = 64;
const QUERY_COUNT_OVER_LIMIT: usize = 65_536;
const QUERY_PATH_DEPTH_OVER_LIMIT: usize = 17;

fn field_path(len: usize) -> Box<[PathSegment]> {
    vec![PathSegment::Field(SYMBOL_ZERO); len].into_boxed_slice()
}

fn query(path_len: usize, yield_cost: u64) -> YbBoundedQuery {
    YbBoundedQuery {
        path: field_path(path_len),
        output_type: QueryOutputType::Boolean,
        yield_cost,
    }
}

/// PO-007: finite public decode smoke/non-closure cases over the production
/// byte boundary. This deliberately does not claim arbitrary hostile-byte or
/// postcard parser-space closure; hostile bytes are covered by proptest,
/// cargo-fuzz, behavior tests, and PO-044.
#[kani::proof]
#[kani::unwind(16)]
fn vb_ajc40_query_decode_smoke_cases() {
    let budget = 0_u64;

    // Malformed public decode case: an invalid leading varint/tag must be
    // propagated as Decode and must not construct an admitted value.
    match from_bytes_compiled_queries(&[0xff], budget) {
        Err(QueryParseError::Decode(_)) => {}
        _ => assert!(false),
    }

    // Canonical decoded-public-shape cases are checked through the production
    // post-decode public validator. This avoids claiming Kani parser-space
    // closure over postcard internals while still asserting public Ok shape.
    match validate_compiled_queries(
        CompiledQueries {
            queries: Vec::new().into_boxed_slice(),
            total_yield_cost: 0,
        },
        budget,
    ) {
        Ok(admitted) => {
            assert!(admitted.is_empty());
            assert_eq!(admitted.len(), 0);
            assert_eq!(admitted.remaining_budget(), 0);
        }
        Err(_) => assert!(false),
    }

    match validate_compiled_queries(
        CompiledQueries {
            queries: vec![query(0, 0)].into_boxed_slice(),
            total_yield_cost: 0,
        },
        budget,
    ) {
        Ok(admitted) => {
            assert_eq!(admitted.len(), 1);
            assert_eq!(admitted.remaining_budget(), 0);
        }
        Err(_) => assert!(false),
    }
}

/// Backward-compatible State 12 harness name for the same PO-007 smoke scope.
#[kani::proof]
#[kani::unwind(16)]
fn vb_ajc40_query_decode_bytes() {
    vb_ajc40_query_decode_smoke_cases();
}

/// PO-007: arbitrary decoded query payloads in 0..=64 never panic the
/// production post-decode admission seam. Hostile byte behavior remains covered
/// by fuzz/proptest over `from_bytes_compiled_queries` rather than Kani postcard
/// symbolic execution.
#[kani::proof]
#[kani::unwind(66)]
fn vb_ajc40_query_post_decode_semantics() {
    let len: usize = kani::any();
    let declared_total: u64 = kani::any();
    let budget: u64 = kani::any();
    kani::assume(len <= SEMANTIC_ITEM_BOUND);

    let recomputed_total: u64 = kani::any();
    let max_path_depth: usize = kani::any();
    kani::assume(max_path_depth <= QUERY_PATH_DEPTH_OVER_LIMIT);

    match validate_compiled_query_summary(
        len,
        recomputed_total,
        declared_total,
        max_path_depth,
        budget,
    ) {
        Ok(remaining_budget) => {
            assert!(len <= MAX_QUERIES_PER_WORKFLOW);
            assert!(remaining_budget <= budget);
            assert!(max_path_depth <= MAX_QUERY_PATH_SEGMENTS);
        }
        Err(QueryParseError::Decode(_))
        | Err(QueryParseError::YbBudgetExceeded { .. })
        | Err(QueryParseError::QueryPathTooDeep { .. })
        | Err(QueryParseError::TooManyQueries { .. })
        | Err(QueryParseError::YieldCostOverflow)
        | Err(QueryParseError::TotalYieldCostMismatch { .. }) => {}
    }
}

/// PO-016: boundary budget arithmetic over the production parser surface.
///
/// This is bound to the repaired production contract: serialized totals are
/// accepted only when they match checked recomputation of item `yield_cost`s.
#[kani::proof]
#[kani::unwind(8)]
fn vb_ajc40_query_budget_boundaries() {
    match validate_compiled_queries(
        CompiledQueries {
            queries: vec![query(0, 0)].into_boxed_slice(),
            total_yield_cost: 0,
        },
        0,
    ) {
        Ok(admitted) => {
            assert_eq!(admitted.len(), 1);
            assert_eq!(admitted.remaining_budget(), 0);
        }
        Err(_) => assert!(false),
    }

    match validate_compiled_queries(
        CompiledQueries {
            queries: vec![query(0, 1)].into_boxed_slice(),
            total_yield_cost: 1,
        },
        1,
    ) {
        Ok(admitted) => {
            assert_eq!(admitted.len(), 1);
            assert_eq!(admitted.remaining_budget(), 0);
        }
        Err(_) => assert!(false),
    }

    match validate_compiled_queries(
        CompiledQueries {
            queries: vec![query(0, 1)].into_boxed_slice(),
            total_yield_cost: 1,
        },
        0,
    ) {
        Err(QueryParseError::YbBudgetExceeded { total, max }) => {
            assert_eq!(total, 1);
            assert_eq!(max, 0);
        }
        _ => assert!(false),
    }
}

/// PO-028: depth 16 is admitted subject to budget; depth 17 rejects before budget.
#[kani::proof]
#[kani::unwind(24)]
fn vb_ajc40_query_path_depth_16_17() {
    match validate_compiled_queries(
        CompiledQueries {
            queries: vec![query(MAX_QUERY_PATH_SEGMENTS, 0)].into_boxed_slice(),
            total_yield_cost: 0,
        },
        0,
    ) {
        Ok(admitted) => assert_eq!(admitted.len(), 1),
        Err(_) => assert!(false),
    }

    match validate_compiled_queries(
        CompiledQueries {
            queries: vec![query(MAX_QUERY_PATH_SEGMENTS + 1, 0)].into_boxed_slice(),
            total_yield_cost: 0,
        },
        0,
    ) {
        Err(QueryParseError::QueryPathTooDeep { depth, max }) => {
            assert_eq!(depth, MAX_QUERY_PATH_SEGMENTS + 1);
            assert_eq!(max, MAX_QUERY_PATH_SEGMENTS);
        }
        _ => assert!(false),
    }
}

/// PO-036: exact 65_535/65_536 count boundary fixture.
///
/// This proof uses the production count helper directly so Kani does not need
/// to allocate a 65k decoded payload.
#[kani::proof]
#[kani::unwind(4)]
fn vb_ajc40_query_count_65535_65536() {
    assert!(validate_compiled_query_count(MAX_QUERIES_PER_WORKFLOW).is_ok());

    match validate_compiled_query_count(QUERY_COUNT_OVER_LIMIT) {
        Err(QueryParseError::TooManyQueries { count, max }) => {
            assert_eq!(count, QUERY_COUNT_OVER_LIMIT);
            assert_eq!(max, MAX_QUERIES_PER_WORKFLOW);
        }
        _ => assert!(false),
    }
}
