#![forbid(unsafe_code)]

use proptest::prelude::*;
use vb_core::workflow::compiled_slug::{
    MAX_SLUG_PATH_SEGMENTS, MAX_SLUGS_PER_WORKFLOW, from_bytes_compiled_slugs,
};

#[path = "vb_ajc40_property_common.rs"]
mod common;

use common::{assert_slug_roundtrip, slug, slug_payload};

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, .. ProptestConfig::default() })]

    #[test]
    fn po_004_slug_roundtrip_or_decode_error(byte_len in 0usize..64, fill in any::<u8>()) {
        let bytes = vec![fill; byte_len];
        if let Ok(admitted) = from_bytes_compiled_slugs(&bytes, u64::MAX) {
            prop_assert!(admitted.len() <= MAX_SLUGS_PER_WORKFLOW);
            prop_assert!(admitted.slugs().iter().all(|slug| slug.path_depth() <= MAX_SLUG_PATH_SEGMENTS));
        }

        let payload = slug_payload(vec![slug(0, 0), slug(1, 1)], 1);
        assert_slug_roundtrip(&payload, 1)?;
    }
}
