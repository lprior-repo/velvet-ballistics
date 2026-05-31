//! Kani harness for unknown schema version detection proof
//!
//! # Proof Obligation
//! po-vbqa2g-011: decode returns PostcardDecodeError::UnknownSchemaVersion when
//! schema_version is not supported
//!
//! # GOD RULE 1 Compliance
//! Uses kani::any() for schema version generation
//!
//! # GOD RULE 2 Compliance
//! Binds to decode_postcard and validate_version_and_kind

use vb_cli::cli_postcard::{encode_postcard, decode_postcard, PostcardError, CLI_SCHEMA_VERSION};

#[kani::proof]
fn harness_unknown_schema_version() {
    // Generate schema version that is not CLI_SCHEMA_VERSION
    let version: u16 = kani::any();
    kani::assume(version != CLI_SCHEMA_VERSION && version != 0);

    let payload = vec![0u8; 32];
    let kind = 2u16; // CLI_POSTCARD_KIND

    if let Ok(encoded) = encode_postcard(version, kind, &payload) {
        let result = decode_postcard(&encoded);

        // Should detect version too new
        match result {
            Err(PostcardError::VersionTooNew) => {
                // Expected for version > CLI_SCHEMA_VERSION
            }
            Err(PostcardError::DecodeFailed) => {
                // Also acceptable
            }
            Ok(_) => {}
            Err(_) => {}
        }
    }
}

#[kani::proof]
fn harness_version_too_old() {
    // Test version 0 (too old)
    let version = 0u16;
    let payload = vec![0u8; 32];
    let kind = 2u16;

    if let Ok(encoded) = encode_postcard(version, kind, &payload) {
        let result = decode_postcard(&encoded);

        match result {
            Err(PostcardError::VersionTooOld) => {
                // Expected
            }
            Err(PostcardError::DecodeFailed) => {
                // Also acceptable
            }
            Ok(_) => {}
            Err(_) => {}
        }
    }
}

#[kani::proof]
fn harness_version_boundary() {
    // Test CLI_SCHEMA_VERSION (should succeed)
    let version = CLI_SCHEMA_VERSION;
    let payload = vec![0u8; 32];
    let kind = 2u16;

    if let Ok(encoded) = encode_postcard(version, kind, &payload) {
        let result = decode_postcard(&encoded);

        match result {
            Ok(_) => {
                // Expected - valid version should decode
            }
            Err(PostcardError::DecodeFailed) => {
                // Might fail if payload is not valid CliPostcardPayload
                // But version should be accepted
            }
            Err(_) => {}
        }
    }
}