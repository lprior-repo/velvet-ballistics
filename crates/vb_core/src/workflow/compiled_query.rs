#![forbid(unsafe_code)]
//! Bounded compiled query types for yield-budget-constrained workflow execution.

use crate::workflow::PathSegment;
use crate::workflow::admission_kernel::{
    AdmissionKernelError, accumulate_yield_cost, validate_admission_summary,
};
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
    /// Recomputing the sum of all per-query yield costs overflowed `u64`.
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

fn checked_total_yield_cost(queries: &[YbBoundedQuery]) -> Result<u64, QueryParseError> {
    let mut total = 0_u64;
    for query in queries {
        total = accumulate_yield_cost(total, query.yield_cost)
            .map_err(|_| QueryParseError::YieldCostOverflow)?;
    }
    Ok(total)
}

/// Validates a decoded query count against the workflow admission limit.
///
/// # Errors
///
/// Returns `QueryParseError::TooManyQueries` when `count` exceeds
/// `MAX_QUERIES_PER_WORKFLOW`.
pub fn validate_compiled_query_count(count: usize) -> Result<(), QueryParseError> {
    if count > MAX_QUERIES_PER_WORKFLOW {
        return Err(QueryParseError::TooManyQueries {
            count,
            max: MAX_QUERIES_PER_WORKFLOW,
        });
    }

    Ok(())
}

/// Validates the summarized post-decode query admission facts.
///
/// # Errors
///
/// Returns the same count, path-depth, total-mismatch, and budget errors as the
/// full decoded-query validator after total recomputation succeeds.
pub fn validate_compiled_query_summary(
    count: usize,
    recomputed_total: u64,
    declared_total_yield_cost: u64,
    max_path_depth: usize,
    max_yield_budget: u64,
) -> Result<u64, QueryParseError> {
    validate_query_admission_kernel(
        count,
        recomputed_total,
        declared_total_yield_cost,
        max_path_depth,
        max_yield_budget,
    )
    .map_err(|error| {
        query_summary_error(
            error,
            count,
            recomputed_total,
            declared_total_yield_cost,
            max_path_depth,
            max_yield_budget,
        )
    })
}

fn validate_query_admission_kernel(
    count: usize,
    recomputed_total: u64,
    declared_total_yield_cost: u64,
    max_path_depth: usize,
    max_yield_budget: u64,
) -> Result<u64, AdmissionKernelError> {
    validate_admission_summary(
        count,
        MAX_QUERIES_PER_WORKFLOW,
        max_path_depth,
        MAX_QUERY_PATH_SEGMENTS,
        recomputed_total,
        declared_total_yield_cost,
        max_yield_budget,
    )
}

fn query_summary_error(
    error: AdmissionKernelError,
    count: usize,
    recomputed_total: u64,
    declared_total_yield_cost: u64,
    max_path_depth: usize,
    max_yield_budget: u64,
) -> QueryParseError {
    match error {
        AdmissionKernelError::TooManyItems => QueryParseError::TooManyQueries {
            count,
            max: MAX_QUERIES_PER_WORKFLOW,
        },
        AdmissionKernelError::PathTooDeep => QueryParseError::QueryPathTooDeep {
            depth: max_path_depth,
            max: MAX_QUERY_PATH_SEGMENTS,
        },
        AdmissionKernelError::TotalYieldCostMismatch => QueryParseError::TotalYieldCostMismatch {
            declared: declared_total_yield_cost,
            recomputed: recomputed_total,
        },
        AdmissionKernelError::YieldBudgetExceeded => QueryParseError::YbBudgetExceeded {
            total: recomputed_total,
            max: max_yield_budget,
        },
    }
}

fn max_query_path_depth(queries: &[YbBoundedQuery]) -> usize {
    let mut max_depth = 0_usize;
    for query in queries {
        max_depth = max_depth.max(query.path_depth());
    }
    max_depth
}

/// Validates decoded query parts and returns the remaining budget.
///
/// This lower-level seam avoids ownership transfer so Kani can verify the
/// post-decode admission contract over bounded fixed arrays.
///
/// # Errors
///
/// Returns the same structural, total, and budget errors as
/// `validate_compiled_queries`.
pub fn validate_compiled_query_parts(
    queries: &[YbBoundedQuery],
    declared_total_yield_cost: u64,
    max_yield_budget: u64,
) -> Result<u64, QueryParseError> {
    validate_compiled_query_count(queries.len())?;
    let max_path_depth = max_query_path_depth(queries);
    let recomputed_total = checked_total_yield_cost(queries)?;
    validate_compiled_query_summary(
        queries.len(),
        recomputed_total,
        declared_total_yield_cost,
        max_path_depth,
        max_yield_budget,
    )
}

/// Validates an already-decoded compiled query payload against structural and
/// yield-budget admission rules.
///
/// This seam is intentionally post-decode and side-effect free so verification
/// tools can prove admission behavior without symbolically executing postcard.
/// `from_bytes_compiled_queries` delegates here after successful deserialization.
///
/// # Errors
///
/// Returns `QueryParseError::TooManyQueries` if `compiled` exceeds
/// `MAX_QUERIES_PER_WORKFLOW`, `QueryParseError::QueryPathTooDeep` if any path
/// exceeds `MAX_QUERY_PATH_SEGMENTS`, `QueryParseError::YieldCostOverflow` if
/// recomputing totals overflows, `QueryParseError::TotalYieldCostMismatch` if
/// the declared total differs from the recomputed total, and
/// `QueryParseError::YbBudgetExceeded` if the recomputed total exceeds
/// `max_yield_budget`.
pub fn validate_compiled_queries(
    compiled: CompiledQueries,
    max_yield_budget: u64,
) -> Result<YbBoundedQueries, QueryParseError> {
    let remaining = validate_compiled_query_parts(
        &compiled.queries,
        compiled.total_yield_cost,
        max_yield_budget,
    )?;

    Ok(YbBoundedQueries {
        queries: compiled.queries,
        remaining_budget: remaining,
    })
}

/// Decodes compiled queries from bytes and validates them against a yield budget.
///
/// Deserializes the `CompiledQueries` structure using `postcard::from_bytes` and
/// recomputes and verifies the accumulated yield cost before checking it against
/// `max_yield_budget`.
/// Each query's path depth is also validated against `MAX_QUERY_PATH_SEGMENTS`.
///
/// # Errors
///
/// Returns `QueryParseError::Decode` if the byte sequence is not valid
/// postcard-encoded `CompiledQueries`. Returns `QueryParseError::YbBudgetExceeded`
/// if the total yield cost of all queries exceeds `max_yield_budget`. Returns
/// `QueryParseError::QueryPathTooDeep` if any query exceeds the path depth limit.
/// Returns `QueryParseError::TooManyQueries` if the number of queries exceeds
/// `MAX_QUERIES_PER_WORKFLOW`. Returns `QueryParseError::YieldCostOverflow` if
/// the recomputed yield sum overflows `u64`. Returns
/// `QueryParseError::TotalYieldCostMismatch` if the serialized total differs
/// from the recomputed sum.
#[allow(clippy::needless_pass_by_value)]
pub fn from_bytes_compiled_queries(
    bytes: &[u8],
    max_yield_budget: u64,
) -> Result<YbBoundedQueries, QueryParseError> {
    let compiled: CompiledQueries = postcard::from_bytes(bytes).map_err(QueryParseError::Decode)?;
    validate_compiled_queries(compiled, max_yield_budget)
}

#[cfg(test)]
mod tests {
    #![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::duplicated_attributes,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_strip,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
    clippy::needless_collect,
    clippy::needless_pass_by_value,
    clippy::needless_range_loop,
    clippy::needless_return,
    clippy::needless_update,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::nonminimal_bool,
    clippy::ok_expect,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::redundant_clone,
    clippy::redundant_closure,
    clippy::redundant_else,
    clippy::redundant_guards,
    clippy::redundant_locals,
    clippy::redundant_pattern_matching,
    clippy::redundant_pub_crate,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::shadow_unrelated,
    clippy::similar_names,
    clippy::single_match,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::todo,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_wraps,
    clippy::unneeded_struct_pattern,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_async,
    clippy::unused_io_amount,
    clippy::unused_self,
    clippy::unused_trait_names,
    clippy::unwrap_used,
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::useless_vec,
    clippy::vec_init_then_push,
    clippy::wildcard_enum_match_arm,
    clippy::wildcard_imports,
    dead_code,
    let_underscore_drop,
    unused_imports,
    unused_variables,
)]

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
        let err = QueryParseError::YbBudgetExceeded {
            total: 100,
            max: 50,
        };
        let msg = err.to_string();
        assert!(msg.contains("YB budget exceeded"));
        assert!(msg.contains("100"));
        assert!(msg.contains("50"));
    }

    #[test]
    fn query_parse_error_too_many_queries() {
        let err = QueryParseError::TooManyQueries {
            count: 70000,
            max: 65535,
        };
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

    #[test]
    fn query_count_helper_accepts_exact_limit_and_rejects_next() {
        assert_eq!(
            validate_compiled_query_count(MAX_QUERIES_PER_WORKFLOW),
            Ok(())
        );
        assert_eq!(
            validate_compiled_query_count(MAX_QUERIES_PER_WORKFLOW + 1),
            Err(QueryParseError::TooManyQueries {
                count: MAX_QUERIES_PER_WORKFLOW + 1,
                max: MAX_QUERIES_PER_WORKFLOW,
            })
        );
    }

    #[test]
    fn query_summary_helper_preserves_error_order_and_remaining_budget() {
        assert_eq!(
            validate_compiled_query_summary(2, 18, 18, MAX_QUERY_PATH_SEGMENTS, 25),
            Ok(7)
        );
        assert_eq!(
            validate_compiled_query_summary(2, 18, 17, MAX_QUERY_PATH_SEGMENTS, 25),
            Err(QueryParseError::TotalYieldCostMismatch {
                declared: 17,
                recomputed: 18,
            })
        );
        assert_eq!(
            validate_compiled_query_summary(2, 18, 18, MAX_QUERY_PATH_SEGMENTS + 1, 25),
            Err(QueryParseError::QueryPathTooDeep {
                depth: MAX_QUERY_PATH_SEGMENTS + 1,
                max: MAX_QUERY_PATH_SEGMENTS,
            })
        );
        assert_eq!(
            validate_compiled_query_summary(2, 18, 18, MAX_QUERY_PATH_SEGMENTS, 17),
            Err(QueryParseError::YbBudgetExceeded { total: 18, max: 17 })
        );
    }

    fn encode_queries(payload: &CompiledQueries) -> Result<Vec<u8>, String> {
        postcard::to_allocvec(payload).map_err(|err| format!("query postcard encode failed: {err}"))
    }

    fn unit_query(cost: u64) -> YbBoundedQuery {
        YbBoundedQuery {
            path: Vec::new().into_boxed_slice(),
            output_type: QueryOutputType::Boolean,
            yield_cost: cost,
        }
    }

    #[test]
    fn compiled_queries_reject_underdeclared_total() -> Result<(), String> {
        let payload = CompiledQueries {
            queries: vec![unit_query(7), unit_query(11)].into(),
            total_yield_cost: 17,
        };
        let bytes = encode_queries(&payload)?;

        let result = from_bytes_compiled_queries(&bytes, 18);

        assert_eq!(
            result,
            Err(QueryParseError::TotalYieldCostMismatch {
                declared: 17,
                recomputed: 18,
            })
        );
        Ok(())
    }

    #[test]
    fn validate_compiled_queries_rejects_underdeclared_total_without_decode() {
        let payload = CompiledQueries {
            queries: vec![unit_query(7), unit_query(11)].into(),
            total_yield_cost: 17,
        };

        let result = validate_compiled_queries(payload, 18);

        assert_eq!(
            result,
            Err(QueryParseError::TotalYieldCostMismatch {
                declared: 17,
                recomputed: 18,
            })
        );
    }

    #[test]
    fn compiled_queries_reject_overdeclared_total() -> Result<(), String> {
        let payload = CompiledQueries {
            queries: vec![unit_query(7), unit_query(11)].into(),
            total_yield_cost: 19,
        };
        let bytes = encode_queries(&payload)?;

        let result = from_bytes_compiled_queries(&bytes, 19);

        assert_eq!(
            result,
            Err(QueryParseError::TotalYieldCostMismatch {
                declared: 19,
                recomputed: 18,
            })
        );
        Ok(())
    }

    #[test]
    fn compiled_queries_reject_yield_sum_overflow() -> Result<(), String> {
        let payload = CompiledQueries {
            queries: vec![unit_query(u64::MAX), unit_query(1)].into(),
            total_yield_cost: 0,
        };
        let bytes = encode_queries(&payload)?;

        let result = from_bytes_compiled_queries(&bytes, u64::MAX);

        assert_eq!(result, Err(QueryParseError::YieldCostOverflow));
        Ok(())
    }

    #[test]
    fn compiled_queries_accept_exact_total_with_remaining_budget() -> Result<(), String> {
        let payload = CompiledQueries {
            queries: vec![unit_query(7), unit_query(11)].into(),
            total_yield_cost: 18,
        };
        let bytes = encode_queries(&payload)?;

        let result = from_bytes_compiled_queries(&bytes, 25);

        match result {
            Ok(admitted) => {
                assert_eq!(admitted.len(), 2);
                assert_eq!(admitted.remaining_budget(), 7);
                Ok(())
            }
            Err(err) => Err(format!("compiled query admission failed: {err}")),
        }
    }

    #[test]
    fn validate_compiled_queries_accepts_exact_total_without_decode() -> Result<(), String> {
        let payload = CompiledQueries {
            queries: vec![unit_query(7), unit_query(11)].into(),
            total_yield_cost: 18,
        };

        let result = validate_compiled_queries(payload, 25);

        match result {
            Ok(admitted) => {
                assert_eq!(admitted.len(), 2);
                assert_eq!(admitted.remaining_budget(), 7);
                Ok(())
            }
            Err(err) => Err(format!("compiled query admission failed: {err}")),
        }
    }

    #[test]
    fn compiled_queries_keep_empty_path_root_accessor_valid() -> Result<(), String> {
        let payload = CompiledQueries {
            queries: vec![unit_query(4)].into(),
            total_yield_cost: 4,
        };
        let bytes = encode_queries(&payload)?;

        let result = from_bytes_compiled_queries(&bytes, 4);

        match result {
            Ok(admitted) => {
                assert_eq!(admitted.len(), 1);
                assert!(matches!(admitted.queries().first(), Some(item) if item.path_depth() == 0));
                assert_eq!(admitted.remaining_budget(), 0);
                Ok(())
            }
            Err(err) => Err(format!("compiled query admission failed: {err}")),
        }
    }
}
