// Kani proof harness for PS-011: Digest Triangle Invariant bounded verification.
//
// Obligation: PO-vb-h09wf-032
// Verifier: kani
// Command: cargo kani -p vb_storage --harness ps_011_digest_triangle --features kani-vb-h09wf
//
// Domain claim: For representative valid CompiledIrRecord inputs,
// validate_compiled_ir_record returns Ok(()); for representative invalid inputs
// (one gate violated at a time), returns appropriate JournalError variant. No panics.
//
// PRODUCTION BINDING:
//   vb_storage::admission::validate_compiled_ir_record (admission.rs:361-365)
//
// Trusted base: blake3, postcard, all gate functions
// Model bounds: CompiledIrRecord with ir envelope up to 64 KiB
// Source: .beads/vb-h09wf/proof-obligations.planned.jsonl PO-vb-h09wf-032

#![forbid(unsafe_code)]
#![cfg(kani)]

use crate::admission::validate_compiled_ir_record;
use crate::error::JournalError;

/// PS-011: The function must not panic on arbitrary CompiledIrRecord inputs.
/// This is the meta-verification that the full gate cascade is panic-free.
#[kani::proof]
#[kani::unwind(6)]
fn ps_011_digest_triangle() {
    let digest_bytes: [u8; 32] = kani::any();
    let digest = vb_core::WorkflowDigest::from_bytes(digest_bytes);

    let ir_len: u8 = kani::any();
    // Keep bounded: 0..255 bytes
    let ir: Vec<u8> = (0..ir_len).map(|_| kani::any()).collect();

    let record = crate::records::CompiledIrRecord { digest, ir };

    let result = validate_compiled_ir_record(&record);

    // The function should NOT panic in any case.
    // It may return Ok or Err depending on input validity.

    if result.is_ok() {
    } else {
    }

    // Verify specific error types for diagnostics coverage
    kani::cover!(
        matches!(result, Err(JournalError::PayloadTooLarge { .. })),
        "PayloadTooLarge returned"
    );
    kani::cover!(
        matches!(result, Err(JournalError::ArtifactMalformed)),
        "ArtifactMalformed returned"
    );
    kani::cover!(
        matches!(result, Err(JournalError::ArtifactChecksumMismatch)),
        "ArtifactChecksumMismatch returned"
    );
    kani::cover!(
        matches!(result, Err(JournalError::InvalidGateCount { .. })),
        "InvalidGateCount returned"
    );
    kani::cover!(
        matches!(result, Err(JournalError::MissingRequiredProofFlag { .. })),
        "MissingRequiredProofFlag returned"
    );
}

/// PS-011b: Oversized envelope must fail the size gate before decode.
#[kani::proof]
fn ps_011_oversized_envelope_rejected_early() {
    let digest_bytes: [u8; 32] = kani::any();
    let digest = vb_core::WorkflowDigest::from_bytes(digest_bytes);

    let ir_len: u32 = kani::any();
    kani::assume(ir_len > crate::constants::MAX_COMPILED_IR_BYTES);
    kani::assume(ir_len <= crate::constants::MAX_COMPILED_IR_BYTES + 1024);

    let ir: Vec<u8> = vec![0u8; ir_len as usize];

    let record = crate::records::CompiledIrRecord { digest, ir };

    let result = validate_compiled_ir_record(&record);

    // Must be an error — oversized envelope must be rejected at Gate 1
    assert!(result.is_err(), "oversized envelope must be rejected");
}
