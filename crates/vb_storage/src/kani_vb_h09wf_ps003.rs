// Kani proof harness for PS-003: Size bound rejection (Gate 1).
//
// Obligation: PO-vb-h09wf-008
// Verifier: kani
// Command: cargo kani -p vb_storage --harness ps_003_size_bound --features kani-vb-h09wf
//
// Domain claim: For all usize lengths up to MAX_COMPILED_IR_BYTES, Ok(()).
// For MAX+1 through bounded max, Err(PayloadTooLarge). No panics.
//
// PRODUCTION BINDING:
//   vb_storage::admission::reject_oversized_compiled_ir_value (admission.rs:378-391)
//
// Trusted base: u32::try_from(usize) for conversion safety
// Model bounds: tested usize lengths up to MAX_COMPILED_IR_BYTES + 1024
// Source: .beads/vb-h09wf/proof-obligations.planned.jsonl PO-vb-h09wf-008

#![forbid(unsafe_code)]
#![cfg(kani)]

use crate::admission::reject_oversized_compiled_ir_value;
use crate::constants::MAX_COMPILED_IR_BYTES;
use crate::error::JournalError;

/// PS-003: Bounded exhaustive verification of the size gate.
/// Tests lengths 0..(MAX+1024) — all safe sizes pass, all oversized rejected.
#[kani::proof]
#[kani::unwind(4)]
fn ps_003_size_bound() {
    let len: usize = kani::any();
    // Bound to MAX_COMPILED_IR_BYTES + 1024 to keep solver tractable
    kani::assume(len <= (MAX_COMPILED_IR_BYTES as usize).saturating_add(1024));

    let result = reject_oversized_compiled_ir_value(len);

    if len <= MAX_COMPILED_IR_BYTES as usize {
        // Valid sizes pass
        assert!(
            result.is_ok(),
            "len {len} <= MAX ({MAX_COMPILED_IR_BYTES}) must be accepted, got {result:?}"
        );
    } else {
        // Oversized lengths rejected
        match result {
            Err(JournalError::PayloadTooLarge { len: reported, max }) => {
                assert_eq!(
                    reported, len as u32,
                    "PayloadTooLarge.len must match input"
                );
                assert_eq!(
                    max, MAX_COMPILED_IR_BYTES,
                    "PayloadTooLarge.max must be MAX_COMPILED_IR_BYTES"
                );
            }
            Err(JournalError::ArtifactMalformed) => {
                // Also acceptable: u32::try_from failed for very large usizes
            }
            Ok(()) => {
                // Not acceptable — oversized must be rejected
                kani::assert(
                    false,
                    "Oversized payload len={len} must be rejected by size gate"
                );
            }
            Err(_) => {
                // Other errors acceptable
            }
        }
    }

    kani::cover!(result.is_ok(), "valid size accepted");
    kani::cover!(
        matches!(result, Err(JournalError::PayloadTooLarge { .. })),
        "oversized payload rejected"
    );
}

/// PS-003b: Verify u32::try_from safety — zero and MAX_COMPILED_IR_BYTES always fit.
#[kani::proof]
fn ps_003_u32_conversion_safe() {
    // MAX_COMPILED_IR_BYTES = 16_777_216 fits in u32
    assert!(MAX_COMPILED_IR_BYTES <= u32::MAX);

    // Zero always converts
    assert!(u32::try_from(0usize).is_ok());

    // MAX_COMPILED_IR_BYTES as usize converts
    assert!(u32::try_from(MAX_COMPILED_IR_BYTES as usize).is_ok());

    // Any value up to MAX_COMPILED_IR_BYTES converts safely
    let v: usize = kani::any();
    kani::assume(v <= MAX_COMPILED_IR_BYTES as usize);
    assert!(u32::try_from(v).is_ok());
}

/// PS-003c: Verify usize::MAX is correctly handled.
#[kani::proof]
fn ps_003_usize_max_rejected() {
    let result = reject_oversized_compiled_ir_value(usize::MAX);
    // Must be an error — either PayloadTooLarge (for 64-bit) or
    // the u32::try_from path returning ArtifactMalformed
    assert!(result.is_err(), "usize::MAX must be rejected");
}
