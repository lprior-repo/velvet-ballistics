#![forbid(unsafe_code)]
//! Query parse-failure taxonomy.
//!
//! All expected failures in the compiled-query decode pipeline are captured
//! here as an enumerable sum type.

use thiserror::Error;

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
    /// The serialized payload exceeds the maximum byte size permitted
    /// before deserialization (CW-011). Rejected up front so the decoder
    /// cannot allocate an oversized `Box<[YbBoundedQuery]>`.
    #[error("query payload too large: {size} bytes (max {max})")]
    PayloadTooLarge {
        /// Actual payload size in bytes.
        size: usize,
        /// Maximum allowed payload size in bytes.
        max: usize,
    },
    /// Recomputing the sum of all per-query yield costs overflows `u64`.
    #[error("query yield cost sum overflowed u64")]
    YieldCostOverflow,
    /// Serialized total yield cost does not match the recomputed sum.
    #[error("query total yield cost mismatch: declared {declared}, recomputed {recomputed}")]
    TotalYieldCostMismatch {
        /// Serialized total yield cost.
        declared: u64,
        /// Recomputed sum of per-query yield costs.
        recomputed: u64,
    },
}
