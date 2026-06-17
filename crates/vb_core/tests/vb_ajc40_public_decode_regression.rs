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
#![forbid(unsafe_code)]

use proptest::prelude::*;
use vb_core::ids::SymbolId;
use vb_core::workflow::PathSegment;
use vb_core::workflow::compiled_query::{
    CompiledQueries, MAX_QUERIES_PER_WORKFLOW, MAX_QUERY_PATH_SEGMENTS, QueryOutputType,
    QueryParseError, YbBoundedQuery, from_bytes_compiled_queries,
};
use vb_core::workflow::compiled_slug::{
    CompiledSlugs, MAX_SLUG_PATH_SEGMENTS, MAX_SLUGS_PER_WORKFLOW, SlugParseError, YbBoundedSlug,
    from_bytes_compiled_slugs,
};

fn path(path_len: usize) -> Box<[PathSegment]> {
    (0..path_len)
        .map(|index| PathSegment::Field(SymbolId::new(index as u32)))
        .collect()
}

fn slug(path_len: usize, yield_cost: u64) -> YbBoundedSlug {
    YbBoundedSlug {
        path: path(path_len),
        yield_cost,
    }
}

fn query(path_len: usize, output_type: QueryOutputType, yield_cost: u64) -> YbBoundedQuery {
    YbBoundedQuery {
        path: path(path_len),
        output_type,
        yield_cost,
    }
}

fn slug_payload(slugs: Vec<YbBoundedSlug>, total_yield_cost: u64) -> CompiledSlugs {
    CompiledSlugs {
        slugs: slugs.into_boxed_slice(),
        total_yield_cost,
    }
}

fn query_payload(queries: Vec<YbBoundedQuery>, total_yield_cost: u64) -> CompiledQueries {
    CompiledQueries {
        queries: queries.into_boxed_slice(),
        total_yield_cost,
    }
}

fn encode_slugs(payload: &CompiledSlugs) -> Result<Vec<u8>, TestCaseError> {
    postcard::to_allocvec(payload)
        .map_err(|err| TestCaseError::fail(format!("slug encode failed: {err}")))
}

fn encode_queries(payload: &CompiledQueries) -> Result<Vec<u8>, TestCaseError> {
    postcard::to_allocvec(payload)
        .map_err(|err| TestCaseError::fail(format!("query encode failed: {err}")))
}

fn first_slug_path_depth(
    bytes: &[u8],
    budget: u64,
) -> Result<(usize, bool, u64, u64), TestCaseError> {
    match from_bytes_compiled_slugs(bytes, budget) {
        Ok(admitted) => match admitted.slugs().first() {
            Some(item) => Ok((
                item.path_depth(),
                item.is_path_too_deep(),
                item.yield_cost,
                admitted.remaining_budget(),
            )),
            None => Err(TestCaseError::fail("admitted slug collection was empty")),
        },
        Err(err) => Err(TestCaseError::fail(format!("slug admission failed: {err}"))),
    }
}

fn first_query_path_depth(
    bytes: &[u8],
    budget: u64,
) -> Result<(usize, bool, QueryOutputType, u64, u64), TestCaseError> {
    match from_bytes_compiled_queries(bytes, budget) {
        Ok(admitted) => match admitted.queries().first() {
            Some(item) => Ok((
                item.path_depth(),
                item.is_path_too_deep(),
                item.output_type,
                item.yield_cost,
                admitted.remaining_budget(),
            )),
            None => Err(TestCaseError::fail("admitted query collection was empty")),
        },
        Err(err) => Err(TestCaseError::fail(format!(
            "query admission failed: {err}"
        ))),
    }
}

#[test]
fn compiled_slugs_return_decode_error_when_bytes_are_malformed() -> Result<(), TestCaseError> {
    let cases: [&[u8]; 3] = [&[], &[0xA5], &[0xFF, 0xFF, 0xFF, 0xFF]];

    for bytes in cases {
        match from_bytes_compiled_slugs(bytes, u64::MAX) {
            Err(SlugParseError::Decode(_)) => {}
            actual => {
                return Err(TestCaseError::fail(format!(
                    "expected slug decode error for malformed bytes, got {actual:?}"
                )));
            }
        }
    }

    Ok(())
}

#[test]
fn compiled_queries_return_decode_error_when_bytes_are_malformed() -> Result<(), TestCaseError> {
    let slug_bytes = encode_slugs(&slug_payload(vec![slug(1, 3)], 3))?;
    let cases: [&[u8]; 3] = [&[], &[0xA5], &slug_bytes];

    for bytes in cases {
        match from_bytes_compiled_queries(bytes, u64::MAX) {
            Err(QueryParseError::Decode(_)) => {}
            actual => {
                return Err(TestCaseError::fail(format!(
                    "expected query decode error for malformed bytes, got {actual:?}"
                )));
            }
        }
    }

    Ok(())
}

#[test]
fn compiled_slugs_admit_empty_collection_with_full_remaining_budget() -> Result<(), TestCaseError> {
    let bytes = encode_slugs(&slug_payload(Vec::new(), 0))?;
    for budget in [0, 1, u64::MAX] {
        match from_bytes_compiled_slugs(&bytes, budget) {
            Ok(admitted) => {
                prop_assert_eq!(admitted.len(), 0);
                prop_assert_eq!(admitted.is_empty(), true);
                prop_assert_eq!(admitted.slugs().len(), 0);
                prop_assert_eq!(admitted.remaining_budget(), budget);
            }
            Err(err) => return Err(TestCaseError::fail(format!("empty slugs rejected: {err}"))),
        }
    }
    Ok(())
}

#[test]
fn compiled_queries_admit_empty_collection_with_full_remaining_budget() -> Result<(), TestCaseError>
{
    let bytes = encode_queries(&query_payload(Vec::new(), 0))?;
    for budget in [0, 1, u64::MAX] {
        match from_bytes_compiled_queries(&bytes, budget) {
            Ok(admitted) => {
                prop_assert_eq!(admitted.len(), 0);
                prop_assert_eq!(admitted.is_empty(), true);
                prop_assert_eq!(admitted.queries().len(), 0);
                prop_assert_eq!(admitted.remaining_budget(), budget);
            }
            Err(err) => {
                return Err(TestCaseError::fail(format!(
                    "empty queries rejected: {err}"
                )));
            }
        }
    }
    Ok(())
}

#[test]
fn compiled_slugs_admit_empty_path_root_accessor() -> Result<(), TestCaseError> {
    let bytes = encode_slugs(&slug_payload(vec![slug(0, 4)], 4))?;
    prop_assert_eq!(first_slug_path_depth(&bytes, 9)?, (0, false, 4, 5));
    Ok(())
}

#[test]
fn compiled_queries_admit_empty_path_root_accessor() -> Result<(), TestCaseError> {
    let bytes = encode_queries(&query_payload(
        vec![query(0, QueryOutputType::Integer, 4)],
        4,
    ))?;
    prop_assert_eq!(
        first_query_path_depth(&bytes, 9)?,
        (0, false, QueryOutputType::Integer, 4, 5)
    );
    Ok(())
}

#[test]
fn compiled_slugs_admit_path_depth_16_and_reject_17() -> Result<(), TestCaseError> {
    let at_limit = encode_slugs(&slug_payload(vec![slug(MAX_SLUG_PATH_SEGMENTS, 6)], 6))?;
    prop_assert_eq!(first_slug_path_depth(&at_limit, 6)?, (16, false, 6, 0));

    let over_limit = encode_slugs(&slug_payload(vec![slug(MAX_SLUG_PATH_SEGMENTS + 1, 6)], 6))?;
    prop_assert_eq!(
        from_bytes_compiled_slugs(&over_limit, 0),
        Err(SlugParseError::SlugPathTooDeep { depth: 17, max: 16 })
    );
    Ok(())
}

#[test]
fn compiled_queries_admit_path_depth_16_and_reject_17() -> Result<(), TestCaseError> {
    let at_limit = encode_queries(&query_payload(
        vec![query(MAX_QUERY_PATH_SEGMENTS, QueryOutputType::Boolean, 6)],
        6,
    ))?;
    prop_assert_eq!(
        first_query_path_depth(&at_limit, 6)?,
        (16, false, QueryOutputType::Boolean, 6, 0)
    );

    let over_limit = encode_queries(&query_payload(
        vec![query(
            MAX_QUERY_PATH_SEGMENTS + 1,
            QueryOutputType::Boolean,
            6,
        )],
        6,
    ))?;
    prop_assert_eq!(
        from_bytes_compiled_queries(&over_limit, 0),
        Err(QueryParseError::QueryPathTooDeep { depth: 17, max: 16 })
    );
    Ok(())
}

#[test]
fn compiled_slugs_return_exact_remaining_budget_when_recomputed_total_fits()
-> Result<(), TestCaseError> {
    for (costs, declared, budget, expected_remaining) in [
        (vec![7, 11], 18, 25, 7),
        (vec![0], 0, 0, 0),
        (vec![18], 18, 18, 0),
        (vec![u64::MAX], u64::MAX, u64::MAX, 0),
    ] {
        let items = costs.into_iter().map(|cost| slug(0, cost)).collect();
        let bytes = encode_slugs(&slug_payload(items, declared))?;
        match from_bytes_compiled_slugs(&bytes, budget) {
            Ok(admitted) => prop_assert_eq!(admitted.remaining_budget(), expected_remaining),
            Err(err) => {
                return Err(TestCaseError::fail(format!(
                    "slug budget fit rejected: {err}"
                )));
            }
        }
    }
    Ok(())
}

#[test]
fn compiled_queries_return_exact_remaining_budget_when_recomputed_total_fits()
-> Result<(), TestCaseError> {
    for (costs, declared, budget, expected_remaining) in [
        (vec![7, 11], 18, 25, 7),
        (vec![0], 0, 0, 0),
        (vec![18], 18, 18, 0),
        (vec![u64::MAX], u64::MAX, u64::MAX, 0),
    ] {
        let items = costs
            .into_iter()
            .map(|cost| query(0, QueryOutputType::Boolean, cost))
            .collect();
        let bytes = encode_queries(&query_payload(items, declared))?;
        match from_bytes_compiled_queries(&bytes, budget) {
            Ok(admitted) => prop_assert_eq!(admitted.remaining_budget(), expected_remaining),
            Err(err) => {
                return Err(TestCaseError::fail(format!(
                    "query budget fit rejected: {err}"
                )));
            }
        }
    }
    Ok(())
}

#[test]
fn compiled_slugs_reject_when_recomputed_total_exceeds_budget() -> Result<(), TestCaseError> {
    let bytes = encode_slugs(&slug_payload(vec![slug(0, 7), slug(0, 11)], 18))?;
    prop_assert_eq!(
        from_bytes_compiled_slugs(&bytes, 17),
        Err(SlugParseError::YbBudgetExceeded { total: 18, max: 17 })
    );
    Ok(())
}

#[test]
fn compiled_queries_reject_when_recomputed_total_exceeds_budget() -> Result<(), TestCaseError> {
    let bytes = encode_queries(&query_payload(
        vec![
            query(0, QueryOutputType::Boolean, 7),
            query(0, QueryOutputType::Boolean, 11),
        ],
        18,
    ))?;
    prop_assert_eq!(
        from_bytes_compiled_queries(&bytes, 17),
        Err(QueryParseError::YbBudgetExceeded { total: 18, max: 17 })
    );
    Ok(())
}

#[test]
fn compiled_slugs_and_queries_reject_underdeclared_total_yield_cost() -> Result<(), TestCaseError> {
    let slug_bytes = encode_slugs(&slug_payload(vec![slug(0, 7), slug(0, 11)], 17))?;
    prop_assert_eq!(
        from_bytes_compiled_slugs(&slug_bytes, 18),
        Err(SlugParseError::TotalYieldCostMismatch {
            declared: 17,
            recomputed: 18
        })
    );

    let query_bytes = encode_queries(&query_payload(
        vec![
            query(0, QueryOutputType::Boolean, 7),
            query(0, QueryOutputType::Boolean, 11),
        ],
        17,
    ))?;
    prop_assert_eq!(
        from_bytes_compiled_queries(&query_bytes, 18),
        Err(QueryParseError::TotalYieldCostMismatch {
            declared: 17,
            recomputed: 18
        })
    );
    Ok(())
}

#[test]
fn compiled_slugs_and_queries_reject_overdeclared_total_yield_cost() -> Result<(), TestCaseError> {
    let slug_bytes = encode_slugs(&slug_payload(vec![slug(0, 7), slug(0, 11)], 19))?;
    prop_assert_eq!(
        from_bytes_compiled_slugs(&slug_bytes, 19),
        Err(SlugParseError::TotalYieldCostMismatch {
            declared: 19,
            recomputed: 18
        })
    );

    let query_bytes = encode_queries(&query_payload(
        vec![
            query(0, QueryOutputType::Boolean, 7),
            query(0, QueryOutputType::Boolean, 11),
        ],
        19,
    ))?;
    prop_assert_eq!(
        from_bytes_compiled_queries(&query_bytes, 19),
        Err(QueryParseError::TotalYieldCostMismatch {
            declared: 19,
            recomputed: 18
        })
    );
    Ok(())
}

#[test]
fn compiled_slugs_and_queries_reject_stale_total_after_item_cost_mutation()
-> Result<(), TestCaseError> {
    let slug_bytes = encode_slugs(&slug_payload(vec![slug(0, 7), slug(0, 12)], 18))?;
    prop_assert_eq!(
        from_bytes_compiled_slugs(&slug_bytes, 19),
        Err(SlugParseError::TotalYieldCostMismatch {
            declared: 18,
            recomputed: 19
        })
    );

    let query_bytes = encode_queries(&query_payload(
        vec![
            query(0, QueryOutputType::Boolean, 7),
            query(0, QueryOutputType::Boolean, 12),
        ],
        18,
    ))?;
    prop_assert_eq!(
        from_bytes_compiled_queries(&query_bytes, 19),
        Err(QueryParseError::TotalYieldCostMismatch {
            declared: 18,
            recomputed: 19
        })
    );
    Ok(())
}

#[test]
fn compiled_slugs_and_queries_reject_checked_add_overflow() -> Result<(), TestCaseError> {
    for costs in [vec![u64::MAX, 1], vec![1, u64::MAX]] {
        let slug_items = costs.iter().copied().map(|cost| slug(0, cost)).collect();
        let slug_bytes = encode_slugs(&slug_payload(slug_items, 0))?;
        prop_assert_eq!(
            from_bytes_compiled_slugs(&slug_bytes, u64::MAX),
            Err(SlugParseError::YieldCostOverflow)
        );

        let query_items = costs
            .iter()
            .copied()
            .map(|cost| query(0, QueryOutputType::Boolean, cost))
            .collect();
        let query_bytes = encode_queries(&query_payload(query_items, 0))?;
        prop_assert_eq!(
            from_bytes_compiled_queries(&query_bytes, u64::MAX),
            Err(QueryParseError::YieldCostOverflow)
        );
    }
    Ok(())
}

#[test]
fn compiled_slugs_and_queries_report_total_mismatch_before_budget_exceeded()
-> Result<(), TestCaseError> {
    let slug_bytes = encode_slugs(&slug_payload(vec![slug(0, 10), slug(0, 8)], 17))?;
    prop_assert_eq!(
        from_bytes_compiled_slugs(&slug_bytes, 0),
        Err(SlugParseError::TotalYieldCostMismatch {
            declared: 17,
            recomputed: 18
        })
    );

    let query_bytes = encode_queries(&query_payload(
        vec![
            query(0, QueryOutputType::Boolean, 10),
            query(0, QueryOutputType::Boolean, 8),
        ],
        17,
    ))?;
    prop_assert_eq!(
        from_bytes_compiled_queries(&query_bytes, 0),
        Err(QueryParseError::TotalYieldCostMismatch {
            declared: 17,
            recomputed: 18
        })
    );
    Ok(())
}

#[test]
fn compiled_queries_preserve_output_type_for_all_variants() -> Result<(), TestCaseError> {
    let variants = [
        QueryOutputType::Boolean,
        QueryOutputType::Integer,
        QueryOutputType::Float,
        QueryOutputType::String,
        QueryOutputType::List,
        QueryOutputType::Object,
    ];
    let payload = query_payload(
        variants
            .iter()
            .copied()
            .map(|output_type| query(0, output_type, 1))
            .collect(),
        6,
    );
    let bytes = encode_queries(&payload)?;
    match from_bytes_compiled_queries(&bytes, 10) {
        Ok(admitted) => {
            prop_assert_eq!(admitted.len(), variants.len());
            prop_assert_eq!(admitted.remaining_budget(), 4);
            for (index, expected) in variants.iter().copied().enumerate() {
                match admitted.queries().get(index) {
                    Some(actual) => prop_assert_eq!(actual.output_type, expected),
                    None => {
                        return Err(TestCaseError::fail(format!(
                            "missing query at index {index}"
                        )));
                    }
                }
            }
        }
        Err(err) => {
            return Err(TestCaseError::fail(format!(
                "output variant payload rejected: {err}"
            )));
        }
    }
    Ok(())
}

#[test]
fn compiled_slugs_accessors_reflect_successful_admission() -> Result<(), TestCaseError> {
    let bytes = encode_slugs(&slug_payload(vec![slug(0, 3), slug(2, 5)], 8))?;
    match from_bytes_compiled_slugs(&bytes, 13) {
        Ok(admitted) => {
            prop_assert_eq!(admitted.len(), 2);
            prop_assert_eq!(admitted.is_empty(), false);
            prop_assert_eq!(admitted.slugs().len(), 2);
            prop_assert_eq!(admitted.remaining_budget(), 5);
            match (admitted.slugs().first(), admitted.slugs().get(1)) {
                (Some(first), Some(second)) => {
                    prop_assert_eq!(first.path_depth(), 0);
                    prop_assert_eq!(first.is_path_too_deep(), false);
                    prop_assert_eq!(first.yield_cost, 3);
                    prop_assert_eq!(second.path_depth(), 2);
                    prop_assert_eq!(second.is_path_too_deep(), false);
                    prop_assert_eq!(second.yield_cost, 5);
                }
                _ => {
                    return Err(TestCaseError::fail(
                        "missing admitted slug accessor entries",
                    ));
                }
            }
        }
        Err(err) => {
            return Err(TestCaseError::fail(format!(
                "slug accessor payload rejected: {err}"
            )));
        }
    }
    Ok(())
}

#[test]
fn compiled_queries_accessors_reflect_successful_admission() -> Result<(), TestCaseError> {
    let bytes = encode_queries(&query_payload(
        vec![
            query(0, QueryOutputType::List, 3),
            query(2, QueryOutputType::Object, 5),
        ],
        8,
    ))?;
    match from_bytes_compiled_queries(&bytes, 13) {
        Ok(admitted) => {
            prop_assert_eq!(admitted.len(), 2);
            prop_assert_eq!(admitted.is_empty(), false);
            prop_assert_eq!(admitted.queries().len(), 2);
            prop_assert_eq!(admitted.remaining_budget(), 5);
            match (admitted.queries().first(), admitted.queries().get(1)) {
                (Some(first), Some(second)) => {
                    prop_assert_eq!(first.path_depth(), 0);
                    prop_assert_eq!(first.is_path_too_deep(), false);
                    prop_assert_eq!(first.output_type, QueryOutputType::List);
                    prop_assert_eq!(first.yield_cost, 3);
                    prop_assert_eq!(second.path_depth(), 2);
                    prop_assert_eq!(second.is_path_too_deep(), false);
                    prop_assert_eq!(second.output_type, QueryOutputType::Object);
                    prop_assert_eq!(second.yield_cost, 5);
                }
                _ => {
                    return Err(TestCaseError::fail(
                        "missing admitted query accessor entries",
                    ));
                }
            }
        }
        Err(err) => {
            return Err(TestCaseError::fail(format!(
                "query accessor payload rejected: {err}"
            )));
        }
    }
    Ok(())
}

#[test]
fn compiled_slugs_report_too_many_before_path_errors() -> Result<(), TestCaseError> {
    let mut items = vec![slug(0, 0); MAX_SLUGS_PER_WORKFLOW + 1];
    match items.last_mut() {
        Some(last) => last.path = path(MAX_SLUG_PATH_SEGMENTS + 1),
        None => return Err(TestCaseError::fail("over-limit slug fixture was empty")),
    }
    let bytes = encode_slugs(&slug_payload(items, 0))?;
    prop_assert_eq!(
        from_bytes_compiled_slugs(&bytes, 0),
        Err(SlugParseError::TooManySlugs {
            count: MAX_SLUGS_PER_WORKFLOW + 1,
            max: MAX_SLUGS_PER_WORKFLOW,
        })
    );
    Ok(())
}

#[test]
fn compiled_queries_report_too_many_before_path_errors() -> Result<(), TestCaseError> {
    let mut items = vec![query(0, QueryOutputType::Boolean, 0); MAX_QUERIES_PER_WORKFLOW + 1];
    match items.last_mut() {
        Some(last) => last.path = path(MAX_QUERY_PATH_SEGMENTS + 1),
        None => return Err(TestCaseError::fail("over-limit query fixture was empty")),
    }
    let bytes = encode_queries(&query_payload(items, 0))?;
    prop_assert_eq!(
        from_bytes_compiled_queries(&bytes, 0),
        Err(QueryParseError::TooManyQueries {
            count: MAX_QUERIES_PER_WORKFLOW + 1,
            max: MAX_QUERIES_PER_WORKFLOW,
        })
    );
    Ok(())
}
