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
    unused_variables
)]

use crate::ids::SymbolId;
use crate::workflow::PathSegment;

use super::domain::{
    CompiledQueries, MAX_QUERIES_PER_WORKFLOW, MAX_QUERY_PATH_SEGMENTS, QueryOutputType,
    YbBoundedQuery,
};
use super::errors::QueryParseError;
use super::{
    from_bytes_compiled_queries, validate_compiled_queries, validate_compiled_query_count,
    validate_compiled_query_summary,
};

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
    let empty_queries: super::domain::YbBoundedQueries =
        super::domain::YbBoundedQueries::new(vec![].into(), 100);
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
    let bounded_queries = super::domain::YbBoundedQueries::new(vec![query].into(), 75);
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
