#![forbid(unsafe_code)]

use proptest::prelude::*;
use vb_core::workflow::compiled_slug::{
    MAX_SLUG_PATH_SEGMENTS, SlugParseError, from_bytes_compiled_slugs,
};

#[path = "vb_ajc40_property_common.rs"]
mod common;

use common::{assert_slug_roundtrip, encode_slugs, slug, slug_payload};

proptest! {
    #![proptest_config(ProptestConfig { cases: 128, .. ProptestConfig::default() })]

    #[test]
    fn po_026_slug_paths_at_or_below_limit_are_admitted(path_len in 0u32..=16, cost in 0u64..1_000_000) {
        let payload = slug_payload(vec![slug(path_len, cost)], cost);
        assert_slug_roundtrip(&payload, cost)?;
    }

    #[test]
    fn po_026_slug_path_over_limit_rejects_before_budget(cost in 0u64..1_000_000) {
        let payload = slug_payload(vec![slug(17, cost)], cost);
        let bytes = encode_slugs(&payload)?;
        prop_assert_eq!(
            from_bytes_compiled_slugs(&bytes, 0),
            Err(SlugParseError::SlugPathTooDeep { depth: MAX_SLUG_PATH_SEGMENTS + 1, max: MAX_SLUG_PATH_SEGMENTS })
        );
    }
}
