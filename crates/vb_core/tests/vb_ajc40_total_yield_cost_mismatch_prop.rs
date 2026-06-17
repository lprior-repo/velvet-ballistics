#![allow(clippy::expect_used, clippy::unwrap_used, clippy::as_conversions, clippy::panic)]
#![forbid(unsafe_code)]

use proptest::prelude::*;
use vb_core::workflow::compiled_query::QueryParseError;
use vb_core::workflow::compiled_slug::SlugParseError;

#[path = "vb_ajc40_property_common.rs"]
mod common;

use common::{assert_query_error, assert_slug_error, query, query_payload, slug, slug_payload};

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, .. ProptestConfig::default() })]

    #[test]
    fn po_022_slug_and_query_reject_underdeclared_totals(actual in 1u64..1_000_000) {
        let declared = actual - 1;
        let slug_payload = slug_payload(vec![slug(0, actual)], declared);
        assert_slug_error(
            &slug_payload,
            actual,
            SlugParseError::TotalYieldCostMismatch { declared, recomputed: actual },
        )?;

        let query_payload = query_payload(vec![query(0, actual)], declared);
        assert_query_error(
            &query_payload,
            actual,
            QueryParseError::TotalYieldCostMismatch { declared, recomputed: actual },
        )?;
    }

    #[test]
    fn po_022_slug_and_query_reject_overdeclared_totals(actual in 0u64..1_000_000) {
        let declared = actual + 1;
        let slug_payload = slug_payload(vec![slug(0, actual)], declared);
        assert_slug_error(
            &slug_payload,
            declared,
            SlugParseError::TotalYieldCostMismatch { declared, recomputed: actual },
        )?;

        let query_payload = query_payload(vec![query(0, actual)], declared);
        assert_query_error(
            &query_payload,
            declared,
            QueryParseError::TotalYieldCostMismatch { declared, recomputed: actual },
        )?;
    }
}

#[test]
fn po_022_slug_and_query_reject_checked_add_overflow() -> Result<(), TestCaseError> {
    let slug_payload = slug_payload(vec![slug(0, u64::MAX), slug(0, 1)], 0);
    assert_slug_error(&slug_payload, u64::MAX, SlugParseError::YieldCostOverflow)?;

    let query_payload = query_payload(vec![query(0, u64::MAX), query(0, 1)], 0);
    assert_query_error(&query_payload, u64::MAX, QueryParseError::YieldCostOverflow)?;
    Ok(())
}
