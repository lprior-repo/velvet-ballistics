#![forbid(unsafe_code)]

use proptest::prelude::*;
use vb_core::workflow::compiled_query::{QueryParseError, from_bytes_compiled_queries};

#[path = "vb_ajc40_property_common.rs"]
mod common;

use common::{encode_queries, query, query_payload};

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, .. ProptestConfig::default() })]

    #[test]
    fn po_018_query_budget_boundaries_and_exact_remaining(total in 0u64..1_000_000, extra in 0u64..1_000_000) {
        let payload = query_payload(vec![query(0, total)], total);
        let bytes = encode_queries(&payload)?;

        match from_bytes_compiled_queries(&bytes, total + extra) {
            Ok(admitted) => prop_assert_eq!(admitted.remaining_budget(), extra),
            Err(err) => return Err(TestCaseError::fail(format!("query budget admission failed: {err}"))),
        }

        if total > 0 {
            prop_assert_eq!(
                from_bytes_compiled_queries(&bytes, total - 1),
                Err(QueryParseError::YbBudgetExceeded { total, max: total - 1 })
            );
        }
    }
}
