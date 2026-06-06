#![forbid(unsafe_code)]

use proptest::prelude::*;

#[path = "vb_ajc40_property_common.rs"]
mod common;

use common::{
    assert_query_roundtrip, assert_slug_roundtrip, query, query_payload, slug, slug_payload,
};

proptest! {
    #![proptest_config(ProptestConfig { cases: 128, .. ProptestConfig::default() })]

    #[test]
    fn po_042_empty_slug_and_query_paths_are_root_accessors(cost in 0u64..1_000_000) {
        let slug_payload = slug_payload(vec![slug(0, cost)], cost);
        assert_slug_roundtrip(&slug_payload, cost)?;

        let query_payload = query_payload(vec![query(0, cost)], cost);
        assert_query_roundtrip(&query_payload, cost)?;
    }
}
