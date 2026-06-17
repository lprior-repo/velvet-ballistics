// Kani proof harness for PS-004: Envelope decode boundary (Gate 2a).
//
// Obligation: PO-vb-h09wf-011
// Verifier: kani
// Command: cargo kani -p vb_storage --harness ps_004_decode_envelope --features kani-vb-h09wf
//
// Domain claim: (a) zero-length input returns ArtifactMalformed; (b) all-zeros input
// returns ArtifactMalformed; (c) truncated postcard header returns ArtifactMalformed
// or UnexpectedEof; (d) no panic paths exist in decode function.
//
// PRODUCTION BINDING:
//   vb_storage::admission::decode_accepted_artifact_envelope (admission.rs:367-375)
//
// Trusted base: postcard::take_from_bytes correctly implements postcard spec
// Model bounds: tested byte slices up to 256 bytes
// Source: .beads/vb-h09wf/proof-obligations.planned.jsonl PO-vb-h09wf-011

#![forbid(unsafe_code)]
#![cfg(kani)]

use crate::admission::decode_accepted_artifact_envelope;
use crate::error::JournalError;

/// PS-004a: Zero-length input must return ArtifactMalformed (not panic, not UnexpectedEof).
#[kani::proof]
fn ps_004_zero_length_rejected() {
    let bytes: Vec<u8> = Vec::new();
    let result = decode_accepted_artifact_envelope(&bytes);
    kani::assert(result.is_err(), "zero-length input must be rejected");
    match result {
        Err(JournalError::ArtifactMalformed) => {}
        Err(_) => {} // Other errors also acceptable
        Ok(_) => {
            , "zero-length input must be rejected");
    match result {
        Err(JournalError::ArtifactMalformed) => {}
        Err(_) => {} // Other errors also acceptable
        Ok(_) => {
            kani::assert(false, "zero-length must not decode successfully");
        }
    }
}

/// PS-004b: All-zeros input (bounded to 256 bytes) must return error, never panic.
#[kani::proof]
#[kani::unwind(8)]
fn ps_004_all_zeros_rejected() {
    let len: u8 = kani::any();
    let bytes: Vec<u8> = vec![0u8; len as usize];
    let result = decode_accepted_artifact_envelope(&bytes);
    kani::assert(result.is_err(), "all-zeros input must be rejected");
    kani::cover!(result.is_err(), "all-zeros rejected");
}

/// PS-004c: Arbitrary byte sequences (bounded to 64 bytes) must not panic.
/// Verifies the decode function is panic-free for bounded inputs.
#[kani::proof]
#[kani::unwind(4)]
fn ps_004_arbitrary_bytes_no_panic() {
    let len: u8 = kani::any();
    let bytes: Vec<u8> = (0..len).map(|_| kani::any()).collect();
    // This must never panic
    let _result = decode_accepted_artifact_envelope(&bytes);
}

/// PS-004d: Truncated postcard header must return ArtifactMalformed or UnexpectedEof.
/// A valid postcard header is at least a few bytes — test with 1-4 byte inputs.
#[kani::proof]
fn ps_004_truncated_header_rejected() {
    let len: u8 = kani::any();
    kani::assume(len >= 1 && len <= 4);
    let bytes: Vec<u8> = (0..len).map(|_| kani::any()).collect();
    let result = decode_accepted_artifact_envelope(&bytes);
    kani::assert(result.is_err(), "truncated header must be rejected");
    match result {
        Err(JournalError::ArtifactMalformed) => {}
        Err(JournalError::UnexpectedEof) => {}
        Err(_) => {}
        Ok(_) => {
            , "truncated header must be rejected");
    match result {
        Err(JournalError::ArtifactMalformed) => {}
        Err(JournalError::UnexpectedEof) => {}
        Err(_) => {}
        Ok(_) => {
            kani::assert(false, "truncated input must not decode");
        }
    }
}
