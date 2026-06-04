#![forbid(unsafe_code)]
//! Bounded compiled query types for yield-budget-constrained workflow execution.

use crate::workflow::PathSegment;
use serde::{Deserialize, Serialize};
use thiserror::Error;

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YbBoundedQueries {
    /// Decoded queries with bounded paths, output types, and yield costs.
    queries: Box<[YbBoundedQuery]>,
    /// Remaining yield budget after decoding and validation.
    remaining_budget: u64,
}

impl YbBoundedQueries {
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

/// Query parse failures.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum QueryParseError {
    /// Deserialization failed.
    #[error("query deserialization failed: {0}")]
    Decode(#[source] postcard::Error),
    /// The accumulated yield cost of all queries exceeds the maximum budget.
    #[error("YB budget exceeded: total {total} exceeds max {max}")]
    YbBudgetExceeded {
        /// Sum of all query yield costs.
        total: u64,
        /// Caller-supplied maximum budget.
        max: u64,
    },
    /// A query path exceeds the maximum allowed depth.
    #[error("query path too deep: {depth} segments (max {max})")]
    QueryPathTooDeep {
        /// Actual path depth.
        depth: usize,
        /// Maximum allowed depth.
        max: usize,
    },
    /// The number of queries exceeds the hard limit.
    #[error("too many queries: {count} (max {max})")]
    TooManyQueries {
        /// Actual query count.
        count: usize,
        /// Maximum allowed queries.
        max: usize,
    },
}

/// Decodes compiled queries from bytes and validates them against a yield budget.
///
/// Deserializes the `CompiledQueries` structure using `postcard::from_bytes` and
/// checks that the accumulated yield cost does not exceed `max_yield_budget`.
/// Each query's path depth is also validated against `MAX_QUERY_PATH_SEGMENTS`.
///
/// # Errors
///
/// Returns `QueryParseError::Decode` if the byte sequence is not valid
/// postcard-encoded `CompiledQueries`. Returns `QueryParseError::YbBudgetExceeded`
/// if the total yield cost of all queries exceeds `max_yield_budget`. Returns
/// `QueryParseError::QueryPathTooDeep` if any query exceeds the path depth limit.
/// Returns `QueryParseError::TooManyQueries` if the number of queries exceeds
/// `MAX_QUERIES_PER_WORKFLOW`.
#[allow(clippy::needless_pass_by_value)]
pub fn from_bytes_compiled_queries(
    bytes: &[u8],
    max_yield_budget: u64,
) -> Result<YbBoundedQueries, QueryParseError> {
    let compiled: CompiledQueries =
        postcard::from_bytes(bytes).map_err(QueryParseError::Decode)?;

    if compiled.queries.len() > MAX_QUERIES_PER_WORKFLOW {
        return Err(QueryParseError::TooManyQueries {
            count: compiled.queries.len(),
            max: MAX_QUERIES_PER_WORKFLOW,
        });
    }

    for query in compiled.queries.iter() {
        if query.is_path_too_deep() {
            return Err(QueryParseError::QueryPathTooDeep {
                depth: query.path_depth(),
                max: MAX_QUERY_PATH_SEGMENTS,
            });
        }
    }

    if compiled.total_yield_cost > max_yield_budget {
        return Err(QueryParseError::YbBudgetExceeded {
            total: compiled.total_yield_cost,
            max: max_yield_budget,
        });
    }

    let remaining = max_yield_budget
        .checked_sub(compiled.total_yield_cost)
        .unwrap_or(0);

    Ok(YbBoundedQueries {
        queries: compiled.queries,
        remaining_budget: remaining,
    })
}

#[cfg(test)]
mod tests {
    use crate::ids::SymbolId;

    use super::*;

    #[test]
    fn query_path_depth_validation() {
        let shallow = YbBoundedQuery {
            path: vec![PathSegment::Field(SymbolId::new(1)), PathSegment::Index(0)].into(),
            output_type: QueryOutputType::Boolean,
            yield_cost: 10,
        };
        assert!(!shallow.is_path_too_deep());

        let deep: Box<[PathSegment]> = (0..20)
            .map(|i| PathSegment::Field(SymbolId::new(i as u32)))
            .collect();
        let deep_query = YbBoundedQuery {
            path: deep,
            output_type: QueryOutputType::Integer,
            yield_cost: 10,
        };
        assert!(deep_query.is_path_too_deep());
    }

    #[test]
    fn query_is_empty_and_len() {
        let empty_queries: YbBoundedQueries = YbBoundedQueries {
            queries: vec![].into(),
            remaining_budget: 100,
        };
        assert!(empty_queries.is_empty());
        assert_eq!(empty_queries.len(), 0);
        assert_eq!(empty_queries.remaining_budget(), 100);
    }

    #[test]
    fn query_len_and_remaining_budget() {
        let query = YbBoundedQuery {
            path: vec![PathSegment::Field(SymbolId::new(1))].into(),
            output_type: QueryOutputType::String,
            yield_cost: 25,
        };
        let bounded_queries = YbBoundedQueries {
            queries: vec![query].into(),
            remaining_budget: 75,
        };
        assert!(!bounded_queries.is_empty());
        assert_eq!(bounded_queries.len(), 1);
        assert_eq!(bounded_queries.remaining_budget(), 75);
    }

    #[test]
    fn query_parse_error_display() {
        let err = QueryParseError::YbBudgetExceeded { total: 100, max: 50 };
        let msg = err.to_string();
        assert!(msg.contains("YB budget exceeded"));
        assert!(msg.contains("100"));
        assert!(msg.contains("50"));
    }

    #[test]
    fn query_parse_error_too_many_queries() {
        let err = QueryParseError::TooManyQueries { count: 70000, max: 65535 };
        let msg = err.to_string();
        assert!(msg.contains("too many queries"));
        assert!(msg.contains("70000"));
    }

    #[test]
    fn query_parse_error_path_too_deep() {
        let err = QueryParseError::QueryPathTooDeep { depth: 20, max: 16 };
        let msg = err.to_string();
        assert!(msg.contains("query path too deep"));
        assert!(msg.contains("20"));
    }

    #[test]
    fn query_output_types() {
        let query_bool = YbBoundedQuery {
            path: vec![PathSegment::Field(SymbolId::new(1))].into(),
            output_type: QueryOutputType::Boolean,
            yield_cost: 5,
        };
        let query_int = YbBoundedQuery {
            path: vec![PathSegment::Field(SymbolId::new(1))].into(),
            output_type: QueryOutputType::Integer,
            yield_cost: 5,
        };
        let query_float = YbBoundedQuery {
            path: vec![PathSegment::Field(SymbolId::new(1))].into(),
            output_type: QueryOutputType::Float,
            yield_cost: 5,
        };
        let query_string = YbBoundedQuery {
            path: vec![PathSegment::Field(SymbolId::new(1))].into(),
            output_type: QueryOutputType::String,
            yield_cost: 5,
        };
        let query_list = YbBoundedQuery {
            path: vec![PathSegment::Field(SymbolId::new(1))].into(),
            output_type: QueryOutputType::List,
            yield_cost: 5,
        };
        let query_obj = YbBoundedQuery {
            path: vec![PathSegment::Field(SymbolId::new(1))].into(),
            output_type: QueryOutputType::Object,
            yield_cost: 5,
        };

        assert_eq!(query_bool.output_type, QueryOutputType::Boolean);
        assert_eq!(query_int.output_type, QueryOutputType::Integer);
        assert_eq!(query_float.output_type, QueryOutputType::Float);
        assert_eq!(query_string.output_type, QueryOutputType::String);
        assert_eq!(query_list.output_type, QueryOutputType::List);
        assert_eq!(query_obj.output_type, QueryOutputType::Object);
    }
}
