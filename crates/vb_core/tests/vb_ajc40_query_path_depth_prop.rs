#![forbid(unsafe_code)]

use proptest::prelude::*;
use vb_core::workflow::compiled_query::{
    MAX_QUERY_PATH_SEGMENTS, QueryParseError, from_bytes_compiled_queries,
};

#[path = "vb_ajc40_property_common.rs"]
mod common;

use common::{assert_query_roundtrip, encode_queries, query, query_payload};

proptest! {
    #![proptest_config(ProptestConfig { cases: 128, .. ProptestConfig::default() })]

    #[test]
    fn po_030_query_paths_at_or_below_limit_are_admitted(path_len in 0u32..=16, cost in 0u64..1_000_000) {
        let payload = query_payload(vec![query(path_len, cost)], cost);
        assert_query_roundtrip(&payload, cost)?;
    }

    #[test]
    fn po_030_query_path_over_limit_rejects_before_budget(cost in 0u64..1_000_000) {
        let payload = query_payload(vec![query(17, cost)], cost);
        let bytes = encode_queries(&payload)?;
        prop_assert_eq!(
            from_bytes_compiled_queries(&bytes, 0),
            Err(QueryParseError::QueryPathTooDeep { depth: MAX_QUERY_PATH_SEGMENTS + 1, max: MAX_QUERY_PATH_SEGMENTS })
        );
    }
}
