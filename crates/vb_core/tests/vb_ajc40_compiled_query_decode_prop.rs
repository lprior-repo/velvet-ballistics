#![forbid(unsafe_code)]

use proptest::prelude::*;
use vb_core::workflow::compiled_query::{
    MAX_QUERIES_PER_WORKFLOW, MAX_QUERY_PATH_SEGMENTS, from_bytes_compiled_queries,
};

#[path = "vb_ajc40_property_common.rs"]
mod common;

use common::{assert_query_roundtrip, query, query_payload};

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, .. ProptestConfig::default() })]

    #[test]
    fn po_009_query_roundtrip_or_decode_error(byte_len in 0usize..64, fill in any::<u8>()) {
        let bytes = vec![fill; byte_len];
        if let Ok(admitted) = from_bytes_compiled_queries(&bytes, u64::MAX) {
            prop_assert!(admitted.len() <= MAX_QUERIES_PER_WORKFLOW);
            prop_assert!(admitted.queries().iter().all(|query| query.path_depth() <= MAX_QUERY_PATH_SEGMENTS));
        }

        let payload = query_payload(vec![query(0, 0), query(1, 1)], 1);
        assert_query_roundtrip(&payload, 1)?;
    }
}
