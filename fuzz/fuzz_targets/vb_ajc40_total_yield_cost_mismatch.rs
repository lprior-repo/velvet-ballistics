#![no_main]
#![forbid(unsafe_code)]
//! libFuzzer target for vb-ajc40 PO-044 / VLD-049.
//!
//! This target keeps `postcard::from_bytes` as an explicit external decoder
//! boundary and exercises the public byte APIs on both slug and query payloads.
//! Successful admission of an underdeclared, overdeclared, stale, or checked-add
//! overflow total-yield payload is a crashing counterexample.

use libfuzzer_sys::fuzz_target;
use vb_core::ids::SymbolId;
use vb_core::workflow::PathSegment;
use vb_core::workflow::compiled_query::{
    CompiledQueries, QueryOutputType, QueryParseError, YbBoundedQuery,
    from_bytes_compiled_queries,
};
use vb_core::workflow::compiled_slug::{
    CompiledSlugs, SlugParseError, YbBoundedSlug, from_bytes_compiled_slugs,
};

fuzz_target!(|data: &[u8]| {
    let budget = budget_from_prefix(data);
    let dynamic = dynamic_costs(data);

    let _ = from_bytes_compiled_slugs(data, budget);
    let _ = from_bytes_compiled_queries(data, budget);

    exercise_slug_cases(dynamic);
    exercise_query_cases(dynamic);
});

#[derive(Clone, Copy)]
struct Costs {
    first: u64,
    second: u64,
    declared_equal: u64,
    budget: u64,
}

fn budget_from_prefix(data: &[u8]) -> u64 {
    u64::from_le_bytes(prefix_8(data, 0))
}

fn dynamic_costs(data: &[u8]) -> Costs {
    let first = u64::from_le_bytes(prefix_8(data, 8)) % 1_000_000;
    let second = u64::from_le_bytes(prefix_8(data, 16)) % 1_000_000;
    let declared_equal = first.saturating_add(second);
    Costs {
        first,
        second,
        declared_equal,
        budget: declared_equal.saturating_add(1),
    }
}

fn prefix_8(data: &[u8], offset: usize) -> [u8; 8] {
    let mut bytes = [0_u8; 8];
    for (slot, value) in bytes.iter_mut().zip(data.iter().skip(offset).copied()) {
        *slot = value;
    }
    bytes
}

fn exercise_slug_cases(costs: Costs) {
    let equal = CompiledSlugs {
        slugs: slug_items(costs.first, costs.second),
        total_yield_cost: costs.declared_equal,
    };
    expect_slug_admitted(&equal, costs.budget);

    let underdeclared = CompiledSlugs {
        slugs: slug_items(3, 5),
        total_yield_cost: 7,
    };
    expect_slug_total_mismatch(&underdeclared, 8);

    let overdeclared = CompiledSlugs {
        slugs: slug_items(3, 5),
        total_yield_cost: 9,
    };
    expect_slug_total_mismatch(&overdeclared, 9);

    let stale = CompiledSlugs {
        slugs: slug_items(costs.first.saturating_add(1), costs.second),
        total_yield_cost: costs.declared_equal,
    };
    expect_slug_total_mismatch(&stale, costs.budget.saturating_add(1));

    let overflow = CompiledSlugs {
        slugs: slug_items(u64::MAX, 1),
        total_yield_cost: u64::MAX,
    };
    expect_slug_overflow(&overflow);
}

fn exercise_query_cases(costs: Costs) {
    let equal = CompiledQueries {
        queries: query_items(costs.first, costs.second),
        total_yield_cost: costs.declared_equal,
    };
    expect_query_admitted(&equal, costs.budget);

    let underdeclared = CompiledQueries {
        queries: query_items(3, 5),
        total_yield_cost: 7,
    };
    expect_query_total_mismatch(&underdeclared, 8);

    let overdeclared = CompiledQueries {
        queries: query_items(3, 5),
        total_yield_cost: 9,
    };
    expect_query_total_mismatch(&overdeclared, 9);

    let stale = CompiledQueries {
        queries: query_items(costs.first.saturating_add(1), costs.second),
        total_yield_cost: costs.declared_equal,
    };
    expect_query_total_mismatch(&stale, costs.budget.saturating_add(1));

    let overflow = CompiledQueries {
        queries: query_items(u64::MAX, 1),
        total_yield_cost: u64::MAX,
    };
    expect_query_overflow(&overflow);
}

fn slug_items(first: u64, second: u64) -> Box<[YbBoundedSlug]> {
    vec![
        YbBoundedSlug {
            path: Vec::new().into_boxed_slice(),
            yield_cost: first,
        },
        YbBoundedSlug {
            path: vec![PathSegment::Field(SymbolId::new(1))].into_boxed_slice(),
            yield_cost: second,
        },
    ]
    .into_boxed_slice()
}

fn query_items(first: u64, second: u64) -> Box<[YbBoundedQuery]> {
    vec![
        YbBoundedQuery {
            path: Vec::new().into_boxed_slice(),
            output_type: QueryOutputType::Boolean,
            yield_cost: first,
        },
        YbBoundedQuery {
            path: vec![PathSegment::Index(0)].into_boxed_slice(),
            output_type: QueryOutputType::Integer,
            yield_cost: second,
        },
    ]
    .into_boxed_slice()
}

fn expect_slug_admitted(payload: &CompiledSlugs, budget: u64) {
    if let Ok(bytes) = postcard::to_allocvec(payload) {
        if from_bytes_compiled_slugs(&bytes, budget).is_err() {
            std::process::abort();
        }
    }
}

fn expect_query_admitted(payload: &CompiledQueries, budget: u64) {
    if let Ok(bytes) = postcard::to_allocvec(payload) {
        if from_bytes_compiled_queries(&bytes, budget).is_err() {
            std::process::abort();
        }
    }
}

fn expect_slug_total_mismatch(payload: &CompiledSlugs, budget: u64) {
    if let Ok(bytes) = postcard::to_allocvec(payload) {
        match from_bytes_compiled_slugs(&bytes, budget) {
            Err(SlugParseError::TotalYieldCostMismatch { .. }) => {}
            _ => std::process::abort(),
        }
    }
}

fn expect_query_total_mismatch(payload: &CompiledQueries, budget: u64) {
    if let Ok(bytes) = postcard::to_allocvec(payload) {
        match from_bytes_compiled_queries(&bytes, budget) {
            Err(QueryParseError::TotalYieldCostMismatch { .. }) => {}
            _ => std::process::abort(),
        }
    }
}

fn expect_slug_overflow(payload: &CompiledSlugs) {
    if let Ok(bytes) = postcard::to_allocvec(payload) {
        match from_bytes_compiled_slugs(&bytes, u64::MAX) {
            Err(SlugParseError::YieldCostOverflow) => {}
            _ => std::process::abort(),
        }
    }
}

fn expect_query_overflow(payload: &CompiledQueries) {
    if let Ok(bytes) = postcard::to_allocvec(payload) {
        match from_bytes_compiled_queries(&bytes, u64::MAX) {
            Err(QueryParseError::YieldCostOverflow) => {}
            _ => std::process::abort(),
        }
    }
}
