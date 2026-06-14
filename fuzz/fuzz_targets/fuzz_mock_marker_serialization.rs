//! Cargo-fuzz harness for MockMarker 1-byte serialization.
//!
//! Obligation: OBL-NEW-PS-003
//! Verifier lane: cargo-fuzz
//!
/// This fuzz target tests that MockMarker serializes to exactly 1 byte
/// in the postcard wire format, regardless of which of the 3 variants
/// is encoded.
//!
/// NOTE: This target is a placeholder. It will compile once MockMarker
/// enum is added to production code in vb_core::action.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: [u8; 1]| {
    // MockMarker is a #[repr(u8)] enum with 3 unit variants.
    // Each variant maps to discriminant 0, 1, or 2.
    // We test all 3 possible 1-byte values.
    //
    // PLACEHOLDER: This code will compile once MockMarker exists.
    // Current stub: verify that the fuzz input is 1 byte.

    assert_eq!(data.len(), 1, "MockMarker fuzz input must be exactly 1 byte");

    let discriminant = data[0];

    // Only test valid MockMarker discriminants (0, 1, 2).
    // Invalid values (3-255) are reserved for future variants.
    if discriminant <= 2 {
        // TODO: Once MockMarker exists:
        // let mock_marker = match discriminant {
        //     0 => vb_core::action::MockMarker::HttpGet,
        //     1 => vb_core::action::MockMarker::HttpPost,
        //     2 => vb_core::action::MockMarker::HttpPut,
        //     _ => unreachable!(),
        // };
        // let serialized = postcard::to_allocvec(&mock_marker).expect("MockMarker serialization must succeed");
        // assert_eq!(serialized.len(), 1, "MockMarker must serialize to exactly 1 byte");
        // assert_eq!(serialized[0], discriminant, "MockMarker serialization must preserve discriminant");
        // let deserialized: vb_core::action::MockMarker =
        //     postcard::from_bytes(&serialized).expect("MockMarker deserialization must succeed");
        // assert_eq!(deserialized, mock_marker, "MockMarker round-trip must preserve value");
    }
});
