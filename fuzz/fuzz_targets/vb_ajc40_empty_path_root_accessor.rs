#![no_main]
#![forbid(unsafe_code)]
//! libFuzzer target for vb-ajc40 PO-043.
//!
//! The target exercises arbitrary hostile bytes plus deterministic empty-path
//! root-accessor seed payloads on each invocation so the root semantics remain
//! covered even before corpus minimization materializes seed files.

use libfuzzer_sys::fuzz_target;
use vb_core::workflow::compiled_query::{
    CompiledQueries, QueryOutputType, YbBoundedQuery, from_bytes_compiled_queries,
};
use vb_core::workflow::compiled_slug::{CompiledSlugs, YbBoundedSlug, from_bytes_compiled_slugs};

fuzz_target!(|data: &[u8]| {
    let budget = budget_from_prefix(data);
    let _ = from_bytes_compiled_slugs(data, budget);
    let _ = from_bytes_compiled_queries(data, budget);
    exercise_empty_path_seeds();
});

fn budget_from_prefix(data: &[u8]) -> u64 {
    let mut bytes = [0_u8; 8];
    for (slot, value) in bytes.iter_mut().zip(data.iter().copied()) {
        *slot = value;
    }
    u64::from_le_bytes(bytes)
}

fn exercise_empty_path_seeds() {
    let slug_payload = CompiledSlugs {
        slugs: vec![YbBoundedSlug {
            path: Vec::new().into_boxed_slice(),
            yield_cost: 0,
        }]
        .into(),
        total_yield_cost: 0,
    };
    if let Ok(bytes) = postcard::to_allocvec(&slug_payload) {
        let _ = from_bytes_compiled_slugs(&bytes, 0);
    }

    let query_payload = CompiledQueries {
        queries: vec![YbBoundedQuery {
            path: Vec::new().into_boxed_slice(),
            output_type: QueryOutputType::Boolean,
            yield_cost: 0,
        }]
        .into(),
        total_yield_cost: 0,
    };
    if let Ok(bytes) = postcard::to_allocvec(&query_payload) {
        let _ = from_bytes_compiled_queries(&bytes, 0);
    }
}
