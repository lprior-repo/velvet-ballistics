//! CLI Postcard Wire-Format Contract Test
//!
//! vb-k8ut.5: this is the typed wire-format contract. A `bool: true`
//! postcard-encodes as a single byte 0x01, NEVER as the four-byte ASCII
//! sequence b"true" (0x74 0x72 0x75 0x65). The replacement for the
//! prior placebo wire-format test asserts that the typed bool survives
//! the postcard encoder without becoming a self-describing JSON-style
//! string. Conversely, the typed `String` field carrying the kind tag
//! ("validate_report") is postcard-encoded as a varint-prefixed UTF-8
//! string, so the byte sequence b"validate_report" MUST appear in the
//! wire bytes — that proves the typed `String` discriminant is still
//! present alongside the typed bool, matching the JSON envelope
//! contract for kind. The same invariant holds for `false` (the
//! b"false" sequence MUST NOT appear because the bool field encodes
//! as a single 0x00 byte, not the ASCII string "false").

use super::super::*;

#[test]
fn typed_postcard_wire_format_carries_typed_bool_not_string() {
    let report = ValidateReport {
        schema_version: EnvelopeSchemaVersion::current(),
        kind: "validate_report".to_string(),
        success: true,
        status: "valid".to_string(),
        exit_code: 0,
        repair_hints: Vec::new(),
    };
    let payload = CliPostcardPayload::Validate(report);
    let bytes = postcard::to_allocvec(&payload).expect("typed validate must encode");

    let contains_true_substring = bytes.windows(b"true".len()).any(|window| window == b"true");
    assert!(
        !contains_true_substring,
        "postcard-encoded bool=true must NOT carry the ASCII substring b\"true\"; \
         bool is encoded as a single byte 0x01. wire bytes: {bytes:?}"
    );

    let contains_false_substring = bytes
        .windows(b"false".len())
        .any(|window| window == b"false");
    assert!(
        !contains_false_substring,
        "postcard-encoded bool=false must NOT carry the ASCII substring b\"false\"; \
         bool is encoded as a single byte 0x00. wire bytes: {bytes:?}"
    );

    let contains_kind_substring = bytes
        .windows(b"validate_report".len())
        .any(|window| window == b"validate_report");
    assert!(
        contains_kind_substring,
        "postcard-encoded String kind field must carry the ASCII substring b\"validate_report\"; \
         the typed struct preserves the kind tag as a String. wire bytes: {bytes:?}"
    );
}
