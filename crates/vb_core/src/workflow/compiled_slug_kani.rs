#![cfg(kani)]
#![forbid(unsafe_code)]
//! Kani harnesses for vb-ajc40 compiled slug admission obligations.
//!
//! Obligation IDs: PO-002, PO-012, PO-024, PO-032.

use crate::ids::SymbolId;
use crate::workflow::PathSegment;
use crate::workflow::compiled_slug::{
    CompiledSlugs, MAX_SLUG_PATH_SEGMENTS, MAX_SLUGS_PER_WORKFLOW, SlugParseError, YbBoundedSlug,
    from_bytes_compiled_slugs, validate_compiled_slug_count, validate_compiled_slug_summary,
    validate_compiled_slugs,
};

const SYMBOL_ZERO: SymbolId = SymbolId::new(0);
const SEMANTIC_ITEM_BOUND: usize = 64;
const SLUG_COUNT_OVER_LIMIT: usize = 65_536;
const SLUG_PATH_DEPTH_OVER_LIMIT: usize = 17;

fn field_path(len: usize) -> Box<[PathSegment]> {
    vec![PathSegment::Field(SYMBOL_ZERO); len].into_boxed_slice()
}

fn slug(path_len: usize, yield_cost: u64) -> YbBoundedSlug {
    YbBoundedSlug {
        path: field_path(path_len),
        yield_cost,
    }
}

/// PO-002: finite public decode smoke/non-closure cases over the production
/// byte boundary. This deliberately does not claim arbitrary hostile-byte or
/// postcard parser-space closure; hostile bytes are covered by proptest,
/// cargo-fuzz, behavior tests, and PO-044.
#[kani::proof]
#[kani::unwind(16)]
fn vb_ajc40_slug_decode_smoke_cases() {
    let budget = 0_u64;

    // Malformed public decode case: an invalid leading varint/tag must be
    // propagated as Decode and must not construct an admitted value.
    match from_bytes_compiled_slugs(&[0xff], budget) {
        Err(SlugParseError::Decode(_)) => {}
        _ => assert!(false),
    }

    // Canonical decoded-public-shape cases are checked through the production
    // post-decode public validator. This avoids claiming Kani parser-space
    // closure over postcard internals while still asserting public Ok shape.
    match validate_compiled_slugs(
        CompiledSlugs {
            slugs: Vec::new().into_boxed_slice(),
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

    match validate_compiled_slugs(
        CompiledSlugs {
            slugs: vec![slug(0, 0)].into_boxed_slice(),
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

/// Backward-compatible State 12 harness name for the same PO-002 smoke scope.
#[kani::proof]
#[kani::unwind(16)]
fn vb_ajc40_slug_decode_bytes() {
    vb_ajc40_slug_decode_smoke_cases();
}

/// PO-002: arbitrary decoded slug payloads in 0..=64 never panic the
/// production post-decode admission seam. Hostile byte behavior remains covered
/// by fuzz/proptest over `from_bytes_compiled_slugs` rather than Kani postcard
/// symbolic execution.
#[kani::proof]
#[kani::unwind(66)]
fn vb_ajc40_slug_post_decode_semantics() {
    let len: usize = kani::any();
    let declared_total: u64 = kani::any();
    let budget: u64 = kani::any();
    kani::assume(len <= SEMANTIC_ITEM_BOUND);

    let recomputed_total: u64 = kani::any();
    let max_path_depth: usize = kani::any();
    kani::assume(max_path_depth <= SLUG_PATH_DEPTH_OVER_LIMIT);

    match validate_compiled_slug_summary(
        len,
        recomputed_total,
        declared_total,
        max_path_depth,
        budget,
    ) {
        Ok(remaining_budget) => {
            assert!(len <= MAX_SLUGS_PER_WORKFLOW);
            assert!(remaining_budget <= budget);
            assert!(max_path_depth <= MAX_SLUG_PATH_SEGMENTS);
        }
        Err(SlugParseError::Decode(_))
        | Err(SlugParseError::YbBudgetExceeded { .. })
        | Err(SlugParseError::SlugPathTooDeep { .. })
        | Err(SlugParseError::TooManySlugs { .. })
        | Err(SlugParseError::YieldCostOverflow)
        | Err(SlugParseError::TotalYieldCostMismatch { .. }) => {}
    }
}

/// PO-012: boundary budget arithmetic over the production parser surface.
///
/// This is bound to the repaired production contract: serialized totals are
/// accepted only when they match checked recomputation of item `yield_cost`s.
#[kani::proof]
#[kani::unwind(8)]
fn vb_ajc40_slug_budget_boundaries() {
    match validate_compiled_slugs(
        CompiledSlugs {
            slugs: vec![slug(0, 0)].into_boxed_slice(),
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

    match validate_compiled_slugs(
        CompiledSlugs {
            slugs: vec![slug(0, 1)].into_boxed_slice(),
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

    match validate_compiled_slugs(
        CompiledSlugs {
            slugs: vec![slug(0, 1)].into_boxed_slice(),
            total_yield_cost: 1,
        },
        0,
    ) {
        Err(SlugParseError::YbBudgetExceeded { total, max }) => {
            assert_eq!(total, 1);
            assert_eq!(max, 0);
        }
        _ => assert!(false),
    }
}

/// PO-024: depth 16 is admitted subject to budget; depth 17 rejects before budget.
#[kani::proof]
#[kani::unwind(24)]
fn vb_ajc40_slug_path_depth_16_17() {
    match validate_compiled_slugs(
        CompiledSlugs {
            slugs: vec![slug(MAX_SLUG_PATH_SEGMENTS, 0)].into_boxed_slice(),
            total_yield_cost: 0,
        },
        0,
    ) {
        Ok(admitted) => assert_eq!(admitted.len(), 1),
        Err(_) => assert!(false),
    }

    match validate_compiled_slugs(
        CompiledSlugs {
            slugs: vec![slug(MAX_SLUG_PATH_SEGMENTS + 1, 0)].into_boxed_slice(),
            total_yield_cost: 0,
        },
        0,
    ) {
        Err(SlugParseError::SlugPathTooDeep { depth, max }) => {
            assert_eq!(depth, MAX_SLUG_PATH_SEGMENTS + 1);
            assert_eq!(max, MAX_SLUG_PATH_SEGMENTS);
        }
        _ => assert!(false),
    }
}

/// PO-032: exact 65_535/65_536 count boundary fixture.
///
/// This proof uses the production count helper directly so Kani does not need
/// to allocate a 65k decoded payload.
#[kani::proof]
#[kani::unwind(2)]
fn vb_ajc40_slug_count_65535_65536() {
    assert!(validate_compiled_slug_count(MAX_SLUGS_PER_WORKFLOW).is_ok());

    match validate_compiled_slug_count(SLUG_COUNT_OVER_LIMIT) {
        Err(SlugParseError::TooManySlugs { count, max }) => {
            assert_eq!(count, SLUG_COUNT_OVER_LIMIT);
            assert_eq!(max, MAX_SLUGS_PER_WORKFLOW);
        }
        _ => assert!(false),
    }
}
