#![forbid(unsafe_code)]
//! Bounded compiled slug types for yield-budget-constrained workflow execution.

pub mod codec;
pub mod types;
pub mod validation;

pub use types::{
    CompiledSlugs, MAX_SLUG_PATH_SEGMENTS, MAX_SLUGS_PER_WORKFLOW, SlugParseError, YbBoundedSlug,
    YbBoundedSlugs,
};

pub use validation::{
    validate_compiled_slug_count, validate_compiled_slug_parts, validate_compiled_slug_summary,
    validate_compiled_slugs,
};

pub use codec::from_bytes_compiled_slugs;
