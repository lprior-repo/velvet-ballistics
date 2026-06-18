// Kani proof harness for PS-004: envelope decode boundary (Gate 2a).

#![forbid(unsafe_code)]
#![cfg(kani)]

use crate::admission::decode_accepted_artifact_envelope;
use crate::error::JournalError;

fn short_bytes(len: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    for index in 0_u8..4 {
        if index < len {
            bytes.push(kani::any());
        }
    }
    bytes
}

/// PS-004a: zero-length input is rejected by the postcard envelope decoder.
#[kani::proof]
fn ps_004_zero_length_rejected() {
    let bytes: [u8; 0] = [];
    let result = decode_accepted_artifact_envelope(&bytes);

    match result {
        Err(JournalError::ArtifactMalformed) => {}
        Err(_) => {}
        Ok(_) => kani::assert(false, "zero-length input must not decode"),
    }
}

/// PS-004d: one-to-four byte inputs are too short for an AcceptedArtifact envelope.
#[kani::proof]
#[kani::unwind(8)]
fn ps_004_truncated_header_rejected() {
    let len: u8 = kani::any();
    kani::assume(len >= 1 && len <= 4);
    let bytes = short_bytes(len);
    let result = decode_accepted_artifact_envelope(&bytes);

    match result {
        Err(JournalError::ArtifactMalformed) => {}
        Err(JournalError::UnexpectedEof) => {}
        Err(_) => {}
        Ok(_) => kani::assert(false, "truncated input must not decode"),
    }
}
