#![forbid(unsafe_code)]
//! Domain types for bounded compiled queries.
//!
//! Core models: `QueryOutputType`, `YbBoundedQuery`, `CompiledQueries`, and
//! `YbBoundedQueries` (admitted container with remaining budget).
//!
//! Hard limits are exported as constants for validation modules to reference.

use crate::workflow::PathSegment;
use serde::{Deserialize, Serialize};

/// Maximum number of queries permitted in a single workflow admission.
pub const MAX_QUERIES_PER_WORKFLOW: usize = 65_535;

/// Maximum number of path segments in a single query.
pub const MAX_QUERY_PATH_SEGMENTS: usize = 16;

/// Runtime output type annotation for a compiled query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum QueryOutputType {
    /// Query returns a boolean value.
    Boolean,
    /// Query returns an integer value.
    Integer,
    /// Query returns a float value.
    Float,
    /// Query returns a string value.
    String,
    /// Query returns a list value.
    List,
    /// Query returns an object value.
    Object,
}

/// A compiled query with a yield cost, a bounded accessor path, and an output type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct YbBoundedQuery {
    /// Accessor path from root slot to the target value.
    pub path: Box<[PathSegment]>,
    /// Runtime type of the query result.
    pub output_type: QueryOutputType,
    /// Computational cost charged against the yield budget upon execution.
    pub yield_cost: u64,
}

impl YbBoundedQuery {
    /// Validates the query path depth against the hard limit.
    #[must_use]
    pub fn path_depth(&self) -> usize {
        self.path.len()
    }

    /// Returns `true` if the query path exceeds the maximum allowed depth.
    #[must_use]
    pub fn is_path_too_deep(&self) -> bool {
        self.path.len() > MAX_QUERY_PATH_SEGMENTS
    }
}

/// The serialized format for a collection of compiled queries, used by
/// `from_bytes_compiled_queries` as the immediate decode target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompiledQueries {
    /// Individual compiled queries.
    pub queries: Box<[YbBoundedQuery]>,
    /// Explicit sum of all yield costs across queries, verified at decode time.
    pub total_yield_cost: u64,
}

/// Container for decoded queries with remaining budget tracking.
///
/// This type represents queries that have passed all admission checks and
/// carries the remaining yield budget for the workflow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YbBoundedQueries {
    /// Decoded queries with bounded paths, output types, and yield costs.
    queries: Box<[YbBoundedQuery]>,
    /// Remaining yield budget after decoding and validation.
    remaining_budget: u64,
}

impl YbBoundedQueries {
    /// Constructs an admitted query set (internal to vb_core).
    pub(crate) fn new(queries: Box<[YbBoundedQuery]>, remaining_budget: u64) -> Self {
        Self {
            queries,
            remaining_budget,
        }
    }

    /// Returns a reference to the contained queries.
    #[must_use]
    pub fn queries(&self) -> &[YbBoundedQuery] {
        &self.queries
    }

    /// Returns the remaining yield budget.
    #[must_use]
    pub const fn remaining_budget(&self) -> u64 {
        self.remaining_budget
    }

    /// Returns the number of queries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.queries.len()
    }

    /// Returns `true` if there are no queries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.queries.is_empty()
    }
}
