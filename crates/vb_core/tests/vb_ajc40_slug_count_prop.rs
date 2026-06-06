#![forbid(unsafe_code)]

use proptest::prelude::*;
use vb_core::workflow::compiled_slug::{
    MAX_SLUGS_PER_WORKFLOW, SlugParseError, from_bytes_compiled_slugs,
};

#[path = "vb_ajc40_property_common.rs"]
mod common;

use common::{encode_slugs, slug, slug_payload};

proptest! {
    #![proptest_config(ProptestConfig { cases: 128, .. ProptestConfig::default() })]

    #[test]
    fn po_034_random_small_slug_counts_are_admitted(count in 0usize..128) {
        let items = vec![slug(0, 0); count];
        let payload = slug_payload(items, 0);
        let bytes = encode_slugs(&payload)?;
        match from_bytes_compiled_slugs(&bytes, 0) {
            Ok(admitted) => prop_assert_eq!(admitted.len(), count),
            Err(err) => return Err(TestCaseError::fail(format!("slug count admission failed: {err}"))),
        }
    }
}

#[test]
fn po_034_slug_exact_count_boundaries_65535_65536() -> Result<(), TestCaseError> {
    let at_limit = slug_payload(vec![slug(0, 0); MAX_SLUGS_PER_WORKFLOW], 0);
    let at_limit_bytes = encode_slugs(&at_limit)?;
    match from_bytes_compiled_slugs(&at_limit_bytes, 0) {
        Ok(admitted) => prop_assert_eq!(admitted.len(), MAX_SLUGS_PER_WORKFLOW),
        Err(err) => {
            return Err(TestCaseError::fail(format!(
                "slug at-limit admission failed: {err}"
            )));
        }
    }

    let over_limit_count = MAX_SLUGS_PER_WORKFLOW + 1;
    let over_limit = slug_payload(vec![slug(0, 0); over_limit_count], 0);
    let over_limit_bytes = encode_slugs(&over_limit)?;
    prop_assert_eq!(
        from_bytes_compiled_slugs(&over_limit_bytes, 0),
        Err(SlugParseError::TooManySlugs {
            count: over_limit_count,
            max: MAX_SLUGS_PER_WORKFLOW
        })
    );
    Ok(())
}
