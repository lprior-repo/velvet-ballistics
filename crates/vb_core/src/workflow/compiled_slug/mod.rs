#![forbid(unsafe_code)]
//! Bounded compiled slug types for yield-budget-constrained workflow execution.

pub mod codec;
pub mod types;
pub mod validation;

pub use types::{
    CompiledSlugs, SlugParseError, YbBoundedSlug, YbBoundedSlugs, MAX_SLUGS_PER_WORKFLOW,
    MAX_SLUG_PATH_SEGMENTS,
};

pub use validation::{
    validate_compiled_slug_count, validate_compiled_slug_parts, validate_compiled_slug_summary,
    validate_compiled_slugs,
};

pub use codec::from_bytes_compiled_slugs;
