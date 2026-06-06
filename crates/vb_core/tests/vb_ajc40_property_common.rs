#![forbid(unsafe_code)]

use proptest::prelude::*;
use vb_core::ids::SymbolId;
use vb_core::workflow::PathSegment;
use vb_core::workflow::compiled_query::{
    CompiledQueries, QueryOutputType, QueryParseError, YbBoundedQuery, from_bytes_compiled_queries,
};
use vb_core::workflow::compiled_slug::{
    CompiledSlugs, SlugParseError, YbBoundedSlug, from_bytes_compiled_slugs,
};

pub(crate) fn slug(path_len: u32, yield_cost: u64) -> YbBoundedSlug {
    YbBoundedSlug {
        path: path(path_len),
        yield_cost,
    }
}

pub(crate) fn query(path_len: u32, yield_cost: u64) -> YbBoundedQuery {
    YbBoundedQuery {
        path: path(path_len),
        output_type: QueryOutputType::Boolean,
        yield_cost,
    }
}

fn path(path_len: u32) -> Box<[PathSegment]> {
    (0..path_len)
        .map(|index| PathSegment::Field(SymbolId::new(index)))
        .collect()
}

pub(crate) fn encode_slugs(payload: &CompiledSlugs) -> Result<Vec<u8>, TestCaseError> {
    postcard::to_allocvec(payload)
        .map_err(|err| TestCaseError::fail(format!("slug postcard encode failed: {err}")))
}

pub(crate) fn encode_queries(payload: &CompiledQueries) -> Result<Vec<u8>, TestCaseError> {
    postcard::to_allocvec(payload)
        .map_err(|err| TestCaseError::fail(format!("query postcard encode failed: {err}")))
}

pub(crate) fn slug_payload(items: Vec<YbBoundedSlug>, total_yield_cost: u64) -> CompiledSlugs {
    CompiledSlugs {
        slugs: items.into_boxed_slice(),
        total_yield_cost,
    }
}

pub(crate) fn query_payload(items: Vec<YbBoundedQuery>, total_yield_cost: u64) -> CompiledQueries {
    CompiledQueries {
        queries: items.into_boxed_slice(),
        total_yield_cost,
    }
}

pub(crate) fn assert_slug_roundtrip(
    payload: &CompiledSlugs,
    budget: u64,
) -> Result<(), TestCaseError> {
    let bytes = encode_slugs(payload)?;
    match from_bytes_compiled_slugs(&bytes, budget) {
        Ok(admitted) => {
            prop_assert_eq!(admitted.len(), payload.slugs.len());
            prop_assert!(admitted.remaining_budget() <= budget);
            Ok(())
        }
        Err(err) => Err(TestCaseError::fail(format!("slug admission failed: {err}"))),
    }
}

pub(crate) fn assert_query_roundtrip(
    payload: &CompiledQueries,
    budget: u64,
) -> Result<(), TestCaseError> {
    let bytes = encode_queries(payload)?;
    match from_bytes_compiled_queries(&bytes, budget) {
        Ok(admitted) => {
            prop_assert_eq!(admitted.len(), payload.queries.len());
            prop_assert!(admitted.remaining_budget() <= budget);
            Ok(())
        }
        Err(err) => Err(TestCaseError::fail(format!(
            "query admission failed: {err}"
        ))),
    }
}

pub(crate) fn assert_slug_error(
    payload: &CompiledSlugs,
    budget: u64,
    expected: SlugParseError,
) -> Result<(), TestCaseError> {
    let bytes = encode_slugs(payload)?;
    prop_assert_eq!(from_bytes_compiled_slugs(&bytes, budget), Err(expected));
    Ok(())
}

pub(crate) fn assert_query_error(
    payload: &CompiledQueries,
    budget: u64,
    expected: QueryParseError,
) -> Result<(), TestCaseError> {
    let bytes = encode_queries(payload)?;
    prop_assert_eq!(from_bytes_compiled_queries(&bytes, budget), Err(expected));
    Ok(())
}

type SlugFactory = fn(u32, u64) -> YbBoundedSlug;
type QueryFactory = fn(u32, u64) -> YbBoundedQuery;
type PathFactory = fn(u32) -> Box<[PathSegment]>;
type EncodeSlugs = fn(&CompiledSlugs) -> Result<Vec<u8>, TestCaseError>;
type EncodeQueries = fn(&CompiledQueries) -> Result<Vec<u8>, TestCaseError>;
type SlugPayload = fn(Vec<YbBoundedSlug>, u64) -> CompiledSlugs;
type QueryPayload = fn(Vec<YbBoundedQuery>, u64) -> CompiledQueries;
type AssertSlugRoundtrip = fn(&CompiledSlugs, u64) -> Result<(), TestCaseError>;
type AssertQueryRoundtrip = fn(&CompiledQueries, u64) -> Result<(), TestCaseError>;
type AssertSlugError = fn(&CompiledSlugs, u64, SlugParseError) -> Result<(), TestCaseError>;
type AssertQueryError = fn(&CompiledQueries, u64, QueryParseError) -> Result<(), TestCaseError>;

#[used]
static VB_AJC40_COMMON_SYMBOLS: (
    SlugFactory,
    QueryFactory,
    PathFactory,
    EncodeSlugs,
    EncodeQueries,
    SlugPayload,
    QueryPayload,
    AssertSlugRoundtrip,
    AssertQueryRoundtrip,
    AssertSlugError,
    AssertQueryError,
) = (
    slug,
    query,
    path,
    encode_slugs,
    encode_queries,
    slug_payload,
    query_payload,
    assert_slug_roundtrip,
    assert_query_roundtrip,
    assert_slug_error,
    assert_query_error,
);
