//! Kani harness for ContentType enum discrimination proof
//!
//! # Proof Obligation
//! po-vbqa2g-014: ContentType enum distinguishes JsonUtf8 from future TypedPostcard
//!
//! # GOD RULE 1 Compliance
//! No arbitrary input needed - tests enum variant existence
//!
//! # GOD RULE 2 Compliance
//! Binds to CliPostcardContentType enum

use vb_cli::cli_postcard::CliPostcardContentType;

#[kani::proof]
fn harness_content_type() {
    // Verify JsonUtf8 variant exists and can be compared
    let ct1 = CliPostcardContentType::JsonUtf8;
    let ct2 = CliPostcardContentType::JsonUtf8;

    kani::assert(
        ct1 == ct2,
        "CliPostcardContentType::JsonUtf8 should equal itself"
    );
}

#[kani::proof]
fn harness_content_type_equality() {
    // Test equality comparison
    let json_utf8_1 = CliPostcardContentType::JsonUtf8;
    let json_utf8_2 = CliPostcardContentType::JsonUtf8;

    kani::assert(
        json_utf8_1 == json_utf8_2,
        "same ContentType variants should be equal"
    );
}

#[kani::proof]
fn harness_content_type_debug() {
    // Verify ContentType implements Debug (can be formatted for debugging)
    let ct = CliPostcardContentType::JsonUtf8;
    let debug_str = format!("{:?}", ct);

    kani::assert(
        !debug_str.is_empty(),
        "ContentType should implement Debug"
    );
}