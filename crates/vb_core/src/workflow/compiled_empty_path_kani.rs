#![cfg(kani)]
#![forbid(unsafe_code)]
//! Kani harness for vb-ajc40 empty-path root accessor semantics.
//!
//! Obligation ID: PO-040.

use crate::workflow::compiled_query::{
    CompiledQueries, QueryOutputType, YbBoundedQuery, validate_compiled_queries,
};
use crate::workflow::compiled_slug::{CompiledSlugs, YbBoundedSlug, validate_compiled_slugs};

fn empty_path_slug() -> YbBoundedSlug {
    YbBoundedSlug {
        path: Vec::new().into_boxed_slice(),
        yield_cost: 0,
    }
}

fn empty_path_query() -> YbBoundedQuery {
    YbBoundedQuery {
        path: Vec::new().into_boxed_slice(),
        output_type: QueryOutputType::Boolean,
        yield_cost: 0,
    }
}

/// PO-040: direct postcard-encoded empty paths are valid root accessors for
/// slug and query admission when count/path/budget checks pass.
#[kani::proof]
#[kani::unwind(8)]
fn vb_ajc40_empty_path_root_accessor() {
    match validate_compiled_slugs(
        CompiledSlugs {
            slugs: vec![empty_path_slug()].into_boxed_slice(),
            total_yield_cost: 0,
        },
        0,
    ) {
        Ok(admitted) => {
            kani::assert(admitted.len() == 1);
            kani::assert(matches!(admitted.slugs().first(), Some(item) if item.path_depth() == 0));
        }
        Err(_) => kani::assert(false),
    }

    match validate_compiled_queries(
        CompiledQueries {
            queries: vec![empty_path_query()].into_boxed_slice(),
            total_yield_cost: 0,
        },
        0,
    ) {
        Ok(admitted) => {
            assert_eq!(admitted.len(), 1);
            kani::assert(matches!(admitted.queries().first(), Some(item) if item.path_depth() == 0),
            );
        }
        Err(_) => assert!(false),
    }
}
