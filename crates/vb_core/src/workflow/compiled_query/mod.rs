#![forbid(unsafe_code)]
//! Bounded compiled query types for yield-budget-constrained workflow execution.
//!
//! Module structure:
//! - [`domain`] — core types and hard limits
//! - [`errors`] — `QueryParseError` taxonomy
//! - [`validation`] — post-decode admission pipeline
//!
//! Public API entry points are re-exported at the `compiled_query` level for
//! ergonomic consumption by callers and verification harnesses.

pub mod domain;
pub mod errors;
pub mod validation;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

// Re-export domain types at module level for backward-compatible paths.
pub use domain::{
    CompiledQueries, MAX_QUERIES_PER_WORKFLOW, MAX_QUERY_PATH_SEGMENTS, QueryOutputType,
    YbBoundedQueries, YbBoundedQuery,
};

// Re-export error taxonomy.
pub use errors::QueryParseError;

// Re-export validation pipeline.
pub use validation::{
    validate_compiled_queries, validate_compiled_query_count, validate_compiled_query_parts,
    validate_compiled_query_summary,
};

/// Maximum allowed payload size for a compiled-query blob.
///
/// CW-011: this preflight is computed from `MAX_QUERIES_PER_WORKFLOW` and the
/// worst-case encoded size of a single query (full path + varint yield cost).
/// Any payload larger than this cannot decode into a value that survives
/// admission, so we reject it before `postcard::from_bytes` allocates a
/// potentially enormous `Box<[YbBoundedQuery]>`.
pub const MAX_QUERY_PAYLOAD_BYTES: usize =
    // Header: varint(MAX_QUERIES_PER_WORKFLOW) (3 bytes max) + u64 total_yield_cost (9 bytes max).
    12
        + MAX_QUERIES_PER_WORKFLOW
            * (
                // Per-query worst case: varint path length (1 byte) +
                // MAX_QUERY_PATH_SEGMENTS segments (each 1-byte variant + 5-byte u32 varint) +
                // 1-byte QueryOutputType variant + u64 yield_cost (9 bytes max).
                1 + MAX_QUERY_PATH_SEGMENTS * 6 + 1 + 9
            );

/// Decodes compiled queries from bytes and validates them against a yield budget.
///
/// Deserializes the `CompiledQueries` structure using `postcard::from_bytes` and
/// recomputes and verifies the accumulated yield cost before checking it against
/// `max_yield_budget`.
/// Each query's path depth is also validated against `MAX_QUERY_PATH_SEGMENTS`.
///
/// # Errors
///
/// Returns `QueryParseError::PayloadTooLarge` if the input bytes exceed
/// `MAX_QUERY_PAYLOAD_BYTES` (CW-011: prevent pre-validation allocation
/// blowup). Returns `QueryParseError::Decode` if the byte sequence is not
/// valid postcard-encoded `CompiledQueries`. Returns
/// `QueryParseError::YbBudgetExceeded` if the total yield cost of all queries
/// exceeds `max_yield_budget`. Returns `QueryParseError::QueryPathTooDeep` if
/// any query exceeds the path depth limit. Returns
/// `QueryParseError::TooManyQueries` if the number of queries exceeds
/// `MAX_QUERIES_PER_WORKFLOW`. Returns `QueryParseError::YieldCostOverflow` if
/// the recomputed yield sum overflows `u64`. Returns
/// `QueryParseError::TotalYieldCostMismatch` if the serialized total differs
/// from the recomputed sum.
#[allow(clippy::needless_pass_by_value)]
pub fn from_bytes_compiled_queries(
    bytes: &[u8],
    max_yield_budget: u64,
) -> Result<YbBoundedQueries, QueryParseError> {
    if bytes.len() > MAX_QUERY_PAYLOAD_BYTES {
        return Err(QueryParseError::PayloadTooLarge {
            size: bytes.len(),
            max: MAX_QUERY_PAYLOAD_BYTES,
        });
    }
    let compiled: CompiledQueries = postcard::from_bytes(bytes).map_err(QueryParseError::Decode)?;
    validate_compiled_queries(compiled, max_yield_budget)
}
