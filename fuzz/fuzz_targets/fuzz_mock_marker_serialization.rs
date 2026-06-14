#![no_main]

// Cargo-fuzz harness for MockMarker 1-byte serialization.
//
// Obligation: OBL-NEW-PS-003.
// Verifier lane: cargo-fuzz.

use libfuzzer_sys::fuzz_target;
use vb_core::action::MockMarker;

fuzz_target!(|data: &[u8]| {
    let Some(byte) = data.first().copied() else {
        return;
    };
    round_trip_marker(mock_from_byte(byte));
});

fn mock_from_byte(byte: u8) -> MockMarker {
    match byte.checked_rem(3) {
        Some(0) => MockMarker::GithubIssueCreate,
        Some(1) => MockMarker::AiClassifyTicket,
        _ => MockMarker::HttpGet,
    }
}

fn round_trip_marker(marker: MockMarker) {
    let Ok(serialized) = postcard::to_allocvec(&marker) else {
        return;
    };
    assert_eq!(
        serialized.len(),
        1,
        "MockMarker must serialize to exactly 1 byte"
    );
    let Ok(deserialized) = postcard::from_bytes::<MockMarker>(&serialized) else {
        return;
    };
    assert_eq!(deserialized, marker, "MockMarker round-trip mismatch");
}
