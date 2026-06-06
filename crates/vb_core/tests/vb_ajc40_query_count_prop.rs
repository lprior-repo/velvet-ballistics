#![forbid(unsafe_code)]

use proptest::prelude::*;
use vb_core::workflow::compiled_query::{
    MAX_QUERIES_PER_WORKFLOW, QueryParseError, from_bytes_compiled_queries,
};

#[path = "vb_ajc40_property_common.rs"]
mod common;

use common::{encode_queries, query, query_payload};

proptest! {
    #![proptest_config(ProptestConfig { cases: 128, .. ProptestConfig::default() })]

    #[test]
    fn po_038_random_small_query_counts_are_admitted(count in 0usize..128) {
        let items = vec![query(0, 0); count];
        let payload = query_payload(items, 0);
        let bytes = encode_queries(&payload)?;
        match from_bytes_compiled_queries(&bytes, 0) {
            Ok(admitted) => prop_assert_eq!(admitted.len(), count),
            Err(err) => return Err(TestCaseError::fail(format!("query count admission failed: {err}"))),
        }
    }
}

#[test]
fn po_038_query_exact_count_boundaries_65535_65536() -> Result<(), TestCaseError> {
    let at_limit = query_payload(vec![query(0, 0); MAX_QUERIES_PER_WORKFLOW], 0);
    let at_limit_bytes = encode_queries(&at_limit)?;
    match from_bytes_compiled_queries(&at_limit_bytes, 0) {
        Ok(admitted) => prop_assert_eq!(admitted.len(), MAX_QUERIES_PER_WORKFLOW),
        Err(err) => {
            return Err(TestCaseError::fail(format!(
                "query at-limit admission failed: {err}"
            )));
        }
    }

    let over_limit_count = MAX_QUERIES_PER_WORKFLOW + 1;
    let over_limit = query_payload(vec![query(0, 0); over_limit_count], 0);
    let over_limit_bytes = encode_queries(&over_limit)?;
    prop_assert_eq!(
        from_bytes_compiled_queries(&over_limit_bytes, 0),
        Err(QueryParseError::TooManyQueries {
            count: over_limit_count,
            max: MAX_QUERIES_PER_WORKFLOW
        })
    );
    Ok(())
}
