#![forbid(unsafe_code)]

use proptest::prelude::*;
use vb_core::workflow::compiled_slug::{SlugParseError, from_bytes_compiled_slugs};

#[path = "vb_ajc40_property_common.rs"]
mod common;

use common::{encode_slugs, slug, slug_payload};

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, .. ProptestConfig::default() })]

    #[test]
    fn po_014_slug_budget_boundaries_and_exact_remaining(total in 0u64..1_000_000, extra in 0u64..1_000_000) {
        let payload = slug_payload(vec![slug(0, total)], total);
        let bytes = encode_slugs(&payload)?;

        match from_bytes_compiled_slugs(&bytes, total + extra) {
            Ok(admitted) => prop_assert_eq!(admitted.remaining_budget(), extra),
            Err(err) => return Err(TestCaseError::fail(format!("slug budget admission failed: {err}"))),
        }

        if total > 0 {
            prop_assert_eq!(
                from_bytes_compiled_slugs(&bytes, total - 1),
                Err(SlugParseError::YbBudgetExceeded { total, max: total - 1 })
            );
        }
    }
}
